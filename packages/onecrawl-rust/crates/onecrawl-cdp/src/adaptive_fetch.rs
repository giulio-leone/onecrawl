//! Adaptive HTTP Fetch — Chrome-like TLS/headers with automatic CDP escalation.
//!
//! Strategy:
//! 1. Try fast HTTP request with Chrome-like headers (reqwest)
//! 2. On 403/429/503 or Cloudflare challenge → escalate to CDP (browser fetch)
//! 3. Optional retry with exponential backoff + jitter
//!
//! The standalone HTTP client mimics Chrome's header ordering and values to avoid
//! server-side fingerprinting that detects non-browser clients.

use chromiumoxide::Page;
use onecrawl_core::{Error, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::Duration;

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

/// Configuration for adaptive fetch behavior.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AdaptiveFetchConfig {
    /// Maximum number of retries before giving up.
    pub max_retries: u32,
    /// Base delay in milliseconds for exponential backoff.
    pub base_delay_ms: u64,
    /// Whether to escalate to CDP on anti-bot responses.
    pub escalate_to_cdp: bool,
    /// Request timeout in milliseconds.
    pub timeout_ms: u64,
    /// Custom User-Agent override (None = use Chrome-like default).
    pub user_agent: Option<String>,
    /// Custom headers to merge with Chrome defaults.
    pub extra_headers: HashMap<String, String>,
}

impl Default for AdaptiveFetchConfig {
    fn default() -> Self {
        Self {
            max_retries: 3,
            base_delay_ms: 1000,
            escalate_to_cdp: true,
            timeout_ms: 30000,
            user_agent: None,
            extra_headers: HashMap::new(),
        }
    }
}

/// Result of an adaptive fetch attempt.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AdaptiveFetchResult {
    pub status: u16,
    pub headers: HashMap<String, String>,
    pub body: String,
    pub url: String,
    /// Which method succeeded: `"http"` or `"cdp"`.
    pub method: String,
    /// Number of attempts before success.
    pub attempts: u32,
    /// Total duration in milliseconds.
    pub duration_ms: f64,
    /// Whether the response was a Cloudflare challenge that required CDP.
    pub was_escalated: bool,
}

// ---------------------------------------------------------------------------
// Chrome-like header profiles
// ---------------------------------------------------------------------------

/// Build Chrome-like request headers.
///
/// Header ordering matters for TLS fingerprinting. Chrome sends headers in a
/// specific order that differs from curl/wget/reqwest defaults.
fn chrome_headers(url: &str, ua: Option<&str>) -> Vec<(String, String)> {
    let default_ua = "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) \
        AppleWebKit/537.36 (KHTML, like Gecko) Chrome/131.0.0.0 Safari/537.36";

    let host = reqwest::Url::parse(url)
        .ok()
        .and_then(|u| u.host_str().map(String::from))
        .unwrap_or_default();

    // Chrome's actual header order (verified via Wireshark)
    vec![
        ("Host".into(), host),
        (
            "sec-ch-ua".into(),
            r#""Google Chrome";v="131", "Chromium";v="131", "Not_A Brand";v="24""#.into(),
        ),
        ("sec-ch-ua-mobile".into(), "?0".into()),
        ("sec-ch-ua-platform".into(), r#""macOS""#.into()),
        ("Upgrade-Insecure-Requests".into(), "1".into()),
        (
            "User-Agent".into(),
            ua.unwrap_or(default_ua).to_string(),
        ),
        (
            "Accept".into(),
            "text/html,application/xhtml+xml,application/xml;q=0.9,\
             image/avif,image/webp,image/apng,*/*;q=0.8,\
             application/signed-exchange;v=b3;q=0.7"
                .into(),
        ),
        ("Sec-Fetch-Site".into(), "none".into()),
        ("Sec-Fetch-Mode".into(), "navigate".into()),
        ("Sec-Fetch-User".into(), "?1".into()),
        ("Sec-Fetch-Dest".into(), "document".into()),
        (
            "Accept-Encoding".into(),
            "gzip, deflate, br".into(),
        ),
        ("Accept-Language".into(), "it-IT,it;q=0.9,en-US;q=0.8,en;q=0.7".into()),
        ("Connection".into(), "keep-alive".into()),
    ]
}

/// Rotate User-Agent between common Chrome versions for retry diversity.
fn rotate_user_agent(attempt: u32) -> &'static str {
    const AGENTS: &[&str] = &[
        "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/131.0.0.0 Safari/537.36",
        "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/131.0.0.0 Safari/537.36",
        "Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/130.0.0.0 Safari/537.36",
        "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/130.0.0.0 Safari/537.36",
    ];
    AGENTS[(attempt as usize) % AGENTS.len()]
}

// ---------------------------------------------------------------------------
// Anti-bot detection
// ---------------------------------------------------------------------------

/// Check if an HTTP response indicates anti-bot protection.
fn is_antibot_response(status: u16, body: &str) -> bool {
    // Cloudflare challenge pages
    if status == 403 || status == 503 {
        let cf_markers = [
            "cf-browser-verification",
            "cf_chl_opt",
            "Just a moment",
            "Checking your browser",
            "Verifying you are human",
            "challenge-platform",
            "__cf_chl_tk",
            "ray ID",
        ];
        if cf_markers.iter().any(|m| body.contains(m)) {
            return true;
        }
    }

    // Rate limiting
    if status == 429 {
        return true;
    }

    // Generic WAF blocks
    if status == 403 {
        let waf_markers = [
            "Access Denied",
            "Request blocked",
            "security check",
            "bot detected",
            "automated access",
        ];
        if waf_markers.iter().any(|m| body.to_lowercase().contains(&m.to_lowercase())) {
            return true;
        }
    }

    false
}

/// Check if the body looks like a Cloudflare challenge (needs browser JS).
fn is_cf_challenge(body: &str) -> bool {
    body.contains("cf-browser-verification")
        || body.contains("cf_chl_opt")
        || (body.contains("Just a moment") && body.contains("cloudflare"))
}

// ---------------------------------------------------------------------------
// Core fetch implementations
// ---------------------------------------------------------------------------

/// Perform a standalone HTTP GET with Chrome-like headers (no browser needed).
async fn http_get(
    url: &str,
    config: &AdaptiveFetchConfig,
    attempt: u32,
) -> std::result::Result<(u16, HashMap<String, String>, String, String), String> {
    let ua = config
        .user_agent
        .as_deref()
        .unwrap_or_else(|| rotate_user_agent(attempt));

    let client = reqwest::Client::builder()
        .timeout(Duration::from_millis(config.timeout_ms))
        .redirect(reqwest::redirect::Policy::limited(10))
        .gzip(true)
        .build()
        .map_err(|e| format!("client build: {e}"))?;

    let mut req = client.get(url);

    // Apply Chrome-like headers in order
    for (key, val) in chrome_headers(url, Some(ua)) {
        // Skip Host — reqwest sets it automatically
        if key == "Host" {
            continue;
        }
        req = req.header(&key, &val);
    }

    // Merge extra headers
    for (key, val) in &config.extra_headers {
        req = req.header(key.as_str(), val.as_str());
    }

    let resp = req.send().await.map_err(|e| format!("request: {e}"))?;
    let status = resp.status().as_u16();
    let final_url = resp.url().to_string();

    let mut headers = HashMap::new();
    for (k, v) in resp.headers() {
        if let Ok(val) = v.to_str() {
            headers.insert(k.to_string(), val.to_string());
        }
    }

    let body = resp.text().await.map_err(|e| format!("body: {e}"))?;

    Ok((status, headers, body, final_url))
}

/// Perform a CDP-based fetch through the browser (inherits session/cookies/TLS).
async fn cdp_get(page: &Page, url: &str) -> Result<(u16, HashMap<String, String>, String, String)> {
    let resp = crate::http_client::get(page, url, None).await?;
    Ok((resp.status, resp.headers, resp.body, resp.url))
}

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/// Adaptive GET — tries HTTP first, escalates to CDP on anti-bot response.
///
/// This is the primary entry point for fetching pages adaptively.
pub async fn adaptive_get(
    page: &Page,
    url: &str,
    config: Option<AdaptiveFetchConfig>,
) -> Result<AdaptiveFetchResult> {
    let cfg = config.unwrap_or_default();
    let start = std::time::Instant::now();
    let mut last_error = String::new();

    // Phase 1: Try standalone HTTP with Chrome-like headers
    for attempt in 0..=cfg.max_retries {
        match http_get(url, &cfg, attempt).await {
            Ok((status, headers, body, final_url)) => {
                if is_antibot_response(status, &body) {
                    tracing::info!(
                        attempt,
                        status,
                        "anti-bot response detected, {}",
                        if cfg.escalate_to_cdp {
                            "escalating to CDP"
                        } else {
                            "retrying"
                        }
                    );

                    // If it's a CF challenge and CDP escalation is enabled, go to Phase 2
                    if cfg.escalate_to_cdp && is_cf_challenge(&body) {
                        break;
                    }

                    // For rate limiting (429), wait with backoff
                    if status == 429 {
                        let delay = cfg.base_delay_ms * 2u64.pow(attempt);
                        let jitter = rand::random::<u64>() % (delay / 2 + 1);
                        tokio::time::sleep(Duration::from_millis(delay + jitter)).await;
                        continue;
                    }

                    // For 403/503 without CF markers, retry with different UA
                    if attempt < cfg.max_retries {
                        let delay = cfg.base_delay_ms * 2u64.pow(attempt);
                        tokio::time::sleep(Duration::from_millis(delay)).await;
                        continue;
                    }
                } else {
                    // Success (or non-antibot error like 404)
                    return Ok(AdaptiveFetchResult {
                        status,
                        headers,
                        body,
                        url: final_url,
                        method: "http".into(),
                        attempts: attempt + 1,
                        duration_ms: start.elapsed().as_secs_f64() * 1000.0,
                        was_escalated: false,
                    });
                }
            }
            Err(e) => {
                last_error = e;
                if attempt < cfg.max_retries {
                    let delay = cfg.base_delay_ms * 2u64.pow(attempt);
                    tokio::time::sleep(Duration::from_millis(delay)).await;
                }
            }
        }
    }

    // Phase 2: Escalate to CDP (browser fetch)
    if cfg.escalate_to_cdp {
        tracing::info!("escalating to CDP for {url}");
        match cdp_get(page, url).await {
            Ok((status, headers, body, final_url)) => {
                return Ok(AdaptiveFetchResult {
                    status,
                    headers,
                    body,
                    url: final_url,
                    method: "cdp".into(),
                    attempts: cfg.max_retries + 2,
                    duration_ms: start.elapsed().as_secs_f64() * 1000.0,
                    was_escalated: true,
                });
            }
            Err(e) => {
                return Err(Error::Cdp(format!(
                    "adaptive fetch failed — HTTP: {last_error}, CDP: {e}"
                )));
            }
        }
    }

    Err(Error::Cdp(format!(
        "adaptive fetch failed after {} retries: {last_error}",
        cfg.max_retries + 1
    )))
}

/// Adaptive GET with default config (convenience).
pub async fn adaptive_get_default(page: &Page, url: &str) -> Result<AdaptiveFetchResult> {
    adaptive_get(page, url, None).await
}

/// Standalone HTTP GET (no CDP, no escalation) with Chrome-like headers.
///
/// Use when you don't have a browser session but want to avoid bot detection.
pub async fn standalone_get(
    url: &str,
    config: Option<AdaptiveFetchConfig>,
) -> std::result::Result<AdaptiveFetchResult, String> {
    let cfg = config.unwrap_or_default();
    let start = std::time::Instant::now();

    for attempt in 0..=cfg.max_retries {
        match http_get(url, &cfg, attempt).await {
            Ok((status, headers, body, final_url)) => {
                if status == 429 && attempt < cfg.max_retries {
                    let delay = cfg.base_delay_ms * 2u64.pow(attempt);
                    let jitter = rand::random::<u64>() % (delay / 2 + 1);
                    tokio::time::sleep(Duration::from_millis(delay + jitter)).await;
                    continue;
                }
                return Ok(AdaptiveFetchResult {
                    status,
                    headers,
                    body,
                    url: final_url,
                    method: "http".into(),
                    attempts: attempt + 1,
                    duration_ms: start.elapsed().as_secs_f64() * 1000.0,
                    was_escalated: false,
                });
            }
            Err(e) => {
                if attempt < cfg.max_retries {
                    let delay = cfg.base_delay_ms * 2u64.pow(attempt);
                    tokio::time::sleep(Duration::from_millis(delay)).await;
                    continue;
                }
                return Err(e);
            }
        }
    }

    Err("max retries exceeded".into())
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let cfg = AdaptiveFetchConfig::default();
        assert_eq!(cfg.max_retries, 3);
        assert_eq!(cfg.base_delay_ms, 1000);
        assert!(cfg.escalate_to_cdp);
        assert_eq!(cfg.timeout_ms, 30000);
        assert!(cfg.user_agent.is_none());
        assert!(cfg.extra_headers.is_empty());
    }

    #[test]
    fn test_chrome_headers_order() {
        let headers = chrome_headers("https://example.com/page", None);
        let names: Vec<&str> = headers.iter().map(|(k, _)| k.as_str()).collect();

        // Verify Chrome header order
        assert_eq!(names[0], "Host");
        assert_eq!(names[1], "sec-ch-ua");
        assert_eq!(names[2], "sec-ch-ua-mobile");
        assert_eq!(names[3], "sec-ch-ua-platform");
        assert!(names.contains(&"User-Agent"));
        assert!(names.contains(&"Accept"));
        assert!(names.contains(&"Accept-Encoding"));
        assert!(names.contains(&"Accept-Language"));
    }

    #[test]
    fn test_chrome_headers_host_extraction() {
        let headers = chrome_headers("https://x.com/home", None);
        let host = &headers[0];
        assert_eq!(host.0, "Host");
        assert_eq!(host.1, "x.com");
    }

    #[test]
    fn test_chrome_headers_custom_ua() {
        let headers = chrome_headers("https://example.com", Some("CustomBot/1.0"));
        let ua = headers.iter().find(|(k, _)| k == "User-Agent").unwrap();
        assert_eq!(ua.1, "CustomBot/1.0");
    }

    #[test]
    fn test_rotate_user_agent() {
        let ua0 = rotate_user_agent(0);
        let ua1 = rotate_user_agent(1);
        assert_ne!(ua0, ua1);
        // Should cycle
        let ua4 = rotate_user_agent(4);
        assert_eq!(ua0, ua4);
    }

    #[test]
    fn test_is_antibot_cloudflare_403() {
        let body = r#"<html><body>Just a moment... Checking your browser cf_chl_opt</body></html>"#;
        assert!(is_antibot_response(403, body));
    }

    #[test]
    fn test_is_antibot_rate_limit() {
        assert!(is_antibot_response(429, "Too many requests"));
    }

    #[test]
    fn test_is_antibot_normal_403() {
        // 403 without WAF markers should not trigger
        assert!(!is_antibot_response(403, "Forbidden - you need a login"));
    }

    #[test]
    fn test_is_antibot_200() {
        assert!(!is_antibot_response(200, "Hello World"));
    }

    #[test]
    fn test_is_cf_challenge_positive() {
        let body = "cf-browser-verification some-token";
        assert!(is_cf_challenge(body));
    }

    #[test]
    fn test_is_cf_challenge_negative() {
        let body = "<html>Normal page</html>";
        assert!(!is_cf_challenge(body));
    }

    #[test]
    fn test_config_serialize_roundtrip() {
        let cfg = AdaptiveFetchConfig {
            max_retries: 5,
            base_delay_ms: 2000,
            escalate_to_cdp: false,
            timeout_ms: 10000,
            user_agent: Some("Test/1.0".into()),
            extra_headers: HashMap::from([("X-Custom".into(), "value".into())]),
        };
        let json = serde_json::to_string(&cfg).unwrap();
        let parsed: AdaptiveFetchConfig = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.max_retries, 5);
        assert_eq!(parsed.user_agent.as_deref(), Some("Test/1.0"));
        assert_eq!(
            parsed.extra_headers.get("X-Custom").map(|s| s.as_str()),
            Some("value")
        );
    }

    #[test]
    fn test_result_serialize_roundtrip() {
        let result = AdaptiveFetchResult {
            status: 200,
            headers: HashMap::from([("content-type".into(), "text/html".into())]),
            body: "<html>test</html>".into(),
            url: "https://example.com".into(),
            method: "http".into(),
            attempts: 1,
            duration_ms: 150.5,
            was_escalated: false,
        };
        let json = serde_json::to_string(&result).unwrap();
        let parsed: AdaptiveFetchResult = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.status, 200);
        assert_eq!(parsed.method, "http");
        assert!(!parsed.was_escalated);
    }
}
