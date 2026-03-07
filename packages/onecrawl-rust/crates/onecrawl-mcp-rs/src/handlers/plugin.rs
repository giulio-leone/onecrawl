//! Handler implementations for the `plugins` super-tool.

use rmcp::{ErrorData as McpError, model::*};
use crate::cdp_tools::*;
use crate::helpers::{json_ok, mcp_err, McpResult};
use crate::OneCrawlMcp;

fn plugins_dir() -> String {
    onecrawl_cdp::default_plugins_dir()
        .to_string_lossy()
        .into_owned()
}

impl OneCrawlMcp {
    // ════════════════════════════════════════════════════════════════
    //  Plugin handlers
    // ════════════════════════════════════════════════════════════════

    pub(crate) fn plugin_install(
        &self,
        p: PluginInstallParams,
    ) -> Result<CallToolResult, McpError> {
        let mut registry = onecrawl_cdp::PluginRegistry::new(&plugins_dir())
            .map_err(|e| mcp_err(e))?;

        let plugin = registry.install_local(&p.path).map_err(|e| mcp_err(e))?;

        json_ok(&serde_json::json!({
            "action": "plugin_install",
            "name": plugin.manifest.name,
            "version": plugin.manifest.version,
            "description": plugin.manifest.description,
            "status": "installed",
            "commands": plugin.manifest.commands.len(),
            "actions": plugin.manifest.actions.len(),
            "hooks": plugin.manifest.hooks.len(),
        }))
    }

    pub(crate) fn plugin_uninstall(
        &self,
        p: PluginUninstallParams,
    ) -> Result<CallToolResult, McpError> {
        let mut registry = onecrawl_cdp::PluginRegistry::new(&plugins_dir())
            .map_err(|e| mcp_err(e))?;

        registry.uninstall(&p.name).map_err(|e| mcp_err(e))?;

        json_ok(&serde_json::json!({
            "action": "plugin_uninstall",
            "name": p.name,
            "status": "uninstalled",
        }))
    }

    pub(crate) fn plugin_enable(
        &self,
        p: PluginEnableParams,
    ) -> Result<CallToolResult, McpError> {
        let mut registry = onecrawl_cdp::PluginRegistry::new(&plugins_dir())
            .map_err(|e| mcp_err(e))?;

        registry.enable(&p.name).map_err(|e| mcp_err(e))?;

        json_ok(&serde_json::json!({
            "action": "plugin_enable",
            "name": p.name,
            "status": "active",
        }))
    }

    pub(crate) fn plugin_disable(
        &self,
        p: PluginDisableParams,
    ) -> Result<CallToolResult, McpError> {
        let mut registry = onecrawl_cdp::PluginRegistry::new(&plugins_dir())
            .map_err(|e| mcp_err(e))?;

        registry.disable(&p.name).map_err(|e| mcp_err(e))?;

        json_ok(&serde_json::json!({
            "action": "plugin_disable",
            "name": p.name,
            "status": "disabled",
        }))
    }

    pub(crate) fn plugin_list(&self) -> Result<CallToolResult, McpError> {
        let registry = onecrawl_cdp::PluginRegistry::new(&plugins_dir())
            .map_err(|e| mcp_err(e))?;

        let plugins: Vec<serde_json::Value> = registry
            .list()
            .iter()
            .map(|p| {
                serde_json::json!({
                    "name": p.manifest.name,
                    "version": p.manifest.version,
                    "description": p.manifest.description,
                    "status": p.status,
                    "commands": p.manifest.commands.len(),
                    "actions": p.manifest.actions.len(),
                    "hooks": p.manifest.hooks.len(),
                    "installed_at": p.installed_at,
                })
            })
            .collect();

        json_ok(&serde_json::json!({
            "action": "plugin_list",
            "plugins": plugins,
            "count": plugins.len(),
        }))
    }

    pub(crate) fn plugin_info(
        &self,
        p: PluginInfoParams,
    ) -> Result<CallToolResult, McpError> {
        let registry = onecrawl_cdp::PluginRegistry::new(&plugins_dir())
            .map_err(|e| mcp_err(e))?;

        let plugin = registry
            .get(&p.name)
            .ok_or_else(|| mcp_err(format!("plugin '{}' not found", p.name)))?;

        json_ok(&serde_json::json!({
            "action": "plugin_info",
            "manifest": plugin.manifest,
            "path": plugin.path,
            "status": plugin.status,
            "installed_at": plugin.installed_at,
            "config": plugin.config,
        }))
    }

    pub(crate) fn plugin_create(
        &self,
        p: PluginCreateParams,
    ) -> Result<CallToolResult, McpError> {
        let registry = onecrawl_cdp::PluginRegistry::new(&plugins_dir())
            .map_err(|e| mcp_err(e))?;

        let path = p.path.unwrap_or_else(|| format!("./{}", p.name));
        registry
            .create_scaffold(&p.name, &path)
            .map_err(|e| mcp_err(e))?;

        let templates: Vec<serde_json::Value> = onecrawl_cdp::builtin_templates()
            .iter()
            .map(|(name, desc)| serde_json::json!({"name": name, "description": desc}))
            .collect();

        json_ok(&serde_json::json!({
            "action": "plugin_create",
            "name": p.name,
            "path": path,
            "status": "created",
            "available_templates": templates,
        }))
    }

    pub(crate) async fn plugin_execute(
        &self,
        p: PluginExecuteParams,
    ) -> Result<CallToolResult, McpError> {
        let registry = onecrawl_cdp::PluginRegistry::new(&plugins_dir())
            .map_err(|e| mcp_err(e))?;

        let result = registry
            .execute_action(&p.plugin, &p.action, p.params)
            .await
            .mcp()?;

        json_ok(&result)
    }

    pub(crate) fn plugin_configure(
        &self,
        p: PluginConfigureParams,
    ) -> Result<CallToolResult, McpError> {
        let mut registry = onecrawl_cdp::PluginRegistry::new(&plugins_dir())
            .map_err(|e| mcp_err(e))?;

        registry
            .configure(&p.name, p.config.clone())
            .map_err(|e| mcp_err(e))?;

        json_ok(&serde_json::json!({
            "action": "plugin_configure",
            "name": p.name,
            "config": p.config,
            "status": "configured",
        }))
    }
}
