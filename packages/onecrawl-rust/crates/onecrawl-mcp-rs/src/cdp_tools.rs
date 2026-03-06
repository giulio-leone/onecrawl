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
    pub tabs: Vec<chromiumoxide::Page>,
    pub active_tab: usize,
    pub snapshots: HashMap<String, onecrawl_cdp::DomSnapshot>,
    pub rate_limiter: Option<onecrawl_cdp::RateLimitState>,
    pub retry_queue: Option<onecrawl_cdp::RetryQueue>,
    pub safety: Option<onecrawl_cdp::SafetyState>,
    pub recording: Option<onecrawl_cdp::RecordingState>,
    pub ios_client: Option<onecrawl_cdp::ios::IosClient>,
    pub pool: onecrawl_cdp::BrowserPool,
    pub memory: Option<onecrawl_cdp::AgentMemory>,
    pub mutation_buffer: Vec<serde_json::Value>,
    pub observing_mutations: bool,
    // Network interception
    pub intercept_rules: Vec<InterceptRule>,
    pub intercepting: bool,
    // Console & dialog capture
    pub console_messages: Vec<ConsoleMessage>,
    pub capturing_console: bool,
    pub last_dialog: Option<DialogInfo>,
    pub dialog_auto_response: Option<DialogAutoResponse>,
    pub page_errors: Vec<PageError>,
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

// ──────────────── Accessibility Snapshot params ─────────────────

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct AgentSnapshotParams {
    #[schemars(description = "Only include interactive elements (buttons, links, inputs)")]
    pub interactive_only: Option<bool>,
    #[schemars(description = "Include cursor-interactive elements (cursor:pointer, onclick, tabindex)")]
    pub cursor: Option<bool>,
    #[schemars(description = "Remove empty structural elements for minimal output")]
    pub compact: Option<bool>,
    #[schemars(description = "Max DOM depth to include")]
    pub depth: Option<usize>,
    #[schemars(description = "CSS selector to scope snapshot to a subtree")]
    pub selector: Option<String>,
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

// ──────────────── Snapshot Diff params ─────────────────

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct SnapshotDiffParams {
    #[schemars(description = "Accessibility snapshot text before (from navigation.snapshot)")]
    pub before: String,
    #[schemars(description = "Accessibility snapshot text after (from navigation.snapshot)")]
    pub after: String,
}

// ──────────────── Agent tools params ─────────────────

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct ChainCommand {
    #[schemars(description = "Tool name to execute (e.g. 'navigation.click')")]
    pub tool: String,
    #[schemars(description = "Arguments as JSON object")]
    pub args: serde_json::Value,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct ExecuteChainParams {
    #[schemars(description = "List of commands to execute in sequence")]
    pub commands: Vec<ChainCommand>,
    #[schemars(description = "Stop on first error (default: true)")]
    pub stop_on_error: Option<bool>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct ElementScreenshotParams {
    #[schemars(description = "CSS selector or @ref (e.g. @e1) of the element to screenshot")]
    pub selector: String,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct ApiCaptureStartParams {
    // no params needed — injects the interceptor
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct ApiCaptureSummaryParams {
    #[schemars(description = "Clear the captured log after reading (default: false)")]
    pub clear: Option<bool>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct IframeSnapshotParams {
    #[schemars(description = "Zero-based index of the iframe to snapshot")]
    pub index: usize,
    #[schemars(description = "Only include interactive elements")]
    pub interactive_only: Option<bool>,
    #[schemars(description = "Remove empty structural elements for minimal output")]
    pub compact: Option<bool>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct IframeListParams {
    // no params needed
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct RemoteCdpParams {
    #[schemars(description = "WebSocket URL of the remote CDP endpoint (e.g. ws://127.0.0.1:9222/devtools/browser/...)")]
    pub ws_url: String,
    #[schemars(description = "Optional HTTP headers for the WebSocket handshake")]
    pub headers: Option<HashMap<String, String>>,
}

// ──────────────── Safety Policy params ─────────────────

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct SafetyPolicySetParams {
    #[schemars(description = "Allowed domains (if empty, all domains allowed)")]
    pub allowed_domains: Option<Vec<String>>,
    #[schemars(description = "Blocked domains")]
    pub blocked_domains: Option<Vec<String>>,
    #[schemars(description = "Blocked URL patterns (glob-style with * wildcards)")]
    pub blocked_url_patterns: Option<Vec<String>>,
    #[schemars(description = "Maximum actions per session (0 = unlimited)")]
    pub max_actions: Option<usize>,
    #[schemars(description = "Require confirmation for form submissions")]
    pub confirm_form_submit: Option<bool>,
    #[schemars(description = "Require confirmation for file uploads")]
    pub confirm_file_upload: Option<bool>,
    #[schemars(description = "Blocked commands")]
    pub blocked_commands: Option<Vec<String>>,
    #[schemars(description = "Allowed commands (if empty, all non-blocked allowed)")]
    pub allowed_commands: Option<Vec<String>>,
    #[schemars(description = "Rate limit: max actions per minute (0 = unlimited)")]
    pub rate_limit_per_minute: Option<usize>,
    #[schemars(description = "Path to a JSON policy file to load (overrides other fields)")]
    pub policy_file: Option<String>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct SafetyStatusParams {
    // no params needed
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct SkillsListParams {
    // no params needed
}

// ──────────────── iOS / Mobile Safari params ─────────────────

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct IosDevicesParams {
    // no params needed
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct IosConnectParams {
    #[schemars(description = "WebDriverAgent URL (default: http://localhost:8100)")]
    pub wda_url: Option<String>,
    #[schemars(description = "Device UDID (auto-detect if omitted)")]
    pub udid: Option<String>,
    #[schemars(description = "Bundle ID to automate (default: com.apple.mobilesafari)")]
    pub bundle_id: Option<String>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct IosNavigateParams {
    #[schemars(description = "URL to navigate to in Mobile Safari")]
    pub url: String,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct IosTapParams {
    #[schemars(description = "X coordinate")]
    pub x: f64,
    #[schemars(description = "Y coordinate")]
    pub y: f64,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct IosScreenshotParams {
    // no params needed — returns base64 image
}

// ──────────────── Computer Use Protocol params ─────────────────

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct ComputerUseActionParams {
    #[schemars(
        description = "Action to perform. JSON object with a \"type\" field. Types: click (with x,y or selector or ref), type (text), key (key name), scroll (x, y, delta_x, delta_y), navigate (url), wait (ms), screenshot, observe, evaluate (expression), fill (selector, value), select (selector, value), drag (from_x, from_y, to_x, to_y), done (result), fail (reason)"
    )]
    pub action: serde_json::Value,
    #[schemars(description = "Include screenshot in observation (default: false)")]
    pub include_screenshot: Option<bool>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct ComputerUseObserveParams {
    #[schemars(description = "Include base64 screenshot in observation")]
    pub include_screenshot: Option<bool>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct ComputerUseBatchParams {
    #[schemars(
        description = "List of actions to execute in sequence. Each action is a JSON object with a \"type\" field."
    )]
    pub actions: Vec<serde_json::Value>,
    #[schemars(description = "Include screenshots between actions (default: false)")]
    pub include_screenshots: Option<bool>,
    #[schemars(description = "Stop on first error (default: true)")]
    pub stop_on_error: Option<bool>,
}

// ──────────────── Browser Pool params ─────────────────

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct PoolListParams {}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct PoolStatusParams {}

// ──────────────── Smart Actions params ─────────────────

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct SmartFindParams {
    #[schemars(description = "Fuzzy text, CSS selector, or element description to search for")]
    pub query: String,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct SmartClickParams {
    #[schemars(description = "Fuzzy text, CSS selector, or element description to click")]
    pub query: String,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct SmartFillParams {
    #[schemars(description = "Fuzzy text, CSS selector, or element description of the input to fill")]
    pub query: String,
    #[schemars(description = "Value to type into the matched input")]
    pub value: String,
}

// ──────────────── Multi-Tab params ─────────────────

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct NewTabParams {
    #[schemars(description = "URL to navigate to in the new tab (defaults to about:blank)")]
    pub url: Option<String>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct SwitchTabParams {
    #[schemars(description = "Tab index (0-based)")]
    pub index: usize,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct CloseTabParams {
    #[schemars(description = "Tab index to close (0-based). Defaults to active tab.")]
    pub index: Option<usize>,
}

// ──────────────── DOM Events params ─────────────────

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct ObserveMutationsParams {
    #[schemars(description = "CSS selector of element to observe (defaults to body)")]
    pub selector: Option<String>,
    #[schemars(description = "Observe child list changes")]
    pub child_list: Option<bool>,
    #[schemars(description = "Observe attribute changes")]
    pub attributes: Option<bool>,
    #[schemars(description = "Observe character data changes")]
    pub character_data: Option<bool>,
    #[schemars(description = "Observe entire subtree")]
    pub subtree: Option<bool>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct WaitForEventParams {
    #[schemars(description = "DOM event to wait for (e.g. click, load, DOMContentLoaded, hashchange)")]
    pub event: String,
    #[schemars(description = "CSS selector to scope the listener (defaults to document)")]
    pub selector: Option<String>,
    #[schemars(description = "Timeout in milliseconds (default 30000)")]
    pub timeout: Option<u64>,
}

// ──────────────── Cookie & Storage params ─────────────────

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct CookiesGetParams {
    #[schemars(description = "Filter cookies by domain")]
    pub domain: Option<String>,
    #[schemars(description = "Filter cookies by name")]
    pub name: Option<String>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct CookieSetParams {
    #[schemars(description = "Cookie name")]
    pub name: String,
    #[schemars(description = "Cookie value")]
    pub value: String,
    #[schemars(description = "Cookie domain")]
    pub domain: String,
    #[schemars(description = "Cookie path (defaults to /)")]
    pub path: Option<String>,
    #[schemars(description = "Secure flag")]
    pub secure: Option<bool>,
    #[schemars(description = "HttpOnly flag")]
    pub http_only: Option<bool>,
    #[schemars(description = "SameSite attribute (Strict, Lax, None)")]
    pub same_site: Option<String>,
    #[schemars(description = "Expiry as Unix timestamp")]
    pub expires: Option<f64>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct CookiesClearParams {
    #[schemars(description = "Clear only cookies matching this domain (omit for all)")]
    pub domain: Option<String>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct StorageGetParams {
    #[schemars(description = "Storage key to retrieve")]
    pub key: String,
    #[schemars(description = "Storage type: 'local' or 'session' (defaults to local)")]
    pub storage_type: Option<String>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct StorageSetParams {
    #[schemars(description = "Storage key")]
    pub key: String,
    #[schemars(description = "Value to store")]
    pub value: String,
    #[schemars(description = "Storage type: 'local' or 'session' (defaults to local)")]
    pub storage_type: Option<String>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct SessionExportParams {
    #[schemars(description = "Include cookies in export")]
    pub cookies: Option<bool>,
    #[schemars(description = "Include localStorage in export")]
    pub local_storage: Option<bool>,
    #[schemars(description = "Include sessionStorage in export")]
    pub session_storage: Option<bool>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct SessionImportParams {
    #[schemars(description = "Session state JSON (from export_session)")]
    pub state: String,
}

// ──────────────── Network Interception types & params ─────────────────

#[derive(Debug, Clone, serde::Serialize, Deserialize)]
pub struct InterceptRule {
    pub id: String,
    pub url_pattern: String,
    pub method: Option<String>,
    pub response_status: u16,
    pub response_headers: HashMap<String, String>,
    pub response_body: String,
}

impl Default for InterceptRule {
    fn default() -> Self {
        Self {
            id: String::new(),
            url_pattern: String::new(),
            method: None,
            response_status: 200,
            response_headers: HashMap::new(),
            response_body: String::new(),
        }
    }
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct InterceptEnableParams {
    #[schemars(description = "URL patterns to intercept (glob syntax, e.g. '**/api/*')")]
    pub patterns: Option<Vec<String>>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct InterceptAddRuleParams {
    #[schemars(description = "URL pattern to match (glob syntax)")]
    pub url_pattern: String,
    #[schemars(description = "HTTP method to match (GET, POST, etc.) — omit to match all")]
    pub method: Option<String>,
    #[schemars(description = "Mock response HTTP status code")]
    pub status: Option<u16>,
    #[schemars(description = "Mock response headers as JSON object")]
    pub headers: Option<HashMap<String, String>>,
    #[schemars(description = "Mock response body")]
    pub body: Option<String>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct InterceptRemoveRuleParams {
    #[schemars(description = "Rule ID to remove (from intercept_add_rule response)")]
    pub rule_id: String,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct BlockRequestsParams {
    #[schemars(description = "URL patterns to block (glob syntax)")]
    pub patterns: Vec<String>,
    #[schemars(description = "Resource types to block: 'image', 'stylesheet', 'script', 'font', 'media'")]
    pub resource_types: Option<Vec<String>>,
}

// ──────────────── Console, Dialog & Error types & params ─────────────────

#[derive(Debug, Clone, serde::Serialize, Deserialize)]
pub struct ConsoleMessage {
    pub level: String,
    pub text: String,
    pub url: Option<String>,
    pub line: Option<u32>,
    pub timestamp_ms: u64,
}

#[derive(Debug, Clone, serde::Serialize, Deserialize)]
pub struct DialogInfo {
    pub dialog_type: String,
    pub message: String,
    pub default_prompt: Option<String>,
    pub was_handled: bool,
    pub response: Option<String>,
}

#[derive(Debug, Clone, serde::Serialize, Deserialize)]
pub struct DialogAutoResponse {
    pub accept: bool,
    pub prompt_text: Option<String>,
}

#[derive(Debug, Clone, serde::Serialize, Deserialize)]
pub struct PageError {
    pub message: String,
    pub url: Option<String>,
    pub line: Option<u32>,
    pub column: Option<u32>,
    pub timestamp_ms: u64,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct DialogHandleParams {
    #[schemars(description = "Accept (true) or dismiss (false) dialogs")]
    pub accept: bool,
    #[schemars(description = "Text to enter for prompt() dialogs")]
    pub prompt_text: Option<String>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct ConsoleFilterParams {
    #[schemars(description = "Filter by level: 'log', 'warn', 'error', 'info'")]
    pub level: Option<String>,
    #[schemars(description = "Max number of messages to return")]
    pub limit: Option<usize>,
}

// ──────────────── Device Emulation params ─────────────────

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct EmulateDeviceParams {
    #[schemars(description = "Preset device: 'iphone-14', 'iphone-14-pro', 'pixel-7', 'ipad-air', 'galaxy-s23', or 'custom'")]
    pub device: Option<String>,
    #[schemars(description = "Custom viewport width (for device='custom')")]
    pub width: Option<u32>,
    #[schemars(description = "Custom viewport height (for device='custom')")]
    pub height: Option<u32>,
    #[schemars(description = "Custom user agent string")]
    pub user_agent: Option<String>,
    #[schemars(description = "Device scale factor (default: 1)")]
    pub device_scale_factor: Option<f64>,
    #[schemars(description = "Emulate touch events")]
    pub has_touch: Option<bool>,
    #[schemars(description = "Landscape orientation")]
    pub is_landscape: Option<bool>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct EmulateGeolocationParams {
    #[schemars(description = "Latitude")]
    pub latitude: f64,
    #[schemars(description = "Longitude")]
    pub longitude: f64,
    #[schemars(description = "Accuracy in meters (default: 1)")]
    pub accuracy: Option<f64>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct EmulateTimezoneParams {
    #[schemars(description = "Timezone ID (e.g. 'America/New_York', 'Europe/Rome', 'Asia/Tokyo')")]
    pub timezone_id: String,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct EmulateMediaParams {
    #[schemars(description = "prefers-color-scheme: 'light', 'dark', or 'no-preference'")]
    pub color_scheme: Option<String>,
    #[schemars(description = "prefers-reduced-motion: 'reduce' or 'no-preference'")]
    pub reduced_motion: Option<String>,
    #[schemars(description = "forced-colors: 'active' or 'none'")]
    pub forced_colors: Option<String>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct EmulateNetworkParams {
    #[schemars(description = "Preset: 'offline', '2g', 'slow-3g', '3g', '4g', 'wifi', or 'custom'")]
    pub preset: Option<String>,
    #[schemars(description = "Download throughput in bytes/sec (for preset='custom')")]
    pub download_throughput: Option<f64>,
    #[schemars(description = "Upload throughput in bytes/sec (for preset='custom')")]
    pub upload_throughput: Option<f64>,
    #[schemars(description = "Latency in ms (for preset='custom')")]
    pub latency: Option<f64>,
    #[schemars(description = "Simulate offline mode")]
    pub offline: Option<bool>,
}
