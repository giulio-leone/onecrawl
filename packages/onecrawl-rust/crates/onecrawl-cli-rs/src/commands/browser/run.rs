//! Generic MCP action runner — provides 100% CLI coverage for all MCP actions.
//!
//! Usage: `onecrawl run <tool> <action> --json '{"param": "value"}'`
//!
//! Instead of 163 individual CLI handler functions, this single entry point
//! delegates to the MCP server's dispatch with zero code duplication (KISS+DRY).

use colored::Colorize;
use onecrawl_mcp_rs::cdp_tools::BrowserState;
use onecrawl_mcp_rs::OneCrawlMcp;
use std::sync::Arc;
use tokio::sync::Mutex;

/// Run any MCP action via CLI. Connects to the daemon session, constructs a
/// pre-populated MCP instance, and delegates to the existing MCP handlers.
pub async fn run_action(tool: &str, action: &str, params_json: Option<&str>) {
    let params: serde_json::Value = params_json
        .map(|j| {
            serde_json::from_str(j).unwrap_or_else(|e| {
                eprintln!("{} Invalid JSON: {e}", "✗".red());
                std::process::exit(1);
            })
        })
        .unwrap_or(serde_json::Value::Object(Default::default()));

    let (session, page) = match super::super::session::connect_to_session().await {
        Ok(v) => v,
        Err(e) => {
            eprintln!("{} {e}", "✗".red());
            std::process::exit(1);
        }
    };

    // Pre-populate a SharedBrowser with the existing daemon session
    let browser = {
        let mut state = BrowserState::default();
        state.tabs.push(page.clone());
        state.active_tab = 0;
        state.page = Some(page);
        state.session = Some(session);
        Arc::new(Mutex::new(state))
    };

    let mcp = OneCrawlMcp::from_browser(
        browser,
        "/tmp/onecrawl-mcp-store".into(),
        "onecrawl-default-key".into(),
    );

    match mcp.run_tool(tool, action, params).await {
        Ok(output) => println!("{output}"),
        Err(e) => {
            eprintln!("{} {e}", "✗".red());
            std::process::exit(1);
        }
    }
}

/// Print all available tools and their actions.
pub fn list_actions() {
    println!("{}", "Available tools and actions:".green().bold());
    println!();

    let tools: &[(&str, &[&str])] = &[
        (
            "browser",
            &[
                "goto", "click", "type", "screenshot", "pdf", "back", "forward",
                "reload", "wait", "evaluate", "snapshot", "css", "xpath",
                "find_text", "text", "html", "markdown", "structured", "stream",
                "detect_forms", "fill_form", "snapshot_diff", "parse_a11y",
                "parse_selector", "parse_text", "parse_links", "new_tab",
                "list_tabs", "switch_tab", "close_tab", "observe_mutations",
                "get_mutations", "stop_mutations", "wait_for_event",
                "cookies_get", "cookies_set", "cookies_clear", "storage_get",
                "storage_set", "export_session", "import_session",
                "intercept_enable", "intercept_add_rule", "intercept_remove_rule",
                "intercept_list", "intercept_disable", "block_requests",
                "console_start", "console_get", "console_clear", "dialog_handle",
                "dialog_get", "errors_get", "emulate_device",
                "emulate_geolocation", "emulate_timezone", "emulate_media",
                "emulate_network", "drag", "hover", "keyboard", "select",
                "upload", "download_wait", "download_list", "download_set_dir",
                "shadow_query", "shadow_text", "deep_query", "context_set",
                "context_get", "context_list", "context_clear",
                "context_transfer", "form_infer", "form_auto_fill",
                "form_validate", "selector_heal", "selector_alternatives",
                "selector_validate", "event_subscribe", "event_unsubscribe",
                "event_poll", "event_clear", "sw_register", "sw_unregister",
                "sw_list", "sw_update", "cache_list", "cache_clear",
                "push_simulate", "offline_mode", "set_mode", "set_stealth",
                "session_info", "spa_nav_watch", "framework_detect",
                "virtual_scroll_detect", "virtual_scroll_extract",
                "wait_hydration", "wait_animation", "wait_network_idle",
                "trigger_lazy_load", "health_check", "circuit_breaker",
                "state_inspect", "form_wizard_track", "dynamic_import_wait",
                "parallel_exec", "token_budget", "compact_state",
                "page_assertions",
            ],
        ),
        (
            "crawl",
            &["spider", "robots", "sitemap", "dom_snapshot", "dom_compare"],
        ),
        (
            "agent",
            &[
                "execute_chain", "element_screenshot", "api_capture_start",
                "api_capture_summary", "iframe_list", "iframe_snapshot",
                "iframe_eval_cdp", "iframe_click_cdp", "iframe_frames",
                "connect_remote", "safety_set", "safety_status", "skills_list",
                "screencast_start", "screencast_stop", "screencast_frame",
                "recording_start", "recording_stop", "recording_status",
                "stream_capture", "stream_to_disk", "recording_encode",
                "recording_capture", "ios_devices", "ios_connect",
                "ios_navigate", "ios_tap", "ios_screenshot", "task_decompose",
                "task_plan", "task_status", "vision_describe", "vision_locate",
                "vision_compare", "wcag_audit", "aria_tree", "contrast_check",
                "landmark_nav", "focus_order", "alt_text_audit",
                "heading_structure", "role_validate", "keyboard_trap_detect",
                "screen_reader_sim", "agent_loop", "goal_assert",
                "annotated_observe", "session_context", "auto_chain", "think",
                "plan_execute", "page_summary", "error_context",
            ],
        ),
        (
            "stealth",
            &[
                "inject", "test", "fingerprint", "block_domains",
                "detect_captcha", "solve_captcha", "human_delay", "human_mouse",
                "human_type", "human_scroll", "human_profile", "stealth_max",
                "stealth_score", "tls_apply", "webrtc_block", "battery_spoof",
                "sensor_block", "canvas_advanced", "timezone_sync",
                "font_protect", "behavior_sim", "behavior_stop",
                "stealth_rotate", "detection_audit", "stealth_status",
            ],
        ),
        (
            "data",
            &[
                "pipeline", "http_get", "http_post", "links", "graph",
                "net_capture", "net_analyze", "net_sdk", "net_mock",
                "net_replay", "extract_schema", "extract_tables",
                "extract_entities", "classify_content", "transform_json",
                "export_csv", "extract_metadata", "extract_feeds", "ws_connect",
                "ws_intercept", "ws_send", "ws_messages", "ws_close",
                "sse_listen", "sse_messages", "graphql_subscribe",
                "extract_compact",
            ],
        ),
        (
            "secure",
            &[
                "encrypt", "decrypt", "pkce", "totp", "kv_set", "kv_get",
                "kv_list", "passkey_enable", "passkey_add", "passkey_list",
                "passkey_log", "passkey_disable", "passkey_remove",
                "auth_oauth2", "auth_session", "auth_form_login", "auth_mfa",
                "auth_status", "auth_logout", "credential_store",
                "credential_get",
            ],
        ),
        (
            "computer",
            &[
                "act", "observe", "batch", "smart_find", "smart_click",
                "smart_fill", "pool_list", "pool_status", "fleet_spawn",
                "fleet_broadcast", "fleet_collect", "fleet_destroy",
                "fleet_status", "fleet_balance", "computer_use", "goal_execute",
                "step_verify", "auto_recover", "annotated_screenshot",
                "adaptive_retry", "click_at_coords", "multi_page_sync",
                "input_replay", "element_info",
            ],
        ),
        (
            "memory",
            &["store", "recall", "search", "forget", "domain_strategy", "stats"],
        ),
        (
            "automate",
            &[
                "workflow_validate", "workflow_run", "plan", "execute",
                "patterns", "rate_limit", "retry", "retry_adapt",
                "error_classify", "recovery_suggest", "error_history",
                "checkpoint_save", "checkpoint_restore", "checkpoint_list",
                "checkpoint_delete", "workflow_while", "workflow_for_each",
                "workflow_if", "workflow_variable", "reconnect_cdp", "gc_tabs",
                "watchdog", "batch_execute", "workflow_execute",
                "workflow_status",
            ],
        ),
        (
            "perf",
            &[
                "audit", "budget", "compare", "trace", "vrt_run", "vrt_compare",
                "vrt_update", "pixel_diff",
            ],
        ),
    ];

    let mut total = 0;
    for (tool, actions) in tools {
        println!(
            "  {} ({} actions)",
            tool.cyan().bold(),
            actions.len()
        );
        for action in *actions {
            println!("    {action}");
        }
        total += actions.len();
        println!();
    }
    println!(
        "{} {} actions across {} tools",
        "Total:".bold(),
        total,
        tools.len()
    );
}
