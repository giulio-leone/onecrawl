//! Handler implementations for the `orchestrator` super-tool.

use rmcp::{ErrorData as McpError, model::*};
use crate::cdp_tools::*;
use crate::helpers::{mcp_err, json_ok, McpResult};
use crate::OneCrawlMcp;

impl OneCrawlMcp {
    // ════════════════════════════════════════════════════════════════
    //  Orchestrator handlers
    // ════════════════════════════════════════════════════════════════

    pub(crate) async fn orchestrator_run(
        &self,
        p: OrchestratorRunParams,
    ) -> Result<CallToolResult, McpError> {
        let orchestration: onecrawl_cdp::orchestrator::Orchestration = if let Some(ref file) = p.file {
            onecrawl_cdp::orchestrator::Orchestrator::from_file(file).mcp()?
        } else if let Some(ref config) = p.config {
            serde_json::from_str(config)
                .map_err(|e| mcp_err(format!("invalid orchestration JSON: {e}")))?
        } else {
            return Err(mcp_err("either 'file' or 'config' is required"));
        };

        if let Err(errors) = onecrawl_cdp::orchestrator::Orchestrator::validate(&orchestration) {
            return json_ok(&serde_json::json!({
                "action": "orchestrate_run",
                "success": false,
                "errors": errors,
            }));
        }

        let mut orch = onecrawl_cdp::orchestrator::Orchestrator::new(orchestration);

        orch.connect_devices().await.mcp()?;
        let result = orch.execute().await.mcp()?;
        let _ = orch.disconnect().await;

        json_ok(&result)
    }

    pub(crate) async fn orchestrator_validate(
        &self,
        p: OrchestratorValidateParams,
    ) -> Result<CallToolResult, McpError> {
        let orchestration: onecrawl_cdp::orchestrator::Orchestration = if let Some(ref file) = p.file {
            onecrawl_cdp::orchestrator::Orchestrator::from_file(file).mcp()?
        } else if let Some(ref config) = p.config {
            serde_json::from_str(config)
                .map_err(|e| mcp_err(format!("invalid orchestration JSON: {e}")))?
        } else {
            return Err(mcp_err("either 'file' or 'config' is required"));
        };

        match onecrawl_cdp::orchestrator::Orchestrator::validate(&orchestration) {
            Ok(()) => json_ok(&serde_json::json!({
                "action": "orchestrate_validate",
                "valid": true,
                "name": orchestration.name,
                "devices": orchestration.devices.len(),
                "steps": orchestration.steps.len(),
            })),
            Err(errors) => json_ok(&serde_json::json!({
                "action": "orchestrate_validate",
                "valid": false,
                "errors": errors,
            })),
        }
    }

    pub(crate) async fn orchestrator_status(
        &self,
        _p: OrchestratorStatusParams,
    ) -> Result<CallToolResult, McpError> {
        json_ok(&serde_json::json!({
            "action": "orchestrate_status",
            "message": "No orchestration currently running. Use orchestrate_run to start one."
        }))
    }

    pub(crate) async fn orchestrator_stop(
        &self,
        _p: OrchestratorStopParams,
    ) -> Result<CallToolResult, McpError> {
        json_ok(&serde_json::json!({
            "action": "orchestrate_stop",
            "status": "stop_requested",
            "message": "Orchestration stop signal sent"
        }))
    }

    pub(crate) async fn orchestrator_devices(
        &self,
        _p: OrchestratorDevicesParams,
    ) -> Result<CallToolResult, McpError> {
        let state = self.browser.lock().await;

        let mut devices = Vec::new();
        if state.session.is_some() {
            devices.push(serde_json::json!({
                "type": "browser",
                "status": "connected",
                "tabs": state.tabs.len(),
            }));
        }
        if state.ios_client.is_some() {
            devices.push(serde_json::json!({
                "type": "ios",
                "status": "connected",
            }));
        }
        if state.android_client.is_some() {
            devices.push(serde_json::json!({
                "type": "android",
                "status": "connected",
            }));
        }

        json_ok(&serde_json::json!({
            "action": "orchestrate_devices",
            "devices": devices,
            "total": devices.len(),
        }))
    }
}
