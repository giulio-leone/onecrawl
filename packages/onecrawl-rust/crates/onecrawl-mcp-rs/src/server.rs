use base64::{Engine as _, engine::general_purpose::STANDARD as B64};
use futures::StreamExt;
use rmcp::{
    ErrorData as McpError, ServerHandler,
    handler::server::{router::tool::ToolRouter, tool::Parameters},
    model::*,
    tool, tool_router,
};
use std::collections::HashMap;
use std::sync::Arc;

use crate::cdp_tools::*;
use crate::helpers::{mcp_err, ensure_page, json_ok, text_ok};
use crate::types::*;

// ──────────────────────────── Server ────────────────────────────

#[derive(Clone)]
pub struct OneCrawlMcp {
    #[allow(dead_code)] // accessed via #[tool_router] proc macro
    tool_router: ToolRouter<Self>,
    store_path: Arc<String>,
    store_password: Arc<String>,
    browser: SharedBrowser,
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

    /// Internal dispatch for `agent.execute_chain`.
    /// Returns `Ok(serde_json::Value)` on success or `Err(String)` with an
    /// error message for that step.
    async fn dispatch_chain_command(
        &self,
        cmd: &ChainCommand,
    ) -> std::result::Result<serde_json::Value, String> {
        let page = ensure_page(&self.browser)
            .await
            .map_err(|e| format!("browser error: {}", e.message))?;

        match cmd.tool.as_str() {
            "navigation.goto" => {
                let url = cmd.args.get("url")
                    .and_then(|v| v.as_str())
                    .ok_or("missing 'url' argument")?;
                onecrawl_cdp::navigation::goto(&page, url)
                    .await
                    .map_err(|e| e.to_string())?;
                let title = onecrawl_cdp::navigation::get_title(&page)
                    .await
                    .unwrap_or_default();
                Ok(serde_json::json!({ "url": url, "title": title }))
            }
            "navigation.click" => {
                let selector_raw = cmd.args.get("selector")
                    .and_then(|v| v.as_str())
                    .ok_or("missing 'selector' argument")?;
                let selector = onecrawl_cdp::accessibility::resolve_ref(selector_raw);
                onecrawl_cdp::element::click(&page, &selector)
                    .await
                    .map_err(|e| e.to_string())?;
                Ok(serde_json::json!({ "clicked": selector_raw }))
            }
            "navigation.type" => {
                let selector_raw = cmd.args.get("selector")
                    .and_then(|v| v.as_str())
                    .ok_or("missing 'selector' argument")?;
                let text = cmd.args.get("text")
                    .and_then(|v| v.as_str())
                    .ok_or("missing 'text' argument")?;
                let selector = onecrawl_cdp::accessibility::resolve_ref(selector_raw);
                onecrawl_cdp::element::type_text(&page, &selector, text)
                    .await
                    .map_err(|e| e.to_string())?;
                Ok(serde_json::json!({ "typed": text.len(), "selector": selector_raw }))
            }
            "navigation.wait" => {
                let selector_raw = cmd.args.get("selector")
                    .and_then(|v| v.as_str())
                    .ok_or("missing 'selector' argument")?;
                let timeout = cmd.args.get("timeout_ms")
                    .and_then(|v| v.as_u64())
                    .unwrap_or(30_000);
                let selector = onecrawl_cdp::accessibility::resolve_ref(selector_raw);
                onecrawl_cdp::navigation::wait_for_selector(&page, &selector, timeout)
                    .await
                    .map_err(|e| e.to_string())?;
                Ok(serde_json::json!({ "found": selector_raw }))
            }
            "navigation.evaluate" => {
                let js = cmd.args.get("js")
                    .and_then(|v| v.as_str())
                    .ok_or("missing 'js' argument")?;
                let result = onecrawl_cdp::page::evaluate_js(&page, js)
                    .await
                    .map_err(|e| e.to_string())?;
                Ok(result)
            }
            "navigation.snapshot" => {
                let opts = onecrawl_cdp::accessibility::AgentSnapshotOptions {
                    interactive_only: cmd.args.get("interactive_only")
                        .and_then(|v| v.as_bool()).unwrap_or(false),
                    cursor: cmd.args.get("cursor")
                        .and_then(|v| v.as_bool()).unwrap_or(false),
                    compact: cmd.args.get("compact")
                        .and_then(|v| v.as_bool()).unwrap_or(false),
                    depth: cmd.args.get("depth")
                        .and_then(|v| v.as_u64()).map(|d| d as usize),
                    selector: cmd.args.get("selector")
                        .and_then(|v| v.as_str()).map(String::from),
                };
                let snap = onecrawl_cdp::accessibility::agent_snapshot(&page, &opts)
                    .await
                    .map_err(|e| e.to_string())?;
                Ok(serde_json::json!({
                    "snapshot": snap.snapshot,
                    "refs": snap.refs,
                    "total": snap.total
                }))
            }
            "scraping.css" => {
                let selector = cmd.args.get("selector")
                    .and_then(|v| v.as_str())
                    .ok_or("missing 'selector' argument")?;
                let result = onecrawl_cdp::selectors::css_select(&page, selector)
                    .await
                    .map_err(|e| e.to_string())?;
                Ok(serde_json::to_value(&result).unwrap_or_default())
            }
            "scraping.text" => {
                let selector = cmd.args.get("selector")
                    .and_then(|v| v.as_str());
                let result = onecrawl_cdp::extract::extract(
                    &page,
                    selector,
                    onecrawl_cdp::ExtractFormat::Text,
                )
                .await
                .map_err(|e| e.to_string())?;
                Ok(serde_json::to_value(&result).unwrap_or_default())
            }
            other => {
                let err = crate::agent_error::unknown_tool(other);
                Err(serde_json::to_string(&err).unwrap_or_else(|_| err.message))
            }
        }
    }

    // ── Crypto tools ──

    #[tool(
        name = "crypto.encrypt",
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

    #[tool(name = "crypto.decrypt", description = "Decrypt base64-encoded AES-256-GCM ciphertext (salt+nonce+ct).")]
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

    #[tool(name = "crypto.generate_pkce", description = "Generate a PKCE S256 challenge pair (code_verifier + code_challenge).")]
    fn generate_pkce(&self) -> Result<CallToolResult, McpError> {
        let challenge =
            onecrawl_crypto::generate_pkce_challenge().map_err(|e| mcp_err(e.to_string()))?;
        json_ok(&PkceResponse {
            code_verifier: &challenge.code_verifier,
            code_challenge: &challenge.code_challenge,
        })
    }

    #[tool(name = "crypto.generate_totp", description = "Generate a 6-digit TOTP code from a base32 secret.")]
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

    #[tool(name = "parser.parse_a11y_tree", description = "Parse HTML into an accessibility tree (text representation).")]
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
        name = "parser.query_selector",
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

    #[tool(name = "parser.extract_text", description = "Extract visible text from HTML.")]
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
        name = "parser.extract_links",
        description = "Extract all links from HTML. Returns JSON array with href, text, is_external."
    )]
    fn html_extract_links(
        &self,
        Parameters(req): Parameters<HtmlRequest>,
    ) -> Result<CallToolResult, McpError> {
        let links = onecrawl_parser::extract::extract_links(&req.html)
            .map_err(|e| mcp_err(e.to_string()))?;
        let result: Vec<LinkInfo> = links
            .into_iter()
            .map(|(href, text)| {
                let is_external = href.starts_with("http://") || href.starts_with("https://");
                LinkInfo { href, text, is_external }
            })
            .collect();
        let json = serde_json::to_string(&result).map_err(|e| mcp_err(e.to_string()))?;
        Ok(CallToolResult::success(vec![Content::text(json)]))
    }

    // ── Storage tools ──

    #[tool(name = "storage.set", description = "Store a key-value pair in encrypted storage.")]
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

    #[tool(name = "storage.get", description = "Retrieve a value from encrypted storage by key.")]
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

    #[tool(name = "storage.list_keys", description = "List all keys in encrypted storage.")]
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

    #[tool(name = "navigation.click", description = "Click an element on the page by CSS selector or @ref (e.g. @e1 from snapshot).")]
    async fn navigation_click(
        &self,
        Parameters(p): Parameters<ClickParams>,
    ) -> Result<CallToolResult, McpError> {
        let page = ensure_page(&self.browser).await?;
        let selector = onecrawl_cdp::accessibility::resolve_ref(&p.selector);
        onecrawl_cdp::element::click(&page, &selector)
            .await
            .map_err(|e| mcp_err(e.to_string()))?;
        text_ok(format!("clicked {}", p.selector))
    }

    #[tool(name = "navigation.type", description = "Type text into an input element identified by CSS selector or @ref (e.g. @e1 from snapshot).")]
    async fn navigation_type(
        &self,
        Parameters(p): Parameters<TypeTextParams>,
    ) -> Result<CallToolResult, McpError> {
        let page = ensure_page(&self.browser).await?;
        let selector = onecrawl_cdp::accessibility::resolve_ref(&p.selector);
        onecrawl_cdp::element::type_text(&page, &selector, &p.text)
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

    #[tool(name = "navigation.wait", description = "Wait for a CSS selector or @ref to appear in the DOM within an optional timeout.")]
    async fn navigation_wait(
        &self,
        Parameters(p): Parameters<WaitForSelectorParams>,
    ) -> Result<CallToolResult, McpError> {
        let page = ensure_page(&self.browser).await?;
        let timeout = p.timeout_ms.unwrap_or(30_000);
        let selector = onecrawl_cdp::accessibility::resolve_ref(&p.selector);
        onecrawl_cdp::navigation::wait_for_selector(&page, &selector, timeout)
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

    #[tool(
        name = "navigation.snapshot",
        description = "Take an AI-optimized accessibility snapshot of the page. Returns element refs (@e1, @e2...) that can be used with click, fill, type commands. Use --interactive to get only actionable elements."
    )]
    async fn navigation_snapshot(
        &self,
        Parameters(p): Parameters<AgentSnapshotParams>,
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
            .map_err(|e| mcp_err(e.to_string()))?;
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

    #[tool(
        name = "scraping.snapshot_diff",
        description = "Compute a line-level unified diff between two accessibility snapshot texts. Returns additions, removals, unchanged count, and the unified diff output."
    )]
    async fn scraping_snapshot_diff(
        &self,
        Parameters(p): Parameters<SnapshotDiffParams>,
    ) -> Result<CallToolResult, McpError> {
        let result = onecrawl_cdp::snapshot_diff::diff_snapshots(&p.before, &p.after);
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
        json_ok(&CrawlResult2 {
            summary,
            pages_crawled: results.len(),
        })
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
        json_ok(&RobotsInfo {
            sitemaps,
            crawl_delay: delay,
            path_allowed: allowed,
        })
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
        json_ok(&StealthInjectResult {
            patches_applied: patches.len(),
            patches,
        })
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
        json_ok(&FingerprintResult {
            user_agent: &fp.user_agent,
            platform: &fp.platform,
        })
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
        json_ok(&RateLimitResult {
            can_proceed: can,
            stats,
        })
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
        json_ok(&RetryResult {
            id,
            queue_stats: stats,
        })
    }

    //  Passkey / WebAuthn tools

    #[tool(name = "auth.passkey_enable", description = "Enable a virtual WebAuthn authenticator for passkey simulation.")]
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

    #[tool(name = "auth.passkey_add", description = "Add a passkey credential to the virtual authenticator.")]
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

    #[tool(name = "auth.passkey_list", description = "List all stored passkey credentials.")]
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

    #[tool(name = "auth.passkey_log", description = "Get the WebAuthn operation log.")]
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

    #[tool(name = "auth.passkey_disable", description = "Disable the virtual WebAuthn authenticator.")]
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

    #[tool(name = "auth.passkey_remove", description = "Remove a passkey credential by ID.")]
    async fn auth_passkey_remove(
        &self,
        Parameters(p): Parameters<PasskeyRemoveParams>,
    ) -> Result<CallToolResult, McpError> {
        let page = ensure_page(&self.browser).await?;
        let removed = onecrawl_cdp::webauthn::remove_virtual_credential(&page, &p.credential_id)
            .await
            .map_err(|e| mcp_err(e.to_string()))?;
        json_ok(&RemovedResult { removed })
    }

    // ════════════════════════════════════════════════════════════════
    //  Agent tools — Enhanced Agentic API Layer
    // ════════════════════════════════════════════════════════════════

    #[tool(
        name = "agent.execute_chain",
        description = "Execute multiple commands in sequence. Supports: navigation.goto, navigation.click, navigation.type, navigation.wait, navigation.evaluate, navigation.snapshot, scraping.css, scraping.text. Returns results array with success/error for each step."
    )]
    async fn agent_execute_chain(
        &self,
        Parameters(p): Parameters<ExecuteChainParams>,
    ) -> Result<CallToolResult, McpError> {
        let stop_on_error = p.stop_on_error.unwrap_or(true);
        let total = p.commands.len();
        let mut results: Vec<serde_json::Value> = Vec::with_capacity(total);
        let mut completed = 0usize;

        for cmd in &p.commands {
            let outcome = self.dispatch_chain_command(cmd).await;
            completed += 1;
            match outcome {
                Ok(data) => {
                    results.push(serde_json::json!({
                        "tool": cmd.tool,
                        "success": true,
                        "data": data
                    }));
                }
                Err(err_msg) => {
                    results.push(serde_json::json!({
                        "tool": cmd.tool,
                        "success": false,
                        "error": err_msg
                    }));
                    if stop_on_error {
                        break;
                    }
                }
            }
        }

        json_ok(&serde_json::json!({
            "results": results,
            "completed": completed,
            "total": total
        }))
    }

    #[tool(
        name = "agent.element_screenshot",
        description = "Take a screenshot of a specific element by CSS selector or @ref. Returns base64 PNG with element bounds."
    )]
    async fn agent_element_screenshot(
        &self,
        Parameters(p): Parameters<ElementScreenshotParams>,
    ) -> Result<CallToolResult, McpError> {
        let page = ensure_page(&self.browser).await?;
        let selector = onecrawl_cdp::accessibility::resolve_ref(&p.selector);

        // Get element bounds via JS
        let bounds_js = format!(
            r#"(() => {{
                const el = document.querySelector({sel});
                if (!el) return null;
                const r = el.getBoundingClientRect();
                return {{ x: r.x, y: r.y, width: r.width, height: r.height }};
            }})()"#,
            sel = serde_json::to_string(&selector).unwrap_or_else(|_| "null".into())
        );
        let bounds_val = onecrawl_cdp::page::evaluate_js(&page, &bounds_js)
            .await
            .map_err(|e| mcp_err(e.to_string()))?;

        if bounds_val.is_null() {
            return Err(crate::helpers::agent_err(
                crate::agent_error::element_not_found(&p.selector),
            ));
        }

        let bytes = onecrawl_cdp::screenshot::screenshot_element(&page, &selector)
            .await
            .map_err(|e| mcp_err(e.to_string()))?;
        let b64 = B64.encode(&bytes);

        json_ok(&serde_json::json!({
            "image": b64,
            "bounds": bounds_val
        }))
    }

    #[tool(
        name = "agent.api_capture_start",
        description = "Inject a fetch/XHR interceptor to capture all API calls made by the page. Call agent.api_capture_summary to read the log."
    )]
    async fn agent_api_capture_start(
        &self,
        Parameters(_p): Parameters<ApiCaptureStartParams>,
    ) -> Result<CallToolResult, McpError> {
        let page = ensure_page(&self.browser).await?;
        let js = r#"
        (() => {
            if (window.__onecrawl_api_log) return { already_active: true, entries: window.__onecrawl_api_log.length };
            window.__onecrawl_api_log = [];

            // Patch fetch
            const origFetch = window.fetch;
            window.fetch = async function(...args) {
                const url = typeof args[0] === 'string' ? args[0] : (args[0]?.url || '');
                const method = (args[1]?.method || 'GET').toUpperCase();
                const entry = { type: 'fetch', method, url, status: null, contentType: null, timestamp: Date.now() };
                try {
                    const resp = await origFetch.apply(this, args);
                    entry.status = resp.status;
                    entry.contentType = resp.headers.get('content-type');
                    window.__onecrawl_api_log.push(entry);
                    return resp;
                } catch(e) {
                    entry.error = e.message;
                    window.__onecrawl_api_log.push(entry);
                    throw e;
                }
            };

            // Patch XMLHttpRequest
            const origOpen = XMLHttpRequest.prototype.open;
            const origSend = XMLHttpRequest.prototype.send;
            XMLHttpRequest.prototype.open = function(method, url, ...rest) {
                this.__oc_method = method;
                this.__oc_url = url;
                return origOpen.call(this, method, url, ...rest);
            };
            XMLHttpRequest.prototype.send = function(...args) {
                const xhr = this;
                const entry = { type: 'xhr', method: (xhr.__oc_method || 'GET').toUpperCase(), url: xhr.__oc_url || '', status: null, contentType: null, timestamp: Date.now() };
                xhr.addEventListener('load', function() {
                    entry.status = xhr.status;
                    entry.contentType = xhr.getResponseHeader('content-type');
                    window.__onecrawl_api_log.push(entry);
                });
                xhr.addEventListener('error', function() {
                    entry.error = 'network error';
                    window.__onecrawl_api_log.push(entry);
                });
                return origSend.apply(this, args);
            };

            return { active: true, entries: 0 };
        })()
        "#;
        let result = onecrawl_cdp::page::evaluate_js(&page, js)
            .await
            .map_err(|e| mcp_err(e.to_string()))?;
        json_ok(&result)
    }

    #[tool(
        name = "agent.api_capture_summary",
        description = "Get a summary of all network API calls (fetch/XHR) captured since agent.api_capture_start. Returns method, URL, status, content-type for each request."
    )]
    async fn agent_api_capture_summary(
        &self,
        Parameters(p): Parameters<ApiCaptureSummaryParams>,
    ) -> Result<CallToolResult, McpError> {
        let page = ensure_page(&self.browser).await?;
        let clear = p.clear.unwrap_or(false);
        let js = format!(
            r#"(() => {{
                const log = window.__onecrawl_api_log || [];
                const result = {{ total: log.length, requests: log.slice() }};
                if ({clear}) {{ window.__onecrawl_api_log = []; }}
                return result;
            }})()"#,
            clear = if clear { "true" } else { "false" }
        );
        let result = onecrawl_cdp::page::evaluate_js(&page, &js)
            .await
            .map_err(|e| mcp_err(e.to_string()))?;
        json_ok(&result)
    }

    #[tool(
        name = "agent.iframe_list",
        description = "List all iframes on the current page with metadata (src, name, id, dimensions, sandbox)."
    )]
    async fn agent_iframe_list(
        &self,
        Parameters(_p): Parameters<IframeListParams>,
    ) -> Result<CallToolResult, McpError> {
        let page = ensure_page(&self.browser).await?;
        let iframes = onecrawl_cdp::iframe::list_iframes(&page)
            .await
            .map_err(|e| mcp_err(e.to_string()))?;
        json_ok(&serde_json::json!({
            "total": iframes.len(),
            "iframes": iframes
        }))
    }

    #[tool(
        name = "agent.iframe_snapshot",
        description = "Take an accessibility snapshot inside a specific iframe by index. Returns refs scoped to that iframe for AI-driven automation."
    )]
    async fn agent_iframe_snapshot(
        &self,
        Parameters(p): Parameters<IframeSnapshotParams>,
    ) -> Result<CallToolResult, McpError> {
        let page = ensure_page(&self.browser).await?;
        let interactive_only = if p.interactive_only.unwrap_or(false) { "true" } else { "false" };
        let compact = if p.compact.unwrap_or(false) { "true" } else { "false" };

        // Inject a lightweight snapshot script into the iframe
        let snap_js = format!(
            r#"(() => {{
                const interactiveOnly = {interactive_only};
                const compactMode = {compact};
                const INTERACTIVE_TAGS = new Set(['A','BUTTON','INPUT','SELECT','TEXTAREA','DETAILS','SUMMARY']);
                const INTERACTIVE_ROLES = new Set(['button','link','textbox','checkbox','radio','combobox','listbox','menuitem','tab','switch','searchbox','slider','spinbutton']);
                let refCounter = 0;
                const refs = {{}};
                function walk(node, depth) {{
                    if (!node || node.nodeType !== 1) return '';
                    const tag = node.tagName.toLowerCase();
                    if (tag === 'script' || tag === 'style' || tag === 'noscript') return '';
                    const role = node.getAttribute('role') || '';
                    const isInteractive = INTERACTIVE_TAGS.has(node.tagName) || INTERACTIVE_ROLES.has(role);
                    if (interactiveOnly && !isInteractive && node.children.length === 0) return '';
                    const refId = 'f{idx}e' + (refCounter++);
                    node.setAttribute('data-onecrawl-ref', refId);
                    const label = node.getAttribute('aria-label') || node.getAttribute('alt') || node.getAttribute('placeholder') || '';
                    const text = node.childNodes.length === 1 && node.childNodes[0].nodeType === 3 ? node.childNodes[0].textContent.trim().substring(0, 80) : '';
                    let line = '  '.repeat(depth) + tag;
                    if (role) line += '[role=' + role + ']';
                    line += ' @' + refId;
                    if (label) line += ' "' + label + '"';
                    else if (text) line += ' "' + text + '"';
                    let children = '';
                    for (const c of node.children) {{ children += walk(c, depth + 1); }}
                    if (compactMode && !isInteractive && !children && !text && !label) return '';
                    refs[refId] = tag + (node.id ? '#' + node.id : '') + (node.className && typeof node.className === 'string' ? '.' + node.className.trim().split(/\\s+/).join('.') : '');
                    return line + '\\n' + children;
                }}
                const snapshot = walk(document.body || document.documentElement, 0);
                return {{ snapshot, refs, total: refCounter, iframe_index: {idx} }};
            }})()"#,
            interactive_only = interactive_only,
            compact = compact,
            idx = p.index
        );

        let result = onecrawl_cdp::iframe::eval_in_iframe(&page, p.index, &snap_js)
            .await
            .map_err(|e| mcp_err(e.to_string()))?;

        if let Some(err) = result.get("error") {
            return Err(mcp_err(format!("iframe snapshot failed: {}", err)));
        }

        json_ok(&result)
    }

    #[tool(
        name = "agent.connect_remote",
        description = "Connect to a remote CDP WebSocket endpoint (e.g. Browserbase, BrowserCloud). Subsequent tools will use the remote browser."
    )]
    async fn agent_connect_remote(
        &self,
        Parameters(p): Parameters<RemoteCdpParams>,
    ) -> Result<CallToolResult, McpError> {
        let mut state = self.browser.lock().await;

        // Validate the WebSocket URL format
        if !p.ws_url.starts_with("ws://") && !p.ws_url.starts_with("wss://") {
            return Err(mcp_err("ws_url must start with ws:// or wss://"));
        }

        // Connect to remote browser via chromiumoxide
        let (browser, mut handler) =
            chromiumoxide::Browser::connect(&p.ws_url)
                .await
                .map_err(|e| mcp_err(format!("remote CDP connect failed: {e}")))?;

        // Spawn the handler loop
        tokio::spawn(async move {
            while let Some(_event) = handler.next().await {}
        });

        let page = browser
            .new_page("about:blank")
            .await
            .map_err(|e| mcp_err(format!("remote new_page failed: {e}")))?;

        // Store in shared state (replace any existing session)
        state.session = None; // drop local session
        state.page = Some(page);

        let _ = &p.headers; // reserved for future handshake header support

        json_ok(&serde_json::json!({
            "connected": true,
            "ws_url": p.ws_url,
            "info": "Remote browser connected. Subsequent tools will use this session."
        }))
    }

    // ════════════════════════════════════════════════════════════════
    //  Safety Policy tools
    // ════════════════════════════════════════════════════════════════

    #[tool(
        name = "agent.safety_policy_set",
        description = "Set or update the safety policy for this session. Controls allowed/blocked domains, URL patterns, commands, rate limits, and confirmation requirements. Pass a policy_file path to load from JSON, or set fields directly."
    )]
    async fn agent_safety_policy_set(
        &self,
        Parameters(p): Parameters<SafetyPolicySetParams>,
    ) -> Result<CallToolResult, McpError> {
        let policy = if let Some(ref path) = p.policy_file {
            onecrawl_cdp::SafetyState::load_from_file(std::path::Path::new(path))
                .map_err(|e| mcp_err(e))?
        } else {
            onecrawl_cdp::SafetyPolicy {
                allowed_domains: p.allowed_domains.unwrap_or_default(),
                blocked_domains: p.blocked_domains.unwrap_or_default(),
                blocked_url_patterns: p.blocked_url_patterns.unwrap_or_default(),
                max_actions: p.max_actions.unwrap_or(0),
                confirm_form_submit: p.confirm_form_submit.unwrap_or(false),
                confirm_file_upload: p.confirm_file_upload.unwrap_or(false),
                blocked_commands: p.blocked_commands.unwrap_or_default(),
                allowed_commands: p.allowed_commands.unwrap_or_default(),
                rate_limit_per_minute: p.rate_limit_per_minute.unwrap_or(0),
            }
        };

        let mut state = self.browser.lock().await;
        match state.safety.as_mut() {
            Some(existing) => existing.set_policy(policy.clone()),
            None => state.safety = Some(onecrawl_cdp::SafetyState::new(policy.clone())),
        }

        json_ok(&serde_json::json!({
            "status": "policy_set",
            "policy": policy
        }))
    }

    #[tool(
        name = "agent.safety_status",
        description = "Get current safety state: active policy, action counts, rate limit window, and all constraints."
    )]
    async fn agent_safety_status(
        &self,
        #[allow(unused_variables)]
        Parameters(_p): Parameters<SafetyStatusParams>,
    ) -> Result<CallToolResult, McpError> {
        let state = self.browser.lock().await;
        match &state.safety {
            Some(safety) => json_ok(&safety.stats()),
            None => json_ok(&serde_json::json!({
                "status": "no_policy",
                "info": "No safety policy is active. Use agent.safety_policy_set to configure one."
            })),
        }
    }

    #[tool(
        name = "agent.skills_list",
        description = "List all available skill packages (built-in and discovered). Returns name, version, description, and tool list for each skill."
    )]
    fn agent_skills_list(
        &self,
        #[allow(unused_variables)]
        Parameters(_p): Parameters<SkillsListParams>,
    ) -> Result<CallToolResult, McpError> {
        let builtins = onecrawl_cdp::skills::SkillRegistry::builtins();
        let skills: Vec<serde_json::Value> = builtins
            .iter()
            .map(|s| {
                serde_json::json!({
                    "name": s.name,
                    "version": s.version,
                    "description": s.description,
                    "tools": s.tools.iter().map(|t| serde_json::json!({
                        "name": t.name,
                        "description": t.description,
                    })).collect::<Vec<_>>(),
                    "requires": s.requires,
                    "author": s.author,
                    "source": "built-in",
                })
            })
            .collect();
        json_ok(&skills)
    }
    #[tool(
        name = "agent.screencast_start",
        description = "Start live browser screencast via CDP Page.startScreencast. Configure format, quality, resolution, and frame rate."
    )]
    async fn agent_screencast_start(
        &self,
        Parameters(p): Parameters<ScreencastStartParams>,
    ) -> Result<CallToolResult, McpError> {
        let page = ensure_page(&self.browser).await?;
        let opts = onecrawl_cdp::screencast::ScreencastOptions {
            format: p.format.unwrap_or_else(|| "jpeg".into()),
            quality: p.quality.or(Some(60)),
            max_width: p.max_width.or(Some(1280)),
            max_height: p.max_height.or(Some(720)),
            every_nth_frame: p.every_nth_frame.or(Some(1)),
        };
        onecrawl_cdp::screencast::start_screencast(&page, &opts)
            .await
            .map_err(|e| mcp_err(e.to_string()))?;
        json_ok(&serde_json::json!({
            "status": "started",
            "format": opts.format,
            "quality": opts.quality,
            "max_width": opts.max_width,
            "max_height": opts.max_height,
            "every_nth_frame": opts.every_nth_frame
        }))
    }

    #[tool(
        name = "agent.screencast_stop",
        description = "Stop the active browser screencast."
    )]
    async fn agent_screencast_stop(
        &self,
        #[allow(unused_variables)]
        Parameters(_p): Parameters<ScreencastStopParams>,
    ) -> Result<CallToolResult, McpError> {
        let page = ensure_page(&self.browser).await?;
        onecrawl_cdp::screencast::stop_screencast(&page)
            .await
            .map_err(|e| mcp_err(e.to_string()))?;
        json_ok(&serde_json::json!({ "status": "stopped" }))
    }

    #[tool(
        name = "agent.screencast_frame",
        description = "Capture a single screencast frame as base64-encoded image data."
    )]
    async fn agent_screencast_frame(
        &self,
        Parameters(p): Parameters<ScreencastFrameParams>,
    ) -> Result<CallToolResult, McpError> {
        let page = ensure_page(&self.browser).await?;
        let opts = onecrawl_cdp::screencast::ScreencastOptions {
            format: p.format.unwrap_or_else(|| "jpeg".into()),
            quality: p.quality.or(Some(80)),
            ..Default::default()
        };
        let bytes = onecrawl_cdp::screencast::capture_frame(&page, &opts)
            .await
            .map_err(|e| mcp_err(e.to_string()))?;
        let b64 = B64.encode(&bytes);
        json_ok(&serde_json::json!({
            "image": b64,
            "format": opts.format,
            "size": bytes.len()
        }))
    }

    #[tool(
        name = "agent.recording_start",
        description = "Start recording the browser session. Frames are captured and stored in memory until stopped."
    )]
    async fn agent_recording_start(
        &self,
        Parameters(p): Parameters<RecordingStartParams>,
    ) -> Result<CallToolResult, McpError> {
        let page = ensure_page(&self.browser).await?;
        let output = p.output.unwrap_or_else(|| "recording.webm".into());
        let fps = p.fps.unwrap_or(5);

        {
            let mut state = self.browser.lock().await;
            if state.recording.as_ref().is_some_and(|r| r.is_recording()) {
                return Err(mcp_err("recording already in progress"));
            }
            let mut rec = onecrawl_cdp::RecordingState::new(
                std::path::PathBuf::from(&output),
                fps,
            );
            rec.start();
            state.recording = Some(rec);
        }

        let opts = onecrawl_cdp::screencast::ScreencastOptions {
            format: "jpeg".into(),
            quality: Some(60),
            max_width: Some(1280),
            max_height: Some(720),
            every_nth_frame: Some(1),
        };
        onecrawl_cdp::screencast::start_screencast(&page, &opts)
            .await
            .map_err(|e| mcp_err(e.to_string()))?;

        json_ok(&serde_json::json!({
            "status": "recording",
            "output": output,
            "fps": fps
        }))
    }

    #[tool(
        name = "agent.recording_stop",
        description = "Stop recording, save frames as image sequence, and return the output path."
    )]
    async fn agent_recording_stop(
        &self,
        #[allow(unused_variables)]
        Parameters(_p): Parameters<RecordingStopParams>,
    ) -> Result<CallToolResult, McpError> {
        let page = ensure_page(&self.browser).await?;
        let _ = onecrawl_cdp::screencast::stop_screencast(&page).await;

        let mut state = self.browser.lock().await;
        let rec = state.recording.as_mut()
            .ok_or_else(|| mcp_err("no recording in progress"))?;

        // If no frames were captured via events, grab one snapshot
        if rec.is_recording() && rec.frame_count() == 0 {
            drop(state);
            let opts = onecrawl_cdp::screencast::ScreencastOptions::default();
            if let Ok(bytes) = onecrawl_cdp::screencast::capture_frame(&page, &opts).await {
                let mut state = self.browser.lock().await;
                if let Some(rec) = state.recording.as_mut() {
                    rec.add_frame(bytes);
                }
            }
            let mut state = self.browser.lock().await;
            let rec = state.recording.as_mut()
                .ok_or_else(|| mcp_err("no recording in progress"))?;
            rec.stop();
            let frame_count = rec.frame_count();
            let result = rec.save_frames().map_err(|e| mcp_err(e))?;
            state.recording = None;
            return json_ok(&serde_json::json!({
                "status": "saved",
                "frames": frame_count,
                "path": result.display().to_string()
            }));
        }

        rec.stop();
        let frame_count = rec.frame_count();
        let result = rec.save_frames().map_err(|e| mcp_err(e))?;
        state.recording = None;
        json_ok(&serde_json::json!({
            "status": "saved",
            "frames": frame_count,
            "path": result.display().to_string()
        }))
    }

    #[tool(
        name = "agent.recording_status",
        description = "Get the current recording state (idle, recording, or stopped) with frame count."
    )]
    async fn agent_recording_status(
        &self,
        #[allow(unused_variables)]
        Parameters(_p): Parameters<RecordingStatusParams>,
    ) -> Result<CallToolResult, McpError> {
        let state = self.browser.lock().await;
        match state.recording.as_ref() {
            Some(rec) => {
                let status = if rec.is_recording() { "recording" } else { "stopped" };
                json_ok(&serde_json::json!({
                    "status": status,
                    "frames": rec.frame_count(),
                    "fps": rec.fps(),
                    "output": rec.output_path().display().to_string()
                }))
            }
            None => json_ok(&serde_json::json!({
                "status": "idle",
                "frames": 0
            })),
        }
    }

    // ════════════════════════════════════════════════════════════════
    //  iOS / Mobile Safari tools
    // ════════════════════════════════════════════════════════════════

    #[tool(
        name = "agent.ios_devices",
        description = "List available iOS simulator devices (via xcrun simctl). Returns device name, UDID, platform, and version."
    )]
    async fn agent_ios_devices(
        &self,
        #[allow(unused_variables)]
        Parameters(_p): Parameters<IosDevicesParams>,
    ) -> Result<CallToolResult, McpError> {
        let devices = onecrawl_cdp::ios::IosClient::list_devices()
            .map_err(|e| mcp_err(format!("iOS list devices failed: {e}")))?;
        json_ok(&serde_json::json!({
            "devices": devices,
            "count": devices.len()
        }))
    }

    #[tool(
        name = "agent.ios_connect",
        description = "Start an iOS Safari session via WebDriverAgent. Returns session ID on success."
    )]
    async fn agent_ios_connect(
        &self,
        Parameters(p): Parameters<IosConnectParams>,
    ) -> Result<CallToolResult, McpError> {
        let config = onecrawl_cdp::ios::IosSessionConfig {
            wda_url: p.wda_url.unwrap_or_else(|| "http://localhost:8100".to_string()),
            device_udid: p.udid,
            bundle_id: p.bundle_id.unwrap_or_else(|| "com.apple.mobilesafari".to_string()),
        };
        let mut client = onecrawl_cdp::ios::IosClient::new(config);
        let session_id = client.create_session().await
            .map_err(|e| mcp_err(format!("iOS connect failed: {e}")))?;
        json_ok(&serde_json::json!({
            "connected": true,
            "session_id": session_id
        }))
    }

    #[tool(
        name = "agent.ios_navigate",
        description = "Navigate Mobile Safari to a URL. Requires an active iOS session (use agent.ios_connect first)."
    )]
    async fn agent_ios_navigate(
        &self,
        Parameters(p): Parameters<IosNavigateParams>,
    ) -> Result<CallToolResult, McpError> {
        let config = onecrawl_cdp::ios::IosSessionConfig::default();
        let client = onecrawl_cdp::ios::IosClient::new(config);
        client.navigate(&p.url).await
            .map_err(|e| mcp_err(format!("iOS navigate failed: {e}")))?;
        json_ok(&serde_json::json!({
            "navigated": true,
            "url": p.url
        }))
    }

    #[tool(
        name = "agent.ios_tap",
        description = "Tap at screen coordinates on the iOS device."
    )]
    async fn agent_ios_tap(
        &self,
        Parameters(p): Parameters<IosTapParams>,
    ) -> Result<CallToolResult, McpError> {
        let config = onecrawl_cdp::ios::IosSessionConfig::default();
        let client = onecrawl_cdp::ios::IosClient::new(config);
        client.tap(p.x, p.y).await
            .map_err(|e| mcp_err(format!("iOS tap failed: {e}")))?;
        json_ok(&serde_json::json!({
            "tapped": true,
            "x": p.x,
            "y": p.y
        }))
    }

    #[tool(
        name = "agent.ios_screenshot",
        description = "Take a screenshot of the iOS device screen. Returns base64-encoded image data."
    )]
    async fn agent_ios_screenshot(
        &self,
        #[allow(unused_variables)]
        Parameters(_p): Parameters<IosScreenshotParams>,
    ) -> Result<CallToolResult, McpError> {
        let config = onecrawl_cdp::ios::IosSessionConfig::default();
        let client = onecrawl_cdp::ios::IosClient::new(config);
        let bytes = client.screenshot().await
            .map_err(|e| mcp_err(format!("iOS screenshot failed: {e}")))?;
        let b64 = B64.encode(&bytes);
        json_ok(&serde_json::json!({
            "format": "png",
            "size": bytes.len(),
            "data": b64
        }))
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
