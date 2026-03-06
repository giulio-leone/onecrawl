use rmcp::{
    ErrorData as McpError,
    handler::server::{router::tool::ToolRouter, tool::Parameters},
    model::*,
    tool, tool_router,
};
use std::sync::Arc;

use crate::actions::*;
use crate::cdp_tools::*;
use crate::helpers::{ensure_page, parse_params, McpResult};
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
        Self {
            tool_router: Self::tool_router(),
            store_path: Arc::new(store_path),
            store_password: Arc::new(store_password),
            browser: new_shared_browser(),
        }
    }

    pub(crate) fn open_store(&self) -> Result<onecrawl_storage::EncryptedStore, McpError> {
        onecrawl_storage::EncryptedStore::open(
            std::path::Path::new(self.store_path.as_ref()),
            &self.store_password,
        )
        .mcp()
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
        description = "Browser navigation, interaction, extraction, multi-tab, DOM events, session, network interception, console/dialog, device emulation, drag/drop, file upload, shadow DOM.\n\nActions:\n- goto {url} — Navigate to URL\n- click {selector} — Click element\n- type {selector, text} — Type into input\n- screenshot {selector?, full_page?} — Screenshot\n- pdf {landscape?} — Export PDF\n- back / forward / reload — Navigation\n- wait {selector, timeout_ms?} — Wait for element\n- evaluate {js} — Execute JavaScript\n- snapshot {interactive_only?, compact?, depth?} — A11y snapshot\n- css / xpath / find_text — Query elements\n- text / html / markdown / structured — Extract content\n- stream — Paginated extraction\n- detect_forms / fill_form — Forms\n- snapshot_diff — Diff snapshots\n- parse_a11y / parse_selector / parse_text / parse_links — Offline\n- new_tab / list_tabs / switch_tab / close_tab — Multi-tab\n- observe_mutations / get_mutations / stop_mutations / wait_for_event — DOM\n- cookies_get / cookies_set / cookies_clear — Cookies\n- storage_get / storage_set / export_session / import_session — Storage\n- intercept_enable / intercept_add_rule / intercept_remove_rule / intercept_list / intercept_disable / block_requests — Network\n- console_start / console_get / console_clear / dialog_handle / dialog_get / errors_get — Debug\n- emulate_device / emulate_geolocation / emulate_timezone / emulate_media / emulate_network — Emulation\n- drag {source, target} — Drag and drop\n- hover {selector} — Mouse hover\n- keyboard {keys, selector?} — Keyboard shortcuts\n- select {selector, value?, text?, index?} — Select dropdown option\n- upload {selector, file_path} — File upload\n- download_wait / download_list / download_set_dir — Downloads\n- shadow_query / shadow_text {host_selector, inner_selector} — Shadow DOM\n- deep_query {selector} — Pierce shadow DOM with >>>\n- context_set {key, value} / context_get {key} / context_list / context_clear / context_transfer {from_tab, to_tab, keys?} — Page context\n- form_infer {selector?} / form_auto_fill {data, selector?, confidence_threshold?} / form_validate — Smart form mapping\n- selector_heal {selector, context?} / selector_alternatives {selector, max_alternatives?} / selector_validate {selector, expected_role?, expected_text?} — Self-healing selectors\n- event_subscribe {event_type, filter?} / event_unsubscribe {event_type} / event_poll {event_type?, limit?, clear?} / event_clear — Event reactions"
    )]
    async fn tool_browser(
        &self,
        Parameters(p): Parameters<ToolAction>,
    ) -> Result<CallToolResult, McpError> {
        let action = p.action;
        let v = p.params;
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
        description = "AI agent orchestration — command chains, element screenshots, API capture, iframes, remote CDP, safety policies, skills, screencast, recording, and iOS automation.\n\nActions:\n- execute_chain {commands} — Execute multiple commands in sequence\n- element_screenshot {selector} — Screenshot a specific element\n- api_capture_start — Start capturing API calls\n- api_capture_summary — Get captured API call summary\n- iframe_list — List all iframes on page\n- iframe_snapshot {index, interactive_only?} — Snapshot an iframe\n- connect_remote {ws_url, headers?} — Connect to remote CDP\n- safety_set {policy} — Set safety policy JSON\n- safety_status — Get current safety policy status\n- skills_list — List available skills\n- screencast_start {quality?, max_width?, max_height?} — Start screencast\n- screencast_stop — Stop screencast\n- screencast_frame — Get latest screencast frame\n- recording_start {output?, fps?, quality?} — Start video recording\n- recording_stop — Stop recording and save\n- recording_status — Get recording status\n- ios_devices — List iOS devices\n- ios_connect {device_id, wda_url?} — Connect to iOS device\n- ios_navigate {url} — Navigate iOS Safari\n- ios_tap {x, y} — Tap on iOS screen\n- ios_screenshot — Take iOS screenshot\n- task_decompose {goal, context?, max_depth?} — Decompose goal into subtasks\n- task_plan {tasks, strategy?} — Generate execution plan\n- task_status — Get current task plans status\n- vision_describe {selector?, format?} — Describe page/element visually\n- vision_locate {description, strategy?} — Find element by description\n- vision_compare {baseline, current?, threshold?} — Compare page states"
    )]
    async fn tool_agent(
        &self,
        Parameters(p): Parameters<ToolAction>,
    ) -> Result<CallToolResult, McpError> {
        let action = p.action;
        let v = p.params;
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
        }
    }

    #[tool(
        name = "stealth",
        description = "Anti-detection and bot evasion — stealth patches, fingerprinting, CAPTCHA detection.\n\nActions:\n- inject — Inject stealth patches into page\n- test — Test if current page detects bot\n- fingerprint {user_agent?} — Generate and apply browser fingerprint\n- block_domains {domains} — Block tracking domains\n- detect_captcha — Detect CAPTCHAs on page"
    )]
    async fn tool_stealth(
        &self,
        Parameters(p): Parameters<ToolAction>,
    ) -> Result<CallToolResult, McpError> {
        let action = p.action;
        let v = p.params;
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
        }
    }

    #[tool(
        name = "data",
        description = "Data processing, HTTP requests, link analysis, and network intelligence.\n\nActions:\n- pipeline {input, steps} — Multi-step data pipeline\n- http_get {url, headers?} — HTTP GET request\n- http_post {url, body?, content_type?, headers?} — HTTP POST request\n- links {base_url?} — Extract link graph from page\n- graph {edges} — Analyze link graph\n- net_capture {duration_ms?} — Capture network traffic\n- net_analyze {traffic?} — Analyze captured API traffic\n- net_sdk {traffic, language?} — Generate API SDK code\n- net_mock {traffic?} — Generate mock server config\n- net_replay {sequence} — Replay captured requests"
    )]
    async fn tool_data(
        &self,
        Parameters(p): Parameters<ToolAction>,
    ) -> Result<CallToolResult, McpError> {
        let action = p.action;
        let v = p.params;
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
        }
    }

    #[tool(
        name = "secure",
        description = "Cryptography, encrypted storage, and WebAuthn passkey management.\n\nActions:\n- encrypt {plaintext, password} — AES-256-GCM encryption\n- decrypt {ciphertext, password} — AES-256-GCM decryption\n- pkce — Generate PKCE S256 challenge pair\n- totp {secret} — Generate 6-digit TOTP code\n- kv_set {key, value} — Store encrypted key-value pair\n- kv_get {key} — Retrieve value by key\n- kv_list — List all stored keys\n- passkey_enable — Enable virtual WebAuthn authenticator\n- passkey_add {rp_id, user_name, credential_id?} — Add passkey credential\n- passkey_list — List stored passkeys\n- passkey_log — Get WebAuthn operation log\n- passkey_disable — Disable authenticator\n- passkey_remove {credential_id} — Remove passkey by ID"
    )]
    async fn tool_secure(
        &self,
        Parameters(p): Parameters<ToolAction>,
    ) -> Result<CallToolResult, McpError> {
        let action = p.action;
        let v = p.params;
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
        }
    }

    #[tool(
        name = "computer",
        description = "AI computer use protocol, smart element resolution, and browser pool management.\n\nActions:\n- act {action_type, coordinate?, text?, key?} — Perform computer action\n- observe {observation_type?} — Observe screen state\n- batch {actions} — Execute multiple actions in sequence\n- smart_find {description, strategy?} — Find element by description\n- smart_click {description} — Click element by description\n- smart_fill {description, value} — Fill input by description\n- pool_list — List browser pool instances\n- pool_status — Get pool status and stats"
    )]
    async fn tool_computer(
        &self,
        Parameters(p): Parameters<ToolAction>,
    ) -> Result<CallToolResult, McpError> {
        let action = p.action;
        let v = p.params;
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
        description = "Workflow automation, AI task planning, and execution control.\n\nActions:\n- workflow_validate {workflow} — Validate a workflow definition\n- workflow_run {workflow} — Execute a workflow\n- plan {goal, context?} — Generate automation plan from goal\n- execute {plan, max_retries?} — Execute a generated plan\n- patterns — List available automation patterns\n- rate_limit {action?, max_per_minute?} — Check/configure rate limiter\n- retry {url?, operation?, reason?} — Enqueue retry with backoff\n- retry_adapt {action, params, max_retries?, strategy?} — Smart retry with adaptive strategy\n- error_classify {error_message} — Classify error into categories\n- recovery_suggest {error_type, context?} — Suggest recovery steps\n- error_history — List recent error history\n- checkpoint_save {name, include_cookies?, include_storage?, include_context?} — Save browser state checkpoint\n- checkpoint_restore {name, restore_url?, restore_cookies?} — Restore from checkpoint\n- checkpoint_list — List all checkpoints\n- checkpoint_delete {name} — Delete a checkpoint\n- workflow_while {condition, actions, max_iterations?} — Loop while condition is true\n- workflow_for_each {collection, variable_name?, actions} — Iterate over collection\n- workflow_if {condition, then_actions, else_actions?} — Conditional execution\n- workflow_variable {name, value?} — Get or set workflow variable"
    )]
    async fn tool_automate(
        &self,
        Parameters(p): Parameters<ToolAction>,
    ) -> Result<CallToolResult, McpError> {
        let action = p.action;
        let v = p.params;
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
        }
    }

    #[tool(
        name = "perf",
        description = "Performance monitoring, budgets, and visual regression testing.\n\nActions:\n- audit {url?} — Collect Core Web Vitals and performance metrics\n- budget {budget, url?} — Check performance against budget\n- compare {baseline, current, threshold_pct?} — Detect performance regressions\n- trace {url, settle_ms?} — Full performance trace with navigation\n- vrt_run {suite, baseline_dir} — Run visual regression test suite\n- vrt_compare {baseline, current, threshold?} — Compare two screenshots\n- vrt_update {suite_name, baseline_dir, tests} — Update VRT baselines"
    )]
    async fn tool_perf(
        &self,
        Parameters(p): Parameters<ToolAction>,
    ) -> Result<CallToolResult, McpError> {
        let action = p.action;
        let v = p.params;
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
