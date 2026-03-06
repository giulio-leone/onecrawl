//! Handler implementations for the `crawl` super-tool.

use rmcp::{ErrorData as McpError, model::*};
use crate::cdp_tools::*;
use crate::helpers::{mcp_err, ensure_page, json_ok, text_ok, parse_json_str, McpResult};
use crate::types::*;
use crate::OneCrawlMcp;

impl OneCrawlMcp {

    // ════════════════════════════════════════════════════════════════
    //  CDP tools — Crawling
    // ════════════════════════════════════════════════════════════════

    pub(crate) async fn crawling_spider(
        &self,
        p: SpiderCrawlParams,
    ) -> Result<CallToolResult, McpError> {
        let page = ensure_page(&self.browser).await?;
        let config = onecrawl_cdp::SpiderConfig {
            start_urls: p.start_urls,
            max_depth: p.max_depth.unwrap_or(2),
            max_pages: p.max_pages.unwrap_or(50),
            concurrency: 1,
            delay_ms: p.delay_ms.unwrap_or(500),
            follow_links: true,
            same_domain_only: p.same_domain_only.unwrap_or(true),
            url_patterns: p.url_patterns.unwrap_or_default(),
            exclude_patterns: p.exclude_patterns.unwrap_or_default(),
            extract_selector: None,
            extract_format: "text".into(),
            timeout_ms: 30_000,
            user_agent: None,
        };
        let results = onecrawl_cdp::spider::crawl(&page, config)
            .await
            .mcp()?;
        let summary = onecrawl_cdp::spider::summarize(&results);
        json_ok(&CrawlResult2 {
            summary,
            pages_crawled: results.len(),
        })
    }


    pub(crate) async fn crawling_robots(
        &self,
        p: CheckRobotsParams,
    ) -> Result<CallToolResult, McpError> {
        let page = ensure_page(&self.browser).await?;
        let robots = onecrawl_cdp::robots::fetch_robots(&page, &p.base_url)
            .await
            .mcp()?;
        let ua = p.user_agent.as_deref().unwrap_or("*");
        let sitemaps = onecrawl_cdp::robots::get_sitemaps(&robots);
        let delay = onecrawl_cdp::robots::get_crawl_delay(&robots, ua);
        let allowed = p.path.as_ref().map(|path| {
            onecrawl_cdp::robots::is_allowed(&robots, ua, path)
        });
        json_ok(&RobotsInfo {
            sitemaps,
            crawl_delay: delay,
            path_allowed: allowed,
        })
    }


    pub(crate) fn crawling_sitemap(
        &self,
        p: GenerateSitemapParams,
    ) -> Result<CallToolResult, McpError> {
        let entries: Vec<onecrawl_cdp::sitemap::SitemapEntry> = parse_json_str(&p.entries, "entries")?;
        let config = onecrawl_cdp::sitemap::SitemapConfig {
            base_url: p.base_url,
            default_changefreq: p.default_changefreq.unwrap_or_else(|| "weekly".into()),
            default_priority: 0.5,
            include_lastmod: true,
        };
        let xml = onecrawl_cdp::sitemap::generate_sitemap(&entries, &config);
        text_ok(xml)
    }


    pub(crate) async fn crawling_snapshot(
        &self,
        p: TakeSnapshotParams,
    ) -> Result<CallToolResult, McpError> {
        let page = ensure_page(&self.browser).await?;
        let snap = onecrawl_cdp::snapshot::take_snapshot(&page)
            .await
            .mcp()?;
        let mut state = self.browser.lock().await;
        state.snapshots.insert(p.label.clone(), snap);
        text_ok(format!("snapshot '{}' saved", p.label))
    }


    pub(crate) async fn crawling_compare(
        &self,
        p: CompareSnapshotsParams,
    ) -> Result<CallToolResult, McpError> {
        let state = self.browser.lock().await;
        let before = state
            .snapshots
            .get(&p.before)
            .ok_or_else(|| mcp_err(format!("snapshot '{}' not found", p.before)))?;
        let after = state
            .snapshots
            .get(&p.after)
            .ok_or_else(|| mcp_err(format!("snapshot '{}' not found", p.after)))?;
        let diff = onecrawl_cdp::snapshot::compare_snapshots(before, after);
        json_ok(&diff)
    }
}
