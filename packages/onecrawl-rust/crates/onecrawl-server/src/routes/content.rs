use axum::extract::{Json, Path, Query, State};
use axum::http::{HeaderMap, StatusCode};
use serde::Serialize;
use std::sync::Arc;

use super::{ApiResult, api_err, get_tab_page};
use crate::snapshot::{PageSnapshot, SnapshotElement, SnapshotQuery, SNAPSHOT_JS, TEXT_EXTRACT_JS};
use crate::state::AppState;

#[derive(Serialize)]
pub(crate) struct TextResponse {
    url: String,
    text: String,
}

#[derive(Serialize)]
pub(crate) struct UrlResponse {
    url: String,
}

#[derive(Serialize)]
pub(crate) struct TitleResponse {
    title: String,
}

#[derive(Serialize)]
pub(crate) struct HtmlResponse {
    url: String,
    html: String,
}

#[derive(Serialize)]
pub(crate) struct EvalResponse {
    result: serde_json::Value,
}

#[derive(Serialize)]
pub(crate) struct ScreenshotResponse {
    format: &'static str,
    data: String,
    size: usize,
}

#[derive(Serialize)]
pub(crate) struct PdfResponse {
    format: &'static str,
    data: String,
}

pub(crate) struct SnapshotResponse {
    url: String,
    title: String,
    elements: Arc<Vec<SnapshotElement>>,
    compact: bool,
}

impl serde::Serialize for SnapshotResponse {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        use serde::ser::SerializeMap;
        let mut map = serializer.serialize_map(Some(3))?;
        map.serialize_entry("url", &self.url)?;
        map.serialize_entry("title", &self.title)?;
        if self.compact {
            let compact: Vec<(&str, &str, &str)> = self
                .elements
                .iter()
                .map(|e| (e.ref_id.as_str(), e.role.as_str(), e.name.as_str()))
                .collect();
            map.serialize_entry("elements", &compact)?;
        } else {
            map.serialize_entry("elements", &*self.elements)?;
        }
        map.end()
    }
}

pub async fn get_snapshot(
    State(state): State<AppState>,
    Path(tab_id): Path<String>,
    headers: HeaderMap,
    Query(query): Query<SnapshotQuery>,
) -> ApiResult<SnapshotResponse> {
    let owner = headers.get("x-agent-owner").and_then(|v| v.to_str().ok());
    let page = get_tab_page(&state, &tab_id, owner).await?;

    let result_str: String = page
        .evaluate(SNAPSHOT_JS)
        .await
        .map_err(|e| api_err(StatusCode::INTERNAL_SERVER_ERROR, &format!("snapshot eval: {e}")))?
        .into_value()
        .map_err(|e| api_err(StatusCode::INTERNAL_SERVER_ERROR, &format!("snapshot parse: {e}")))?;

    let mut snapshot: PageSnapshot = serde_json::from_str(&result_str)
        .map_err(|e| api_err(StatusCode::INTERNAL_SERVER_ERROR, &format!("snapshot json: {e}")))?;

    if query.filter.as_deref() == Some("interactive") {
        snapshot.elements.retain(|e| e.interactive);
    }

    let elements = Arc::new(snapshot.elements);
    state.cache_snapshot(tab_id, Arc::clone(&elements)).await;

    Ok(Json(SnapshotResponse {
        url: snapshot.url,
        title: snapshot.title,
        elements,
        compact: query.format.as_deref() == Some("compact"),
    }))
}

pub async fn get_text(
    State(state): State<AppState>,
    Path(tab_id): Path<String>,
    headers: HeaderMap,
) -> ApiResult<TextResponse> {
    let owner = headers.get("x-agent-owner").and_then(|v| v.to_str().ok());
    let page = get_tab_page(&state, &tab_id, owner).await?;
    let text: String = page
        .evaluate(TEXT_EXTRACT_JS)
        .await
        .map_err(|e| api_err(StatusCode::INTERNAL_SERVER_ERROR, &format!("text eval: {e}")))?
        .into_value()
        .map_err(|e| api_err(StatusCode::INTERNAL_SERVER_ERROR, &format!("text parse: {e}")))?;
    let url = page.url().await.ok().flatten().unwrap_or_default();
    Ok(Json(TextResponse { url, text }))
}

pub async fn get_url(
    State(state): State<AppState>,
    Path(tab_id): Path<String>,
    headers: HeaderMap,
) -> ApiResult<UrlResponse> {
    let owner = headers.get("x-agent-owner").and_then(|v| v.to_str().ok());
    let page = get_tab_page(&state, &tab_id, owner).await?;
    let url = page.url().await.ok().flatten().unwrap_or_default();
    Ok(Json(UrlResponse { url }))
}

pub async fn get_title(
    State(state): State<AppState>,
    Path(tab_id): Path<String>,
    headers: HeaderMap,
) -> ApiResult<TitleResponse> {
    let owner = headers.get("x-agent-owner").and_then(|v| v.to_str().ok());
    let page = get_tab_page(&state, &tab_id, owner).await?;
    let title: String = page
        .evaluate("document.title")
        .await
        .ok()
        .and_then(|v| v.into_value().ok())
        .unwrap_or_default();
    Ok(Json(TitleResponse { title }))
}

pub async fn get_html(
    State(state): State<AppState>,
    Path(tab_id): Path<String>,
    headers: HeaderMap,
) -> ApiResult<HtmlResponse> {
    let owner = headers.get("x-agent-owner").and_then(|v| v.to_str().ok());
    let page = get_tab_page(&state, &tab_id, owner).await?;
    let html: String = page
        .evaluate("document.documentElement.outerHTML")
        .await
        .ok()
        .and_then(|v| v.into_value().ok())
        .unwrap_or_default();
    let url = page.url().await.ok().flatten().unwrap_or_default();
    Ok(Json(HtmlResponse { url, html }))
}

#[derive(serde::Deserialize)]
pub(crate) struct EvalRequest {
    expression: String,
}

pub async fn evaluate_js(
    State(state): State<AppState>,
    Path(tab_id): Path<String>,
    headers: HeaderMap,
    Json(req): Json<EvalRequest>,
) -> ApiResult<EvalResponse> {
    let owner = headers.get("x-agent-owner").and_then(|v| v.to_str().ok());
    let page = get_tab_page(&state, &tab_id, owner).await?;
    let val = page
        .evaluate(req.expression.as_str())
        .await
        .map_err(|e| api_err(StatusCode::INTERNAL_SERVER_ERROR, &format!("evaluate: {e}")))?;
    let result: serde_json::Value = val
        .into_value()
        .unwrap_or(serde_json::Value::Null);
    Ok(Json(EvalResponse { result }))
}

pub async fn take_screenshot(
    State(state): State<AppState>,
    Path(tab_id): Path<String>,
    headers: HeaderMap,
) -> ApiResult<ScreenshotResponse> {
    let owner = headers.get("x-agent-owner").and_then(|v| v.to_str().ok());
    let page = get_tab_page(&state, &tab_id, owner).await?;
    let bytes = page
        .screenshot(
            onecrawl_browser::page::ScreenshotParams::builder()
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

pub async fn export_pdf(
    State(state): State<AppState>,
    Path(tab_id): Path<String>,
    headers: HeaderMap,
) -> ApiResult<PdfResponse> {
    let owner = headers.get("x-agent-owner").and_then(|v| v.to_str().ok());
    let page = get_tab_page(&state, &tab_id, owner).await?;
    let params = onecrawl_browser::cdp::browser_protocol::page::PrintToPdfParams::builder()
        .build();
    let response = page
        .execute(params)
        .await
        .map_err(|e| api_err(StatusCode::INTERNAL_SERVER_ERROR, &format!("pdf: {e}")))?;
    let data_str: String = response.result.data.into();
    Ok(Json(PdfResponse { format: "pdf", data: data_str }))
}
