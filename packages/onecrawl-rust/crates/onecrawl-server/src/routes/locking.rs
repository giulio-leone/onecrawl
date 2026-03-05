use axum::extract::{Json, Path, State};
use axum::http::StatusCode;
use axum::response::IntoResponse;

use crate::state::AppState;

#[derive(serde::Deserialize)]
pub(crate) struct LockTabRequest {
    owner: String,
    ttl_secs: Option<u64>,
}

#[derive(serde::Deserialize)]
pub(crate) struct UnlockTabRequest {
    owner: String,
}

pub async fn lock_tab(
    State(state): State<AppState>,
    Path(tab_id): Path<String>,
    Json(req): Json<LockTabRequest>,
) -> impl IntoResponse {
    match state.lock_tab(&tab_id, &req.owner, req.ttl_secs).await {
        Ok(()) => {
            let lock = state.get_tab_lock(&tab_id).await.unwrap();
            (
                StatusCode::OK,
                Json(serde_json::json!({
                    "locked": true,
                    "owner": lock.owner,
                    "ttl_secs": lock.ttl_secs
                })),
            )
        }
        Err(current_owner) => (
            StatusCode::CONFLICT,
            Json(serde_json::json!({
                "error": "tab already locked",
                "current_owner": current_owner
            })),
        ),
    }
}

pub async fn unlock_tab(
    State(state): State<AppState>,
    Path(tab_id): Path<String>,
    Json(req): Json<UnlockTabRequest>,
) -> impl IntoResponse {
    let released = state.unlock_tab(&tab_id, &req.owner).await;
    if released {
        (StatusCode::OK, Json(serde_json::json!({"unlocked": true})))
    } else {
        (
            StatusCode::FORBIDDEN,
            Json(serde_json::json!({"error": "not the lock owner"})),
        )
    }
}

pub async fn get_tab_lock(
    State(state): State<AppState>,
    Path(tab_id): Path<String>,
) -> impl IntoResponse {
    match state.get_tab_lock(&tab_id).await {
        Some(lock) => Json(serde_json::json!({
            "locked": true,
            "owner": lock.owner,
            "ttl_secs": lock.ttl_secs
        })),
        None => Json(serde_json::json!({"locked": false})),
    }
}
