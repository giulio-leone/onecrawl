use rmcp::{
    ErrorData as McpError,
    handler::server::{router::tool::ToolRouter, tool::Parameters},
    model::*,
    tool, tool_router,
};
use std::sync::Arc;

use crate::actions::*;
use crate::cdp_tools::*;
use crate::helpers::{ensure_page, json_ok, parse_params, McpResult};
use crate::types::*;

// ──────────────────────────── Server ────────────────────────────

#[derive(Clone)]
pub struct OneCrawlMcp {
    #[allow(dead_code)] // accessed via #[tool_router] proc macro
    tool_router: ToolRouter<Self>,
    pub(crate) store_path: Arc<String>,
    pub(crate) store_password: Arc<String>,
    pub(crate) browser: SharedBrowser,
}

#[tool_router]
impl OneCrawlMcp {
    pub fn new(store_path: String, store_password: String) -> Self {
        let browser = new_shared_browser();

        // Auto-load safety policy from env var or default path
        let policy_path = std::env::var("ONECRAWL_POLICY")
            .map(std::path::PathBuf::from)
            .unwrap_or_else(|_| {
                std::env::var("HOME")
                    .map(|h| std::path::PathBuf::from(h).join(".onecrawl/policy.json"))
                    .unwrap_or_default()
            });
        if policy_path.exists() {
            match onecrawl_cdp::SafetyState::load_from_file(&policy_path) {
                Ok(policy) => {
                    if let Ok(mut state) = browser.try_lock() {
                        state.safety = Some(onecrawl_cdp::SafetyState::new(policy));
                    }
                }
                Err(e) => {
                    eprintln!("warning: failed to load safety policy: {e}");
                }
            }
        }

        Self {
            tool_router: Self::tool_router(),
            store_path: Arc::new(store_path),
            store_password: Arc::new(store_password),
            browser,
        }
    }

    pub(crate) fn open_store(&self) -> Result<onecrawl_storage::EncryptedStore, McpError> {
        onecrawl_storage::EncryptedStore::open(
            std::path::Path::new(self.store_path.as_ref()),
            &self.store_password,
        )
        .mcp()
    }

    /// Enforce safety policy before executing an action.
    /// Returns Ok(()) if allowed, Err(McpError) if denied or over rate limit.
    async fn enforce_safety(&self, tool_name: &str, action_name: &str) -> Result<(), McpError> {
        let mut state = self.browser.lock().await;
        if let Some(ref mut safety) = state.safety {
            let cmd = format!("{}.{}", tool_name, action_name);
            match safety.check_command(&cmd) {
                onecrawl_cdp::SafetyCheck::Allowed => {}
                onecrawl_cdp::SafetyCheck::Denied(reason) => {
                    return Err(McpError::invalid_params(
                        format!("safety policy denied: {reason}"),
                        None,
                    ));
                }
                onecrawl_cdp::SafetyCheck::RequiresConfirmation(reason) => {
                    return Err(McpError::invalid_params(
                        format!("safety policy requires confirmation: {reason}"),
                        None,
                    ));
                }
            }
            match safety.check_rate_limit() {
                onecrawl_cdp::SafetyCheck::Allowed => {}
                onecrawl_cdp::SafetyCheck::Denied(reason) => {
                    return Err(McpError::invalid_params(
                        format!("rate limit: {reason}"),
                        None,
                    ));
                }
                _ => {}
            }
        }
        Ok(())
    }

    /// Record a successful action for safety policy counters.
    #[allow(dead_code)]
    async fn record_safety_action(&self) {
        let mut state = self.browser.lock().await;
        if let Some(ref mut safety) = state.safety {
            safety.record_action();
        }
    }

    /// Internal dispatch for `agent.execute_chain`.
    /// Returns `Ok(serde_json::Value)` on success or `Err(String)` with an
    /// error message for that step.
    pub(crate) async fn dispatch_chain_command(
        &self,
        cmd: &ChainCommand,
    ) -> std::result::Result<serde_json::Value, String> {
        let page = ensure_page(&self.browser)
            .await
            .map_err(|e| format!("browser error: {}", e.message))?;

        match cmd.tool.as_str() {
            "navigation.goto" => {
                let url = cmd.args.get("url")
                    .and_then(|v| v.as_str())
                    .ok_or("missing 'url' argument")?;
                onecrawl_cdp::navigation::goto(&page, url)
                    .await
                    .map_err(|e| e.to_string())?;
                let title = onecrawl_cdp::navigation::get_title(&page)
                    .await
                    .unwrap_or_default();
                Ok(serde_json::json!({ "url": url, "title": title }))
            }
            "navigation.click" => {
                let selector_raw = cmd.args.get("selector")
                    .and_then(|v| v.as_str())
                    .ok_or("missing 'selector' argument")?;
                if selector_raw.is_empty() {
                    return Err("'selector' must not be empty".into());
                }
                let selector = onecrawl_cdp::accessibility::resolve_ref(selector_raw);
                onecrawl_cdp::element::click(&page, &selector)
                    .await
                    .map_err(|e| e.to_string())?;
                Ok(serde_json::json!({ "clicked": selector_raw }))
            }
            "navigation.type" => {
                let selector_raw = cmd.args.get("selector")
                    .and_then(|v| v.as_str())
                    .ok_or("missing 'selector' argument")?;
                if selector_raw.is_empty() {
                    return Err("'selector' must not be empty".into());
                }
                let text = cmd.args.get("text")
                    .and_then(|v| v.as_str())
                    .ok_or("missing 'text' argument")?;
                let selector = onecrawl_cdp::accessibility::resolve_ref(selector_raw);
                onecrawl_cdp::element::type_text(&page, &selector, text)
                    .await
                    .map_err(|e| e.to_string())?;
                Ok(serde_json::json!({ "typed": text.len(), "selector": selector_raw }))
            }
            "navigation.wait" => {
                let selector_raw = cmd.args.get("selector")
                    .and_then(|v| v.as_str())
                    .ok_or("missing 'selector' argument")?;
                if selector_raw.is_empty() {
                    return Err("'selector' must not be empty".into());
                }
                let timeout = cmd.args.get("timeout_ms")
                    .and_then(|v| v.as_u64())
                    .unwrap_or(30_000);
                let selector = onecrawl_cdp::accessibility::resolve_ref(selector_raw);
                onecrawl_cdp::navigation::wait_for_selector(&page, &selector, timeout)
                    .await
                    .map_err(|e| e.to_string())?;
                Ok(serde_json::json!({ "found": selector_raw }))
            }
            "navigation.evaluate" => {
                let js = cmd.args.get("js")
                    .and_then(|v| v.as_str())
                    .ok_or("missing 'js' argument")?;
                let result = onecrawl_cdp::page::evaluate_js(&page, js)
                    .await
                    .map_err(|e| e.to_string())?;
                Ok(result)
            }
            "navigation.snapshot" => {
                let opts = onecrawl_cdp::accessibility::AgentSnapshotOptions {
                    interactive_only: cmd.args.get("interactive_only")
                        .and_then(|v| v.as_bool()).unwrap_or(false),
                    cursor: cmd.args.get("cursor")
                        .and_then(|v| v.as_bool()).unwrap_or(false),
                    compact: cmd.args.get("compact")
                        .and_then(|v| v.as_bool()).unwrap_or(false),
                    depth: cmd.args.get("depth")
                        .and_then(|v| v.as_u64()).map(|d| d as usize),
                    selector: cmd.args.get("selector")
                        .and_then(|v| v.as_str()).map(String::from),
                };
                let snap = onecrawl_cdp::accessibility::agent_snapshot(&page, &opts)
                    .await
                    .map_err(|e| e.to_string())?;
                Ok(serde_json::json!({
                    "snapshot": snap.snapshot,
                    "refs": snap.refs,
                    "total": snap.total
                }))
            }
            "scraping.css" => {
                let selector = cmd.args.get("selector")
                    .and_then(|v| v.as_str())
                    .ok_or("missing 'selector' argument")?;
                if selector.is_empty() {
                    return Err("'selector' must not be empty".into());
                }
                let result = onecrawl_cdp::selectors::css_select(&page, selector)
                    .await
                    .map_err(|e| e.to_string())?;
                Ok(serde_json::to_value(&result).unwrap_or_default())
            }
            "scraping.text" => {
                let selector = cmd.args.get("selector")
                    .and_then(|v| v.as_str());
                let result = onecrawl_cdp::extract::extract(
                    &page,
                    selector,
                    onecrawl_cdp::ExtractFormat::Text,
                )
                .await
                .map_err(|e| e.to_string())?;
                Ok(serde_json::to_value(&result).unwrap_or_default())
            }
            other => {
                let err = crate::agent_error::unknown_tool(other);
                Err(serde_json::to_string(&err).unwrap_or_else(|_| err.message))
            }
        }
    }

    // ── Consolidated MCP tool dispatchers ──

    #[tool(
        name = "browser",
        description = "Browser navigation, interaction, extraction, multi-tab, DOM events, session, network interception, console/dialog, device emulation, drag/drop, file upload, shadow DOM, Service Worker/PWA.\n\nActions:\n- goto {url} — Navigate to URL\n- click {selector} — Click element\n- type {selector, text} — Type into input\n- screenshot {selector?, full_page?} — Screenshot\n- pdf {landscape?} — Export PDF\n- back / forward / reload — Navigation\n- wait {selector, timeout_ms?} — Wait for element\n- evaluate {js} — Execute JavaScript\n- snapshot {interactive_only?, compact?, depth?} — A11y snapshot\n- css / xpath / find_text — Query elements\n- text / html / markdown / structured — Extract content\n- stream — Paginated extraction\n- detect_forms / fill_form — Forms\n- snapshot_diff — Diff snapshots\n- parse_a11y / parse_selector / parse_text / parse_links — Offline\n- new_tab / list_tabs / switch_tab / close_tab — Multi-tab\n- observe_mutations / get_mutations / stop_mutations / wait_for_event — DOM\n- cookies_get / cookies_set / cookies_clear — Cookies\n- storage_get / storage_set / export_session / import_session — Storage\n- intercept_enable / intercept_add_rule / intercept_remove_rule / intercept_list / intercept_disable / block_requests — Network\n- console_start / console_get / console_clear / dialog_handle / dialog_get / errors_get — Debug\n- emulate_device / emulate_geolocation / emulate_timezone / emulate_media / emulate_network — Emulation\n- drag {source, target} — Drag and drop\n- hover {selector} — Mouse hover\n- keyboard {keys, selector?} — Keyboard shortcuts\n- select {selector, value?, text?, index?} — Select dropdown option\n- upload {selector, file_path} — File upload\n- download_wait / download_list / download_set_dir — Downloads\n- shadow_query / shadow_text {host_selector, inner_selector} — Shadow DOM\n- deep_query {selector} — Pierce shadow DOM with >>>\n- context_set {key, value} / context_get {key} / context_list / context_clear / context_transfer {from_tab, to_tab, keys?} — Page context\n- form_infer {selector?} / form_auto_fill {data, selector?, confidence_threshold?} / form_validate — Smart form mapping\n- selector_heal {selector, context?} / selector_alternatives {selector, max_alternatives?} / selector_validate {selector, expected_role?, expected_text?} — Self-healing selectors\n- event_subscribe {event_type, filter?} / event_unsubscribe {event_type} / event_poll {event_type?, limit?, clear?} / event_clear — Event reactions\n- sw_register {script_url, scope?} / sw_unregister {scope?} / sw_list / sw_update {scope?} — Service Worker\n- cache_list / cache_clear — Cache Storage\n- push_simulate {title, body?, icon?, data?} — Push notifications\n- offline_mode {enabled, bypass_for?} — Offline simulation\n- set_mode {mode} — Set browser mode: 'headed' or 'headless'\n- set_stealth {enabled} — Enable/disable stealth (ON by default)\n- session_info — Get session status, mode, stealth, tabs\n- virtual_scroll_detect — Detect virtual/infinite scroll containers\n- virtual_scroll_extract {container, item_selector, max_items?} — Extract items from virtual scroll\n- wait_hydration {timeout_ms?} — Wait for SPA framework hydration\n- wait_animation {selector, timeout_ms?} — Wait for CSS/JS animations to complete\n- wait_network_idle {idle_ms?, timeout_ms?} — Wait until network is idle\n- trigger_lazy_load {selector?} — Trigger lazy-loaded elements\n- health_check — Browser health diagnostics\n- circuit_breaker {command, error?, threshold?} — Circuit breaker state management\n- token_budget {max_tokens?, selector?} — Truncate page content to token budget\n- compact_state — Minimal page state for AI agents (URL, title, element counts)\n- page_assertions {assertions} — Verify multiple page conditions at once"
    )]
    async fn tool_browser(
        &self,
        Parameters(p): Parameters<ToolAction>,
    ) -> Result<CallToolResult, McpError> {
        let action = p.action;
        let v = p.params;
        self.enforce_safety("browser", &action).await?;
        let action = BrowserAction::parse(&action)?;
        match action {
            BrowserAction::Goto => {
                let params: NavigateParams = parse_params(v, "goto")?;
                self.navigation_goto(params).await
            }
            BrowserAction::Click => {
                let params: ClickParams = parse_params(v, "click")?;
                self.navigation_click(params).await
            }
            BrowserAction::Type => {
                let params: TypeTextParams = parse_params(v, "type")?;
                self.navigation_type(params).await
            }
            BrowserAction::Screenshot => {
                let params: ScreenshotParams = parse_params(v, "screenshot")?;
                self.navigation_screenshot(params).await
            }
            BrowserAction::Pdf => {
                let params: PdfExportParams = parse_params(v, "pdf")?;
                self.navigation_pdf(params).await
            }
            BrowserAction::Back => self.navigation_back().await,
            BrowserAction::Forward => self.navigation_forward().await,
            BrowserAction::Reload => self.navigation_reload().await,
            BrowserAction::Wait => {
                let params: WaitForSelectorParams = parse_params(v, "wait")?;
                self.navigation_wait(params).await
            }
            BrowserAction::Evaluate => {
                let params: EvaluateJsParams = parse_params(v, "evaluate")?;
                self.navigation_evaluate(params).await
            }
            BrowserAction::Snapshot => {
                let params: AgentSnapshotParams = parse_params(v, "snapshot")?;
                self.navigation_snapshot(params).await
            }
            BrowserAction::Css => {
                let params: CssSelectorParams = parse_params(v, "css")?;
                self.scraping_css(params).await
            }
            BrowserAction::Xpath => {
                let params: XPathParams = parse_params(v, "xpath")?;
                self.scraping_xpath(params).await
            }
            BrowserAction::FindText => {
                let params: FindByTextParams = parse_params(v, "find_text")?;
                self.scraping_find_text(params).await
            }
            BrowserAction::Text => {
                let params: ExtractTextParams = parse_params(v, "text")?;
                self.scraping_text(params).await
            }
            BrowserAction::Html => {
                let params: ExtractHtmlParams = parse_params(v, "html")?;
                self.scraping_html(params).await
            }
            BrowserAction::Markdown => {
                let params: ExtractMarkdownParams = parse_params(v, "markdown")?;
                self.scraping_markdown(params).await
            }
            BrowserAction::Structured => self.scraping_structured().await,
            BrowserAction::Stream => {
                let params: StreamExtractParams = parse_params(v, "stream")?;
                self.scraping_stream(params).await
            }
            BrowserAction::DetectForms => {
                let params: DetectFormsParams = parse_params(v, "detect_forms")?;
                self.scraping_detect_forms(params).await
            }
            BrowserAction::FillForm => {
                let params: FillFormParams = parse_params(v, "fill_form")?;
                self.scraping_fill_form(params).await
            }
            BrowserAction::SnapshotDiff => {
                let params: SnapshotDiffParams = parse_params(v, "snapshot_diff")?;
                self.scraping_snapshot_diff(params).await
            }
            BrowserAction::ParseA11y => {
                let params: HtmlRequest = parse_params(v, "parse_a11y")?;
                self.parse_accessibility_tree(params)
            }
            BrowserAction::ParseSelector => {
                let params: SelectorRequest = parse_params(v, "parse_selector")?;
                self.query_selector(params)
            }
            BrowserAction::ParseText => {
                let params: HtmlRequest = parse_params(v, "parse_text")?;
                self.html_extract_text(params)
            }
            BrowserAction::ParseLinks => {
                let params: HtmlRequest = parse_params(v, "parse_links")?;
                self.html_extract_links(params)
            }
            // Multi-tab
            BrowserAction::NewTab => {
                let params: NewTabParams = parse_params(v, "new_tab")?;
                self.tab_new(params).await
            }
            BrowserAction::ListTabs => self.tab_list().await,
            BrowserAction::SwitchTab => {
                let params: SwitchTabParams = parse_params(v, "switch_tab")?;
                self.tab_switch(params).await
            }
            BrowserAction::CloseTab => {
                let params: CloseTabParams = parse_params(v, "close_tab")?;
                self.tab_close(params).await
            }
            // DOM events
            BrowserAction::ObserveMutations => {
                let params: ObserveMutationsParams = parse_params(v, "observe_mutations")?;
                self.observe_mutations(params).await
            }
            BrowserAction::GetMutations => self.get_mutations().await,
            BrowserAction::StopMutations => self.stop_mutations().await,
            BrowserAction::WaitForEvent => {
                let params: WaitForEventParams = parse_params(v, "wait_for_event")?;
                self.wait_for_event(params).await
            }
            // Cookies & storage
            BrowserAction::CookiesGet => {
                let params: CookiesGetParams = parse_params(v, "cookies_get")?;
                self.cookies_get(params).await
            }
            BrowserAction::CookiesSet => {
                let params: CookieSetParams = parse_params(v, "cookies_set")?;
                self.cookies_set(params).await
            }
            BrowserAction::CookiesClear => {
                let params: CookiesClearParams = parse_params(v, "cookies_clear")?;
                self.cookies_clear(params).await
            }
            BrowserAction::StorageGet => {
                let params: StorageGetParams = parse_params(v, "storage_get")?;
                self.storage_get(params).await
            }
            BrowserAction::StorageSet => {
                let params: StorageSetParams = parse_params(v, "storage_set")?;
                self.storage_set(params).await
            }
            BrowserAction::ExportSession => {
                let params: SessionExportParams = parse_params(v, "export_session")?;
                self.session_export(params).await
            }
            BrowserAction::ImportSession => {
                let params: SessionImportParams = parse_params(v, "import_session")?;
                self.session_import(params).await
            }
            // Network Interception
            BrowserAction::InterceptEnable => {
                let params: InterceptEnableParams = parse_params(v, "intercept_enable")?;
                self.intercept_enable(params).await
            }
            BrowserAction::InterceptAddRule => {
                let params: InterceptAddRuleParams = parse_params(v, "intercept_add_rule")?;
                self.intercept_add_rule(params).await
            }
            BrowserAction::InterceptRemoveRule => {
                let params: InterceptRemoveRuleParams = parse_params(v, "intercept_remove_rule")?;
                self.intercept_remove_rule(params).await
            }
            BrowserAction::InterceptList => self.intercept_list(v).await,
            BrowserAction::InterceptDisable => self.intercept_disable(v).await,
            BrowserAction::BlockRequests => {
                let params: BlockRequestsParams = parse_params(v, "block_requests")?;
                self.block_requests(params).await
            }
            // Console, Dialog & Error Capture
            BrowserAction::ConsoleStart => self.console_start(v).await,
            BrowserAction::ConsoleGet => {
                let params: ConsoleFilterParams = parse_params(v, "console_get")?;
                self.console_get(params).await
            }
            BrowserAction::ConsoleClear => self.console_clear(v).await,
            BrowserAction::DialogHandle => {
                let params: DialogHandleParams = parse_params(v, "dialog_handle")?;
                self.dialog_handle(params).await
            }
            BrowserAction::DialogGet => self.dialog_get(v).await,
            BrowserAction::ErrorsGet => self.errors_get(v).await,
            // Device Emulation
            BrowserAction::EmulateDevice => {
                let params: EmulateDeviceParams = parse_params(v, "emulate_device")?;
                self.emulate_device(params).await
            }
            BrowserAction::EmulateGeolocation => {
                let params: EmulateGeolocationParams = parse_params(v, "emulate_geolocation")?;
                self.emulate_geolocation(params).await
            }
            BrowserAction::EmulateTimezone => {
                let params: EmulateTimezoneParams = parse_params(v, "emulate_timezone")?;
                self.emulate_timezone(params).await
            }
            BrowserAction::EmulateMedia => {
                let params: EmulateMediaParams = parse_params(v, "emulate_media")?;
                self.emulate_media(params).await
            }
            BrowserAction::EmulateNetwork => {
                let params: EmulateNetworkParams = parse_params(v, "emulate_network")?;
                self.emulate_network(params).await
            }
            // Drag & Drop, Hover, Keyboard, Select
            BrowserAction::Drag => {
                let params: DragParams = parse_params(v, "drag")?;
                self.drag(params).await
            }
            BrowserAction::Hover => {
                let params: HoverParams = parse_params(v, "hover")?;
                self.hover(params).await
            }
            BrowserAction::Keyboard => {
                let params: KeyboardParams = parse_params(v, "keyboard")?;
                self.keyboard(params).await
            }
            BrowserAction::Select => {
                let params: SelectParams = parse_params(v, "select")?;
                self.select_option(params).await
            }
            // File Upload & Download
            BrowserAction::Upload => {
                let params: UploadParams = parse_params(v, "upload")?;
                self.upload(params).await
            }
            BrowserAction::DownloadWait => {
                let params: DownloadWaitParams = parse_params(v, "download_wait")?;
                self.download_wait(params).await
            }
            BrowserAction::DownloadList => self.download_list(v).await,
            BrowserAction::DownloadSetDir => {
                let params: DownloadSetDirParams = parse_params(v, "download_set_dir")?;
                self.download_set_dir(params).await
            }
            // Shadow DOM
            BrowserAction::ShadowQuery => {
                let params: ShadowQueryParams = parse_params(v, "shadow_query")?;
                self.shadow_query(params).await
            }
            BrowserAction::ShadowText => {
                let params: ShadowQueryParams = parse_params(v, "shadow_text")?;
                self.shadow_text(params).await
            }
            BrowserAction::DeepQuery => {
                let params: DeepQueryParams = parse_params(v, "deep_query")?;
                self.deep_query(params).await
            }
            // Page Context
            BrowserAction::ContextSet => {
                let params: PageContextSetParams = parse_params(v, "context_set")?;
                self.context_set(params).await
            }
            BrowserAction::ContextGet => {
                let params: PageContextGetParams = parse_params(v, "context_get")?;
                self.context_get(params).await
            }
            BrowserAction::ContextList => self.context_list(v).await,
            BrowserAction::ContextClear => self.context_clear(v).await,
            BrowserAction::ContextTransfer => {
                let params: PageContextTransferParams = parse_params(v, "context_transfer")?;
                self.context_transfer(params).await
            }
            // Smart Form Mapping
            BrowserAction::FormInfer => {
                let params: FormInferParams = parse_params(v, "form_infer")?;
                self.form_infer(params).await
            }
            BrowserAction::FormAutoFill => {
                let params: FormAutoFillParams = parse_params(v, "form_auto_fill")?;
                self.form_auto_fill(params).await
            }
            BrowserAction::FormValidate => self.form_validate(v).await,
            // Self-healing selector recovery
            BrowserAction::SelectorHeal => {
                let params: SelectorHealParams = parse_params(v, "selector_heal")?;
                self.selector_heal(params).await
            }
            BrowserAction::SelectorAlternatives => {
                let params: SelectorAlternativesParams = parse_params(v, "selector_alternatives")?;
                self.selector_alternatives(params).await
            }
            BrowserAction::SelectorValidate => {
                let params: SelectorValidateParams = parse_params(v, "selector_validate")?;
                self.selector_validate(params).await
            }
            // Event-driven reaction system
            BrowserAction::EventSubscribe => {
                let params: EventSubscribeParams = parse_params(v, "event_subscribe")?;
                self.event_subscribe(params).await
            }
            BrowserAction::EventUnsubscribe => {
                let params: EventUnsubscribeParams = parse_params(v, "event_unsubscribe")?;
                self.event_unsubscribe(params).await
            }
            BrowserAction::EventPoll => {
                let params: EventPollParams = parse_params(v, "event_poll")?;
                self.event_poll(params).await
            }
            BrowserAction::EventClear => self.event_clear(v).await,
            BrowserAction::SwRegister => {
                let params: SwRegisterParams = parse_params(v, "sw_register")?;
                self.sw_register(params).await
            }
            BrowserAction::SwUnregister => {
                let params: SwUnregisterParams = parse_params(v, "sw_unregister")?;
                self.sw_unregister(params).await
            }
            BrowserAction::SwList => self.sw_list().await,
            BrowserAction::SwUpdate => {
                let params: SwUpdateParams = parse_params(v, "sw_update")?;
                self.sw_update(params).await
            }
            BrowserAction::CacheList => self.cache_list().await,
            BrowserAction::CacheClear => self.cache_clear().await,
            BrowserAction::PushSimulate => {
                let params: PushSimulateParams = parse_params(v, "push_simulate")?;
                self.push_simulate(params).await
            }
            BrowserAction::OfflineMode => {
                let params: OfflineModeParams = parse_params(v, "offline_mode")?;
                self.offline_mode(params).await
            }
            BrowserAction::SetMode => {
                let params: SetModeParams = parse_params(v, "set_mode")?;
                self.set_mode(params).await
            }
            BrowserAction::SetStealth => {
                let params: SetStealthParams = parse_params(v, "set_stealth")?;
                self.set_stealth(params).await
            }
            BrowserAction::SessionInfo => self.session_info().await,
            BrowserAction::SpaNavWatch => {
                let params: SpaNavWatchParams = parse_params(v, "spa_nav_watch")?;
                self.spa_nav_watch(params).await
            }
            BrowserAction::FrameworkDetect => {
                let params: FrameworkDetectParams = parse_params(v, "framework_detect")?;
                self.framework_detect(params).await
            }
            BrowserAction::VirtualScrollDetect => {
                let params: VirtualScrollDetectParams = parse_params(v, "virtual_scroll_detect")?;
                self.virtual_scroll_detect(params).await
            }
            BrowserAction::VirtualScrollExtract => {
                let params: VirtualScrollExtractParams = parse_params(v, "virtual_scroll_extract")?;
                self.virtual_scroll_extract(params).await
            }
            BrowserAction::WaitHydration => {
                let params: WaitHydrationParams = parse_params(v, "wait_hydration")?;
                self.wait_hydration(params).await
            }
            BrowserAction::WaitAnimation => {
                let params: WaitAnimationParams = parse_params(v, "wait_animation")?;
                self.wait_animation(params).await
            }
            BrowserAction::WaitNetworkIdle => {
                let params: WaitNetworkIdleParams = parse_params(v, "wait_network_idle")?;
                self.wait_network_idle_smart(params).await
            }
            BrowserAction::TriggerLazyLoad => {
                let params: TriggerLazyLoadParams = parse_params(v, "trigger_lazy_load")?;
                self.trigger_lazy_load(params).await
            }
            BrowserAction::HealthCheck => {
                let params: HealthCheckParams = parse_params(v, "health_check")?;
                self.health_check(params).await
            }
            BrowserAction::CircuitBreaker => {
                let params: CircuitBreakerParams = parse_params(v, "circuit_breaker")?;
                self.circuit_breaker(params).await
            }
            BrowserAction::StateInspect => {
                let params: StateInspectParams = parse_params(v, "state_inspect")?;
                self.state_inspect(params).await
            }
            BrowserAction::FormWizardTrack => {
                let params: FormWizardTrackParams = parse_params(v, "form_wizard_track")?;
                self.form_wizard_track(params).await
            }
            BrowserAction::DynamicImportWait => {
                let params: DynamicImportWaitParams = parse_params(v, "dynamic_import_wait")?;
                self.dynamic_import_wait(params).await
            }
            BrowserAction::ParallelExec => {
                let params: ParallelExecParams = parse_params(v, "parallel_exec")?;
                self.parallel_exec(params).await
            }
            BrowserAction::TokenBudget => {
                let params: TokenBudgetParams = parse_params(v, "token_budget")?;
                self.token_budget(params).await
            }
            BrowserAction::CompactState => {
                let params: CompactStateParams = parse_params(v, "compact_state")?;
                self.compact_state(params).await
            }
            BrowserAction::PageAssertions => {
                let params: PageAssertionsParams = parse_params(v, "page_assertions")?;
                self.page_assertions(params).await
            }
        }
    }

    #[tool(
        name = "crawl",
        description = "Web crawling, robots.txt, sitemaps, and DOM snapshot management.\n\nActions:\n- spider {url, max_depth?, max_pages?, same_origin?} — Crawl website\n- robots {url, user_agent?, test_path?} — Parse robots.txt\n- sitemap {entries} — Generate XML sitemap\n- dom_snapshot {label} — Take labeled DOM snapshot\n- dom_compare {before, after} — Compare two snapshots"
    )]
    async fn tool_crawl(
        &self,
        Parameters(p): Parameters<ToolAction>,
    ) -> Result<CallToolResult, McpError> {
        let action = p.action;
        let v = p.params;
        self.enforce_safety("crawl", &action).await?;
        let action = CrawlAction::parse(&action)?;
        match action {
            CrawlAction::Spider => {
                let params: SpiderCrawlParams = parse_params(v, "spider")?;
                self.crawling_spider(params).await
            }
            CrawlAction::Robots => {
                let params: CheckRobotsParams = parse_params(v, "robots")?;
                self.crawling_robots(params).await
            }
            CrawlAction::Sitemap => {
                let params: GenerateSitemapParams = parse_params(v, "sitemap")?;
                self.crawling_sitemap(params)
            }
            CrawlAction::DomSnapshot => {
                let params: TakeSnapshotParams = parse_params(v, "dom_snapshot")?;
                self.crawling_snapshot(params).await
            }
            CrawlAction::DomCompare => {
                let params: CompareSnapshotsParams = parse_params(v, "dom_compare")?;
                self.crawling_compare(params).await
            }
        }
    }

    #[tool(
        name = "agent",
        description = "AI agent orchestration — command chains, element screenshots, API capture, iframes (same-origin + cross-origin CDP), remote CDP, safety policies, skills, screencast, recording, iOS automation, WCAG accessibility auditing, session context, auto-chain, and structured reasoning.\n\nActions:\n- execute_chain {commands} — Execute multiple commands in sequence\n- element_screenshot {selector} — Screenshot a specific element\n- api_capture_start — Start capturing API calls\n- api_capture_summary — Get captured API call summary\n- iframe_list — List all iframes on page (DOM-based)\n- iframe_snapshot {index, interactive_only?} — Snapshot an iframe\n- iframe_eval_cdp {frame_url, expression} — Evaluate JS in cross-origin iframe via CDP (bypasses SOP)\n- iframe_click_cdp {frame_url, selector, human_like?} — Click element inside cross-origin iframe\n- iframe_frames — List all frames via CDP (includes cross-origin)\n- connect_remote {ws_url, headers?} — Connect to remote CDP\n- safety_set {policy} — Set safety policy JSON\n- safety_status — Get current safety policy status\n- skills_list — List available skills\n- screencast_start {quality?, max_width?, max_height?} — Start screencast\n- screencast_stop — Stop screencast\n- screencast_frame — Get latest screencast frame\n- recording_start {output?, fps?, quality?} — Start video recording\n- recording_stop — Stop recording and save\n- recording_status — Get recording status\n- ios_devices — List iOS devices\n- ios_connect {device_id, wda_url?} — Connect to iOS device\n- ios_navigate {url} — Navigate iOS Safari\n- ios_tap {x, y} — Tap on iOS screen\n- ios_screenshot — Take iOS screenshot\n- task_decompose {goal, context?, max_depth?} — Decompose goal into subtasks\n- task_plan {tasks, strategy?} — Generate execution plan\n- task_status — Get current task plans status\n- vision_describe {selector?, format?} — Describe page/element visually\n- vision_locate {description, strategy?} — Find element by description\n- vision_compare {baseline, current?, threshold?} — Compare page states\n- wcag_audit {level?, selector?} — Full WCAG compliance audit\n- aria_tree — Build ARIA accessibility tree\n- contrast_check {selector?, threshold?} — Color contrast ratio check\n- landmark_nav — List ARIA landmark regions\n- focus_order — Map tab/focus order of interactive elements\n- alt_text_audit {selector?, include_decorative?} — Audit image alt text\n- heading_structure — Validate heading hierarchy (h1-h6)\n- role_validate {selector?, roles?} — Validate ARIA roles and properties\n- keyboard_trap_detect — Detect keyboard focus traps\n- screen_reader_sim {selector?, max_elements?} — Simulate screen reader output\n- session_context {command, key?, value?} — Store/retrieve persistent context (set/get/get_all/clear)\n- auto_chain {actions, on_error?, max_retries?} — Execute JS chain with error recovery\n- think {context?} — Structured reasoning: observe page state and recommend actions\n- plan_execute {steps, stop_on_error?} — Execute multi-step JS plan with reporting\n- page_summary — AI-optimized page summary (headings, nav, forms, errors)\n- error_context — Get error info for debugging (console, network, DOM errors)"
    )]
    async fn tool_agent(
        &self,
        Parameters(p): Parameters<ToolAction>,
    ) -> Result<CallToolResult, McpError> {
        let action = p.action;
        let v = p.params;
        // Don't enforce safety on safety management commands themselves
        if action != "safety_set" && action != "safety_status" {
            self.enforce_safety("agent", &action).await?;
        }
        let action = AgentAction::parse(&action)?;
        match action {
            AgentAction::ExecuteChain => {
                let params: ExecuteChainParams = parse_params(v, "execute_chain")?;
                self.agent_execute_chain(params).await
            }
            AgentAction::ElementScreenshot => {
                let params: ElementScreenshotParams = parse_params(v, "element_screenshot")?;
                self.agent_element_screenshot(params).await
            }
            AgentAction::ApiCaptureStart => {
                let params: ApiCaptureStartParams = parse_params(v, "api_capture_start")?;
                self.agent_api_capture_start(params).await
            }
            AgentAction::ApiCaptureSummary => {
                let params: ApiCaptureSummaryParams = parse_params(v, "api_capture_summary")?;
                self.agent_api_capture_summary(params).await
            }
            AgentAction::IframeList => {
                let params: IframeListParams = parse_params(v, "iframe_list")?;
                self.agent_iframe_list(params).await
            }
            AgentAction::IframeSnapshot => {
                let params: IframeSnapshotParams = parse_params(v, "iframe_snapshot")?;
                self.agent_iframe_snapshot(params).await
            }
            AgentAction::IframeEvalCdp => {
                let params: IframeEvalCdpParams = parse_params(v, "iframe_eval_cdp")?;
                self.agent_iframe_eval_cdp(params).await
            }
            AgentAction::IframeClickCdp => {
                let params: IframeClickCdpParams = parse_params(v, "iframe_click_cdp")?;
                self.agent_iframe_click_cdp(params).await
            }
            AgentAction::IframeFrames => {
                self.agent_iframe_frames().await
            }
            AgentAction::ConnectRemote => {
                let params: RemoteCdpParams = parse_params(v, "connect_remote")?;
                self.agent_connect_remote(params).await
            }
            AgentAction::SafetySet => {
                let params: SafetyPolicySetParams = parse_params(v, "safety_set")?;
                self.agent_safety_policy_set(params).await
            }
            AgentAction::SafetyStatus => {
                let params: SafetyStatusParams = parse_params(v, "safety_status")?;
                self.agent_safety_status(params).await
            }
            AgentAction::SkillsList => {
                let params: SkillsListParams = parse_params(v, "skills_list")?;
                self.agent_skills_list(params)
            }
            AgentAction::ScreencastStart => {
                let params: ScreencastStartParams = parse_params(v, "screencast_start")?;
                self.agent_screencast_start(params).await
            }
            AgentAction::ScreencastStop => {
                let params: ScreencastStopParams = parse_params(v, "screencast_stop")?;
                self.agent_screencast_stop(params).await
            }
            AgentAction::ScreencastFrame => {
                let params: ScreencastFrameParams = parse_params(v, "screencast_frame")?;
                self.agent_screencast_frame(params).await
            }
            AgentAction::RecordingStart => {
                let params: RecordingStartParams = parse_params(v, "recording_start")?;
                self.agent_recording_start(params).await
            }
            AgentAction::RecordingStop => {
                let params: RecordingStopParams = parse_params(v, "recording_stop")?;
                self.agent_recording_stop(params).await
            }
            AgentAction::RecordingStatus => {
                let params: RecordingStatusParams = parse_params(v, "recording_status")?;
                self.agent_recording_status(params).await
            }
            AgentAction::StreamCapture => {
                let params: StreamCaptureParams = parse_params(v, "stream_capture")?;
                self.agent_stream_capture(params).await
            }
            AgentAction::StreamToDisk => {
                let params: StreamToDiskParams = parse_params(v, "stream_to_disk")?;
                self.agent_stream_to_disk(params).await
            }
            AgentAction::RecordingEncode => {
                let params: RecordingEncodeParams = parse_params(v, "recording_encode")?;
                self.agent_recording_encode(params).await
            }
            AgentAction::RecordingCapture => {
                let params: RecordingCaptureParams = parse_params(v, "recording_capture")?;
                self.agent_recording_capture(params).await
            }
            AgentAction::IosDevices => {
                let params: IosDevicesParams = parse_params(v, "ios_devices")?;
                self.agent_ios_devices(params).await
            }
            AgentAction::IosConnect => {
                let params: IosConnectParams = parse_params(v, "ios_connect")?;
                self.agent_ios_connect(params).await
            }
            AgentAction::IosNavigate => {
                let params: IosNavigateParams = parse_params(v, "ios_navigate")?;
                self.agent_ios_navigate(params).await
            }
            AgentAction::IosTap => {
                let params: IosTapParams = parse_params(v, "ios_tap")?;
                self.agent_ios_tap(params).await
            }
            AgentAction::IosScreenshot => {
                let params: IosScreenshotParams = parse_params(v, "ios_screenshot")?;
                self.agent_ios_screenshot(params).await
            }
            // Task decomposition engine
            AgentAction::TaskDecompose => {
                let params: TaskDecomposeParams = parse_params(v, "task_decompose")?;
                self.task_decompose(params).await
            }
            AgentAction::TaskPlan => {
                let params: TaskPlanParams = parse_params(v, "task_plan")?;
                self.task_plan(params).await
            }
            AgentAction::TaskStatus => self.task_status(v).await,
            // Vision/LLM observation layer
            AgentAction::VisionDescribe => {
                let params: VisionDescribeParams = parse_params(v, "vision_describe")?;
                self.vision_describe(params).await
            }
            AgentAction::VisionLocate => {
                let params: VisionLocateParams = parse_params(v, "vision_locate")?;
                self.vision_locate(params).await
            }
            AgentAction::VisionCompare => {
                let params: VisionCompareParams = parse_params(v, "vision_compare")?;
                self.vision_compare(params).await
            }
            AgentAction::WcagAudit => {
                let params: WcagAuditParams = parse_params(v, "wcag_audit")?;
                self.wcag_audit(params).await
            }
            AgentAction::AriaTree => self.aria_tree().await,
            AgentAction::ContrastCheck => {
                let params: ContrastCheckParams = parse_params(v, "contrast_check")?;
                self.contrast_check(params).await
            }
            AgentAction::LandmarkNav => self.landmark_nav().await,
            AgentAction::FocusOrder => self.focus_order().await,
            AgentAction::AltTextAudit => {
                let params: AltTextAuditParams = parse_params(v, "alt_text_audit")?;
                self.alt_text_audit(params).await
            }
            AgentAction::HeadingStructure => self.heading_structure().await,
            AgentAction::RoleValidate => {
                let params: RoleValidateParams = parse_params(v, "role_validate")?;
                self.role_validate(params).await
            }
            AgentAction::KeyboardTrapDetect => self.keyboard_trap_detect().await,
            AgentAction::ScreenReaderSim => {
                let params: ScreenReaderSimParams = parse_params(v, "screen_reader_sim")?;
                self.screen_reader_sim(params).await
            }
            AgentAction::AgentLoop => {
                let params: AgentLoopParams = parse_params(v, "agent_loop")?;
                self.agent_loop(params).await
            }
            AgentAction::GoalAssert => {
                let params: GoalAssertParams = parse_params(v, "goal_assert")?;
                self.goal_assert(params).await
            }
            AgentAction::AnnotatedObserve => {
                let params: AnnotatedObserveParams = parse_params(v, "annotated_observe")?;
                self.annotated_observe(params).await
            }
            AgentAction::SessionContext => {
                let params: SessionContextParams = parse_params(v, "session_context")?;
                self.session_context(params).await
            }
            AgentAction::AutoChain => {
                let params: AutoChainParams = parse_params(v, "auto_chain")?;
                self.auto_chain(params).await
            }
            AgentAction::Think => {
                let params: ThinkParams = parse_params(v, "think")?;
                self.think(params).await
            }
            AgentAction::PlanExecute => {
                let params: PlanExecuteParams = parse_params(v, "plan_execute")?;
                self.plan_execute(params).await
            }
            AgentAction::PageSummary => {
                let params: PageSummaryParams = parse_params(v, "page_summary")?;
                self.page_summary(params).await
            }
            AgentAction::ErrorContext => {
                let params: ErrorContextParams = parse_params(v, "error_context")?;
                self.error_context(params).await
            }
        }
    }

    #[tool(
        name = "stealth",
        description = "Anti-detection, bot evasion, stealth patches, fingerprinting, CAPTCHA detection/solving, and human behavior simulation.\n\nActions:\n- inject — Inject stealth patches into page\n- test — Test if current page detects bot\n- fingerprint {user_agent?} — Generate and apply browser fingerprint\n- block_domains {domains} — Block tracking domains\n- detect_captcha — Detect CAPTCHAs on page\n- solve_captcha {captcha_type?, timeout_ms?} — Solve CAPTCHA: 'recaptcha_checkbox' (CDP cross-origin frame click), 'recaptcha_audio' (Whisper STT), 'turnstile', 'auto'\n- human_delay {min_ms?, max_ms?, pattern?} — Random human-like delay\n- human_mouse {target, speed?, curve?} — Bézier curve mouse movement\n- human_type {selector, text, speed?, mistakes?} — Natural typing with typos\n- human_scroll {direction?, amount?, speed?} — Human-like scroll behavior\n- human_profile {profile?} — Set human behavior profile (casual/fast/careful)\n- stealth_max {features?} — Enable maximum stealth (all patches + human sim)\n- stealth_score — Score current page stealth level\n- tls_apply {profile?} — Apply TLS fingerprint profile (chrome-win/mac, firefox-win, safari-mac, edge-win, random, detect)\n- webrtc_block {mode?} — Block WebRTC leaks ('block' or 'turn_only')\n- battery_spoof {charging?, level?} — Spoof Battery API (desktop disguise)\n- sensor_block {sensors?} — Block device sensor APIs (gyroscope, accelerometer, etc.)\n- canvas_advanced {intensity?} — Advanced canvas fingerprint noise (Gaussian, 0.0-10.0)\n- timezone_sync {timezone} — Spoof IANA timezone across all JS APIs\n- font_protect — Limit font enumeration to cross-platform subset\n- behavior_sim {interval_ms?, command?} — Start/stop continuous human behavior simulation\n- behavior_stop — Stop behavior simulation\n- stealth_rotate {per_page?} — Auto-rotate fingerprint + stealth profile (fresh identity)\n- detection_audit {detailed?} — Comprehensive bot detection test suite (12 tests, A+ to F grade)\n- stealth_status — Comprehensive stealth status report (webdriver, plugins, fingerprint details)"
    )]
    async fn tool_stealth(
        &self,
        Parameters(p): Parameters<ToolAction>,
    ) -> Result<CallToolResult, McpError> {
        let action = p.action;
        let v = p.params;
        self.enforce_safety("stealth", &action).await?;
        let action = StealthAction::parse(&action)?;
        match action {
            StealthAction::Inject => {
                let params: InjectStealthParams = parse_params(v, "inject")?;
                self.stealth_inject(params).await
            }
            StealthAction::Test => {
                let params: BotDetectionTestParams = parse_params(v, "test")?;
                self.stealth_test(params).await
            }
            StealthAction::Fingerprint => {
                let params: ApplyFingerprintParams = parse_params(v, "fingerprint")?;
                self.stealth_fingerprint(params).await
            }
            StealthAction::BlockDomains => {
                let params: BlockDomainsParams = parse_params(v, "block_domains")?;
                self.stealth_block_domains(params).await
            }
            StealthAction::DetectCaptcha => {
                let params: DetectCaptchaParams = parse_params(v, "detect_captcha")?;
                self.stealth_detect_captcha(params).await
            }
            StealthAction::SolveCaptcha => {
                let params: SolveCaptchaParams = parse_params(v, "solve_captcha")?;
                self.stealth_solve_captcha(params).await
            }
            StealthAction::HumanDelay => {
                let params: HumanDelayParams = parse_params(v, "human_delay")?;
                self.human_delay(params).await
            }
            StealthAction::HumanMouse => {
                let params: HumanMouseParams = parse_params(v, "human_mouse")?;
                self.human_mouse(params).await
            }
            StealthAction::HumanType => {
                let params: HumanTypeParams = parse_params(v, "human_type")?;
                self.human_type(params).await
            }
            StealthAction::HumanScroll => {
                let params: HumanScrollParams = parse_params(v, "human_scroll")?;
                self.human_scroll(params).await
            }
            StealthAction::HumanProfile => {
                let params: HumanProfileParams = parse_params(v, "human_profile")?;
                self.human_profile(params).await
            }
            StealthAction::StealthMax => {
                let params: StealthMaxParams = parse_params(v, "stealth_max")?;
                self.stealth_max(params).await
            }
            StealthAction::StealthScore => self.stealth_score().await,
            StealthAction::TlsApply => {
                let params: TlsApplyParams = parse_params(v, "tls_apply")?;
                self.stealth_tls_apply(params).await
            }
            StealthAction::WebrtcBlock => {
                let params: WebrtcBlockParams = parse_params(v, "webrtc_block")?;
                self.stealth_webrtc_block(params).await
            }
            StealthAction::BatterySpoof => {
                let params: BatterySpoofParams = parse_params(v, "battery_spoof")?;
                self.stealth_battery_spoof(params).await
            }
            StealthAction::SensorBlock => {
                let params: SensorBlockParams = parse_params(v, "sensor_block")?;
                self.stealth_sensor_block(params).await
            }
            StealthAction::CanvasAdvanced => {
                let params: CanvasAdvancedParams = parse_params(v, "canvas_advanced")?;
                self.stealth_canvas_advanced(params).await
            }
            StealthAction::TimezoneSync => {
                let params: TimezoneSyncParams = parse_params(v, "timezone_sync")?;
                self.stealth_timezone_sync(params).await
            }
            StealthAction::FontProtect => {
                let params: FontProtectParams = parse_params(v, "font_protect")?;
                self.stealth_font_protect(params).await
            }
            StealthAction::BehaviorSim => {
                let params: BehaviorSimParams = parse_params(v, "behavior_sim")?;
                self.stealth_behavior_sim(params).await
            }
            StealthAction::BehaviorStop => {
                drop(v);
                let page = ensure_page(&self.browser).await?;
                onecrawl_cdp::antibot::stop_behavior_simulation(&page).await.mcp()?;
                json_ok(&serde_json::json!({ "action": "behavior_stop", "status": "stopped" }))
            }
            StealthAction::StealthRotate => {
                let params: StealthRotateParams = parse_params(v, "stealth_rotate")?;
                self.stealth_rotate(params).await
            }
            StealthAction::DetectionAudit => {
                let params: DetectionAuditParams = parse_params(v, "detection_audit")?;
                self.stealth_detection_audit(params).await
            }
            StealthAction::StealthStatus => {
                let params: StealthStatusParams = parse_params(v, "stealth_status")?;
                self.stealth_status(params).await
            }
        }
    }

    #[tool(
        name = "data",
        description = "Data processing, HTTP requests, link analysis, network intelligence, structured data extraction, WebSocket/SSE/GraphQL real-time protocols.\n\nActions:\n- pipeline {input, steps} — Multi-step data pipeline\n- http_get {url, headers?} — HTTP GET request\n- http_post {url, body?, content_type?, headers?} — HTTP POST request\n- links {base_url?} — Extract link graph from page\n- graph {edges} — Analyze link graph\n- net_capture {duration_ms?} — Capture network traffic\n- net_analyze {traffic?} — Analyze captured API traffic\n- net_sdk {traffic, language?} — Generate API SDK code\n- net_mock {traffic?} — Generate mock server config\n- net_replay {sequence} — Replay captured requests\n- extract_schema {schema_type?} — Extract JSON-LD, OpenGraph, Twitter Card, microdata\n- extract_tables {selector?, format?, headers?} — Extract HTML tables to JSON/CSV\n- extract_entities {types?, selector?} — Extract emails, phones, URLs, dates, prices\n- classify_content {strategy?, selector?} — Classify page content type and structure\n- transform_json {data, transform, output_format?} — Transform JSON data (flatten, keys, values, unique, field access)\n- export_csv {data, columns?, delimiter?} — Export JSON array to CSV\n- extract_metadata {include_og?, include_twitter?, include_all?} — Extract page metadata\n- extract_feeds {feed_type?} — Discover RSS, Atom, JSON feeds\n- ws_connect {url, protocols?} — Connect to WebSocket server\n- ws_intercept {url_pattern?, capture_only?} — Intercept WebSocket traffic\n- ws_send {target, message} — Send WebSocket message\n- ws_messages {url_filter?, limit?} — Get captured WebSocket messages\n- ws_close {target?} — Close WebSocket connections\n- sse_listen {url, duration_ms?} — Listen to Server-Sent Events\n- sse_messages {url_filter?, limit?} — Get captured SSE messages\n- graphql_subscribe {url, query, variables?, duration_ms?} — GraphQL subscription\n- extract_compact {format?, max_tokens?} — Extract page content in agent-optimized format with token budget"
    )]
    async fn tool_data(
        &self,
        Parameters(p): Parameters<ToolAction>,
    ) -> Result<CallToolResult, McpError> {
        let action = p.action;
        let v = p.params;
        self.enforce_safety("data", &action).await?;
        let action = DataAction::parse(&action)?;
        match action {
            DataAction::Pipeline => {
                let params: PipelineExecuteParams = parse_params(v, "pipeline")?;
                self.data_pipeline(params)
            }
            DataAction::HttpGet => {
                let params: HttpGetParams = parse_params(v, "http_get")?;
                self.data_http_get(params).await
            }
            DataAction::HttpPost => {
                let params: HttpPostParams = parse_params(v, "http_post")?;
                self.data_http_post(params).await
            }
            DataAction::Links => {
                let params: ExtractLinksParams = parse_params(v, "links")?;
                self.data_links(params).await
            }
            DataAction::Graph => {
                let params: AnalyzeGraphParams = parse_params(v, "graph")?;
                self.data_graph(params)
            }
            DataAction::NetCapture => {
                let params: NetIntelCaptureParams = parse_params(v, "net_capture")?;
                self.net_capture(params).await
            }
            DataAction::NetAnalyze => {
                let params: NetIntelAnalyzeParams = parse_params(v, "net_analyze")?;
                self.net_analyze(params).await
            }
            DataAction::NetSdk => {
                let params: NetIntelSdkParams = parse_params(v, "net_sdk")?;
                self.net_sdk(params).await
            }
            DataAction::NetMock => {
                let params: NetIntelMockParams = parse_params(v, "net_mock")?;
                self.net_mock(params).await
            }
            DataAction::NetReplay => {
                let params: NetIntelReplayParams = parse_params(v, "net_replay")?;
                self.net_replay(params).await
            }
            // Structured data pipeline
            DataAction::ExtractSchema => {
                let params: ExtractSchemaParams = parse_params(v, "extract_schema")?;
                self.extract_schema(params).await
            }
            DataAction::ExtractTables => {
                let params: ExtractTablesParams = parse_params(v, "extract_tables")?;
                self.extract_tables(params).await
            }
            DataAction::ExtractEntities => {
                let params: ExtractEntitiesParams = parse_params(v, "extract_entities")?;
                self.extract_entities(params).await
            }
            DataAction::ClassifyContent => {
                let params: ClassifyContentParams = parse_params(v, "classify_content")?;
                self.classify_content(params).await
            }
            DataAction::TransformJson => {
                let params: TransformJsonParams = parse_params(v, "transform_json")?;
                self.transform_json(params)
            }
            DataAction::ExportCsv => {
                let params: ExportCsvParams = parse_params(v, "export_csv")?;
                self.export_csv(params)
            }
            DataAction::ExtractMetadata => {
                let params: ExtractMetadataParams = parse_params(v, "extract_metadata")?;
                self.extract_metadata(params).await
            }
            DataAction::ExtractFeeds => {
                let params: ExtractFeedsParams = parse_params(v, "extract_feeds")?;
                self.extract_feeds(params).await
            }
            DataAction::WsConnect => {
                let params: WsConnectParams = parse_params(v, "ws_connect")?;
                self.ws_connect(params).await
            }
            DataAction::WsIntercept => {
                let params: WsInterceptParams = parse_params(v, "ws_intercept")?;
                self.ws_intercept(params).await
            }
            DataAction::WsSend => {
                let params: WsSendParams = parse_params(v, "ws_send")?;
                self.ws_send(params).await
            }
            DataAction::WsMessages => {
                let params: WsMessagesParams = parse_params(v, "ws_messages")?;
                self.ws_messages(params).await
            }
            DataAction::WsClose => {
                let params: WsCloseParams = parse_params(v, "ws_close")?;
                self.ws_close(params).await
            }
            DataAction::SseListen => {
                let params: SseListenParams = parse_params(v, "sse_listen")?;
                self.sse_listen(params).await
            }
            DataAction::SseMessages => {
                let params: SseMessagesParams = parse_params(v, "sse_messages")?;
                self.sse_messages(params).await
            }
            DataAction::GraphqlSubscribe => {
                let params: GraphqlSubscribeParams = parse_params(v, "graphql_subscribe")?;
                self.graphql_subscribe(params).await
            }
            DataAction::ExtractCompact => {
                let params: ExtractCompactParams = parse_params(v, "extract_compact")?;
                self.extract_compact(params).await
            }
        }
    }

    #[tool(
        name = "secure",
        description = "Cryptography, encrypted storage, WebAuthn passkey management, and authentication flows.\n\nActions:\n- encrypt {plaintext, password} — AES-256-GCM encryption\n- decrypt {ciphertext, password} — AES-256-GCM decryption\n- pkce — Generate PKCE S256 challenge pair\n- totp {secret} — Generate 6-digit TOTP code\n- kv_set {key, value} — Store encrypted key-value pair\n- kv_get {key} — Retrieve value by key\n- kv_list — List all stored keys\n- passkey_enable — Enable virtual WebAuthn authenticator\n- passkey_add {rp_id, user_name} — Add passkey credential\n- passkey_list — List stored passkeys\n- passkey_log — Get WebAuthn operation log\n- passkey_disable — Disable authenticator\n- passkey_remove {credential_id} — Remove passkey by ID\n- auth_oauth2 {auth_url, token_url, client_id} — OAuth2 authorization flow with PKCE\n- auth_session {name, export?, import_data?} — Export/import browser session\n- auth_form_login {url, username, password} — Automated form-based login\n- auth_mfa {mfa_type, totp_secret?, code?} — Handle MFA/2FA challenges\n- auth_status — Check authentication status\n- auth_logout — Clear all auth state\n- credential_store {label, username, password} — Store credentials in encrypted vault\n- credential_get {label} — Retrieve stored credentials"
    )]
    async fn tool_secure(
        &self,
        Parameters(p): Parameters<ToolAction>,
    ) -> Result<CallToolResult, McpError> {
        let action = p.action;
        let v = p.params;
        self.enforce_safety("secure", &action).await?;
        let action = SecureAction::parse(&action)?;
        match action {
            SecureAction::Encrypt => {
                let params: EncryptRequest = parse_params(v, "encrypt")?;
                self.encrypt(params)
            }
            SecureAction::Decrypt => {
                let params: DecryptRequest = parse_params(v, "decrypt")?;
                self.decrypt(params)
            }
            SecureAction::Pkce => self.generate_pkce(),
            SecureAction::Totp => {
                let params: TotpRequest = parse_params(v, "totp")?;
                self.generate_totp(params)
            }
            SecureAction::KvSet => {
                let params: StoreSetRequest = parse_params(v, "kv_set")?;
                self.store_set(params)
            }
            SecureAction::KvGet => {
                let params: StoreGetRequest = parse_params(v, "kv_get")?;
                self.store_get(params)
            }
            SecureAction::KvList => self.store_list(),
            SecureAction::PasskeyEnable => {
                let params: PasskeyEnableParams = parse_params(v, "passkey_enable")?;
                self.auth_passkey_enable(params).await
            }
            SecureAction::PasskeyAdd => {
                let params: PasskeyAddParams = parse_params(v, "passkey_add")?;
                self.auth_passkey_add(params).await
            }
            SecureAction::PasskeyList => {
                let params: PasskeyListParams = parse_params(v, "passkey_list")?;
                self.auth_passkey_list(params).await
            }
            SecureAction::PasskeyLog => {
                let params: PasskeyLogParams = parse_params(v, "passkey_log")?;
                self.auth_passkey_log(params).await
            }
            SecureAction::PasskeyDisable => {
                let params: PasskeyDisableParams = parse_params(v, "passkey_disable")?;
                self.auth_passkey_disable(params).await
            }
            SecureAction::PasskeyRemove => {
                let params: PasskeyRemoveParams = parse_params(v, "passkey_remove")?;
                self.auth_passkey_remove(params).await
            }
            // Authentication flows
            SecureAction::AuthOauth2 => {
                let params: AuthOauth2Params = parse_params(v, "auth_oauth2")?;
                self.auth_oauth2(params).await
            }
            SecureAction::AuthSession => {
                let params: AuthSessionParams = parse_params(v, "auth_session")?;
                self.auth_session(params).await
            }
            SecureAction::AuthFormLogin => {
                let params: AuthFormLoginParams = parse_params(v, "auth_form_login")?;
                self.auth_form_login(params).await
            }
            SecureAction::AuthMfa => {
                let params: AuthMfaParams = parse_params(v, "auth_mfa")?;
                self.auth_mfa(params).await
            }
            SecureAction::AuthStatus => self.auth_status_check().await,
            SecureAction::AuthLogout => self.auth_logout().await,
            SecureAction::CredentialStore => {
                let params: CredentialStoreParams = parse_params(v, "credential_store")?;
                self.credential_store(params)
            }
            SecureAction::CredentialGet => {
                let params: CredentialGetParams = parse_params(v, "credential_get")?;
                self.credential_get(params)
            }
        }
    }

    #[tool(
        name = "computer",
        description = "AI computer use protocol, smart element resolution, browser pool, multi-browser fleet, autonomous goal execution, coordinate clicks, multi-page sync, and input replay.\n\nActions:\n- act {action_type, coordinate?, text?, key?} — Perform computer action\n- observe {observation_type?} — Observe screen state\n- batch {actions} — Execute multiple actions in sequence\n- smart_find {description, strategy?} — Find element by description\n- smart_click {description} — Click element by description\n- smart_fill {description, value} — Fill input by description\n- pool_list — List browser pool instances\n- pool_status — Get pool status and stats\n- fleet_spawn {count?, fleet_name?} — Launch multi-browser fleet\n- fleet_broadcast {fleet_name, action} — Send action to all fleet instances\n- fleet_collect {fleet_name, selector?, attribute?} — Collect data from all instances\n- fleet_destroy {fleet_name} — Terminate fleet\n- fleet_status — Get all fleet statuses\n- fleet_balance {fleet_name, urls} — Distribute URLs across fleet\n- computer_use {goal, url?, max_steps?, screenshots?} — Autonomous goal execution with planning\n- goal_execute {plan_id, from_step?, until_step?} — Execute plan steps\n- step_verify {plan_id, step_id, expect?} — Verify step completion\n- auto_recover {plan_id, step_id, error?, max_retries?} — Auto-recover from failed steps\n- click_at_coords {x, y} — Click at viewport coordinates with element feedback\n- multi_page_sync {tab_indices?} — Get synchronized state from all pages\n- input_replay {events} — Replay a sequence of input events (click/type/scroll/wait)\n- element_info {selector} — Detailed element inspection (tag, classes, rect, ARIA, visibility)"
    )]
    async fn tool_computer(
        &self,
        Parameters(p): Parameters<ToolAction>,
    ) -> Result<CallToolResult, McpError> {
        let action = p.action;
        let v = p.params;
        self.enforce_safety("computer", &action).await?;
        let action = ComputerAction::parse(&action)?;
        match action {
            ComputerAction::Act => {
                let params: ComputerUseActionParams = parse_params(v, "act")?;
                self.computer_act(params).await
            }
            ComputerAction::Observe => {
                let params: ComputerUseObserveParams = parse_params(v, "observe")?;
                self.computer_observe(params).await
            }
            ComputerAction::Batch => {
                let params: ComputerUseBatchParams = parse_params(v, "batch")?;
                self.computer_batch(params).await
            }
            ComputerAction::SmartFind => {
                let params: SmartFindParams = parse_params(v, "smart_find")?;
                self.smart_find(params).await
            }
            ComputerAction::SmartClick => {
                let params: SmartClickParams = parse_params(v, "smart_click")?;
                self.smart_click(params).await
            }
            ComputerAction::SmartFill => {
                let params: SmartFillParams = parse_params(v, "smart_fill")?;
                self.smart_fill(params).await
            }
            ComputerAction::PoolList => {
                let params: PoolListParams = parse_params(v, "pool_list")?;
                self.pool_list(params).await
            }
            ComputerAction::PoolStatus => {
                let params: PoolStatusParams = parse_params(v, "pool_status")?;
                self.pool_status(params).await
            }
            // Multi-browser fleet
            ComputerAction::FleetSpawn => {
                let params: FleetSpawnParams = parse_params(v, "fleet_spawn")?;
                self.fleet_spawn(params).await
            }
            ComputerAction::FleetBroadcast => {
                let params: FleetBroadcastParams = parse_params(v, "fleet_broadcast")?;
                self.fleet_broadcast(params).await
            }
            ComputerAction::FleetCollect => {
                let params: FleetCollectParams = parse_params(v, "fleet_collect")?;
                self.fleet_collect(params).await
            }
            ComputerAction::FleetDestroy => {
                let params: FleetDestroyParams = parse_params(v, "fleet_destroy")?;
                self.fleet_destroy(params).await
            }
            ComputerAction::FleetStatus => self.fleet_status().await,
            ComputerAction::FleetBalance => {
                let params: FleetBalanceParams = parse_params(v, "fleet_balance")?;
                self.fleet_balance(params).await
            }
            ComputerAction::ComputerUse => {
                let params: ComputerUseParams = parse_params(v, "computer_use")?;
                self.computer_use(params).await
            }
            ComputerAction::GoalExecute => {
                let params: GoalExecuteParams = parse_params(v, "goal_execute")?;
                self.goal_execute(params).await
            }
            ComputerAction::StepVerify => {
                let params: StepVerifyParams = parse_params(v, "step_verify")?;
                self.step_verify(params).await
            }
            ComputerAction::AutoRecover => {
                let params: AutoRecoverParams = parse_params(v, "auto_recover")?;
                self.auto_recover(params).await
            }
            ComputerAction::AnnotatedScreenshot => {
                let params: AnnotatedScreenshotParams = parse_params(v, "annotated_screenshot")?;
                self.annotated_screenshot(params).await
            }
            ComputerAction::AdaptiveRetry => {
                let params: AdaptiveRetryParams = parse_params(v, "adaptive_retry")?;
                self.adaptive_retry(params).await
            }
            ComputerAction::ClickAtCoords => {
                let params: ClickAtCoordsParams = parse_params(v, "click_at_coords")?;
                self.click_at_coords(params).await
            }
            ComputerAction::MultiPageSync => {
                let params: MultiPageSyncParams = parse_params(v, "multi_page_sync")?;
                self.multi_page_sync(params).await
            }
            ComputerAction::InputReplay => {
                let params: InputReplayParams = parse_params(v, "input_replay")?;
                self.input_replay(params).await
            }
            ComputerAction::ElementInfo => {
                let params: ElementInfoParams = parse_params(v, "element_info")?;
                self.element_info(params).await
            }
        }
    }

    #[tool(
        name = "memory",
        description = "Persistent agent memory — store, recall, and search across sessions.\n\nActions:\n- store {key, value, domain?, ttl_secs?} — Store a memory\n- recall {key, domain?} — Recall a memory by key\n- search {query, domain?, limit?} — Search memories\n- forget {key, domain?} — Delete a memory\n- domain_strategy {domain, strategy} — Set domain-specific strategy\n- stats — Get memory statistics"
    )]
    async fn tool_memory(
        &self,
        Parameters(p): Parameters<ToolAction>,
    ) -> Result<CallToolResult, McpError> {
        let action = p.action;
        let v = p.params;
        self.enforce_safety("memory", &action).await?;
        let action = MemoryAction::parse(&action)?;
        match action {
            MemoryAction::Store => {
                let params: MemoryStoreParams = parse_params(v, "store")?;
                self.memory_store(params).await
            }
            MemoryAction::Recall => {
                let params: MemoryRecallParams = parse_params(v, "recall")?;
                self.memory_recall(params).await
            }
            MemoryAction::Search => {
                let params: MemorySearchParams = parse_params(v, "search")?;
                self.memory_search(params).await
            }
            MemoryAction::Forget => {
                let params: MemoryForgetParams = parse_params(v, "forget")?;
                self.memory_forget(params).await
            }
            MemoryAction::DomainStrategy => {
                let params: MemoryDomainStrategyParams = parse_params(v, "domain_strategy")?;
                self.memory_domain_strategy(params).await
            }
            MemoryAction::Stats => {
                let params: MemoryStatsParams = parse_params(v, "stats")?;
                self.memory_stats(params).await
            }
        }
    }

    #[tool(
        name = "automate",
        description = "Workflow automation, AI task planning, and execution control.\n\nActions:\n- workflow_validate {workflow} — Validate a workflow definition\n- workflow_run {workflow} — Execute a workflow\n- plan {goal, context?} — Generate automation plan from goal\n- execute {plan, max_retries?} — Execute a generated plan\n- patterns — List available automation patterns\n- rate_limit {action?, max_per_minute?} — Check/configure rate limiter\n- retry {url?, operation?, reason?} — Enqueue retry with backoff\n- retry_adapt {action, params, max_retries?, strategy?} — Smart retry with adaptive strategy\n- error_classify {error_message} — Classify error into categories\n- recovery_suggest {error_type, context?} — Suggest recovery steps\n- error_history — List recent error history\n- checkpoint_save {name, include_cookies?, include_storage?, include_context?} — Save browser state checkpoint\n- checkpoint_restore {name, restore_url?, restore_cookies?} — Restore from checkpoint\n- checkpoint_list — List all checkpoints\n- checkpoint_delete {name} — Delete a checkpoint\n- workflow_while {condition, actions, max_iterations?} — Loop while condition is true\n- workflow_for_each {collection, variable_name?, actions} — Iterate over collection\n- workflow_if {condition, then_actions, else_actions?} — Conditional execution\n- workflow_variable {name, value?} — Get or set workflow variable\n- reconnect_cdp {max_retries?} — Auto-reconnect CDP with exponential backoff\n- gc_tabs {max_count?} — Garbage collect tabs / report tab info\n- batch_execute {commands, stop_on_error?} — Execute multiple JS commands in sequence\n- workflow_execute {workflow, variables?} — Execute a workflow using the standalone engine\n- workflow_status — Get workflow engine status and supported actions"
    )]
    async fn tool_automate(
        &self,
        Parameters(p): Parameters<ToolAction>,
    ) -> Result<CallToolResult, McpError> {
        let action = p.action;
        let v = p.params;
        self.enforce_safety("automate", &action).await?;
        let action = AutomateAction::parse(&action)?;
        match action {
            AutomateAction::WorkflowValidate => {
                let params: WorkflowValidateParams = parse_params(v, "workflow_validate")?;
                self.workflow_validate(params).await
            }
            AutomateAction::WorkflowRun => {
                let params: WorkflowRunParams = parse_params(v, "workflow_run")?;
                self.workflow_run(params).await
            }
            AutomateAction::Plan => {
                let params: PlannerPlanParams = parse_params(v, "plan")?;
                self.planner_plan(params).await
            }
            AutomateAction::Execute => {
                let params: PlannerExecuteParams = parse_params(v, "execute")?;
                self.planner_execute(params).await
            }
            AutomateAction::Patterns => {
                let params: PlannerPatternsParams = parse_params(v, "patterns")?;
                self.planner_patterns(params).await
            }
            AutomateAction::RateLimit => {
                let params: RateLimitCheckParams = parse_params(v, "rate_limit")?;
                self.automation_rate_limit(params).await
            }
            AutomateAction::Retry => {
                let params: RetryEnqueueParams = parse_params(v, "retry")?;
                self.automation_retry(params).await
            }
            // Error Recovery
            AutomateAction::RetryAdapt => {
                let params: RetryAdaptParams = parse_params(v, "retry_adapt")?;
                self.retry_adapt(params).await
            }
            AutomateAction::ErrorClassify => {
                let params: ErrorClassifyParams = parse_params(v, "error_classify")?;
                self.error_classify(params).await
            }
            AutomateAction::RecoverySuggest => {
                let params: RecoveryStrategyParams = parse_params(v, "recovery_suggest")?;
                self.recovery_suggest(params).await
            }
            AutomateAction::ErrorHistory => self.error_history(v).await,
            // Session checkpoints/resume
            AutomateAction::CheckpointSave => {
                let params: CheckpointSaveParams = parse_params(v, "checkpoint_save")?;
                self.checkpoint_save(params).await
            }
            AutomateAction::CheckpointRestore => {
                let params: CheckpointRestoreParams = parse_params(v, "checkpoint_restore")?;
                self.checkpoint_restore(params).await
            }
            AutomateAction::CheckpointList => self.checkpoint_list(v).await,
            AutomateAction::CheckpointDelete => {
                let params: CheckpointDeleteParams = parse_params(v, "checkpoint_delete")?;
                self.checkpoint_delete(params).await
            }
            // Extended workflow DSL
            AutomateAction::WorkflowWhile => {
                let params: WorkflowWhileParams = parse_params(v, "workflow_while")?;
                self.workflow_while(params).await
            }
            AutomateAction::WorkflowForEach => {
                let params: WorkflowForEachParams = parse_params(v, "workflow_for_each")?;
                self.workflow_for_each(params).await
            }
            AutomateAction::WorkflowIf => {
                let params: WorkflowIfParams = parse_params(v, "workflow_if")?;
                self.workflow_if(params).await
            }
            AutomateAction::WorkflowVariable => {
                let params: WorkflowVariableParams = parse_params(v, "workflow_variable")?;
                self.workflow_variable(params).await
            }
            // Long-running harness
            AutomateAction::ReconnectCdp => {
                let params: ReconnectCdpParams = parse_params(v, "reconnect_cdp")?;
                self.reconnect_cdp(params).await
            }
            AutomateAction::GcTabs => {
                let params: GcTabsParams = parse_params(v, "gc_tabs")?;
                self.gc_tabs(params).await
            }
            AutomateAction::Watchdog => {
                let params: WatchdogParams = parse_params(v, "watchdog")?;
                self.watchdog(params).await
            }
            AutomateAction::BatchExecute => {
                let params: BatchExecuteParams = parse_params(v, "batch_execute")?;
                self.batch_execute(params).await
            }
            AutomateAction::WorkflowExecute => {
                let params: WorkflowExecuteParams = parse_params(v, "workflow_execute")?;
                self.workflow_execute(params).await
            }
            AutomateAction::WorkflowStatus => {
                let params: WorkflowStatusParams = parse_params(v, "workflow_status")?;
                self.workflow_status(params).await
            }
        }
    }

    #[tool(
        name = "perf",
        description = "Performance monitoring, budgets, and visual regression testing.\n\nActions:\n- audit {url?} — Collect Core Web Vitals and performance metrics\n- budget {budget, url?} — Check performance against budget\n- compare {baseline, current, threshold_pct?} — Detect performance regressions\n- trace {url, settle_ms?} — Full performance trace with navigation\n- vrt_run {suite, baseline_dir} — Run visual regression test suite\n- vrt_compare {baseline, current, threshold?} — Compare two screenshots\n- vrt_update {suite_name, baseline_dir, tests} — Update VRT baselines\n- pixel_diff {image_a, image_b, threshold?} — In-browser pixel-level screenshot comparison"
    )]
    async fn tool_perf(
        &self,
        Parameters(p): Parameters<ToolAction>,
    ) -> Result<CallToolResult, McpError> {
        let action = p.action;
        let v = p.params;
        self.enforce_safety("perf", &action).await?;
        let action = PerfAction::parse(&action)?;
        match action {
            PerfAction::Audit => {
                let params: PerfAuditParams = parse_params(v, "audit")?;
                self.perf_audit(params).await
            }
            PerfAction::Budget => {
                let params: PerfBudgetCheckParams = parse_params(v, "budget")?;
                self.perf_budget(params).await
            }
            PerfAction::Compare => {
                let params: PerfCompareParams = parse_params(v, "compare")?;
                self.perf_compare(params).await
            }
            PerfAction::Trace => {
                let params: PerfTraceParams = parse_params(v, "trace")?;
                self.perf_trace(params).await
            }
            PerfAction::VrtRun => {
                let params: VrtRunParams = parse_params(v, "vrt_run")?;
                self.vrt_run(params).await
            }
            PerfAction::VrtCompare => {
                let params: VrtCompareParams = parse_params(v, "vrt_compare")?;
                self.vrt_compare(params).await
            }
            PerfAction::VrtUpdate => {
                let params: VrtUpdateBaselineParams = parse_params(v, "vrt_update")?;
                self.vrt_update_baseline(params).await
            }
            PerfAction::PixelDiff => {
                let params: PixelDiffParams = parse_params(v, "pixel_diff")?;
                self.pixel_diff(params).await
            }
        }
    }

    // ── Public CLI bridge ────────────────────────────────────────────

    /// Create an `OneCrawlMcp` reusing an existing browser session.
    /// Used by the CLI `run` command to delegate to MCP handlers directly.
    pub fn from_browser(
        browser: SharedBrowser,
        store_path: String,
        store_password: String,
    ) -> Self {
        Self {
            tool_router: Self::tool_router(),
            store_path: Arc::new(store_path),
            store_password: Arc::new(store_password),
            browser,
        }
    }

    /// Execute any MCP tool action and return the text output.
    /// Bridges the CLI to the full MCP handler dispatch with zero duplication.
    pub async fn run_tool(
        &self,
        tool: &str,
        action: &str,
        params: serde_json::Value,
    ) -> Result<String, String> {
        let ta = Parameters(ToolAction {
            action: action.to_string(),
            params,
        });

        let result = match tool {
            "browser" => self.tool_browser(ta).await,
            "crawl" => self.tool_crawl(ta).await,
            "agent" => self.tool_agent(ta).await,
            "stealth" => self.tool_stealth(ta).await,
            "data" => self.tool_data(ta).await,
            "secure" => self.tool_secure(ta).await,
            "computer" => self.tool_computer(ta).await,
            "memory" => self.tool_memory(ta).await,
            "automate" => self.tool_automate(ta).await,
            "perf" => self.tool_perf(ta).await,
            _ => return Err(format!(
                "unknown tool: '{tool}'. Available: browser, crawl, agent, stealth, data, secure, computer, memory, automate, perf"
            )),
        };

        match result {
            Ok(call_result) => {
                let texts: Vec<String> = call_result
                    .content
                    .unwrap_or_default()
                    .iter()
                    .filter_map(|c| c.as_text().map(|t| t.text.clone()))
                    .collect();
                Ok(texts.join("\n"))
            }
            Err(e) => Err(e.message.to_string()),
        }
    }
}

impl rmcp::ServerHandler for OneCrawlMcp {
    fn get_info(&self) -> ServerInfo {
        ServerInfo {
            instructions: Some(
                "OneCrawl MCP server — crypto, parser, storage, and CDP browser automation tools"
                    .into(),
            ),
            capabilities: ServerCapabilities::builder().enable_tools().build(),
            ..Default::default()
        }
    }
}
