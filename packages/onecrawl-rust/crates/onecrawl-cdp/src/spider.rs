//! Spider/Crawler framework — lightweight, configurable web crawler
//! using the existing CDP browser infrastructure.

use chromiumoxide::Page;
use onecrawl_core::Result;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};

/// Configuration for a crawl session.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpiderConfig {
    pub start_urls: Vec<String>,
    pub max_depth: usize,
    pub max_pages: usize,
    pub concurrency: usize,
    pub delay_ms: u64,
    pub follow_links: bool,
    pub same_domain_only: bool,
    /// Substring patterns a URL must contain to be included (empty = allow all).
    pub url_patterns: Vec<String>,
    /// Substring patterns that cause a URL to be excluded.
    pub exclude_patterns: Vec<String>,
    /// Optional CSS selector to extract content from each page.
    pub extract_selector: Option<String>,
    /// Format for extracted content: "text", "html", "markdown", "json".
    pub extract_format: String,
    pub timeout_ms: u64,
    pub user_agent: Option<String>,
}

impl Default for SpiderConfig {
    fn default() -> Self {
        Self {
            start_urls: vec![],
            max_depth: 3,
            max_pages: 100,
            concurrency: 3,
            delay_ms: 500,
            follow_links: true,
            same_domain_only: true,
            url_patterns: vec![],
            exclude_patterns: vec![],
            extract_selector: None,
            extract_format: "text".to_string(),
            timeout_ms: 30000,
            user_agent: None,
        }
    }
}

/// Result of crawling a single page.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CrawlResult {
    pub url: String,
    /// "success", "error", "timeout", "skipped"
    pub status: String,
    pub title: String,
    pub depth: usize,
    pub links_found: usize,
    pub content: Option<String>,
    pub error: Option<String>,
    pub duration_ms: f64,
    pub timestamp: f64,
}

/// Aggregate statistics for a completed crawl.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CrawlSummary {
    pub total_pages: usize,
    pub successful: usize,
    pub failed: usize,
    pub skipped: usize,
    pub total_links_found: usize,
    pub total_duration_ms: f64,
    pub pages_per_second: f64,
    pub domain_stats: HashMap<String, usize>,
}

/// Serialisable crawl state for pause/resume.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CrawlState {
    pub config: SpiderConfig,
    pub visited: Vec<String>,
    /// (url, depth)
    pub pending: Vec<(String, usize)>,
    pub results: Vec<CrawlResult>,
    /// "running", "paused", "completed", "stopped"
    pub status: String,
}

// ── helpers ────────────────────────────────────────────────────

fn extract_domain(url_str: &str) -> &str {
    url_str
        .split("://")
        .nth(1)
        .unwrap_or(url_str)
        .split('/')
        .next()
        .unwrap_or("")
        .split(':')
        .next()
        .unwrap_or("")
}

fn now_epoch_ms() -> f64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs_f64()
        * 1000.0
}

/// Returns `true` when the URL should be skipped (asset, mailto, etc.).
fn is_non_page_url(url: &str) -> bool {
    let lower = url.to_ascii_lowercase();
    if lower.starts_with("mailto:") || lower.starts_with("tel:") || lower.starts_with("javascript:")
    {
        return true;
    }
    let path = lower.split('?').next().unwrap_or(&lower);
    let path = path.split('#').next().unwrap_or(path);
    let asset_exts = [
        ".jpg", ".jpeg", ".png", ".gif", ".svg", ".ico", ".css", ".js", ".woff", ".woff2", ".ttf",
        ".eot", ".mp4", ".mp3", ".pdf", ".zip",
    ];
    asset_exts.iter().any(|ext| path.ends_with(ext))
}

/// Check whether `url` matches any exclude substring pattern (pre-lowercased).
fn matches_exclude(url_lower: &str, patterns_lower: &[String]) -> bool {
    patterns_lower.iter().any(|p| url_lower.contains(p.as_str()))
}

/// Check whether `url` matches at least one include pattern (pre-lowercased, empty = allow all).
fn matches_include(url_lower: &str, patterns_lower: &[String]) -> bool {
    if patterns_lower.is_empty() {
        return true;
    }
    patterns_lower.iter().any(|p| url_lower.contains(p.as_str()))
}

// ── public API ─────────────────────────────────────────────────

/// Run a sequential crawl using the provided browser `page`.
pub async fn crawl(page: &Page, config: SpiderConfig) -> Result<Vec<CrawlResult>> {
    let mut visited: HashSet<String> = HashSet::with_capacity(config.max_pages);
    let mut queue: Vec<(String, usize)> =
        config.start_urls.iter().map(|u| (u.clone(), 0)).collect();
    let mut results: Vec<CrawlResult> = Vec::with_capacity(config.max_pages);

    let exclude_lower: Vec<String> = config
        .exclude_patterns
        .iter()
        .map(|p| p.to_ascii_lowercase())
        .collect();
    let include_lower: Vec<String> = config
        .url_patterns
        .iter()
        .map(|p| p.to_ascii_lowercase())
        .collect();

    let start_domain = config
        .start_urls
        .first()
        .map(|u| extract_domain(u).to_string())
        .unwrap_or_default();

    while let Some((url, depth)) = queue.pop() {
        if visited.len() >= config.max_pages {
            break;
        }
        if depth > config.max_depth || visited.contains(&url) || is_non_page_url(&url) {
            continue;
        }
        let url_lower = url.to_ascii_lowercase();
        if matches_exclude(&url_lower, &exclude_lower)
            || !matches_include(&url_lower, &include_lower)
        {
            continue;
        }
        if config.same_domain_only && extract_domain(&url) != start_domain {
            continue;
        }

        visited.insert(url.clone());
        let start_time = std::time::Instant::now();

        let result = crawl_single_page(page, &config, &url, depth, &mut visited, &mut queue, start_time).await;
        results.push(result);

        if config.delay_ms > 0 {
            tokio::time::sleep(std::time::Duration::from_millis(config.delay_ms)).await;
        }
    }

    Ok(results)
}

async fn crawl_single_page(
    page: &Page,
    config: &SpiderConfig,
    url: &str,
    depth: usize,
    visited: &mut HashSet<String>,
    queue: &mut Vec<(String, usize)>,
    start_time: std::time::Instant,
) -> CrawlResult {
    let make_result = |status: &str, title: String, links: usize, content: Option<String>, error: Option<String>, elapsed_ms: f64| {
        CrawlResult {
            url: url.to_string(),
            status: status.to_string(),
            title,
            depth,
            links_found: links,
            content,
            error,
            duration_ms: elapsed_ms,
            timestamp: now_epoch_ms(),
        }
    };

    match tokio::time::timeout(
        std::time::Duration::from_millis(config.timeout_ms),
        page.goto(url),
    )
    .await
    {
        Ok(Ok(_)) => {
            tokio::time::sleep(std::time::Duration::from_millis(500)).await;

            let title: String = page
                .evaluate("document.title")
                .await
                .ok()
                .and_then(|v| v.into_value::<String>().ok())
                .unwrap_or_default();

            let mut links_found: usize = 0;
            if config.follow_links && depth < config.max_depth {
                let links_js = r#"
                    Array.from(document.querySelectorAll('a[href]'))
                        .map(a => a.href)
                        .filter(h => h.startsWith('http'))
                "#;
                if let Ok(val) = page.evaluate(links_js).await
                    && let Ok(links) = val.into_value::<Vec<String>>()
                {
                    for href in &links {
                        if !visited.contains(href) {
                            queue.push((href.clone(), depth + 1));
                            links_found += 1;
                        }
                    }
                }
            }

            let content = if let Some(ref sel) = config.extract_selector {
                let js = match config.extract_format.as_str() {
                    "html" => format!(
                        "document.querySelector('{}')?.innerHTML || ''",
                        sel.replace('\'', "\\'")
                    ),
                    _ => format!(
                        "document.querySelector('{}')?.textContent || ''",
                        sel.replace('\'', "\\'")
                    ),
                };
                page.evaluate(js)
                    .await
                    .ok()
                    .and_then(|v| v.into_value::<String>().ok())
            } else {
                None
            };

            let ms = start_time.elapsed().as_secs_f64() * 1000.0;
            make_result("success", title, links_found, content, None, ms)
        }
        Ok(Err(e)) => {
            let ms = start_time.elapsed().as_secs_f64() * 1000.0;
            make_result("error", String::new(), 0, None, Some(e.to_string()), ms)
        }
        Err(_) => {
            make_result("timeout", String::new(), 0, None, Some("Timeout".to_string()), config.timeout_ms as f64)
        }
    }
}

/// Compute aggregate statistics from crawl results.
pub fn summarize(results: &[CrawlResult]) -> CrawlSummary {
    let mut domain_stats: HashMap<String, usize> = HashMap::new();
    let mut successful: usize = 0;
    let mut failed: usize = 0;
    let mut skipped: usize = 0;
    let mut total_links: usize = 0;
    let mut total_duration: f64 = 0.0;

    for r in results {
        match r.status.as_str() {
            "success" => successful += 1,
            "error" | "timeout" => failed += 1,
            _ => skipped += 1,
        }
        total_links += r.links_found;
        total_duration += r.duration_ms;

        let domain = extract_domain(&r.url).to_string();
        *domain_stats.entry(domain).or_insert(0) += 1;
    }

    let pages_per_second = if total_duration > 0.0 {
        (results.len() as f64 / total_duration) * 1000.0
    } else {
        0.0
    };

    CrawlSummary {
        total_pages: results.len(),
        successful,
        failed,
        skipped,
        total_links_found: total_links,
        total_duration_ms: total_duration,
        pages_per_second,
        domain_stats,
    }
}

/// Persist crawl state to a JSON file (for pause/resume).
pub fn save_state(state: &CrawlState, path: &std::path::Path) -> Result<()> {
    let json = serde_json::to_string_pretty(state)?;
    std::fs::write(path, json)?;
    Ok(())
}

/// Load crawl state from a JSON file.
pub fn load_state(path: &std::path::Path) -> Result<CrawlState> {
    let json = std::fs::read_to_string(path)?;
    Ok(serde_json::from_str(&json)?)
}

/// Export results to a JSON file. Returns the number of results written.
pub fn export_results(results: &[CrawlResult], path: &std::path::Path) -> Result<usize> {
    let json = serde_json::to_string_pretty(results)?;
    let count = results.len();
    std::fs::write(path, json)?;
    Ok(count)
}

/// Export results as JSONL (one JSON object per line). Returns the count.
pub fn export_results_jsonl(results: &[CrawlResult], path: &std::path::Path) -> Result<usize> {
    let mut content = String::new();
    for r in results {
        content.push_str(&serde_json::to_string(r)?);
        content.push('\n');
    }
    let count = results.len();
    std::fs::write(path, content)?;
    Ok(count)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_spider_config_defaults() {
        let cfg = SpiderConfig::default();
        assert_eq!(cfg.max_depth, 3);
        assert_eq!(cfg.max_pages, 100);
        assert_eq!(cfg.concurrency, 3);
        assert_eq!(cfg.delay_ms, 500);
        assert!(cfg.follow_links);
        assert!(cfg.same_domain_only);
        assert!(cfg.start_urls.is_empty());
        assert!(cfg.url_patterns.is_empty());
        assert!(cfg.exclude_patterns.is_empty());
        assert!(cfg.extract_selector.is_none());
        assert_eq!(cfg.extract_format, "text");
        assert_eq!(cfg.timeout_ms, 30000);
        assert!(cfg.user_agent.is_none());
    }

    fn make_result(url: &str, status: &str, links: usize, duration: f64) -> CrawlResult {
        CrawlResult {
            url: url.to_string(),
            status: status.to_string(),
            title: format!("Title of {url}"),
            depth: 0,
            links_found: links,
            content: None,
            error: if status == "error" {
                Some("err".into())
            } else {
                None
            },
            duration_ms: duration,
            timestamp: 1000.0,
        }
    }

    #[test]
    fn test_summarize_empty() {
        let summary = summarize(&[]);
        assert_eq!(summary.total_pages, 0);
        assert_eq!(summary.successful, 0);
        assert_eq!(summary.pages_per_second, 0.0);
    }

    #[test]
    fn test_summarize_mixed() {
        let results = vec![
            make_result("https://a.com/1", "success", 5, 100.0),
            make_result("https://a.com/2", "success", 3, 200.0),
            make_result("https://b.com/1", "error", 0, 50.0),
            make_result("https://a.com/3", "timeout", 0, 30000.0),
        ];
        let summary = summarize(&results);
        assert_eq!(summary.total_pages, 4);
        assert_eq!(summary.successful, 2);
        assert_eq!(summary.failed, 2);
        assert_eq!(summary.total_links_found, 8);
        assert!(summary.pages_per_second > 0.0);
        assert_eq!(*summary.domain_stats.get("a.com").unwrap(), 3);
        assert_eq!(*summary.domain_stats.get("b.com").unwrap(), 1);
    }

    #[test]
    fn test_save_load_state_roundtrip() {
        let state = CrawlState {
            config: SpiderConfig::default(),
            visited: vec!["https://example.com".into()],
            pending: vec![("https://example.com/page2".into(), 1)],
            results: vec![make_result("https://example.com", "success", 2, 150.0)],
            status: "paused".into(),
        };
        let tmp = std::env::temp_dir().join("onecrawl_spider_state_test.json");
        save_state(&state, &tmp).unwrap();
        let loaded = load_state(&tmp).unwrap();
        assert_eq!(loaded.visited, state.visited);
        assert_eq!(loaded.pending, state.pending);
        assert_eq!(loaded.status, "paused");
        assert_eq!(loaded.results.len(), 1);
        std::fs::remove_file(&tmp).ok();
    }

    #[test]
    fn test_export_results_json() {
        let results = vec![
            make_result("https://a.com", "success", 3, 100.0),
            make_result("https://b.com", "error", 0, 50.0),
        ];
        let tmp = std::env::temp_dir().join("onecrawl_spider_export_test.json");
        let count = export_results(&results, &tmp).unwrap();
        assert_eq!(count, 2);
        let content = std::fs::read_to_string(&tmp).unwrap();
        let parsed: Vec<CrawlResult> = serde_json::from_str(&content).unwrap();
        assert_eq!(parsed.len(), 2);
        assert_eq!(parsed[0].url, "https://a.com");
        std::fs::remove_file(&tmp).ok();
    }

    #[test]
    fn test_export_results_jsonl() {
        let results = vec![
            make_result("https://a.com", "success", 1, 100.0),
            make_result("https://b.com", "success", 2, 200.0),
        ];
        let tmp = std::env::temp_dir().join("onecrawl_spider_export_test.jsonl");
        let count = export_results_jsonl(&results, &tmp).unwrap();
        assert_eq!(count, 2);
        let content = std::fs::read_to_string(&tmp).unwrap();
        let lines: Vec<&str> = content.lines().collect();
        assert_eq!(lines.len(), 2);
        let first: CrawlResult = serde_json::from_str(lines[0]).unwrap();
        assert_eq!(first.url, "https://a.com");
        let second: CrawlResult = serde_json::from_str(lines[1]).unwrap();
        assert_eq!(second.url, "https://b.com");
        std::fs::remove_file(&tmp).ok();
    }
}
