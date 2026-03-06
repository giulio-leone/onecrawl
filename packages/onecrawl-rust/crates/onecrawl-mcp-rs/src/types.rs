//! Request parameter types and response structs for MCP tools.

use rmcp::schemars;

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
