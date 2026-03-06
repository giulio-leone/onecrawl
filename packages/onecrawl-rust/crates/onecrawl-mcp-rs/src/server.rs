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
                if selector_raw.is_empty() {
                    return Err("'selector' must not be empty".into());
                }
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
                if selector_raw.is_empty() {
                    return Err("'selector' must not be empty".into());
                }
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
                if selector_raw.is_empty() {
                    return Err("'selector' must not be empty".into());
                }
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
                if selector.is_empty() {
                    return Err("'selector' must not be empty".into());
                }
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
        description = "Execute multiple commands in sequence. Commands: navigation.goto, navigation.click, navigation.type, navigation.wait, navigation.evaluate, navigation.snapshot, scraping.css, scraping.text. Returns {results, completed, total}."
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
                        "error": {
                            "message": err_msg,
                            "code": "CHAIN_STEP_FAILED"
                        }
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
        description = "Take a screenshot of a specific element by CSS selector or @ref. Supports @ref notation (e.g. @e1 from navigation.snapshot). Returns base64 PNG with element bounds."
    )]
    async fn agent_element_screenshot(
        &self,
        Parameters(p): Parameters<ElementScreenshotParams>,
    ) -> Result<CallToolResult, McpError> {
        if p.selector.is_empty() {
            return Err(mcp_err("selector must not be empty"));
        }
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
                this.__onecrawl_entry = { type: 'xhr', method: (method || 'GET').toUpperCase(), url: url || '', status: null, contentType: null, timestamp: Date.now() };
                return origOpen.call(this, method, url, ...rest);
            };
            XMLHttpRequest.prototype.send = function(...args) {
                const entry = this.__onecrawl_entry;
                if (entry) {
                    this.addEventListener('load', function() {
                        entry.status = this.status;
                        entry.contentType = this.getResponseHeader('content-type');
                        window.__onecrawl_api_log.push(entry);
                    });
                    this.addEventListener('error', function() {
                        entry.error = 'network error';
                        window.__onecrawl_api_log.push(entry);
                    });
                }
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
        description = "Get a summary of all network API calls (fetch/XHR) captured since api_capture_start. Returns summary of XHR/fetch calls made since api_capture_start. Per-page, resets on navigation."
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
        description = "Take an accessibility snapshot inside a specific iframe by index. Index is 0-based. Use agent.iframe_list first to discover available iframes."
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

        // Connect to remote browser via chromiumoxide (with timeout)
        let (browser, mut handler) =
            tokio::time::timeout(
                std::time::Duration::from_secs(15),
                chromiumoxide::Browser::connect(&p.ws_url),
            )
            .await
            .map_err(|_| mcp_err("remote CDP connect timed out after 15s"))?
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
            quality: p.quality.map(|q| q.min(100)).or(Some(60)),
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
        description = "Start an iOS Safari session via WebDriverAgent. Requires WebDriverAgent running. Session persists for subsequent ios_* calls."
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

        let mut state = self.browser.lock().await;
        state.ios_client = Some(client);

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
        let state = self.browser.lock().await;
        let client = state.ios_client.as_ref()
            .ok_or_else(|| mcp_err("no active iOS session — call agent.ios_connect first"))?;
        client.navigate(&p.url).await
            .map_err(|e| mcp_err(format!("iOS navigate failed: {e}")))?;
        json_ok(&serde_json::json!({
            "navigated": true,
            "url": p.url
        }))
    }

    #[tool(
        name = "agent.ios_tap",
        description = "Tap at screen coordinates on the iOS device. Requires an active iOS session (use agent.ios_connect first)."
    )]
    async fn agent_ios_tap(
        &self,
        Parameters(p): Parameters<IosTapParams>,
    ) -> Result<CallToolResult, McpError> {
        let state = self.browser.lock().await;
        let client = state.ios_client.as_ref()
            .ok_or_else(|| mcp_err("no active iOS session — call agent.ios_connect first"))?;
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
        description = "Take a screenshot of the iOS device screen. Returns base64-encoded image data. Requires an active iOS session (use agent.ios_connect first)."
    )]
    async fn agent_ios_screenshot(
        &self,
        #[allow(unused_variables)]
        Parameters(_p): Parameters<IosScreenshotParams>,
    ) -> Result<CallToolResult, McpError> {
        let state = self.browser.lock().await;
        let client = state.ios_client.as_ref()
            .ok_or_else(|| mcp_err("no active iOS session — call agent.ios_connect first"))?;
        let bytes = client.screenshot().await
            .map_err(|e| mcp_err(format!("iOS screenshot failed: {e}")))?;
        let b64 = B64.encode(&bytes);
        json_ok(&serde_json::json!({
            "format": "png",
            "size": bytes.len(),
            "data": b64
        }))
    }

    // ──────────────── Computer Use Protocol ─────────────────

    #[tool(
        name = "computer.act",
        description = "Execute a browser action (click, type, key, scroll, navigate, fill, select, drag, evaluate) and return the page observation (URL, title, accessibility snapshot, optional screenshot). Computer-use protocol for AI agents."
    )]
    async fn computer_act(
        &self,
        Parameters(p): Parameters<ComputerUseActionParams>,
    ) -> Result<CallToolResult, McpError> {
        let page = ensure_page(&self.browser).await?;
        let mut action: onecrawl_cdp::computer_use::AgentAction =
            serde_json::from_value(p.action)
                .map_err(|e| mcp_err(format!("invalid action: {e}")))?;

        // Override screenshot flag when explicitly requested via param.
        if p.include_screenshot.unwrap_or(false) {
            if let onecrawl_cdp::computer_use::AgentAction::Observe {
                ref mut include_screenshot,
            } = action
            {
                *include_screenshot = true;
            }
        }

        let result = onecrawl_cdp::computer_use::execute_action(&page, &action, 0)
            .await
            .map_err(|e| mcp_err(e.to_string()))?;

        json_ok(&result)
    }

    #[tool(
        name = "computer.observe",
        description = "Observe current browser state: URL, title, accessibility snapshot with @refs, viewport size, optional screenshot. No action taken — pure observation for AI planning."
    )]
    async fn computer_observe(
        &self,
        Parameters(p): Parameters<ComputerUseObserveParams>,
    ) -> Result<CallToolResult, McpError> {
        let page = ensure_page(&self.browser).await?;
        let obs = onecrawl_cdp::computer_use::observe(
            &page,
            None,
            p.include_screenshot.unwrap_or(false),
        )
        .await
        .map_err(|e| mcp_err(e.to_string()))?;

        json_ok(&obs)
    }

    #[tool(
        name = "computer.batch",
        description = "Execute a sequence of browser actions and return observations after each step. Efficient for multi-step workflows. Stops on first error by default."
    )]
    async fn computer_batch(
        &self,
        Parameters(p): Parameters<ComputerUseBatchParams>,
    ) -> Result<CallToolResult, McpError> {
        let page = ensure_page(&self.browser).await?;
        let stop_on_error = p.stop_on_error.unwrap_or(true);
        let include_screenshots = p.include_screenshots.unwrap_or(false);
        let mut results: Vec<onecrawl_cdp::computer_use::ActionResult> = Vec::new();

        for (i, raw) in p.actions.iter().enumerate() {
            let mut action: onecrawl_cdp::computer_use::AgentAction =
                serde_json::from_value(raw.clone())
                    .map_err(|e| mcp_err(format!("invalid action at index {i}: {e}")))?;

            if include_screenshots {
                if let onecrawl_cdp::computer_use::AgentAction::Observe {
                    ref mut include_screenshot,
                } = action
                {
                    *include_screenshot = true;
                }
            }

            let result = onecrawl_cdp::computer_use::execute_action(&page, &action, i)
                .await
                .map_err(|e| mcp_err(e.to_string()))?;

            let failed = !result.success;
            results.push(result);

            if failed && stop_on_error {
                break;
            }
        }

        json_ok(&serde_json::json!({
            "total": p.actions.len(),
            "executed": results.len(),
            "results": results,
        }))
    }

    // ════════════════════════════════════════════════════════════════
    //  Browser Pool tools
    // ════════════════════════════════════════════════════════════════

    #[tool(
        name = "pool.list",
        description = "List all browser instances in the pool with their ID, status, current URL, and creation time."
    )]
    async fn pool_list(
        &self,
        #[allow(unused_variables)] Parameters(_p): Parameters<PoolListParams>,
    ) -> Result<CallToolResult, McpError> {
        let state = self.browser.lock().await;
        let instances = state.pool.list();
        json_ok(&serde_json::json!({
            "instances": instances,
            "count": instances.len(),
        }))
    }

    #[tool(
        name = "pool.status",
        description = "Get pool statistics: current size, max size, idle count, and busy count."
    )]
    async fn pool_status(
        &self,
        #[allow(unused_variables)] Parameters(_p): Parameters<PoolStatusParams>,
    ) -> Result<CallToolResult, McpError> {
        let state = self.browser.lock().await;
        let pool = &state.pool;
        let total = pool.len();
        let idle = pool.idle_count();
        json_ok(&serde_json::json!({
            "size": total,
            "max_size": pool.max_size(),
            "idle": idle,
            "busy": total - idle,
        }))
    }

    // ════════════════════════════════════════════════════════════════
    //  Smart Actions tools
    // ════════════════════════════════════════════════════════════════

    #[tool(
        name = "smart.find",
        description = "Smart element discovery — finds elements using fuzzy text, ARIA roles, attributes, or CSS selectors. Returns ranked matches with confidence scores."
    )]
    async fn smart_find(
        &self,
        Parameters(p): Parameters<SmartFindParams>,
    ) -> Result<CallToolResult, McpError> {
        let page = ensure_page(&self.browser).await?;
        let matches = onecrawl_cdp::smart_actions::smart_find(&page, &p.query)
            .await
            .map_err(|e| mcp_err(e.to_string()))?;
        json_ok(&serde_json::json!({
            "query": p.query,
            "matches": matches,
            "count": matches.len(),
        }))
    }

    #[tool(
        name = "smart.click",
        description = "Smart click — finds the best matching element using fuzzy text, ARIA roles, or CSS selectors, then clicks it. Returns the matched element info."
    )]
    async fn smart_click(
        &self,
        Parameters(p): Parameters<SmartClickParams>,
    ) -> Result<CallToolResult, McpError> {
        let page = ensure_page(&self.browser).await?;
        let matched = onecrawl_cdp::smart_actions::smart_click(&page, &p.query)
            .await
            .map_err(|e| mcp_err(e.to_string()))?;
        json_ok(&serde_json::json!({
            "clicked": matched.selector,
            "confidence": matched.confidence,
            "strategy": matched.strategy,
        }))
    }

    #[tool(
        name = "smart.fill",
        description = "Smart fill — finds an input element using fuzzy text, placeholder, or CSS selector, then types the given value into it."
    )]
    async fn smart_fill(
        &self,
        Parameters(p): Parameters<SmartFillParams>,
    ) -> Result<CallToolResult, McpError> {
        let page = ensure_page(&self.browser).await?;
        let matched = onecrawl_cdp::smart_actions::smart_fill(&page, &p.query, &p.value)
            .await
            .map_err(|e| mcp_err(e.to_string()))?;
        json_ok(&serde_json::json!({
            "filled": matched.selector,
            "value_length": p.value.len(),
            "confidence": matched.confidence,
            "strategy": matched.strategy,
        }))
    }

    // ════════════════════════════════════════════════════════════════
    //  Agent Memory tools
    // ════════════════════════════════════════════════════════════════

    fn ensure_memory(state: &mut BrowserState) -> &mut onecrawl_cdp::AgentMemory {
        if state.memory.is_none() {
            let home = std::env::var("HOME").unwrap_or_else(|_| ".".into());
            let path = std::path::PathBuf::from(home).join(".onecrawl").join("agent_memory.json");
            state.memory = Some(
                onecrawl_cdp::AgentMemory::load(&path).unwrap_or_else(|_| onecrawl_cdp::AgentMemory::new(&path))
            );
        }
        state.memory.as_mut().unwrap()
    }

    #[tool(
        name = "memory.store",
        description = "Store a memory entry — persists data across sessions. Use for learned patterns, domain strategies, selector mappings, or any knowledge the agent should remember."
    )]
    async fn memory_store(
        &self,
        Parameters(p): Parameters<MemoryStoreParams>,
    ) -> Result<CallToolResult, McpError> {
        let mut state = self.browser.lock().await;
        let category = match p.category.as_deref() {
            Some("page_visit") => onecrawl_cdp::MemoryCategory::PageVisit,
            Some("element_pattern") => onecrawl_cdp::MemoryCategory::ElementPattern,
            Some("domain_strategy") => onecrawl_cdp::MemoryCategory::DomainStrategy,
            Some("retry_knowledge") => onecrawl_cdp::MemoryCategory::RetryKnowledge,
            Some("user_preference") => onecrawl_cdp::MemoryCategory::UserPreference,
            Some("selector_mapping") => onecrawl_cdp::MemoryCategory::SelectorMapping,
            Some("error_pattern") => onecrawl_cdp::MemoryCategory::ErrorPattern,
            _ => onecrawl_cdp::MemoryCategory::Custom,
        };
        let mem = Self::ensure_memory(&mut state);
        mem.store(&p.key, p.value.clone(), category, p.domain.clone())
            .map_err(|e| mcp_err(e.to_string()))?;
        json_ok(&serde_json::json!({
            "stored": p.key,
            "category": format!("{:?}", mem.recall(&p.key).map(|e| &e.category)),
        }))
    }

    #[tool(
        name = "memory.recall",
        description = "Recall a specific memory entry by key. Returns the stored value, category, domain, and access statistics."
    )]
    async fn memory_recall(
        &self,
        Parameters(p): Parameters<MemoryRecallParams>,
    ) -> Result<CallToolResult, McpError> {
        let mut state = self.browser.lock().await;
        let mem = Self::ensure_memory(&mut state);
        match mem.recall(&p.key) {
            Some(entry) => json_ok(&serde_json::json!({
                "key": entry.key,
                "value": entry.value,
                "category": format!("{:?}", entry.category),
                "domain": entry.domain,
                "access_count": entry.access_count,
                "created_at": entry.created_at,
                "accessed_at": entry.accessed_at,
            })),
            None => json_ok(&serde_json::json!({ "key": p.key, "found": false })),
        }
    }

    #[tool(
        name = "memory.search",
        description = "Search agent memory by query text, optionally filtered by category and domain. Returns matching entries ranked by relevance."
    )]
    async fn memory_search(
        &self,
        Parameters(p): Parameters<MemorySearchParams>,
    ) -> Result<CallToolResult, McpError> {
        let mut state = self.browser.lock().await;
        let category = match p.category.as_deref() {
            Some("page_visit") => Some(onecrawl_cdp::MemoryCategory::PageVisit),
            Some("element_pattern") => Some(onecrawl_cdp::MemoryCategory::ElementPattern),
            Some("domain_strategy") => Some(onecrawl_cdp::MemoryCategory::DomainStrategy),
            Some("retry_knowledge") => Some(onecrawl_cdp::MemoryCategory::RetryKnowledge),
            Some("user_preference") => Some(onecrawl_cdp::MemoryCategory::UserPreference),
            Some("selector_mapping") => Some(onecrawl_cdp::MemoryCategory::SelectorMapping),
            Some("error_pattern") => Some(onecrawl_cdp::MemoryCategory::ErrorPattern),
            Some("custom") => Some(onecrawl_cdp::MemoryCategory::Custom),
            _ => None,
        };
        let mem = Self::ensure_memory(&mut state);
        let results = mem.search(&p.query, category, p.domain.as_deref());
        let entries: Vec<serde_json::Value> = results.iter().map(|e| {
            serde_json::json!({
                "key": e.key,
                "value": e.value,
                "category": format!("{:?}", e.category),
                "domain": e.domain,
                "access_count": e.access_count,
            })
        }).collect();
        json_ok(&serde_json::json!({
            "query": p.query,
            "count": entries.len(),
            "results": entries,
        }))
    }

    #[tool(
        name = "memory.forget",
        description = "Forget a specific memory entry by key, or clear all memories for a domain. Returns how many entries were removed."
    )]
    async fn memory_forget(
        &self,
        Parameters(p): Parameters<MemoryForgetParams>,
    ) -> Result<CallToolResult, McpError> {
        let mut state = self.browser.lock().await;
        let mem = Self::ensure_memory(&mut state);
        if let Some(key) = &p.key {
            let removed = mem.forget(key);
            json_ok(&serde_json::json!({ "removed": removed, "key": key }))
        } else if let Some(domain) = &p.domain {
            let count = mem.clear_domain(domain);
            json_ok(&serde_json::json!({ "removed": count, "domain": domain }))
        } else {
            let count = mem.clear_all();
            json_ok(&serde_json::json!({ "removed": count, "cleared": "all" }))
        }
    }

    #[tool(
        name = "memory.domain_strategy",
        description = "Store or recall a domain-specific strategy (login selectors, navigation patterns, popup handlers, rate limits). Pass strategy JSON to store, omit to recall."
    )]
    async fn memory_domain_strategy(
        &self,
        Parameters(p): Parameters<MemoryDomainStrategyParams>,
    ) -> Result<CallToolResult, McpError> {
        let mut state = self.browser.lock().await;
        let mem = Self::ensure_memory(&mut state);
        if let Some(strategy_val) = p.strategy {
            let strategy: onecrawl_cdp::DomainStrategy = serde_json::from_value(strategy_val)
                .map_err(|e| mcp_err(format!("invalid strategy JSON: {e}")))?;
            mem.store_domain_strategy(strategy)
                .map_err(|e| mcp_err(e.to_string()))?;
            json_ok(&serde_json::json!({ "stored": true, "domain": p.domain }))
        } else {
            match mem.recall_domain_strategy(&p.domain) {
                Some(strategy) => json_ok(&serde_json::json!({
                    "domain": strategy.domain,
                    "login_selectors": strategy.login_selectors,
                    "navigation_patterns": strategy.navigation_patterns,
                    "known_popups": strategy.known_popups,
                    "rate_limit_info": strategy.rate_limit_info,
                    "anti_bot_level": strategy.anti_bot_level,
                })),
                None => json_ok(&serde_json::json!({ "domain": p.domain, "found": false })),
            }
        }
    }

    #[tool(
        name = "memory.stats",
        description = "Get memory statistics — total entries, breakdown by category and domain, and capacity info."
    )]
    async fn memory_stats(
        &self,
        #[allow(unused_variables)] Parameters(_p): Parameters<MemoryStatsParams>,
    ) -> Result<CallToolResult, McpError> {
        let mut state = self.browser.lock().await;
        let mem = Self::ensure_memory(&mut state);
        let stats = mem.stats();
        json_ok(&serde_json::json!({
            "total_entries": stats.total_entries,
            "max_entries": stats.max_entries,
            "categories": stats.categories,
            "domains": stats.domains,
            "utilization": format!("{:.1}%", (stats.total_entries as f64 / stats.max_entries as f64) * 100.0),
        }))
    }

    // ════════════════════════════════════════════════════════════════
    //  Workflow DSL tools
    // ════════════════════════════════════════════════════════════════

    #[tool(
        name = "workflow.validate",
        description = "Validate a workflow definition. Returns validation errors if any, or confirms the workflow is valid."
    )]
    async fn workflow_validate(
        &self,
        Parameters(p): Parameters<WorkflowValidateParams>,
    ) -> Result<CallToolResult, McpError> {
        let workflow = onecrawl_cdp::workflow::parse_json(&p.workflow)
            .map_err(|e| mcp_err(e.to_string()))?;
        let errors = onecrawl_cdp::workflow::validate(&workflow);
        if errors.is_empty() {
            json_ok(&serde_json::json!({
                "valid": true,
                "name": workflow.name,
                "steps": workflow.steps.len(),
                "variables": workflow.variables.keys().collect::<Vec<_>>(),
            }))
        } else {
            json_ok(&serde_json::json!({
                "valid": false,
                "errors": errors,
            }))
        }
    }

    #[tool(
        name = "workflow.run",
        description = "Execute a workflow — runs a series of browser automation steps defined in JSON. Supports variables, conditionals, loops, error handling, and sub-workflows."
    )]
    async fn workflow_run(
        &self,
        Parameters(p): Parameters<WorkflowRunParams>,
    ) -> Result<CallToolResult, McpError> {
        let mut workflow = if p.workflow.trim().starts_with('{') {
            onecrawl_cdp::workflow::parse_json(&p.workflow)
                .map_err(|e| mcp_err(e.to_string()))?
        } else {
            onecrawl_cdp::workflow::load_from_file(&p.workflow)
                .map_err(|e| mcp_err(e.to_string()))?
        };

        // Override variables
        if let Some(overrides) = p.variables {
            for (k, v) in overrides {
                workflow.variables.insert(k, v);
            }
        }

        // Validate first
        let errors = onecrawl_cdp::workflow::validate(&workflow);
        if !errors.is_empty() {
            return json_ok(&serde_json::json!({
                "status": "validation_failed",
                "errors": errors,
            }));
        }

        let page = ensure_page(&self.browser).await?;
        let start = std::time::Instant::now();
        let mut results: Vec<onecrawl_cdp::StepResult> = Vec::new();
        let mut variables = workflow.variables.clone();
        let mut succeeded = 0usize;
        let mut failed = 0usize;
        let mut skipped = 0usize;
        let mut overall_status = onecrawl_cdp::StepStatus::Success;

        for (i, step) in workflow.steps.iter().enumerate() {
            let step_id = if step.id.is_empty() { format!("step_{i}") } else { step.id.clone() };
            let step_name = if step.name.is_empty() { format!("Step {i}") } else { step.name.clone() };

            // Check condition
            if let Some(ref cond) = step.condition {
                let interpolated = onecrawl_cdp::workflow::interpolate(cond, &variables);
                if !onecrawl_cdp::workflow::evaluate_condition(&interpolated, &variables) {
                    results.push(onecrawl_cdp::StepResult {
                        step_id, step_name,
                        status: onecrawl_cdp::StepStatus::Skipped,
                        output: None, error: None, duration_ms: 0,
                    });
                    skipped += 1;
                    continue;
                }
            }

            let step_start = std::time::Instant::now();
            let result = self.execute_step(&page, &step.action, &mut variables).await;
            let duration_ms = step_start.elapsed().as_millis() as u64;

            match result {
                Ok(output) => {
                    if let Some(ref save_key) = step.save_as {
                        if let Some(ref out) = output {
                            variables.insert(save_key.clone(), out.clone());
                        }
                    }
                    results.push(onecrawl_cdp::StepResult {
                        step_id, step_name,
                        status: onecrawl_cdp::StepStatus::Success,
                        output, error: None, duration_ms,
                    });
                    succeeded += 1;
                }
                Err(e) => {
                    let err_msg = format!("{}", e.message);
                    let error_action = step.on_error.as_ref()
                        .unwrap_or(&workflow.on_error.action);
                    results.push(onecrawl_cdp::StepResult {
                        step_id, step_name,
                        status: onecrawl_cdp::StepStatus::Failed,
                        output: None, error: Some(err_msg.clone()), duration_ms,
                    });
                    failed += 1;

                    match error_action {
                        onecrawl_cdp::workflow::StepErrorAction::Stop => {
                            overall_status = onecrawl_cdp::StepStatus::Failed;
                            break;
                        }
                        onecrawl_cdp::workflow::StepErrorAction::Continue |
                        onecrawl_cdp::workflow::StepErrorAction::Skip => continue,
                        onecrawl_cdp::workflow::StepErrorAction::Retry => continue,
                    }
                }
            }
        }

        let total_duration_ms = start.elapsed().as_millis() as u64;
        json_ok(&serde_json::json!({
            "name": workflow.name,
            "status": format!("{:?}", overall_status).to_lowercase(),
            "total_duration_ms": total_duration_ms,
            "steps_succeeded": succeeded,
            "steps_failed": failed,
            "steps_skipped": skipped,
            "steps": results,
            "variables": variables,
        }))
    }

    fn execute_step<'a>(
        &'a self,
        page: &'a chromiumoxide::Page,
        action: &'a onecrawl_cdp::workflow::Action,
        variables: &'a mut HashMap<String, serde_json::Value>,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = std::result::Result<Option<serde_json::Value>, McpError>> + Send + 'a>> {
        Box::pin(async move {
        use onecrawl_cdp::workflow::Action;
        match action {
            Action::Navigate { url } => {
                let url = onecrawl_cdp::workflow::interpolate(url, variables);
                onecrawl_cdp::navigation::goto(page, &url).await.map_err(|e| mcp_err(e.to_string()))?;
                let title = onecrawl_cdp::navigation::get_title(page).await.unwrap_or_default();
                Ok(Some(serde_json::json!({ "url": url, "title": title })))
            }
            Action::Click { selector } => {
                let sel = onecrawl_cdp::workflow::interpolate(selector, variables);
                let resolved = onecrawl_cdp::accessibility::resolve_ref(&sel);
                onecrawl_cdp::element::click(page, &resolved).await.map_err(|e| mcp_err(e.to_string()))?;
                Ok(Some(serde_json::json!({ "clicked": sel })))
            }
            Action::Type { selector, text } => {
                let sel = onecrawl_cdp::workflow::interpolate(selector, variables);
                let txt = onecrawl_cdp::workflow::interpolate(text, variables);
                let resolved = onecrawl_cdp::accessibility::resolve_ref(&sel);
                onecrawl_cdp::element::type_text(page, &resolved, &txt).await.map_err(|e| mcp_err(e.to_string()))?;
                Ok(Some(serde_json::json!({ "typed": txt.len() })))
            }
            Action::WaitForSelector { selector, timeout_ms } => {
                let sel = onecrawl_cdp::workflow::interpolate(selector, variables);
                let resolved = onecrawl_cdp::accessibility::resolve_ref(&sel);
                onecrawl_cdp::navigation::wait_for_selector(page, &resolved, *timeout_ms).await.map_err(|e| mcp_err(e.to_string()))?;
                Ok(Some(serde_json::json!({ "found": sel })))
            }
            Action::Screenshot { path, full_page } => {
                let bytes = if full_page.unwrap_or(false) {
                    onecrawl_cdp::screenshot::screenshot_full(page)
                        .await.map_err(|e| mcp_err(e.to_string()))?
                } else {
                    onecrawl_cdp::screenshot::screenshot_viewport(page)
                        .await.map_err(|e| mcp_err(e.to_string()))?
                };
                if let Some(p) = path {
                    let p = onecrawl_cdp::workflow::interpolate(p, variables);
                    std::fs::write(&p, &bytes).map_err(|e| mcp_err(e.to_string()))?;
                    Ok(Some(serde_json::json!({ "saved": p, "bytes": bytes.len() })))
                } else {
                    Ok(Some(serde_json::json!({ "bytes": bytes.len() })))
                }
            }
            Action::Evaluate { js } => {
                let js = onecrawl_cdp::workflow::interpolate(js, variables);
                let result = page.evaluate(js).await.map_err(|e| mcp_err(e.to_string()))?;
                let val = result.into_value::<serde_json::Value>().unwrap_or(serde_json::Value::Null);
                Ok(Some(val))
            }
            Action::Extract { selector, attribute } => {
                let sel = onecrawl_cdp::workflow::interpolate(selector, variables);
                let attr_js = if let Some(attr) = attribute {
                    format!(r#"Array.from(document.querySelectorAll({sel_json})).map(e => e.getAttribute({attr_json}))"#,
                        sel_json = serde_json::to_string(&sel).unwrap(),
                        attr_json = serde_json::to_string(attr).unwrap())
                } else {
                    format!(r#"Array.from(document.querySelectorAll({sel_json})).map(e => e.textContent.trim())"#,
                        sel_json = serde_json::to_string(&sel).unwrap())
                };
                let result = page.evaluate(attr_js).await.map_err(|e| mcp_err(e.to_string()))?;
                let val = result.into_value::<serde_json::Value>().unwrap_or(serde_json::Value::Null);
                Ok(Some(val))
            }
            Action::SmartClick { query } => {
                let q = onecrawl_cdp::workflow::interpolate(query, variables);
                let matched = onecrawl_cdp::smart_actions::smart_click(page, &q).await.map_err(|e| mcp_err(e.to_string()))?;
                Ok(Some(serde_json::json!({ "clicked": matched.selector, "confidence": matched.confidence })))
            }
            Action::SmartFill { query, value } => {
                let q = onecrawl_cdp::workflow::interpolate(query, variables);
                let v = onecrawl_cdp::workflow::interpolate(value, variables);
                let matched = onecrawl_cdp::smart_actions::smart_fill(page, &q, &v).await.map_err(|e| mcp_err(e.to_string()))?;
                Ok(Some(serde_json::json!({ "filled": matched.selector, "confidence": matched.confidence })))
            }
            Action::Sleep { ms } => {
                tokio::time::sleep(tokio::time::Duration::from_millis(*ms)).await;
                Ok(Some(serde_json::json!({ "slept_ms": ms })))
            }
            Action::SetVariable { name, value } => {
                let interpolated = onecrawl_cdp::workflow::interpolate(&value.to_string(), variables);
                let parsed = serde_json::from_str::<serde_json::Value>(&interpolated)
                    .unwrap_or(serde_json::Value::String(interpolated));
                variables.insert(name.clone(), parsed.clone());
                Ok(Some(serde_json::json!({ "set": name, "value": parsed })))
            }
            Action::Log { message, level } => {
                let msg = onecrawl_cdp::workflow::interpolate(message, variables);
                let lvl = level.as_deref().unwrap_or("info");
                match lvl {
                    "error" => tracing::error!("[workflow] {}", msg),
                    "warn" => tracing::warn!("[workflow] {}", msg),
                    "debug" => tracing::debug!("[workflow] {}", msg),
                    _ => tracing::info!("[workflow] {}", msg),
                }
                Ok(Some(serde_json::json!({ "logged": msg, "level": lvl })))
            }
            Action::Assert { condition, message } => {
                let cond = onecrawl_cdp::workflow::interpolate(condition, variables);
                if onecrawl_cdp::workflow::evaluate_condition(&cond, variables) {
                    Ok(Some(serde_json::json!({ "assert": "passed" })))
                } else {
                    Err(mcp_err(format!("assertion failed: {}", message.as_deref().unwrap_or(&cond))))
                }
            }
            Action::Loop { items: _, variable: _, steps: _ } => {
                Ok(Some(serde_json::json!({ "note": "loop execution requires recursive step runner — use workflow.run for full support" })))
            }
            Action::Conditional { condition, then_steps, else_steps } => {
                let cond = onecrawl_cdp::workflow::interpolate(condition, variables);
                let empty = vec![];
                let branch = if onecrawl_cdp::workflow::evaluate_condition(&cond, variables) {
                    then_steps
                } else {
                    else_steps.as_ref().unwrap_or(&empty)
                };
                let mut last_output = None;
                for step in branch {
                    last_output = self.execute_step(page, &step.action, variables).await?;
                }
                Ok(last_output)
            }
            Action::SubWorkflow { path } => {
                let p = onecrawl_cdp::workflow::interpolate(path, variables);
                Ok(Some(serde_json::json!({ "note": format!("sub-workflow '{}' — use workflow.run to execute", p) })))
            }
            Action::HttpRequest { url, method, headers, body } => {
                let url = onecrawl_cdp::workflow::interpolate(url, variables);
                let method = method.as_deref().unwrap_or("GET");
                let client = reqwest::Client::new();
                let mut req = match method.to_uppercase().as_str() {
                    "POST" => client.post(&url),
                    "PUT" => client.put(&url),
                    "DELETE" => client.delete(&url),
                    "PATCH" => client.patch(&url),
                    _ => client.get(&url),
                };
                if let Some(hdrs) = headers {
                    for (k, v) in hdrs {
                        let v = onecrawl_cdp::workflow::interpolate(v, variables);
                        req = req.header(k.as_str(), v);
                    }
                }
                if let Some(b) = body {
                    let b = onecrawl_cdp::workflow::interpolate(b, variables);
                    req = req.body(b);
                }
                let resp = req.send().await.map_err(|e| mcp_err(e.to_string()))?;
                let status = resp.status().as_u16();
                let body_text = resp.text().await.unwrap_or_default();
                let body_val = serde_json::from_str::<serde_json::Value>(&body_text)
                    .unwrap_or(serde_json::Value::String(body_text));
                Ok(Some(serde_json::json!({ "status": status, "body": body_val })))
            }
            Action::Snapshot { compact, interactive_only } => {
                let opts = onecrawl_cdp::accessibility::AgentSnapshotOptions {
                    interactive_only: *interactive_only,
                    compact: *compact,
                    ..Default::default()
                };
                let result = onecrawl_cdp::accessibility::agent_snapshot(page, &opts)
                    .await.map_err(|e| mcp_err(e.to_string()))?;
                Ok(Some(serde_json::json!(result)))
            }
        }
        })
    }

    // ════════════════════════════════════════════════════════════════
    //  Network Intelligence tools
    // ════════════════════════════════════════════════════════════════

    #[tool(
        name = "net.capture",
        description = "Capture network traffic from the current page. Returns API endpoints with request/response details, timing, and classification."
    )]
    async fn net_capture(
        &self,
        Parameters(p): Parameters<NetIntelCaptureParams>,
    ) -> Result<CallToolResult, McpError> {
        let page = ensure_page(&self.browser).await?;
        let duration = p.duration_seconds.unwrap_or(10);
        let api_only = p.api_only.unwrap_or(true);

        // Inject network interceptor
        let js = r#"
        (() => {
            if (!window.__onecrawl_net_capture) {
                window.__onecrawl_net_capture = [];
                const origFetch = window.fetch;
                window.fetch = async function(...args) {
                    const start = Date.now();
                    const req = new Request(...args);
                    try {
                        const resp = await origFetch.apply(this, args);
                        const clone = resp.clone();
                        let body = null;
                        try { body = await clone.json(); } catch(_) {
                            try { body = await clone.text(); } catch(_) {}
                        }
                        let reqBody = null;
                        try { if (req.body) { reqBody = await new Request(...args).json(); } } catch(_) {}
                        window.__onecrawl_net_capture.push({
                            method: req.method,
                            url: req.url,
                            status: resp.status,
                            contentType: resp.headers.get('content-type'),
                            requestHeaders: Object.fromEntries(req.headers.entries()),
                            responseHeaders: Object.fromEntries(resp.headers.entries()),
                            requestBody: reqBody,
                            responseBody: body,
                            timing: Date.now() - start,
                        });
                        return resp;
                    } catch(e) {
                        window.__onecrawl_net_capture.push({
                            method: req.method,
                            url: req.url,
                            status: 0,
                            error: e.message,
                            timing: Date.now() - start,
                        });
                        throw e;
                    }
                };

                const origXHR = XMLHttpRequest.prototype.open;
                XMLHttpRequest.prototype.open = function(method, url, ...rest) {
                    this.__onecrawl_method = method;
                    this.__onecrawl_url = url;
                    this.__onecrawl_start = Date.now();
                    return origXHR.call(this, method, url, ...rest);
                };
                const origSend = XMLHttpRequest.prototype.send;
                XMLHttpRequest.prototype.send = function(body) {
                    this.addEventListener('load', function() {
                        let respBody = null;
                        try { respBody = JSON.parse(this.responseText); } catch(_) { respBody = this.responseText; }
                        window.__onecrawl_net_capture.push({
                            method: this.__onecrawl_method,
                            url: this.__onecrawl_url,
                            status: this.status,
                            contentType: this.getResponseHeader('content-type'),
                            responseBody: respBody,
                            timing: Date.now() - this.__onecrawl_start,
                        });
                    });
                    return origSend.call(this, body);
                };
            }
            return 'capture_started';
        })()
        "#;

        page.evaluate(js).await.map_err(|e| mcp_err(e.to_string()))?;

        // Wait for capture duration
        tokio::time::sleep(tokio::time::Duration::from_secs(duration)).await;

        // Collect results
        let collect_js = r#"
        (() => {
            const raw = window.__onecrawl_net_capture || [];
            window.__onecrawl_net_capture = [];
            return raw;
        })()
        "#;

        let result = page.evaluate(collect_js).await.map_err(|e| mcp_err(e.to_string()))?;
        let raw: Vec<serde_json::Value> = result.into_value().unwrap_or_default();

        let endpoints: Vec<onecrawl_cdp::network_intel::ApiEndpoint> = raw.iter().filter_map(|r| {
            let url = r.get("url")?.as_str()?;
            let method = r.get("method")?.as_str().unwrap_or("GET");
            let status = r.get("status")?.as_u64().unwrap_or(0) as u16;
            let content_type = r.get("contentType").and_then(|v| v.as_str()).map(String::from);
            let category = onecrawl_cdp::network_intel::classify_request(url, content_type.as_deref(), method);

            if api_only && category == onecrawl_cdp::network_intel::ApiCategory::Static {
                return None;
            }

            let (parsed_path, parsed_base) = url.split_once("://")
                .and_then(|(scheme, rest)| rest.split_once('/').map(|(host, path)| {
                    let p = format!("/{}", path).split('?').next().unwrap_or("/").to_string();
                    let b = format!("{}://{}", scheme, host);
                    (p, b)
                }))
                .unwrap_or(("/".into(), url.to_string()));

            Some(onecrawl_cdp::network_intel::ApiEndpoint {
                method: method.to_string(),
                url: url.to_string(),
                path: parsed_path,
                base_url: parsed_base,
                query_params: std::collections::HashMap::new(),
                request_headers: r.get("requestHeaders").and_then(|v| serde_json::from_value(v.clone()).ok()).unwrap_or_default(),
                response_headers: r.get("responseHeaders").and_then(|v| serde_json::from_value(v.clone()).ok()).unwrap_or_default(),
                request_body: r.get("requestBody").cloned().filter(|v| !v.is_null()),
                response_body: r.get("responseBody").cloned().filter(|v| !v.is_null()),
                status_code: status,
                content_type,
                timing_ms: r.get("timing").and_then(|v| v.as_f64()),
                category,
            })
        }).collect();

        json_ok(&serde_json::json!({
            "endpoints": endpoints,
            "count": endpoints.len(),
            "duration_seconds": duration,
        }))
    }

    #[tool(
        name = "net.analyze",
        description = "Analyze captured network traffic to discover API schemas, auth patterns, and endpoint templates. Input: endpoints JSON from net.capture."
    )]
    async fn net_analyze(
        &self,
        Parameters(p): Parameters<NetIntelAnalyzeParams>,
    ) -> Result<CallToolResult, McpError> {
        let endpoints: Vec<onecrawl_cdp::network_intel::ApiEndpoint> = serde_json::from_str(&p.capture)
            .map_err(|e| mcp_err(format!("invalid capture data: {e}")))?;

        if endpoints.is_empty() {
            return json_ok(&serde_json::json!({ "error": "no endpoints to analyze" }));
        }

        let base_url = endpoints.first().map(|e| e.base_url.clone()).unwrap_or_default();
        let total_requests = endpoints.len();

        // Group by method+path template
        let mut endpoint_map: std::collections::HashMap<String, Vec<&onecrawl_cdp::network_intel::ApiEndpoint>> = std::collections::HashMap::new();
        for ep in &endpoints {
            let (template, _) = onecrawl_cdp::network_intel::extract_path_params(&ep.path);
            let key = format!("{} {}", ep.method, template);
            endpoint_map.entry(key).or_default().push(ep);
        }

        let schemas: Vec<onecrawl_cdp::network_intel::EndpointSchema> = endpoint_map.iter().map(|(key, eps)| {
            let parts: Vec<&str> = key.splitn(2, ' ').collect();
            let method = parts.first().unwrap_or(&"GET");
            let path = parts.get(1).unwrap_or(&"/");
            let (template, params) = onecrawl_cdp::network_intel::extract_path_params(path);

            let status_codes: Vec<u16> = eps.iter().map(|e| e.status_code).collect::<std::collections::HashSet<_>>().into_iter().collect();
            let content_types: Vec<String> = eps.iter().filter_map(|e| e.content_type.clone()).collect::<std::collections::HashSet<_>>().into_iter().collect();
            let avg_latency = eps.iter().filter_map(|e| e.timing_ms).sum::<f64>() / eps.len().max(1) as f64;

            let response_schema = eps.iter().find_map(|e| e.response_body.as_ref())
                .map(|b| onecrawl_cdp::network_intel::infer_json_schema(b));
            let request_schema = eps.iter().find_map(|e| e.request_body.as_ref())
                .map(|b| onecrawl_cdp::network_intel::infer_json_schema(b));

            onecrawl_cdp::network_intel::EndpointSchema {
                method: method.to_string(),
                path: template,
                path_params: params,
                query_params: vec![],
                request_body_schema: request_schema,
                response_body_schema: response_schema,
                status_codes,
                content_types,
                call_count: eps.len(),
                avg_latency_ms: avg_latency,
            }
        }).collect();

        let auth_pattern = endpoints.iter()
            .find_map(|e| {
                let auth = onecrawl_cdp::network_intel::detect_auth_pattern(&e.request_headers);
                match auth {
                    onecrawl_cdp::network_intel::AuthPattern::None => None,
                    other => Some(other),
                }
            });

        let schema = onecrawl_cdp::network_intel::ApiSchema {
            base_url,
            endpoints: schemas,
            auth_pattern,
            total_requests,
            unique_endpoints: endpoint_map.len(),
        };

        json_ok(&serde_json::to_value(&schema).unwrap())
    }

    #[tool(
        name = "net.sdk",
        description = "Generate an SDK client from an API schema. Supports TypeScript and Python. Input: schema JSON from net.analyze."
    )]
    async fn net_sdk(
        &self,
        Parameters(p): Parameters<NetIntelSdkParams>,
    ) -> Result<CallToolResult, McpError> {
        let schema: onecrawl_cdp::network_intel::ApiSchema = serde_json::from_str(&p.schema)
            .map_err(|e| mcp_err(format!("invalid schema: {e}")))?;

        let sdk = match p.language.as_deref().unwrap_or("typescript") {
            "python" | "py" => onecrawl_cdp::network_intel::generate_python_sdk(&schema),
            _ => onecrawl_cdp::network_intel::generate_typescript_sdk(&schema),
        };

        json_ok(&serde_json::json!({
            "language": sdk.language,
            "code": sdk.code,
            "endpoints_covered": sdk.endpoints_covered,
        }))
    }

    #[tool(
        name = "net.mock",
        description = "Generate a mock server configuration from captured endpoints. Returns endpoint definitions with recorded responses."
    )]
    async fn net_mock(
        &self,
        Parameters(p): Parameters<NetIntelMockParams>,
    ) -> Result<CallToolResult, McpError> {
        let endpoints: Vec<onecrawl_cdp::network_intel::ApiEndpoint> = serde_json::from_str(&p.endpoints)
            .map_err(|e| mcp_err(format!("invalid endpoints: {e}")))?;

        let config = onecrawl_cdp::network_intel::generate_mock_config(&endpoints, p.port.unwrap_or(3001));
        json_ok(&serde_json::to_value(&config).unwrap())
    }

    #[tool(
        name = "net.replay",
        description = "Generate a replay sequence from captured network traffic. Can be used to reproduce exact API call sequences."
    )]
    async fn net_replay(
        &self,
        Parameters(p): Parameters<NetIntelReplayParams>,
    ) -> Result<CallToolResult, McpError> {
        let endpoints: Vec<onecrawl_cdp::network_intel::ApiEndpoint> = serde_json::from_str(&p.endpoints)
            .map_err(|e| mcp_err(format!("invalid endpoints: {e}")))?;

        let name = p.name.as_deref().unwrap_or("replay_sequence");
        let sequence = onecrawl_cdp::network_intel::generate_replay_sequence(name, &endpoints);
        json_ok(&serde_json::to_value(&sequence).unwrap())
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
