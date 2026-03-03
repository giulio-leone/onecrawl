//! Domain blocker — block ads, trackers, social widgets, fonts and media.
//!
//! Installs fetch/XHR interceptors in the page to silently drop requests to
//! domains on a configurable blocklist.  Provides predefined category lists
//! and per-domain hit-count statistics.

use chromiumoxide::Page;
use onecrawl_core::{Error, Result};
use serde::{Deserialize, Serialize};

/// A single blocked domain with its category and hit count.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlockedDomain {
    pub domain: String,
    pub category: String,
    pub blocked_count: usize,
}

/// Aggregate blocking statistics.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlockStats {
    pub total_blocked: usize,
    pub domains: Vec<BlockedDomain>,
}

/// Get a predefined blocklist by category.
pub fn get_blocklist(category: &str) -> Vec<String> {
    match category {
        "ads" => vec![
            "doubleclick.net",
            "googlesyndication.com",
            "googleadservices.com",
            "google-analytics.com",
            "adnxs.com",
            "adsrvr.org",
            "adserver.com",
            "advertising.com",
            "criteo.com",
            "outbrain.com",
            "taboola.com",
            "amazon-adsystem.com",
            "moatads.com",
            "pubmatic.com",
            "rubiconproject.com",
        ]
        .into_iter()
        .map(String::from)
        .collect(),

        "trackers" => vec![
            "facebook.net",
            "facebook.com/tr",
            "connect.facebook.net",
            "google-analytics.com",
            "googletagmanager.com",
            "hotjar.com",
            "mixpanel.com",
            "segment.io",
            "segment.com",
            "amplitude.com",
            "heap.io",
            "heapanalytics.com",
            "fullstory.com",
            "inspectlet.com",
            "mouseflow.com",
            "clarity.ms",
            "newrelic.com",
            "sentry.io",
        ]
        .into_iter()
        .map(String::from)
        .collect(),

        "social" => vec![
            "platform.twitter.com",
            "platform.linkedin.com",
            "connect.facebook.net",
            "apis.google.com",
            "platform.instagram.com",
            "widgets.pinterest.com",
            "static.addtoany.com",
            "s7.addthis.com",
            "disqus.com",
            "disquscdn.com",
        ]
        .into_iter()
        .map(String::from)
        .collect(),

        "fonts" => vec![
            "fonts.googleapis.com",
            "fonts.gstatic.com",
            "use.typekit.net",
            "fast.fonts.net",
            "cloud.typography.com",
        ]
        .into_iter()
        .map(String::from)
        .collect(),

        "media" => vec![
            "youtube.com",
            "vimeo.com",
            "dailymotion.com",
            "twitch.tv",
            "spotify.com",
            "soundcloud.com",
        ]
        .into_iter()
        .map(String::from)
        .collect(),

        _ => vec![],
    }
}

/// Available blocklist categories with their domain counts.
pub fn available_categories() -> Vec<(String, usize)> {
    vec![
        ("ads".to_string(), get_blocklist("ads").len()),
        ("trackers".to_string(), get_blocklist("trackers").len()),
        ("social".to_string(), get_blocklist("social").len()),
        ("fonts".to_string(), get_blocklist("fonts").len()),
        ("media".to_string(), get_blocklist("media").len()),
    ]
}

/// Install a domain blocker that intercepts fetch/XHR to the given domains.
///
/// Returns the total number of domains currently on the blocklist.
pub async fn block_domains(page: &Page, domains: &[String]) -> Result<usize> {
    let domains_json = serde_json::to_string(domains).map_err(|e| Error::Cdp(e.to_string()))?;

    let js = format!(
        r#"
        (() => {{
            window.__onecrawl_blocked_domains = window.__onecrawl_blocked_domains || [];
            window.__onecrawl_blocked_count = window.__onecrawl_blocked_count || {{}};

            const newDomains = {domains_json};
            window.__onecrawl_blocked_domains.push(...newDomains);

            // Deduplicate
            window.__onecrawl_blocked_domains = [...new Set(window.__onecrawl_blocked_domains)];

            // Install fetch interceptor
            if (!window.__onecrawl_fetch_intercepted) {{
                const origFetch = window.fetch;
                window.fetch = function(url, opts) {{
                    const urlStr = typeof url === 'string' ? url : url.url || '';
                    for (const domain of window.__onecrawl_blocked_domains) {{
                        if (urlStr.includes(domain)) {{
                            window.__onecrawl_blocked_count[domain] = (window.__onecrawl_blocked_count[domain] || 0) + 1;
                            return Promise.reject(new Error('Blocked by OneCrawl: ' + domain));
                        }}
                    }}
                    return origFetch.apply(this, arguments);
                }};
                window.__onecrawl_fetch_intercepted = true;
            }}

            // Install XHR interceptor
            if (!window.__onecrawl_xhr_intercepted) {{
                const origOpen = XMLHttpRequest.prototype.open;
                XMLHttpRequest.prototype.open = function(method, url) {{
                    const urlStr = typeof url === 'string' ? url : '';
                    for (const domain of window.__onecrawl_blocked_domains) {{
                        if (urlStr.includes(domain)) {{
                            window.__onecrawl_blocked_count[domain] = (window.__onecrawl_blocked_count[domain] || 0) + 1;
                            this.__onecrawl_blocked = true;
                            return;
                        }}
                    }}
                    return origOpen.apply(this, arguments);
                }};

                const origSend = XMLHttpRequest.prototype.send;
                XMLHttpRequest.prototype.send = function() {{
                    if (this.__onecrawl_blocked) return;
                    return origSend.apply(this, arguments);
                }};
                window.__onecrawl_xhr_intercepted = true;
            }}

            return window.__onecrawl_blocked_domains.length;
        }})()
    "#
    );

    let result = page
        .evaluate(js)
        .await
        .map_err(|e| Error::Cdp(format!("block_domains failed: {e}")))?;

    let count: usize = result.into_value().unwrap_or(0);
    Ok(count)
}

/// Block domains by category name (`ads`, `trackers`, `social`, `fonts`, `media`).
pub async fn block_category(page: &Page, category: &str) -> Result<usize> {
    let domains = get_blocklist(category);
    if domains.is_empty() {
        return Err(Error::Config(format!("Unknown category: {category}")));
    }
    block_domains(page, &domains).await
}

/// Get blocking statistics — total blocked count and per-domain breakdown.
pub async fn block_stats(page: &Page) -> Result<BlockStats> {
    let js = r#"
        (() => {
            const domains = window.__onecrawl_blocked_domains || [];
            const counts = window.__onecrawl_blocked_count || {};
            return JSON.stringify({
                total_blocked: Object.values(counts).reduce((a, b) => a + b, 0),
                domains: domains.map(d => ({
                    domain: d,
                    category: 'custom',
                    blocked_count: counts[d] || 0
                }))
            });
        })()
    "#;

    let result = page
        .evaluate(js)
        .await
        .map_err(|e| Error::Cdp(format!("block_stats failed: {e}")))?;

    let raw: String = result
        .into_value()
        .unwrap_or_else(|_| r#"{"total_blocked":0,"domains":[]}"#.to_string());
    let stats: BlockStats = serde_json::from_str(&raw)?;
    Ok(stats)
}

/// Clear all blocked domains and reset counters.
pub async fn clear_blocks(page: &Page) -> Result<()> {
    let js = r#"
        window.__onecrawl_blocked_domains = [];
        window.__onecrawl_blocked_count = {};
    "#;
    page.evaluate(js)
        .await
        .map_err(|e| Error::Cdp(format!("clear_blocks failed: {e}")))?;
    Ok(())
}

/// Get list of currently blocked domains.
pub async fn list_blocked(page: &Page) -> Result<Vec<String>> {
    let js = "JSON.stringify(window.__onecrawl_blocked_domains || [])";
    let result = page
        .evaluate(js)
        .await
        .map_err(|e| Error::Cdp(format!("list_blocked failed: {e}")))?;

    let raw: String = result.into_value().unwrap_or_else(|_| "[]".to_string());
    let domains: Vec<String> = serde_json::from_str(&raw)?;
    Ok(domains)
}
