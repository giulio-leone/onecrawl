//! Robots.txt parser and compliance checker.

use chromiumoxide::Page;
use onecrawl_core::Result;
use serde::{Deserialize, Serialize};

/// A single user-agent block in robots.txt.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RobotsRule {
    pub user_agent: String,
    pub allow: Vec<String>,
    pub disallow: Vec<String>,
    pub crawl_delay: Option<f64>,
    pub sitemaps: Vec<String>,
}

/// Parsed robots.txt content.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RobotsTxt {
    pub rules: Vec<RobotsRule>,
    pub sitemaps: Vec<String>,
}

// ── parser ────────────────────────────────────────────────────────

/// Parse robots.txt content into structured rules.
pub fn parse_robots(content: &str) -> RobotsTxt {
    let mut rules: Vec<RobotsRule> = Vec::new();
    let mut global_sitemaps: Vec<String> = Vec::new();
    let mut current_ua: Option<String> = None;
    let mut allow: Vec<String> = Vec::new();
    let mut disallow: Vec<String> = Vec::new();
    let mut crawl_delay: Option<f64> = None;
    let mut rule_sitemaps: Vec<String> = Vec::new();

    for raw_line in content.lines() {
        let line = raw_line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        // Strip inline comments
        let line = line.split('#').next().unwrap_or(line).trim();
        let Some((key, value)) = line.split_once(':') else {
            continue;
        };
        let key = key.trim();
        let value = value.trim();

        if key.eq_ignore_ascii_case("user-agent") {
            // Flush previous rule block when we see a new user-agent
            if let Some(ua) = current_ua.take() {
                rules.push(RobotsRule {
                    user_agent: ua,
                    allow: std::mem::take(&mut allow),
                    disallow: std::mem::take(&mut disallow),
                    crawl_delay: crawl_delay.take(),
                    sitemaps: std::mem::take(&mut rule_sitemaps),
                });
            }
            current_ua = Some(value.to_string());
        } else if key.eq_ignore_ascii_case("allow") {
            if !value.is_empty() {
                allow.push(value.to_string());
            }
        } else if key.eq_ignore_ascii_case("disallow") {
            if !value.is_empty() {
                disallow.push(value.to_string());
            }
        } else if key.eq_ignore_ascii_case("crawl-delay") {
            crawl_delay = value.parse::<f64>().ok();
        } else if key.eq_ignore_ascii_case("sitemap") {
            let url = value.to_string();
            if current_ua.is_some() {
                rule_sitemaps.push(url.clone());
            }
            global_sitemaps.push(url);
        }
    }

    // Flush last rule block
    if let Some(ua) = current_ua {
        rules.push(RobotsRule {
            user_agent: ua,
            allow,
            disallow,
            crawl_delay,
            sitemaps: rule_sitemaps,
        });
    }

    RobotsTxt {
        rules,
        sitemaps: global_sitemaps,
    }
}

// ── path matching ────────────────────────────────────────────────

/// Simple robots.txt path prefix match (supports `*` wildcard and `$` anchor).
fn path_matches(pattern: &str, path: &str) -> bool {
    if pattern.is_empty() {
        return false;
    }
    let anchored = pattern.ends_with('$');
    let pattern = if anchored {
        &pattern[..pattern.len() - 1]
    } else {
        pattern
    };

    let parts: Vec<&str> = pattern.split('*').collect();
    let mut pos = 0usize;
    for (i, part) in parts.iter().enumerate() {
        if part.is_empty() {
            continue;
        }
        match path[pos..].find(part) {
            Some(idx) => {
                // First segment must match at the start
                if i == 0 && idx != 0 {
                    return false;
                }
                pos += idx + part.len();
            }
            None => return false,
        }
    }

    if anchored {
        pos == path.len()
    } else {
        true
    }
}

/// Find the best-matching rule for a given user-agent (case-insensitive).
fn find_rule<'a>(robots: &'a RobotsTxt, user_agent: &str) -> Option<&'a RobotsRule> {
    let ua_lower = user_agent.to_ascii_lowercase();
    // Prefer exact match first, then wildcard
    robots
        .rules
        .iter()
        .find(|r| r.user_agent.to_ascii_lowercase() == ua_lower)
        .or_else(|| {
            robots
                .rules
                .iter()
                .find(|r| ua_lower.contains(&r.user_agent.to_ascii_lowercase()))
        })
        .or_else(|| robots.rules.iter().find(|r| r.user_agent == "*"))
}

// ── public helpers ───────────────────────────────────────────────

/// Check if a path is allowed for the given user-agent.
pub fn is_allowed(robots: &RobotsTxt, user_agent: &str, path: &str) -> bool {
    let Some(rule) = find_rule(robots, user_agent) else {
        return true; // no matching rule → allowed
    };

    // Longest match wins (allow vs disallow).
    let mut best_allow: Option<usize> = None;
    let mut best_disallow: Option<usize> = None;

    for pattern in &rule.allow {
        if path_matches(pattern, path) {
            let len = pattern.len();
            if best_allow.is_none() || len > best_allow.unwrap() {
                best_allow = Some(len);
            }
        }
    }
    for pattern in &rule.disallow {
        if path_matches(pattern, path) {
            let len = pattern.len();
            if best_disallow.is_none() || len > best_disallow.unwrap() {
                best_disallow = Some(len);
            }
        }
    }

    match (best_allow, best_disallow) {
        (Some(a), Some(d)) => a >= d,
        (_, Some(_)) => false,
        _ => true,
    }
}

/// Get the crawl-delay for a given user-agent.
pub fn get_crawl_delay(robots: &RobotsTxt, user_agent: &str) -> Option<f64> {
    find_rule(robots, user_agent).and_then(|r| r.crawl_delay)
}

/// Get all declared sitemap URLs.
pub fn get_sitemaps(robots: &RobotsTxt) -> Vec<String> {
    robots.sitemaps.clone()
}

/// Fetch and parse robots.txt from a URL using browser `fetch()`.
pub async fn fetch_robots(page: &Page, base_url: &str) -> Result<RobotsTxt> {
    let url = format!(
        "{}/robots.txt",
        base_url.trim_end_matches('/')
    );
    let js = format!(
        r#"fetch("{url}").then(r => r.ok ? r.text() : "").catch(() => "")"#,
    );
    let body = page
        .evaluate(js)
        .await
        .map_err(|e| onecrawl_core::Error::Browser(format!("fetch_robots failed: {e}")))?
        .into_value::<String>()
        .unwrap_or_default();
    Ok(parse_robots(&body))
}
