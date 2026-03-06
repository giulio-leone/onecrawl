use rmcp::{
    ErrorData as McpError,
    handler::server::{router::tool::ToolRouter, tool::Parameters},
    model::*,
    tool, tool_router,
};
use std::sync::Arc;

use crate::cdp_tools::*;
use crate::helpers::{mcp_err, ensure_page, parse_params, McpResult};
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
        description = "Browser navigation, interaction, content extraction, multi-tab, DOM events, and session management.\n\nActions:\n- goto {url} — Navigate to URL\n- click {selector} — Click element\n- type {selector, text} — Type into input\n- screenshot {selector?, full_page?} — Screenshot\n- pdf {landscape?} — Export PDF\n- back — Navigate back\n- forward — Navigate forward\n- reload — Reload\n- wait {selector, timeout_ms?} — Wait for element\n- evaluate {js} — Execute JavaScript\n- snapshot {interactive_only?, compact?, depth?} — Accessibility snapshot\n- css {selector} — CSS query\n- xpath {expression} — XPath query\n- find_text {text, tag?} — Find by text\n- text {selector?} — Extract text\n- html {selector?} — Extract HTML\n- markdown {selector?} — Extract Markdown\n- structured — Extract JSON-LD/OG\n- stream {start_url, selector, next_selector?, max_pages?} — Paginated extraction\n- detect_forms — Detect forms\n- fill_form {form_selector?, fields, submit?} — Fill form\n- snapshot_diff {before, after} — Diff snapshots\n- parse_a11y {html} — Parse a11y tree offline\n- parse_selector {html, selector} — CSS query offline\n- parse_text {html} — Extract text offline\n- parse_links {html} — Extract links offline\n- new_tab {url?} — Open new tab\n- list_tabs — List all tabs\n- switch_tab {index} — Switch active tab\n- close_tab {index?} — Close tab\n- observe_mutations {selector?, child_list?, attributes?, subtree?} — Start mutation observer\n- get_mutations — Get recorded mutations\n- stop_mutations — Stop mutation observer\n- wait_for_event {event, selector?, timeout?} — Wait for DOM event\n- cookies_get {domain?, name?} — Get cookies\n- cookies_set {name, value, domain, path?, secure?, http_only?} — Set cookie\n- cookies_clear {domain?} — Clear cookies\n- storage_get {key, storage_type?} — Get localStorage/sessionStorage\n- storage_set {key, value, storage_type?} — Set storage\n- export_session {cookies?, local_storage?, session_storage?} — Export session state\n- import_session {state} — Import session state"
    )]
    async fn tool_browser(
        &self,
        Parameters(p): Parameters<ToolAction>,
    ) -> Result<CallToolResult, McpError> {
        let action = p.action;
        let v = p.params;
        match action.as_str() {
            "goto" => {
                let params: NavigateParams = parse_params(v, "goto")?;
                self.navigation_goto(params).await
            }
            "click" => {
                let params: ClickParams = parse_params(v, "click")?;
                self.navigation_click(params).await
            }
            "type" => {
                let params: TypeTextParams = parse_params(v, "type")?;
                self.navigation_type(params).await
            }
            "screenshot" => {
                let params: ScreenshotParams = parse_params(v, "screenshot")?;
                self.navigation_screenshot(params).await
            }
            "pdf" => {
                let params: PdfExportParams = parse_params(v, "pdf")?;
                self.navigation_pdf(params).await
            }
            "back" => self.navigation_back().await,
            "forward" => self.navigation_forward().await,
            "reload" => self.navigation_reload().await,
            "wait" => {
                let params: WaitForSelectorParams = parse_params(v, "wait")?;
                self.navigation_wait(params).await
            }
            "evaluate" => {
                let params: EvaluateJsParams = parse_params(v, "evaluate")?;
                self.navigation_evaluate(params).await
            }
            "snapshot" => {
                let params: AgentSnapshotParams = parse_params(v, "snapshot")?;
                self.navigation_snapshot(params).await
            }
            "css" => {
                let params: CssSelectorParams = parse_params(v, "css")?;
                self.scraping_css(params).await
            }
            "xpath" => {
                let params: XPathParams = parse_params(v, "xpath")?;
                self.scraping_xpath(params).await
            }
            "find_text" => {
                let params: FindByTextParams = parse_params(v, "find_text")?;
                self.scraping_find_text(params).await
            }
            "text" => {
                let params: ExtractTextParams = parse_params(v, "text")?;
                self.scraping_text(params).await
            }
            "html" => {
                let params: ExtractHtmlParams = parse_params(v, "html")?;
                self.scraping_html(params).await
            }
            "markdown" => {
                let params: ExtractMarkdownParams = parse_params(v, "markdown")?;
                self.scraping_markdown(params).await
            }
            "structured" => self.scraping_structured().await,
            "stream" => {
                let params: StreamExtractParams = parse_params(v, "stream")?;
                self.scraping_stream(params).await
            }
            "detect_forms" => {
                let params: DetectFormsParams = parse_params(v, "detect_forms")?;
                self.scraping_detect_forms(params).await
            }
            "fill_form" => {
                let params: FillFormParams = parse_params(v, "fill_form")?;
                self.scraping_fill_form(params).await
            }
            "snapshot_diff" => {
                let params: SnapshotDiffParams = parse_params(v, "snapshot_diff")?;
                self.scraping_snapshot_diff(params).await
            }
            "parse_a11y" => {
                let params: HtmlRequest = parse_params(v, "parse_a11y")?;
                self.parse_accessibility_tree(params)
            }
            "parse_selector" => {
                let params: SelectorRequest = parse_params(v, "parse_selector")?;
                self.query_selector(params)
            }
            "parse_text" => {
                let params: HtmlRequest = parse_params(v, "parse_text")?;
                self.html_extract_text(params)
            }
            "parse_links" => {
                let params: HtmlRequest = parse_params(v, "parse_links")?;
                self.html_extract_links(params)
            }
            // Multi-tab
            "new_tab" => {
                let params: NewTabParams = parse_params(v, "new_tab")?;
                self.tab_new(params).await
            }
            "list_tabs" => self.tab_list().await,
            "switch_tab" => {
                let params: SwitchTabParams = parse_params(v, "switch_tab")?;
                self.tab_switch(params).await
            }
            "close_tab" => {
                let params: CloseTabParams = parse_params(v, "close_tab")?;
                self.tab_close(params).await
            }
            // DOM events
            "observe_mutations" => {
                let params: ObserveMutationsParams = parse_params(v, "observe_mutations")?;
                self.observe_mutations(params).await
            }
            "get_mutations" => self.get_mutations().await,
            "stop_mutations" => self.stop_mutations().await,
            "wait_for_event" => {
                let params: WaitForEventParams = parse_params(v, "wait_for_event")?;
                self.wait_for_event(params).await
            }
            // Cookies & storage
            "cookies_get" => {
                let params: CookiesGetParams = parse_params(v, "cookies_get")?;
                self.cookies_get(params).await
            }
            "cookies_set" => {
                let params: CookieSetParams = parse_params(v, "cookies_set")?;
                self.cookies_set(params).await
            }
            "cookies_clear" => {
                let params: CookiesClearParams = parse_params(v, "cookies_clear")?;
                self.cookies_clear(params).await
            }
            "storage_get" => {
                let params: StorageGetParams = parse_params(v, "storage_get")?;
                self.storage_get(params).await
            }
            "storage_set" => {
                let params: StorageSetParams = parse_params(v, "storage_set")?;
                self.storage_set(params).await
            }
            "export_session" => {
                let params: SessionExportParams = parse_params(v, "export_session")?;
                self.session_export(params).await
            }
            "import_session" => {
                let params: SessionImportParams = parse_params(v, "import_session")?;
                self.session_import(params).await
            }
            other => Err(mcp_err(format!(
                "unknown browser action: {other}. Available: goto, click, type, screenshot, pdf, \
                 back, forward, reload, wait, evaluate, snapshot, css, xpath, find_text, text, \
                 html, markdown, structured, stream, detect_forms, fill_form, snapshot_diff, \
                 parse_a11y, parse_selector, parse_text, parse_links, new_tab, list_tabs, \
                 switch_tab, close_tab, observe_mutations, get_mutations, stop_mutations, \
                 wait_for_event, cookies_get, cookies_set, cookies_clear, storage_get, \
                 storage_set, export_session, import_session"
            )))
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
        match action.as_str() {
            "spider" => {
                let params: SpiderCrawlParams = parse_params(v, "spider")?;
                self.crawling_spider(params).await
            }
            "robots" => {
                let params: CheckRobotsParams = parse_params(v, "robots")?;
                self.crawling_robots(params).await
            }
            "sitemap" => {
                let params: GenerateSitemapParams = parse_params(v, "sitemap")?;
                self.crawling_sitemap(params)
            }
            "dom_snapshot" => {
                let params: TakeSnapshotParams = parse_params(v, "dom_snapshot")?;
                self.crawling_snapshot(params).await
            }
            "dom_compare" => {
                let params: CompareSnapshotsParams = parse_params(v, "dom_compare")?;
                self.crawling_compare(params).await
            }
            other => Err(mcp_err(format!(
                "unknown crawl action: {other}. Available: spider, robots, sitemap,                  dom_snapshot, dom_compare"
            )))
        }
    }

    #[tool(
        name = "agent",
        description = "AI agent orchestration — command chains, element screenshots, API capture, iframes, remote CDP, safety policies, skills, screencast, recording, and iOS automation.\n\nActions:\n- execute_chain {commands} — Execute multiple commands in sequence\n- element_screenshot {selector} — Screenshot a specific element\n- api_capture_start — Start capturing API calls\n- api_capture_summary — Get captured API call summary\n- iframe_list — List all iframes on page\n- iframe_snapshot {index, interactive_only?} — Snapshot an iframe\n- connect_remote {ws_url, headers?} — Connect to remote CDP\n- safety_set {policy} — Set safety policy JSON\n- safety_status — Get current safety policy status\n- skills_list — List available skills\n- screencast_start {quality?, max_width?, max_height?} — Start screencast\n- screencast_stop — Stop screencast\n- screencast_frame — Get latest screencast frame\n- recording_start {output?, fps?, quality?} — Start video recording\n- recording_stop — Stop recording and save\n- recording_status — Get recording status\n- ios_devices — List iOS devices\n- ios_connect {device_id, wda_url?} — Connect to iOS device\n- ios_navigate {url} — Navigate iOS Safari\n- ios_tap {x, y} — Tap on iOS screen\n- ios_screenshot — Take iOS screenshot"
    )]
    async fn tool_agent(
        &self,
        Parameters(p): Parameters<ToolAction>,
    ) -> Result<CallToolResult, McpError> {
        let action = p.action;
        let v = p.params;
        match action.as_str() {
            "execute_chain" => {
                let params: ExecuteChainParams = parse_params(v, "execute_chain")?;
                self.agent_execute_chain(params).await
            }
            "element_screenshot" => {
                let params: ElementScreenshotParams = parse_params(v, "element_screenshot")?;
                self.agent_element_screenshot(params).await
            }
            "api_capture_start" => {
                let params: ApiCaptureStartParams = parse_params(v, "api_capture_start")?;
                self.agent_api_capture_start(params).await
            }
            "api_capture_summary" => {
                let params: ApiCaptureSummaryParams = parse_params(v, "api_capture_summary")?;
                self.agent_api_capture_summary(params).await
            }
            "iframe_list" => {
                let params: IframeListParams = parse_params(v, "iframe_list")?;
                self.agent_iframe_list(params).await
            }
            "iframe_snapshot" => {
                let params: IframeSnapshotParams = parse_params(v, "iframe_snapshot")?;
                self.agent_iframe_snapshot(params).await
            }
            "connect_remote" => {
                let params: RemoteCdpParams = parse_params(v, "connect_remote")?;
                self.agent_connect_remote(params).await
            }
            "safety_set" => {
                let params: SafetyPolicySetParams = parse_params(v, "safety_set")?;
                self.agent_safety_policy_set(params).await
            }
            "safety_status" => {
                let params: SafetyStatusParams = parse_params(v, "safety_status")?;
                self.agent_safety_status(params).await
            }
            "skills_list" => {
                let params: SkillsListParams = parse_params(v, "skills_list")?;
                self.agent_skills_list(params)
            }
            "screencast_start" => {
                let params: ScreencastStartParams = parse_params(v, "screencast_start")?;
                self.agent_screencast_start(params).await
            }
            "screencast_stop" => {
                let params: ScreencastStopParams = parse_params(v, "screencast_stop")?;
                self.agent_screencast_stop(params).await
            }
            "screencast_frame" => {
                let params: ScreencastFrameParams = parse_params(v, "screencast_frame")?;
                self.agent_screencast_frame(params).await
            }
            "recording_start" => {
                let params: RecordingStartParams = parse_params(v, "recording_start")?;
                self.agent_recording_start(params).await
            }
            "recording_stop" => {
                let params: RecordingStopParams = parse_params(v, "recording_stop")?;
                self.agent_recording_stop(params).await
            }
            "recording_status" => {
                let params: RecordingStatusParams = parse_params(v, "recording_status")?;
                self.agent_recording_status(params).await
            }
            "ios_devices" => {
                let params: IosDevicesParams = parse_params(v, "ios_devices")?;
                self.agent_ios_devices(params).await
            }
            "ios_connect" => {
                let params: IosConnectParams = parse_params(v, "ios_connect")?;
                self.agent_ios_connect(params).await
            }
            "ios_navigate" => {
                let params: IosNavigateParams = parse_params(v, "ios_navigate")?;
                self.agent_ios_navigate(params).await
            }
            "ios_tap" => {
                let params: IosTapParams = parse_params(v, "ios_tap")?;
                self.agent_ios_tap(params).await
            }
            "ios_screenshot" => {
                let params: IosScreenshotParams = parse_params(v, "ios_screenshot")?;
                self.agent_ios_screenshot(params).await
            }
            other => Err(mcp_err(format!(
                "unknown agent action: {other}. Available: execute_chain, element_screenshot,                  api_capture_start, api_capture_summary, iframe_list, iframe_snapshot,                  connect_remote, safety_set, safety_status, skills_list, screencast_start,                  screencast_stop, screencast_frame, recording_start, recording_stop,                  recording_status, ios_devices, ios_connect, ios_navigate, ios_tap, ios_screenshot"
            )))
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
        match action.as_str() {
            "inject" => {
                let params: InjectStealthParams = parse_params(v, "inject")?;
                self.stealth_inject(params).await
            }
            "test" => {
                let params: BotDetectionTestParams = parse_params(v, "test")?;
                self.stealth_test(params).await
            }
            "fingerprint" => {
                let params: ApplyFingerprintParams = parse_params(v, "fingerprint")?;
                self.stealth_fingerprint(params).await
            }
            "block_domains" => {
                let params: BlockDomainsParams = parse_params(v, "block_domains")?;
                self.stealth_block_domains(params).await
            }
            "detect_captcha" => {
                let params: DetectCaptchaParams = parse_params(v, "detect_captcha")?;
                self.stealth_detect_captcha(params).await
            }
            other => Err(mcp_err(format!(
                "unknown stealth action: {other}. Available: inject, test, fingerprint,                  block_domains, detect_captcha"
            )))
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
        match action.as_str() {
            "pipeline" => {
                let params: PipelineExecuteParams = parse_params(v, "pipeline")?;
                self.data_pipeline(params)
            }
            "http_get" => {
                let params: HttpGetParams = parse_params(v, "http_get")?;
                self.data_http_get(params).await
            }
            "http_post" => {
                let params: HttpPostParams = parse_params(v, "http_post")?;
                self.data_http_post(params).await
            }
            "links" => {
                let params: ExtractLinksParams = parse_params(v, "links")?;
                self.data_links(params).await
            }
            "graph" => {
                let params: AnalyzeGraphParams = parse_params(v, "graph")?;
                self.data_graph(params)
            }
            "net_capture" => {
                let params: NetIntelCaptureParams = parse_params(v, "net_capture")?;
                self.net_capture(params).await
            }
            "net_analyze" => {
                let params: NetIntelAnalyzeParams = parse_params(v, "net_analyze")?;
                self.net_analyze(params).await
            }
            "net_sdk" => {
                let params: NetIntelSdkParams = parse_params(v, "net_sdk")?;
                self.net_sdk(params).await
            }
            "net_mock" => {
                let params: NetIntelMockParams = parse_params(v, "net_mock")?;
                self.net_mock(params).await
            }
            "net_replay" => {
                let params: NetIntelReplayParams = parse_params(v, "net_replay")?;
                self.net_replay(params).await
            }
            other => Err(mcp_err(format!(
                "unknown data action: {other}. Available: pipeline, http_get, http_post,                  links, graph, net_capture, net_analyze, net_sdk, net_mock, net_replay"
            )))
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
        match action.as_str() {
            "encrypt" => {
                let params: EncryptRequest = parse_params(v, "encrypt")?;
                self.encrypt(params)
            }
            "decrypt" => {
                let params: DecryptRequest = parse_params(v, "decrypt")?;
                self.decrypt(params)
            }
            "pkce" => self.generate_pkce(),
            "totp" => {
                let params: TotpRequest = parse_params(v, "totp")?;
                self.generate_totp(params)
            }
            "kv_set" => {
                let params: StoreSetRequest = parse_params(v, "kv_set")?;
                self.store_set(params)
            }
            "kv_get" => {
                let params: StoreGetRequest = parse_params(v, "kv_get")?;
                self.store_get(params)
            }
            "kv_list" => self.store_list(),
            "passkey_enable" => {
                let params: PasskeyEnableParams = parse_params(v, "passkey_enable")?;
                self.auth_passkey_enable(params).await
            }
            "passkey_add" => {
                let params: PasskeyAddParams = parse_params(v, "passkey_add")?;
                self.auth_passkey_add(params).await
            }
            "passkey_list" => {
                let params: PasskeyListParams = parse_params(v, "passkey_list")?;
                self.auth_passkey_list(params).await
            }
            "passkey_log" => {
                let params: PasskeyLogParams = parse_params(v, "passkey_log")?;
                self.auth_passkey_log(params).await
            }
            "passkey_disable" => {
                let params: PasskeyDisableParams = parse_params(v, "passkey_disable")?;
                self.auth_passkey_disable(params).await
            }
            "passkey_remove" => {
                let params: PasskeyRemoveParams = parse_params(v, "passkey_remove")?;
                self.auth_passkey_remove(params).await
            }
            other => Err(mcp_err(format!(
                "unknown secure action: {other}. Available: encrypt, decrypt, pkce, totp,                  kv_set, kv_get, kv_list, passkey_enable, passkey_add, passkey_list,                  passkey_log, passkey_disable, passkey_remove"
            )))
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
        match action.as_str() {
            "act" => {
                let params: ComputerUseActionParams = parse_params(v, "act")?;
                self.computer_act(params).await
            }
            "observe" => {
                let params: ComputerUseObserveParams = parse_params(v, "observe")?;
                self.computer_observe(params).await
            }
            "batch" => {
                let params: ComputerUseBatchParams = parse_params(v, "batch")?;
                self.computer_batch(params).await
            }
            "smart_find" => {
                let params: SmartFindParams = parse_params(v, "smart_find")?;
                self.smart_find(params).await
            }
            "smart_click" => {
                let params: SmartClickParams = parse_params(v, "smart_click")?;
                self.smart_click(params).await
            }
            "smart_fill" => {
                let params: SmartFillParams = parse_params(v, "smart_fill")?;
                self.smart_fill(params).await
            }
            "pool_list" => {
                let params: PoolListParams = parse_params(v, "pool_list")?;
                self.pool_list(params).await
            }
            "pool_status" => {
                let params: PoolStatusParams = parse_params(v, "pool_status")?;
                self.pool_status(params).await
            }
            other => Err(mcp_err(format!(
                "unknown computer action: {other}. Available: act, observe, batch,                  smart_find, smart_click, smart_fill, pool_list, pool_status"
            )))
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
        match action.as_str() {
            "store" => {
                let params: MemoryStoreParams = parse_params(v, "store")?;
                self.memory_store(params).await
            }
            "recall" => {
                let params: MemoryRecallParams = parse_params(v, "recall")?;
                self.memory_recall(params).await
            }
            "search" => {
                let params: MemorySearchParams = parse_params(v, "search")?;
                self.memory_search(params).await
            }
            "forget" => {
                let params: MemoryForgetParams = parse_params(v, "forget")?;
                self.memory_forget(params).await
            }
            "domain_strategy" => {
                let params: MemoryDomainStrategyParams = parse_params(v, "domain_strategy")?;
                self.memory_domain_strategy(params).await
            }
            "stats" => {
                let params: MemoryStatsParams = parse_params(v, "stats")?;
                self.memory_stats(params).await
            }
            other => Err(mcp_err(format!(
                "unknown memory action: {other}. Available: store, recall, search, forget,                  domain_strategy, stats"
            )))
        }
    }

    #[tool(
        name = "automate",
        description = "Workflow automation, AI task planning, and execution control.\n\nActions:\n- workflow_validate {workflow} — Validate a workflow definition\n- workflow_run {workflow} — Execute a workflow\n- plan {goal, context?} — Generate automation plan from goal\n- execute {plan, max_retries?} — Execute a generated plan\n- patterns — List available automation patterns\n- rate_limit {action?, max_per_minute?} — Check/configure rate limiter\n- retry {url?, operation?, reason?} — Enqueue retry with backoff"
    )]
    async fn tool_automate(
        &self,
        Parameters(p): Parameters<ToolAction>,
    ) -> Result<CallToolResult, McpError> {
        let action = p.action;
        let v = p.params;
        match action.as_str() {
            "workflow_validate" => {
                let params: WorkflowValidateParams = parse_params(v, "workflow_validate")?;
                self.workflow_validate(params).await
            }
            "workflow_run" => {
                let params: WorkflowRunParams = parse_params(v, "workflow_run")?;
                self.workflow_run(params).await
            }
            "plan" => {
                let params: PlannerPlanParams = parse_params(v, "plan")?;
                self.planner_plan(params).await
            }
            "execute" => {
                let params: PlannerExecuteParams = parse_params(v, "execute")?;
                self.planner_execute(params).await
            }
            "patterns" => {
                let params: PlannerPatternsParams = parse_params(v, "patterns")?;
                self.planner_patterns(params).await
            }
            "rate_limit" => {
                let params: RateLimitCheckParams = parse_params(v, "rate_limit")?;
                self.automation_rate_limit(params).await
            }
            "retry" => {
                let params: RetryEnqueueParams = parse_params(v, "retry")?;
                self.automation_retry(params).await
            }
            other => Err(mcp_err(format!(
                "unknown automate action: {other}. Available: workflow_validate, workflow_run,                  plan, execute, patterns, rate_limit, retry"
            )))
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
        match action.as_str() {
            "audit" => {
                let params: PerfAuditParams = parse_params(v, "audit")?;
                self.perf_audit(params).await
            }
            "budget" => {
                let params: PerfBudgetCheckParams = parse_params(v, "budget")?;
                self.perf_budget(params).await
            }
            "compare" => {
                let params: PerfCompareParams = parse_params(v, "compare")?;
                self.perf_compare(params).await
            }
            "trace" => {
                let params: PerfTraceParams = parse_params(v, "trace")?;
                self.perf_trace(params).await
            }
            "vrt_run" => {
                let params: VrtRunParams = parse_params(v, "vrt_run")?;
                self.vrt_run(params).await
            }
            "vrt_compare" => {
                let params: VrtCompareParams = parse_params(v, "vrt_compare")?;
                self.vrt_compare(params).await
            }
            "vrt_update" => {
                let params: VrtUpdateBaselineParams = parse_params(v, "vrt_update")?;
                self.vrt_update_baseline(params).await
            }
            other => Err(mcp_err(format!(
                "unknown perf action: {other}. Available: audit, budget, compare, trace,                  vrt_run, vrt_compare, vrt_update"
            )))
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
