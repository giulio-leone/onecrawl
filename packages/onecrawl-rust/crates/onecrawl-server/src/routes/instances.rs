use axum::extract::{Json, Path, State};
use axum::http::StatusCode;
use axum::response::IntoResponse;
use serde::Serialize;

use super::{ApiResult, api_err};
use crate::instance::{CreateInstanceRequest, InstanceInfo, TabSummary};
use crate::state::AppState;

#[derive(Serialize)]
pub(crate) struct InstanceResponse {
    instance: InstanceInfo,
}

#[derive(Serialize)]
pub(crate) struct InstancesResponse {
    instances: Vec<InstanceInfo>,
}

#[derive(Serialize)]
pub(crate) struct StoppedResponse {
    stopped: String,
}

pub async fn create_instance(
    State(state): State<AppState>,
    Json(req): Json<CreateInstanceRequest>,
) -> ApiResult<InstanceResponse> {
    let headless = req.headless.unwrap_or(true);
    let id = format!("inst_{}", uuid::Uuid::new_v4().as_simple());

    let mut port_guard = state.next_instance_port.write().await;
    let port = *port_guard;
    *port_guard += 1;
    drop(port_guard);

    let user_data_dir = if let Some(ref profile_name) = req.profile {
        let profiles = state.profiles.read().await;
        profiles.get(profile_name).map(|p| p.user_data_dir())
    } else {
        None
    };

    let instance = crate::instance::Instance::launch(id.clone(), headless, port, req.profile.clone(), user_data_dir)
        .await
        .map_err(|e| api_err(StatusCode::INTERNAL_SERVER_ERROR, &e))?;

    let info = instance.info().await;
    state.instances.write().await.insert(id, instance);

    Ok(Json(InstanceResponse { instance: info }))
}

pub async fn list_instances(State(state): State<AppState>) -> impl IntoResponse {
    // Collect instance data and page handles inside the lock, then drop it
    let instance_data = {
        let instances = state.instances.read().await;
        let mut data = Vec::with_capacity(instances.len());
        for inst in instances.values() {
            let pages: Vec<_> = {
                let tabs = inst.tabs.read().await;
                tabs.iter().map(|(tid, p)| (tid.clone(), p.clone())).collect()
            };
            data.push((
                inst.id.clone(),
                inst.profile.clone(),
                inst.headless,
                inst.port,
                inst.start_time.clone(),
                pages,
            ));
        }
        data
    }; // instances lock dropped

    let mut infos = Vec::with_capacity(instance_data.len());
    for (id, profile, headless, port, start_time, pages) in instance_data {
        let futs = pages.into_iter().map(|(tid, page)| async move {
            let url = page.url().await.ok().flatten().unwrap_or_default();
            let title: String = page
                .evaluate("document.title")
                .await
                .ok()
                .and_then(|v| v.into_value().ok())
                .unwrap_or_default();
            TabSummary { id: tid, url, title }
        });
        let tab_summaries = futures::future::join_all(futs).await;
        infos.push(InstanceInfo {
            id,
            profile,
            headless,
            status: "running",
            port,
            start_time,
            tabs: tab_summaries,
        });
    }
    Json(InstancesResponse { instances: infos })
}

pub async fn get_instance(
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

pub async fn stop_instance(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> ApiResult<StoppedResponse> {
    let tab_ids = {
        let mut instances = state.instances.write().await;
        let inst = instances
            .remove(&id)
            .ok_or_else(|| api_err(StatusCode::NOT_FOUND, "instance not found"))?;
        let ids: Vec<String> = inst.tabs.read().await.keys().cloned().collect();
        ids
        // inst dropped here (browser closes)
    }; // instances write lock dropped

    for tid in &tab_ids {
        state.unregister_tab(tid).await;
    }

    Ok(Json(StoppedResponse { stopped: id }))
}
