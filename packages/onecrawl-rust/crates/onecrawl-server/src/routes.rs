use axum::extract::{Json, Path, Query, State};
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::routing::{delete, get, post};
use axum::Router;
use serde::Serialize;
use std::sync::Arc;
use tower_http::cors::{Any, CorsLayer};

use crate::action::{parse_ref_id, Action, ActionResult};
use crate::instance::{CreateInstanceRequest, Instance, InstanceInfo};
use crate::profile::{CreateProfileRequest, Profile};
use crate::snapshot::{
    PageSnapshot, SnapshotQuery, SNAPSHOT_JS, TEXT_EXTRACT_JS,
    click_by_index_js, fill_by_index_js, focus_by_index_js, hover_by_index_js,
    select_by_index_js, type_by_index_js,
};
use crate::state::AppState;
use crate::tab::{NavigateRequest, OpenTabRequest, TabInfo};

type ApiResult<T> = Result<Json<T>, (StatusCode, Json<serde_json::Value>)>;

fn api_err(status: StatusCode, msg: &str) -> (StatusCode, Json<serde_json::Value>) {
    (
        status,
        Json(serde_json::json!({ "error": msg })),
    )
}

// ── Typed Response Structs (avoid json!() macro overhead) ───

#[derive(Serialize)]
struct HealthResponse {
    status: &'static str,
    service: &'static str,
}

#[derive(Serialize)]
struct InstanceResponse {
    instance: InstanceInfo,
}

#[derive(Serialize)]
struct InstancesResponse {
    instances: Vec<InstanceInfo>,
}

#[derive(Serialize)]
struct TabResponse {
    tab: TabInfo,
}

#[derive(Serialize)]
struct TabsResponse {
    tabs: Vec<TabInfo>,
}

#[derive(Serialize)]
struct NavigateResponse {
    url: String,
    title: String,
}

#[derive(Serialize)]
struct TextResponse {
    url: String,
    text: String,
}

#[derive(Serialize)]
struct EvalResponse {
    result: serde_json::Value,
}

#[derive(Serialize)]
struct ScreenshotResponse {
    format: &'static str,
    data: String,
    size: usize,
}

#[derive(Serialize)]
struct PdfResponse {
    format: &'static str,
    data: String,
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
        .route("/instances", post(create_instance))
        .route("/instances", get(list_instances))
        .route("/instances/{id}", get(get_instance))
        .route("/instances/{id}", delete(stop_instance))
        // Instance tabs
        .route("/instances/{id}/tabs/open", post(open_tab))
        .route("/instances/{id}/tabs", get(get_instance_tabs))
        // Tab operations
        .route("/tabs", get(list_all_tabs))
        .route("/tabs/{tab_id}/navigate", post(navigate_tab))
        .route("/tabs/{tab_id}/snapshot", get(get_snapshot))
        .route("/tabs/{tab_id}/text", get(get_text))
        .route("/tabs/{tab_id}/action", post(execute_action))
        .route("/tabs/{tab_id}/actions", post(execute_actions))
        .route("/tabs/{tab_id}/evaluate", post(evaluate_js))
        .route("/tabs/{tab_id}/screenshot", get(take_screenshot))
        .route("/tabs/{tab_id}/pdf", get(export_pdf))
        // Profiles
        .route("/profiles", get(list_profiles))
        .route("/profiles", post(create_profile))
        .layer(cors)
        .with_state(state)
}

// ── Health ──────────────────────────────────────────────────

async fn health() -> impl IntoResponse {
    Json(HealthResponse { status: "ok", service: "onecrawl-server" })
}

// ── Instance Management ─────────────────────────────────────

async fn create_instance(
    State(state): State<AppState>,
    Json(req): Json<CreateInstanceRequest>,
) -> ApiResult<InstanceResponse> {
    let headless = req.headless.unwrap_or(true);
    let id = format!("inst_{}", uuid::Uuid::new_v4().as_simple());

    let mut port_guard = state.next_instance_port.write().await;
    let port = *port_guard;
    *port_guard += 1;
    drop(port_guard);

    // Resolve profile's user data dir if specified.
    let user_data_dir = if let Some(ref profile_name) = req.profile {
        let profiles = state.profiles.read().await;
        profiles.get(profile_name).map(|p| p.user_data_dir())
    } else {
        None
    };

    let instance = Instance::launch(id.clone(), headless, port, req.profile.clone(), user_data_dir)
        .await
        .map_err(|e| api_err(StatusCode::INTERNAL_SERVER_ERROR, &e))?;

    let info = instance.info().await;
    state.instances.write().await.insert(id, instance);

    Ok(Json(InstanceResponse { instance: info }))
}

async fn list_instances(State(state): State<AppState>) -> impl IntoResponse {
    let instances = state.instances.read().await;
    let mut infos = Vec::with_capacity(instances.len());
    for inst in instances.values() {
        infos.push(inst.info().await);
    }
    Json(InstancesResponse { instances: infos })
}

async fn get_instance(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> ApiResult<InstanceResponse> {
    let instances = state.instances.read().await;
    let inst = instances
        .get(&id)
        .ok_or_else(|| api_err(StatusCode::NOT_FOUND, "instance not found"))?;
    let info = inst.info().await;
    Ok(Json(InstanceResponse { instance: info }))
}

async fn stop_instance(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> ApiResult<serde_json::Value> {
    let mut instances = state.instances.write().await;
    let inst = instances
        .remove(&id)
        .ok_or_else(|| api_err(StatusCode::NOT_FOUND, "instance not found"))?;
    // Unregister all tabs from the index
    let tabs = inst.tabs.read().await;
    let tab_ids: Vec<String> = tabs.keys().cloned().collect();
    drop(tabs);
    for tid in &tab_ids {
        state.unregister_tab(tid).await;
    }
    Ok(Json(serde_json::json!({ "stopped": id })))
}

// ── Tab Management ──────────────────────────────────────────

async fn open_tab(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Json(req): Json<OpenTabRequest>,
) -> ApiResult<TabResponse> {
    let instances = state.instances.read().await;
    let inst = instances
        .get(&id)
        .ok_or_else(|| api_err(StatusCode::NOT_FOUND, "instance not found"))?;

    let url = req.url.as_deref().unwrap_or("about:blank");
    let page = inst
        .browser
        .new_page(url)
        .await
        .map_err(|e| api_err(StatusCode::INTERNAL_SERVER_ERROR, &format!("new_page: {e}")))?;

    let mut counter = inst.tab_counter.write().await;
    *counter += 1;
    let tab_id = format!("tab_{}_{}", inst.id, counter);
    drop(counter);

    let tab_url = page.url().await.ok().flatten().unwrap_or_default();
    let tab_title: String = page
        .evaluate("document.title")
        .await
        .ok()
        .and_then(|v| v.into_value().ok())
        .unwrap_or_default();

    let info = TabInfo {
        id: tab_id.clone(),
        url: tab_url,
        title: tab_title,
        instance_id: id.clone(),
    };

    inst.tabs.write().await.insert(tab_id.clone(), page);
    // Register in O(1) tab index
    state.register_tab(&tab_id, &id).await;

    Ok(Json(TabResponse { tab: info }))
}

async fn get_instance_tabs(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> ApiResult<TabsResponse> {
    let instances = state.instances.read().await;
    let inst = instances
        .get(&id)
        .ok_or_else(|| api_err(StatusCode::NOT_FOUND, "instance not found"))?;

    // Collect tab IDs first, then drop the lock before async calls
    let tabs = inst.tabs.read().await;
    let tab_entries: Vec<(String, String)> = {
        let mut entries = Vec::with_capacity(tabs.len());
        for (tid, page) in tabs.iter() {
            let url = page.url().await.ok().flatten().unwrap_or_default();
            let title: String = page
                .evaluate("document.title")
                .await
                .ok()
                .and_then(|v| v.into_value().ok())
                .unwrap_or_default();
            entries.push((tid.clone(), format!("{}\t{}", url, title)));
        }
        entries
    };
    drop(tabs);

    let infos: Vec<TabInfo> = tab_entries
        .into_iter()
        .map(|(tid, combined)| {
            let (url, title) = combined.split_once('\t').unwrap_or(("", ""));
            TabInfo {
                id: tid,
                url: url.to_owned(),
                title: title.to_owned(),
                instance_id: id.clone(),
            }
        })
        .collect();

    Ok(Json(TabsResponse { tabs: infos }))
}

async fn list_all_tabs(State(state): State<AppState>) -> impl IntoResponse {
    let instances = state.instances.read().await;
    let mut all_tabs = Vec::new();
    for inst in instances.values() {
        let tabs = inst.tabs.read().await;
        for (tid, page) in tabs.iter() {
            let url = page.url().await.ok().flatten().unwrap_or_default();
            let title: String = page
                .evaluate("document.title")
                .await
                .ok()
                .and_then(|v| v.into_value().ok())
                .unwrap_or_default();
            all_tabs.push(TabInfo {
                id: tid.clone(),
                url,
                title,
                instance_id: inst.id.clone(),
            });
        }
    }
    Json(TabsResponse { tabs: all_tabs })
}

// ── Tab Operations ──────────────────────────────────────────

/// O(1) tab lookup via the index in AppState.
async fn resolve_tab<'a>(
    state: &AppState,
    tab_id: &str,
) -> Result<String, (StatusCode, Json<serde_json::Value>)> {
    state
        .instance_for_tab(tab_id)
        .await
        .ok_or_else(|| api_err(StatusCode::NOT_FOUND, "tab not found"))
}

async fn navigate_tab(
    State(state): State<AppState>,
    Path(tab_id): Path<String>,
    Json(req): Json<NavigateRequest>,
) -> ApiResult<NavigateResponse> {
    let inst_id = resolve_tab(&state, &tab_id).await?;
    let instances = state.instances.read().await;
    let inst = instances.get(&inst_id).ok_or_else(|| api_err(StatusCode::NOT_FOUND, "instance gone"))?;
    let tabs = inst.tabs.read().await;
    let page = tabs.get(&tab_id).ok_or_else(|| api_err(StatusCode::NOT_FOUND, "tab gone"))?;

    page.goto(&req.url)
        .await
        .map_err(|e| api_err(StatusCode::INTERNAL_SERVER_ERROR, &format!("navigate: {e}")))?;
    let current_url = page.url().await.ok().flatten().unwrap_or_default();
    let title: String = page
        .evaluate("document.title")
        .await
        .ok()
        .and_then(|v| v.into_value().ok())
        .unwrap_or_default();
    Ok(Json(NavigateResponse { url: current_url, title }))
}

async fn get_snapshot(
    State(state): State<AppState>,
    Path(tab_id): Path<String>,
    Query(query): Query<SnapshotQuery>,
) -> ApiResult<serde_json::Value> {
    let inst_id = resolve_tab(&state, &tab_id).await?;
    let instances = state.instances.read().await;
    let inst = instances.get(&inst_id).ok_or_else(|| api_err(StatusCode::NOT_FOUND, "instance gone"))?;
    let tabs = inst.tabs.read().await;
    let page = tabs.get(&tab_id).ok_or_else(|| api_err(StatusCode::NOT_FOUND, "tab gone"))?;

    let result_str: String = page
        .evaluate(SNAPSHOT_JS)
        .await
        .map_err(|e| api_err(StatusCode::INTERNAL_SERVER_ERROR, &format!("snapshot eval: {e}")))?
        .into_value()
        .map_err(|e| api_err(StatusCode::INTERNAL_SERVER_ERROR, &format!("snapshot parse: {e}")))?;

    let mut snapshot: PageSnapshot = serde_json::from_str(&result_str)
        .map_err(|e| api_err(StatusCode::INTERNAL_SERVER_ERROR, &format!("snapshot json: {e}")))?;

    // Apply filter
    if query.filter.as_deref() == Some("interactive") {
        snapshot.elements.retain(|e| e.interactive);
    }

    // Cache for action lookups using Arc (avoids full clone)
    let elements = Arc::new(snapshot.elements);
    state
        .snapshots
        .write()
        .await
        .insert(tab_id, Arc::clone(&elements));

    // Format output
    if query.format.as_deref() == Some("compact") {
        let compact: Vec<serde_json::Value> = elements
            .iter()
            .map(|e| serde_json::json!([e.ref_id, e.role, e.name]))
            .collect();
        return Ok(Json(serde_json::json!({
            "url": snapshot.url,
            "title": snapshot.title,
            "elements": compact
        })));
    }

    Ok(Json(serde_json::json!({
        "url": snapshot.url,
        "title": snapshot.title,
        "elements": *elements
    })))
}

async fn get_text(
    State(state): State<AppState>,
    Path(tab_id): Path<String>,
) -> ApiResult<TextResponse> {
    let inst_id = resolve_tab(&state, &tab_id).await?;
    let instances = state.instances.read().await;
    let inst = instances.get(&inst_id).ok_or_else(|| api_err(StatusCode::NOT_FOUND, "instance gone"))?;
    let tabs = inst.tabs.read().await;
    let page = tabs.get(&tab_id).ok_or_else(|| api_err(StatusCode::NOT_FOUND, "tab gone"))?;

    let text: String = page
        .evaluate(TEXT_EXTRACT_JS)
        .await
        .map_err(|e| api_err(StatusCode::INTERNAL_SERVER_ERROR, &format!("text eval: {e}")))?
        .into_value()
        .map_err(|e| api_err(StatusCode::INTERNAL_SERVER_ERROR, &format!("text parse: {e}")))?;
    let url = page.url().await.ok().flatten().unwrap_or_default();
    Ok(Json(TextResponse { url, text }))
}

/// Execute a single action.
async fn execute_action(
    State(state): State<AppState>,
    Path(tab_id): Path<String>,
    Json(action): Json<Action>,
) -> ApiResult<ActionResult> {
    let result = run_action(&state, &tab_id, &action).await;
    Ok(Json(result))
}

/// Execute a batch of actions.
async fn execute_actions(
    State(state): State<AppState>,
    Path(tab_id): Path<String>,
    Json(actions): Json<Vec<Action>>,
) -> ApiResult<Vec<ActionResult>> {
    let mut results = Vec::with_capacity(actions.len());
    for action in &actions {
        let r = run_action(&state, &tab_id, action).await;
        let failed = !r.success;
        results.push(r);
        if failed {
            break;
        }
    }
    Ok(Json(results))
}

async fn run_action(state: &AppState, tab_id: &str, action: &Action) -> ActionResult {
    let inst_id = match state.instance_for_tab(tab_id).await {
        Some(id) => id,
        None => return ActionResult::err("tab not found"),
    };

    let instances = state.instances.read().await;
    let inst = match instances.get(&inst_id) {
        Some(i) => i,
        None => return ActionResult::err("instance not found"),
    };

    let tabs = inst.tabs.read().await;
    let page = match tabs.get(tab_id) {
        Some(p) => p,
        None => return ActionResult::err("tab not found"),
    };

    execute_single_action(page, action, state, tab_id).await
}

fn execute_single_action<'a>(
    page: &'a chromiumoxide::Page,
    action: &'a Action,
    _state: &'a AppState,
    _tab_id: &'a str,
) -> std::pin::Pin<Box<dyn std::future::Future<Output = ActionResult> + Send + 'a>> {
    Box::pin(async move {
    match action {
        Action::Click { ref_id } => {
            let idx = match parse_ref_id(ref_id) {
                Ok(i) => i,
                Err(e) => return ActionResult::err(e),
            };
            match page.evaluate(click_by_index_js(idx)).await {
                Ok(_) => ActionResult::ok(),
                Err(e) => ActionResult::err(format!("click failed: {e}")),
            }
        }
        Action::Type { ref_id, text } => {
            let idx = match parse_ref_id(ref_id) {
                Ok(i) => i,
                Err(e) => return ActionResult::err(e),
            };
            match page.evaluate(type_by_index_js(idx, text)).await {
                Ok(_) => ActionResult::ok(),
                Err(e) => ActionResult::err(format!("type failed: {e}")),
            }
        }
        Action::Fill { ref_id, text } => {
            let idx = match parse_ref_id(ref_id) {
                Ok(i) => i,
                Err(e) => return ActionResult::err(e),
            };
            match page.evaluate(fill_by_index_js(idx, text)).await {
                Ok(_) => ActionResult::ok(),
                Err(e) => ActionResult::err(format!("fill failed: {e}")),
            }
        }
        Action::Press { key, ref_id } => {
            if let Some(rid) = ref_id {
                let idx = match parse_ref_id(rid) {
                    Ok(i) => i,
                    Err(e) => return ActionResult::err(e),
                };
                if let Err(e) = page.evaluate(focus_by_index_js(idx)).await {
                    return ActionResult::err(format!("focus for press failed: {e}"));
                }
            }
            let escaped = key.replace('\'', "\\'");
            let js = format!(
                "document.activeElement.dispatchEvent(new KeyboardEvent('keydown', {{ key: '{escaped}', bubbles: true }})); \
                 document.activeElement.dispatchEvent(new KeyboardEvent('keyup', {{ key: '{escaped}', bubbles: true }}))"
            );
            match page.evaluate(js).await {
                Ok(_) => ActionResult::ok(),
                Err(e) => ActionResult::err(format!("press failed: {e}")),
            }
        }
        Action::Hover { ref_id } => {
            let idx = match parse_ref_id(ref_id) {
                Ok(i) => i,
                Err(e) => return ActionResult::err(e),
            };
            match page.evaluate(hover_by_index_js(idx)).await {
                Ok(_) => ActionResult::ok(),
                Err(e) => ActionResult::err(format!("hover failed: {e}")),
            }
        }
        Action::Focus { ref_id } => {
            let idx = match parse_ref_id(ref_id) {
                Ok(i) => i,
                Err(e) => return ActionResult::err(e),
            };
            match page.evaluate(focus_by_index_js(idx)).await {
                Ok(_) => ActionResult::ok(),
                Err(e) => ActionResult::err(format!("focus failed: {e}")),
            }
        }
        Action::Scroll { ref_id, pixels } => {
            let px = pixels.unwrap_or(300);
            let js = if let Some(rid) = ref_id {
                let idx = match parse_ref_id(rid) {
                    Ok(i) => i,
                    Err(e) => return ActionResult::err(e),
                };
                format!(
                    r#"(() => {{
                        const walker = document.createTreeWalker(document.body||document.documentElement, NodeFilter.SHOW_ELEMENT, null);
                        let node = walker.currentNode; let i = 0;
                        while (node) {{ if (i === {idx}) {{ node.scrollBy(0, {px}); return 'scrolled'; }} i++; node = walker.nextNode(); }}
                        throw new Error('not found');
                    }})()"#,
                    idx = idx,
                    px = px
                )
            } else {
                format!("window.scrollBy(0, {px}); 'scrolled'")
            };
            match page.evaluate(js).await {
                Ok(_) => ActionResult::ok(),
                Err(e) => ActionResult::err(format!("scroll failed: {e}")),
            }
        }
        Action::Select { ref_id, value } => {
            let idx = match parse_ref_id(ref_id) {
                Ok(i) => i,
                Err(e) => return ActionResult::err(e),
            };
            match page.evaluate(select_by_index_js(idx, value)).await {
                Ok(_) => ActionResult::ok(),
                Err(e) => ActionResult::err(format!("select failed: {e}")),
            }
        }
        Action::Wait { time } => {
            tokio::time::sleep(tokio::time::Duration::from_millis(*time)).await;
            ActionResult::ok()
        }
        Action::Batch { actions } => {
            for a in actions {
                let r = execute_single_action(page, a, _state, _tab_id).await;
                if !r.success {
                    return r;
                }
            }
            ActionResult::ok()
        }
    }
    })
}

// ── JS Evaluate ─────────────────────────────────────────────

#[derive(serde::Deserialize)]
struct EvalRequest {
    expression: String,
}

async fn evaluate_js(
    State(state): State<AppState>,
    Path(tab_id): Path<String>,
    Json(req): Json<EvalRequest>,
) -> ApiResult<EvalResponse> {
    let inst_id = resolve_tab(&state, &tab_id).await?;
    let instances = state.instances.read().await;
    let inst = instances.get(&inst_id).ok_or_else(|| api_err(StatusCode::NOT_FOUND, "instance gone"))?;
    let tabs = inst.tabs.read().await;
    let page = tabs.get(&tab_id).ok_or_else(|| api_err(StatusCode::NOT_FOUND, "tab gone"))?;

    let val = page
        .evaluate(req.expression.as_str())
        .await
        .map_err(|e| api_err(StatusCode::INTERNAL_SERVER_ERROR, &format!("evaluate: {e}")))?;
    let result: serde_json::Value = val
        .into_value()
        .unwrap_or(serde_json::Value::Null);
    Ok(Json(EvalResponse { result }))
}

// ── Screenshot ──────────────────────────────────────────────

async fn take_screenshot(
    State(state): State<AppState>,
    Path(tab_id): Path<String>,
) -> Result<impl IntoResponse, (StatusCode, Json<serde_json::Value>)> {
    let inst_id = resolve_tab(&state, &tab_id).await?;
    let instances = state.instances.read().await;
    let inst = instances.get(&inst_id).ok_or_else(|| api_err(StatusCode::NOT_FOUND, "instance gone"))?;
    let tabs = inst.tabs.read().await;
    let page = tabs.get(&tab_id).ok_or_else(|| api_err(StatusCode::NOT_FOUND, "tab gone"))?;

    let bytes = page
        .screenshot(
            chromiumoxide::page::ScreenshotParams::builder()
                .full_page(true)
                .build(),
        )
        .await
        .map_err(|e| api_err(StatusCode::INTERNAL_SERVER_ERROR, &format!("screenshot: {e}")))?;
    use base64::Engine as _;
    let b64 = base64::engine::general_purpose::STANDARD.encode(&bytes);
    let size = bytes.len();
    Ok(Json(ScreenshotResponse { format: "png", data: b64, size }))
}

// ── PDF Export ───────────────────────────────────────────────

async fn export_pdf(
    State(state): State<AppState>,
    Path(tab_id): Path<String>,
) -> Result<impl IntoResponse, (StatusCode, Json<serde_json::Value>)> {
    let inst_id = resolve_tab(&state, &tab_id).await?;
    let instances = state.instances.read().await;
    let inst = instances.get(&inst_id).ok_or_else(|| api_err(StatusCode::NOT_FOUND, "instance gone"))?;
    let tabs = inst.tabs.read().await;
    let page = tabs.get(&tab_id).ok_or_else(|| api_err(StatusCode::NOT_FOUND, "tab gone"))?;

    let params = chromiumoxide::cdp::browser_protocol::page::PrintToPdfParams::builder()
        .build();
    let response = page
        .execute(params)
        .await
        .map_err(|e| api_err(StatusCode::INTERNAL_SERVER_ERROR, &format!("pdf: {e}")))?;
    let data_str = format!("{:?}", response.result.data);
    Ok(Json(PdfResponse { format: "pdf", data: data_str }))
}

// ── Profiles ────────────────────────────────────────────────

async fn list_profiles(State(state): State<AppState>) -> impl IntoResponse {
    let profiles = state.profiles.read().await;
    let list: Vec<Profile> = profiles.values().cloned().collect();
    Json(serde_json::json!({ "profiles": list }))
}

async fn create_profile(
    State(state): State<AppState>,
    Json(req): Json<CreateProfileRequest>,
) -> ApiResult<serde_json::Value> {
    let profile = Profile::new(&req.name);
    let info = serde_json::json!({ "profile": &profile });
    state
        .profiles
        .write()
        .await
        .insert(profile.name.clone(), profile);
    Ok(Json(info))
}
