//! CDP tool parameter types and browser session management for MCP.

use rmcp::schemars;
use serde::Deserialize;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;

// ──────────────────────────── Browser state ────────────────────────────

/// Shared browser session — lazily initialised on first CDP tool call.
#[derive(Default)]
pub struct BrowserState {
    pub session: Option<onecrawl_cdp::BrowserSession>,
    pub page: Option<chromiumoxide::Page>,
    pub snapshots: HashMap<String, onecrawl_cdp::DomSnapshot>,
    pub rate_limiter: Option<onecrawl_cdp::RateLimitState>,
    pub retry_queue: Option<onecrawl_cdp::RetryQueue>,
}

pub type SharedBrowser = Arc<Mutex<BrowserState>>;

pub fn new_shared_browser() -> SharedBrowser {
    Arc::new(Mutex::new(BrowserState::default()))
}

// ──────────────── Navigation & Page Control params ─────────────────

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct NavigateParams {
    #[schemars(description = "URL to navigate to")]
    pub url: String,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct ClickParams {
    #[schemars(description = "CSS selector of element to click")]
    pub selector: String,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct TypeTextParams {
    #[schemars(description = "CSS selector of target input element")]
    pub selector: String,
    #[schemars(description = "Text to type")]
    pub text: String,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct ScreenshotParams {
    #[schemars(description = "CSS selector for element screenshot (omit for full page)")]
    pub selector: Option<String>,
    #[schemars(description = "If true, capture the full scrollable page")]
    pub full_page: Option<bool>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct WaitForSelectorParams {
    #[schemars(description = "CSS selector to wait for")]
    pub selector: String,
    #[schemars(description = "Timeout in milliseconds (default 30000)")]
    pub timeout_ms: Option<u64>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct EvaluateJsParams {
    #[schemars(description = "JavaScript code to evaluate in the page context")]
    pub js: String,
}

// ──────────────── Scraping & Extraction params ─────────────────

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct CssSelectorParams {
    #[schemars(description = "CSS selector to query (supports ::text, ::attr(name) pseudo-elements)")]
    pub selector: String,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct XPathParams {
    #[schemars(description = "XPath expression to evaluate")]
    pub expression: String,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct FindByTextParams {
    #[schemars(description = "Text content to search for")]
    pub text: String,
    #[schemars(description = "Optional HTML tag to constrain search (e.g. 'a', 'button')")]
    pub tag: Option<String>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct ExtractTextParams {
    #[schemars(description = "CSS selector (default 'body')")]
    pub selector: Option<String>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct ExtractHtmlParams {
    #[schemars(description = "CSS selector (default 'body')")]
    pub selector: Option<String>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct ExtractMarkdownParams {
    #[schemars(description = "CSS selector (default 'body')")]
    pub selector: Option<String>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct StreamExtractParams {
    #[schemars(description = "CSS selector for repeating item container")]
    pub item_selector: String,
    #[schemars(
        description = "Field extraction rules as JSON array: [{\"name\":\"title\",\"selector\":\"h2\",\"extract\":\"text\"}]"
    )]
    pub fields: String,
    #[schemars(description = "Optional pagination: {\"next_selector\":\".next\",\"max_pages\":5,\"delay_ms\":1000}")]
    pub pagination: Option<String>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct DetectFormsParams {
    // no params needed — operates on current page
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct FillFormParams {
    #[schemars(description = "CSS selector of the form element")]
    pub form_selector: String,
    #[schemars(description = "JSON object mapping field selectors to values, e.g. {\"#email\":\"a@b.com\"}")]
    pub data: String,
    #[schemars(description = "If true, submit the form after filling")]
    pub submit: Option<bool>,
}

// ──────────────── Crawling params ─────────────────

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct SpiderCrawlParams {
    #[schemars(description = "Starting URL(s) to crawl")]
    pub start_urls: Vec<String>,
    #[schemars(description = "Maximum link depth (default 2)")]
    pub max_depth: Option<usize>,
    #[schemars(description = "Maximum pages to visit (default 50)")]
    pub max_pages: Option<usize>,
    #[schemars(description = "Stay on the same domain only (default true)")]
    pub same_domain_only: Option<bool>,
    #[schemars(description = "URL patterns to include (regex)")]
    pub url_patterns: Option<Vec<String>>,
    #[schemars(description = "URL patterns to exclude (regex)")]
    pub exclude_patterns: Option<Vec<String>>,
    #[schemars(description = "Delay between requests in ms (default 500)")]
    pub delay_ms: Option<u64>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct CheckRobotsParams {
    #[schemars(description = "Base URL to fetch robots.txt from")]
    pub base_url: String,
    #[schemars(description = "Path to check (e.g. '/admin')")]
    pub path: Option<String>,
    #[schemars(description = "User-agent string (default '*')")]
    pub user_agent: Option<String>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct GenerateSitemapParams {
    #[schemars(description = "Base URL for the sitemap")]
    pub base_url: String,
    #[schemars(description = "URLs to include as JSON array: [{\"url\":\"...\",\"priority\":0.8}]")]
    pub entries: String,
    #[schemars(description = "Default change frequency (e.g. 'weekly')")]
    pub default_changefreq: Option<String>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct TakeSnapshotParams {
    #[schemars(description = "Label to identify this snapshot for later comparison")]
    pub label: String,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct CompareSnapshotsParams {
    #[schemars(description = "Label of the 'before' snapshot")]
    pub before: String,
    #[schemars(description = "Label of the 'after' snapshot")]
    pub after: String,
}

// ──────────────── Stealth & Anti-Detection params ─────────────────

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct InjectStealthParams {
    // no additional params needed
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct BotDetectionTestParams {
    // no additional params — runs tests on the current page
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct ApplyFingerprintParams {
    #[schemars(description = "Optional user-agent override")]
    pub user_agent: Option<String>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct BlockDomainsParams {
    #[schemars(description = "List of domains to block, OR a category name (e.g. 'ads', 'trackers')")]
    pub domains: Option<Vec<String>>,
    #[schemars(description = "Block an entire built-in category: 'ads', 'trackers', 'social'")]
    pub category: Option<String>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct DetectCaptchaParams {
    // no additional params needed
}

// ──────────────── Data Processing params ─────────────────

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct PipelineExecuteParams {
    #[schemars(description = "Pipeline name")]
    pub name: String,
    #[schemars(description = "Pipeline steps as JSON array (see docs for step types)")]
    pub steps: String,
    #[schemars(description = "Input data as a JSON array of objects with string values")]
    pub input: String,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct HttpGetParams {
    #[schemars(description = "URL to fetch")]
    pub url: String,
    #[schemars(description = "Optional headers as JSON object")]
    pub headers: Option<String>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct HttpPostParams {
    #[schemars(description = "URL to post to")]
    pub url: String,
    #[schemars(description = "Request body (string)")]
    pub body: String,
    #[schemars(description = "Optional headers as JSON object")]
    pub headers: Option<String>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct ExtractLinksParams {
    #[schemars(description = "Base URL for resolving relative links")]
    pub base_url: String,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct AnalyzeGraphParams {
    #[schemars(description = "Link edges as JSON array: [{\"source\":\"...\",\"target\":\"...\"}]")]
    pub edges: String,
}

// ──────────────── Automation params ─────────────────

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct RateLimitCheckParams {
    #[schemars(description = "Max requests per second (default 2.0)")]
    pub max_per_second: Option<f64>,
    #[schemars(description = "Max requests per minute (default 60.0)")]
    pub max_per_minute: Option<f64>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct RetryEnqueueParams {
    #[schemars(description = "URL to retry")]
    pub url: String,
    #[schemars(description = "Operation label (e.g. 'navigate', 'extract')")]
    pub operation: String,
    #[schemars(description = "Optional payload string")]
    pub payload: Option<String>,
}

// ──────────────── Passkey / WebAuthn params ─────────────────

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct PasskeyEnableParams {
    #[schemars(description = "Protocol: 'ctap2' or 'u2f' (default ctap2)")]
    pub protocol: Option<String>,
    #[schemars(description = "Transport: 'internal', 'usb', 'nfc', 'ble' (default internal)")]
    pub transport: Option<String>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct PasskeyAddParams {
    #[schemars(description = "Base64url-encoded credential ID")]
    pub credential_id: String,
    #[schemars(description = "Relying party domain (e.g. 'example.com')")]
    pub rp_id: String,
    #[schemars(description = "Optional base64url-encoded user handle")]
    pub user_handle: Option<String>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct PasskeyListParams {
    // no params needed
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct PasskeyLogParams {
    // no params needed
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct PasskeyDisableParams {
    // no params needed
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct PasskeyRemoveParams {
    #[schemars(description = "Credential ID to remove")]
    pub credential_id: String,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct PdfExportParams {
    #[schemars(description = "Print background graphics (default false)")]
    pub print_background: Option<bool>,
    #[schemars(description = "Paper format: 'A4', 'Letter', etc. (default 'A4')")]
    pub format: Option<String>,
    #[schemars(description = "Landscape orientation (default false)")]
    pub landscape: Option<bool>,
}
