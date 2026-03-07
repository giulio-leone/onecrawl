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
    onecrawl_cdp::util::iso_now()
}
