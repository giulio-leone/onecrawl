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

/// Print all available tools and their actions, grouped by category.
pub fn list_actions() {
    println!("{}", "Available tools and actions:".green().bold());
    println!("{}", "═══════════════════════════".green());
    println!();

    // (tool_name, emoji, &[(category, &[(action, description)])])
    const TOOLS: &[(&str, &str, &[(&str, &[(&str, &str)])])] = &[
        ("browser", "🌐", &[
            ("Navigation", &[
                ("goto",         "Navigate to URL"),
                ("back",         "Go back in history"),
                ("forward",      "Go forward in history"),
                ("reload",       "Reload current page"),
            ]),
            ("Waiting", &[
                ("wait",              "Wait for selector, time, or condition"),
                ("wait_for_event",    "Wait for a DOM/network event"),
                ("wait_hydration",    "Wait for framework hydration to complete"),
                ("wait_animation",    "Wait for CSS animations to finish"),
                ("wait_network_idle", "Wait until network is idle"),
            ]),
            ("Content", &[
                ("text",         "Extract visible page text"),
                ("html",         "Get page HTML source"),
                ("markdown",     "Convert page to Markdown"),
                ("structured",   "Extract structured data (JSON-LD, OpenGraph)"),
                ("stream",       "Stream page content progressively"),
                ("find_text",    "Search for text on page"),
                ("screenshot",   "Take viewport/full-page/element screenshot"),
                ("pdf",          "Export page as PDF"),
            ]),
            ("Interaction", &[
                ("click",    "Click an element"),
                ("type",     "Type text into an element"),
                ("drag",     "Drag and drop between elements"),
                ("hover",    "Hover over an element"),
                ("keyboard", "Send keyboard events"),
                ("select",   "Select option in dropdown"),
                ("upload",   "Upload files to input"),
            ]),
            ("DOM & Selectors", &[
                ("evaluate",              "Execute JavaScript in page context"),
                ("css",                   "Query elements with CSS selector"),
                ("xpath",                 "Query elements with XPath"),
                ("snapshot",              "Capture accessibility tree snapshot"),
                ("snapshot_diff",         "Diff two accessibility snapshots"),
                ("deep_query",            "Query through shadow DOM boundaries"),
                ("shadow_query",          "Query inside shadow roots"),
                ("shadow_text",           "Extract text from shadow DOM"),
                ("selector_heal",         "Auto-heal broken selectors"),
                ("selector_alternatives", "Suggest alternative selectors"),
                ("selector_validate",     "Validate selector matches element"),
            ]),
            ("Parsing", &[
                ("parse_a11y",      "Parse accessibility properties"),
                ("parse_selector",  "Parse and analyze CSS selector"),
                ("parse_text",      "Parse and extract text content"),
                ("parse_links",     "Extract and parse all links"),
            ]),
            ("Forms", &[
                ("detect_forms",    "Detect all forms on page"),
                ("fill_form",       "Fill form fields"),
                ("form_infer",      "Infer form field types and purposes"),
                ("form_auto_fill",  "Auto-fill form with generated data"),
                ("form_validate",   "Validate form field constraints"),
                ("form_wizard_track", "Track multi-step form wizard progress"),
            ]),
            ("Tabs", &[
                ("new_tab",    "Open a new browser tab"),
                ("list_tabs",  "List all open tabs"),
                ("switch_tab", "Switch to a tab by index"),
                ("close_tab",  "Close a tab"),
            ]),
            ("Storage & Sessions", &[
                ("cookies_get",    "Get cookies"),
                ("cookies_set",    "Set cookies"),
                ("cookies_clear",  "Clear cookies"),
                ("storage_get",    "Get localStorage/sessionStorage"),
                ("storage_set",    "Set localStorage/sessionStorage"),
                ("export_session", "Export browser session state"),
                ("import_session", "Import browser session state"),
            ]),
            ("Network", &[
                ("intercept_enable",      "Enable request interception"),
                ("intercept_add_rule",    "Add interception rule"),
                ("intercept_remove_rule", "Remove interception rule"),
                ("intercept_list",        "List active interception rules"),
                ("intercept_disable",     "Disable request interception"),
                ("block_requests",        "Block requests by URL pattern"),
            ]),
            ("Console & Errors", &[
                ("console_start",  "Start capturing console output"),
                ("console_get",    "Get captured console messages"),
                ("console_clear",  "Clear captured console messages"),
                ("dialog_handle",  "Handle alert/confirm/prompt dialog"),
                ("dialog_get",     "Get current dialog info"),
                ("errors_get",     "Get page errors and exceptions"),
            ]),
            ("Emulation", &[
                ("emulate_device",      "Emulate device (mobile, tablet, etc.)"),
                ("emulate_geolocation", "Emulate GPS coordinates"),
                ("emulate_timezone",    "Emulate timezone"),
                ("emulate_media",       "Emulate media features (dark mode, print)"),
                ("emulate_network",     "Emulate network conditions (3G, offline)"),
            ]),
            ("Downloads", &[
                ("download_wait",    "Wait for download to complete"),
                ("download_list",    "List downloaded files"),
                ("download_set_dir", "Set download directory"),
            ]),
            ("DOM Observation", &[
                ("observe_mutations", "Start observing DOM mutations"),
                ("get_mutations",     "Get recorded DOM mutations"),
                ("stop_mutations",    "Stop observing DOM mutations"),
                ("event_subscribe",   "Subscribe to DOM events"),
                ("event_unsubscribe", "Unsubscribe from DOM events"),
                ("event_poll",        "Poll for subscribed events"),
                ("event_clear",       "Clear event buffer"),
            ]),
            ("Service Workers & PWA", &[
                ("sw_register",    "Register a service worker"),
                ("sw_unregister",  "Unregister a service worker"),
                ("sw_list",        "List registered service workers"),
                ("sw_update",      "Force service worker update"),
                ("cache_list",     "List service worker caches"),
                ("cache_clear",    "Clear service worker caches"),
                ("push_simulate",  "Simulate push notification"),
                ("offline_mode",   "Toggle offline mode"),
            ]),
            ("Context", &[
                ("context_set",      "Set named context variable"),
                ("context_get",      "Get named context variable"),
                ("context_list",     "List all context variables"),
                ("context_clear",    "Clear context variables"),
                ("context_transfer", "Transfer context between tabs"),
            ]),
            ("SPA & Framework", &[
                ("spa_nav_watch",          "Watch SPA navigation events"),
                ("framework_detect",       "Detect frontend framework in use"),
                ("virtual_scroll_detect",  "Detect virtual/infinite scroll"),
                ("virtual_scroll_extract", "Extract virtual scroll content"),
                ("trigger_lazy_load",      "Trigger lazy-loaded content"),
                ("dynamic_import_wait",    "Wait for dynamic imports to load"),
            ]),
            ("Reliability & Config", &[
                ("health_check",    "Run browser health check"),
                ("circuit_breaker", "Get circuit breaker status"),
                ("state_inspect",   "Inspect internal browser state"),
                ("page_assertions", "Run assertions against page"),
                ("set_mode",        "Set browser operation mode"),
                ("set_stealth",     "Toggle stealth mode"),
                ("session_info",    "Get current session info"),
                ("parallel_exec",   "Execute actions in parallel"),
                ("token_budget",    "Get/set token budget for responses"),
                ("compact_state",   "Compact internal state representation"),
            ]),
        ]),
        ("crawl", "🕷️", &[
            ("Crawling", &[
                ("spider",       "Crawl site following links"),
                ("robots",       "Parse robots.txt rules"),
                ("sitemap",      "Parse XML sitemap"),
                ("dom_snapshot", "Capture full DOM snapshot"),
                ("dom_compare",  "Compare two DOM snapshots"),
            ]),
        ]),
        ("agent", "🤖", &[
            ("Execution", &[
                ("execute_chain", "Run multiple actions sequentially"),
                ("agent_loop",    "Autonomous goal-directed browsing"),
                ("auto_chain",    "Auto-generate and run action chain"),
                ("plan_execute",  "Execute a structured plan"),
            ]),
            ("Visual & Screenshots", &[
                ("element_screenshot", "Screenshot a specific element"),
                ("annotated_observe",  "Observe page with visual annotations"),
                ("vision_describe",    "Describe page content using vision"),
                ("vision_locate",      "Locate element using visual description"),
                ("vision_compare",     "Compare visual states"),
            ]),
            ("API Capture", &[
                ("api_capture_start",   "Start capturing API requests"),
                ("api_capture_summary", "Get API capture summary"),
            ]),
            ("Iframes", &[
                ("iframe_list",      "List all iframes on page"),
                ("iframe_snapshot",  "Snapshot iframe content"),
                ("iframe_eval_cdp",  "Evaluate JS in iframe via CDP"),
                ("iframe_click_cdp", "Click element in iframe via CDP"),
                ("iframe_frames",    "List nested frame tree"),
            ]),
            ("Remote & Safety", &[
                ("connect_remote", "Connect to remote CDP endpoint"),
                ("safety_set",     "Set safety policy rules"),
                ("safety_status",  "Get current safety policy"),
            ]),
            ("Skills", &[
                ("skills_list", "List available agent skills"),
            ]),
            ("Screencast & Recording", &[
                ("screencast_start",  "Start CDP screencast (alias for recording_start)"),
                ("screencast_stop",   "Stop CDP screencast"),
                ("screencast_frame",  "Capture single screencast frame"),
                ("recording_start",   "Start video recording to file"),
                ("recording_stop",    "Stop video recording"),
                ("recording_status",  "Get recording status"),
                ("stream_capture",    "Capture frame from active stream"),
                ("stream_to_disk",    "Stream frames directly to disk"),
                ("recording_encode",  "Encode captured frames to video"),
                ("recording_capture", "Capture frame for recording"),
            ]),
            ("iOS Automation", &[
                ("ios_devices",    "List connected iOS devices"),
                ("ios_connect",    "Connect to iOS device"),
                ("ios_navigate",   "Navigate on iOS device"),
                ("ios_tap",        "Tap on iOS device screen"),
                ("ios_screenshot", "Take iOS device screenshot"),
                ("ios_pinch",      "Pinch gesture (zoom in/out)"),
                ("ios_long_press", "Long press at coordinates"),
                ("ios_double_tap", "Double tap at coordinates"),
                ("ios_orientation","Get or set device orientation"),
                ("ios_scroll",     "Scroll to element"),
                ("ios_script",     "Execute JS in Safari"),
                ("ios_cookies",    "Get Safari cookies"),
                ("ios_app_launch", "Launch iOS app by bundle ID"),
                ("ios_app_kill",   "Kill iOS app by bundle ID"),
                ("ios_app_state",  "Get app running state"),
                ("ios_lock",       "Lock the device"),
                ("ios_unlock",     "Unlock the device"),
                ("ios_home",       "Press home button"),
                ("ios_button",     "Press hardware button"),
                ("ios_battery",    "Get battery info"),
                ("ios_info",       "Get device info"),
                ("ios_simulator",  "Manage iOS simulators"),
                ("ios_url",        "Get current page URL"),
                ("ios_title",      "Get current page title"),
            ]),
            ("Android Automation", &[
                ("android_devices",    "List connected Android devices via ADB"),
                ("android_connect",    "Connect to Android device via UIAutomator2"),
                ("android_navigate",   "Navigate to URL in Chrome"),
                ("android_tap",        "Tap at screen coordinates"),
                ("android_swipe",      "Swipe between two points"),
                ("android_long_press", "Long press at coordinates"),
                ("android_double_tap", "Double tap at coordinates"),
                ("android_pinch",      "Pinch gesture (zoom in/out)"),
                ("android_type",       "Type text into focused element"),
                ("android_find",       "Find element by locator"),
                ("android_click",      "Click element by ID"),
                ("android_screenshot", "Take device screenshot"),
                ("android_orientation","Get or set device orientation"),
                ("android_key",        "Press hardware key by keycode"),
                ("android_app_launch", "Launch app by package name"),
                ("android_app_kill",   "Kill app by package name"),
                ("android_app_state",  "Get app running state"),
                ("android_install",    "Install APK on device"),
                ("android_script",     "Execute JS in Chrome context"),
                ("android_shell",      "Run ADB shell command"),
                ("android_push",       "Push file to device"),
                ("android_pull",       "Pull file from device"),
                ("android_info",       "Get device info via ADB"),
                ("android_battery",    "Get battery info via ADB"),
                ("android_url",        "Get current page URL"),
                ("android_title",      "Get current page title"),
            ]),
            ("Task Planning", &[
                ("task_decompose",  "Decompose goal into subtasks"),
                ("task_plan",       "Create structured task plan"),
                ("task_status",     "Get task execution status"),
                ("think",           "Structured reasoning step"),
                ("page_summary",    "Generate page summary"),
                ("error_context",   "Get error context and diagnostics"),
                ("session_context", "Get/set session context"),
                ("goal_assert",     "Assert goal completion"),
            ]),
            ("Accessibility (WCAG)", &[
                ("wcag_audit",          "Run WCAG accessibility audit"),
                ("aria_tree",           "Get ARIA role tree"),
                ("contrast_check",      "Check color contrast ratios"),
                ("landmark_nav",        "Navigate ARIA landmarks"),
                ("focus_order",         "Analyze tab/focus order"),
                ("alt_text_audit",      "Audit image alt text"),
                ("heading_structure",   "Analyze heading hierarchy"),
                ("role_validate",       "Validate ARIA roles"),
                ("keyboard_trap_detect","Detect keyboard traps"),
                ("screen_reader_sim",   "Simulate screen reader output"),
            ]),
        ]),
        ("stealth", "🥷", &[
            ("Core", &[
                ("inject",          "Inject stealth patches into page"),
                ("test",            "Test stealth effectiveness"),
                ("fingerprint",     "Get/spoof browser fingerprint"),
                ("stealth_max",     "Apply maximum stealth configuration"),
                ("stealth_score",   "Get current stealth score"),
                ("stealth_status",  "Get stealth patch status"),
                ("stealth_rotate",  "Rotate stealth identity"),
                ("detection_audit", "Audit bot detection exposure"),
            ]),
            ("Human Simulation", &[
                ("human_delay",   "Add human-like delays"),
                ("human_mouse",   "Simulate human mouse movement"),
                ("human_type",    "Simulate human typing patterns"),
                ("human_scroll",  "Simulate human scrolling"),
                ("human_profile", "Set human behavior profile"),
                ("behavior_sim",  "Start behavioral simulation"),
                ("behavior_stop", "Stop behavioral simulation"),
            ]),
            ("Anti-Detection", &[
                ("block_domains",   "Block tracking/fingerprint domains"),
                ("detect_captcha",  "Detect CAPTCHA presence"),
                ("solve_captcha",   "Attempt CAPTCHA solving"),
                ("tls_apply",       "Apply TLS fingerprint config"),
                ("webrtc_block",    "Block WebRTC leak"),
                ("battery_spoof",   "Spoof Battery API"),
                ("sensor_block",    "Block device sensor APIs"),
                ("canvas_advanced", "Advanced canvas fingerprint protection"),
                ("timezone_sync",   "Sync timezone with proxy location"),
                ("font_protect",    "Protect font enumeration"),
            ]),
        ]),
        ("data", "📊", &[
            ("HTTP", &[
                ("pipeline",  "Run data extraction pipeline"),
                ("http_get",  "Make HTTP GET request"),
                ("http_post", "Make HTTP POST request"),
            ]),
            ("Links & Graph", &[
                ("links", "Extract all page links"),
                ("graph", "Build link/content graph"),
            ]),
            ("Network Capture", &[
                ("net_capture", "Capture network traffic"),
                ("net_analyze", "Analyze captured traffic"),
                ("net_sdk",     "Generate SDK from API calls"),
                ("net_mock",    "Create mock server from traffic"),
                ("net_replay",  "Replay captured traffic"),
            ]),
            ("Extraction", &[
                ("extract_schema",   "Extract data schema from page"),
                ("extract_tables",   "Extract HTML tables as data"),
                ("extract_entities", "Extract named entities"),
                ("extract_metadata", "Extract page metadata"),
                ("extract_feeds",    "Extract RSS/Atom feeds"),
                ("extract_compact",  "Extract compact data summary"),
                ("classify_content", "Classify page content type"),
            ]),
            ("Transform", &[
                ("transform_json", "Transform data with JSONPath/jq"),
                ("export_csv",     "Export data as CSV"),
            ]),
            ("WebSocket", &[
                ("ws_connect",   "Connect to WebSocket"),
                ("ws_intercept", "Intercept WebSocket messages"),
                ("ws_send",      "Send WebSocket message"),
                ("ws_messages",  "Get WebSocket messages"),
                ("ws_close",     "Close WebSocket connection"),
            ]),
            ("Server-Sent Events", &[
                ("sse_listen",   "Listen to SSE stream"),
                ("sse_messages", "Get SSE messages"),
            ]),
            ("GraphQL", &[
                ("graphql_subscribe", "Subscribe to GraphQL subscription"),
            ]),
        ]),
        ("secure", "🔐", &[
            ("Crypto", &[
                ("encrypt", "Encrypt data"),
                ("decrypt", "Decrypt data"),
                ("pkce",    "Generate PKCE challenge/verifier"),
                ("totp",    "Generate TOTP code"),
            ]),
            ("Key-Value Store", &[
                ("kv_set",  "Set key-value pair"),
                ("kv_get",  "Get value by key"),
                ("kv_list", "List all keys"),
            ]),
            ("Passkeys", &[
                ("passkey_enable",  "Enable passkey support"),
                ("passkey_add",     "Add a passkey credential"),
                ("passkey_list",    "List stored passkeys"),
                ("passkey_log",     "Get passkey event log"),
                ("passkey_disable", "Disable passkey support"),
                ("passkey_remove",  "Remove a passkey"),
            ]),
            ("Authentication", &[
                ("auth_oauth2",      "OAuth2 authentication flow"),
                ("auth_session",     "Session-based authentication"),
                ("auth_form_login",  "Form-based login automation"),
                ("auth_mfa",         "Handle MFA challenge"),
                ("auth_status",      "Get authentication status"),
                ("auth_logout",      "Logout from session"),
                ("credential_store", "Store credentials securely"),
                ("credential_get",   "Retrieve stored credentials"),
            ]),
        ]),
        ("computer", "🖥️", &[
            ("Actions", &[
                ("act",          "Perform high-level computer action"),
                ("observe",      "Observe screen state"),
                ("batch",        "Batch multiple actions"),
                ("computer_use", "Claude-compatible computer-use action"),
            ]),
            ("Smart Interaction", &[
                ("smart_find",      "Find element using natural language"),
                ("smart_click",     "Click using natural language target"),
                ("smart_fill",      "Fill form using natural language"),
                ("element_info",    "Get element details at coordinates"),
                ("click_at_coords", "Click at x,y coordinates"),
                ("input_replay",    "Replay recorded input sequence"),
            ]),
            ("Fleet Management", &[
                ("pool_list",       "List browser pool instances"),
                ("pool_status",     "Get browser pool status"),
                ("fleet_spawn",     "Spawn fleet browser instance"),
                ("fleet_broadcast", "Broadcast action to fleet"),
                ("fleet_collect",   "Collect results from fleet"),
                ("fleet_destroy",   "Destroy fleet instance"),
                ("fleet_status",    "Get fleet status"),
                ("fleet_balance",   "Rebalance fleet workload"),
            ]),
            ("Goals & Recovery", &[
                ("goal_execute",         "Execute goal-driven sequence"),
                ("step_verify",          "Verify step completion"),
                ("auto_recover",         "Auto-recover from errors"),
                ("adaptive_retry",       "Retry with adaptive strategy"),
                ("annotated_screenshot", "Screenshot with element annotations"),
                ("multi_page_sync",      "Synchronize across multiple pages"),
            ]),
        ]),
        ("memory", "🧠", &[
            ("Memory", &[
                ("store",           "Store key-value in memory"),
                ("recall",          "Recall value from memory"),
                ("search",          "Search memory by query"),
                ("forget",          "Remove value from memory"),
                ("domain_strategy", "Get/set domain-specific strategy"),
                ("stats",           "Get memory usage stats"),
            ]),
        ]),
        ("automate", "⚙️", &[
            ("Workflows", &[
                ("workflow_validate",  "Validate workflow definition"),
                ("workflow_run",       "Run a named workflow"),
                ("workflow_while",     "While-loop workflow construct"),
                ("workflow_for_each",  "For-each workflow construct"),
                ("workflow_if",        "Conditional workflow construct"),
                ("workflow_variable",  "Set/get workflow variable"),
                ("workflow_execute",   "Execute workflow by name"),
                ("workflow_status",    "Get workflow execution status"),
            ]),
            ("Execution", &[
                ("plan",          "Create execution plan"),
                ("execute",       "Execute planned actions"),
                ("patterns",      "List available automation patterns"),
                ("batch_execute", "Execute batch of actions"),
            ]),
            ("Error Handling", &[
                ("rate_limit",       "Configure rate limiting"),
                ("retry",            "Retry failed action"),
                ("retry_adapt",      "Retry with adaptive backoff"),
                ("error_classify",   "Classify error type"),
                ("recovery_suggest", "Suggest recovery strategy"),
                ("error_history",    "Get error history"),
            ]),
            ("Checkpoints", &[
                ("checkpoint_save",    "Save execution checkpoint"),
                ("checkpoint_restore", "Restore from checkpoint"),
                ("checkpoint_list",    "List saved checkpoints"),
                ("checkpoint_delete",  "Delete a checkpoint"),
            ]),
            ("Infrastructure", &[
                ("reconnect_cdp", "Reconnect CDP session"),
                ("gc_tabs",       "Garbage-collect stale tabs"),
                ("watchdog",      "Get/configure watchdog timer"),
            ]),
        ]),
        ("perf", "📈", &[
            ("Performance", &[
                ("audit",   "Run performance audit"),
                ("budget",  "Check against performance budget"),
                ("compare", "Compare performance between runs"),
                ("trace",   "Capture performance trace"),
            ]),
            ("Visual Regression", &[
                ("vrt_run",     "Run visual regression test"),
                ("vrt_compare", "Compare visual regression results"),
                ("vrt_update",  "Update visual regression baseline"),
                ("pixel_diff",  "Pixel-level image diff"),
            ]),
        ]),
    ];

    let mut total = 0usize;
    for (tool, emoji, categories) in TOOLS {
        let action_count: usize = categories.iter().map(|(_, acts)| acts.len()).sum();
        println!(
            "{} {} ({} actions)",
            emoji,
            tool.cyan().bold(),
            action_count
        );
        for (cat_name, actions) in *categories {
            println!("  {}:", cat_name.yellow());
            for (name, desc) in *actions {
                println!("    {:<24}— {}", name, desc);
            }
        }
        total += action_count;
        println!();
    }
    println!(
        "{} {} actions across {} tools",
        "Total:".bold(),
        total,
        TOOLS.len()
    );
}
