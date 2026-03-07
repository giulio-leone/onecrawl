//! Handler implementations for the `durable` super-tool.

use rmcp::{ErrorData as McpError, model::*};
use crate::cdp_tools::*;
use crate::helpers::{mcp_err, ensure_page, json_ok, text_ok, McpResult};
use crate::OneCrawlMcp;

impl OneCrawlMcp {
    // ════════════════════════════════════════════════════════════════
    //  Durable Session handlers
    // ════════════════════════════════════════════════════════════════

    pub(crate) async fn durable_start(
        &self,
        p: DurableStartParams,
    ) -> Result<CallToolResult, McpError> {
        let page = ensure_page(&self.browser).await?;

        let on_crash = match p.on_crash.as_deref() {
            Some("stop") => onecrawl_cdp::CrashPolicy::Stop,
            Some("notify") => onecrawl_cdp::CrashPolicy::Notify,
            _ => onecrawl_cdp::CrashPolicy::Restart,
        };

        let mut config = onecrawl_cdp::DurableConfig {
            name: p.name.clone(),
            on_crash,
            ..onecrawl_cdp::DurableConfig::default()
        };

        if let Some(interval) = p.checkpoint_interval_secs {
            config.checkpoint_interval_secs = interval;
        }
        if let Some(path) = &p.state_path {
            config.state_path = std::path::PathBuf::from(path);
        }
        if let Some(ar) = p.auto_reconnect {
            config.auto_reconnect = ar;
        }
        if let Some(max) = p.max_reconnect_attempts {
            config.max_reconnect_attempts = max;
        }
        if let Some(max) = p.max_uptime_secs {
            config.max_uptime_secs = Some(max);
        }
        if let Some(pa) = p.persist_auth {
            config.persist_auth = pa;
        }

        let mut session = onecrawl_cdp::DurableSession::new(config.clone());

        // Perform an initial checkpoint
        let state = session.checkpoint(&page).await.mcp()?;

        json_ok(&serde_json::json!({
            "action": "durable_start",
            "name": p.name,
            "status": "running",
            "checkpoint_interval_secs": config.checkpoint_interval_secs,
            "auto_reconnect": config.auto_reconnect,
            "state_path": config.state_path.to_string_lossy(),
            "initial_checkpoint": state.last_checkpoint,
        }))
    }

    pub(crate) async fn durable_stop(
        &self,
        p: DurableStopParams,
    ) -> Result<CallToolResult, McpError> {
        let state_dir = onecrawl_cdp::DurableSession::default_state_dir();
        let mut state = onecrawl_cdp::DurableSession::get_status(&state_dir, &p.name)
            .map_err(|e| mcp_err(format!("durable_stop: {e}")))?;

        // Perform a final checkpoint if possible
        if let Ok(page) = ensure_page(&self.browser).await {
            let config = onecrawl_cdp::DurableConfig {
                name: p.name.clone(),
                state_path: state_dir.clone(),
                ..onecrawl_cdp::DurableConfig::default()
            };
            let mut durable = onecrawl_cdp::DurableSession::new(config);
            let _ = durable.checkpoint(&page).await;
            state = durable.state.clone();
        }

        state.status = onecrawl_cdp::DurableStatus::Stopped;
        let json = serde_json::to_string_pretty(&state)
            .map_err(|e| mcp_err(format!("serialize: {e}")))?;
        let path = state_dir.join(format!("{}.state", p.name));
        std::fs::write(&path, &json).map_err(|e| mcp_err(format!("write: {e}")))?;

        text_ok(format!("durable session '{}' stopped", p.name))
    }

    pub(crate) async fn durable_checkpoint(
        &self,
        p: DurableCheckpointParams,
    ) -> Result<CallToolResult, McpError> {
        let page = ensure_page(&self.browser).await?;
        let state_dir = onecrawl_cdp::DurableSession::default_state_dir();

        let config = onecrawl_cdp::DurableConfig {
            name: p.name.clone(),
            state_path: state_dir,
            ..onecrawl_cdp::DurableConfig::default()
        };
        let mut session = onecrawl_cdp::DurableSession::new(config);
        let state = session.checkpoint(&page).await.mcp()?;

        json_ok(&serde_json::json!({
            "action": "durable_checkpoint",
            "name": p.name,
            "last_checkpoint": state.last_checkpoint,
            "url": state.url,
            "cookies_count": state.cookies.len(),
            "local_storage_count": state.local_storage.len(),
            "session_storage_count": state.session_storage.len(),
        }))
    }

    pub(crate) async fn durable_restore(
        &self,
        p: DurableRestoreParams,
    ) -> Result<CallToolResult, McpError> {
        let page = ensure_page(&self.browser).await?;
        let state_dir = onecrawl_cdp::DurableSession::default_state_dir();

        let config = onecrawl_cdp::DurableConfig {
            name: p.name.clone(),
            state_path: state_dir,
            ..onecrawl_cdp::DurableConfig::default()
        };
        let mut session = onecrawl_cdp::DurableSession::new(config);
        session.restore(&page).await.mcp()?;

        json_ok(&serde_json::json!({
            "action": "durable_restore",
            "name": p.name,
            "restored_url": session.state.url,
            "cookies_count": session.state.cookies.len(),
            "local_storage_count": session.state.local_storage.len(),
            "session_storage_count": session.state.session_storage.len(),
            "status": "running",
        }))
    }

    pub(crate) async fn durable_status(
        &self,
        p: DurableStatusParams,
    ) -> Result<CallToolResult, McpError> {
        let state_dir = onecrawl_cdp::DurableSession::default_state_dir();
        let name = p.name.as_deref().unwrap_or("default");
        let state = onecrawl_cdp::DurableSession::get_status(&state_dir, name)
            .map_err(|e| mcp_err(format!("durable_status: {e}")))?;
        json_ok(&state)
    }

    pub(crate) async fn durable_list(
        &self,
        _p: DurableListParams,
    ) -> Result<CallToolResult, McpError> {
        let state_dir = onecrawl_cdp::DurableSession::default_state_dir();
        let sessions = onecrawl_cdp::DurableSession::list_sessions(&state_dir)
            .map_err(|e| mcp_err(format!("durable_list: {e}")))?;

        let summary: Vec<serde_json::Value> = sessions
            .iter()
            .map(|s| {
                serde_json::json!({
                    "name": s.name,
                    "status": s.status,
                    "url": s.url,
                    "last_checkpoint": s.last_checkpoint,
                    "reconnect_count": s.reconnect_count,
                    "total_uptime_secs": s.total_uptime_secs,
                })
            })
            .collect();

        json_ok(&serde_json::json!({
            "sessions": summary,
            "count": summary.len(),
        }))
    }

    pub(crate) async fn durable_delete(
        &self,
        p: DurableDeleteParams,
    ) -> Result<CallToolResult, McpError> {
        let state_dir = onecrawl_cdp::DurableSession::default_state_dir();
        onecrawl_cdp::DurableSession::delete_session(&state_dir, &p.name)
            .map_err(|e| mcp_err(format!("durable_delete: {e}")))?;
        text_ok(format!("durable session '{}' deleted", p.name))
    }

    pub(crate) async fn durable_config(
        &self,
        p: DurableConfigParams,
    ) -> Result<CallToolResult, McpError> {
        let state_dir = onecrawl_cdp::DurableSession::default_state_dir();
        let state = onecrawl_cdp::DurableSession::get_status(&state_dir, &p.name)
            .map_err(|e| mcp_err(format!("durable_config: {e}")))?;

        let mut config = onecrawl_cdp::DurableConfig {
            name: p.name.clone(),
            state_path: state_dir,
            ..onecrawl_cdp::DurableConfig::default()
        };

        if let Some(interval) = p.checkpoint_interval_secs {
            config.checkpoint_interval_secs = interval;
        }
        if let Some(ar) = p.auto_reconnect {
            config.auto_reconnect = ar;
        }
        if let Some(on_crash) = &p.on_crash {
            config.on_crash = match on_crash.as_str() {
                "stop" => onecrawl_cdp::CrashPolicy::Stop,
                "notify" => onecrawl_cdp::CrashPolicy::Notify,
                _ => onecrawl_cdp::CrashPolicy::Restart,
            };
        }
        config.max_uptime_secs = p.max_uptime_secs;

        json_ok(&serde_json::json!({
            "action": "durable_config",
            "name": p.name,
            "current_status": state.status,
            "updated_config": {
                "checkpoint_interval_secs": config.checkpoint_interval_secs,
                "auto_reconnect": config.auto_reconnect,
                "on_crash": format!("{:?}", config.on_crash),
                "max_uptime_secs": config.max_uptime_secs,
            },
        }))
    }
}
