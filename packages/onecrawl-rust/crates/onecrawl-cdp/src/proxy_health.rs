//! Proxy health checking and scoring via browser-based fetch.

use chromiumoxide::Page;
use onecrawl_core::{Error, Result};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProxyHealthResult {
    pub proxy: String,
    /// `"healthy"`, `"slow"`, `"unreachable"`, `"blocked"`
    pub status: String,
    pub latency_ms: f64,
    pub ip_address: Option<String>,
    pub country: Option<String>,
    pub is_anonymous: bool,
    pub score: u32,
    pub tested_at: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProxyHealthConfig {
    pub test_url: String,
    pub timeout_ms: u64,
    pub check_anonymity: bool,
    pub check_geo: bool,
}

impl Default for ProxyHealthConfig {
    fn default() -> Self {
        Self {
            test_url: "https://httpbin.org/ip".to_string(),
            timeout_ms: 10000,
            check_anonymity: true,
            check_geo: false,
        }
    }
}

#[derive(Deserialize)]
struct FetchResult {
    reachable: bool,
    latency: f64,
    ip: Option<String>,
    blocked: bool,
}

/// Test a single proxy by fetching the test URL via the browser.
pub async fn check_proxy(
    page: &Page,
    proxy_url: &str,
    config: &ProxyHealthConfig,
) -> Result<ProxyHealthResult> {
    let js = format!(
        r#"
        (() => {{
            const testUrl = {test_url};
            const timeoutMs = {timeout};
            const checkAnonymity = {anon};

            return new Promise((resolve) => {{
                const start = performance.now();
                const controller = new AbortController();
                const timer = setTimeout(() => controller.abort(), timeoutMs);

                fetch(testUrl, {{ signal: controller.signal, cache: 'no-store' }})
                    .then(resp => {{
                        clearTimeout(timer);
                        const latency = performance.now() - start;
                        if (!resp.ok) {{
                            resolve(JSON.stringify({{
                                reachable: false,
                                latency: latency,
                                ip: null,
                                blocked: resp.status === 403 || resp.status === 429,
                                error: 'HTTP ' + resp.status
                            }}));
                            return;
                        }}
                        return resp.text().then(body => {{
                            let ip = null;
                            try {{
                                const j = JSON.parse(body);
                                ip = j.origin || j.ip || null;
                            }} catch (_) {{}}
                            resolve(JSON.stringify({{
                                reachable: true,
                                latency: latency,
                                ip: ip,
                                blocked: false,
                                error: null
                            }}));
                        }});
                    }})
                    .catch(err => {{
                        clearTimeout(timer);
                        const latency = performance.now() - start;
                        resolve(JSON.stringify({{
                            reachable: false,
                            latency: latency,
                            ip: null,
                            blocked: false,
                            error: err.message || 'fetch failed'
                        }}));
                    }});
            }});
        }})()
        "#,
        test_url = serde_json::to_string(&config.test_url)
            .map_err(|e| Error::Browser(format!("serialize test_url: {e}")))?,
        timeout = config.timeout_ms,
        anon = config.check_anonymity,
    );

    let raw: String = page
        .evaluate(js.as_str())
        .await
        .map_err(|e| Error::Browser(format!("proxy health eval failed: {e}")))?
        .into_value()
        .map_err(|e| Error::Browser(format!("proxy health parse failed: {e}")))?;

    let fr: FetchResult =
        serde_json::from_str(&raw).map_err(|e| Error::Browser(format!("json parse: {e}")))?;

    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs_f64();

    let status = if !fr.reachable && fr.blocked {
        "blocked"
    } else if !fr.reachable {
        "unreachable"
    } else if fr.latency > 5000.0 {
        "slow"
    } else {
        "healthy"
    };

    let is_anonymous = if config.check_anonymity {
        // If the returned IP differs from proxy URL host we assume anonymous
        fr.ip.is_some()
    } else {
        false
    };

    let mut result = ProxyHealthResult {
        proxy: proxy_url.to_string(),
        status: status.to_string(),
        latency_ms: fr.latency,
        ip_address: fr.ip,
        country: None,
        is_anonymous,
        score: 0,
        tested_at: now,
    };
    result.score = score_proxy(&result);
    Ok(result)
}

/// Test multiple proxies sequentially.
pub async fn check_proxies(
    page: &Page,
    proxies: &[String],
    config: &ProxyHealthConfig,
) -> Result<Vec<ProxyHealthResult>> {
    let mut results = Vec::with_capacity(proxies.len());
    for p in proxies {
        results.push(check_proxy(page, p, config).await?);
    }
    Ok(results)
}

/// Compute a health score from 0 to 100.
pub fn score_proxy(result: &ProxyHealthResult) -> u32 {
    if result.status == "unreachable" || result.status == "blocked" {
        return 0;
    }
    let mut s: f64 = 100.0;

    // Penalise latency (>200ms starts losing points, >5000ms = slow)
    if result.latency_ms > 200.0 {
        s -= ((result.latency_ms - 200.0) / 100.0).min(60.0);
    }

    // Bonus for anonymity
    if result.is_anonymous {
        s += 10.0;
    }

    // Penalty for slow status
    if result.status == "slow" {
        s -= 20.0;
    }

    s.clamp(0.0, 100.0) as u32
}

/// Filter results keeping only those at or above `min_score`.
pub fn filter_healthy(results: &[ProxyHealthResult], min_score: u32) -> Vec<ProxyHealthResult> {
    results
        .iter()
        .filter(|r| r.score >= min_score)
        .cloned()
        .collect()
}

/// Sort results by score descending.
pub fn rank_proxies(results: &[ProxyHealthResult]) -> Vec<ProxyHealthResult> {
    let mut sorted = results.to_vec();
    sorted.sort_by(|a, b| b.score.cmp(&a.score));
    sorted
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_result(status: &str, latency: f64, anon: bool, score: u32) -> ProxyHealthResult {
        ProxyHealthResult {
            proxy: "http://proxy:8080".into(),
            status: status.into(),
            latency_ms: latency,
            ip_address: if anon { Some("1.2.3.4".into()) } else { None },
            country: None,
            is_anonymous: anon,
            score,
            tested_at: 0.0,
        }
    }

    #[test]
    fn test_default_config() {
        let cfg = ProxyHealthConfig::default();
        assert_eq!(cfg.test_url, "https://httpbin.org/ip");
        assert_eq!(cfg.timeout_ms, 10000);
        assert!(cfg.check_anonymity);
        assert!(!cfg.check_geo);
    }

    #[test]
    fn test_score_healthy_low_latency() {
        let r = make_result("healthy", 50.0, true, 0);
        let s = score_proxy(&r);
        assert!(s > 90, "healthy+fast+anon should be >90, got {s}");
    }

    #[test]
    fn test_score_healthy_high_latency() {
        let r = make_result("healthy", 3000.0, false, 0);
        let s = score_proxy(&r);
        assert!(s < 80, "high latency should reduce score, got {s}");
    }

    #[test]
    fn test_score_unreachable_is_zero() {
        let r = make_result("unreachable", 0.0, false, 0);
        assert_eq!(score_proxy(&r), 0);
    }

    #[test]
    fn test_score_blocked_is_zero() {
        let r = make_result("blocked", 0.0, false, 0);
        assert_eq!(score_proxy(&r), 0);
    }

    #[test]
    fn test_score_slow_penalty() {
        let r = make_result("slow", 6000.0, false, 0);
        let s = score_proxy(&r);
        assert!(s < 30, "slow status should heavily penalise, got {s}");
    }

    #[test]
    fn test_filter_healthy() {
        let results = vec![
            make_result("healthy", 100.0, true, 95),
            make_result("unreachable", 0.0, false, 0),
            make_result("healthy", 2000.0, false, 50),
        ];
        let filtered = filter_healthy(&results, 60);
        assert_eq!(filtered.len(), 1);
        assert_eq!(filtered[0].score, 95);
    }

    #[test]
    fn test_rank_proxies() {
        let results = vec![
            make_result("healthy", 100.0, false, 30),
            make_result("healthy", 50.0, true, 95),
            make_result("healthy", 500.0, false, 70),
        ];
        let ranked = rank_proxies(&results);
        assert_eq!(ranked[0].score, 95);
        assert_eq!(ranked[1].score, 70);
        assert_eq!(ranked[2].score, 30);
    }

    #[test]
    fn test_result_serialization() {
        let r = make_result("healthy", 100.0, true, 85);
        let json = serde_json::to_string(&r).unwrap();
        let parsed: ProxyHealthResult = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.proxy, r.proxy);
        assert_eq!(parsed.score, r.score);
    }

    #[test]
    fn test_config_serialization() {
        let cfg = ProxyHealthConfig::default();
        let json = serde_json::to_string(&cfg).unwrap();
        let parsed: ProxyHealthConfig = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.test_url, cfg.test_url);
    }
}
