use axum::extract::{Json, Path, State};
use axum::http::{HeaderMap, StatusCode};
use axum::response::IntoResponse;
use serde::Serialize;

use super::{ApiResult, api_err};
use crate::state::AppState;
use crate::tab::{NavigateRequest, OpenTabRequest, TabInfo};

#[derive(Serialize)]
pub(crate) struct TabResponse {
    tab: TabInfo,
}

#[derive(Serialize)]
pub(crate) struct TabsResponse {
    tabs: Vec<TabInfo>,
}

#[derive(Serialize)]
pub(crate) struct NavigateResponse {
    url: String,
    title: String,
}

pub async fn open_tab(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Json(req): Json<OpenTabRequest>,
) -> ApiResult<TabResponse> {
    let url_str = req.url.as_deref().unwrap_or("about:blank");

    // Hold the lock only for new_page + tab insertion; drop before CDP I/O
    let (page, tab_id) = {
        let instances = state.instances.read().await;
        let inst = instances
            .get(&id)
            .ok_or_else(|| api_err(StatusCode::NOT_FOUND, "instance not found"))?;

        let page = inst
            .browser
            .new_page(url_str)
            .await
            .map_err(|e| api_err(StatusCode::INTERNAL_SERVER_ERROR, &format!("new_page: {e}")))?;

        let mut counter = inst.tab_counter.write().await;
        *counter += 1;
        let tab_id = format!("tab_{}_{}", inst.id, counter);
        drop(counter);

        inst.tabs.write().await.insert(tab_id.clone(), page.clone());
        (page, tab_id)
    }; // instances read lock dropped

    // CDP network I/O outside the lock
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

    state.register_tab(&tab_id, &id).await;

    Ok(Json(TabResponse { tab: info }))
}

pub async fn get_instance_tabs(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> ApiResult<TabsResponse> {
    let instances = state.instances.read().await;
    let inst = instances
        .get(&id)
        .ok_or_else(|| api_err(StatusCode::NOT_FOUND, "instance not found"))?;

    let pages: Vec<(String, onecrawl_browser::Page)> = {
        let tabs = inst.tabs.read().await;
        tabs.iter().map(|(tid, p)| (tid.clone(), p.clone())).collect()
    };

    let futs = pages.into_iter().map(|(tid, page)| {
        let instance_id = id.clone();
        async move {
            let url = page.url().await.ok().flatten().unwrap_or_default();
            let title: String = page
                .evaluate("document.title")
                .await
                .ok()
                .and_then(|v| v.into_value().ok())
                .unwrap_or_default();
            TabInfo { id: tid, url, title, instance_id }
        }
    });
    let infos = futures::future::join_all(futs).await;

    Ok(Json(TabsResponse { tabs: infos }))
}

pub async fn list_all_tabs(State(state): State<AppState>) -> impl IntoResponse {
    let page_handles: Vec<(String, String, onecrawl_browser::Page)> = {
        let instances = state.instances.read().await;
        let mut handles = Vec::new();
        for inst in instances.values() {
            let tabs = inst.tabs.read().await;
            for (tid, page) in tabs.iter() {
                handles.push((tid.clone(), inst.id.clone(), page.clone()));
            }
        }
        handles
    };

    let futs = page_handles.into_iter().map(|(tid, inst_id, page)| async move {
        let url = page.url().await.ok().flatten().unwrap_or_default();
        let title: String = page
            .evaluate("document.title")
            .await
            .ok()
            .and_then(|v| v.into_value().ok())
            .unwrap_or_default();
        TabInfo { id: tid, url, title, instance_id: inst_id }
    });
    let all_tabs = futures::future::join_all(futs).await;

    Json(TabsResponse { tabs: all_tabs })
}

pub async fn navigate_tab(
    State(state): State<AppState>,
    Path(tab_id): Path<String>,
    headers: HeaderMap,
    Json(req): Json<NavigateRequest>,
) -> ApiResult<NavigateResponse> {
    let owner = headers.get("x-agent-owner").and_then(|v| v.to_str().ok());
    let page = super::get_tab_page(&state, &tab_id, owner).await?;
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
