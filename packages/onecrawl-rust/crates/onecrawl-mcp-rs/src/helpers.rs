//! Shared helpers for MCP tool handlers.

use rmcp::{ErrorData as McpError, model::*};

use crate::agent_error::AgentError;
use crate::cdp_tools::SharedBrowser;

/// Create an internal MCP error.
pub fn mcp_err(msg: impl Into<String>) -> McpError {
    McpError::internal_error(msg.into(), None)
}

/// Create an MCP error from a structured [`AgentError`], serialised as JSON
/// so that agent consumers receive machine-readable error metadata.
pub fn agent_err(err: AgentError) -> McpError {
    let json = serde_json::to_string(&err).unwrap_or_else(|_| err.message.clone());
    McpError::internal_error(json, None)
}

/// Ensure browser session + page are initialised, return a clone of the page handle.
pub async fn ensure_page(browser: &SharedBrowser) -> Result<chromiumoxide::Page, McpError> {
    let mut state = browser.lock().await;
    if state.session.is_none() {
        let session = onecrawl_cdp::BrowserSession::launch_headless()
            .await
            .map_err(|e| mcp_err(format!("browser launch failed: {e}")))?;
        let page = session
            .new_page("about:blank")
            .await
            .map_err(|e| mcp_err(format!("new page failed: {e}")))?;
        state.session = Some(session);
        state.page = Some(page);
    }
    state
        .page
        .clone()
        .ok_or_else(|| mcp_err("no active page"))
}

/// Serialize a value as JSON and return a success result.
pub fn json_ok(value: &impl serde::Serialize) -> Result<CallToolResult, McpError> {
    let json = serde_json::to_string(value).map_err(|e| mcp_err(e.to_string()))?;
    Ok(CallToolResult::success(vec![Content::text(json)]))
}

/// Return a plain-text success result.
pub fn text_ok(msg: impl Into<String>) -> Result<CallToolResult, McpError> {
    Ok(CallToolResult::success(vec![Content::text(msg.into())]))
}
