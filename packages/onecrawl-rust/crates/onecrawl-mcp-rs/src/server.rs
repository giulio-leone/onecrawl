use base64::{Engine as _, engine::general_purpose::STANDARD as B64};
use rmcp::{
    ErrorData as McpError, ServerHandler,
    handler::server::{router::tool::ToolRouter, tool::Parameters},
    model::*,
    schemars, tool, tool_router,
};
use std::collections::HashMap;
use std::sync::Arc;

use crate::cdp_tools::*;

// ──────────────────────────── Parameter types ────────────────────────────

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

// ──────────────────────────── Server ────────────────────────────

#[derive(Clone)]
pub struct OneCrawlMcp {
    #[allow(dead_code)] // accessed via #[tool_router] proc macro
    tool_router: ToolRouter<Self>,
    store_path: Arc<String>,
    store_password: Arc<String>,
    browser: SharedBrowser,
}

fn mcp_err(msg: impl Into<String>) -> McpError {
    McpError::internal_error(msg.into(), None)
}

/// Ensure browser session + page are initialised, return a clone of the page handle.
async fn ensure_page(browser: &SharedBrowser) -> Result<chromiumoxide::Page, McpError> {
    let mut state = browser.lock().await;
    if state.session.is_none() {
        let session = onecrawl_cdp::BrowserSession::launch_headless()
            .await
            .map_err(|e| mcp_err(format!("browser launch failed: {e}")))?;
        let page = session
            .new_page("about:blank")
            .await
            .map_err(|e| mcp_err(format!("new page failed: {e}")))?;
        state.session = Some(session);
        state.page = Some(page);
    }
    state
        .page
        .clone()
        .ok_or_else(|| mcp_err("no active page"))
}

fn json_ok(value: &impl serde::Serialize) -> Result<CallToolResult, McpError> {
    let json = serde_json::to_string(value).map_err(|e| mcp_err(e.to_string()))?;
    Ok(CallToolResult::success(vec![Content::text(json)]))
}

fn text_ok(msg: impl Into<String>) -> Result<CallToolResult, McpError> {
    Ok(CallToolResult::success(vec![Content::text(msg.into())]))
}

#[tool_router]
impl OneCrawlMcp {
    pub fn new(store_path: String, store_password: String) -> Self {
        Self {
            tool_router: Self::tool_router(),
            store_path: Arc::new(store_path),
            store_password: Arc::new(store_password),
            browser: new_shared_browser(),
        }
    }

    fn open_store(&self) -> Result<onecrawl_storage::EncryptedStore, McpError> {
        onecrawl_storage::EncryptedStore::open(
            std::path::Path::new(self.store_path.as_ref()),
            &self.store_password,
        )
        .map_err(|e| mcp_err(e.to_string()))
    }

    // ── Crypto tools ──

    #[tool(
        description = "Encrypt text with AES-256-GCM. Returns base64-encoded ciphertext (salt+nonce+ct)."
    )]
    fn encrypt(
        &self,
        Parameters(req): Parameters<EncryptRequest>,
    ) -> Result<CallToolResult, McpError> {
        let payload = onecrawl_crypto::encrypt(req.plaintext.as_bytes(), &req.password)
            .map_err(|e| mcp_err(e.to_string()))?;

        let salt = B64
            .decode(&payload.salt)
            .map_err(|e| mcp_err(e.to_string()))?;
        let nonce = B64
            .decode(&payload.nonce)
            .map_err(|e| mcp_err(e.to_string()))?;
        let ct = B64
            .decode(&payload.ciphertext)
            .map_err(|e| mcp_err(e.to_string()))?;

        let mut packed = Vec::with_capacity(salt.len() + nonce.len() + ct.len());
        packed.extend_from_slice(&salt);
        packed.extend_from_slice(&nonce);
        packed.extend_from_slice(&ct);

        Ok(CallToolResult::success(vec![Content::text(
            B64.encode(&packed),
        )]))
    }

    #[tool(description = "Decrypt base64-encoded AES-256-GCM ciphertext (salt+nonce+ct).")]
    fn decrypt(
        &self,
        Parameters(req): Parameters<DecryptRequest>,
    ) -> Result<CallToolResult, McpError> {
        let raw = B64
            .decode(&req.ciphertext)
            .map_err(|e| mcp_err(format!("invalid base64: {e}")))?;

        if raw.len() < 29 {
            return Err(mcp_err(
                "ciphertext too short: need at least 29 bytes (16 salt + 12 nonce + 1 ct)",
            ));
        }

        let payload = onecrawl_core::EncryptedPayload {
            salt: B64.encode(&raw[..16]),
            nonce: B64.encode(&raw[16..28]),
            ciphertext: B64.encode(&raw[28..]),
        };

        let plaintext = onecrawl_crypto::decrypt(&payload, &req.password)
            .map_err(|e| mcp_err(e.to_string()))?;

        let text = String::from_utf8(plaintext).unwrap_or_else(|e| B64.encode(e.into_bytes()));

        Ok(CallToolResult::success(vec![Content::text(text)]))
    }

    #[tool(description = "Generate a PKCE S256 challenge pair (code_verifier + code_challenge).")]
    fn generate_pkce(&self) -> Result<CallToolResult, McpError> {
        let challenge =
            onecrawl_crypto::generate_pkce_challenge().map_err(|e| mcp_err(e.to_string()))?;
        let json = serde_json::json!({
            "code_verifier": challenge.code_verifier,
            "code_challenge": challenge.code_challenge,
        });
        Ok(CallToolResult::success(vec![Content::text(
            json.to_string(),
        )]))
    }

    #[tool(description = "Generate a 6-digit TOTP code from a base32 secret.")]
    fn generate_totp(
        &self,
        Parameters(req): Parameters<TotpRequest>,
    ) -> Result<CallToolResult, McpError> {
        let config = onecrawl_core::TotpConfig {
            secret: req.secret,
            ..Default::default()
        };
        let code =
            onecrawl_crypto::totp::generate_totp(&config).map_err(|e| mcp_err(e.to_string()))?;
        Ok(CallToolResult::success(vec![Content::text(code)]))
    }

    // ── Parser tools ──

    #[tool(description = "Parse HTML into an accessibility tree (text representation).")]
    fn parse_accessibility_tree(
        &self,
        Parameters(req): Parameters<HtmlRequest>,
    ) -> Result<CallToolResult, McpError> {
        let tree = onecrawl_parser::get_accessibility_tree(&req.html)
            .map_err(|e| mcp_err(e.to_string()))?;
        let rendered = onecrawl_parser::accessibility::render_tree(&tree, 0, false);
        Ok(CallToolResult::success(vec![Content::text(rendered)]))
    }

    #[tool(
        description = "Query HTML with a CSS selector. Returns JSON array of matching elements."
    )]
    fn query_selector(
        &self,
        Parameters(req): Parameters<SelectorRequest>,
    ) -> Result<CallToolResult, McpError> {
        let elements = onecrawl_parser::query_selector(&req.html, &req.selector)
            .map_err(|e| mcp_err(e.to_string()))?;
        let json = serde_json::to_string(&elements).map_err(|e| mcp_err(e.to_string()))?;
        Ok(CallToolResult::success(vec![Content::text(json)]))
    }

    #[tool(description = "Extract visible text from HTML.")]
    fn html_extract_text(
        &self,
        Parameters(req): Parameters<HtmlRequest>,
    ) -> Result<CallToolResult, McpError> {
        let texts =
            onecrawl_parser::extract_text(&req.html, "body").map_err(|e| mcp_err(e.to_string()))?;
        Ok(CallToolResult::success(vec![Content::text(
            texts.join("\n"),
        )]))
    }

    #[tool(
        description = "Extract all links from HTML. Returns JSON array with href, text, is_external."
    )]
    fn html_extract_links(
        &self,
        Parameters(req): Parameters<HtmlRequest>,
    ) -> Result<CallToolResult, McpError> {
        let links = onecrawl_parser::extract::extract_links(&req.html)
            .map_err(|e| mcp_err(e.to_string()))?;
        let result: Vec<serde_json::Value> = links
            .into_iter()
            .map(|(href, text)| {
                let is_external = href.starts_with("http://") || href.starts_with("https://");
                serde_json::json!({ "href": href, "text": text, "is_external": is_external })
            })
            .collect();
        let json = serde_json::to_string(&result).map_err(|e| mcp_err(e.to_string()))?;
        Ok(CallToolResult::success(vec![Content::text(json)]))
    }

    // ── Storage tools ──

    #[tool(description = "Store a key-value pair in encrypted storage.")]
    fn store_set(
        &self,
        Parameters(req): Parameters<StoreSetRequest>,
    ) -> Result<CallToolResult, McpError> {
        let store = self.open_store()?;
        store
            .set(&req.key, req.value.as_bytes())
            .map_err(|e| mcp_err(e.to_string()))?;
        Ok(CallToolResult::success(vec![Content::text(format!(
            "stored key \"{}\"",
            req.key
        ))]))
    }

    #[tool(description = "Retrieve a value from encrypted storage by key.")]
    fn store_get(
        &self,
        Parameters(req): Parameters<StoreGetRequest>,
    ) -> Result<CallToolResult, McpError> {
        let store = self.open_store()?;
        let value = store.get(&req.key).map_err(|e| mcp_err(e.to_string()))?;
        match value {
            Some(v) => {
                let text = String::from_utf8(v).unwrap_or_else(|e| B64.encode(e.into_bytes()));
                Ok(CallToolResult::success(vec![Content::text(text)]))
            }
            None => Ok(CallToolResult::success(vec![Content::text(format!(
                "key \"{}\" not found",
                req.key
            ))])),
        }
    }

    #[tool(description = "List all keys in encrypted storage.")]
    fn store_list(&self) -> Result<CallToolResult, McpError> {
        let store = self.open_store()?;
        let keys = store.list("").map_err(|e| mcp_err(e.to_string()))?;
        let json = serde_json::to_string(&keys).map_err(|e| mcp_err(e.to_string()))?;
        Ok(CallToolResult::success(vec![Content::text(json)]))
    }

    // ════════════════════════════════════════════════════════════════
    //  CDP tools — Navigation & Page Control
    // ════════════════════════════════════════════════════════════════

    #[tool(name = "navigation.goto", description = "Navigate the browser to a URL. Launches a headless browser on first call.")]
    async fn navigation_goto(
        &self,
        Parameters(p): Parameters<NavigateParams>,
    ) -> Result<CallToolResult, McpError> {
        let page = ensure_page(&self.browser).await?;
        onecrawl_cdp::navigation::goto(&page, &p.url)
            .await
            .map_err(|e| mcp_err(e.to_string()))?;
        let title = onecrawl_cdp::navigation::get_title(&page)
            .await
            .unwrap_or_default();
        text_ok(format!("navigated to {} — title: {title}", p.url))
    }

    #[tool(name = "navigation.click", description = "Click an element on the page by CSS selector.")]
    async fn navigation_click(
        &self,
        Parameters(p): Parameters<ClickParams>,
    ) -> Result<CallToolResult, McpError> {
        let page = ensure_page(&self.browser).await?;
        onecrawl_cdp::element::click(&page, &p.selector)
            .await
            .map_err(|e| mcp_err(e.to_string()))?;
        text_ok(format!("clicked {}", p.selector))
    }

    #[tool(name = "navigation.type", description = "Type text into an input element identified by CSS selector.")]
    async fn navigation_type(
        &self,
        Parameters(p): Parameters<TypeTextParams>,
    ) -> Result<CallToolResult, McpError> {
        let page = ensure_page(&self.browser).await?;
        onecrawl_cdp::element::type_text(&page, &p.selector, &p.text)
            .await
            .map_err(|e| mcp_err(e.to_string()))?;
        text_ok(format!("typed {} chars into {}", p.text.len(), p.selector))
    }

    #[tool(
        name = "navigation.screenshot",
        description = "Take a screenshot of the current page as base64-encoded PNG. Optionally target a specific element or capture the full scrollable page."
    )]
    async fn navigation_screenshot(
        &self,
        Parameters(p): Parameters<ScreenshotParams>,
    ) -> Result<CallToolResult, McpError> {
        let page = ensure_page(&self.browser).await?;
        let bytes = if let Some(sel) = &p.selector {
            onecrawl_cdp::screenshot::screenshot_element(&page, sel)
                .await
                .map_err(|e| mcp_err(e.to_string()))?
        } else if p.full_page.unwrap_or(false) {
            onecrawl_cdp::screenshot::screenshot_full(&page)
                .await
                .map_err(|e| mcp_err(e.to_string()))?
        } else {
            onecrawl_cdp::screenshot::screenshot_viewport(&page)
                .await
                .map_err(|e| mcp_err(e.to_string()))?
        };
        let b64 = B64.encode(&bytes);
        Ok(CallToolResult::success(vec![Content::image(
            b64,
            "image/png",
        )]))
    }

    #[tool(name = "navigation.pdf", description = "Export the current page as a PDF document. Returns base64-encoded PDF data.")]
    async fn navigation_pdf(
        &self,
        Parameters(p): Parameters<PdfExportParams>,
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
            .map_err(|e| mcp_err(e.to_string()))?;
        let b64 = B64.encode(&bytes);
        text_ok(format!(
            "pdf exported ({} bytes, base64 length {})",
            bytes.len(),
            b64.len()
        ))
    }

    #[tool(name = "navigation.back", description = "Navigate back in browser history.")]
    async fn navigation_back(&self) -> Result<CallToolResult, McpError> {
        let page = ensure_page(&self.browser).await?;
        onecrawl_cdp::navigation::go_back(&page)
            .await
            .map_err(|e| mcp_err(e.to_string()))?;
        text_ok("navigated back")
    }

    #[tool(name = "navigation.forward", description = "Navigate forward in browser history.")]
    async fn navigation_forward(&self) -> Result<CallToolResult, McpError> {
        let page = ensure_page(&self.browser).await?;
        onecrawl_cdp::navigation::go_forward(&page)
            .await
            .map_err(|e| mcp_err(e.to_string()))?;
        text_ok("navigated forward")
    }

    #[tool(name = "navigation.reload", description = "Reload the current page.")]
    async fn navigation_reload(&self) -> Result<CallToolResult, McpError> {
        let page = ensure_page(&self.browser).await?;
        onecrawl_cdp::navigation::reload(&page)
            .await
            .map_err(|e| mcp_err(e.to_string()))?;
        text_ok("page reloaded")
    }

    #[tool(name = "navigation.wait", description = "Wait for a CSS selector to appear in the DOM within an optional timeout.")]
    async fn navigation_wait(
        &self,
        Parameters(p): Parameters<WaitForSelectorParams>,
    ) -> Result<CallToolResult, McpError> {
        let page = ensure_page(&self.browser).await?;
        let timeout = p.timeout_ms.unwrap_or(30_000);
        onecrawl_cdp::navigation::wait_for_selector(&page, &p.selector, timeout)
            .await
            .map_err(|e| mcp_err(e.to_string()))?;
        text_ok(format!("selector {} found", p.selector))
    }

    #[tool(name = "navigation.evaluate", description = "Evaluate arbitrary JavaScript in the browser page context. Returns the result as JSON.")]
    async fn navigation_evaluate(
        &self,
        Parameters(p): Parameters<EvaluateJsParams>,
    ) -> Result<CallToolResult, McpError> {
        let page = ensure_page(&self.browser).await?;
        let result = onecrawl_cdp::page::evaluate_js(&page, &p.js)
            .await
            .map_err(|e| mcp_err(e.to_string()))?;
        json_ok(&result)
    }

    // ════════════════════════════════════════════════════════════════
    //  CDP tools — Scraping & Extraction
    // ════════════════════════════════════════════════════════════════

    #[tool(
        name = "scraping.css",
        description = "Query the live DOM with a CSS selector. Supports ::text and ::attr(name) pseudo-elements. Returns JSON array of matching elements."
    )]
    async fn scraping_css(
        &self,
        Parameters(p): Parameters<CssSelectorParams>,
    ) -> Result<CallToolResult, McpError> {
        let page = ensure_page(&self.browser).await?;
        let result = onecrawl_cdp::selectors::css_select(&page, &p.selector)
            .await
            .map_err(|e| mcp_err(e.to_string()))?;
        json_ok(&result)
    }

    #[tool(name = "scraping.xpath", description = "Query the live DOM with an XPath expression. Returns JSON array of matching elements.")]
    async fn scraping_xpath(
        &self,
        Parameters(p): Parameters<XPathParams>,
    ) -> Result<CallToolResult, McpError> {
        let page = ensure_page(&self.browser).await?;
        let result = onecrawl_cdp::selectors::xpath_select(&page, &p.expression)
            .await
            .map_err(|e| mcp_err(e.to_string()))?;
        json_ok(&result)
    }

    #[tool(name = "scraping.find_text", description = "Find elements by visible text content. Optionally restrict search to a specific HTML tag.")]
    async fn scraping_find_text(
        &self,
        Parameters(p): Parameters<FindByTextParams>,
    ) -> Result<CallToolResult, McpError> {
        let page = ensure_page(&self.browser).await?;
        let result =
            onecrawl_cdp::selectors::find_by_text(&page, &p.text, p.tag.as_deref())
                .await
                .map_err(|e| mcp_err(e.to_string()))?;
        json_ok(&result)
    }

    #[tool(name = "scraping.text", description = "Extract visible text content from the live page, optionally scoped to a CSS selector.")]
    async fn scraping_text(
        &self,
        Parameters(p): Parameters<ExtractTextParams>,
    ) -> Result<CallToolResult, McpError> {
        let page = ensure_page(&self.browser).await?;
        let result = onecrawl_cdp::extract::extract(
            &page,
            p.selector.as_deref(),
            onecrawl_cdp::ExtractFormat::Text,
        )
        .await
        .map_err(|e| mcp_err(e.to_string()))?;
        json_ok(&result)
    }

    #[tool(name = "scraping.html", description = "Extract raw HTML from the live page, optionally scoped to a CSS selector.")]
    async fn scraping_html(
        &self,
        Parameters(p): Parameters<ExtractHtmlParams>,
    ) -> Result<CallToolResult, McpError> {
        let page = ensure_page(&self.browser).await?;
        let result = onecrawl_cdp::extract::extract(
            &page,
            p.selector.as_deref(),
            onecrawl_cdp::ExtractFormat::Html,
        )
        .await
        .map_err(|e| mcp_err(e.to_string()))?;
        json_ok(&result)
    }

    #[tool(name = "scraping.markdown", description = "Extract page content as clean Markdown, optionally scoped to a CSS selector.")]
    async fn scraping_markdown(
        &self,
        Parameters(p): Parameters<ExtractMarkdownParams>,
    ) -> Result<CallToolResult, McpError> {
        let page = ensure_page(&self.browser).await?;
        let result = onecrawl_cdp::extract::extract(
            &page,
            p.selector.as_deref(),
            onecrawl_cdp::ExtractFormat::Markdown,
        )
        .await
        .map_err(|e| mcp_err(e.to_string()))?;
        json_ok(&result)
    }

    #[tool(
        name = "scraping.structured",
        description = "Extract structured data from the page including JSON-LD, OpenGraph, Twitter Card, and meta tags."
    )]
    async fn scraping_structured(&self) -> Result<CallToolResult, McpError> {
        let page = ensure_page(&self.browser).await?;
        let result = onecrawl_cdp::structured_data::extract_all(&page)
            .await
            .map_err(|e| mcp_err(e.to_string()))?;
        json_ok(&result)
    }

    #[tool(
        name = "scraping.stream",
        description = "Schema-based extraction of repeating items using field rules with optional pagination support."
    )]
    async fn scraping_stream(
        &self,
        Parameters(p): Parameters<StreamExtractParams>,
    ) -> Result<CallToolResult, McpError> {
        let page = ensure_page(&self.browser).await?;
        let fields: Vec<onecrawl_cdp::ExtractionRule> = serde_json::from_str(&p.fields)
            .map_err(|e| mcp_err(format!("invalid fields JSON: {e}")))?;
        let pagination: Option<onecrawl_cdp::PaginationConfig> = match &p.pagination {
            Some(s) => Some(
                serde_json::from_str(s)
                    .map_err(|e| mcp_err(format!("invalid pagination JSON: {e}")))?,
            ),
            None => None,
        };
        let schema = onecrawl_cdp::ExtractionSchema {
            item_selector: p.item_selector,
            fields,
            pagination,
        };
        let result = if schema.pagination.is_some() {
            onecrawl_cdp::streaming::extract_with_pagination(&page, &schema)
                .await
                .map_err(|e| mcp_err(e.to_string()))?
        } else {
            onecrawl_cdp::streaming::extract_items(&page, &schema)
                .await
                .map_err(|e| mcp_err(e.to_string()))?
        };
        json_ok(&result)
    }

    #[tool(name = "scraping.detect_forms", description = "Detect all forms on the current page and enumerate their fields, types, and attributes.")]
    async fn scraping_detect_forms(
        &self,
        Parameters(_p): Parameters<DetectFormsParams>,
    ) -> Result<CallToolResult, McpError> {
        let page = ensure_page(&self.browser).await?;
        let forms = onecrawl_cdp::form_filler::detect_forms(&page)
            .await
            .map_err(|e| mcp_err(e.to_string()))?;
        json_ok(&forms)
    }

    #[tool(name = "scraping.fill_form", description = "Fill form fields by selector-to-value mapping and optionally submit the form.")]
    async fn scraping_fill_form(
        &self,
        Parameters(p): Parameters<FillFormParams>,
    ) -> Result<CallToolResult, McpError> {
        let page = ensure_page(&self.browser).await?;
        let values: HashMap<String, String> = serde_json::from_str(&p.data)
            .map_err(|e| mcp_err(format!("invalid data JSON: {e}")))?;
        let result =
            onecrawl_cdp::form_filler::fill_form(&page, &p.form_selector, &values)
                .await
                .map_err(|e| mcp_err(e.to_string()))?;
        if p.submit.unwrap_or(false) {
            onecrawl_cdp::form_filler::submit_form(&page, &p.form_selector)
                .await
                .map_err(|e| mcp_err(e.to_string()))?;
        }
        json_ok(&result)
    }

    // ════════════════════════════════════════════════════════════════
    //  CDP tools — Crawling
    // ════════════════════════════════════════════════════════════════

    #[tool(
        name = "crawling.spider",
        description = "Crawl a website starting from one or more seed URLs. Follows links with configurable depth, domain, and pattern filters."
    )]
    async fn crawling_spider(
        &self,
        Parameters(p): Parameters<SpiderCrawlParams>,
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
            .map_err(|e| mcp_err(e.to_string()))?;
        let summary = onecrawl_cdp::spider::summarize(&results);
        json_ok(&serde_json::json!({
            "summary": summary,
            "pages_crawled": results.len(),
        }))
    }

    #[tool(name = "crawling.robots", description = "Fetch and parse robots.txt for a domain. Optionally test if a specific path is allowed for a given user-agent.")]
    async fn crawling_robots(
        &self,
        Parameters(p): Parameters<CheckRobotsParams>,
    ) -> Result<CallToolResult, McpError> {
        let page = ensure_page(&self.browser).await?;
        let robots = onecrawl_cdp::robots::fetch_robots(&page, &p.base_url)
            .await
            .map_err(|e| mcp_err(e.to_string()))?;
        let ua = p.user_agent.as_deref().unwrap_or("*");
        let sitemaps = onecrawl_cdp::robots::get_sitemaps(&robots);
        let delay = onecrawl_cdp::robots::get_crawl_delay(&robots, ua);
        let allowed = p.path.as_ref().map(|path| {
            onecrawl_cdp::robots::is_allowed(&robots, ua, path)
        });
        json_ok(&serde_json::json!({
            "sitemaps": sitemaps,
            "crawl_delay": delay,
            "path_allowed": allowed,
        }))
    }

    #[tool(name = "crawling.sitemap", description = "Generate a standards-compliant XML sitemap from a list of URL entries with priority and changefreq.")]
    fn crawling_sitemap(
        &self,
        Parameters(p): Parameters<GenerateSitemapParams>,
    ) -> Result<CallToolResult, McpError> {
        let entries: Vec<onecrawl_cdp::sitemap::SitemapEntry> = serde_json::from_str(&p.entries)
            .map_err(|e| mcp_err(format!("invalid entries JSON: {e}")))?;
        let config = onecrawl_cdp::sitemap::SitemapConfig {
            base_url: p.base_url,
            default_changefreq: p.default_changefreq.unwrap_or_else(|| "weekly".into()),
            default_priority: 0.5,
            include_lastmod: true,
        };
        let xml = onecrawl_cdp::sitemap::generate_sitemap(&entries, &config);
        text_ok(xml)
    }

    #[tool(name = "crawling.snapshot", description = "Take a labeled DOM snapshot of the current page for later comparison with crawling.compare.")]
    async fn crawling_snapshot(
        &self,
        Parameters(p): Parameters<TakeSnapshotParams>,
    ) -> Result<CallToolResult, McpError> {
        let page = ensure_page(&self.browser).await?;
        let snap = onecrawl_cdp::snapshot::take_snapshot(&page)
            .await
            .map_err(|e| mcp_err(e.to_string()))?;
        let mut state = self.browser.lock().await;
        state.snapshots.insert(p.label.clone(), snap);
        text_ok(format!("snapshot '{}' saved", p.label))
    }

    #[tool(name = "crawling.compare", description = "Compare two previously taken DOM snapshots by label and return a structured diff report.")]
    async fn crawling_compare(
        &self,
        Parameters(p): Parameters<CompareSnapshotsParams>,
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

    // ════════════════════════════════════════════════════════════════
    //  CDP tools — Stealth & Anti-Detection
    // ════════════════════════════════════════════════════════════════

    #[tool(
        name = "stealth.inject",
        description = "Inject comprehensive stealth anti-bot patches into the browser page. Returns list of applied patches."
    )]
    async fn stealth_inject(
        &self,
        Parameters(_p): Parameters<InjectStealthParams>,
    ) -> Result<CallToolResult, McpError> {
        let page = ensure_page(&self.browser).await?;
        let patches = onecrawl_cdp::antibot::inject_stealth_full(&page)
            .await
            .map_err(|e| mcp_err(e.to_string()))?;
        json_ok(&serde_json::json!({
            "patches_applied": patches.len(),
            "patches": patches,
        }))
    }

    #[tool(
        name = "stealth.test",
        description = "Run bot detection tests on the current page. Returns a detection score and detailed test results."
    )]
    async fn stealth_test(
        &self,
        Parameters(_p): Parameters<BotDetectionTestParams>,
    ) -> Result<CallToolResult, McpError> {
        let page = ensure_page(&self.browser).await?;
        let result = onecrawl_cdp::antibot::bot_detection_test(&page)
            .await
            .map_err(|e| mcp_err(e.to_string()))?;
        json_ok(&result)
    }

    #[tool(name = "stealth.fingerprint", description = "Generate and apply a realistic browser fingerprint with configurable user-agent to evade bot detection.")]
    async fn stealth_fingerprint(
        &self,
        Parameters(p): Parameters<ApplyFingerprintParams>,
    ) -> Result<CallToolResult, McpError> {
        let page = ensure_page(&self.browser).await?;
        let mut fp = onecrawl_cdp::stealth::generate_fingerprint();
        if let Some(ua) = &p.user_agent {
            fp.user_agent = ua.clone();
        }
        let script = onecrawl_cdp::stealth::get_stealth_init_script(&fp);
        onecrawl_cdp::page::evaluate_js(&page, &script)
            .await
            .map_err(|e| mcp_err(e.to_string()))?;
        json_ok(&serde_json::json!({
            "user_agent": fp.user_agent,
            "platform": fp.platform,
        }))
    }

    #[tool(
        name = "stealth.block_domains",
        description = "Block network requests to specified domains or a built-in category such as ads, trackers, or social widgets."
    )]
    async fn stealth_block_domains(
        &self,
        Parameters(p): Parameters<BlockDomainsParams>,
    ) -> Result<CallToolResult, McpError> {
        let page = ensure_page(&self.browser).await?;
        let count = if let Some(cat) = &p.category {
            onecrawl_cdp::domain_blocker::block_category(&page, cat)
                .await
                .map_err(|e| mcp_err(e.to_string()))?
        } else if let Some(domains) = &p.domains {
            onecrawl_cdp::domain_blocker::block_domains(&page, domains)
                .await
                .map_err(|e| mcp_err(e.to_string()))?
        } else {
            return Err(mcp_err(
                "provide either 'domains' or 'category'",
            ));
        };
        text_ok(format!("{count} domains blocked"))
    }

    #[tool(name = "stealth.detect_captcha", description = "Detect CAPTCHAs on the current page. Returns the CAPTCHA type, provider, and confidence score.")]
    async fn stealth_detect_captcha(
        &self,
        Parameters(_p): Parameters<DetectCaptchaParams>,
    ) -> Result<CallToolResult, McpError> {
        let page = ensure_page(&self.browser).await?;
        let detection = onecrawl_cdp::captcha::detect_captcha(&page)
            .await
            .map_err(|e| mcp_err(e.to_string()))?;
        json_ok(&detection)
    }

    // ════════════════════════════════════════════════════════════════
    //  CDP tools — Data Processing
    // ════════════════════════════════════════════════════════════════

    #[tool(name = "data.pipeline", description = "Execute a multi-step data pipeline with filter, transform, sort, and deduplicate operations on JSON input.")]
    fn data_pipeline(
        &self,
        Parameters(p): Parameters<PipelineExecuteParams>,
    ) -> Result<CallToolResult, McpError> {
        let steps: Vec<onecrawl_cdp::PipelineStep> = serde_json::from_str(&p.steps)
            .map_err(|e| mcp_err(format!("invalid steps JSON: {e}")))?;
        let pipeline = onecrawl_cdp::Pipeline {
            name: p.name,
            steps,
        };
        let items: Vec<HashMap<String, String>> = serde_json::from_str(&p.input)
            .map_err(|e| mcp_err(format!("invalid input JSON: {e}")))?;
        let result = onecrawl_cdp::data_pipeline::execute_pipeline(&pipeline, items);
        json_ok(&result)
    }

    #[tool(name = "data.http_get", description = "Perform an HTTP GET request through the browser session. Returns status code, headers, and response body.")]
    async fn data_http_get(
        &self,
        Parameters(p): Parameters<HttpGetParams>,
    ) -> Result<CallToolResult, McpError> {
        let page = ensure_page(&self.browser).await?;
        let headers: Option<HashMap<String, String>> = match &p.headers {
            Some(s) => Some(
                serde_json::from_str(s)
                    .map_err(|e| mcp_err(format!("invalid headers JSON: {e}")))?,
            ),
            None => None,
        };
        let resp = onecrawl_cdp::http_client::get(&page, &p.url, headers)
            .await
            .map_err(|e| mcp_err(e.to_string()))?;
        json_ok(&resp)
    }

    #[tool(name = "data.http_post", description = "Perform an HTTP POST request through the browser session. Returns status code, headers, and response body.")]
    async fn data_http_post(
        &self,
        Parameters(p): Parameters<HttpPostParams>,
    ) -> Result<CallToolResult, McpError> {
        let page = ensure_page(&self.browser).await?;
        let headers: Option<HashMap<String, String>> = match &p.headers {
            Some(s) => Some(
                serde_json::from_str(s)
                    .map_err(|e| mcp_err(format!("invalid headers JSON: {e}")))?,
            ),
            None => None,
        };
        let resp =
            onecrawl_cdp::http_client::post(&page, &p.url, &p.body, "application/json", headers)
                .await
                .map_err(|e| mcp_err(e.to_string()))?;
        json_ok(&resp)
    }

    #[tool(name = "data.links", description = "Extract all links from the live page and return as directed edges suitable for graph analysis.")]
    async fn data_links(
        &self,
        Parameters(p): Parameters<ExtractLinksParams>,
    ) -> Result<CallToolResult, McpError> {
        let page = ensure_page(&self.browser).await?;
        let edges = onecrawl_cdp::link_graph::extract_links(&page, &p.base_url)
            .await
            .map_err(|e| mcp_err(e.to_string()))?;
        json_ok(&edges)
    }

    #[tool(name = "data.graph", description = "Analyze a link graph to compute stats, find orphan pages, identify hubs, and detect broken links.")]
    fn data_graph(
        &self,
        Parameters(p): Parameters<AnalyzeGraphParams>,
    ) -> Result<CallToolResult, McpError> {
        let edges: Vec<onecrawl_cdp::LinkEdge> = serde_json::from_str(&p.edges)
            .map_err(|e| mcp_err(format!("invalid edges JSON: {e}")))?;
        let graph = onecrawl_cdp::link_graph::build_graph(&edges);
        let stats = onecrawl_cdp::link_graph::analyze_graph(&graph);
        json_ok(&stats)
    }

    // ════════════════════════════════════════════════════════════════
    //  CDP tools — Automation
    // ════════════════════════════════════════════════════════════════

    #[tool(name = "automation.rate_limit", description = "Check rate limiter status and whether new requests can proceed. Initializes the limiter on first call.")]
    async fn automation_rate_limit(
        &self,
        Parameters(p): Parameters<RateLimitCheckParams>,
    ) -> Result<CallToolResult, McpError> {
        let mut state = self.browser.lock().await;
        if state.rate_limiter.is_none() {
            let config = onecrawl_cdp::RateLimitConfig {
                max_requests_per_second: p.max_per_second.unwrap_or(2.0),
                max_requests_per_minute: p.max_per_minute.unwrap_or(60.0),
                max_requests_per_hour: 3600.0,
                burst_size: 5,
                cooldown_ms: 500,
            };
            state.rate_limiter = Some(onecrawl_cdp::RateLimitState::new(config));
        }
        let limiter = state.rate_limiter.as_ref().unwrap();
        let can = onecrawl_cdp::rate_limiter::can_proceed(limiter);
        let stats = onecrawl_cdp::rate_limiter::get_stats(limiter);
        json_ok(&serde_json::json!({
            "can_proceed": can,
            "stats": stats,
        }))
    }

    #[tool(name = "automation.retry", description = "Enqueue a failed URL or operation into the retry queue with exponential backoff and jitter.")]
    async fn automation_retry(
        &self,
        Parameters(p): Parameters<RetryEnqueueParams>,
    ) -> Result<CallToolResult, McpError> {
        let mut state = self.browser.lock().await;
        if state.retry_queue.is_none() {
            state.retry_queue = Some(onecrawl_cdp::RetryQueue::new(onecrawl_cdp::RetryConfig {
                max_retries: 3,
                initial_delay_ms: 1000,
                max_delay_ms: 30_000,
                backoff_factor: 2.0,
                jitter: true,
            }));
        }
        let queue = state.retry_queue.as_mut().unwrap();
        let id = onecrawl_cdp::retry_queue::enqueue(
            queue,
            &p.url,
            &p.operation,
            p.payload.as_deref(),
        );
        let stats = onecrawl_cdp::retry_queue::get_stats(queue);
        json_ok(&serde_json::json!({
            "id": id,
            "queue_stats": stats,
        }))
    }

    //  Passkey / WebAuthn tools

    #[tool(description = "Enable a virtual WebAuthn authenticator for passkey simulation.")]
    async fn auth_passkey_enable(
        &self,
        Parameters(p): Parameters<PasskeyEnableParams>,
    ) -> Result<CallToolResult, McpError> {
        let page = ensure_page(&self.browser).await?;
        let config = onecrawl_cdp::webauthn::VirtualAuthenticator {
            id: format!(
                "auth-{}",
                std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_millis()
            ),
            protocol: p.protocol.unwrap_or_else(|| "ctap2".into()),
            transport: p.transport.unwrap_or_else(|| "internal".into()),
            has_resident_key: true,
            has_user_verification: true,
            is_user_verified: true,
        };
        onecrawl_cdp::webauthn::enable_virtual_authenticator(&page, &config)
            .await
            .map_err(|e| mcp_err(e.to_string()))?;
        text_ok("Virtual authenticator enabled")
    }

    #[tool(description = "Add a passkey credential to the virtual authenticator.")]
    async fn auth_passkey_add(
        &self,
        Parameters(p): Parameters<PasskeyAddParams>,
    ) -> Result<CallToolResult, McpError> {
        let page = ensure_page(&self.browser).await?;
        let cred = onecrawl_cdp::webauthn::VirtualCredential {
            credential_id: p.credential_id,
            rp_id: p.rp_id,
            user_handle: p.user_handle.unwrap_or_default(),
            sign_count: 0,
        };
        onecrawl_cdp::webauthn::add_virtual_credential(&page, &cred)
            .await
            .map_err(|e| mcp_err(e.to_string()))?;
        text_ok("Credential added")
    }

    #[tool(description = "List all stored passkey credentials.")]
    async fn auth_passkey_list(
        &self,
        Parameters(_p): Parameters<PasskeyListParams>,
    ) -> Result<CallToolResult, McpError> {
        let page = ensure_page(&self.browser).await?;
        let creds = onecrawl_cdp::webauthn::get_virtual_credentials(&page)
            .await
            .map_err(|e| mcp_err(e.to_string()))?;
        json_ok(&creds)
    }

    #[tool(description = "Get the WebAuthn operation log.")]
    async fn auth_passkey_log(
        &self,
        Parameters(_p): Parameters<PasskeyLogParams>,
    ) -> Result<CallToolResult, McpError> {
        let page = ensure_page(&self.browser).await?;
        let log = onecrawl_cdp::webauthn::get_webauthn_log(&page)
            .await
            .map_err(|e| mcp_err(e.to_string()))?;
        json_ok(&log)
    }

    #[tool(description = "Disable the virtual WebAuthn authenticator.")]
    async fn auth_passkey_disable(
        &self,
        Parameters(_p): Parameters<PasskeyDisableParams>,
    ) -> Result<CallToolResult, McpError> {
        let page = ensure_page(&self.browser).await?;
        onecrawl_cdp::webauthn::disable_virtual_authenticator(&page)
            .await
            .map_err(|e| mcp_err(e.to_string()))?;
        text_ok("Virtual authenticator disabled")
    }

    #[tool(description = "Remove a passkey credential by ID.")]
    async fn auth_passkey_remove(
        &self,
        Parameters(p): Parameters<PasskeyRemoveParams>,
    ) -> Result<CallToolResult, McpError> {
        let page = ensure_page(&self.browser).await?;
        let removed = onecrawl_cdp::webauthn::remove_virtual_credential(&page, &p.credential_id)
            .await
            .map_err(|e| mcp_err(e.to_string()))?;
        json_ok(&serde_json::json!({ "removed": removed }))
    }
}

impl ServerHandler for OneCrawlMcp {
    fn get_info(&self) -> ServerInfo {
        ServerInfo {
            instructions: Some(
                "OneCrawl MCP server — crypto, parser, storage, and CDP browser automation tools"
                    .into(),
            ),
            capabilities: ServerCapabilities::builder().enable_tools().build(),
            ..Default::default()
        }
    }
}
