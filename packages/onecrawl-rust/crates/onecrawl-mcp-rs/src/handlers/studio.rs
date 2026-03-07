//! Handler implementations for the `studio` super-tool (Visual Workflow Builder).

use rmcp::{ErrorData as McpError, model::*};

use crate::cdp_tools::*;
use crate::helpers::{json_ok, mcp_err};
use crate::OneCrawlMcp;

use onecrawl_cdp::studio::{StudioWorkspace, StudioProject};

const WORKSPACE_DIR: &str = "~/.onecrawl/studio";

impl OneCrawlMcp {
    fn studio_workspace(&self) -> Result<StudioWorkspace, McpError> {
        StudioWorkspace::new(WORKSPACE_DIR).map_err(|e| mcp_err(format!("workspace: {e}")))
    }

    pub(crate) async fn studio_templates(
        &self,
        _p: StudioTemplatesParams,
    ) -> Result<CallToolResult, McpError> {
        let templates = StudioWorkspace::templates();
        json_ok(&serde_json::json!({
            "action": "studio_templates",
            "templates": templates,
            "count": templates.len()
        }))
    }

    pub(crate) async fn studio_projects(
        &self,
        _p: StudioProjectsParams,
    ) -> Result<CallToolResult, McpError> {
        let ws = self.studio_workspace()?;
        let projects = ws
            .list_projects()
            .map_err(|e| mcp_err(format!("list projects: {e}")))?;
        let count = projects.len();
        json_ok(&serde_json::json!({
            "action": "studio_projects",
            "projects": projects,
            "count": count
        }))
    }

    pub(crate) async fn studio_save(
        &self,
        p: StudioSaveParams,
    ) -> Result<CallToolResult, McpError> {
        let ws = self.studio_workspace()?;

        let now = simple_iso_now();
        let created_at = ws
            .load_project(&p.id)
            .ok()
            .map(|proj| proj.created_at)
            .unwrap_or_else(|| now.clone());

        let project = StudioProject {
            id: p.id.clone(),
            name: p.name,
            description: p.description,
            workflow: p.workflow,
            created_at,
            updated_at: now,
            last_run: None,
            run_count: 0,
        };
        ws.save_project(&project)
            .map_err(|e| mcp_err(format!("save: {e}")))?;

        json_ok(&serde_json::json!({
            "action": "studio_save",
            "id": project.id,
            "status": "saved"
        }))
    }

    pub(crate) async fn studio_load(
        &self,
        p: StudioLoadParams,
    ) -> Result<CallToolResult, McpError> {
        let ws = self.studio_workspace()?;
        let project = ws
            .load_project(&p.id)
            .map_err(|e| mcp_err(format!("load: {e}")))?;
        json_ok(&serde_json::json!({
            "action": "studio_load",
            "project": project
        }))
    }

    pub(crate) async fn studio_delete(
        &self,
        p: StudioDeleteParams,
    ) -> Result<CallToolResult, McpError> {
        let ws = self.studio_workspace()?;
        ws.delete_project(&p.id)
            .map_err(|e| mcp_err(format!("delete: {e}")))?;
        json_ok(&serde_json::json!({
            "action": "studio_delete",
            "id": p.id,
            "status": "deleted"
        }))
    }

    pub(crate) async fn studio_validate(
        &self,
        p: StudioValidateParams,
    ) -> Result<CallToolResult, McpError> {
        match StudioWorkspace::validate_workflow(&p.workflow) {
            Ok(warnings) => json_ok(&serde_json::json!({
                "action": "studio_validate",
                "valid": warnings.is_empty(),
                "warnings": warnings
            })),
            Err(e) => json_ok(&serde_json::json!({
                "action": "studio_validate",
                "valid": false,
                "error": e
            })),
        }
    }

    pub(crate) async fn studio_export(
        &self,
        p: StudioExportParams,
    ) -> Result<CallToolResult, McpError> {
        let ws = self.studio_workspace()?;
        let wf = ws
            .export_workflow(&p.id)
            .map_err(|e| mcp_err(format!("export: {e}")))?;
        json_ok(&serde_json::json!({
            "action": "studio_export",
            "id": p.id,
            "workflow": serde_json::from_str::<serde_json::Value>(&wf).unwrap_or_default()
        }))
    }

    pub(crate) async fn studio_import(
        &self,
        p: StudioImportParams,
    ) -> Result<CallToolResult, McpError> {
        let ws = self.studio_workspace()?;
        let wf_json = serde_json::to_string(&p.workflow)
            .map_err(|e| mcp_err(format!("import: {e}")))?;
        let project = ws
            .import_workflow(&p.name, &wf_json)
            .map_err(|e| mcp_err(format!("import: {e}")))?;
        json_ok(&serde_json::json!({
            "action": "studio_import",
            "project": project
        }))
    }
}

fn simple_iso_now() -> String {
    use std::time::SystemTime;
    let d = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap_or_default();
    let secs = d.as_secs();
    let (s, m, h) = (secs % 60, (secs / 60) % 60, (secs / 3600) % 24);
    let days = secs / 86400;
    let (y, mo, day) = days_to_ymd(days);
    format!("{y:04}-{mo:02}-{day:02}T{h:02}:{m:02}:{s:02}Z")
}

fn days_to_ymd(days: u64) -> (u64, u64, u64) {
    let days = days + 719_468;
    let era = days / 146_097;
    let doe = days - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146_096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = if m <= 2 { y + 1 } else { y };
    (y, m, d)
}
