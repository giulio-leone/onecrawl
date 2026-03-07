//! HTTP route handlers for the Studio visual workflow builder.

use axum::extract::{Json, Path};
use axum::http::StatusCode;
use axum::response::Html;
use serde::{Deserialize, Serialize};

use crate::routes::{api_err, ApiResult};

use onecrawl_cdp::studio::{StudioProject, StudioWorkspace, WorkflowTemplate};

const WORKSPACE_DIR: &str = "~/.onecrawl/studio";

fn workspace() -> Result<StudioWorkspace, (StatusCode, Json<crate::routes::ErrorBody>)> {
    StudioWorkspace::new(WORKSPACE_DIR)
        .map_err(|e| api_err(StatusCode::INTERNAL_SERVER_ERROR, &e))
}

// ── HTML page ───────────────────────────────────────────────────────

pub async fn studio_page() -> Html<&'static str> {
    Html(crate::studio_html::studio_html())
}

// ── Templates ───────────────────────────────────────────────────────

pub async fn list_templates() -> Json<Vec<WorkflowTemplate>> {
    Json(StudioWorkspace::templates())
}

pub async fn get_template(Path(id): Path<String>) -> ApiResult<WorkflowTemplate> {
    let templates = StudioWorkspace::templates();
    templates
        .into_iter()
        .find(|t| t.id == id)
        .map(Json)
        .ok_or_else(|| api_err(StatusCode::NOT_FOUND, "template not found"))
}

// ── Projects ────────────────────────────────────────────────────────

pub async fn list_projects() -> ApiResult<Vec<StudioProject>> {
    let ws = workspace()?;
    ws.list_projects()
        .map(Json)
        .map_err(|e| api_err(StatusCode::INTERNAL_SERVER_ERROR, &e))
}

#[derive(Deserialize)]
pub struct SaveProjectRequest {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub workflow: serde_json::Value,
}

#[derive(Serialize)]
pub struct SaveProjectResponse {
    pub id: String,
    pub status: String,
}

pub async fn save_project(
    Json(req): Json<SaveProjectRequest>,
) -> ApiResult<SaveProjectResponse> {
    let ws = workspace()?;

    // Validate the project ID (alphanumeric, dash, underscore only)
    if req.id.is_empty()
        || req.id.len() > 128
        || !req
            .id
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-')
    {
        return Err(api_err(
            StatusCode::BAD_REQUEST,
            "Invalid project ID: only alphanumeric, dash, and underscore allowed",
        ));
    }

    // Check if project exists to preserve created_at
    let now = onecrawl_cdp::util::iso_now();

    let created_at = ws
        .load_project(&req.id)
        .ok()
        .map(|p| p.created_at)
        .unwrap_or(now.clone());

    let project = StudioProject {
        id: req.id.clone(),
        name: req.name,
        description: req.description,
        workflow: req.workflow,
        created_at,
        updated_at: now,
        last_run: None,
        run_count: 0,
    };

    ws.save_project(&project)
        .map_err(|e| api_err(StatusCode::INTERNAL_SERVER_ERROR, &e))?;

    Ok(Json(SaveProjectResponse {
        id: project.id,
        status: "saved".into(),
    }))
}

pub async fn get_project(Path(id): Path<String>) -> ApiResult<StudioProject> {
    let ws = workspace()?;
    ws.load_project(&id)
        .map(Json)
        .map_err(|e| api_err(StatusCode::NOT_FOUND, &e))
}

#[derive(Serialize)]
pub struct DeleteResponse {
    pub status: String,
}

pub async fn delete_project(Path(id): Path<String>) -> ApiResult<DeleteResponse> {
    let ws = workspace()?;
    ws.delete_project(&id)
        .map_err(|e| api_err(StatusCode::NOT_FOUND, &e))?;
    Ok(Json(DeleteResponse {
        status: "deleted".into(),
    }))
}

// ── Validate ────────────────────────────────────────────────────────

#[derive(Serialize)]
pub struct ValidateResponse {
    pub valid: bool,
    pub warnings: Vec<String>,
    pub error: Option<String>,
}

pub async fn validate_workflow(
    Json(workflow): Json<serde_json::Value>,
) -> Json<ValidateResponse> {
    match StudioWorkspace::validate_workflow(&workflow) {
        Ok(warnings) => Json(ValidateResponse {
            valid: warnings.is_empty(),
            warnings,
            error: None,
        }),
        Err(e) => Json(ValidateResponse {
            valid: false,
            warnings: vec![],
            error: Some(e),
        }),
    }
}

// ── Export/Import ───────────────────────────────────────────────────

pub async fn export_project(Path(id): Path<String>) -> ApiResult<serde_json::Value> {
    let ws = workspace()?;
    let project = ws
        .load_project(&id)
        .map_err(|e| api_err(StatusCode::NOT_FOUND, &e))?;
    Ok(Json(project.workflow))
}

#[derive(Deserialize)]
pub struct ImportRequest {
    pub name: String,
    pub workflow: serde_json::Value,
}

pub async fn import_workflow(
    Json(req): Json<ImportRequest>,
) -> ApiResult<StudioProject> {
    let ws = workspace()?;
    let wf_json = serde_json::to_string(&req.workflow)
        .map_err(|e| api_err(StatusCode::BAD_REQUEST, &e.to_string()))?;
    ws.import_workflow(&req.name, &wf_json)
        .map(Json)
        .map_err(|e| api_err(StatusCode::INTERNAL_SERVER_ERROR, &e))
}

// ── Validate ────────────────────────────────────────────────────────