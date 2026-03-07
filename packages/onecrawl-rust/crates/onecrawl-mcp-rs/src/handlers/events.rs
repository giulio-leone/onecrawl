//! Handler implementations for the `events` super-tool (Event Bus).

use rmcp::{ErrorData as McpError, model::*};
use std::sync::Arc;
use crate::cdp_tools::*;
use crate::helpers::{mcp_err, json_ok};
use crate::OneCrawlMcp;

impl OneCrawlMcp {
    /// Ensure the event bus exists, creating it lazily if needed.
    async fn ensure_event_bus(&self) -> Arc<onecrawl_cdp::EventBus> {
        let mut state = self.browser.lock().await;
        if state.event_bus.is_none() {
            state.event_bus = Some(Arc::new(onecrawl_cdp::EventBus::new(
                onecrawl_cdp::EventBusConfig::default(),
            )));
        }
        state.event_bus.clone().unwrap()
    }

    // ════════════════════════════════════════════════════════════════
    //  Event Bus handlers
    // ════════════════════════════════════════════════════════════════

    pub(crate) async fn events_emit(
        &self,
        p: EventsEmitParams,
    ) -> Result<CallToolResult, McpError> {
        let bus = self.ensure_event_bus().await;
        let event = onecrawl_cdp::BusEvent {
            id: onecrawl_cdp::event_bus::generate_id(),
            event_type: p.event_type.clone(),
            source: p.source.unwrap_or_else(|| "mcp".to_string()),
            timestamp: onecrawl_cdp::event_bus::iso_now(),
            data: p.data.unwrap_or(serde_json::json!({})),
            metadata: p.metadata,
        };
        let id = event.id.clone();
        bus.emit(event)
            .await
            .map_err(|e| mcp_err(format!("emit failed: {e}")))?;

        json_ok(&serde_json::json!({
            "action": "events_emit",
            "id": id,
            "event_type": p.event_type,
            "status": "emitted"
        }))
    }

    pub(crate) async fn events_subscribe(
        &self,
        p: EventsSubscribeParams,
    ) -> Result<CallToolResult, McpError> {
        let bus = self.ensure_event_bus().await;
        let sub = onecrawl_cdp::WebhookSubscription {
            id: String::new(),
            event_pattern: p.event_pattern.clone(),
            url: p.url.clone(),
            method: p.method,
            headers: p.headers,
            secret: p.secret,
            active: true,
            retry_count: p.retry_count.unwrap_or(3),
            retry_delay_ms: p.retry_delay_ms.unwrap_or(1000),
            created_at: onecrawl_cdp::event_bus::iso_now(),
            last_triggered: None,
            trigger_count: 0,
            last_error: None,
        };
        let id = bus
            .subscribe_webhook(sub)
            .await
            .map_err(|e| mcp_err(format!("subscribe failed: {e}")))?;

        json_ok(&serde_json::json!({
            "action": "events_subscribe",
            "id": id,
            "event_pattern": p.event_pattern,
            "url": p.url,
            "status": "subscribed"
        }))
    }

    pub(crate) async fn events_unsubscribe(
        &self,
        p: EventsUnsubscribeParams,
    ) -> Result<CallToolResult, McpError> {
        let bus = self.ensure_event_bus().await;
        bus.unsubscribe_webhook(&p.id)
            .await
            .map_err(|e| mcp_err(format!("unsubscribe failed: {e}")))?;

        json_ok(&serde_json::json!({
            "action": "events_unsubscribe",
            "id": p.id,
            "status": "unsubscribed"
        }))
    }

    pub(crate) async fn events_list_subscriptions(
        &self,
        _p: EventsListParams,
    ) -> Result<CallToolResult, McpError> {
        let bus = self.ensure_event_bus().await;
        let subs = bus.list_webhooks().await;
        let count = subs.len();

        json_ok(&serde_json::json!({
            "action": "events_list_subscriptions",
            "subscriptions": subs,
            "count": count
        }))
    }

    pub(crate) async fn events_recent(
        &self,
        p: EventsRecentParams,
    ) -> Result<CallToolResult, McpError> {
        let limit = p.limit.unwrap_or(50);
        let bus = self.ensure_event_bus().await;
        let events = bus.recent_events(limit).await;
        let count = events.len();

        json_ok(&serde_json::json!({
            "action": "events_recent",
            "events": events,
            "count": count
        }))
    }

    pub(crate) async fn events_replay(
        &self,
        p: EventsReplayParams,
    ) -> Result<CallToolResult, McpError> {
        let bus = self.ensure_event_bus().await;
        let events = bus
            .replay(&p.event_pattern, p.since.as_deref())
            .await
            .map_err(|e| mcp_err(format!("replay failed: {e}")))?;
        let count = events.len();

        json_ok(&serde_json::json!({
            "action": "events_replay",
            "event_pattern": p.event_pattern,
            "events": events,
            "count": count
        }))
    }

    pub(crate) async fn events_stats(
        &self,
        _p: EventsStatsParams,
    ) -> Result<CallToolResult, McpError> {
        let bus = self.ensure_event_bus().await;
        let stats = bus.stats().await;

        json_ok(&serde_json::json!({
            "action": "events_stats",
            "stats": stats
        }))
    }

    pub(crate) async fn events_clear(
        &self,
        _p: EventsClearParams,
    ) -> Result<CallToolResult, McpError> {
        let bus = self.ensure_event_bus().await;
        bus.clear_journal()
            .await
            .map_err(|e| mcp_err(format!("clear failed: {e}")))?;

        json_ok(&serde_json::json!({
            "action": "events_clear",
            "status": "cleared"
        }))
    }
}
