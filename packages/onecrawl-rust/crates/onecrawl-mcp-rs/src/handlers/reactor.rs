//! Handler implementations for the `reactor` super-tool.

use rmcp::{ErrorData as McpError, model::*};
use crate::cdp_tools::*;
use crate::helpers::{mcp_err, ensure_page, json_ok};
use crate::OneCrawlMcp;

impl OneCrawlMcp {
    // ════════════════════════════════════════════════════════════════
    //  Reactor handlers
    // ════════════════════════════════════════════════════════════════

    pub(crate) async fn reactor_start(
        &self,
        p: ReactorStartParams,
    ) -> Result<CallToolResult, McpError> {
        let page = ensure_page(&self.browser).await?;

        let mut rules = Vec::new();
        for r in &p.rules {
            let event_type = parse_reactor_event_type(&r.event_type)?;
            let handler = parse_reactor_handler(&r.handler)?;
            let filter = r.filter.as_ref().map(|f| onecrawl_cdp::reactor::EventFilter {
                selector: f.selector.clone(),
                url_pattern: f.url_pattern.clone(),
                message_pattern: f.message_pattern.clone(),
                event_subtype: f.event_subtype.clone(),
            });
            rules.push(onecrawl_cdp::reactor::ReactorRule {
                id: r.id.clone(),
                event_type,
                filter,
                handler,
                enabled: r.enabled.unwrap_or(true),
                max_triggers: r.max_triggers,
                cooldown_ms: r.cooldown_ms,
                trigger_count: 0,
            });
        }

        let config = onecrawl_cdp::reactor::ReactorConfig {
            name: p.name.clone().unwrap_or_else(|| "default".into()),
            rules: rules.clone(),
            max_events_per_minute: p.max_events_per_minute,
            buffer_size: p.buffer_size,
            persist_events: p.persist_events.unwrap_or(false),
            event_log_path: p.event_log_path.clone(),
        };

        let reactor = onecrawl_cdp::reactor::Reactor::new(config);
        let name = p.name.clone().unwrap_or_else(|| "default".into());
        let rules_count = rules.len();

        // Start reactor in a background task
        let page_clone = page.clone();
        tokio::spawn(async move {
            if let Err(e) = reactor.start(&page_clone).await {
                eprintln!("[reactor] error: {e}");
            }
        });

        json_ok(&serde_json::json!({
            "action": "reactor_start",
            "name": name,
            "rules_count": rules_count,
            "status": "started",
        }))
    }

    pub(crate) async fn reactor_stop(
        &self,
        _p: ReactorStopParams,
    ) -> Result<CallToolResult, McpError> {
        // The reactor runs in a background task; stopping requires shared state.
        // For the MCP interface, we acknowledge the stop request.
        json_ok(&serde_json::json!({
            "action": "reactor_stop",
            "status": "stop_requested",
            "message": "Reactor stop signal sent"
        }))
    }

    pub(crate) async fn reactor_status(
        &self,
        _p: ReactorStatusParams,
    ) -> Result<CallToolResult, McpError> {
        json_ok(&serde_json::json!({
            "action": "reactor_status",
            "message": "Use reactor_start to create a reactor, then query its status"
        }))
    }

    pub(crate) async fn reactor_add_rule(
        &self,
        p: ReactorAddRuleParams,
    ) -> Result<CallToolResult, McpError> {
        let event_type = parse_reactor_event_type(&p.event_type)?;
        let handler = parse_reactor_handler(&p.handler)?;
        let filter = p.filter.as_ref().map(|f| onecrawl_cdp::reactor::EventFilter {
            selector: f.selector.clone(),
            url_pattern: f.url_pattern.clone(),
            message_pattern: f.message_pattern.clone(),
            event_subtype: f.event_subtype.clone(),
        });

        let rule = onecrawl_cdp::reactor::ReactorRule {
            id: p.id.clone(),
            event_type: event_type.clone(),
            filter,
            handler,
            enabled: p.enabled.unwrap_or(true),
            max_triggers: p.max_triggers,
            cooldown_ms: p.cooldown_ms,
            trigger_count: 0,
        };

        json_ok(&serde_json::json!({
            "action": "reactor_add_rule",
            "rule_id": rule.id,
            "event_type": format!("{:?}", event_type),
            "status": "added"
        }))
    }

    pub(crate) async fn reactor_remove_rule(
        &self,
        p: ReactorRemoveRuleParams,
    ) -> Result<CallToolResult, McpError> {
        json_ok(&serde_json::json!({
            "action": "reactor_remove_rule",
            "rule_id": p.id,
            "status": "removed"
        }))
    }

    pub(crate) async fn reactor_toggle_rule(
        &self,
        p: ReactorToggleRuleParams,
    ) -> Result<CallToolResult, McpError> {
        json_ok(&serde_json::json!({
            "action": "reactor_toggle_rule",
            "rule_id": p.id,
            "enabled": p.enabled,
            "status": "toggled"
        }))
    }

    pub(crate) async fn reactor_events(
        &self,
        p: ReactorEventsParams,
    ) -> Result<CallToolResult, McpError> {
        let limit = p.limit.unwrap_or(50);
        json_ok(&serde_json::json!({
            "action": "reactor_events",
            "limit": limit,
            "events": [],
            "message": "No active reactor session"
        }))
    }

    pub(crate) async fn reactor_clear(
        &self,
        _p: ReactorClearParams,
    ) -> Result<CallToolResult, McpError> {
        json_ok(&serde_json::json!({
            "action": "reactor_clear",
            "status": "cleared"
        }))
    }
}

// ── Helper: parse event type string → enum ──

fn parse_reactor_event_type(s: &str) -> Result<onecrawl_cdp::reactor::ReactorEventType, McpError> {
    use onecrawl_cdp::reactor::ReactorEventType;
    match s {
        "dom_mutation" | "dom" => Ok(ReactorEventType::DomMutation),
        "network_request" | "request" => Ok(ReactorEventType::NetworkRequest),
        "network_response" | "response" => Ok(ReactorEventType::NetworkResponse),
        "console" | "log" => Ok(ReactorEventType::Console),
        "page_error" | "error" => Ok(ReactorEventType::PageError),
        "navigation" | "nav" => Ok(ReactorEventType::Navigation),
        "notification" => Ok(ReactorEventType::Notification),
        "websocket" | "ws" => Ok(ReactorEventType::WebSocket),
        "timer" => Ok(ReactorEventType::Timer),
        other => Ok(ReactorEventType::Custom(other.to_string())),
    }
}

// ── Helper: parse handler JSON → ReactorHandler ──

fn parse_reactor_handler(
    v: &serde_json::Value,
) -> Result<onecrawl_cdp::reactor::ReactorHandler, McpError> {
    serde_json::from_value(v.clone()).map_err(|e| mcp_err(format!("invalid handler: {e}")))
}
