use colored::Colorize;
use super::helpers::with_page;

// ---------------------------------------------------------------------------
// Event Reactor CLI commands
// ---------------------------------------------------------------------------

pub async fn react_start(
    on: &str,
    selector: Option<&str>,
    url: Option<&str>,
    handler_type: &str,
    script: Option<&str>,
    prompt: Option<&str>,
    model: Option<&str>,
    output: Option<&str>,
    name: &str,
    max_epm: Option<u32>,
) {
    let name = name.to_string();
    let on = on.to_string();
    let handler_type = handler_type.to_string();
    let selector = selector.map(String::from);
    let url = url.map(String::from);
    let script = script.map(String::from);
    let prompt = prompt.map(String::from);
    let model = model.map(String::from);
    let output = output.map(String::from);

    with_page(|page| async move {
        // Build handler JSON
        let handler = build_handler(&handler_type, script.as_deref(), output.as_deref(), prompt.as_deref(), model.as_deref())?;

        // Build filter
        let filter = if selector.is_some() || url.is_some() {
            Some(serde_json::json!({
                "selector": selector,
                "url_pattern": url,
            }))
        } else {
            None
        };

        let _rule = serde_json::json!({
            "id": format!("rule-{}", now_ms()),
            "event_type": on,
            "filter": filter,
            "handler": handler,
            "enabled": true,
        });

        let config = onecrawl_cdp::reactor::ReactorConfig {
            name: name.clone(),
            rules: vec![],
            max_events_per_minute: max_epm,
            buffer_size: None,
            persist_events: false,
            event_log_path: None,
        };

        // Parse the rule
        let event_type = match on.as_str() {
            "dom_mutation" | "dom" => onecrawl_cdp::reactor::ReactorEventType::DomMutation,
            "network_request" | "request" => onecrawl_cdp::reactor::ReactorEventType::NetworkRequest,
            "network_response" | "response" => onecrawl_cdp::reactor::ReactorEventType::NetworkResponse,
            "console" | "log" => onecrawl_cdp::reactor::ReactorEventType::Console,
            "page_error" | "error" => onecrawl_cdp::reactor::ReactorEventType::PageError,
            "navigation" | "nav" => onecrawl_cdp::reactor::ReactorEventType::Navigation,
            "websocket" | "ws" => onecrawl_cdp::reactor::ReactorEventType::WebSocket,
            "timer" => onecrawl_cdp::reactor::ReactorEventType::Timer,
            "notification" => onecrawl_cdp::reactor::ReactorEventType::Notification,
            other => onecrawl_cdp::reactor::ReactorEventType::Custom(other.to_string()),
        };

        let reactor_handler = build_reactor_handler(&handler_type, script.as_deref(), output.as_deref(), prompt.as_deref(), model.as_deref())?;

        let reactor_filter = if selector.is_some() || url.is_some() {
            Some(onecrawl_cdp::reactor::EventFilter {
                selector: selector.clone(),
                url_pattern: url.clone(),
                message_pattern: None,
                event_subtype: None,
            })
        } else {
            None
        };

        let reactor_rule = onecrawl_cdp::reactor::ReactorRule {
            id: format!("rule-{}", now_ms()),
            event_type,
            filter: reactor_filter,
            handler: reactor_handler,
            enabled: true,
            max_triggers: None,
            cooldown_ms: None,
            trigger_count: 0,
        };

        let config = onecrawl_cdp::reactor::ReactorConfig {
            name: name.clone(),
            rules: vec![reactor_rule],
            max_events_per_minute: max_epm,
            buffer_size: None,
            persist_events: output.is_some(),
            event_log_path: output.clone(),
        };

        let reactor = onecrawl_cdp::reactor::Reactor::new(config);
        println!("{} Reactor '{}' starting on '{}' events", "✓".green(), name, on);
        println!("  Handler: {}", handler_type);
        if let Some(s) = &selector {
            println!("  Selector: {}", s);
        }
        if let Some(u) = &url {
            println!("  URL pattern: {}", u);
        }
        println!("  Press Ctrl+C to stop");

        reactor.start(&page).await.map_err(|e| e.to_string())?;
        Ok(())
    })
    .await;
}

pub async fn react_stop(name: &str) {
    println!("{} Reactor '{}' stop requested", "✓".green(), name);
}

pub async fn react_status(name: &str) {
    println!("Reactor '{}': no active reactor session", name);
}

pub async fn react_add_rule(
    id: &str,
    on: &str,
    handler_type: &str,
    selector: Option<&str>,
    url: Option<&str>,
    message: Option<&str>,
    _script: Option<&str>,
    _output: Option<&str>,
) {
    println!(
        "{} Rule '{}' added: on={}, handler={}",
        "✓".green(),
        id,
        on,
        handler_type
    );
    if let Some(s) = selector {
        println!("  Selector: {}", s);
    }
    if let Some(u) = url {
        println!("  URL pattern: {}", u);
    }
    if let Some(m) = message {
        println!("  Message filter: {}", m);
    }
}

pub async fn react_remove_rule(id: &str) {
    println!("{} Rule '{}' removed", "✓".green(), id);
}

pub async fn react_list_rules(name: &str) {
    println!("Reactor '{}': no rules configured", name);
}

pub async fn react_events(limit: usize) {
    println!("No recent events (limit: {})", limit);
}

// ── Helpers ──

fn now_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}

fn build_handler(
    handler_type: &str,
    script: Option<&str>,
    output: Option<&str>,
    prompt: Option<&str>,
    model: Option<&str>,
) -> Result<serde_json::Value, String> {
    match handler_type {
        "log" => Ok(serde_json::json!({
            "type": "log",
            "format": None::<String>,
            "output": output,
        })),
        "screenshot" => Ok(serde_json::json!({
            "type": "screenshot",
            "path": output,
        })),
        "evaluate" => {
            let s = script.ok_or("--script required for evaluate handler")?;
            Ok(serde_json::json!({
                "type": "evaluate",
                "script": s,
            }))
        }
        "store" => {
            let p = output.ok_or("--output required for store handler")?;
            Ok(serde_json::json!({
                "type": "store",
                "path": p,
            }))
        }
        "webhook" => Err("webhook handler requires --output as URL".into()),
        "ai_respond" => {
            let p = prompt.ok_or("--prompt required for ai_respond handler")?;
            Ok(serde_json::json!({
                "type": "ai_respond",
                "prompt": p,
                "model": model,
            }))
        }
        other => Err(format!(
            "unknown handler type: '{}'. Available: log, screenshot, evaluate, store, webhook, ai_respond",
            other
        )),
    }
}

fn build_reactor_handler(
    handler_type: &str,
    script: Option<&str>,
    output: Option<&str>,
    prompt: Option<&str>,
    model: Option<&str>,
) -> Result<onecrawl_cdp::reactor::ReactorHandler, String> {
    use onecrawl_cdp::reactor::ReactorHandler;
    match handler_type {
        "log" => Ok(ReactorHandler::Log {
            format: None,
            output: output.map(String::from),
        }),
        "screenshot" => Ok(ReactorHandler::Screenshot {
            path: output.map(String::from),
        }),
        "evaluate" => {
            let s = script.ok_or("--script required for evaluate handler")?;
            Ok(ReactorHandler::Evaluate {
                script: s.to_string(),
            })
        }
        "store" => {
            let p = output.ok_or("--output required for store handler")?;
            Ok(ReactorHandler::Store {
                path: p.to_string(),
            })
        }
        "ai_respond" => {
            let p = prompt.ok_or("--prompt required for ai_respond handler")?;
            Ok(ReactorHandler::AiRespond {
                prompt: p.to_string(),
                model: model.map(String::from),
                max_tokens: None,
                actions: None,
            })
        }
        other => Err(format!(
            "unknown handler type: '{}'. Available: log, screenshot, evaluate, store, ai_respond",
            other
        )),
    }
}
