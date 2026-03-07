//! HTTP routes — split by domain.

mod actions;
mod content;
mod events;
mod instances;
mod locking;
mod profiles;
mod studio;
mod tabs;

use axum::extract::Json;
use axum::http::StatusCode;
use axum::routing::{delete, get, post};
use axum::Router;
use serde::Serialize;
use tower_http::compression::CompressionLayer;
use tower_http::cors::{Any, CorsLayer};

use crate::state::AppState;

#[derive(Serialize)]
pub(crate) struct ErrorBody {
    pub error: String,
}

pub(crate) type ApiResult<T> = Result<Json<T>, (StatusCode, Json<ErrorBody>)>;

pub(crate) fn api_err(status: StatusCode, msg: &str) -> (StatusCode, Json<ErrorBody>) {
    (
        status,
        Json(ErrorBody { error: msg.to_owned() }),
    )
}

/// Helper: resolve tab, check lock ownership, clone Page handle (cheap channel clone).
pub(crate) async fn get_tab_page(
    state: &AppState,
    tab_id: &str,
    owner: Option<&str>,
) -> Result<chromiumoxide::Page, (StatusCode, Json<ErrorBody>)> {
    // Lock enforcement: if tab is locked by a different owner, reject with 409
    if let Some(lock) = state.get_tab_lock(tab_id).await {
        match owner {
            Some(o) if o == lock.owner => {}
            _ => {
                return Err(api_err(
                    StatusCode::CONFLICT,
                    &format!("tab locked by {}", lock.owner),
                ));
            }
        }
    }

    let inst_id = state
        .instance_for_tab(tab_id)
        .await
        .ok_or_else(|| api_err(StatusCode::NOT_FOUND, "tab not found"))?;
    let instances = state.instances.read().await;
    let inst = instances
        .get(&inst_id)
        .ok_or_else(|| api_err(StatusCode::NOT_FOUND, "instance gone"))?;
    let tabs = inst.tabs.read().await;
    let page = tabs
        .get(tab_id)
        .ok_or_else(|| api_err(StatusCode::NOT_FOUND, "tab gone"))?
        .clone();
    Ok(page)
}

#[derive(Serialize)]
struct HealthResponse {
    status: &'static str,
    service: &'static str,
}

async fn health() -> axum::response::Json<HealthResponse> {
    Json(HealthResponse { status: "ok", service: "onecrawl-server" })
}

pub fn create_router(state: AppState) -> Router {
    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    Router::new()
        // Health
        .route("/health", get(health))
        // Instance management
        .route("/instances", post(instances::create_instance))
        .route("/instances", get(instances::list_instances))
        .route("/instances/{id}", get(instances::get_instance))
        .route("/instances/{id}", delete(instances::stop_instance))
        // Instance tabs
        .route("/instances/{id}/tabs/open", post(tabs::open_tab))
        .route("/instances/{id}/tabs", get(tabs::get_instance_tabs))
        // Tab operations
        .route("/tabs", get(tabs::list_all_tabs))
        .route("/tabs/{tab_id}/navigate", post(tabs::navigate_tab))
        .route("/tabs/{tab_id}/snapshot", get(content::get_snapshot))
        .route("/tabs/{tab_id}/text", get(content::get_text))
        .route("/tabs/{tab_id}/url", get(content::get_url))
        .route("/tabs/{tab_id}/title", get(content::get_title))
        .route("/tabs/{tab_id}/html", get(content::get_html))
        .route("/tabs/{tab_id}/action", post(actions::execute_action))
        .route("/tabs/{tab_id}/actions", post(actions::execute_actions))
        .route("/tabs/{tab_id}/evaluate", post(content::evaluate_js))
        .route("/tabs/{tab_id}/screenshot", get(content::take_screenshot))
        .route("/tabs/{tab_id}/pdf", get(content::export_pdf))
        // Profiles
        .route("/profiles", get(profiles::list_profiles))
        .route("/profiles", post(profiles::create_profile))
        // Tab locking
        .route("/tabs/{tab_id}/lock", post(locking::lock_tab))
        .route("/tabs/{tab_id}/lock", delete(locking::unlock_tab))
        .route("/tabs/{tab_id}/lock", get(locking::get_tab_lock))
        // Event bus
        .route("/events/emit", post(events::emit_event))
        .route("/events/subscribe", post(events::subscribe_webhook))
        .route("/events/subscribe/{id}", delete(events::unsubscribe_webhook))
        .route("/events/subscriptions", get(events::list_subscriptions))
        .route("/events/recent", get(events::recent_events))
        .route("/events/replay", post(events::replay_events))
        .route("/events/stats", get(events::event_stats))
        .route("/events/stream", get(events::event_stream))
        .route("/events/journal", delete(events::clear_journal))
        // Studio
        .route("/studio", get(studio::studio_page))
        .route("/studio/api/templates", get(studio::list_templates))
        .route("/studio/api/templates/{id}", get(studio::get_template))
        .route("/studio/api/projects", get(studio::list_projects))
        .route("/studio/api/projects", post(studio::save_project))
        .route("/studio/api/projects/{id}", get(studio::get_project))
        .route("/studio/api/projects/{id}", delete(studio::delete_project))
        .route("/studio/api/validate", post(studio::validate_workflow))
        .route("/studio/api/export/{id}", post(studio::export_project))
        .route("/studio/api/import", post(studio::import_workflow))
        .layer(cors)
        .layer(CompressionLayer::new())
        .with_state(state)
}
