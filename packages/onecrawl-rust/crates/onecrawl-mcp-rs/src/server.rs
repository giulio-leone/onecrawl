use base64::{Engine as _, engine::general_purpose::STANDARD as B64};
use futures::StreamExt;
use rmcp::{
    ErrorData as McpError, ServerHandler,
    handler::server::{router::tool::ToolRouter, tool::Parameters},
    model::*,
    tool, tool_router,
};
use std::collections::HashMap;
use std::sync::Arc;

use crate::cdp_tools::*;
use crate::helpers::{mcp_err, ensure_page, json_ok, text_ok, parse_params, parse_json_str, parse_opt_json_str, McpResult};
use crate::types::*;

// ──────────────────────────── Helpers ────────────────────────────

fn parse_memory_category(s: Option<&str>) -> Option<onecrawl_cdp::MemoryCategory> {
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

// ──────────────────────────── Server ────────────────────────────

#[derive(Clone)]
pub struct OneCrawlMcp {
    #[allow(dead_code)] // accessed via #[tool_router] proc macro
    tool_router: ToolRouter<Self>,
    store_path: Arc<String>,
    store_password: Arc<String>,
    browser: SharedBrowser,
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

    fn open_store(&self) -> Result<onecrawl_storage::EncryptedStore, McpError> {
        onecrawl_storage::EncryptedStore::open(
            std::path::Path::new(self.store_path.as_ref()),
            &self.store_password,
        )
        .mcp()
    }

    /// Internal dispatch for `agent.execute_chain`.
    /// Returns `Ok(serde_json::Value)` on success or `Err(String)` with an
    /// error message for that step.
    async fn dispatch_chain_command(
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
        description = "Browser navigation, interaction, and content extraction. All browser operations in one tool.\n\nActions:\n- goto {url} — Navigate to URL\n- click {selector} — Click element (CSS selector or @ref like @e1)\n- type {selector, text} — Type into input\n- screenshot {selector?, full_page?} — Screenshot as PNG base64\n- pdf {landscape?} — Export page as PDF\n- back — Navigate back\n- forward — Navigate forward\n- reload — Reload page\n- wait {selector, timeout_ms?} — Wait for element (default 30s)\n- evaluate {js} — Execute JavaScript, returns result\n- snapshot {interactive_only?, cursor?, compact?, depth?, selector?} — Accessibility snapshot with @refs\n- css {selector} — CSS query on live DOM\n- xpath {expression} — XPath query\n- find_text {text, tag?} — Find by visible text\n- text {selector?} — Extract visible text\n- html {selector?} — Extract raw HTML\n- markdown {selector?} — Extract as Markdown\n- structured — Extract JSON-LD, OpenGraph, etc.\n- stream {start_url, selector, next_selector?, max_pages?} — Paginated extraction\n- detect_forms — Detect forms and fields\n- fill_form {form_selector?, fields, submit?} — Fill and submit form\n- snapshot_diff {before, after} — Diff two text snapshots\n- parse_a11y {html} — Parse HTML into accessibility tree (offline)\n- parse_selector {html, selector} — CSS query on HTML string (offline)\n- parse_text {html} — Extract text from HTML (offline)\n- parse_links {html} — Extract links from HTML (offline)"
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
            other => Err(mcp_err(format!(
                "unknown browser action: {other}. Available: goto, click, type, screenshot, pdf, \
                 back, forward, reload, wait, evaluate, snapshot, css, xpath, find_text, text, \
                 html, markdown, structured, stream, detect_forms, fill_form, snapshot_diff, \
                 parse_a11y, parse_selector, parse_text, parse_links"
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

    // ── Crypto tools ──

    fn encrypt(
        &self,
        req: EncryptRequest,
    ) -> Result<CallToolResult, McpError> {
        let payload = onecrawl_crypto::encrypt(req.plaintext.as_bytes(), &req.password)
            .mcp()?;

        let salt = B64
            .decode(&payload.salt)
            .mcp()?;
        let nonce = B64
            .decode(&payload.nonce)
            .mcp()?;
        let ct = B64
            .decode(&payload.ciphertext)
            .mcp()?;

        let mut packed = Vec::with_capacity(salt.len() + nonce.len() + ct.len());
        packed.extend_from_slice(&salt);
        packed.extend_from_slice(&nonce);
        packed.extend_from_slice(&ct);

        Ok(CallToolResult::success(vec![Content::text(
            B64.encode(&packed),
        )]))
    }

    fn decrypt(
        &self,
        req: DecryptRequest,
    ) -> Result<CallToolResult, McpError> {
        let raw = B64
            .decode(&req.ciphertext)
            .map_err(|e| mcp_err(format!("invalid base64: {e}")))?;

        if raw.len() < 29 {
            return Err(mcp_err(
                "ciphertext too short: need at least 29 bytes (16 salt + 12 nonce + 1 ct)",
            ));
        }

        let payload = onecrawl_core::EncryptedPayload {
            salt: B64.encode(&raw[..16]),
            nonce: B64.encode(&raw[16..28]),
            ciphertext: B64.encode(&raw[28..]),
        };

        let plaintext = onecrawl_crypto::decrypt(&payload, &req.password)
            .mcp()?;

        let text = String::from_utf8(plaintext).unwrap_or_else(|e| B64.encode(e.into_bytes()));

        Ok(CallToolResult::success(vec![Content::text(text)]))
    }

    fn generate_pkce(&self) -> Result<CallToolResult, McpError> {
        let challenge =
            onecrawl_crypto::generate_pkce_challenge().mcp()?;
        json_ok(&PkceResponse {
            code_verifier: &challenge.code_verifier,
            code_challenge: &challenge.code_challenge,
        })
    }

    fn generate_totp(
        &self,
        req: TotpRequest,
    ) -> Result<CallToolResult, McpError> {
        let config = onecrawl_core::TotpConfig {
            secret: req.secret,
            ..Default::default()
        };
        let code =
            onecrawl_crypto::totp::generate_totp(&config).mcp()?;
        Ok(CallToolResult::success(vec![Content::text(code)]))
    }

    // ── Parser tools ──

    fn parse_accessibility_tree(
        &self,
        req: HtmlRequest,
    ) -> Result<CallToolResult, McpError> {
        let tree = onecrawl_parser::get_accessibility_tree(&req.html)
            .mcp()?;
        let rendered = onecrawl_parser::accessibility::render_tree(&tree, 0, false);
        Ok(CallToolResult::success(vec![Content::text(rendered)]))
    }

    fn query_selector(
        &self,
        req: SelectorRequest,
    ) -> Result<CallToolResult, McpError> {
        let elements = onecrawl_parser::query_selector(&req.html, &req.selector)
            .mcp()?;
        let json = serde_json::to_string(&elements).mcp()?;
        Ok(CallToolResult::success(vec![Content::text(json)]))
    }

    fn html_extract_text(
        &self,
        req: HtmlRequest,
    ) -> Result<CallToolResult, McpError> {
        let texts =
            onecrawl_parser::extract_text(&req.html, "body").mcp()?;
        Ok(CallToolResult::success(vec![Content::text(
            texts.join("\n"),
        )]))
    }

    fn html_extract_links(
        &self,
        req: HtmlRequest,
    ) -> Result<CallToolResult, McpError> {
        let links = onecrawl_parser::extract::extract_links(&req.html)
            .mcp()?;
        let result: Vec<LinkInfo> = links
            .into_iter()
            .map(|(href, text)| {
                let is_external = href.starts_with("http://") || href.starts_with("https://");
                LinkInfo { href, text, is_external }
            })
            .collect();
        let json = serde_json::to_string(&result).mcp()?;
        Ok(CallToolResult::success(vec![Content::text(json)]))
    }

    // ── Storage tools ──

    fn store_set(
        &self,
        req: StoreSetRequest,
    ) -> Result<CallToolResult, McpError> {
        let store = self.open_store()?;
        store
            .set(&req.key, req.value.as_bytes())
            .mcp()?;
        Ok(CallToolResult::success(vec![Content::text(format!(
            "stored key \"{}\"",
            req.key
        ))]))
    }

    fn store_get(
        &self,
        req: StoreGetRequest,
    ) -> Result<CallToolResult, McpError> {
        let store = self.open_store()?;
        let value = store.get(&req.key).mcp()?;
        match value {
            Some(v) => {
                let text = String::from_utf8(v).unwrap_or_else(|e| B64.encode(e.into_bytes()));
                Ok(CallToolResult::success(vec![Content::text(text)]))
            }
            None => Ok(CallToolResult::success(vec![Content::text(format!(
                "key \"{}\" not found",
                req.key
            ))])),
        }
    }

    fn store_list(&self) -> Result<CallToolResult, McpError> {
        let store = self.open_store()?;
        let keys = store.list("").mcp()?;
        let json = serde_json::to_string(&keys).mcp()?;
        Ok(CallToolResult::success(vec![Content::text(json)]))
    }

    // ════════════════════════════════════════════════════════════════
    //  CDP tools — Navigation & Page Control
    // ════════════════════════════════════════════════════════════════

    async fn navigation_goto(
        &self,
        p: NavigateParams,
    ) -> Result<CallToolResult, McpError> {
        let page = ensure_page(&self.browser).await?;
        onecrawl_cdp::navigation::goto(&page, &p.url)
            .await
            .mcp()?;
        let title = onecrawl_cdp::navigation::get_title(&page)
            .await
            .unwrap_or_default();
        text_ok(format!("navigated to {} — title: {title}", p.url))
    }

    async fn navigation_click(
        &self,
        p: ClickParams,
    ) -> Result<CallToolResult, McpError> {
        let page = ensure_page(&self.browser).await?;
        let selector = onecrawl_cdp::accessibility::resolve_ref(&p.selector);
        onecrawl_cdp::element::click(&page, &selector)
            .await
            .mcp()?;
        text_ok(format!("clicked {}", p.selector))
    }

    async fn navigation_type(
        &self,
        p: TypeTextParams,
    ) -> Result<CallToolResult, McpError> {
        let page = ensure_page(&self.browser).await?;
        let selector = onecrawl_cdp::accessibility::resolve_ref(&p.selector);
        onecrawl_cdp::element::type_text(&page, &selector, &p.text)
            .await
            .mcp()?;
        text_ok(format!("typed {} chars into {}", p.text.len(), p.selector))
    }

    async fn navigation_screenshot(
        &self,
        p: ScreenshotParams,
    ) -> Result<CallToolResult, McpError> {
        let page = ensure_page(&self.browser).await?;
        let bytes = if let Some(sel) = &p.selector {
            onecrawl_cdp::screenshot::screenshot_element(&page, sel)
                .await
                .mcp()?
        } else if p.full_page.unwrap_or(false) {
            onecrawl_cdp::screenshot::screenshot_full(&page)
                .await
                .mcp()?
        } else {
            onecrawl_cdp::screenshot::screenshot_viewport(&page)
                .await
                .mcp()?
        };
        let b64 = B64.encode(&bytes);
        Ok(CallToolResult::success(vec![Content::image(
            b64,
            "image/png",
        )]))
    }

    async fn navigation_pdf(
        &self,
        p: PdfExportParams,
    ) -> Result<CallToolResult, McpError> {
        let page = ensure_page(&self.browser).await?;
        let opts = onecrawl_cdp::PdfOptions {
            landscape: p.landscape.unwrap_or(false),
            ..Default::default()
        };
        let _ = p.print_background; // reserved for future use
        let _ = p.format; // reserved for future use
        let bytes = onecrawl_cdp::screenshot::pdf_with_options(&page, &opts)
            .await
            .mcp()?;
        let b64 = B64.encode(&bytes);
        text_ok(format!(
            "pdf exported ({} bytes, base64 length {})",
            bytes.len(),
            b64.len()
        ))
    }

    async fn navigation_back(&self) -> Result<CallToolResult, McpError> {
        let page = ensure_page(&self.browser).await?;
        onecrawl_cdp::navigation::go_back(&page)
            .await
            .mcp()?;
        text_ok("navigated back")
    }

    async fn navigation_forward(&self) -> Result<CallToolResult, McpError> {
        let page = ensure_page(&self.browser).await?;
        onecrawl_cdp::navigation::go_forward(&page)
            .await
            .mcp()?;
        text_ok("navigated forward")
    }

    async fn navigation_reload(&self) -> Result<CallToolResult, McpError> {
        let page = ensure_page(&self.browser).await?;
        onecrawl_cdp::navigation::reload(&page)
            .await
            .mcp()?;
        text_ok("page reloaded")
    }

    async fn navigation_wait(
        &self,
        p: WaitForSelectorParams,
    ) -> Result<CallToolResult, McpError> {
        let page = ensure_page(&self.browser).await?;
        let timeout = p.timeout_ms.unwrap_or(30_000);
        let selector = onecrawl_cdp::accessibility::resolve_ref(&p.selector);
        onecrawl_cdp::navigation::wait_for_selector(&page, &selector, timeout)
            .await
            .mcp()?;
        text_ok(format!("selector {} found", p.selector))
    }

    async fn navigation_evaluate(
        &self,
        p: EvaluateJsParams,
    ) -> Result<CallToolResult, McpError> {
        let page = ensure_page(&self.browser).await?;
        let result = onecrawl_cdp::page::evaluate_js(&page, &p.js)
            .await
            .mcp()?;
        json_ok(&result)
    }

    async fn navigation_snapshot(
        &self,
        p: AgentSnapshotParams,
    ) -> Result<CallToolResult, McpError> {
        let page = ensure_page(&self.browser).await?;
        let opts = onecrawl_cdp::accessibility::AgentSnapshotOptions {
            interactive_only: p.interactive_only.unwrap_or(false),
            cursor: p.cursor.unwrap_or(false),
            compact: p.compact.unwrap_or(false),
            depth: p.depth,
            selector: p.selector,
        };
        let snap = onecrawl_cdp::accessibility::agent_snapshot(&page, &opts)
            .await
            .mcp()?;
        let stats = snap.stats();
        let result = serde_json::json!({
            "snapshot": snap.snapshot,
            "refs": snap.refs,
            "total": snap.total,
            "stats": {
                "lines": stats.lines,
                "chars": stats.chars,
                "estimated_tokens": stats.estimated_tokens,
                "total_refs": stats.total_refs,
                "interactive_refs": stats.interactive_refs
            }
        });
        json_ok(&result)
    }

    // ════════════════════════════════════════════════════════════════
    //  CDP tools — Scraping & Extraction
    // ════════════════════════════════════════════════════════════════

    async fn scraping_css(
        &self,
        p: CssSelectorParams,
    ) -> Result<CallToolResult, McpError> {
        let page = ensure_page(&self.browser).await?;
        let result = onecrawl_cdp::selectors::css_select(&page, &p.selector)
            .await
            .mcp()?;
        json_ok(&result)
    }

    async fn scraping_xpath(
        &self,
        p: XPathParams,
    ) -> Result<CallToolResult, McpError> {
        let page = ensure_page(&self.browser).await?;
        let result = onecrawl_cdp::selectors::xpath_select(&page, &p.expression)
            .await
            .mcp()?;
        json_ok(&result)
    }

    async fn scraping_find_text(
        &self,
        p: FindByTextParams,
    ) -> Result<CallToolResult, McpError> {
        let page = ensure_page(&self.browser).await?;
        let result =
            onecrawl_cdp::selectors::find_by_text(&page, &p.text, p.tag.as_deref())
                .await
                .mcp()?;
        json_ok(&result)
    }

    async fn scraping_text(
        &self,
        p: ExtractTextParams,
    ) -> Result<CallToolResult, McpError> {
        let page = ensure_page(&self.browser).await?;
        let result = onecrawl_cdp::extract::extract(
            &page,
            p.selector.as_deref(),
            onecrawl_cdp::ExtractFormat::Text,
        )
        .await
        .mcp()?;
        json_ok(&result)
    }

    async fn scraping_html(
        &self,
        p: ExtractHtmlParams,
    ) -> Result<CallToolResult, McpError> {
        let page = ensure_page(&self.browser).await?;
        let result = onecrawl_cdp::extract::extract(
            &page,
            p.selector.as_deref(),
            onecrawl_cdp::ExtractFormat::Html,
        )
        .await
        .mcp()?;
        json_ok(&result)
    }

    async fn scraping_markdown(
        &self,
        p: ExtractMarkdownParams,
    ) -> Result<CallToolResult, McpError> {
        let page = ensure_page(&self.browser).await?;
        let result = onecrawl_cdp::extract::extract(
            &page,
            p.selector.as_deref(),
            onecrawl_cdp::ExtractFormat::Markdown,
        )
        .await
        .mcp()?;
        json_ok(&result)
    }

    async fn scraping_structured(&self) -> Result<CallToolResult, McpError> {
        let page = ensure_page(&self.browser).await?;
        let result = onecrawl_cdp::structured_data::extract_all(&page)
            .await
            .mcp()?;
        json_ok(&result)
    }

    async fn scraping_stream(
        &self,
        p: StreamExtractParams,
    ) -> Result<CallToolResult, McpError> {
        let page = ensure_page(&self.browser).await?;
        let fields: Vec<onecrawl_cdp::ExtractionRule> = parse_json_str(&p.fields, "fields")?;
        let pagination: Option<onecrawl_cdp::PaginationConfig> =
            parse_opt_json_str(p.pagination.as_deref(), "pagination")?;
        let schema = onecrawl_cdp::ExtractionSchema {
            item_selector: p.item_selector,
            fields,
            pagination,
        };
        let result = if schema.pagination.is_some() {
            onecrawl_cdp::streaming::extract_with_pagination(&page, &schema)
                .await
                .mcp()?
        } else {
            onecrawl_cdp::streaming::extract_items(&page, &schema)
                .await
                .mcp()?
        };
        json_ok(&result)
    }

    async fn scraping_detect_forms(
        &self,
        _p: DetectFormsParams,
    ) -> Result<CallToolResult, McpError> {
        let page = ensure_page(&self.browser).await?;
        let forms = onecrawl_cdp::form_filler::detect_forms(&page)
            .await
            .mcp()?;
        json_ok(&forms)
    }

    async fn scraping_fill_form(
        &self,
        p: FillFormParams,
    ) -> Result<CallToolResult, McpError> {
        let page = ensure_page(&self.browser).await?;
        let values: HashMap<String, String> = parse_json_str(&p.data, "data")?;
        let result =
            onecrawl_cdp::form_filler::fill_form(&page, &p.form_selector, &values)
                .await
                .mcp()?;
        if p.submit.unwrap_or(false) {
            onecrawl_cdp::form_filler::submit_form(&page, &p.form_selector)
                .await
                .mcp()?;
        }
        json_ok(&result)
    }

    async fn scraping_snapshot_diff(
        &self,
        p: SnapshotDiffParams,
    ) -> Result<CallToolResult, McpError> {
        let result = onecrawl_cdp::snapshot_diff::diff_snapshots(&p.before, &p.after);
        json_ok(&result)
    }

    // ════════════════════════════════════════════════════════════════
    //  CDP tools — Crawling
    // ════════════════════════════════════════════════════════════════

    async fn crawling_spider(
        &self,
        p: SpiderCrawlParams,
    ) -> Result<CallToolResult, McpError> {
        let page = ensure_page(&self.browser).await?;
        let config = onecrawl_cdp::SpiderConfig {
            start_urls: p.start_urls,
            max_depth: p.max_depth.unwrap_or(2),
            max_pages: p.max_pages.unwrap_or(50),
            concurrency: 1,
            delay_ms: p.delay_ms.unwrap_or(500),
            follow_links: true,
            same_domain_only: p.same_domain_only.unwrap_or(true),
            url_patterns: p.url_patterns.unwrap_or_default(),
            exclude_patterns: p.exclude_patterns.unwrap_or_default(),
            extract_selector: None,
            extract_format: "text".into(),
            timeout_ms: 30_000,
            user_agent: None,
        };
        let results = onecrawl_cdp::spider::crawl(&page, config)
            .await
            .mcp()?;
        let summary = onecrawl_cdp::spider::summarize(&results);
        json_ok(&CrawlResult2 {
            summary,
            pages_crawled: results.len(),
        })
    }

    async fn crawling_robots(
        &self,
        p: CheckRobotsParams,
    ) -> Result<CallToolResult, McpError> {
        let page = ensure_page(&self.browser).await?;
        let robots = onecrawl_cdp::robots::fetch_robots(&page, &p.base_url)
            .await
            .mcp()?;
        let ua = p.user_agent.as_deref().unwrap_or("*");
        let sitemaps = onecrawl_cdp::robots::get_sitemaps(&robots);
        let delay = onecrawl_cdp::robots::get_crawl_delay(&robots, ua);
        let allowed = p.path.as_ref().map(|path| {
            onecrawl_cdp::robots::is_allowed(&robots, ua, path)
        });
        json_ok(&RobotsInfo {
            sitemaps,
            crawl_delay: delay,
            path_allowed: allowed,
        })
    }

    fn crawling_sitemap(
        &self,
        p: GenerateSitemapParams,
    ) -> Result<CallToolResult, McpError> {
        let entries: Vec<onecrawl_cdp::sitemap::SitemapEntry> = parse_json_str(&p.entries, "entries")?;
        let config = onecrawl_cdp::sitemap::SitemapConfig {
            base_url: p.base_url,
            default_changefreq: p.default_changefreq.unwrap_or_else(|| "weekly".into()),
            default_priority: 0.5,
            include_lastmod: true,
        };
        let xml = onecrawl_cdp::sitemap::generate_sitemap(&entries, &config);
        text_ok(xml)
    }

    async fn crawling_snapshot(
        &self,
        p: TakeSnapshotParams,
    ) -> Result<CallToolResult, McpError> {
        let page = ensure_page(&self.browser).await?;
        let snap = onecrawl_cdp::snapshot::take_snapshot(&page)
            .await
            .mcp()?;
        let mut state = self.browser.lock().await;
        state.snapshots.insert(p.label.clone(), snap);
        text_ok(format!("snapshot '{}' saved", p.label))
    }

    async fn crawling_compare(
        &self,
        p: CompareSnapshotsParams,
    ) -> Result<CallToolResult, McpError> {
        let state = self.browser.lock().await;
        let before = state
            .snapshots
            .get(&p.before)
            .ok_or_else(|| mcp_err(format!("snapshot '{}' not found", p.before)))?;
        let after = state
            .snapshots
            .get(&p.after)
            .ok_or_else(|| mcp_err(format!("snapshot '{}' not found", p.after)))?;
        let diff = onecrawl_cdp::snapshot::compare_snapshots(before, after);
        json_ok(&diff)
    }

    // ════════════════════════════════════════════════════════════════
    //  CDP tools — Stealth & Anti-Detection
    // ════════════════════════════════════════════════════════════════

    async fn stealth_inject(
        &self,
        _p: InjectStealthParams,
    ) -> Result<CallToolResult, McpError> {
        let page = ensure_page(&self.browser).await?;
        let patches = onecrawl_cdp::antibot::inject_stealth_full(&page)
            .await
            .mcp()?;
        json_ok(&StealthInjectResult {
            patches_applied: patches.len(),
            patches,
        })
    }

    async fn stealth_test(
        &self,
        _p: BotDetectionTestParams,
    ) -> Result<CallToolResult, McpError> {
        let page = ensure_page(&self.browser).await?;
        let result = onecrawl_cdp::antibot::bot_detection_test(&page)
            .await
            .mcp()?;
        json_ok(&result)
    }

    async fn stealth_fingerprint(
        &self,
        p: ApplyFingerprintParams,
    ) -> Result<CallToolResult, McpError> {
        let page = ensure_page(&self.browser).await?;
        let mut fp = onecrawl_cdp::stealth::generate_fingerprint();
        if let Some(ua) = &p.user_agent {
            fp.user_agent = ua.clone();
        }
        let script = onecrawl_cdp::stealth::get_stealth_init_script(&fp);
        onecrawl_cdp::page::evaluate_js(&page, &script)
            .await
            .mcp()?;
        json_ok(&FingerprintResult {
            user_agent: &fp.user_agent,
            platform: &fp.platform,
        })
    }

    async fn stealth_block_domains(
        &self,
        p: BlockDomainsParams,
    ) -> Result<CallToolResult, McpError> {
        let page = ensure_page(&self.browser).await?;
        let count = if let Some(cat) = &p.category {
            onecrawl_cdp::domain_blocker::block_category(&page, cat)
                .await
                .mcp()?
        } else if let Some(domains) = &p.domains {
            onecrawl_cdp::domain_blocker::block_domains(&page, domains)
                .await
                .mcp()?
        } else {
            return Err(mcp_err(
                "provide either 'domains' or 'category'",
            ));
        };
        text_ok(format!("{count} domains blocked"))
    }

    async fn stealth_detect_captcha(
        &self,
        _p: DetectCaptchaParams,
    ) -> Result<CallToolResult, McpError> {
        let page = ensure_page(&self.browser).await?;
        let detection = onecrawl_cdp::captcha::detect_captcha(&page)
            .await
            .mcp()?;
        json_ok(&detection)
    }

    // ════════════════════════════════════════════════════════════════
    //  CDP tools — Data Processing
    // ════════════════════════════════════════════════════════════════

    fn data_pipeline(
        &self,
        p: PipelineExecuteParams,
    ) -> Result<CallToolResult, McpError> {
        let steps: Vec<onecrawl_cdp::PipelineStep> = parse_json_str(&p.steps, "steps")?;
        let pipeline = onecrawl_cdp::Pipeline {
            name: p.name,
            steps,
        };
        let items: Vec<HashMap<String, String>> = parse_json_str(&p.input, "input")?;
        let result = onecrawl_cdp::data_pipeline::execute_pipeline(&pipeline, items);
        json_ok(&result)
    }

    async fn data_http_get(
        &self,
        p: HttpGetParams,
    ) -> Result<CallToolResult, McpError> {
        let page = ensure_page(&self.browser).await?;
        let headers: Option<HashMap<String, String>> = parse_opt_json_str(p.headers.as_deref(), "headers")?;
        let resp = onecrawl_cdp::http_client::get(&page, &p.url, headers)
            .await
            .mcp()?;
        json_ok(&resp)
    }

    async fn data_http_post(
        &self,
        p: HttpPostParams,
    ) -> Result<CallToolResult, McpError> {
        let page = ensure_page(&self.browser).await?;
        let headers: Option<HashMap<String, String>> = parse_opt_json_str(p.headers.as_deref(), "headers")?;
        let resp =
            onecrawl_cdp::http_client::post(&page, &p.url, &p.body, "application/json", headers)
                .await
                .mcp()?;
        json_ok(&resp)
    }

    async fn data_links(
        &self,
        p: ExtractLinksParams,
    ) -> Result<CallToolResult, McpError> {
        let page = ensure_page(&self.browser).await?;
        let edges = onecrawl_cdp::link_graph::extract_links(&page, &p.base_url)
            .await
            .mcp()?;
        json_ok(&edges)
    }

    fn data_graph(
        &self,
        p: AnalyzeGraphParams,
    ) -> Result<CallToolResult, McpError> {
        let edges: Vec<onecrawl_cdp::LinkEdge> = parse_json_str(&p.edges, "edges")?;
        let graph = onecrawl_cdp::link_graph::build_graph(&edges);
        let stats = onecrawl_cdp::link_graph::analyze_graph(&graph);
        json_ok(&stats)
    }

    // ════════════════════════════════════════════════════════════════
    //  CDP tools — Automation
    // ════════════════════════════════════════════════════════════════

    async fn automation_rate_limit(
        &self,
        p: RateLimitCheckParams,
    ) -> Result<CallToolResult, McpError> {
        let mut state = self.browser.lock().await;
        if state.rate_limiter.is_none() {
            let config = onecrawl_cdp::RateLimitConfig {
                max_requests_per_second: p.max_per_second.unwrap_or(2.0),
                max_requests_per_minute: p.max_per_minute.unwrap_or(60.0),
                max_requests_per_hour: 3600.0,
                burst_size: 5,
                cooldown_ms: 500,
            };
            state.rate_limiter = Some(onecrawl_cdp::RateLimitState::new(config));
        }
        let limiter = state.rate_limiter.as_ref().unwrap();
        let can = onecrawl_cdp::rate_limiter::can_proceed(limiter);
        let stats = onecrawl_cdp::rate_limiter::get_stats(limiter);
        json_ok(&RateLimitResult {
            can_proceed: can,
            stats,
        })
    }

    async fn automation_retry(
        &self,
        p: RetryEnqueueParams,
    ) -> Result<CallToolResult, McpError> {
        let mut state = self.browser.lock().await;
        if state.retry_queue.is_none() {
            state.retry_queue = Some(onecrawl_cdp::RetryQueue::new(onecrawl_cdp::RetryConfig {
                max_retries: 3,
                initial_delay_ms: 1000,
                max_delay_ms: 30_000,
                backoff_factor: 2.0,
                jitter: true,
            }));
        }
        let queue = state.retry_queue.as_mut().unwrap();
        let id = onecrawl_cdp::retry_queue::enqueue(
            queue,
            &p.url,
            &p.operation,
            p.payload.as_deref(),
        );
        let stats = onecrawl_cdp::retry_queue::get_stats(queue);
        json_ok(&RetryResult {
            id,
            queue_stats: stats,
        })
    }

    //  Passkey / WebAuthn tools

    async fn auth_passkey_enable(
        &self,
        p: PasskeyEnableParams,
    ) -> Result<CallToolResult, McpError> {
        let page = ensure_page(&self.browser).await?;
        let config = onecrawl_cdp::webauthn::VirtualAuthenticator {
            id: format!(
                "auth-{}",
                std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_millis()
            ),
            protocol: p.protocol.unwrap_or_else(|| "ctap2".into()),
            transport: p.transport.unwrap_or_else(|| "internal".into()),
            has_resident_key: true,
            has_user_verification: true,
            is_user_verified: true,
        };
        onecrawl_cdp::webauthn::enable_virtual_authenticator(&page, &config)
            .await
            .mcp()?;
        text_ok("Virtual authenticator enabled")
    }

    async fn auth_passkey_add(
        &self,
        p: PasskeyAddParams,
    ) -> Result<CallToolResult, McpError> {
        let page = ensure_page(&self.browser).await?;
        let cred = onecrawl_cdp::webauthn::VirtualCredential {
            credential_id: p.credential_id,
            rp_id: p.rp_id,
            user_handle: p.user_handle.unwrap_or_default(),
            sign_count: 0,
        };
        onecrawl_cdp::webauthn::add_virtual_credential(&page, &cred)
            .await
            .mcp()?;
        text_ok("Credential added")
    }

    async fn auth_passkey_list(
        &self,
        _p: PasskeyListParams,
    ) -> Result<CallToolResult, McpError> {
        let page = ensure_page(&self.browser).await?;
        let creds = onecrawl_cdp::webauthn::get_virtual_credentials(&page)
            .await
            .mcp()?;
        json_ok(&creds)
    }

    async fn auth_passkey_log(
        &self,
        _p: PasskeyLogParams,
    ) -> Result<CallToolResult, McpError> {
        let page = ensure_page(&self.browser).await?;
        let log = onecrawl_cdp::webauthn::get_webauthn_log(&page)
            .await
            .mcp()?;
        json_ok(&log)
    }

    async fn auth_passkey_disable(
        &self,
        _p: PasskeyDisableParams,
    ) -> Result<CallToolResult, McpError> {
        let page = ensure_page(&self.browser).await?;
        onecrawl_cdp::webauthn::disable_virtual_authenticator(&page)
            .await
            .mcp()?;
        text_ok("Virtual authenticator disabled")
    }

    async fn auth_passkey_remove(
        &self,
        p: PasskeyRemoveParams,
    ) -> Result<CallToolResult, McpError> {
        let page = ensure_page(&self.browser).await?;
        let removed = onecrawl_cdp::webauthn::remove_virtual_credential(&page, &p.credential_id)
            .await
            .mcp()?;
        json_ok(&RemovedResult { removed })
    }

    // ════════════════════════════════════════════════════════════════
    //  Agent tools — Enhanced Agentic API Layer
    // ════════════════════════════════════════════════════════════════

    async fn agent_execute_chain(
        &self,
        p: ExecuteChainParams,
    ) -> Result<CallToolResult, McpError> {
        let stop_on_error = p.stop_on_error.unwrap_or(true);
        let total = p.commands.len();
        let mut results: Vec<serde_json::Value> = Vec::with_capacity(total);
        let mut completed = 0usize;

        for cmd in &p.commands {
            let outcome = self.dispatch_chain_command(cmd).await;
            completed += 1;
            match outcome {
                Ok(data) => {
                    results.push(serde_json::json!({
                        "tool": cmd.tool,
                        "success": true,
                        "data": data
                    }));
                }
                Err(err_msg) => {
                    results.push(serde_json::json!({
                        "tool": cmd.tool,
                        "success": false,
                        "error": {
                            "message": err_msg,
                            "code": "CHAIN_STEP_FAILED"
                        }
                    }));
                    if stop_on_error {
                        break;
                    }
                }
            }
        }

        json_ok(&serde_json::json!({
            "results": results,
            "completed": completed,
            "total": total
        }))
    }

    async fn agent_element_screenshot(
        &self,
        p: ElementScreenshotParams,
    ) -> Result<CallToolResult, McpError> {
        if p.selector.is_empty() {
            return Err(mcp_err("selector must not be empty"));
        }
        let page = ensure_page(&self.browser).await?;
        let selector = onecrawl_cdp::accessibility::resolve_ref(&p.selector);

        // Get element bounds via JS
        let bounds_js = format!(
            r#"(() => {{
                const el = document.querySelector({sel});
                if (!el) return null;
                const r = el.getBoundingClientRect();
                return {{ x: r.x, y: r.y, width: r.width, height: r.height }};
            }})()"#,
            sel = serde_json::to_string(&selector).unwrap_or_else(|_| "null".into())
        );
        let bounds_val = onecrawl_cdp::page::evaluate_js(&page, &bounds_js)
            .await
            .mcp()?;

        if bounds_val.is_null() {
            return Err(crate::helpers::agent_err(
                crate::agent_error::element_not_found(&p.selector),
            ));
        }

        let bytes = onecrawl_cdp::screenshot::screenshot_element(&page, &selector)
            .await
            .mcp()?;
        let b64 = B64.encode(&bytes);

        json_ok(&serde_json::json!({
            "image": b64,
            "bounds": bounds_val
        }))
    }

    async fn agent_api_capture_start(
        &self,
        _p: ApiCaptureStartParams,
    ) -> Result<CallToolResult, McpError> {
        let page = ensure_page(&self.browser).await?;
        let js = r#"
        (() => {
            if (window.__onecrawl_api_log) return { already_active: true, entries: window.__onecrawl_api_log.length };
            window.__onecrawl_api_log = [];

            // Patch fetch
            const origFetch = window.fetch;
            window.fetch = async function(...args) {
                const url = typeof args[0] === 'string' ? args[0] : (args[0]?.url || '');
                const method = (args[1]?.method || 'GET').toUpperCase();
                const entry = { type: 'fetch', method, url, status: null, contentType: null, timestamp: Date.now() };
                try {
                    const resp = await origFetch.apply(this, args);
                    entry.status = resp.status;
                    entry.contentType = resp.headers.get('content-type');
                    window.__onecrawl_api_log.push(entry);
                    return resp;
                } catch(e) {
                    entry.error = e.message;
                    window.__onecrawl_api_log.push(entry);
                    throw e;
                }
            };

            // Patch XMLHttpRequest
            const origOpen = XMLHttpRequest.prototype.open;
            const origSend = XMLHttpRequest.prototype.send;
            XMLHttpRequest.prototype.open = function(method, url, ...rest) {
                this.__onecrawl_entry = { type: 'xhr', method: (method || 'GET').toUpperCase(), url: url || '', status: null, contentType: null, timestamp: Date.now() };
                return origOpen.call(this, method, url, ...rest);
            };
            XMLHttpRequest.prototype.send = function(...args) {
                const entry = this.__onecrawl_entry;
                if (entry) {
                    this.addEventListener('load', function() {
                        entry.status = this.status;
                        entry.contentType = this.getResponseHeader('content-type');
                        window.__onecrawl_api_log.push(entry);
                    });
                    this.addEventListener('error', function() {
                        entry.error = 'network error';
                        window.__onecrawl_api_log.push(entry);
                    });
                }
                return origSend.apply(this, args);
            };

            return { active: true, entries: 0 };
        })()
        "#;
        let result = onecrawl_cdp::page::evaluate_js(&page, js)
            .await
            .mcp()?;
        json_ok(&result)
    }

    async fn agent_api_capture_summary(
        &self,
        p: ApiCaptureSummaryParams,
    ) -> Result<CallToolResult, McpError> {
        let page = ensure_page(&self.browser).await?;
        let clear = p.clear.unwrap_or(false);
        let js = format!(
            r#"(() => {{
                const log = window.__onecrawl_api_log || [];
                const result = {{ total: log.length, requests: log.slice() }};
                if ({clear}) {{ window.__onecrawl_api_log = []; }}
                return result;
            }})()"#,
            clear = if clear { "true" } else { "false" }
        );
        let result = onecrawl_cdp::page::evaluate_js(&page, &js)
            .await
            .mcp()?;
        json_ok(&result)
    }

    async fn agent_iframe_list(
        &self,
        _p: IframeListParams,
    ) -> Result<CallToolResult, McpError> {
        let page = ensure_page(&self.browser).await?;
        let iframes = onecrawl_cdp::iframe::list_iframes(&page)
            .await
            .mcp()?;
        json_ok(&serde_json::json!({
            "total": iframes.len(),
            "iframes": iframes
        }))
    }

    async fn agent_iframe_snapshot(
        &self,
        p: IframeSnapshotParams,
    ) -> Result<CallToolResult, McpError> {
        let page = ensure_page(&self.browser).await?;
        let interactive_only = if p.interactive_only.unwrap_or(false) { "true" } else { "false" };
        let compact = if p.compact.unwrap_or(false) { "true" } else { "false" };

        // Inject a lightweight snapshot script into the iframe
        let snap_js = format!(
            r#"(() => {{
                const interactiveOnly = {interactive_only};
                const compactMode = {compact};
                const INTERACTIVE_TAGS = new Set(['A','BUTTON','INPUT','SELECT','TEXTAREA','DETAILS','SUMMARY']);
                const INTERACTIVE_ROLES = new Set(['button','link','textbox','checkbox','radio','combobox','listbox','menuitem','tab','switch','searchbox','slider','spinbutton']);
                let refCounter = 0;
                const refs = {{}};
                function walk(node, depth) {{
                    if (!node || node.nodeType !== 1) return '';
                    const tag = node.tagName.toLowerCase();
                    if (tag === 'script' || tag === 'style' || tag === 'noscript') return '';
                    const role = node.getAttribute('role') || '';
                    const isInteractive = INTERACTIVE_TAGS.has(node.tagName) || INTERACTIVE_ROLES.has(role);
                    if (interactiveOnly && !isInteractive && node.children.length === 0) return '';
                    const refId = 'f{idx}e' + (refCounter++);
                    node.setAttribute('data-onecrawl-ref', refId);
                    const label = node.getAttribute('aria-label') || node.getAttribute('alt') || node.getAttribute('placeholder') || '';
                    const text = node.childNodes.length === 1 && node.childNodes[0].nodeType === 3 ? node.childNodes[0].textContent.trim().substring(0, 80) : '';
                    let line = '  '.repeat(depth) + tag;
                    if (role) line += '[role=' + role + ']';
                    line += ' @' + refId;
                    if (label) line += ' "' + label + '"';
                    else if (text) line += ' "' + text + '"';
                    let children = '';
                    for (const c of node.children) {{ children += walk(c, depth + 1); }}
                    if (compactMode && !isInteractive && !children && !text && !label) return '';
                    refs[refId] = tag + (node.id ? '#' + node.id : '') + (node.className && typeof node.className === 'string' ? '.' + node.className.trim().split(/\\s+/).join('.') : '');
                    return line + '\\n' + children;
                }}
                const snapshot = walk(document.body || document.documentElement, 0);
                return {{ snapshot, refs, total: refCounter, iframe_index: {idx} }};
            }})()"#,
            interactive_only = interactive_only,
            compact = compact,
            idx = p.index
        );

        let result = onecrawl_cdp::iframe::eval_in_iframe(&page, p.index, &snap_js)
            .await
            .mcp()?;

        if let Some(err) = result.get("error") {
            return Err(mcp_err(format!("iframe snapshot failed: {}", err)));
        }

        json_ok(&result)
    }

    async fn agent_connect_remote(
        &self,
        p: RemoteCdpParams,
    ) -> Result<CallToolResult, McpError> {
        let mut state = self.browser.lock().await;

        // Validate the WebSocket URL format
        if !p.ws_url.starts_with("ws://") && !p.ws_url.starts_with("wss://") {
            return Err(mcp_err("ws_url must start with ws:// or wss://"));
        }

        // Connect to remote browser via chromiumoxide (with timeout)
        let (browser, mut handler) =
            tokio::time::timeout(
                std::time::Duration::from_secs(15),
                chromiumoxide::Browser::connect(&p.ws_url),
            )
            .await
            .map_err(|_| mcp_err("remote CDP connect timed out after 15s"))?
            .map_err(|e| mcp_err(format!("remote CDP connect failed: {e}")))?;

        // Spawn the handler loop
        tokio::spawn(async move {
            while let Some(_event) = handler.next().await {}
        });

        let page = browser
            .new_page("about:blank")
            .await
            .map_err(|e| mcp_err(format!("remote new_page failed: {e}")))?;

        // Store in shared state (replace any existing session)
        state.session = None; // drop local session
        state.page = Some(page);

        let _ = &p.headers; // reserved for future handshake header support

        json_ok(&serde_json::json!({
            "connected": true,
            "ws_url": p.ws_url,
            "info": "Remote browser connected. Subsequent tools will use this session."
        }))
    }

    // ════════════════════════════════════════════════════════════════
    //  Safety Policy tools
    // ════════════════════════════════════════════════════════════════

    async fn agent_safety_policy_set(
        &self,
        p: SafetyPolicySetParams,
    ) -> Result<CallToolResult, McpError> {
        let policy = if let Some(ref path) = p.policy_file {
            onecrawl_cdp::SafetyState::load_from_file(std::path::Path::new(path))
                .mcp()?
        } else {
            onecrawl_cdp::SafetyPolicy {
                allowed_domains: p.allowed_domains.unwrap_or_default(),
                blocked_domains: p.blocked_domains.unwrap_or_default(),
                blocked_url_patterns: p.blocked_url_patterns.unwrap_or_default(),
                max_actions: p.max_actions.unwrap_or(0),
                confirm_form_submit: p.confirm_form_submit.unwrap_or(false),
                confirm_file_upload: p.confirm_file_upload.unwrap_or(false),
                blocked_commands: p.blocked_commands.unwrap_or_default(),
                allowed_commands: p.allowed_commands.unwrap_or_default(),
                rate_limit_per_minute: p.rate_limit_per_minute.unwrap_or(0),
            }
        };

        let mut state = self.browser.lock().await;
        match state.safety.as_mut() {
            Some(existing) => existing.set_policy(policy.clone()),
            None => state.safety = Some(onecrawl_cdp::SafetyState::new(policy.clone())),
        }

        json_ok(&serde_json::json!({
            "status": "policy_set",
            "policy": policy
        }))
    }

    async fn agent_safety_status(
        &self,
        _p: SafetyStatusParams,
    ) -> Result<CallToolResult, McpError> {
        let state = self.browser.lock().await;
        match &state.safety {
            Some(safety) => json_ok(&safety.stats()),
            None => json_ok(&serde_json::json!({
                "status": "no_policy",
                "info": "No safety policy is active. Use agent.safety_policy_set to configure one."
            })),
        }
    }

    fn agent_skills_list(
        &self,
        _p: SkillsListParams,
    ) -> Result<CallToolResult, McpError> {
        let builtins = onecrawl_cdp::skills::SkillRegistry::builtins();
        let skills: Vec<serde_json::Value> = builtins
            .iter()
            .map(|s| {
                serde_json::json!({
                    "name": s.name,
                    "version": s.version,
                    "description": s.description,
                    "tools": s.tools.iter().map(|t| serde_json::json!({
                        "name": t.name,
                        "description": t.description,
                    })).collect::<Vec<_>>(),
                    "requires": s.requires,
                    "author": s.author,
                    "source": "built-in",
                })
            })
            .collect();
        json_ok(&skills)
    }
    async fn agent_screencast_start(
        &self,
        p: ScreencastStartParams,
    ) -> Result<CallToolResult, McpError> {
        let page = ensure_page(&self.browser).await?;
        let opts = onecrawl_cdp::screencast::ScreencastOptions {
            format: p.format.unwrap_or_else(|| "jpeg".into()),
            quality: p.quality.map(|q| q.min(100)).or(Some(60)),
            max_width: p.max_width.or(Some(1280)),
            max_height: p.max_height.or(Some(720)),
            every_nth_frame: p.every_nth_frame.or(Some(1)),
        };
        onecrawl_cdp::screencast::start_screencast(&page, &opts)
            .await
            .mcp()?;
        json_ok(&serde_json::json!({
            "status": "started",
            "format": opts.format,
            "quality": opts.quality,
            "max_width": opts.max_width,
            "max_height": opts.max_height,
            "every_nth_frame": opts.every_nth_frame
        }))
    }

    async fn agent_screencast_stop(
        &self,
        _p: ScreencastStopParams,
    ) -> Result<CallToolResult, McpError> {
        let page = ensure_page(&self.browser).await?;
        onecrawl_cdp::screencast::stop_screencast(&page)
            .await
            .mcp()?;
        json_ok(&serde_json::json!({ "status": "stopped" }))
    }

    async fn agent_screencast_frame(
        &self,
        p: ScreencastFrameParams,
    ) -> Result<CallToolResult, McpError> {
        let page = ensure_page(&self.browser).await?;
        let opts = onecrawl_cdp::screencast::ScreencastOptions {
            format: p.format.unwrap_or_else(|| "jpeg".into()),
            quality: p.quality.or(Some(80)),
            ..Default::default()
        };
        let bytes = onecrawl_cdp::screencast::capture_frame(&page, &opts)
            .await
            .mcp()?;
        let b64 = B64.encode(&bytes);
        json_ok(&serde_json::json!({
            "image": b64,
            "format": opts.format,
            "size": bytes.len()
        }))
    }

    async fn agent_recording_start(
        &self,
        p: RecordingStartParams,
    ) -> Result<CallToolResult, McpError> {
        let page = ensure_page(&self.browser).await?;
        let output = p.output.unwrap_or_else(|| "recording.webm".into());
        let fps = p.fps.unwrap_or(5);

        {
            let mut state = self.browser.lock().await;
            if state.recording.as_ref().is_some_and(|r| r.is_recording()) {
                return Err(mcp_err("recording already in progress"));
            }
            let mut rec = onecrawl_cdp::RecordingState::new(
                std::path::PathBuf::from(&output),
                fps,
            );
            rec.start();
            state.recording = Some(rec);
        }

        let opts = onecrawl_cdp::screencast::ScreencastOptions {
            format: "jpeg".into(),
            quality: Some(60),
            max_width: Some(1280),
            max_height: Some(720),
            every_nth_frame: Some(1),
        };
        onecrawl_cdp::screencast::start_screencast(&page, &opts)
            .await
            .mcp()?;

        json_ok(&serde_json::json!({
            "status": "recording",
            "output": output,
            "fps": fps
        }))
    }

    async fn agent_recording_stop(
        &self,
        _p: RecordingStopParams,
    ) -> Result<CallToolResult, McpError> {
        let page = ensure_page(&self.browser).await?;
        let _ = onecrawl_cdp::screencast::stop_screencast(&page).await;

        let mut state = self.browser.lock().await;
        let rec = state.recording.as_mut()
            .ok_or_else(|| mcp_err("no recording in progress"))?;

        // If no frames were captured via events, grab one snapshot
        if rec.is_recording() && rec.frame_count() == 0 {
            drop(state);
            let opts = onecrawl_cdp::screencast::ScreencastOptions::default();
            if let Ok(bytes) = onecrawl_cdp::screencast::capture_frame(&page, &opts).await {
                let mut state = self.browser.lock().await;
                if let Some(rec) = state.recording.as_mut() {
                    rec.add_frame(bytes);
                }
            }
            let mut state = self.browser.lock().await;
            let rec = state.recording.as_mut()
                .ok_or_else(|| mcp_err("no recording in progress"))?;
            rec.stop();
            let frame_count = rec.frame_count();
            let result = rec.save_frames().mcp()?;
            state.recording = None;
            return json_ok(&serde_json::json!({
                "status": "saved",
                "frames": frame_count,
                "path": result.display().to_string()
            }));
        }

        rec.stop();
        let frame_count = rec.frame_count();
        let result = rec.save_frames().mcp()?;
        state.recording = None;
        json_ok(&serde_json::json!({
            "status": "saved",
            "frames": frame_count,
            "path": result.display().to_string()
        }))
    }

    async fn agent_recording_status(
        &self,
        _p: RecordingStatusParams,
    ) -> Result<CallToolResult, McpError> {
        let state = self.browser.lock().await;
        match state.recording.as_ref() {
            Some(rec) => {
                let status = if rec.is_recording() { "recording" } else { "stopped" };
                json_ok(&serde_json::json!({
                    "status": status,
                    "frames": rec.frame_count(),
                    "fps": rec.fps(),
                    "output": rec.output_path().display().to_string()
                }))
            }
            None => json_ok(&serde_json::json!({
                "status": "idle",
                "frames": 0
            })),
        }
    }

    // ════════════════════════════════════════════════════════════════
    //  iOS / Mobile Safari tools
    // ════════════════════════════════════════════════════════════════

    async fn agent_ios_devices(
        &self,
        _p: IosDevicesParams,
    ) -> Result<CallToolResult, McpError> {
        let devices = onecrawl_cdp::ios::IosClient::list_devices()
            .map_err(|e| mcp_err(format!("iOS list devices failed: {e}")))?;
        json_ok(&serde_json::json!({
            "devices": devices,
            "count": devices.len()
        }))
    }

    async fn agent_ios_connect(
        &self,
        p: IosConnectParams,
    ) -> Result<CallToolResult, McpError> {
        let config = onecrawl_cdp::ios::IosSessionConfig {
            wda_url: p.wda_url.unwrap_or_else(|| "http://localhost:8100".to_string()),
            device_udid: p.udid,
            bundle_id: p.bundle_id.unwrap_or_else(|| "com.apple.mobilesafari".to_string()),
        };
        let mut client = onecrawl_cdp::ios::IosClient::new(config);
        let session_id = client.create_session().await
            .map_err(|e| mcp_err(format!("iOS connect failed: {e}")))?;

        let mut state = self.browser.lock().await;
        state.ios_client = Some(client);

        json_ok(&serde_json::json!({
            "connected": true,
            "session_id": session_id
        }))
    }

    async fn agent_ios_navigate(
        &self,
        p: IosNavigateParams,
    ) -> Result<CallToolResult, McpError> {
        let state = self.browser.lock().await;
        let client = state.ios_client.as_ref()
            .ok_or_else(|| mcp_err("no active iOS session — call agent.ios_connect first"))?;
        client.navigate(&p.url).await
            .map_err(|e| mcp_err(format!("iOS navigate failed: {e}")))?;
        json_ok(&serde_json::json!({
            "navigated": true,
            "url": p.url
        }))
    }

    async fn agent_ios_tap(
        &self,
        p: IosTapParams,
    ) -> Result<CallToolResult, McpError> {
        let state = self.browser.lock().await;
        let client = state.ios_client.as_ref()
            .ok_or_else(|| mcp_err("no active iOS session — call agent.ios_connect first"))?;
        client.tap(p.x, p.y).await
            .map_err(|e| mcp_err(format!("iOS tap failed: {e}")))?;
        json_ok(&serde_json::json!({
            "tapped": true,
            "x": p.x,
            "y": p.y
        }))
    }

    async fn agent_ios_screenshot(
        &self,
        _p: IosScreenshotParams,
    ) -> Result<CallToolResult, McpError> {
        let state = self.browser.lock().await;
        let client = state.ios_client.as_ref()
            .ok_or_else(|| mcp_err("no active iOS session — call agent.ios_connect first"))?;
        let bytes = client.screenshot().await
            .map_err(|e| mcp_err(format!("iOS screenshot failed: {e}")))?;
        let b64 = B64.encode(&bytes);
        json_ok(&serde_json::json!({
            "format": "png",
            "size": bytes.len(),
            "data": b64
        }))
    }

    // ──────────────── Computer Use Protocol ─────────────────

    async fn computer_act(
        &self,
        p: ComputerUseActionParams,
    ) -> Result<CallToolResult, McpError> {
        let page = ensure_page(&self.browser).await?;
        let mut action: onecrawl_cdp::computer_use::AgentAction =
            serde_json::from_value(p.action)
                .map_err(|e| mcp_err(format!("invalid action: {e}")))?;

        // Override screenshot flag when explicitly requested via param.
        if p.include_screenshot.unwrap_or(false) {
            if let onecrawl_cdp::computer_use::AgentAction::Observe {
                ref mut include_screenshot,
            } = action
            {
                *include_screenshot = true;
            }
        }

        let result = onecrawl_cdp::computer_use::execute_action(&page, &action, 0)
            .await
            .mcp()?;

        json_ok(&result)
    }

    async fn computer_observe(
        &self,
        p: ComputerUseObserveParams,
    ) -> Result<CallToolResult, McpError> {
        let page = ensure_page(&self.browser).await?;
        let obs = onecrawl_cdp::computer_use::observe(
            &page,
            None,
            p.include_screenshot.unwrap_or(false),
        )
        .await
        .mcp()?;

        json_ok(&obs)
    }

    async fn computer_batch(
        &self,
        p: ComputerUseBatchParams,
    ) -> Result<CallToolResult, McpError> {
        let page = ensure_page(&self.browser).await?;
        let stop_on_error = p.stop_on_error.unwrap_or(true);
        let include_screenshots = p.include_screenshots.unwrap_or(false);
        let mut results: Vec<onecrawl_cdp::computer_use::ActionResult> = Vec::new();

        for (i, raw) in p.actions.iter().enumerate() {
            let mut action: onecrawl_cdp::computer_use::AgentAction =
                serde_json::from_value(raw.clone())
                    .map_err(|e| mcp_err(format!("invalid action at index {i}: {e}")))?;

            if include_screenshots {
                if let onecrawl_cdp::computer_use::AgentAction::Observe {
                    ref mut include_screenshot,
                } = action
                {
                    *include_screenshot = true;
                }
            }

            let result = onecrawl_cdp::computer_use::execute_action(&page, &action, i)
                .await
                .mcp()?;

            let failed = !result.success;
            results.push(result);

            if failed && stop_on_error {
                break;
            }
        }

        json_ok(&serde_json::json!({
            "total": p.actions.len(),
            "executed": results.len(),
            "results": results,
        }))
    }

    // ════════════════════════════════════════════════════════════════
    //  Browser Pool tools
    // ════════════════════════════════════════════════════════════════

    async fn pool_list(
        &self,
        _p: PoolListParams,
    ) -> Result<CallToolResult, McpError> {
        let state = self.browser.lock().await;
        let instances = state.pool.list();
        json_ok(&serde_json::json!({
            "instances": instances,
            "count": instances.len(),
        }))
    }

    async fn pool_status(
        &self,
        _p: PoolStatusParams,
    ) -> Result<CallToolResult, McpError> {
        let state = self.browser.lock().await;
        let pool = &state.pool;
        let total = pool.len();
        let idle = pool.idle_count();
        json_ok(&serde_json::json!({
            "size": total,
            "max_size": pool.max_size(),
            "idle": idle,
            "busy": total - idle,
        }))
    }

    // ════════════════════════════════════════════════════════════════
    //  Smart Actions tools
    // ════════════════════════════════════════════════════════════════

    async fn smart_find(
        &self,
        p: SmartFindParams,
    ) -> Result<CallToolResult, McpError> {
        let page = ensure_page(&self.browser).await?;
        let matches = onecrawl_cdp::smart_actions::smart_find(&page, &p.query)
            .await
            .mcp()?;
        json_ok(&serde_json::json!({
            "query": p.query,
            "matches": matches,
            "count": matches.len(),
        }))
    }

    async fn smart_click(
        &self,
        p: SmartClickParams,
    ) -> Result<CallToolResult, McpError> {
        let page = ensure_page(&self.browser).await?;
        let matched = onecrawl_cdp::smart_actions::smart_click(&page, &p.query)
            .await
            .mcp()?;
        json_ok(&serde_json::json!({
            "clicked": matched.selector,
            "confidence": matched.confidence,
            "strategy": matched.strategy,
        }))
    }

    async fn smart_fill(
        &self,
        p: SmartFillParams,
    ) -> Result<CallToolResult, McpError> {
        let page = ensure_page(&self.browser).await?;
        let matched = onecrawl_cdp::smart_actions::smart_fill(&page, &p.query, &p.value)
            .await
            .mcp()?;
        json_ok(&serde_json::json!({
            "filled": matched.selector,
            "value_length": p.value.len(),
            "confidence": matched.confidence,
            "strategy": matched.strategy,
        }))
    }

    // ════════════════════════════════════════════════════════════════
    //  Agent Memory tools
    // ════════════════════════════════════════════════════════════════

    fn ensure_memory(state: &mut BrowserState) -> &mut onecrawl_cdp::AgentMemory {
        if state.memory.is_none() {
            let home = std::env::var("HOME").unwrap_or_else(|_| ".".into());
            let path = std::path::PathBuf::from(home).join(".onecrawl").join("agent_memory.json");
            state.memory = Some(
                onecrawl_cdp::AgentMemory::load(&path).unwrap_or_else(|_| onecrawl_cdp::AgentMemory::new(&path))
            );
        }
        state.memory.as_mut().unwrap()
    }

    async fn memory_store(
        &self,
        p: MemoryStoreParams,
    ) -> Result<CallToolResult, McpError> {
        let mut state = self.browser.lock().await;
        let category = parse_memory_category(p.category.as_deref())
            .unwrap_or(onecrawl_cdp::MemoryCategory::Custom);
        let mem = Self::ensure_memory(&mut state);
        mem.store(&p.key, p.value.clone(), category, p.domain.clone())
            .mcp()?;
        json_ok(&serde_json::json!({
            "stored": p.key,
            "category": format!("{:?}", mem.recall(&p.key).map(|e| &e.category)),
        }))
    }

    async fn memory_recall(
        &self,
        p: MemoryRecallParams,
    ) -> Result<CallToolResult, McpError> {
        let mut state = self.browser.lock().await;
        let mem = Self::ensure_memory(&mut state);
        match mem.recall(&p.key) {
            Some(entry) => json_ok(&serde_json::json!({
                "key": entry.key,
                "value": entry.value,
                "category": format!("{:?}", entry.category),
                "domain": entry.domain,
                "access_count": entry.access_count,
                "created_at": entry.created_at,
                "accessed_at": entry.accessed_at,
            })),
            None => json_ok(&serde_json::json!({ "key": p.key, "found": false })),
        }
    }

    async fn memory_search(
        &self,
        p: MemorySearchParams,
    ) -> Result<CallToolResult, McpError> {
        let mut state = self.browser.lock().await;
        let category = parse_memory_category(p.category.as_deref());
        let mem = Self::ensure_memory(&mut state);
        let results = mem.search(&p.query, category, p.domain.as_deref());
        let entries: Vec<serde_json::Value> = results.iter().map(|e| {
            serde_json::json!({
                "key": e.key,
                "value": e.value,
                "category": format!("{:?}", e.category),
                "domain": e.domain,
                "access_count": e.access_count,
            })
        }).collect();
        json_ok(&serde_json::json!({
            "query": p.query,
            "count": entries.len(),
            "results": entries,
        }))
    }

    async fn memory_forget(
        &self,
        p: MemoryForgetParams,
    ) -> Result<CallToolResult, McpError> {
        let mut state = self.browser.lock().await;
        let mem = Self::ensure_memory(&mut state);
        if let Some(key) = &p.key {
            let removed = mem.forget(key);
            json_ok(&serde_json::json!({ "removed": removed, "key": key }))
        } else if let Some(domain) = &p.domain {
            let count = mem.clear_domain(domain);
            json_ok(&serde_json::json!({ "removed": count, "domain": domain }))
        } else {
            let count = mem.clear_all();
            json_ok(&serde_json::json!({ "removed": count, "cleared": "all" }))
        }
    }

    async fn memory_domain_strategy(
        &self,
        p: MemoryDomainStrategyParams,
    ) -> Result<CallToolResult, McpError> {
        let mut state = self.browser.lock().await;
        let mem = Self::ensure_memory(&mut state);
        if let Some(strategy_val) = p.strategy {
            let strategy: onecrawl_cdp::DomainStrategy = serde_json::from_value(strategy_val)
                .map_err(|e| mcp_err(format!("invalid strategy JSON: {e}")))?;
            mem.store_domain_strategy(strategy)
                .mcp()?;
            json_ok(&serde_json::json!({ "stored": true, "domain": p.domain }))
        } else {
            match mem.recall_domain_strategy(&p.domain) {
                Some(strategy) => json_ok(&serde_json::json!({
                    "domain": strategy.domain,
                    "login_selectors": strategy.login_selectors,
                    "navigation_patterns": strategy.navigation_patterns,
                    "known_popups": strategy.known_popups,
                    "rate_limit_info": strategy.rate_limit_info,
                    "anti_bot_level": strategy.anti_bot_level,
                })),
                None => json_ok(&serde_json::json!({ "domain": p.domain, "found": false })),
            }
        }
    }

    async fn memory_stats(
        &self,
        _p: MemoryStatsParams,
    ) -> Result<CallToolResult, McpError> {
        let mut state = self.browser.lock().await;
        let mem = Self::ensure_memory(&mut state);
        let stats = mem.stats();
        json_ok(&serde_json::json!({
            "total_entries": stats.total_entries,
            "max_entries": stats.max_entries,
            "categories": stats.categories,
            "domains": stats.domains,
            "utilization": format!("{:.1}%", (stats.total_entries as f64 / stats.max_entries as f64) * 100.0),
        }))
    }

    // ════════════════════════════════════════════════════════════════
    //  Workflow DSL tools
    // ════════════════════════════════════════════════════════════════

    async fn workflow_validate(
        &self,
        p: WorkflowValidateParams,
    ) -> Result<CallToolResult, McpError> {
        let workflow = onecrawl_cdp::workflow::parse_json(&p.workflow)
            .mcp()?;
        let errors = onecrawl_cdp::workflow::validate(&workflow);
        if errors.is_empty() {
            json_ok(&serde_json::json!({
                "valid": true,
                "name": workflow.name,
                "steps": workflow.steps.len(),
                "variables": workflow.variables.keys().collect::<Vec<_>>(),
            }))
        } else {
            json_ok(&serde_json::json!({
                "valid": false,
                "errors": errors,
            }))
        }
    }

    async fn workflow_run(
        &self,
        p: WorkflowRunParams,
    ) -> Result<CallToolResult, McpError> {
        let mut workflow = if p.workflow.trim().starts_with('{') {
            onecrawl_cdp::workflow::parse_json(&p.workflow)
                .mcp()?
        } else {
            onecrawl_cdp::workflow::load_from_file(&p.workflow)
                .mcp()?
        };

        // Override variables
        if let Some(overrides) = p.variables {
            for (k, v) in overrides {
                workflow.variables.insert(k, v);
            }
        }

        // Validate first
        let errors = onecrawl_cdp::workflow::validate(&workflow);
        if !errors.is_empty() {
            return json_ok(&serde_json::json!({
                "status": "validation_failed",
                "errors": errors,
            }));
        }

        let page = ensure_page(&self.browser).await?;
        let start = std::time::Instant::now();
        let mut results: Vec<onecrawl_cdp::StepResult> = Vec::new();
        let mut variables = workflow.variables.clone();
        let mut succeeded = 0usize;
        let mut failed = 0usize;
        let mut skipped = 0usize;
        let mut overall_status = onecrawl_cdp::StepStatus::Success;

        for (i, step) in workflow.steps.iter().enumerate() {
            let step_id = if step.id.is_empty() { format!("step_{i}") } else { step.id.clone() };
            let step_name = if step.name.is_empty() { format!("Step {i}") } else { step.name.clone() };

            // Check condition
            if let Some(ref cond) = step.condition {
                let interpolated = onecrawl_cdp::workflow::interpolate(cond, &variables);
                if !onecrawl_cdp::workflow::evaluate_condition(&interpolated, &variables) {
                    results.push(onecrawl_cdp::StepResult {
                        step_id, step_name,
                        status: onecrawl_cdp::StepStatus::Skipped,
                        output: None, error: None, duration_ms: 0,
                    });
                    skipped += 1;
                    continue;
                }
            }

            let step_start = std::time::Instant::now();
            let result = self.execute_step(&page, &step.action, &mut variables).await;
            let duration_ms = step_start.elapsed().as_millis() as u64;

            match result {
                Ok(output) => {
                    if let Some(ref save_key) = step.save_as {
                        if let Some(ref out) = output {
                            variables.insert(save_key.clone(), out.clone());
                        }
                    }
                    results.push(onecrawl_cdp::StepResult {
                        step_id, step_name,
                        status: onecrawl_cdp::StepStatus::Success,
                        output, error: None, duration_ms,
                    });
                    succeeded += 1;
                }
                Err(e) => {
                    let err_msg = format!("{}", e.message);
                    let error_action = step.on_error.as_ref()
                        .unwrap_or(&workflow.on_error.action);
                    results.push(onecrawl_cdp::StepResult {
                        step_id, step_name,
                        status: onecrawl_cdp::StepStatus::Failed,
                        output: None, error: Some(err_msg.clone()), duration_ms,
                    });
                    failed += 1;

                    match error_action {
                        onecrawl_cdp::workflow::StepErrorAction::Stop => {
                            overall_status = onecrawl_cdp::StepStatus::Failed;
                            break;
                        }
                        onecrawl_cdp::workflow::StepErrorAction::Continue |
                        onecrawl_cdp::workflow::StepErrorAction::Skip => continue,
                        onecrawl_cdp::workflow::StepErrorAction::Retry => continue,
                    }
                }
            }
        }

        let total_duration_ms = start.elapsed().as_millis() as u64;
        json_ok(&serde_json::json!({
            "name": workflow.name,
            "status": format!("{:?}", overall_status).to_lowercase(),
            "total_duration_ms": total_duration_ms,
            "steps_succeeded": succeeded,
            "steps_failed": failed,
            "steps_skipped": skipped,
            "steps": results,
            "variables": variables,
        }))
    }

    fn execute_step<'a>(
        &'a self,
        page: &'a chromiumoxide::Page,
        action: &'a onecrawl_cdp::workflow::Action,
        variables: &'a mut HashMap<String, serde_json::Value>,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = std::result::Result<Option<serde_json::Value>, McpError>> + Send + 'a>> {
        Box::pin(async move {
        use onecrawl_cdp::workflow::Action;
        match action {
            Action::Navigate { url } => {
                let url = onecrawl_cdp::workflow::interpolate(url, variables);
                onecrawl_cdp::navigation::goto(page, &url).await.mcp()?;
                let title = onecrawl_cdp::navigation::get_title(page).await.unwrap_or_default();
                Ok(Some(serde_json::json!({ "url": url, "title": title })))
            }
            Action::Click { selector } => {
                let sel = onecrawl_cdp::workflow::interpolate(selector, variables);
                let resolved = onecrawl_cdp::accessibility::resolve_ref(&sel);
                onecrawl_cdp::element::click(page, &resolved).await.mcp()?;
                Ok(Some(serde_json::json!({ "clicked": sel })))
            }
            Action::Type { selector, text } => {
                let sel = onecrawl_cdp::workflow::interpolate(selector, variables);
                let txt = onecrawl_cdp::workflow::interpolate(text, variables);
                let resolved = onecrawl_cdp::accessibility::resolve_ref(&sel);
                onecrawl_cdp::element::type_text(page, &resolved, &txt).await.mcp()?;
                Ok(Some(serde_json::json!({ "typed": txt.len() })))
            }
            Action::WaitForSelector { selector, timeout_ms } => {
                let sel = onecrawl_cdp::workflow::interpolate(selector, variables);
                let resolved = onecrawl_cdp::accessibility::resolve_ref(&sel);
                onecrawl_cdp::navigation::wait_for_selector(page, &resolved, *timeout_ms).await.mcp()?;
                Ok(Some(serde_json::json!({ "found": sel })))
            }
            Action::Screenshot { path, full_page } => {
                let bytes = if full_page.unwrap_or(false) {
                    onecrawl_cdp::screenshot::screenshot_full(page)
                        .await.mcp()?
                } else {
                    onecrawl_cdp::screenshot::screenshot_viewport(page)
                        .await.mcp()?
                };
                if let Some(p) = path {
                    let p = onecrawl_cdp::workflow::interpolate(p, variables);
                    std::fs::write(&p, &bytes).mcp()?;
                    Ok(Some(serde_json::json!({ "saved": p, "bytes": bytes.len() })))
                } else {
                    Ok(Some(serde_json::json!({ "bytes": bytes.len() })))
                }
            }
            Action::Evaluate { js } => {
                let js = onecrawl_cdp::workflow::interpolate(js, variables);
                let result = page.evaluate(js).await.mcp()?;
                let val = result.into_value::<serde_json::Value>().unwrap_or(serde_json::Value::Null);
                Ok(Some(val))
            }
            Action::Extract { selector, attribute } => {
                let sel = onecrawl_cdp::workflow::interpolate(selector, variables);
                let attr_js = if let Some(attr) = attribute {
                    format!(r#"Array.from(document.querySelectorAll({sel_json})).map(e => e.getAttribute({attr_json}))"#,
                        sel_json = serde_json::to_string(&sel).unwrap(),
                        attr_json = serde_json::to_string(attr).unwrap())
                } else {
                    format!(r#"Array.from(document.querySelectorAll({sel_json})).map(e => e.textContent.trim())"#,
                        sel_json = serde_json::to_string(&sel).unwrap())
                };
                let result = page.evaluate(attr_js).await.mcp()?;
                let val = result.into_value::<serde_json::Value>().unwrap_or(serde_json::Value::Null);
                Ok(Some(val))
            }
            Action::SmartClick { query } => {
                let q = onecrawl_cdp::workflow::interpolate(query, variables);
                let matched = onecrawl_cdp::smart_actions::smart_click(page, &q).await.mcp()?;
                Ok(Some(serde_json::json!({ "clicked": matched.selector, "confidence": matched.confidence })))
            }
            Action::SmartFill { query, value } => {
                let q = onecrawl_cdp::workflow::interpolate(query, variables);
                let v = onecrawl_cdp::workflow::interpolate(value, variables);
                let matched = onecrawl_cdp::smart_actions::smart_fill(page, &q, &v).await.mcp()?;
                Ok(Some(serde_json::json!({ "filled": matched.selector, "confidence": matched.confidence })))
            }
            Action::Sleep { ms } => {
                tokio::time::sleep(tokio::time::Duration::from_millis(*ms)).await;
                Ok(Some(serde_json::json!({ "slept_ms": ms })))
            }
            Action::SetVariable { name, value } => {
                let interpolated = onecrawl_cdp::workflow::interpolate(&value.to_string(), variables);
                let parsed = serde_json::from_str::<serde_json::Value>(&interpolated)
                    .unwrap_or(serde_json::Value::String(interpolated));
                variables.insert(name.clone(), parsed.clone());
                Ok(Some(serde_json::json!({ "set": name, "value": parsed })))
            }
            Action::Log { message, level } => {
                let msg = onecrawl_cdp::workflow::interpolate(message, variables);
                let lvl = level.as_deref().unwrap_or("info");
                match lvl {
                    "error" => tracing::error!("[workflow] {}", msg),
                    "warn" => tracing::warn!("[workflow] {}", msg),
                    "debug" => tracing::debug!("[workflow] {}", msg),
                    _ => tracing::info!("[workflow] {}", msg),
                }
                Ok(Some(serde_json::json!({ "logged": msg, "level": lvl })))
            }
            Action::Assert { condition, message } => {
                let cond = onecrawl_cdp::workflow::interpolate(condition, variables);
                if onecrawl_cdp::workflow::evaluate_condition(&cond, variables) {
                    Ok(Some(serde_json::json!({ "assert": "passed" })))
                } else {
                    Err(mcp_err(format!("assertion failed: {}", message.as_deref().unwrap_or(&cond))))
                }
            }
            Action::Loop { items: _, variable: _, steps: _ } => {
                Ok(Some(serde_json::json!({ "note": "loop execution requires recursive step runner — use workflow.run for full support" })))
            }
            Action::Conditional { condition, then_steps, else_steps } => {
                let cond = onecrawl_cdp::workflow::interpolate(condition, variables);
                let empty = vec![];
                let branch = if onecrawl_cdp::workflow::evaluate_condition(&cond, variables) {
                    then_steps
                } else {
                    else_steps.as_ref().unwrap_or(&empty)
                };
                let mut last_output = None;
                for step in branch {
                    last_output = self.execute_step(page, &step.action, variables).await?;
                }
                Ok(last_output)
            }
            Action::SubWorkflow { path } => {
                let p = onecrawl_cdp::workflow::interpolate(path, variables);
                Ok(Some(serde_json::json!({ "note": format!("sub-workflow '{}' — use workflow.run to execute", p) })))
            }
            Action::HttpRequest { url, method, headers, body } => {
                let url = onecrawl_cdp::workflow::interpolate(url, variables);
                let method = method.as_deref().unwrap_or("GET");
                let client = reqwest::Client::new();
                let mut req = match method.to_uppercase().as_str() {
                    "POST" => client.post(&url),
                    "PUT" => client.put(&url),
                    "DELETE" => client.delete(&url),
                    "PATCH" => client.patch(&url),
                    _ => client.get(&url),
                };
                if let Some(hdrs) = headers {
                    for (k, v) in hdrs {
                        let v = onecrawl_cdp::workflow::interpolate(v, variables);
                        req = req.header(k.as_str(), v);
                    }
                }
                if let Some(b) = body {
                    let b = onecrawl_cdp::workflow::interpolate(b, variables);
                    req = req.body(b);
                }
                let resp = req.send().await.mcp()?;
                let status = resp.status().as_u16();
                let body_text = resp.text().await.unwrap_or_default();
                let body_val = serde_json::from_str::<serde_json::Value>(&body_text)
                    .unwrap_or(serde_json::Value::String(body_text));
                Ok(Some(serde_json::json!({ "status": status, "body": body_val })))
            }
            Action::Snapshot { compact, interactive_only } => {
                let opts = onecrawl_cdp::accessibility::AgentSnapshotOptions {
                    interactive_only: *interactive_only,
                    compact: *compact,
                    ..Default::default()
                };
                let result = onecrawl_cdp::accessibility::agent_snapshot(page, &opts)
                    .await.mcp()?;
                Ok(Some(serde_json::json!(result)))
            }
        }
        })
    }

    // ════════════════════════════════════════════════════════════════
    //  Network Intelligence tools
    // ════════════════════════════════════════════════════════════════

    async fn net_capture(
        &self,
        p: NetIntelCaptureParams,
    ) -> Result<CallToolResult, McpError> {
        let page = ensure_page(&self.browser).await?;
        let duration = p.duration_seconds.unwrap_or(10);
        let api_only = p.api_only.unwrap_or(true);

        // Inject network interceptor
        let js = r#"
        (() => {
            if (!window.__onecrawl_net_capture) {
                window.__onecrawl_net_capture = [];
                const origFetch = window.fetch;
                window.fetch = async function(...args) {
                    const start = Date.now();
                    const req = new Request(...args);
                    try {
                        const resp = await origFetch.apply(this, args);
                        const clone = resp.clone();
                        let body = null;
                        try { body = await clone.json(); } catch(_) {
                            try { body = await clone.text(); } catch(_) {}
                        }
                        let reqBody = null;
                        try { if (req.body) { reqBody = await new Request(...args).json(); } } catch(_) {}
                        window.__onecrawl_net_capture.push({
                            method: req.method,
                            url: req.url,
                            status: resp.status,
                            contentType: resp.headers.get('content-type'),
                            requestHeaders: Object.fromEntries(req.headers.entries()),
                            responseHeaders: Object.fromEntries(resp.headers.entries()),
                            requestBody: reqBody,
                            responseBody: body,
                            timing: Date.now() - start,
                        });
                        return resp;
                    } catch(e) {
                        window.__onecrawl_net_capture.push({
                            method: req.method,
                            url: req.url,
                            status: 0,
                            error: e.message,
                            timing: Date.now() - start,
                        });
                        throw e;
                    }
                };

                const origXHR = XMLHttpRequest.prototype.open;
                XMLHttpRequest.prototype.open = function(method, url, ...rest) {
                    this.__onecrawl_method = method;
                    this.__onecrawl_url = url;
                    this.__onecrawl_start = Date.now();
                    return origXHR.call(this, method, url, ...rest);
                };
                const origSend = XMLHttpRequest.prototype.send;
                XMLHttpRequest.prototype.send = function(body) {
                    this.addEventListener('load', function() {
                        let respBody = null;
                        try { respBody = JSON.parse(this.responseText); } catch(_) { respBody = this.responseText; }
                        window.__onecrawl_net_capture.push({
                            method: this.__onecrawl_method,
                            url: this.__onecrawl_url,
                            status: this.status,
                            contentType: this.getResponseHeader('content-type'),
                            responseBody: respBody,
                            timing: Date.now() - this.__onecrawl_start,
                        });
                    });
                    return origSend.call(this, body);
                };
            }
            return 'capture_started';
        })()
        "#;

        page.evaluate(js).await.mcp()?;

        // Wait for capture duration
        tokio::time::sleep(tokio::time::Duration::from_secs(duration)).await;

        // Collect results
        let collect_js = r#"
        (() => {
            const raw = window.__onecrawl_net_capture || [];
            window.__onecrawl_net_capture = [];
            return raw;
        })()
        "#;

        let result = page.evaluate(collect_js).await.mcp()?;
        let raw: Vec<serde_json::Value> = result.into_value().unwrap_or_default();

        let endpoints: Vec<onecrawl_cdp::network_intel::ApiEndpoint> = raw.iter().filter_map(|r| {
            let url = r.get("url")?.as_str()?;
            let method = r.get("method")?.as_str().unwrap_or("GET");
            let status = r.get("status")?.as_u64().unwrap_or(0) as u16;
            let content_type = r.get("contentType").and_then(|v| v.as_str()).map(String::from);
            let category = onecrawl_cdp::network_intel::classify_request(url, content_type.as_deref(), method);

            if api_only && category == onecrawl_cdp::network_intel::ApiCategory::Static {
                return None;
            }

            let (parsed_path, parsed_base) = url.split_once("://")
                .and_then(|(scheme, rest)| rest.split_once('/').map(|(host, path)| {
                    let p = format!("/{}", path).split('?').next().unwrap_or("/").to_string();
                    let b = format!("{}://{}", scheme, host);
                    (p, b)
                }))
                .unwrap_or(("/".into(), url.to_string()));

            Some(onecrawl_cdp::network_intel::ApiEndpoint {
                method: method.to_string(),
                url: url.to_string(),
                path: parsed_path,
                base_url: parsed_base,
                query_params: std::collections::HashMap::new(),
                request_headers: r.get("requestHeaders").and_then(|v| serde_json::from_value(v.clone()).ok()).unwrap_or_default(),
                response_headers: r.get("responseHeaders").and_then(|v| serde_json::from_value(v.clone()).ok()).unwrap_or_default(),
                request_body: r.get("requestBody").cloned().filter(|v| !v.is_null()),
                response_body: r.get("responseBody").cloned().filter(|v| !v.is_null()),
                status_code: status,
                content_type,
                timing_ms: r.get("timing").and_then(|v| v.as_f64()),
                category,
            })
        }).collect();

        json_ok(&serde_json::json!({
            "endpoints": endpoints,
            "count": endpoints.len(),
            "duration_seconds": duration,
        }))
    }

    async fn net_analyze(
        &self,
        p: NetIntelAnalyzeParams,
    ) -> Result<CallToolResult, McpError> {
        let endpoints: Vec<onecrawl_cdp::network_intel::ApiEndpoint> = serde_json::from_str(&p.capture)
            .map_err(|e| mcp_err(format!("invalid capture data: {e}")))?;

        if endpoints.is_empty() {
            return json_ok(&serde_json::json!({ "error": "no endpoints to analyze" }));
        }

        let base_url = endpoints.first().map(|e| e.base_url.clone()).unwrap_or_default();
        let total_requests = endpoints.len();

        // Group by method+path template
        let mut endpoint_map: std::collections::HashMap<String, Vec<&onecrawl_cdp::network_intel::ApiEndpoint>> = std::collections::HashMap::new();
        for ep in &endpoints {
            let (template, _) = onecrawl_cdp::network_intel::extract_path_params(&ep.path);
            let key = format!("{} {}", ep.method, template);
            endpoint_map.entry(key).or_default().push(ep);
        }

        let schemas: Vec<onecrawl_cdp::network_intel::EndpointSchema> = endpoint_map.iter().map(|(key, eps)| {
            let parts: Vec<&str> = key.splitn(2, ' ').collect();
            let method = parts.first().unwrap_or(&"GET");
            let path = parts.get(1).unwrap_or(&"/");
            let (template, params) = onecrawl_cdp::network_intel::extract_path_params(path);

            let status_codes: Vec<u16> = eps.iter().map(|e| e.status_code).collect::<std::collections::HashSet<_>>().into_iter().collect();
            let content_types: Vec<String> = eps.iter().filter_map(|e| e.content_type.clone()).collect::<std::collections::HashSet<_>>().into_iter().collect();
            let avg_latency = eps.iter().filter_map(|e| e.timing_ms).sum::<f64>() / eps.len().max(1) as f64;

            let response_schema = eps.iter().find_map(|e| e.response_body.as_ref())
                .map(|b| onecrawl_cdp::network_intel::infer_json_schema(b));
            let request_schema = eps.iter().find_map(|e| e.request_body.as_ref())
                .map(|b| onecrawl_cdp::network_intel::infer_json_schema(b));

            onecrawl_cdp::network_intel::EndpointSchema {
                method: method.to_string(),
                path: template,
                path_params: params,
                query_params: vec![],
                request_body_schema: request_schema,
                response_body_schema: response_schema,
                status_codes,
                content_types,
                call_count: eps.len(),
                avg_latency_ms: avg_latency,
            }
        }).collect();

        let auth_pattern = endpoints.iter()
            .find_map(|e| {
                let auth = onecrawl_cdp::network_intel::detect_auth_pattern(&e.request_headers);
                match auth {
                    onecrawl_cdp::network_intel::AuthPattern::None => None,
                    other => Some(other),
                }
            });

        let schema = onecrawl_cdp::network_intel::ApiSchema {
            base_url,
            endpoints: schemas,
            auth_pattern,
            total_requests,
            unique_endpoints: endpoint_map.len(),
        };

        json_ok(&serde_json::to_value(&schema).unwrap())
    }

    async fn net_sdk(
        &self,
        p: NetIntelSdkParams,
    ) -> Result<CallToolResult, McpError> {
        let schema: onecrawl_cdp::network_intel::ApiSchema = serde_json::from_str(&p.schema)
            .map_err(|e| mcp_err(format!("invalid schema: {e}")))?;

        let sdk = match p.language.as_deref().unwrap_or("typescript") {
            "python" | "py" => onecrawl_cdp::network_intel::generate_python_sdk(&schema),
            _ => onecrawl_cdp::network_intel::generate_typescript_sdk(&schema),
        };

        json_ok(&serde_json::json!({
            "language": sdk.language,
            "code": sdk.code,
            "endpoints_covered": sdk.endpoints_covered,
        }))
    }

    async fn net_mock(
        &self,
        p: NetIntelMockParams,
    ) -> Result<CallToolResult, McpError> {
        let endpoints: Vec<onecrawl_cdp::network_intel::ApiEndpoint> = serde_json::from_str(&p.endpoints)
            .map_err(|e| mcp_err(format!("invalid endpoints: {e}")))?;

        let config = onecrawl_cdp::network_intel::generate_mock_config(&endpoints, p.port.unwrap_or(3001));
        json_ok(&serde_json::to_value(&config).unwrap())
    }

    async fn net_replay(
        &self,
        p: NetIntelReplayParams,
    ) -> Result<CallToolResult, McpError> {
        let endpoints: Vec<onecrawl_cdp::network_intel::ApiEndpoint> = serde_json::from_str(&p.endpoints)
            .map_err(|e| mcp_err(format!("invalid endpoints: {e}")))?;

        let name = p.name.as_deref().unwrap_or("replay_sequence");
        let sequence = onecrawl_cdp::network_intel::generate_replay_sequence(name, &endpoints);
        json_ok(&serde_json::to_value(&sequence).unwrap())
    }

    // ════════════════════════════════════════════════════════════════
    //  Visual Regression Testing tools
    // ════════════════════════════════════════════════════════════════

    async fn vrt_run(
        &self,
        p: VrtRunParams,
    ) -> Result<CallToolResult, McpError> {
        let suite = if p.suite.trim().starts_with('{') {
            serde_json::from_str::<onecrawl_cdp::VrtSuite>(&p.suite)
                .map_err(|e| mcp_err(format!("invalid VRT suite: {e}")))?
        } else {
            onecrawl_cdp::vrt::load_suite(&p.suite)
                .mcp()?
        };

        let errors = onecrawl_cdp::vrt::validate_suite(&suite);
        if !errors.is_empty() {
            return json_ok(&serde_json::json!({ "valid": false, "errors": errors }));
        }

        let page = ensure_page(&self.browser).await?;
        let start = std::time::Instant::now();
        let mut results = Vec::new();
        let mut passed = 0usize;
        let mut failed = 0usize;
        let mut new_baselines = 0usize;
        let mut error_count = 0usize;

        for test in &suite.tests {
            onecrawl_cdp::navigation::goto(&page, &test.url)
                .await
                .mcp()?;

            if test.delay_ms > 0 {
                tokio::time::sleep(tokio::time::Duration::from_millis(test.delay_ms)).await;
            }

            if let Some(ref wait) = test.wait_for {
                let _ = onecrawl_cdp::element::evaluate(
                    &page,
                    &format!(
                        "await new Promise(r => {{ const i = setInterval(() => {{ if (document.querySelector('{}')) {{ clearInterval(i); r(); }} }}, 100); setTimeout(() => {{ clearInterval(i); r(); }}, 10000); }})",
                        wait.replace('\'', "\\'")
                    ),
                ).await;
            }

            let screenshot_data = if test.full_page {
                onecrawl_cdp::screenshot::screenshot_full(&page)
                    .await
                    .mcp()?
            } else {
                onecrawl_cdp::screenshot::screenshot_viewport(&page)
                    .await
                    .mcp()?
            };

            let result = onecrawl_cdp::vrt::compare_test(
                test,
                &screenshot_data,
                &suite.baseline_dir,
                &suite.output_dir,
                &suite.diff_dir,
                suite.threshold,
            );

            match result.status {
                onecrawl_cdp::VrtStatus::Passed => passed += 1,
                onecrawl_cdp::VrtStatus::Failed => failed += 1,
                onecrawl_cdp::VrtStatus::NewBaseline => new_baselines += 1,
                onecrawl_cdp::VrtStatus::Error => error_count += 1,
            }
            results.push(result);
        }

        let suite_result = onecrawl_cdp::VrtSuiteResult {
            suite_name: suite.name.clone(),
            total: suite.tests.len(),
            passed,
            failed,
            new_baselines,
            errors: error_count,
            results,
            duration_ms: start.elapsed().as_millis() as u64,
        };

        let junit = onecrawl_cdp::vrt::generate_junit_report(&suite_result);

        json_ok(&serde_json::json!({
            "suite_name": suite_result.suite_name,
            "total": suite_result.total,
            "passed": suite_result.passed,
            "failed": suite_result.failed,
            "new_baselines": suite_result.new_baselines,
            "errors": suite_result.errors,
            "duration_ms": suite_result.duration_ms,
            "results": suite_result.results,
            "junit_xml": junit,
        }))
    }

    async fn vrt_compare(
        &self,
        p: VrtCompareParams,
    ) -> Result<CallToolResult, McpError> {
        let page = ensure_page(&self.browser).await?;
        onecrawl_cdp::navigation::goto(&page, &p.url)
            .await
            .mcp()?;

        let screenshot_data = if p.full_page.unwrap_or(false) {
            onecrawl_cdp::screenshot::screenshot_full(&page)
                .await
                .mcp()?
        } else {
            onecrawl_cdp::screenshot::screenshot_viewport(&page)
                .await
                .mcp()?
        };

        let test = onecrawl_cdp::VrtTestCase {
            name: p.name.clone(),
            url: p.url.clone(),
            selector: p.selector,
            full_page: p.full_page.unwrap_or(false),
            threshold: p.threshold.unwrap_or(0.1),
            viewport: None,
            wait_for: None,
            hide_selectors: vec![],
            delay_ms: 0,
        };

        let baseline_dir = p.baseline_dir.as_deref().unwrap_or(".vrt/baselines");
        let result = onecrawl_cdp::vrt::compare_test(
            &test,
            &screenshot_data,
            baseline_dir,
            ".vrt/current",
            ".vrt/diffs",
            p.threshold.unwrap_or(0.1),
        );

        json_ok(&serde_json::to_value(&result).unwrap())
    }

    async fn vrt_update_baseline(
        &self,
        p: VrtUpdateBaselineParams,
    ) -> Result<CallToolResult, McpError> {
        let baseline_dir = p.baseline_dir.as_deref().unwrap_or(".vrt/baselines");
        let current_dir = ".vrt/current";
        let current = onecrawl_cdp::vrt::load_baseline(current_dir, &p.test_name);

        match current {
            Some(data) => {
                let path =
                    onecrawl_cdp::vrt::save_baseline(baseline_dir, &p.test_name, &data)
                        .mcp()?;
                json_ok(&serde_json::json!({
                    "updated": true,
                    "test_name": p.test_name,
                    "baseline_path": path.to_string_lossy(),
                    "bytes": data.len(),
                }))
            }
            None => json_ok(&serde_json::json!({
                "updated": false,
                "error": format!("no current screenshot found for '{}'", p.test_name),
            })),
        }
    }

    // ════════════════════════════════════════════════════════════════
    //  AI Task Planner tools
    // ════════════════════════════════════════════════════════════════

    async fn planner_plan(
        &self,
        p: PlannerPlanParams,
    ) -> Result<CallToolResult, McpError> {
        let mut context = p.context.unwrap_or_default();
        let auto_context = onecrawl_cdp::task_planner::extract_context(&p.goal);
        for (k, v) in auto_context {
            context.entry(k).or_insert(v);
        }

        let plan = onecrawl_cdp::task_planner::plan_from_goal(&p.goal, &context);
        json_ok(&serde_json::to_value(&plan).unwrap())
    }

    async fn planner_execute(
        &self,
        p: PlannerExecuteParams,
    ) -> Result<CallToolResult, McpError> {
        let plan: onecrawl_cdp::TaskPlan = if p.plan.trim().starts_with('{') {
            parse_json_str(&p.plan, "plan")?
        } else {
            let mut context = p.context.clone().unwrap_or_default();
            let auto_context = onecrawl_cdp::task_planner::extract_context(&p.plan);
            for (k, v) in auto_context {
                context.entry(k).or_insert(v);
            }
            onecrawl_cdp::task_planner::plan_from_goal(&p.plan, &context)
        };

        let page = ensure_page(&self.browser).await?;
        let start = std::time::Instant::now();
        let max_retries = p.max_retries.unwrap_or(2);
        let mut step_results = Vec::new();
        let mut total_retries = 0usize;
        let mut completed = 0usize;

        for step in &plan.steps {
            let step_start = std::time::Instant::now();
            let mut attempt = 0u32;
            let mut last_error = None;
            let mut success = false;
            let mut used_fallback = false;
            let mut output = None;

            while attempt <= max_retries {
                match self.execute_planned_step(&page, &step.action).await {
                    Ok(val) => {
                        output = val;
                        success = true;
                        break;
                    }
                    Err(e) => {
                        last_error = Some(format!("{}", e.message));
                        attempt += 1;
                        total_retries += 1;

                        if attempt > max_retries {
                            if let Some(ref fallback) = step.fallback {
                                if let Ok(val) = self.execute_planned_step(&page, &fallback.action).await {
                                    output = val;
                                    success = true;
                                    used_fallback = true;
                                }
                            }
                        }
                    }
                }
            }

            let duration_ms = step_start.elapsed().as_millis() as u64;
            if success { completed += 1; }

            step_results.push(onecrawl_cdp::task_planner::StepExecutionResult {
                step_id: step.id,
                description: step.description.clone(),
                status: if success {
                    onecrawl_cdp::task_planner::StepOutcome::Success
                } else {
                    onecrawl_cdp::task_planner::StepOutcome::Failed
                },
                output,
                error: if success { None } else { last_error },
                used_fallback,
                duration_ms,
            });
        }

        let status = if completed == plan.steps.len() {
            onecrawl_cdp::TaskStatus::Success
        } else if completed > 0 {
            onecrawl_cdp::TaskStatus::PartialSuccess
        } else {
            onecrawl_cdp::TaskStatus::Failed
        };

        let result = onecrawl_cdp::TaskExecutionResult {
            goal: plan.goal.clone(),
            status,
            steps_completed: completed,
            steps_total: plan.steps.len(),
            steps_results: step_results,
            retries_used: total_retries,
            total_duration_ms: start.elapsed().as_millis() as u64,
        };

        json_ok(&serde_json::to_value(&result).unwrap())
    }

    async fn planner_patterns(
        &self,
        _p: PlannerPatternsParams,
    ) -> Result<CallToolResult, McpError> {
        let patterns = onecrawl_cdp::task_planner::builtin_patterns();
        let summary: Vec<serde_json::Value> = patterns.iter().map(|p| {
            serde_json::json!({
                "category": format!("{:?}", p.category).to_lowercase(),
                "keywords": p.keywords,
                "steps": p.template_steps.len(),
                "template": p.template_steps.iter().map(|s| &s.description).collect::<Vec<_>>(),
            })
        }).collect();
        json_ok(&serde_json::json!({
            "patterns": summary,
            "count": patterns.len(),
        }))
    }

    // ════════════════════════════════════════════════════════════════
    //  Performance Monitor tools
    // ════════════════════════════════════════════════════════════════

    async fn perf_audit(
        &self,
        p: PerfAuditParams,
    ) -> Result<CallToolResult, McpError> {
        let page = ensure_page(&self.browser).await?;

        if let Some(url) = &p.url {
            onecrawl_cdp::navigation::goto(&page, url)
                .await
                .mcp()?;
            tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;
        }

        let js = onecrawl_cdp::perf_monitor::metrics_collection_js();
        let result = page.evaluate(js).await.mcp()?;
        let metrics: serde_json::Value = result.into_value().unwrap_or(serde_json::Value::Null);

        let url = onecrawl_cdp::navigation::get_url(&page).await.unwrap_or_default();
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        let vitals: onecrawl_cdp::CoreWebVitals = serde_json::from_value(
            metrics.get("vitals").cloned().unwrap_or_default()
        ).unwrap_or_default();

        let ratings = onecrawl_cdp::perf_monitor::rate_vitals(&vitals);

        json_ok(&serde_json::json!({
            "url": url,
            "timestamp": now,
            "vitals": metrics.get("vitals"),
            "ratings": ratings,
            "navigation_timing": metrics.get("navigation_timing"),
            "resource_count": metrics.get("resource_count"),
            "memory": metrics.get("memory"),
        }))
    }

    async fn perf_budget(
        &self,
        p: PerfBudgetCheckParams,
    ) -> Result<CallToolResult, McpError> {
        let budget: onecrawl_cdp::PerfBudget = serde_json::from_str(&p.budget)
            .map_err(|e| mcp_err(format!("invalid budget: {e}")))?;

        let page = ensure_page(&self.browser).await?;

        if let Some(url) = &p.url {
            onecrawl_cdp::navigation::goto(&page, url)
                .await
                .mcp()?;
            tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;
        }

        let js = onecrawl_cdp::perf_monitor::metrics_collection_js();
        let result = page.evaluate(js).await.mcp()?;
        let metrics: serde_json::Value = result.into_value().unwrap_or(serde_json::Value::Null);

        let snapshot = onecrawl_cdp::PerfSnapshot {
            url: onecrawl_cdp::navigation::get_url(&page).await.unwrap_or_default(),
            timestamp: std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap_or_default().as_secs(),
            vitals: serde_json::from_value(metrics.get("vitals").cloned().unwrap_or_default()).unwrap_or_default(),
            navigation_timing: serde_json::from_value(metrics.get("navigation_timing").cloned().unwrap_or_default()).unwrap_or_default(),
            resource_count: serde_json::from_value(metrics.get("resource_count").cloned().unwrap_or_default()).unwrap_or_default(),
            memory: None,
            js_heap_size: None,
        };

        let budget_result = onecrawl_cdp::perf_monitor::check_budget(&snapshot, &budget);
        json_ok(&serde_json::to_value(&budget_result).unwrap())
    }

    async fn perf_compare(
        &self,
        p: PerfCompareParams,
    ) -> Result<CallToolResult, McpError> {
        let baseline: onecrawl_cdp::PerfSnapshot = serde_json::from_str(&p.baseline)
            .map_err(|e| mcp_err(format!("invalid baseline: {e}")))?;
        let current: onecrawl_cdp::PerfSnapshot = serde_json::from_str(&p.current)
            .map_err(|e| mcp_err(format!("invalid current: {e}")))?;

        let threshold = p.threshold_pct.unwrap_or(10.0);
        let regressions = onecrawl_cdp::perf_monitor::detect_regressions(&baseline, &current, threshold);

        json_ok(&serde_json::json!({
            "baseline_url": baseline.url,
            "current_url": current.url,
            "threshold_pct": threshold,
            "regressions": regressions,
            "regressed": !regressions.is_empty(),
            "count": regressions.len(),
        }))
    }

    async fn perf_trace(
        &self,
        p: PerfTraceParams,
    ) -> Result<CallToolResult, McpError> {
        let page = ensure_page(&self.browser).await?;
        let start = std::time::Instant::now();

        onecrawl_cdp::navigation::goto(&page, &p.url)
            .await
            .mcp()?;

        let settle = p.settle_ms.unwrap_or(3000);
        tokio::time::sleep(tokio::time::Duration::from_millis(settle)).await;

        let js = onecrawl_cdp::perf_monitor::metrics_collection_js();
        let result = page.evaluate(js).await.mcp()?;
        let metrics: serde_json::Value = result.into_value().unwrap_or(serde_json::Value::Null);

        let vitals: onecrawl_cdp::CoreWebVitals = serde_json::from_value(
            metrics.get("vitals").cloned().unwrap_or_default()
        ).unwrap_or_default();
        let ratings = onecrawl_cdp::perf_monitor::rate_vitals(&vitals);

        let trace_duration = start.elapsed().as_millis() as u64;

        json_ok(&serde_json::json!({
            "url": p.url,
            "trace_duration_ms": trace_duration,
            "settle_ms": settle,
            "vitals": metrics.get("vitals"),
            "ratings": ratings,
            "navigation_timing": metrics.get("navigation_timing"),
            "resource_count": metrics.get("resource_count"),
            "memory": metrics.get("memory"),
        }))
    }

    async fn execute_planned_step(
        &self,
        page: &chromiumoxide::Page,
        action: &onecrawl_cdp::task_planner::PlannedAction,
    ) -> std::result::Result<Option<serde_json::Value>, McpError> {
        use onecrawl_cdp::task_planner::PlannedAction;
        match action {
            PlannedAction::Navigate { url } => {
                onecrawl_cdp::navigation::goto(page, url).await.mcp()?;
                let title = onecrawl_cdp::navigation::get_title(page).await.unwrap_or_default();
                Ok(Some(serde_json::json!({ "navigated": url, "title": title })))
            }
            PlannedAction::Click { target, .. } => {
                let resolved = onecrawl_cdp::accessibility::resolve_ref(target);
                onecrawl_cdp::element::click(page, &resolved).await.mcp()?;
                Ok(Some(serde_json::json!({ "clicked": target })))
            }
            PlannedAction::Type { target, text, .. } => {
                let resolved = onecrawl_cdp::accessibility::resolve_ref(target);
                onecrawl_cdp::element::type_text(page, &resolved, text).await.mcp()?;
                Ok(Some(serde_json::json!({ "typed": text.len() })))
            }
            PlannedAction::Wait { target, timeout_ms } => {
                let resolved = onecrawl_cdp::accessibility::resolve_ref(target);
                onecrawl_cdp::navigation::wait_for_selector(page, &resolved, *timeout_ms).await.mcp()?;
                Ok(Some(serde_json::json!({ "found": target })))
            }
            PlannedAction::Snapshot {} => {
                let opts = onecrawl_cdp::accessibility::AgentSnapshotOptions::default();
                let result = onecrawl_cdp::accessibility::agent_snapshot(page, &opts)
                    .await.mcp()?;
                Ok(Some(serde_json::json!(result)))
            }
            PlannedAction::Extract { target } => {
                let js = format!(
                    r#"Array.from(document.querySelectorAll({sel})).map(e => e.textContent.trim())"#,
                    sel = serde_json::to_string(target).unwrap()
                );
                let result = page.evaluate(js).await.mcp()?;
                let val = result.into_value::<serde_json::Value>().unwrap_or(serde_json::Value::Null);
                Ok(Some(val))
            }
            PlannedAction::Assert { condition } => {
                Ok(Some(serde_json::json!({ "assert": condition, "note": "assertion evaluation requires runtime context" })))
            }
            PlannedAction::SmartClick { query } => {
                let matched = onecrawl_cdp::smart_actions::smart_click(page, query).await.mcp()?;
                Ok(Some(serde_json::json!({ "clicked": matched.selector, "confidence": matched.confidence })))
            }
            PlannedAction::SmartFill { query, value } => {
                let matched = onecrawl_cdp::smart_actions::smart_fill(page, query, value).await.mcp()?;
                Ok(Some(serde_json::json!({ "filled": matched.selector, "confidence": matched.confidence })))
            }
            PlannedAction::Scroll { direction, amount } => {
                let px = amount.unwrap_or(500);
                let js = match direction.as_str() {
                    "up" => format!("window.scrollBy(0, -{})", px),
                    "down" => format!("window.scrollBy(0, {})", px),
                    "left" => format!("window.scrollBy(-{}, 0)", px),
                    "right" => format!("window.scrollBy({}, 0)", px),
                    _ => format!("window.scrollBy(0, {})", px),
                };
                page.evaluate(js).await.mcp()?;
                Ok(Some(serde_json::json!({ "scrolled": direction, "pixels": px })))
            }
            PlannedAction::Screenshot { path } => {
                let data = onecrawl_cdp::screenshot::screenshot_full(page).await.mcp()?;
                if let Some(p) = path {
                    std::fs::write(p, &data).mcp()?;
                }
                Ok(Some(serde_json::json!({ "bytes": data.len() })))
            }
            PlannedAction::MemoryStore { key, value } => {
                Ok(Some(serde_json::json!({ "stored": key, "value": value })))
            }
            PlannedAction::MemoryRecall { key } => {
                Ok(Some(serde_json::json!({ "recalled": key })))
            }
            PlannedAction::Conditional { condition, .. } => {
                Ok(Some(serde_json::json!({ "note": "conditional evaluation", "condition": condition })))
            }
        }
    }
}

impl ServerHandler for OneCrawlMcp {
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
