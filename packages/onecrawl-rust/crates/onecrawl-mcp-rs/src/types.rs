//! Request parameter types and response structs for MCP tools.

use rmcp::schemars;

// ──────────────────── Consolidated Tool Action ─────────────────────

fn default_empty_object() -> serde_json::Value {
    serde_json::Value::Object(serde_json::Map::new())
}

/// Universal action-based tool param used by all consolidated super-tools.
#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct ToolAction {
    #[schemars(description = "Action to perform. See tool description for available actions.")]
    pub action: String,

    #[schemars(description = "Parameters for the action as a JSON object.")]
    #[serde(default = "default_empty_object")]
    pub params: serde_json::Value,
}

// ──────────────────────────── Request Parameter Types ────────────────────────────

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct EncryptRequest {
    #[schemars(description = "Plaintext string to encrypt")]
    pub plaintext: String,
    #[schemars(description = "Password for key derivation")]
    pub password: String,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct DecryptRequest {
    #[schemars(description = "Base64-encoded ciphertext (salt + nonce + ciphertext)")]
    pub ciphertext: String,
    #[schemars(description = "Password for key derivation")]
    pub password: String,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct TotpRequest {
    #[schemars(description = "Base32-encoded TOTP secret")]
    pub secret: String,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct HtmlRequest {
    #[schemars(description = "Raw HTML string")]
    pub html: String,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct SelectorRequest {
    #[schemars(description = "Raw HTML string")]
    pub html: String,
    #[schemars(description = "CSS selector to query")]
    pub selector: String,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct StoreSetRequest {
    #[schemars(description = "Storage key")]
    pub key: String,
    #[schemars(description = "Value to store")]
    pub value: String,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct StoreGetRequest {
    #[schemars(description = "Storage key to retrieve")]
    pub key: String,
}

// ──────────────────── Screencast & Recording params ─────────────────────

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct ScreencastStartParams {
    #[schemars(description = "Image format: jpeg or png")]
    pub format: Option<String>,
    #[schemars(description = "Compression quality 0-100 (jpeg only)")]
    pub quality: Option<u32>,
    #[schemars(description = "Maximum width in pixels")]
    pub max_width: Option<u32>,
    #[schemars(description = "Maximum height in pixels")]
    pub max_height: Option<u32>,
    #[schemars(description = "Capture every N-th frame")]
    pub every_nth_frame: Option<u32>,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct ScreencastStopParams {}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct ScreencastFrameParams {
    #[schemars(description = "Image format: jpeg or png (default: jpeg)")]
    pub format: Option<String>,
    #[schemars(description = "Compression quality 0-100")]
    pub quality: Option<u32>,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct RecordingStartParams {
    #[schemars(description = "Output file path (e.g. recording.webm)")]
    pub output: Option<String>,
    #[schemars(description = "Frames per second")]
    pub fps: Option<u32>,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct RecordingStopParams {}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct RecordingStatusParams {}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct StreamCaptureParams {
    #[schemars(description = "Number of frames to capture (default: 10)")]
    pub count: Option<usize>,
    #[schemars(description = "Interval between frames in milliseconds (default: 200)")]
    pub interval_ms: Option<u64>,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct StreamToDiskParams {
    #[schemars(description = "Output directory for saved frames")]
    pub output_dir: Option<String>,
    #[schemars(description = "Number of frames to capture (default: 30)")]
    pub count: Option<usize>,
    #[schemars(description = "Interval between frames in milliseconds (default: 200)")]
    pub interval_ms: Option<u64>,
    #[schemars(description = "Image format: jpeg or png (default: jpeg)")]
    pub format: Option<String>,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct RecordingEncodeParams {
    #[schemars(description = "Directory containing frame_NNNN.jpg files")]
    pub frames_dir: Option<String>,
    #[schemars(description = "Output video file path")]
    pub output: Option<String>,
    #[schemars(description = "Frames per second (default: 5)")]
    pub fps: Option<u32>,
    #[schemars(description = "Video format: mp4, webm, or gif (default: mp4)")]
    pub format: Option<String>,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct RecordingCaptureParams {
    #[schemars(description = "Recording duration in seconds (default: 5)")]
    pub duration_secs: Option<u64>,
    #[schemars(description = "Frames per second (default: 5)")]
    pub fps: Option<u32>,
    #[schemars(description = "Output video file path")]
    pub output: Option<String>,
    #[schemars(description = "Video format: mp4, webm, or gif (default: mp4)")]
    pub format: Option<String>,
}

// ──────────────────────────── Response Types ────────────────────────────

#[derive(serde::Serialize)]
pub struct PkceResponse<'a> {
    pub code_verifier: &'a str,
    pub code_challenge: &'a str,
}

#[derive(serde::Serialize)]
pub struct LinkInfo {
    pub href: String,
    pub text: String,
    pub is_external: bool,
}

#[derive(serde::Serialize)]
pub struct CrawlResult2 {
    pub summary: onecrawl_cdp::spider::CrawlSummary,
    pub pages_crawled: usize,
}

#[derive(serde::Serialize)]
pub struct RobotsInfo {
    pub sitemaps: Vec<String>,
    pub crawl_delay: Option<f64>,
    pub path_allowed: Option<bool>,
}

#[derive(serde::Serialize)]
pub struct StealthInjectResult {
    pub patches_applied: usize,
    pub patches: Vec<String>,
}

#[derive(serde::Serialize)]
pub struct FingerprintResult<'a> {
    pub user_agent: &'a str,
    pub platform: &'a str,
}

#[derive(serde::Serialize)]
pub struct RateLimitResult {
    pub can_proceed: bool,
    pub stats: onecrawl_cdp::rate_limiter::RateLimitStats,
}

#[derive(serde::Serialize)]
pub struct RetryResult {
    pub id: String,
    pub queue_stats: onecrawl_cdp::retry_queue::QueueStats,
}

#[derive(serde::Serialize)]
pub struct RemovedResult {
    pub removed: bool,
}

// ──────────────────── Agent Memory params ─────────────────────

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct MemoryStoreParams {
    #[schemars(description = "Unique key for this memory entry")]
    pub key: String,
    #[schemars(description = "JSON value to store")]
    pub value: serde_json::Value,
    #[schemars(description = "Category: page_visit, element_pattern, domain_strategy, retry_knowledge, user_preference, selector_mapping, error_pattern, custom")]
    pub category: Option<String>,
    #[schemars(description = "Domain this memory is associated with (e.g. 'example.com')")]
    pub domain: Option<String>,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct MemoryRecallParams {
    #[schemars(description = "Key of the memory entry to recall")]
    pub key: String,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct MemorySearchParams {
    #[schemars(description = "Search query (matches against keys and values)")]
    pub query: String,
    #[schemars(description = "Filter by category")]
    pub category: Option<String>,
    #[schemars(description = "Filter by domain")]
    pub domain: Option<String>,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct MemoryForgetParams {
    #[schemars(description = "Key to forget, or domain to clear all memories for")]
    pub key: Option<String>,
    #[schemars(description = "Domain to clear all memories for")]
    pub domain: Option<String>,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct MemoryDomainStrategyParams {
    #[schemars(description = "Domain to store/recall strategy for")]
    pub domain: String,
    #[schemars(description = "Strategy data as JSON (omit to recall existing)")]
    pub strategy: Option<serde_json::Value>,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct MemoryStatsParams {}

// ──────────────────── Workflow DSL params ─────────────────────

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct WorkflowValidateParams {
    #[schemars(description = "Workflow definition as JSON string")]
    pub workflow: String,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct WorkflowRunParams {
    #[schemars(description = "Workflow definition as JSON string, or file path to workflow JSON")]
    pub workflow: String,
    #[schemars(description = "Override variables as key-value pairs")]
    pub variables: Option<std::collections::HashMap<String, serde_json::Value>>,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct WorkflowListParams {}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct WorkflowExecuteParams {
    #[schemars(description = "Workflow definition as JSON string, or file path to workflow JSON")]
    pub workflow: String,
    #[schemars(description = "Override variables as key-value pairs")]
    pub variables: Option<std::collections::HashMap<String, serde_json::Value>>,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct WorkflowStatusParams {}

// ──────────────────── Network Intelligence params ─────────────────────

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct NetIntelCaptureParams {
    #[schemars(description = "Duration in seconds to capture network traffic (default: 10)")]
    pub duration_seconds: Option<u64>,
    #[schemars(description = "Only capture API calls (exclude static assets)")]
    pub api_only: Option<bool>,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct NetIntelAnalyzeParams {
    #[schemars(description = "Network capture data (from net.capture output)")]
    pub capture: String,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct NetIntelSdkParams {
    #[schemars(description = "API schema JSON (from net.analyze output)")]
    pub schema: String,
    #[schemars(description = "Target language: typescript or python (default: typescript)")]
    pub language: Option<String>,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct NetIntelMockParams {
    #[schemars(description = "Captured endpoints JSON (from net.capture)")]
    pub endpoints: String,
    #[schemars(description = "Port for mock server (default: 3001)")]
    pub port: Option<u16>,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct NetIntelReplayParams {
    #[schemars(description = "Captured endpoints JSON (from net.capture)")]
    pub endpoints: String,
    #[schemars(description = "Name for the replay sequence")]
    pub name: Option<String>,
}

// ──────────────────── Visual Regression Testing params ─────────────────────

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct VrtRunParams {
    #[schemars(description = "VRT suite definition as JSON, or path to suite JSON file")]
    pub suite: String,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct VrtUpdateBaselineParams {
    #[schemars(description = "Test name to update baseline for")]
    pub test_name: String,
    #[schemars(description = "Baseline directory (default: .vrt/baselines)")]
    pub baseline_dir: Option<String>,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct VrtCompareParams {
    #[schemars(description = "URL to capture and compare against baseline")]
    pub url: String,
    #[schemars(description = "Test name for baseline lookup")]
    pub name: String,
    #[schemars(description = "Mismatch threshold percentage (default: 0.1)")]
    pub threshold: Option<f64>,
    #[schemars(description = "CSS selector for element screenshot")]
    pub selector: Option<String>,
    #[schemars(description = "Capture full scrollable page")]
    pub full_page: Option<bool>,
    #[schemars(description = "Baseline directory")]
    pub baseline_dir: Option<String>,
}

// ──────────────────── AI Task Planner params ─────────────────────

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct PlannerPlanParams {
    #[schemars(description = "Natural language goal (e.g. 'log into Gmail and check inbox')")]
    pub goal: String,
    #[schemars(description = "Additional context as key-value pairs (url, credentials, etc.)")]
    pub context: Option<std::collections::HashMap<String, String>>,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct PlannerExecuteParams {
    #[schemars(description = "Task plan JSON (from planner.plan output), or natural language goal")]
    pub plan: String,
    #[schemars(description = "Additional context/variables")]
    pub context: Option<std::collections::HashMap<String, String>>,
    #[schemars(description = "Maximum retries per step (default: 2)")]
    pub max_retries: Option<u32>,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct PlannerPatternsParams {}

// ──────────────────── Performance Monitor params ─────────────────────

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct PerfAuditParams {
    #[schemars(description = "URL to audit (navigates there first). If omitted, audits the current page.")]
    pub url: Option<String>,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct PerfBudgetCheckParams {
    #[schemars(description = "Budget definition as JSON")]
    pub budget: String,
    #[schemars(description = "URL to check (optional, uses current page if omitted)")]
    pub url: Option<String>,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct PerfCompareParams {
    #[schemars(description = "Baseline snapshot JSON (from perf.audit output)")]
    pub baseline: String,
    #[schemars(description = "Current snapshot JSON (from perf.audit output)")]
    pub current: String,
    #[schemars(description = "Regression threshold percentage (default: 10)")]
    pub threshold_pct: Option<f64>,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct PerfTraceParams {
    #[schemars(description = "URL to trace (navigates + measures full lifecycle)")]
    pub url: String,
    #[schemars(description = "Wait time in ms after load for late metrics (default: 3000)")]
    pub settle_ms: Option<u64>,
}

// ──────────────────── Pixel Diff params ─────────────────────

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct PixelDiffParams {
    #[schemars(description = "First image as base64-encoded PNG")]
    pub image_a: String,
    #[schemars(description = "Second image as base64-encoded PNG")]
    pub image_b: String,
    #[schemars(description = "Mismatch percentage threshold (default: 5.0)")]
    pub threshold: Option<f64>,
    #[schemars(description = "Generate a diff visualization image (default: true)")]
    pub generate_diff: Option<bool>,
}
