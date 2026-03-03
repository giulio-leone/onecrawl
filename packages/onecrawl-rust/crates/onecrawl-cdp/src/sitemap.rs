//! Sitemap Generator — produce XML sitemaps from crawl results.

use onecrawl_core::Result;
use serde::{Deserialize, Serialize};

use crate::spider::CrawlResult;

/// A single URL entry in a sitemap.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SitemapEntry {
    pub url: String,
    pub lastmod: Option<String>,
    /// "always", "hourly", "daily", "weekly", "monthly", "yearly", "never"
    pub changefreq: Option<String>,
    /// 0.0 to 1.0
    pub priority: Option<f64>,
}

/// Configuration for sitemap generation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SitemapConfig {
    pub base_url: String,
    pub default_changefreq: String,
    pub default_priority: f64,
    pub include_lastmod: bool,
}

impl Default for SitemapConfig {
    fn default() -> Self {
        Self {
            base_url: String::new(),
            default_changefreq: "weekly".to_string(),
            default_priority: 0.5,
            include_lastmod: true,
        }
    }
}

// ── helpers ───────────────────────────────────────────────────────

fn escape_xml(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
}

fn today_iso() -> String {
    let secs = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    let days = secs / 86400;
    // Approximate calendar date
    let mut y: i64 = 1970;
    let mut rem = days as i64;
    loop {
        let days_in_year: i64 = if y % 4 == 0 && (y % 100 != 0 || y % 400 == 0) {
            366
        } else {
            365
        };
        if rem < days_in_year {
            break;
        }
        rem -= days_in_year;
        y += 1;
    }
    let leap = y % 4 == 0 && (y % 100 != 0 || y % 400 == 0);
    let month_days: [i64; 12] = [
        31,
        if leap { 29 } else { 28 },
        31,
        30,
        31,
        30,
        31,
        31,
        30,
        31,
        30,
        31,
    ];
    let mut m: usize = 0;
    for (i, &md) in month_days.iter().enumerate() {
        if rem < md {
            m = i;
            break;
        }
        rem -= md;
    }
    format!("{y:04}-{:02}-{:02}", m + 1, rem + 1)
}

// ── public API ────────────────────────────────────────────────────

/// Generate an XML sitemap string from entries.
pub fn generate_sitemap(entries: &[SitemapEntry], config: &SitemapConfig) -> String {
    let mut xml = String::from(
        "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n\
         <urlset xmlns=\"http://www.sitemaps.org/schemas/sitemap/0.9\">\n",
    );

    for entry in entries {
        xml.push_str("  <url>\n");
        xml.push_str(&format!("    <loc>{}</loc>\n", escape_xml(&entry.url)));

        if let Some(ref lm) = entry.lastmod {
            xml.push_str(&format!("    <lastmod>{}</lastmod>\n", escape_xml(lm)));
        } else if config.include_lastmod {
            xml.push_str(&format!("    <lastmod>{}</lastmod>\n", today_iso()));
        }

        let freq = entry
            .changefreq
            .as_deref()
            .unwrap_or(&config.default_changefreq);
        xml.push_str(&format!("    <changefreq>{freq}</changefreq>\n"));

        let prio = entry.priority.unwrap_or(config.default_priority);
        xml.push_str(&format!("    <priority>{prio:.1}</priority>\n"));

        xml.push_str("  </url>\n");
    }

    xml.push_str("</urlset>\n");
    xml
}

/// Generate a sitemap index XML from a list of sitemap URLs.
pub fn generate_sitemap_index(sitemap_urls: &[String]) -> String {
    let mut xml = String::from(
        "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n\
         <sitemapindex xmlns=\"http://www.sitemaps.org/schemas/sitemap/0.9\">\n",
    );
    let today = today_iso();
    for url in sitemap_urls {
        xml.push_str("  <sitemap>\n");
        xml.push_str(&format!("    <loc>{}</loc>\n", escape_xml(url)));
        xml.push_str(&format!("    <lastmod>{today}</lastmod>\n"));
        xml.push_str("  </sitemap>\n");
    }
    xml.push_str("</sitemapindex>\n");
    xml
}

/// Save a sitemap to a file. Returns the number of entries written.
pub fn save_sitemap(
    entries: &[SitemapEntry],
    config: &SitemapConfig,
    path: &std::path::Path,
) -> Result<usize> {
    let xml = generate_sitemap(entries, config);
    std::fs::write(path, xml).map_err(onecrawl_core::Error::Io)?;
    Ok(entries.len())
}

/// Parse an existing sitemap XML string into entries.
pub fn parse_sitemap(xml: &str) -> Result<Vec<SitemapEntry>> {
    let mut entries = Vec::new();
    let mut rest = xml;

    while let Some(url_start) = rest.find("<url>") {
        let after_tag = &rest[url_start + 5..];
        let url_end = after_tag
            .find("</url>")
            .ok_or_else(|| onecrawl_core::Error::Parser("unclosed <url> tag".into()))?;
        let block = &after_tag[..url_end];
        rest = &after_tag[url_end + 6..];

        let loc = extract_tag(block, "loc").unwrap_or_default();
        if loc.is_empty() {
            continue;
        }
        let lastmod = extract_tag(block, "lastmod");
        let changefreq = extract_tag(block, "changefreq");
        let priority = extract_tag(block, "priority").and_then(|s| s.parse::<f64>().ok());

        entries.push(SitemapEntry {
            url: loc,
            lastmod,
            changefreq,
            priority,
        });
    }

    Ok(entries)
}

/// Convert spider `CrawlResult`s to sitemap entries.
pub fn from_crawl_results(results: &[CrawlResult], config: &SitemapConfig) -> Vec<SitemapEntry> {
    results
        .iter()
        .filter(|r| r.status == "success")
        .map(|r| {
            let lastmod = if config.include_lastmod {
                Some(today_iso())
            } else {
                None
            };
            SitemapEntry {
                url: r.url.clone(),
                lastmod,
                changefreq: Some(config.default_changefreq.clone()),
                priority: Some(config.default_priority),
            }
        })
        .collect()
}

// ── helpers ───────────────────────────────────────────────────────

fn extract_tag(block: &str, tag: &str) -> Option<String> {
    let open = format!("<{tag}>");
    let close = format!("</{tag}>");
    let start = block.find(&open)?;
    let after = &block[start + open.len()..];
    let end = after.find(&close)?;
    let content = after[..end].trim().to_string();
    if content.is_empty() {
        None
    } else {
        Some(content)
    }
}
