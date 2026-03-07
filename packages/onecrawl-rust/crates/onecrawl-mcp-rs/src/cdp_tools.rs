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
    pub android_client: Option<onecrawl_cdp::android::AndroidClient>,
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
    // Page context (shared across tabs)
    pub page_context: HashMap<String, serde_json::Value>,
    // Error recovery history
    pub error_history: Vec<(String, String, String)>,
    // Task plans (agent task decomposition)
    pub task_plans: Vec<serde_json::Value>,
    // Self-healing selector cache
    pub selector_cache: HashMap<String, Vec<String>>,
    // Session checkpoints
    pub checkpoints: HashMap<String, serde_json::Value>,
    // Workflow variables
    pub workflow_variables: HashMap<String, serde_json::Value>,
    // Event-driven reaction system
    pub event_subscriptions: Vec<String>,
    pub event_buffer: Vec<serde_json::Value>,
    // Multi-browser fleet
    pub fleet_instances: Vec<(String, Option<chromiumoxide::Page>)>,
    pub fleet_name: Option<String>,
    // Auth sessions
    pub auth_sessions: HashMap<String, serde_json::Value>,
    pub auth_status: Option<String>,
    // Credential vault
    pub credentials: HashMap<String, serde_json::Value>,
    // Stealth mode (ON by default, disable with stealth_disabled = true)
    pub stealth_disabled: bool,
    pub stealth_applied: bool,
    // Browser mode
    pub headed: bool,
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

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct TlsApplyParams {
    /// TLS profile name: "chrome-win", "chrome-mac", "firefox-win", "safari-mac", "edge-win", "random", "detect"
    pub profile: Option<String>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct WebrtcBlockParams {
    /// Mode: "block" (disable WebRTC entirely) or "turn_only" (allow only TURN relay)
    pub mode: Option<String>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct BatterySpoofParams {
    /// Override charging status (default: true = plugged in)
    pub charging: Option<bool>,
    /// Override battery level 0.0-1.0 (default: 1.0)
    pub level: Option<f64>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct SensorBlockParams {
    /// Block specific sensors only. Default: all
    pub sensors: Option<Vec<String>>,
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

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct IosPinchParams {
    #[schemars(description = "X coordinate of pinch center")]
    pub x: f64,
    #[schemars(description = "Y coordinate of pinch center")]
    pub y: f64,
    #[schemars(description = "Scale factor (>1 zoom in, <1 zoom out)")]
    pub scale: f64,
    #[schemars(description = "Pinch velocity (default: 1.0)")]
    pub velocity: Option<f64>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct IosLongPressParams {
    #[schemars(description = "X coordinate")]
    pub x: f64,
    #[schemars(description = "Y coordinate")]
    pub y: f64,
    #[schemars(description = "Duration in milliseconds (default: 1000)")]
    pub duration_ms: Option<u64>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct IosDoubleTapParams {
    #[schemars(description = "X coordinate")]
    pub x: f64,
    #[schemars(description = "Y coordinate")]
    pub y: f64,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct IosOrientationParams {
    #[schemars(description = "Orientation to set (PORTRAIT/LANDSCAPE). Omit to get current.")]
    pub set: Option<String>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct IosScrollParams {
    #[schemars(description = "Locator strategy (e.g. 'accessibility id', 'class name')")]
    pub using: String,
    #[schemars(description = "Locator value")]
    pub value: String,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct IosScriptParams {
    #[schemars(description = "JavaScript code to execute in Safari")]
    pub script: String,
    #[schemars(description = "Arguments to pass to the script")]
    pub args: Option<Vec<serde_json::Value>>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct IosCookiesParams {
    // no params needed
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct IosAppLaunchParams {
    #[schemars(description = "Bundle ID of the app to launch")]
    pub bundle_id: String,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct IosAppKillParams {
    #[schemars(description = "Bundle ID of the app to terminate")]
    pub bundle_id: String,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct IosAppStateParams {
    #[schemars(description = "Bundle ID of the app to check")]
    pub bundle_id: String,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct IosLockParams {
    // no params needed
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct IosUnlockParams {
    // no params needed
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct IosHomeParams {
    // no params needed
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct IosButtonParams {
    #[schemars(description = "Button name (e.g. 'volumeUp', 'volumeDown')")]
    pub name: String,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct IosBatteryParams {
    // no params needed
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct IosInfoParams {
    // no params needed
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct IosSimulatorParams {
    #[schemars(description = "Simulator action: list, boot, shutdown, create, delete")]
    pub action: String,
    #[schemars(description = "Device UDID (required for boot/shutdown/delete)")]
    pub udid: Option<String>,
    #[schemars(description = "Device type for create (e.g. 'iPhone 15')")]
    pub device_type: Option<String>,
    #[schemars(description = "Runtime for create (e.g. 'com.apple.CoreSimulator.SimRuntime.iOS-17-0')")]
    pub runtime: Option<String>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct IosUrlParams {
    // no params needed
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct IosTitleParams {
    // no params needed
}

// ──────────────── Android / UIAutomator2 params ─────────────────

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct AndroidDevicesParams {
    // no params needed — lists connected Android devices via ADB
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct AndroidConnectParams {
    #[schemars(description = "UIAutomator2 server URL (default: http://localhost:4723)")]
    pub server_url: Option<String>,
    #[schemars(description = "Device serial (auto-detect if omitted)")]
    pub serial: Option<String>,
    #[schemars(description = "Android package to automate (default: com.android.chrome)")]
    pub package: Option<String>,
    #[schemars(description = "Activity to launch (optional)")]
    pub activity: Option<String>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct AndroidNavigateParams {
    #[schemars(description = "URL to navigate to in Chrome")]
    pub url: String,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct AndroidTapParams {
    #[schemars(description = "X coordinate")]
    pub x: f64,
    #[schemars(description = "Y coordinate")]
    pub y: f64,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct AndroidSwipeParams {
    #[schemars(description = "Start X coordinate")]
    pub from_x: f64,
    #[schemars(description = "Start Y coordinate")]
    pub from_y: f64,
    #[schemars(description = "End X coordinate")]
    pub to_x: f64,
    #[schemars(description = "End Y coordinate")]
    pub to_y: f64,
    #[schemars(description = "Duration in milliseconds (default: 500)")]
    pub duration_ms: Option<u64>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct AndroidLongPressParams {
    #[schemars(description = "X coordinate")]
    pub x: f64,
    #[schemars(description = "Y coordinate")]
    pub y: f64,
    #[schemars(description = "Duration in milliseconds (default: 1000)")]
    pub duration_ms: Option<u64>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct AndroidDoubleTapParams {
    #[schemars(description = "X coordinate")]
    pub x: f64,
    #[schemars(description = "Y coordinate")]
    pub y: f64,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct AndroidPinchParams {
    #[schemars(description = "X coordinate of pinch center")]
    pub x: f64,
    #[schemars(description = "Y coordinate of pinch center")]
    pub y: f64,
    #[schemars(description = "Scale factor (>1 zoom in, <1 zoom out)")]
    pub scale: f64,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct AndroidTypeParams {
    #[schemars(description = "Text to type into the focused element")]
    pub text: String,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct AndroidFindParams {
    #[schemars(description = "Locator strategy (e.g. 'id', 'xpath', 'accessibility id', 'class name')")]
    pub strategy: String,
    #[schemars(description = "Locator value")]
    pub value: String,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct AndroidClickParams {
    #[schemars(description = "Element ID to click")]
    pub element_id: String,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct AndroidScreenshotParams {
    // no params needed — returns base64 PNG
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct AndroidOrientationParams {
    #[schemars(description = "Orientation to set (PORTRAIT/LANDSCAPE). Omit to get current.")]
    pub set: Option<String>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct AndroidKeyParams {
    #[schemars(description = "Android keycode (e.g. 3=HOME, 4=BACK, 24=VOLUME_UP, 25=VOLUME_DOWN, 26=POWER)")]
    pub keycode: i32,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct AndroidAppLaunchParams {
    #[schemars(description = "Package name of the app to launch")]
    pub package: String,
    #[schemars(description = "Activity to launch (optional)")]
    pub activity: Option<String>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct AndroidAppKillParams {
    #[schemars(description = "Package name of the app to terminate")]
    pub package: String,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct AndroidAppStateParams {
    #[schemars(description = "Package name of the app to check")]
    pub package: String,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct AndroidInstallParams {
    #[schemars(description = "Path to the APK file to install")]
    pub apk_path: String,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct AndroidScriptParams {
    #[schemars(description = "JavaScript code to execute in Chrome")]
    pub script: String,
    #[schemars(description = "Arguments to pass to the script")]
    pub args: Option<Vec<serde_json::Value>>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct AndroidShellParams {
    #[schemars(description = "Device serial number")]
    pub serial: String,
    #[schemars(description = "Shell command to execute")]
    pub command: String,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct AndroidPushParams {
    #[schemars(description = "Device serial number")]
    pub serial: String,
    #[schemars(description = "Local file path")]
    pub local: String,
    #[schemars(description = "Remote path on device")]
    pub remote: String,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct AndroidPullParams {
    #[schemars(description = "Device serial number")]
    pub serial: String,
    #[schemars(description = "Remote path on device")]
    pub remote: String,
    #[schemars(description = "Local file path")]
    pub local: String,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct AndroidInfoParams {
    #[schemars(description = "Device serial number")]
    pub serial: String,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct AndroidBatteryParams {
    #[schemars(description = "Device serial number")]
    pub serial: String,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct AndroidUrlParams {
    // no params needed
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct AndroidTitleParams {
    // no params needed
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

// ──────────────── Drag, Hover, Keyboard, Select params ─────────────────

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct DragParams {
    #[schemars(description = "CSS selector of source element")]
    pub source: String,
    #[schemars(description = "CSS selector of target element")]
    pub target: String,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct HoverParams {
    #[schemars(description = "CSS selector of element to hover")]
    pub selector: String,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct KeyboardParams {
    #[schemars(description = "Key combo string (e.g. 'Control+a', 'Shift+Tab', 'Enter', 'Escape')")]
    pub keys: String,
    #[schemars(description = "CSS selector to focus before sending keys")]
    pub selector: Option<String>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct SelectParams {
    #[schemars(description = "CSS selector of <select> element")]
    pub selector: String,
    #[schemars(description = "Option value to select")]
    pub value: Option<String>,
    #[schemars(description = "Option visible text to select")]
    pub text: Option<String>,
    #[schemars(description = "Option index to select (0-based)")]
    pub index: Option<usize>,
}

// ──────────────── File Upload/Download params ─────────────────

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct UploadParams {
    #[schemars(description = "CSS selector of <input type='file'> element")]
    pub selector: String,
    #[schemars(description = "Absolute path to file to upload")]
    pub file_path: String,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct DownloadWaitParams {
    #[schemars(description = "Timeout in ms (default: 30000)")]
    pub timeout: Option<u64>,
    #[schemars(description = "Download directory to monitor")]
    pub dir: Option<String>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct DownloadSetDirParams {
    #[schemars(description = "Directory path for downloads")]
    pub path: String,
}

// ──────────────── Shadow DOM params ─────────────────

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct ShadowQueryParams {
    #[schemars(description = "CSS selector of shadow host element")]
    pub host_selector: String,
    #[schemars(description = "CSS selector within the shadow root")]
    pub inner_selector: String,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct DeepQueryParams {
    #[schemars(description = "CSS selector using >>> for shadow-piercing (e.g. 'my-element >>> .inner')")]
    pub selector: String,
}

// ──────────────── Page Context params ─────────────────

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct PageContextSetParams {
    #[schemars(description = "Context key to set")]
    pub key: String,
    #[schemars(description = "Context value (any JSON)")]
    pub value: serde_json::Value,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct PageContextGetParams {
    #[schemars(description = "Context key to retrieve")]
    pub key: String,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct PageContextTransferParams {
    #[schemars(description = "Source tab index")]
    pub from_tab: usize,
    #[schemars(description = "Target tab index")]
    pub to_tab: usize,
    #[schemars(description = "Keys to transfer (None = all)")]
    pub keys: Option<Vec<String>>,
}

// ──────────────── Smart Form Mapping params ─────────────────

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct FormInferParams {
    #[schemars(description = "CSS selector of the form (default: 'form')")]
    pub selector: Option<String>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct FormAutoFillParams {
    #[schemars(description = "Data to fill as JSON object (e.g. {\"email\":\"test@x.com\"})")]
    pub data: serde_json::Value,
    #[schemars(description = "CSS selector of the form (default: 'form')")]
    pub selector: Option<String>,
    #[schemars(description = "Minimum match confidence 0.0-1.0 (default: 0.5)")]
    pub confidence_threshold: Option<f64>,
}

// ──────────────── Error Recovery params ─────────────────

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct RetryAdaptParams {
    #[schemars(description = "Action name to retry")]
    pub action: String,
    #[schemars(description = "Parameters for the action")]
    pub params: serde_json::Value,
    #[schemars(description = "Maximum retries (default: 3)")]
    pub max_retries: Option<u32>,
    #[schemars(description = "Retry strategy: 'exponential' | 'linear' | 'immediate'")]
    pub strategy: Option<String>,
    #[schemars(description = "Error handling: 'retry' | 'skip' | 'alternative'")]
    pub on_error: Option<String>,
    #[schemars(description = "Alternative action name if on_error='alternative'")]
    pub alternative_action: Option<String>,
    #[schemars(description = "Alternative action params")]
    pub alternative_params: Option<serde_json::Value>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct ErrorClassifyParams {
    #[schemars(description = "Error message to classify")]
    pub error_message: String,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct RecoveryStrategyParams {
    #[schemars(description = "Error type: 'selector_not_found' | 'timeout' | 'navigation' | 'network'")]
    pub error_type: String,
    #[schemars(description = "Additional context for generating recovery steps")]
    pub context: Option<serde_json::Value>,
}

// ──────────────── Task Decomposition Engine params ─────────────────

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct TaskDecomposeParams {
    #[schemars(description = "High-level goal to decompose into atomic subtasks")]
    pub goal: String,
    #[schemars(description = "Additional context about the goal")]
    pub context: Option<String>,
    #[schemars(description = "Maximum decomposition depth (default: 3)")]
    pub max_depth: Option<u32>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct TaskPlanParams {
    #[schemars(description = "List of task descriptions to plan")]
    pub tasks: Vec<String>,
    #[schemars(description = "Planning strategy: 'sequential' | 'parallel' | 'dependency'")]
    pub strategy: Option<String>,
}

// ──────────────── Vision/LLM Observation Layer params ─────────────────

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct VisionDescribeParams {
    #[schemars(description = "CSS selector to scope description (whole page if absent)")]
    pub selector: Option<String>,
    #[schemars(description = "Output format: 'brief' | 'detailed' | 'structured'")]
    pub format: Option<String>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct VisionLocateParams {
    #[schemars(description = "Natural language description of the element to find (e.g. 'blue submit button')")]
    pub description: String,
    #[schemars(description = "Search strategy: 'aria' | 'visual' | 'semantic'")]
    pub strategy: Option<String>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct VisionCompareParams {
    #[schemars(description = "Baseline accessibility tree text or description")]
    pub baseline: String,
    #[schemars(description = "Current state to compare (if absent, captures current page state)")]
    pub current: Option<String>,
    #[schemars(description = "Similarity threshold 0.0–1.0 (default: 0.9)")]
    pub threshold: Option<f64>,
}

// ──────────────── Self-Healing Selector Recovery params ─────────────────

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct SelectorHealParams {
    #[schemars(description = "The broken CSS/XPath selector to heal")]
    pub selector: String,
    #[schemars(description = "What the element should be (e.g. 'login button')")]
    pub context: Option<String>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct SelectorAlternativesParams {
    #[schemars(description = "CSS selector of a currently working element")]
    pub selector: String,
    #[schemars(description = "Maximum number of alternative selectors to generate (default: 5)")]
    pub max_alternatives: Option<u32>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct SelectorValidateParams {
    #[schemars(description = "CSS selector to validate")]
    pub selector: String,
    #[schemars(description = "Expected ARIA role of the matched element")]
    pub expected_role: Option<String>,
    #[schemars(description = "Expected visible text of the matched element")]
    pub expected_text: Option<String>,
}

// ──────────────── Session Checkpoints/Resume params ─────────────────

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct CheckpointSaveParams {
    #[schemars(description = "Unique name for this checkpoint")]
    pub name: String,
    #[schemars(description = "Include cookies in checkpoint (default: true)")]
    pub include_cookies: Option<bool>,
    #[schemars(description = "Include localStorage/sessionStorage (default: true)")]
    pub include_storage: Option<bool>,
    #[schemars(description = "Include page context from BrowserState (default: true)")]
    pub include_context: Option<bool>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct CheckpointRestoreParams {
    #[schemars(description = "Name of the checkpoint to restore")]
    pub name: String,
    #[schemars(description = "Navigate to the saved URL (default: true)")]
    pub restore_url: Option<bool>,
    #[schemars(description = "Restore cookies (default: true)")]
    pub restore_cookies: Option<bool>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct CheckpointDeleteParams {
    #[schemars(description = "Name of the checkpoint to delete")]
    pub name: String,
}

// ──────────────── Extended Workflow DSL params ─────────────────

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct WorkflowWhileParams {
    #[schemars(description = "JavaScript expression to evaluate as loop condition")]
    pub condition: String,
    #[schemars(description = "Actions to execute each iteration (ChainCommand objects)")]
    pub actions: Vec<serde_json::Value>,
    #[schemars(description = "Maximum iterations to prevent infinite loops (default: 100)")]
    pub max_iterations: Option<u32>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct WorkflowForEachParams {
    #[schemars(description = "JS expression returning an array, or a JSON array string")]
    pub collection: String,
    #[schemars(description = "Variable name for current item (default: 'item')")]
    pub variable_name: Option<String>,
    #[schemars(description = "Actions to execute for each item (ChainCommand objects)")]
    pub actions: Vec<serde_json::Value>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct WorkflowIfParams {
    #[schemars(description = "JavaScript expression to evaluate as condition")]
    pub condition: String,
    #[schemars(description = "Actions to execute if condition is truthy")]
    pub then_actions: Vec<serde_json::Value>,
    #[schemars(description = "Actions to execute if condition is falsy")]
    pub else_actions: Option<Vec<serde_json::Value>>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct WorkflowVariableParams {
    #[schemars(description = "Variable name")]
    pub name: String,
    #[schemars(description = "Value to set (omit to GET the current value)")]
    pub value: Option<serde_json::Value>,
}

// ──────────────── Event-Driven Reaction System params ─────────────────

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct EventSubscribeParams {
    #[schemars(description = "Event type: 'navigation' | 'console' | 'dialog' | 'error' | 'network' | 'dom_change'")]
    pub event_type: String,
    #[schemars(description = "Optional filter pattern for events")]
    pub filter: Option<String>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct EventUnsubscribeParams {
    #[schemars(description = "Event type to unsubscribe from")]
    pub event_type: String,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct EventPollParams {
    #[schemars(description = "Filter by event type (all types if absent)")]
    pub event_type: Option<String>,
    #[schemars(description = "Maximum events to return (default: 50)")]
    pub limit: Option<u32>,
    #[schemars(description = "Clear returned events from buffer (default: false)")]
    pub clear: Option<bool>,
}

// ──────────────── Structured Data Pipeline params ─────────────────

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct ExtractSchemaParams {
    #[schemars(description = "Schema type: json_ld, open_graph, twitter_card, microdata, all")]
    pub schema_type: Option<String>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct ExtractTablesParams {
    #[schemars(description = "CSS selector for table (default: 'table')")]
    pub selector: Option<String>,
    #[schemars(description = "Output format: json, csv, array (default: json)")]
    pub format: Option<String>,
    #[schemars(description = "Use first row as headers (default: true)")]
    pub headers: Option<bool>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct ExtractEntitiesParams {
    #[schemars(description = "Entity types to extract: emails, phones, urls, dates, prices, addresses")]
    pub types: Option<Vec<String>>,
    #[schemars(description = "CSS selector to scope extraction")]
    pub selector: Option<String>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct ClassifyContentParams {
    #[schemars(description = "Classification strategy: topic, sentiment, language, type")]
    pub strategy: Option<String>,
    #[schemars(description = "CSS selector to scope content")]
    pub selector: Option<String>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct TransformJsonParams {
    #[schemars(description = "Input JSON data")]
    pub data: serde_json::Value,
    #[schemars(description = "JMESPath-style expression or transform operations")]
    pub transform: String,
    #[schemars(description = "Output format: json, csv, yaml, table")]
    pub output_format: Option<String>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct ExportCsvParams {
    #[schemars(description = "JSON array of objects to export")]
    pub data: serde_json::Value,
    #[schemars(description = "Column headers (auto-detected if absent)")]
    pub columns: Option<Vec<String>>,
    #[schemars(description = "Delimiter character (default: comma)")]
    pub delimiter: Option<String>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct ExtractMetadataParams {
    #[schemars(description = "Include Open Graph metadata")]
    pub include_og: Option<bool>,
    #[schemars(description = "Include Twitter Card metadata")]
    pub include_twitter: Option<bool>,
    #[schemars(description = "Include all meta tags")]
    pub include_all: Option<bool>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct ExtractFeedsParams {
    #[schemars(description = "Feed types to detect: rss, atom, json_feed, all")]
    pub feed_type: Option<String>,
}

// ──────────────── Multi-Browser Fleet params ─────────────────

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct FleetSpawnParams {
    #[schemars(description = "Number of browser instances to spawn")]
    pub count: u32,
    #[schemars(description = "Browser type: chrome, firefox, webkit (default: chrome)")]
    pub browser_type: Option<String>,
    #[schemars(description = "Run headless (default: true)")]
    pub headless: Option<bool>,
    #[schemars(description = "Fleet name/label for identification")]
    pub name: Option<String>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct FleetBroadcastParams {
    #[schemars(description = "Action to broadcast to all fleet instances")]
    pub action: String,
    #[schemars(description = "Parameters for the action")]
    pub params: Option<serde_json::Value>,
    #[schemars(description = "Specific instance indices to target (all if absent)")]
    pub targets: Option<Vec<u32>>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct FleetCollectParams {
    #[schemars(description = "What to collect: screenshots, text, html, data, all")]
    pub collect_type: String,
    #[schemars(description = "CSS selector to scope collection")]
    pub selector: Option<String>,
    #[schemars(description = "Merge strategy: concat, zip, group (default: group)")]
    pub merge_strategy: Option<String>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct FleetDestroyParams {
    #[schemars(description = "Specific instance indices to destroy (all if absent)")]
    pub targets: Option<Vec<u32>>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct FleetBalanceParams {
    #[schemars(description = "URLs to distribute across fleet")]
    pub urls: Vec<String>,
    #[schemars(description = "Distribution strategy: round_robin, random, load_based (default: round_robin)")]
    pub strategy: Option<String>,
    #[schemars(description = "Action to perform on each URL")]
    pub action: Option<String>,
}

// ──────────────── Authentication Flows params ─────────────────

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct AuthOauth2Params {
    #[schemars(description = "OAuth2 authorization URL")]
    pub auth_url: String,
    #[schemars(description = "Token endpoint URL")]
    pub token_url: String,
    #[schemars(description = "Client ID")]
    pub client_id: String,
    #[schemars(description = "Client secret (optional for PKCE flows)")]
    pub client_secret: Option<String>,
    #[schemars(description = "Redirect URI")]
    pub redirect_uri: Option<String>,
    #[schemars(description = "OAuth2 scopes")]
    pub scopes: Option<Vec<String>>,
    #[schemars(description = "Use PKCE (default: true)")]
    pub use_pkce: Option<bool>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct AuthSessionParams {
    #[schemars(description = "Session name for identification")]
    pub name: String,
    #[schemars(description = "Export current session cookies/storage")]
    pub export: Option<bool>,
    #[schemars(description = "Session data to import (JSON)")]
    pub import_data: Option<String>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct AuthFormLoginParams {
    #[schemars(description = "Login page URL")]
    pub url: String,
    #[schemars(description = "Username/email field selector")]
    pub username_selector: Option<String>,
    #[schemars(description = "Password field selector")]
    pub password_selector: Option<String>,
    #[schemars(description = "Submit button selector")]
    pub submit_selector: Option<String>,
    #[schemars(description = "Username/email value")]
    pub username: String,
    #[schemars(description = "Password value")]
    pub password: String,
    #[schemars(description = "Expected URL or selector after successful login")]
    pub success_indicator: Option<String>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct AuthMfaParams {
    #[schemars(description = "MFA type: totp, sms, email")]
    pub mfa_type: String,
    #[schemars(description = "TOTP secret for code generation")]
    pub totp_secret: Option<String>,
    #[schemars(description = "MFA code input field selector")]
    pub code_selector: Option<String>,
    #[schemars(description = "Manual MFA code (if not auto-generating)")]
    pub code: Option<String>,
    #[schemars(description = "Submit button selector")]
    pub submit_selector: Option<String>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct CredentialStoreParams {
    #[schemars(description = "Credential label/name")]
    pub label: String,
    #[schemars(description = "Username/email")]
    pub username: String,
    #[schemars(description = "Password")]
    pub password: String,
    #[schemars(description = "Associated domain/URL")]
    pub domain: Option<String>,
    #[schemars(description = "Additional metadata")]
    pub metadata: Option<serde_json::Value>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct CredentialGetParams {
    #[schemars(description = "Credential label to retrieve")]
    pub label: String,
}

// ── Service Worker & PWA Control ──────────────────────────────────

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct SwRegisterParams {
    #[schemars(description = "Service worker script URL")]
    pub script_url: String,
    #[schemars(description = "Scope for the service worker")]
    pub scope: Option<String>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct SwUnregisterParams {
    #[schemars(description = "Scope of service worker to unregister")]
    pub scope: Option<String>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct SwUpdateParams {
    #[schemars(description = "Scope of service worker to update")]
    pub scope: Option<String>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct PushSimulateParams {
    #[schemars(description = "Push notification title")]
    pub title: String,
    #[schemars(description = "Push notification body")]
    pub body: Option<String>,
    #[schemars(description = "Push notification tag")]
    pub tag: Option<String>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct OfflineModeParams {
    #[schemars(description = "Enable offline mode")]
    pub enabled: bool,
    #[schemars(description = "Latency in ms to simulate (0 = full offline)")]
    pub latency_ms: Option<u64>,
}

// ── Accessibility & WCAG Engine ──────────────────────────────────

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct WcagAuditParams {
    #[schemars(description = "WCAG level: A, AA, or AAA")]
    pub level: Option<String>,
    #[schemars(description = "CSS selector to scope audit")]
    pub selector: Option<String>,
    #[schemars(description = "Include passing rules in results")]
    pub include_passes: Option<bool>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct ContrastCheckParams {
    #[schemars(description = "CSS selector to check")]
    pub selector: Option<String>,
    #[schemars(description = "Minimum contrast ratio (default 4.5 for AA)")]
    pub min_ratio: Option<f64>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct AltTextAuditParams {
    #[schemars(description = "Include decorative images in report")]
    pub include_decorative: Option<bool>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct RoleValidateParams {
    #[schemars(description = "CSS selector to scope validation")]
    pub selector: Option<String>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct ScreenReaderSimParams {
    #[schemars(description = "CSS selector of element to start reading from")]
    pub start_selector: Option<String>,
    #[schemars(description = "Maximum elements to read")]
    pub max_elements: Option<u32>,
}

// ── WebSocket & Real-Time Protocol ──────────────────────────────

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct WsConnectParams {
    #[schemars(description = "WebSocket URL to connect to")]
    pub url: String,
    #[schemars(description = "Sub-protocols to request")]
    pub protocols: Option<Vec<String>>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct WsInterceptParams {
    #[schemars(description = "URL pattern to intercept (glob)")]
    pub url_pattern: Option<String>,
    #[schemars(description = "Only capture (true) or also modify (false)")]
    pub capture_only: Option<bool>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct WsSendParams {
    #[schemars(description = "WebSocket URL or connection ID")]
    pub target: String,
    #[schemars(description = "Message to send")]
    pub message: String,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct WsMessagesParams {
    #[schemars(description = "URL filter for messages")]
    pub url_filter: Option<String>,
    #[schemars(description = "Maximum messages to return")]
    pub limit: Option<u32>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct WsCloseParams {
    #[schemars(description = "WebSocket URL or connection ID to close")]
    pub target: Option<String>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct SseListenParams {
    #[schemars(description = "SSE endpoint URL")]
    pub url: String,
    #[schemars(description = "Duration in ms to listen")]
    pub duration_ms: Option<u64>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct SseMessagesParams {
    #[schemars(description = "URL filter for SSE messages")]
    pub url_filter: Option<String>,
    #[schemars(description = "Maximum messages to return")]
    pub limit: Option<u32>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct GraphqlSubscribeParams {
    #[schemars(description = "GraphQL endpoint URL")]
    pub url: String,
    #[schemars(description = "GraphQL subscription query")]
    pub query: String,
    #[schemars(description = "GraphQL variables")]
    pub variables: Option<serde_json::Value>,
    #[schemars(description = "Duration in ms to listen")]
    pub duration_ms: Option<u64>,
}

// ── Human Behavior Simulation ────────────────────────────────────

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct HumanDelayParams {
    #[schemars(description = "Minimum delay in milliseconds")]
    pub min_ms: u64,
    #[schemars(description = "Maximum delay in milliseconds")]
    pub max_ms: u64,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct HumanMouseParams {
    #[schemars(description = "Target X coordinate")]
    pub x: f64,
    #[schemars(description = "Target Y coordinate")]
    pub y: f64,
    #[schemars(description = "Number of intermediate steps (default 20)")]
    pub steps: Option<u32>,
    #[schemars(description = "Movement speed: slow, normal, fast")]
    pub speed: Option<String>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct HumanTypeParams {
    #[schemars(description = "CSS selector of input element")]
    pub selector: String,
    #[schemars(description = "Text to type")]
    pub text: String,
    #[schemars(description = "Min delay between keystrokes in ms (default 50)")]
    pub min_delay_ms: Option<u64>,
    #[schemars(description = "Max delay between keystrokes in ms (default 200)")]
    pub max_delay_ms: Option<u64>,
    #[schemars(description = "Simulate occasional typos and corrections")]
    pub typos: Option<bool>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct HumanScrollParams {
    #[schemars(description = "Scroll direction: down, up, left, right")]
    pub direction: Option<String>,
    #[schemars(description = "Distance in pixels")]
    pub distance: Option<i32>,
    #[schemars(description = "Number of scroll steps (default 5)")]
    pub steps: Option<u32>,
    #[schemars(description = "Speed: slow, normal, fast")]
    pub speed: Option<String>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct HumanProfileParams {
    #[schemars(description = "Profile name: fast, normal, careful, elderly")]
    pub profile: String,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct StealthMaxParams {
    #[schemars(description = "Enable all stealth features")]
    pub enable_all: Option<bool>,
    #[schemars(description = "Include human behavior simulation")]
    pub human_simulation: Option<bool>,
}

// ── Session Configuration ──────────────────────────────────

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct SetModeParams {
    #[schemars(description = "Browser mode: 'headless' (default) or 'headed'")]
    pub mode: String,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct SetStealthParams {
    #[schemars(description = "Enable or disable stealth patches (enabled by default)")]
    pub enabled: bool,
}

// ── Enhanced Computer Use ──────────────────────────────────

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct ComputerUseParams {
    #[schemars(description = "High-level goal to accomplish (e.g. 'search for X on Google and extract results')")]
    pub goal: String,
    #[schemars(description = "Starting URL (optional, uses current page if omitted)")]
    pub url: Option<String>,
    #[schemars(description = "Maximum number of steps to attempt")]
    pub max_steps: Option<u32>,
    #[schemars(description = "Take screenshot at each step")]
    pub screenshots: Option<bool>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct GoalExecuteParams {
    #[schemars(description = "Plan ID from task_plan or computer_use to execute")]
    pub plan_id: String,
    #[schemars(description = "Step ID to start from (optional, starts from first pending)")]
    pub from_step: Option<String>,
    #[schemars(description = "Stop after this step ID")]
    pub until_step: Option<String>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct StepVerifyParams {
    #[schemars(description = "Plan ID to verify")]
    pub plan_id: String,
    #[schemars(description = "Step ID to verify")]
    pub step_id: String,
    #[schemars(description = "Expected condition (CSS selector exists, text contains, URL matches)")]
    pub expect: Option<String>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct AutoRecoverParams {
    #[schemars(description = "Plan ID where the failure occurred")]
    pub plan_id: String,
    #[schemars(description = "Failed step ID")]
    pub step_id: String,
    #[schemars(description = "Error message from the failure")]
    pub error: Option<String>,
    #[schemars(description = "Maximum recovery attempts")]
    pub max_retries: Option<u32>,
}

// ── CAPTCHA Solving ──

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct SolveCaptchaParams {
    #[schemars(description = "CAPTCHA type to solve: 'recaptcha_checkbox' (click the checkbox), 'recaptcha_audio' (audio challenge + Whisper STT), 'turnstile' (Cloudflare Turnstile), 'auto' (detect and solve). Default: 'auto'")]
    pub captcha_type: Option<String>,
    #[schemars(description = "Timeout in milliseconds for challenge clearance. Default: 15000")]
    pub timeout_ms: Option<u64>,
}

// ── Cross-Origin Iframe (CDP) ──

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct IframeEvalCdpParams {
    #[schemars(description = "URL substring to match the target iframe (e.g. 'recaptcha', 'stripe.com'). Uses CDP Page.getFrameTree to find the frame, then createIsolatedWorld with universal access to evaluate cross-origin.")]
    pub frame_url: String,
    #[schemars(description = "JavaScript expression to evaluate inside the iframe")]
    pub expression: String,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct IframeClickCdpParams {
    #[schemars(description = "URL substring to match the target iframe")]
    pub frame_url: String,
    #[schemars(description = "CSS selector of the element to click inside the iframe")]
    pub selector: String,
    #[schemars(description = "Use human-like bezier mouse movement to the element. Default: true")]
    pub human_like: Option<bool>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn intercept_rule_default() {
        let rule = InterceptRule::default();
        assert_eq!(rule.response_status, 200);
        assert!(rule.id.is_empty());
        assert!(rule.response_headers.is_empty());
    }

    #[test]
    fn intercept_rule_roundtrip() {
        let mut headers = HashMap::new();
        headers.insert("Content-Type".into(), "application/json".into());
        let rule = InterceptRule {
            id: "rule_1".into(),
            url_pattern: "**/api/*".into(),
            method: Some("GET".into()),
            response_status: 404,
            response_headers: headers,
            response_body: r#"{"error":"not found"}"#.into(),
        };
        let json = serde_json::to_string(&rule).unwrap();
        let restored: InterceptRule = serde_json::from_str(&json).unwrap();
        assert_eq!(restored.id, "rule_1");
        assert_eq!(restored.response_status, 404);
        assert_eq!(restored.method.as_deref(), Some("GET"));
    }

    #[test]
    fn intercept_enable_params_defaults() {
        let p: InterceptEnableParams = serde_json::from_str("{}").unwrap();
        assert!(p.patterns.is_none());
    }

    #[test]
    fn intercept_add_rule_params_minimal() {
        let p: InterceptAddRuleParams = serde_json::from_str(r#"{"url_pattern":"**/api/*"}"#).unwrap();
        assert_eq!(p.url_pattern, "**/api/*");
        assert!(p.status.is_none());
        assert!(p.body.is_none());
    }

    #[test]
    fn block_requests_params() {
        let p: BlockRequestsParams = serde_json::from_str(r#"{"patterns":["*.png","*.jpg"],"resource_types":["image"]}"#).unwrap();
        assert_eq!(p.patterns.len(), 2);
        assert_eq!(p.resource_types.as_ref().unwrap()[0], "image");
    }

    #[test]
    fn console_message_roundtrip() {
        let msg = ConsoleMessage {
            level: "error".into(),
            text: "Uncaught TypeError".into(),
            url: Some("https://example.com/app.js".into()),
            line: Some(42),
            timestamp_ms: 1700000000000,
        };
        let json = serde_json::to_string(&msg).unwrap();
        let restored: ConsoleMessage = serde_json::from_str(&json).unwrap();
        assert_eq!(restored.level, "error");
        assert_eq!(restored.line, Some(42));
    }

    #[test]
    fn dialog_info_roundtrip() {
        let info = DialogInfo {
            dialog_type: "confirm".into(),
            message: "Are you sure?".into(),
            default_prompt: None,
            was_handled: true,
            response: Some("true".into()),
        };
        let json = serde_json::to_string(&info).unwrap();
        let restored: DialogInfo = serde_json::from_str(&json).unwrap();
        assert_eq!(restored.dialog_type, "confirm");
        assert!(restored.was_handled);
    }

    #[test]
    fn page_error_roundtrip() {
        let err = PageError {
            message: "ReferenceError: x is not defined".into(),
            url: Some("https://example.com".into()),
            line: Some(10),
            column: Some(5),
            timestamp_ms: 1700000000000,
        };
        let json = serde_json::to_string(&err).unwrap();
        let restored: PageError = serde_json::from_str(&json).unwrap();
        assert_eq!(restored.column, Some(5));
    }

    #[test]
    fn dialog_handle_params() {
        let p: DialogHandleParams = serde_json::from_str(r#"{"accept":true,"prompt_text":"hello"}"#).unwrap();
        assert!(p.accept);
        assert_eq!(p.prompt_text.as_deref(), Some("hello"));
    }

    #[test]
    fn console_filter_params() {
        let p: ConsoleFilterParams = serde_json::from_str(r#"{"level":"error","limit":10}"#).unwrap();
        assert_eq!(p.level.as_deref(), Some("error"));
        assert_eq!(p.limit, Some(10));
    }

    #[test]
    fn emulate_device_params_preset() {
        let p: EmulateDeviceParams = serde_json::from_str(r#"{"device":"iphone-14"}"#).unwrap();
        assert_eq!(p.device.as_deref(), Some("iphone-14"));
        assert!(p.width.is_none());
    }

    #[test]
    fn emulate_device_params_custom() {
        let p: EmulateDeviceParams = serde_json::from_str(
            r#"{"device":"custom","width":1920,"height":1080,"device_scale_factor":2.0,"has_touch":false}"#
        ).unwrap();
        assert_eq!(p.width, Some(1920));
        assert_eq!(p.height, Some(1080));
        assert_eq!(p.device_scale_factor, Some(2.0));
        assert_eq!(p.has_touch, Some(false));
    }

    #[test]
    fn emulate_geolocation_params() {
        let p: EmulateGeolocationParams = serde_json::from_str(
            r#"{"latitude":41.9028,"longitude":12.4964,"accuracy":10.0}"#
        ).unwrap();
        assert!((p.latitude - 41.9028).abs() < 0.001);
        assert!((p.longitude - 12.4964).abs() < 0.001);
        assert_eq!(p.accuracy, Some(10.0));
    }

    #[test]
    fn emulate_timezone_params() {
        let p: EmulateTimezoneParams = serde_json::from_str(r#"{"timezone_id":"Europe/Rome"}"#).unwrap();
        assert_eq!(p.timezone_id, "Europe/Rome");
    }

    #[test]
    fn emulate_media_params() {
        let p: EmulateMediaParams = serde_json::from_str(
            r#"{"color_scheme":"dark","reduced_motion":"reduce"}"#
        ).unwrap();
        assert_eq!(p.color_scheme.as_deref(), Some("dark"));
        assert_eq!(p.reduced_motion.as_deref(), Some("reduce"));
    }

    #[test]
    fn emulate_network_params_preset() {
        let p: EmulateNetworkParams = serde_json::from_str(r#"{"preset":"3g"}"#).unwrap();
        assert_eq!(p.preset.as_deref(), Some("3g"));
        assert!(p.offline.is_none());
    }

    #[test]
    fn emulate_network_params_offline() {
        let p: EmulateNetworkParams = serde_json::from_str(r#"{"preset":"offline","offline":true}"#).unwrap();
        assert_eq!(p.offline, Some(true));
    }

    #[test]
    fn browser_state_default_fields() {
        let state = BrowserState::default();
        assert!(state.intercept_rules.is_empty());
        assert!(!state.intercepting);
        assert!(state.console_messages.is_empty());
        assert!(!state.capturing_console);
        assert!(state.last_dialog.is_none());
        assert!(state.dialog_auto_response.is_none());
        assert!(state.page_errors.is_empty());
    }

    #[test]
    fn dialog_auto_response_serde() {
        let resp = DialogAutoResponse {
            accept: false,
            prompt_text: Some("cancel".into()),
        };
        let json = serde_json::to_string(&resp).unwrap();
        let restored: DialogAutoResponse = serde_json::from_str(&json).unwrap();
        assert!(!restored.accept);
        assert_eq!(restored.prompt_text.as_deref(), Some("cancel"));
    }

    #[test]
    fn drag_params() {
        let p: DragParams = serde_json::from_str(r##"{"source":"#item1","target":"#dropzone"}"##).unwrap();
        assert_eq!(p.source, "#item1");
        assert_eq!(p.target, "#dropzone");
    }

    #[test]
    fn keyboard_params() {
        let p: KeyboardParams = serde_json::from_str(r##"{"keys":"Control+a","selector":"#editor"}"##).unwrap();
        assert_eq!(p.keys, "Control+a");
        assert_eq!(p.selector.as_deref(), Some("#editor"));
    }

    #[test]
    fn select_params_by_value() {
        let p: SelectParams = serde_json::from_str(r##"{"selector":"#country","value":"it"}"##).unwrap();
        assert_eq!(p.value.as_deref(), Some("it"));
        assert!(p.text.is_none());
    }

    #[test]
    fn upload_params() {
        let p: UploadParams = serde_json::from_str(r#"{"selector":"input[type=file]","file_path":"/tmp/doc.pdf"}"#).unwrap();
        assert_eq!(p.file_path, "/tmp/doc.pdf");
    }

    #[test]
    fn shadow_query_params() {
        let p: ShadowQueryParams = serde_json::from_str(r#"{"host_selector":"my-element","inner_selector":".inner-btn"}"#).unwrap();
        assert_eq!(p.host_selector, "my-element");
        assert_eq!(p.inner_selector, ".inner-btn");
    }

    #[test]
    fn deep_query_params() {
        let p: DeepQueryParams = serde_json::from_str(r#"{"selector":"my-element >>> .inner"}"#).unwrap();
        assert!(p.selector.contains(">>>"));
    }

    // ── Agentic Task Decomposition ──

    #[test]
    fn task_decompose_params() {
        let p: TaskDecomposeParams = serde_json::from_str(r#"{"goal":"click login"}"#).unwrap();
        assert_eq!(p.goal, "click login");
        assert!(p.context.is_none());
        assert!(p.max_depth.is_none());
    }

    #[test]
    fn task_decompose_params_full() {
        let p: TaskDecomposeParams = serde_json::from_str(
            r#"{"goal":"fill form","context":"login page","max_depth":3}"#,
        )
        .unwrap();
        assert_eq!(p.goal, "fill form");
        assert_eq!(p.context.as_deref(), Some("login page"));
        assert_eq!(p.max_depth, Some(3));
    }

    #[test]
    fn task_plan_params() {
        let p: TaskPlanParams = serde_json::from_str(
            r#"{"tasks":["navigate","click","verify"],"strategy":"sequential"}"#,
        )
        .unwrap();
        assert_eq!(p.tasks.len(), 3);
        assert_eq!(p.strategy.as_deref(), Some("sequential"));
    }

    // ── Vision/LLM Observation Layer ──

    #[test]
    fn vision_describe_params_defaults() {
        let p: VisionDescribeParams = serde_json::from_str(r#"{}"#).unwrap();
        assert!(p.selector.is_none());
        assert!(p.format.is_none());
    }

    #[test]
    fn vision_describe_params_full() {
        let p: VisionDescribeParams =
            serde_json::from_str(r#"{"selector":"main","format":"detailed"}"#).unwrap();
        assert_eq!(p.selector.as_deref(), Some("main"));
        assert_eq!(p.format.as_deref(), Some("detailed"));
    }

    #[test]
    fn vision_locate_params() {
        let p: VisionLocateParams = serde_json::from_str(
            r#"{"description":"blue submit button","strategy":"aria"}"#,
        )
        .unwrap();
        assert_eq!(p.description, "blue submit button");
        assert_eq!(p.strategy.as_deref(), Some("aria"));
    }

    #[test]
    fn vision_compare_params() {
        let p: VisionCompareParams =
            serde_json::from_str(r#"{"baseline":"base64data","threshold":0.95}"#).unwrap();
        assert_eq!(p.baseline, "base64data");
        assert!(p.current.is_none());
        assert_eq!(p.threshold, Some(0.95));
    }

    // ── Self-Healing Selector Recovery ──

    #[test]
    fn selector_heal_params() {
        let p: SelectorHealParams =
            serde_json::from_str(r#"{"selector":".btn-login","context":"login button"}"#)
                .unwrap();
        assert_eq!(p.selector, ".btn-login");
        assert_eq!(p.context.as_deref(), Some("login button"));
    }

    #[test]
    fn selector_alternatives_params() {
        let p: SelectorAlternativesParams =
            serde_json::from_str(r##"{"selector":"#main","max_alternatives":5}"##).unwrap();
        assert_eq!(p.selector, "#main");
        assert_eq!(p.max_alternatives, Some(5));
    }

    #[test]
    fn selector_validate_params() {
        let p: SelectorValidateParams = serde_json::from_str(
            r#"{"selector":"button","expected_role":"button","expected_text":"Submit"}"#,
        )
        .unwrap();
        assert_eq!(p.selector, "button");
        assert_eq!(p.expected_role.as_deref(), Some("button"));
        assert_eq!(p.expected_text.as_deref(), Some("Submit"));
    }

    // ── Session Checkpoints/Resume ──

    #[test]
    fn checkpoint_save_params_minimal() {
        let p: CheckpointSaveParams =
            serde_json::from_str(r#"{"name":"before-login"}"#).unwrap();
        assert_eq!(p.name, "before-login");
        assert!(p.include_cookies.is_none());
        assert!(p.include_storage.is_none());
        assert!(p.include_context.is_none());
    }

    #[test]
    fn checkpoint_save_params_full() {
        let p: CheckpointSaveParams = serde_json::from_str(
            r#"{"name":"after-auth","include_cookies":true,"include_storage":true,"include_context":true}"#,
        )
        .unwrap();
        assert_eq!(p.name, "after-auth");
        assert_eq!(p.include_cookies, Some(true));
        assert_eq!(p.include_storage, Some(true));
        assert_eq!(p.include_context, Some(true));
    }

    #[test]
    fn checkpoint_restore_params() {
        let p: CheckpointRestoreParams = serde_json::from_str(
            r#"{"name":"before-login","restore_url":true,"restore_cookies":false}"#,
        )
        .unwrap();
        assert_eq!(p.name, "before-login");
        assert_eq!(p.restore_url, Some(true));
        assert_eq!(p.restore_cookies, Some(false));
    }

    #[test]
    fn checkpoint_delete_params() {
        let p: CheckpointDeleteParams =
            serde_json::from_str(r#"{"name":"old-checkpoint"}"#).unwrap();
        assert_eq!(p.name, "old-checkpoint");
    }

    // ── Extended Workflow DSL ──

    #[test]
    fn workflow_while_params() {
        let p: WorkflowWhileParams = serde_json::from_str(
            r#"{"condition":"document.querySelector('.next')","actions":[{"action":"click","params":{}}],"max_iterations":10}"#,
        )
        .unwrap();
        assert_eq!(p.condition, "document.querySelector('.next')");
        assert_eq!(p.actions.len(), 1);
        assert_eq!(p.max_iterations, Some(10));
    }

    #[test]
    fn workflow_for_each_params() {
        let p: WorkflowForEachParams = serde_json::from_str(
            r#"{"collection":"document.querySelectorAll('a')","variable_name":"link","actions":[{"action":"click"}]}"#,
        )
        .unwrap();
        assert_eq!(p.collection, "document.querySelectorAll('a')");
        assert_eq!(p.variable_name.as_deref(), Some("link"));
        assert_eq!(p.actions.len(), 1);
    }

    #[test]
    fn workflow_if_params_then_only() {
        let p: WorkflowIfParams = serde_json::from_str(
            r#"{"condition":"true","then_actions":[{"action":"click"}]}"#,
        )
        .unwrap();
        assert_eq!(p.condition, "true");
        assert_eq!(p.then_actions.len(), 1);
        assert!(p.else_actions.is_none());
    }

    #[test]
    fn workflow_if_params_full() {
        let p: WorkflowIfParams = serde_json::from_str(
            r#"{"condition":"false","then_actions":[{"action":"click"}],"else_actions":[{"action":"wait"}]}"#,
        )
        .unwrap();
        assert_eq!(p.condition, "false");
        assert_eq!(p.then_actions.len(), 1);
        assert!(p.else_actions.is_some());
        assert_eq!(p.else_actions.unwrap().len(), 1);
    }

    #[test]
    fn workflow_variable_set() {
        let p: WorkflowVariableParams =
            serde_json::from_str(r#"{"name":"counter","value":42}"#).unwrap();
        assert_eq!(p.name, "counter");
        assert!(p.value.is_some());
    }

    #[test]
    fn workflow_variable_get() {
        let p: WorkflowVariableParams =
            serde_json::from_str(r#"{"name":"counter"}"#).unwrap();
        assert_eq!(p.name, "counter");
        assert!(p.value.is_none());
    }

    // ── Event-Driven Reaction System ──

    #[test]
    fn event_subscribe_params() {
        let p: EventSubscribeParams =
            serde_json::from_str(r#"{"event_type":"navigation","filter":"*.html"}"#).unwrap();
        assert_eq!(p.event_type, "navigation");
        assert_eq!(p.filter.as_deref(), Some("*.html"));
    }

    #[test]
    fn event_unsubscribe_params() {
        let p: EventUnsubscribeParams =
            serde_json::from_str(r#"{"event_type":"console"}"#).unwrap();
        assert_eq!(p.event_type, "console");
    }

    #[test]
    fn event_poll_params_defaults() {
        let p: EventPollParams = serde_json::from_str(r#"{}"#).unwrap();
        assert!(p.event_type.is_none());
        assert!(p.limit.is_none());
        assert!(p.clear.is_none());
    }

    #[test]
    fn event_poll_params_full() {
        let p: EventPollParams =
            serde_json::from_str(r#"{"event_type":"error","limit":10,"clear":true}"#).unwrap();
        assert_eq!(p.event_type.as_deref(), Some("error"));
        assert_eq!(p.limit, Some(10));
        assert_eq!(p.clear, Some(true));
    }

    // ── BrowserState agentic defaults ──

    #[test]
    fn browser_state_agentic_defaults() {
        let state = BrowserState::default();
        assert!(state.task_plans.is_empty());
        assert!(state.selector_cache.is_empty());
        assert!(state.checkpoints.is_empty());
        assert!(state.workflow_variables.is_empty());
        assert!(state.event_subscriptions.is_empty());
        assert!(state.event_buffer.is_empty());
    }
}

#[derive(Debug, Deserialize, rmcp::schemars::JsonSchema)]
pub struct SpaNavWatchParams {
    /// "start" to begin watching, "poll" to get recorded navigations, "stop" to stop
    pub command: String,
    /// Clear buffer after polling (default: true)
    pub clear: Option<bool>,
}

#[derive(Debug, Deserialize, rmcp::schemars::JsonSchema)]
pub struct FrameworkDetectParams {
    /// Include router info if detected (default: true)
    pub include_router: Option<bool>,
}

#[derive(Debug, Deserialize, rmcp::schemars::JsonSchema)]
pub struct CanvasAdvancedParams {
    /// Noise intensity 0.0-10.0 (default: 2.0). Higher = more noise, harder to fingerprint but may affect canvas rendering.
    pub intensity: Option<f64>,
}

#[derive(Debug, Deserialize, rmcp::schemars::JsonSchema)]
pub struct TimezoneSyncParams {
    /// IANA timezone to spoof, e.g. "America/New_York", "Europe/London", "Asia/Tokyo"
    pub timezone: String,
}

#[derive(Debug, Deserialize, rmcp::schemars::JsonSchema)]
pub struct FontProtectParams {
    /// Additional fonts to allow beyond the default cross-platform set
    pub allow_extra: Option<Vec<String>>,
}

#[derive(Debug, Deserialize, rmcp::schemars::JsonSchema)]
pub struct BehaviorSimParams {
    /// Interval in ms between micro-movements (default: 2000)
    pub interval_ms: Option<u64>,
    /// "start" or "stop" (default: "start")
    pub command: Option<String>,
}

#[derive(Debug, Deserialize, rmcp::schemars::JsonSchema)]
pub struct StealthRotateParams {
    /// Rotate on every new page/navigation (default: false, rotate only on new domain)
    pub per_page: Option<bool>,
}

#[derive(Debug, Deserialize, rmcp::schemars::JsonSchema)]
pub struct DetectionAuditParams {
    /// Include detailed per-test results (default: true)
    pub detailed: Option<bool>,
}

#[derive(Debug, Deserialize, rmcp::schemars::JsonSchema)]
pub struct VirtualScrollDetectParams {
    /// Optional CSS selector to check specific container (default: auto-detect)
    pub container: Option<String>,
}

#[derive(Debug, Deserialize, rmcp::schemars::JsonSchema)]
pub struct VirtualScrollExtractParams {
    /// CSS selector of the scroll container
    pub container: String,
    /// CSS selector of individual items within the container
    pub item_selector: String,
    /// Maximum items to extract (default: 1000)
    pub max_items: Option<usize>,
}

#[derive(Debug, Deserialize, rmcp::schemars::JsonSchema)]
pub struct WaitHydrationParams {
    /// Timeout in ms (default: 10000)
    pub timeout_ms: Option<u64>,
}

#[derive(Debug, Deserialize, rmcp::schemars::JsonSchema)]
pub struct WaitAnimationParams {
    /// CSS selector of the element to watch for animations
    pub selector: String,
    /// Timeout in ms (default: 5000)
    pub timeout_ms: Option<u64>,
}

#[derive(Debug, Deserialize, rmcp::schemars::JsonSchema)]
pub struct WaitNetworkIdleParams {
    /// How long network must be idle in ms (default: 500)
    pub idle_ms: Option<u64>,
    /// Overall timeout in ms (default: 30000)
    pub timeout_ms: Option<u64>,
}

#[derive(Debug, Deserialize, rmcp::schemars::JsonSchema)]
pub struct TriggerLazyLoadParams {
    /// CSS selector for lazy-loaded elements (default: "img[data-src], img[loading='lazy'], [data-lazy]")
    pub selector: Option<String>,
}

#[derive(Debug, Deserialize, rmcp::schemars::JsonSchema)]
pub struct HealthCheckParams {
    /// Include memory usage details (default: true)
    pub include_memory: Option<bool>,
}

#[derive(Debug, Deserialize, rmcp::schemars::JsonSchema)]
pub struct CircuitBreakerParams {
    /// Command: "status", "record_success", "record_failure", "reset"
    pub command: String,
    /// Error message (only for record_failure)
    pub error: Option<String>,
    /// Failure threshold before circuit opens (default: 5)
    pub threshold: Option<u32>,
}

// ──────────────── Agent loop params ─────────────────

#[derive(Debug, Deserialize, rmcp::schemars::JsonSchema)]
pub struct AgentLoopParams {
    /// The goal to achieve (natural language description)
    pub goal: String,
    /// Maximum observation steps before stopping (default: 10)
    pub max_steps: Option<usize>,
    /// Optional JavaScript expression that returns "true" when goal is met
    pub verify_js: Option<String>,
}

#[derive(Debug, Deserialize, rmcp::schemars::JsonSchema)]
pub struct GoalAssertParams {
    /// List of assertions to check. Each has a "type" and "value".
    /// Types: url_contains, url_equals, title_contains, title_equals, element_exists, text_contains, element_visible
    pub assertions: Vec<GoalAssertion>,
}

#[derive(Debug, Deserialize, rmcp::schemars::JsonSchema)]
pub struct GoalAssertion {
    /// Assertion type: url_contains, url_equals, title_contains, title_equals, element_exists, text_contains, element_visible
    #[serde(rename = "type")]
    pub assertion_type: String,
    /// Value to check against
    pub value: String,
}

#[derive(Debug, Deserialize, rmcp::schemars::JsonSchema)]
pub struct AnnotatedObserveParams {
    /// Optional CSS selector to scope observation (default: full page)
    pub scope: Option<String>,
}

#[derive(Debug, Deserialize, rmcp::schemars::JsonSchema)]
pub struct AnnotatedScreenshotParams {
    /// Whether to include full page (default: false, viewport only)
    pub full_page: Option<bool>,
}

#[derive(Debug, Deserialize, rmcp::schemars::JsonSchema)]
pub struct AdaptiveRetryParams {
    /// Primary JavaScript action to try
    pub action_js: String,
    /// Alternative JavaScript strategies to try if primary fails
    pub alternatives: Vec<String>,
    /// Maximum retries (default: 3)
    pub max_retries: Option<usize>,
}

// ──────────────── Long-running harness params ─────────────────

#[derive(Debug, Deserialize, rmcp::schemars::JsonSchema)]
pub struct ReconnectCdpParams {
    /// Max reconnection attempts (default: 5)
    pub max_retries: Option<usize>,
}

#[derive(Debug, Deserialize, rmcp::schemars::JsonSchema)]
pub struct GcTabsParams {
    /// Maximum number of tabs to keep (default: 10)
    pub max_count: Option<usize>,
}

#[derive(Debug, Deserialize, rmcp::schemars::JsonSchema)]
pub struct WatchdogParams {
    /// Include memory details (default: true)
    pub include_memory: Option<bool>,
}

#[derive(Debug, Deserialize, rmcp::schemars::JsonSchema)]
pub struct StateInspectParams {
    /// Optional dot-separated path into store state (e.g., "user.profile")
    pub path: Option<String>,
}

#[derive(Debug, Deserialize, rmcp::schemars::JsonSchema)]
pub struct FormWizardTrackParams {
    /// Optional form index to inspect (default: all forms)
    pub form_index: Option<usize>,
}

#[derive(Debug, Deserialize, rmcp::schemars::JsonSchema)]
pub struct DynamicImportWaitParams {
    /// Pattern to match against loaded script URLs
    pub module_pattern: String,
    /// Timeout in ms (default: 10000)
    pub timeout_ms: Option<u64>,
}

#[derive(Debug, Deserialize, rmcp::schemars::JsonSchema)]
pub struct ParallelExecParams {
    /// List of JavaScript expressions to execute in parallel
    pub actions: Vec<String>,
}

// ──────────────── Session context / auto-chain / think params ──────────────

#[derive(Debug, Deserialize, rmcp::schemars::JsonSchema)]
pub struct SessionContextParams {
    /// Command: "set", "get", "get_all", "clear"
    pub command: String,
    /// Key for set/get operations
    pub key: Option<String>,
    /// Value for set operation
    pub value: Option<String>,
}

#[derive(Debug, Deserialize, rmcp::schemars::JsonSchema)]
pub struct AutoChainParams {
    /// List of JavaScript expressions to execute in sequence
    pub actions: Vec<String>,
    /// Error handling: "retry", "skip", "abort" (default: "skip")
    pub on_error: Option<String>,
    /// Max retries per action (default: 2)
    pub max_retries: Option<usize>,
}

#[derive(Debug, Deserialize, rmcp::schemars::JsonSchema)]
pub struct ThinkParams {
    /// Optional context to consider (ignored, for LLM prompt chaining)
    pub context: Option<String>,
}

// ──────────────── Coordinate click / multi-page sync / input replay params ──

#[derive(Debug, Deserialize, rmcp::schemars::JsonSchema)]
pub struct ClickAtCoordsParams {
    /// X coordinate (viewport pixels)
    pub x: f64,
    /// Y coordinate (viewport pixels)
    pub y: f64,
}

#[derive(Debug, Deserialize, rmcp::schemars::JsonSchema)]
pub struct MultiPageSyncParams {
    /// Optional: specific tab indices to query (default: all)
    pub tab_indices: Option<Vec<usize>>,
}

#[derive(Debug, Deserialize, rmcp::schemars::JsonSchema)]
pub struct InputReplayParams {
    /// Sequence of input events. Each event has "type" (click/type/scroll/wait) and params.
    pub events: Vec<serde_json::Value>,
}

// ──────────────── Enhanced agentic API params ──────────────────────────────

#[derive(Debug, Deserialize, rmcp::schemars::JsonSchema)]
pub struct TokenBudgetParams {
    /// Maximum token budget (approx 4 chars per token). Defaults to 4000.
    pub max_tokens: Option<usize>,
    /// Optional CSS selector to extract content from.
    pub selector: Option<String>,
}

#[derive(Debug, Deserialize, rmcp::schemars::JsonSchema)]
pub struct CompactStateParams {}

#[derive(Debug, Deserialize, rmcp::schemars::JsonSchema)]
pub struct PlanExecuteParams {
    /// JavaScript expressions to execute in sequence.
    pub steps: Vec<String>,
    /// Stop execution on first error. Defaults to true.
    pub stop_on_error: Option<bool>,
}

#[derive(Debug, Deserialize, rmcp::schemars::JsonSchema)]
pub struct PageSummaryParams {}

#[derive(Debug, Deserialize, rmcp::schemars::JsonSchema)]
pub struct ErrorContextParams {}

#[derive(Debug, Deserialize, rmcp::schemars::JsonSchema)]
pub struct ElementInfoParams {
    /// CSS selector of the element to inspect.
    pub selector: String,
}

#[derive(Debug, Deserialize, rmcp::schemars::JsonSchema)]
pub struct BatchExecuteParams {
    /// JavaScript commands to execute in sequence.
    pub commands: Vec<String>,
    /// Stop execution on first error. Defaults to false.
    pub stop_on_error: Option<bool>,
}

#[derive(Debug, Deserialize, rmcp::schemars::JsonSchema)]
pub struct PageAssertionsParams {
    /// List of assertions to check.
    pub assertions: Vec<AssertionCheck>,
}

#[derive(Debug, Deserialize, rmcp::schemars::JsonSchema)]
pub struct AssertionCheck {
    /// Type of check: url_contains, title_contains, element_exists, element_visible, text_contains.
    pub check_type: String,
    /// Expected value for the check.
    pub expected: String,
}

#[derive(Debug, Deserialize, rmcp::schemars::JsonSchema)]
pub struct ExtractCompactParams {
    /// Output format: "text" (default) or "markdown".
    pub format: Option<String>,
    /// Maximum token budget (approx 4 chars per token). Defaults to 8000.
    pub max_tokens: Option<usize>,
}

#[derive(Debug, Deserialize, rmcp::schemars::JsonSchema)]
pub struct StealthStatusParams {}

// ──────────────── Durable Session params ─────────────────

#[derive(Debug, Deserialize, rmcp::schemars::JsonSchema)]
pub struct DurableStartParams {
    #[schemars(description = "Unique name for the durable session")]
    pub name: String,
    #[schemars(description = "Checkpoint interval in seconds (default 30)")]
    pub checkpoint_interval_secs: Option<u64>,
    #[schemars(description = "State directory path (default ~/.onecrawl/states/)")]
    pub state_path: Option<String>,
    #[schemars(description = "Enable auto-reconnect on crash (default true)")]
    pub auto_reconnect: Option<bool>,
    #[schemars(description = "Maximum reconnect attempts (default 10)")]
    pub max_reconnect_attempts: Option<u32>,
    #[schemars(description = "Crash policy: restart, stop, or notify (default restart)")]
    pub on_crash: Option<String>,
    #[schemars(description = "Maximum uptime in seconds (omit for unlimited)")]
    pub max_uptime_secs: Option<u64>,
    #[schemars(description = "Persist auth state (cookies, storage) (default true)")]
    pub persist_auth: Option<bool>,
}

#[derive(Debug, Deserialize, rmcp::schemars::JsonSchema)]
pub struct DurableStopParams {
    #[schemars(description = "Name of the durable session to stop")]
    pub name: String,
}

#[derive(Debug, Deserialize, rmcp::schemars::JsonSchema)]
pub struct DurableCheckpointParams {
    #[schemars(description = "Name of the durable session to checkpoint")]
    pub name: String,
}

#[derive(Debug, Deserialize, rmcp::schemars::JsonSchema)]
pub struct DurableRestoreParams {
    #[schemars(description = "Name of the durable session to restore")]
    pub name: String,
}

#[derive(Debug, Deserialize, rmcp::schemars::JsonSchema)]
pub struct DurableStatusParams {
    #[schemars(description = "Name of the durable session (omit to get default)")]
    pub name: Option<String>,
}

#[derive(Debug, Deserialize, rmcp::schemars::JsonSchema)]
pub struct DurableListParams {}

#[derive(Debug, Deserialize, rmcp::schemars::JsonSchema)]
pub struct DurableDeleteParams {
    #[schemars(description = "Name of the durable session to delete")]
    pub name: String,
}

#[derive(Debug, Deserialize, rmcp::schemars::JsonSchema)]
pub struct DurableConfigParams {
    #[schemars(description = "Name of the durable session")]
    pub name: String,
    #[schemars(description = "New checkpoint interval in seconds")]
    pub checkpoint_interval_secs: Option<u64>,
    #[schemars(description = "Enable/disable auto-reconnect")]
    pub auto_reconnect: Option<bool>,
    #[schemars(description = "New crash policy: restart, stop, or notify")]
    pub on_crash: Option<String>,
    #[schemars(description = "New max uptime in seconds (null for unlimited)")]
    pub max_uptime_secs: Option<u64>,
}

// ──────────────── Event Reactor params ─────────────────

#[derive(Debug, Deserialize, rmcp::schemars::JsonSchema)]
pub struct ReactorRuleParam {
    #[schemars(description = "Unique rule ID")]
    pub id: String,
    #[schemars(description = "Event type: dom_mutation, network_request, network_response, console, page_error, navigation, notification, websocket, timer, or custom name")]
    pub event_type: String,
    #[schemars(description = "Optional filter to narrow events")]
    pub filter: Option<ReactorFilterParam>,
    #[schemars(description = "Handler configuration (JSON with 'type' field: log, evaluate, webhook, screenshot, ai_respond, chain, store, command)")]
    pub handler: serde_json::Value,
    #[schemars(description = "Whether this rule is enabled (default true)")]
    pub enabled: Option<bool>,
    #[schemars(description = "Maximum number of triggers (null for unlimited)")]
    pub max_triggers: Option<u64>,
    #[schemars(description = "Cooldown between triggers in ms")]
    pub cooldown_ms: Option<u64>,
}

#[derive(Debug, Deserialize, rmcp::schemars::JsonSchema)]
pub struct ReactorFilterParam {
    #[schemars(description = "CSS selector for DOM mutations")]
    pub selector: Option<String>,
    #[schemars(description = "Glob pattern for network events")]
    pub url_pattern: Option<String>,
    #[schemars(description = "Substring for console/notification content")]
    pub message_pattern: Option<String>,
    #[schemars(description = "Event subtype filter (e.g. 'error' for console errors)")]
    pub event_subtype: Option<String>,
}

#[derive(Debug, Deserialize, rmcp::schemars::JsonSchema)]
pub struct ReactorStartParams {
    #[schemars(description = "Reactor name (default 'default')")]
    pub name: Option<String>,
    #[schemars(description = "Array of reactor rules")]
    pub rules: Vec<ReactorRuleParam>,
    #[schemars(description = "Max events per minute rate limit (default 60)")]
    pub max_events_per_minute: Option<u32>,
    #[schemars(description = "Event buffer size (default 1000, max 10000)")]
    pub buffer_size: Option<usize>,
    #[schemars(description = "Persist events to disk")]
    pub persist_events: Option<bool>,
    #[schemars(description = "Path for event log file")]
    pub event_log_path: Option<String>,
}

#[derive(Debug, Deserialize, rmcp::schemars::JsonSchema)]
pub struct ReactorStopParams {
    #[schemars(description = "Reactor name (default 'default')")]
    pub name: Option<String>,
}

#[derive(Debug, Deserialize, rmcp::schemars::JsonSchema)]
pub struct ReactorStatusParams {
    #[schemars(description = "Reactor name (default 'default')")]
    pub name: Option<String>,
}

#[derive(Debug, Deserialize, rmcp::schemars::JsonSchema)]
pub struct ReactorAddRuleParams {
    #[schemars(description = "Unique rule ID")]
    pub id: String,
    #[schemars(description = "Event type to react to")]
    pub event_type: String,
    #[schemars(description = "Optional filter")]
    pub filter: Option<ReactorFilterParam>,
    #[schemars(description = "Handler configuration (JSON)")]
    pub handler: serde_json::Value,
    #[schemars(description = "Whether this rule is enabled")]
    pub enabled: Option<bool>,
    #[schemars(description = "Maximum triggers")]
    pub max_triggers: Option<u64>,
    #[schemars(description = "Cooldown in ms")]
    pub cooldown_ms: Option<u64>,
}

#[derive(Debug, Deserialize, rmcp::schemars::JsonSchema)]
pub struct ReactorRemoveRuleParams {
    #[schemars(description = "Rule ID to remove")]
    pub id: String,
}

#[derive(Debug, Deserialize, rmcp::schemars::JsonSchema)]
pub struct ReactorToggleRuleParams {
    #[schemars(description = "Rule ID to toggle")]
    pub id: String,
    #[schemars(description = "Enable or disable")]
    pub enabled: bool,
}

#[derive(Debug, Deserialize, rmcp::schemars::JsonSchema)]
pub struct ReactorEventsParams {
    #[schemars(description = "Max events to return (default 50)")]
    pub limit: Option<usize>,
}

#[derive(Debug, Deserialize, rmcp::schemars::JsonSchema)]
pub struct ReactorClearParams {}

// ────────────────────────────────────────────────────────────────────
//  Agent Auto params
// ────────────────────────────────────────────────────────────────────

#[derive(Debug, Deserialize, rmcp::schemars::JsonSchema)]
pub struct AgentAutoRunParams {
    #[schemars(description = "Natural language goal for the agent")]
    pub goal: String,
    #[schemars(description = "LLM model name (for caller reference)")]
    pub model: Option<String>,
    #[schemars(description = "Max steps (default 50)")]
    pub max_steps: Option<u32>,
    #[schemars(description = "Cost cap in cents (e.g. 50 = $0.50)")]
    pub max_cost_cents: Option<u32>,
    #[schemars(description = "Capture screenshot after each step")]
    pub screenshot_every_step: Option<bool>,
    #[schemars(description = "Directory for screenshots")]
    pub screenshot_dir: Option<String>,
    #[schemars(description = "Output file path (e.g. results.csv)")]
    pub output: Option<String>,
    #[schemars(description = "Output format: csv, json, jsonl")]
    pub output_format: Option<String>,
    #[schemars(description = "Save state file path for resume")]
    pub save_state: Option<String>,
    #[schemars(description = "Enable verbose logging")]
    pub verbose: Option<bool>,
    #[schemars(description = "Allowed domains (safety)")]
    pub allowed_domains: Option<Vec<String>>,
    #[schemars(description = "Blocked domains (safety)")]
    pub blocked_domains: Option<Vec<String>>,
    #[schemars(description = "Overall timeout in seconds")]
    pub timeout_secs: Option<u64>,
    #[schemars(description = "Use agent memory for learning (default true)")]
    pub use_memory: Option<bool>,
    #[schemars(description = "Path for memory persistence")]
    pub memory_path: Option<String>,
}

#[derive(Debug, Deserialize, rmcp::schemars::JsonSchema)]
pub struct AgentAutoPlanParams {
    #[schemars(description = "Natural language goal to plan")]
    pub goal: String,
    #[schemars(description = "Enable verbose output")]
    pub verbose: Option<bool>,
}

#[derive(Debug, Deserialize, rmcp::schemars::JsonSchema)]
pub struct AgentAutoStatusParams {}

#[derive(Debug, Deserialize, rmcp::schemars::JsonSchema)]
pub struct AgentAutoStopParams {
    #[schemars(description = "Path to save state for resume")]
    pub save_state: Option<String>,
}

#[derive(Debug, Deserialize, rmcp::schemars::JsonSchema)]
pub struct AgentAutoResumeParams {
    #[schemars(description = "Path to saved state file")]
    pub state_file: String,
    #[schemars(description = "Max additional steps")]
    pub max_steps: Option<u32>,
    #[schemars(description = "Cost cap in cents")]
    pub max_cost_cents: Option<u32>,
    #[schemars(description = "Enable verbose logging")]
    pub verbose: Option<bool>,
}

#[derive(Debug, Deserialize, rmcp::schemars::JsonSchema)]
pub struct AgentAutoResultParams {}

// ──────────────── Orchestrator params ─────────────────

#[derive(Debug, Deserialize, rmcp::schemars::JsonSchema)]
pub struct OrchestratorRunParams {
    #[schemars(description = "Path to orchestration JSON file")]
    pub file: Option<String>,
    #[schemars(description = "Inline orchestration JSON config")]
    pub config: Option<String>,
}

#[derive(Debug, Deserialize, rmcp::schemars::JsonSchema)]
pub struct OrchestratorValidateParams {
    #[schemars(description = "Path to orchestration JSON file")]
    pub file: Option<String>,
    #[schemars(description = "Inline orchestration JSON config")]
    pub config: Option<String>,
}

#[derive(Debug, Deserialize, rmcp::schemars::JsonSchema)]
pub struct OrchestratorStatusParams {}

#[derive(Debug, Deserialize, rmcp::schemars::JsonSchema)]
pub struct OrchestratorStopParams {}

#[derive(Debug, Deserialize, rmcp::schemars::JsonSchema)]
pub struct OrchestratorDevicesParams {}

// ──────────────── Vault params ─────────────────

#[derive(Debug, Deserialize, rmcp::schemars::JsonSchema)]
pub struct VaultCreateParams {
    #[schemars(description = "Path to the vault file (default ~/.onecrawl/vault.enc)")]
    pub path: Option<String>,
    #[schemars(description = "Master password for the vault")]
    pub password: String,
}

#[derive(Debug, Deserialize, rmcp::schemars::JsonSchema)]
pub struct VaultOpenParams {
    #[schemars(description = "Path to the vault file (default ~/.onecrawl/vault.enc)")]
    pub path: Option<String>,
    #[schemars(description = "Master password for the vault")]
    pub password: String,
}

#[derive(Debug, Deserialize, rmcp::schemars::JsonSchema)]
pub struct VaultSetParams {
    #[schemars(description = "Path to the vault file (default ~/.onecrawl/vault.enc)")]
    pub path: Option<String>,
    #[schemars(description = "Master password for the vault")]
    pub password: String,
    #[schemars(description = "Secret key (e.g. 'linkedin.email')")]
    pub key: String,
    #[schemars(description = "Secret value")]
    pub value: String,
    #[schemars(description = "Optional category/service (e.g. 'linkedin')")]
    pub category: Option<String>,
}

#[derive(Debug, Deserialize, rmcp::schemars::JsonSchema)]
pub struct VaultGetParams {
    #[schemars(description = "Path to the vault file (default ~/.onecrawl/vault.enc)")]
    pub path: Option<String>,
    #[schemars(description = "Master password for the vault")]
    pub password: String,
    #[schemars(description = "Secret key to retrieve")]
    pub key: String,
}

#[derive(Debug, Deserialize, rmcp::schemars::JsonSchema)]
pub struct VaultDeleteParams {
    #[schemars(description = "Path to the vault file (default ~/.onecrawl/vault.enc)")]
    pub path: Option<String>,
    #[schemars(description = "Master password for the vault")]
    pub password: String,
    #[schemars(description = "Secret key to delete")]
    pub key: String,
}

#[derive(Debug, Deserialize, rmcp::schemars::JsonSchema)]
pub struct VaultListParams {
    #[schemars(description = "Path to the vault file (default ~/.onecrawl/vault.enc)")]
    pub path: Option<String>,
    #[schemars(description = "Master password for the vault")]
    pub password: String,
    #[schemars(description = "Optional category filter")]
    pub category: Option<String>,
}

#[derive(Debug, Deserialize, rmcp::schemars::JsonSchema)]
pub struct VaultUseParams {
    #[schemars(description = "Path to the vault file (default ~/.onecrawl/vault.enc)")]
    pub path: Option<String>,
    #[schemars(description = "Master password for the vault")]
    pub password: String,
    #[schemars(description = "Service name to export (e.g. 'linkedin')")]
    pub service: String,
}

#[derive(Debug, Deserialize, rmcp::schemars::JsonSchema)]
pub struct VaultChangePasswordParams {
    #[schemars(description = "Path to the vault file (default ~/.onecrawl/vault.enc)")]
    pub path: Option<String>,
    #[schemars(description = "Current master password")]
    pub password: String,
    #[schemars(description = "New master password")]
    pub new_password: String,
}

#[derive(Debug, Deserialize, rmcp::schemars::JsonSchema)]
pub struct VaultImportEnvParams {
    #[schemars(description = "Path to the vault file (default ~/.onecrawl/vault.enc)")]
    pub path: Option<String>,
    #[schemars(description = "Master password for the vault")]
    pub password: String,
    #[schemars(description = "Environment variable prefix (default 'ONECRAWL_VAULT_')")]
    pub prefix: Option<String>,
}
