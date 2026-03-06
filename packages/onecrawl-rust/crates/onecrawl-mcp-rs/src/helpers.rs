//! Shared helpers for MCP tool handlers.

use rmcp::{ErrorData as McpError, model::*};

use crate::agent_error::AgentError;
use crate::cdp_tools::SharedBrowser;

// ── Error helpers ──

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

/// Extension trait: convert any `Result<T, E: Display>` into `Result<T, McpError>`.
///
/// Replaces the pervasive `.map_err(|e| mcp_err(e.to_string()))` pattern.
pub trait McpResult<T> {
    fn mcp(self) -> Result<T, McpError>;
}

impl<T, E: std::fmt::Display> McpResult<T> for Result<T, E> {
    fn mcp(self) -> Result<T, McpError> {
        self.map_err(|e| mcp_err(e.to_string()))
    }
}

// ── Deserialization helpers ──

/// Deserialize a `serde_json::Value` into `T` with a contextual action label.
///
/// Replaces: `serde_json::from_value(v).map_err(|e| mcp_err(format!("{action}: {e}")))`
pub fn parse_params<T: serde::de::DeserializeOwned>(
    v: serde_json::Value,
    action: &str,
) -> Result<T, McpError> {
    serde_json::from_value(v).map_err(|e| mcp_err(format!("{action}: {e}")))
}

/// Deserialize a JSON string into `T` with a contextual field label.
///
/// Replaces: `serde_json::from_str(s).map_err(|e| mcp_err(format!("invalid {field} JSON: {e}")))`
pub fn parse_json_str<T: serde::de::DeserializeOwned>(
    s: &str,
    field: &str,
) -> Result<T, McpError> {
    serde_json::from_str(s).map_err(|e| mcp_err(format!("invalid {field} JSON: {e}")))
}

/// Deserialize an optional JSON string, returning `Ok(None)` when the input is `None`.
pub fn parse_opt_json_str<T: serde::de::DeserializeOwned>(
    s: Option<&str>,
    field: &str,
) -> Result<Option<T>, McpError> {
    match s {
        Some(s) => Ok(Some(parse_json_str(s, field)?)),
        None => Ok(None),
    }
}

// ── Browser helpers ──

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

// ── Response helpers ──

/// Serialize a value as JSON and return a success result.
pub fn json_ok(value: &impl serde::Serialize) -> Result<CallToolResult, McpError> {
    let json = serde_json::to_string(value).mcp()?;
    Ok(CallToolResult::success(vec![Content::text(json)]))
}

/// Return a plain-text success result.
pub fn text_ok(msg: impl Into<String>) -> Result<CallToolResult, McpError> {
    Ok(CallToolResult::success(vec![Content::text(msg.into())]))
}

/// Parse a string into a `MemoryCategory` enum variant.
pub fn parse_memory_category(s: Option<&str>) -> Option<onecrawl_cdp::MemoryCategory> {
    match s {
        Some("page_visit") => Some(onecrawl_cdp::MemoryCategory::PageVisit),
        Some("element_pattern") => Some(onecrawl_cdp::MemoryCategory::ElementPattern),
        Some("domain_strategy") => Some(onecrawl_cdp::MemoryCategory::DomainStrategy),
        Some("retry_knowledge") => Some(onecrawl_cdp::MemoryCategory::RetryKnowledge),
        Some("user_preference") => Some(onecrawl_cdp::MemoryCategory::UserPreference),
        Some("selector_mapping") => Some(onecrawl_cdp::MemoryCategory::SelectorMapping),
        Some("error_pattern") => Some(onecrawl_cdp::MemoryCategory::ErrorPattern),
        Some("custom") => Some(onecrawl_cdp::MemoryCategory::Custom),
        _ => None,
    }
}
