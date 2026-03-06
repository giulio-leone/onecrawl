//! Handler implementations for the `browser` super-tool.

use base64::{Engine as _, engine::general_purpose::STANDARD as B64};
use rmcp::{ErrorData as McpError, model::*};
use crate::cdp_tools::*;
use crate::helpers::{ensure_page, json_ok, text_ok, parse_json_str, parse_opt_json_str, McpResult};
use crate::OneCrawlMcp;
use std::collections::HashMap;

impl OneCrawlMcp {

    // ════════════════════════════════════════════════════════════════
    //  CDP tools — Navigation & Page Control
    // ════════════════════════════════════════════════════════════════

    pub(crate) async fn navigation_goto(
        &self,
        p: NavigateParams,
    ) -> Result<CallToolResult, McpError> {
        let page = ensure_page(&self.browser).await?;
        onecrawl_cdp::navigation::goto(&page, &p.url)
            .await
            .mcp()?;
        let title = onecrawl_cdp::navigation::get_title(&page)
            .await
            .unwrap_or_default();
        text_ok(format!("navigated to {} — title: {title}", p.url))
    }


    pub(crate) async fn navigation_click(
        &self,
        p: ClickParams,
    ) -> Result<CallToolResult, McpError> {
        let page = ensure_page(&self.browser).await?;
        let selector = onecrawl_cdp::accessibility::resolve_ref(&p.selector);
        onecrawl_cdp::element::click(&page, &selector)
            .await
            .mcp()?;
        text_ok(format!("clicked {}", p.selector))
    }


    pub(crate) async fn navigation_type(
        &self,
        p: TypeTextParams,
    ) -> Result<CallToolResult, McpError> {
        let page = ensure_page(&self.browser).await?;
        let selector = onecrawl_cdp::accessibility::resolve_ref(&p.selector);
        onecrawl_cdp::element::type_text(&page, &selector, &p.text)
            .await
            .mcp()?;
        text_ok(format!("typed {} chars into {}", p.text.len(), p.selector))
    }


    pub(crate) async fn navigation_screenshot(
        &self,
        p: ScreenshotParams,
    ) -> Result<CallToolResult, McpError> {
        let page = ensure_page(&self.browser).await?;
        let bytes = if let Some(sel) = &p.selector {
            onecrawl_cdp::screenshot::screenshot_element(&page, sel)
                .await
                .mcp()?
        } else if p.full_page.unwrap_or(false) {
            onecrawl_cdp::screenshot::screenshot_full(&page)
                .await
                .mcp()?
        } else {
            onecrawl_cdp::screenshot::screenshot_viewport(&page)
                .await
                .mcp()?
        };
        let b64 = B64.encode(&bytes);
        Ok(CallToolResult::success(vec![Content::image(
            b64,
            "image/png",
        )]))
    }


    pub(crate) async fn navigation_pdf(
        &self,
        p: PdfExportParams,
    ) -> Result<CallToolResult, McpError> {
        let page = ensure_page(&self.browser).await?;
        let opts = onecrawl_cdp::PdfOptions {
            landscape: p.landscape.unwrap_or(false),
            ..Default::default()
        };
        let _ = p.print_background; // reserved for future use
        let _ = p.format; // reserved for future use
        let bytes = onecrawl_cdp::screenshot::pdf_with_options(&page, &opts)
            .await
            .mcp()?;
        let b64 = B64.encode(&bytes);
        text_ok(format!(
            "pdf exported ({} bytes, base64 length {})",
            bytes.len(),
            b64.len()
        ))
    }


    pub(crate) async fn navigation_back(&self) -> Result<CallToolResult, McpError> {
        let page = ensure_page(&self.browser).await?;
        onecrawl_cdp::navigation::go_back(&page)
            .await
            .mcp()?;
        text_ok("navigated back")
    }


    pub(crate) async fn navigation_forward(&self) -> Result<CallToolResult, McpError> {
        let page = ensure_page(&self.browser).await?;
        onecrawl_cdp::navigation::go_forward(&page)
            .await
            .mcp()?;
        text_ok("navigated forward")
    }


    pub(crate) async fn navigation_reload(&self) -> Result<CallToolResult, McpError> {
        let page = ensure_page(&self.browser).await?;
        onecrawl_cdp::navigation::reload(&page)
            .await
            .mcp()?;
        text_ok("page reloaded")
    }


    pub(crate) async fn navigation_wait(
        &self,
        p: WaitForSelectorParams,
    ) -> Result<CallToolResult, McpError> {
        let page = ensure_page(&self.browser).await?;
        let timeout = p.timeout_ms.unwrap_or(30_000);
        let selector = onecrawl_cdp::accessibility::resolve_ref(&p.selector);
        onecrawl_cdp::navigation::wait_for_selector(&page, &selector, timeout)
            .await
            .mcp()?;
        text_ok(format!("selector {} found", p.selector))
    }


    pub(crate) async fn navigation_evaluate(
        &self,
        p: EvaluateJsParams,
    ) -> Result<CallToolResult, McpError> {
        let page = ensure_page(&self.browser).await?;
        let result = onecrawl_cdp::page::evaluate_js(&page, &p.js)
            .await
            .mcp()?;
        json_ok(&result)
    }


    pub(crate) async fn navigation_snapshot(
        &self,
        p: AgentSnapshotParams,
    ) -> Result<CallToolResult, McpError> {
        let page = ensure_page(&self.browser).await?;
        let opts = onecrawl_cdp::accessibility::AgentSnapshotOptions {
            interactive_only: p.interactive_only.unwrap_or(false),
            cursor: p.cursor.unwrap_or(false),
            compact: p.compact.unwrap_or(false),
            depth: p.depth,
            selector: p.selector,
        };
        let snap = onecrawl_cdp::accessibility::agent_snapshot(&page, &opts)
            .await
            .mcp()?;
        let stats = snap.stats();
        let result = serde_json::json!({
            "snapshot": snap.snapshot,
            "refs": snap.refs,
            "total": snap.total,
            "stats": {
                "lines": stats.lines,
                "chars": stats.chars,
                "estimated_tokens": stats.estimated_tokens,
                "total_refs": stats.total_refs,
                "interactive_refs": stats.interactive_refs
            }
        });
        json_ok(&result)
    }

    // ════════════════════════════════════════════════════════════════
    //  CDP tools — Scraping & Extraction
    // ════════════════════════════════════════════════════════════════


    // ════════════════════════════════════════════════════════════════
    //  CDP tools — Scraping & Extraction
    // ════════════════════════════════════════════════════════════════

    pub(crate) async fn scraping_css(
        &self,
        p: CssSelectorParams,
    ) -> Result<CallToolResult, McpError> {
        let page = ensure_page(&self.browser).await?;
        let result = onecrawl_cdp::selectors::css_select(&page, &p.selector)
            .await
            .mcp()?;
        json_ok(&result)
    }


    pub(crate) async fn scraping_xpath(
        &self,
        p: XPathParams,
    ) -> Result<CallToolResult, McpError> {
        let page = ensure_page(&self.browser).await?;
        let result = onecrawl_cdp::selectors::xpath_select(&page, &p.expression)
            .await
            .mcp()?;
        json_ok(&result)
    }


    pub(crate) async fn scraping_find_text(
        &self,
        p: FindByTextParams,
    ) -> Result<CallToolResult, McpError> {
        let page = ensure_page(&self.browser).await?;
        let result =
            onecrawl_cdp::selectors::find_by_text(&page, &p.text, p.tag.as_deref())
                .await
                .mcp()?;
        json_ok(&result)
    }


    pub(crate) async fn scraping_text(
        &self,
        p: ExtractTextParams,
    ) -> Result<CallToolResult, McpError> {
        let page = ensure_page(&self.browser).await?;
        let result = onecrawl_cdp::extract::extract(
            &page,
            p.selector.as_deref(),
            onecrawl_cdp::ExtractFormat::Text,
        )
        .await
        .mcp()?;
        json_ok(&result)
    }


    pub(crate) async fn scraping_html(
        &self,
        p: ExtractHtmlParams,
    ) -> Result<CallToolResult, McpError> {
        let page = ensure_page(&self.browser).await?;
        let result = onecrawl_cdp::extract::extract(
            &page,
            p.selector.as_deref(),
            onecrawl_cdp::ExtractFormat::Html,
        )
        .await
        .mcp()?;
        json_ok(&result)
    }


    pub(crate) async fn scraping_markdown(
        &self,
        p: ExtractMarkdownParams,
    ) -> Result<CallToolResult, McpError> {
        let page = ensure_page(&self.browser).await?;
        let result = onecrawl_cdp::extract::extract(
            &page,
            p.selector.as_deref(),
            onecrawl_cdp::ExtractFormat::Markdown,
        )
        .await
        .mcp()?;
        json_ok(&result)
    }


    pub(crate) async fn scraping_structured(&self) -> Result<CallToolResult, McpError> {
        let page = ensure_page(&self.browser).await?;
        let result = onecrawl_cdp::structured_data::extract_all(&page)
            .await
            .mcp()?;
        json_ok(&result)
    }


    pub(crate) async fn scraping_stream(
        &self,
        p: StreamExtractParams,
    ) -> Result<CallToolResult, McpError> {
        let page = ensure_page(&self.browser).await?;
        let fields: Vec<onecrawl_cdp::ExtractionRule> = parse_json_str(&p.fields, "fields")?;
        let pagination: Option<onecrawl_cdp::PaginationConfig> =
            parse_opt_json_str(p.pagination.as_deref(), "pagination")?;
        let schema = onecrawl_cdp::ExtractionSchema {
            item_selector: p.item_selector,
            fields,
            pagination,
        };
        let result = if schema.pagination.is_some() {
            onecrawl_cdp::streaming::extract_with_pagination(&page, &schema)
                .await
                .mcp()?
        } else {
            onecrawl_cdp::streaming::extract_items(&page, &schema)
                .await
                .mcp()?
        };
        json_ok(&result)
    }


    pub(crate) async fn scraping_detect_forms(
        &self,
        _p: DetectFormsParams,
    ) -> Result<CallToolResult, McpError> {
        let page = ensure_page(&self.browser).await?;
        let forms = onecrawl_cdp::form_filler::detect_forms(&page)
            .await
            .mcp()?;
        json_ok(&forms)
    }


    pub(crate) async fn scraping_fill_form(
        &self,
        p: FillFormParams,
    ) -> Result<CallToolResult, McpError> {
        let page = ensure_page(&self.browser).await?;
        let values: HashMap<String, String> = parse_json_str(&p.data, "data")?;
        let result =
            onecrawl_cdp::form_filler::fill_form(&page, &p.form_selector, &values)
                .await
                .mcp()?;
        if p.submit.unwrap_or(false) {
            onecrawl_cdp::form_filler::submit_form(&page, &p.form_selector)
                .await
                .mcp()?;
        }
        json_ok(&result)
    }


    pub(crate) async fn scraping_snapshot_diff(
        &self,
        p: SnapshotDiffParams,
    ) -> Result<CallToolResult, McpError> {
        let result = onecrawl_cdp::snapshot_diff::diff_snapshots(&p.before, &p.after);
        json_ok(&result)
    }
}
