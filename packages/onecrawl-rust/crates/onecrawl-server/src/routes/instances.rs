use axum::extract::{Json, Path, State};
use axum::http::StatusCode;
use axum::response::IntoResponse;
use serde::Serialize;

use super::{ApiResult, api_err};
use crate::instance::{CreateInstanceRequest, InstanceInfo};
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
    let instances = state.instances.read().await;
    let mut infos = Vec::with_capacity(instances.len());
    for inst in instances.values() {
        infos.push(inst.info().await);
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
    let mut instances = state.instances.write().await;
    let inst = instances
        .remove(&id)
        .ok_or_else(|| api_err(StatusCode::NOT_FOUND, "instance not found"))?;
    {
        let tabs = inst.tabs.read().await;
        let mut index = state.tab_index.write().await;
        for tid in tabs.keys() {
            index.remove(tid);
        }
    }
    Ok(Json(StoppedResponse { stopped: id }))
}
