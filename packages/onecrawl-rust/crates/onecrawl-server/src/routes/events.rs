//! HTTP route handlers for the Event Bus.

use axum::extract::{Json, Path, Query, State};
use axum::http::StatusCode;
use axum::response::sse::{Event, KeepAlive, Sse};
use futures::stream::Stream;
use serde::Deserialize;
use std::collections::HashMap;
use std::convert::Infallible;
use std::pin::Pin;
use std::task::{Context, Poll};

use crate::routes::{ApiResult, api_err};
use crate::state::AppState;

// ── Request/Response types ──────────────────────────────────────────

#[derive(Deserialize)]
pub struct EmitRequest {
    pub event_type: String,
    #[serde(default = "default_source")]
    pub source: String,
    #[serde(default = "default_data")]
    pub data: serde_json::Value,
    pub metadata: Option<HashMap<String, String>>,
}

fn default_source() -> String {
    "webhook".into()
}
fn default_data() -> serde_json::Value {
    serde_json::json!({})
}

#[derive(Deserialize)]
pub struct SubscribeRequest {
    pub event_pattern: String,
    pub url: String,
    pub method: Option<String>,
    pub headers: Option<HashMap<String, String>>,
    pub secret: Option<String>,
    pub retry_count: Option<u32>,
    pub retry_delay_ms: Option<u64>,
}

#[derive(Deserialize)]
pub struct ReplayRequest {
    pub event_pattern: String,
    pub since: Option<String>,
}

#[derive(Deserialize)]
pub struct RecentQuery {
    pub limit: Option<usize>,
}

#[derive(Deserialize)]
pub struct StreamQuery {
    pub pattern: Option<String>,
}

// ── Handlers ────────────────────────────────────────────────────────

/// POST /events/emit
pub async fn emit_event(
    State(state): State<AppState>,
    Json(req): Json<EmitRequest>,
) -> ApiResult<serde_json::Value> {
    let event = onecrawl_cdp::BusEvent {
        id: onecrawl_cdp::event_bus::generate_id(),
        event_type: req.event_type,
        source: req.source,
        timestamp: onecrawl_cdp::event_bus::iso_now(),
        data: req.data,
        metadata: req.metadata,
    };
    let id = event.id.clone();
    state
        .event_bus
        .emit(event)
        .await
        .map_err(|e| api_err(StatusCode::INTERNAL_SERVER_ERROR, &e))?;
    Ok(Json(serde_json::json!({ "status": "emitted", "id": id })))
}

/// POST /events/subscribe
pub async fn subscribe_webhook(
    State(state): State<AppState>,
    Json(req): Json<SubscribeRequest>,
) -> ApiResult<serde_json::Value> {
    let sub = onecrawl_cdp::WebhookSubscription {
        id: String::new(),
        event_pattern: req.event_pattern,
        url: req.url,
        method: req.method,
        headers: req.headers,
        secret: req.secret,
        active: true,
        retry_count: req.retry_count.unwrap_or(3),
        retry_delay_ms: req.retry_delay_ms.unwrap_or(1000),
        created_at: onecrawl_cdp::event_bus::iso_now(),
        last_triggered: None,
        trigger_count: 0,
        last_error: None,
    };
    let id = state
        .event_bus
        .subscribe_webhook(sub)
        .await
        .map_err(|e| api_err(StatusCode::BAD_REQUEST, &e))?;
    Ok(Json(
        serde_json::json!({ "status": "subscribed", "id": id }),
    ))
}

/// DELETE /events/subscribe/:id
pub async fn unsubscribe_webhook(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> ApiResult<serde_json::Value> {
    state
        .event_bus
        .unsubscribe_webhook(&id)
        .await
        .map_err(|e| api_err(StatusCode::NOT_FOUND, &e))?;
    Ok(Json(
        serde_json::json!({ "status": "unsubscribed", "id": id }),
    ))
}

/// GET /events/subscriptions
pub async fn list_subscriptions(
    State(state): State<AppState>,
) -> ApiResult<Vec<onecrawl_cdp::WebhookSubscription>> {
    let subs = state.event_bus.list_webhooks().await;
    Ok(Json(subs))
}

/// GET /events/recent
pub async fn recent_events(
    State(state): State<AppState>,
    Query(q): Query<RecentQuery>,
) -> ApiResult<Vec<onecrawl_cdp::BusEvent>> {
    let limit = q.limit.unwrap_or(50);
    let events = state.event_bus.recent_events(limit).await;
    Ok(Json(events))
}

/// POST /events/replay
pub async fn replay_events(
    State(state): State<AppState>,
    Json(req): Json<ReplayRequest>,
) -> ApiResult<Vec<onecrawl_cdp::BusEvent>> {
    let events = state
        .event_bus
        .replay(&req.event_pattern, req.since.as_deref())
        .await
        .map_err(|e| api_err(StatusCode::BAD_REQUEST, &e))?;
    Ok(Json(events))
}

/// GET /events/stats
pub async fn event_stats(
    State(state): State<AppState>,
) -> ApiResult<onecrawl_cdp::BusStats> {
    let stats = state.event_bus.stats().await;
    Ok(Json(stats))
}

/// DELETE /events/journal
pub async fn clear_journal(
    State(state): State<AppState>,
) -> ApiResult<serde_json::Value> {
    state
        .event_bus
        .clear_journal()
        .await
        .map_err(|e| api_err(StatusCode::INTERNAL_SERVER_ERROR, &e))?;
    Ok(Json(serde_json::json!({ "status": "cleared" })))
}

/// Wrapper around broadcast::Receiver that implements Stream.
struct BusEventStream {
    rx: tokio::sync::broadcast::Receiver<onecrawl_cdp::BusEvent>,
    pattern: String,
}

impl Stream for BusEventStream {
    type Item = Result<Event, Infallible>;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        loop {
            match self.rx.try_recv() {
                Ok(event) => {
                    if onecrawl_cdp::event_bus::matches_pattern(&event.event_type, &self.pattern) {
                        if let Ok(json) = serde_json::to_string(&event) {
                            return Poll::Ready(Some(Ok(Event::default()
                                .event(event.event_type)
                                .data(json))));
                        }
                    }
                    // Skip events that don't match — loop to next
                }
                Err(tokio::sync::broadcast::error::TryRecvError::Empty) => {
                    // Register waker — wake on next message via a spawned task
                    let waker = cx.waker().clone();
                    let mut rx2 = self.rx.resubscribe();
                    tokio::spawn(async move {
                        let _ = rx2.recv().await;
                        waker.wake();
                    });
                    return Poll::Pending;
                }
                Err(tokio::sync::broadcast::error::TryRecvError::Lagged(n)) => {
                    return Poll::Ready(Some(Ok(Event::default()
                        .event("system")
                        .data(format!("{{\"warning\":\"lagged {} events\"}}", n)))));
                }
                Err(tokio::sync::broadcast::error::TryRecvError::Closed) => {
                    return Poll::Ready(None);
                }
            }
        }
    }
}

/// GET /events/stream — SSE real-time event stream
pub async fn event_stream(
    State(state): State<AppState>,
    Query(q): Query<StreamQuery>,
) -> Sse<impl Stream<Item = Result<Event, Infallible>>> {
    let rx = state.event_bus.subscribe_stream();
    let pattern = q.pattern.unwrap_or_else(|| "**".to_string());

    Sse::new(BusEventStream { rx, pattern }).keep_alive(KeepAlive::default())
}
