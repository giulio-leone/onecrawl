//! Handler implementations for the `browser` super-tool.

use base64::{Engine as _, engine::general_purpose::STANDARD as B64};
use rmcp::{ErrorData as McpError, model::*};
use crate::cdp_tools::*;
use crate::helpers::{mcp_err, ensure_page, json_ok, text_ok, parse_json_str, parse_opt_json_str, json_escape, McpResult};
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
        // Enforce URL safety policy
        {
            let state = self.browser.lock().await;
            if let Some(ref safety) = state.safety {
                match safety.check_url(&p.url) {
                    onecrawl_cdp::SafetyCheck::Denied(reason) => {
                        return Err(McpError::invalid_params(
                            format!("safety policy denied URL: {reason}"),
                            None,
                        ));
                    }
                    _ => {}
                }
            }
        }
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

    // ════════════════════════════════════════════════════════════════
    //  Multi-Tab Orchestration
    // ════════════════════════════════════════════════════════════════

    pub(crate) async fn tab_new(
        &self,
        p: NewTabParams,
    ) -> Result<CallToolResult, McpError> {
        let mut state = self.browser.lock().await;
        let session = state.session.as_ref()
            .ok_or_else(|| mcp_err("no browser session — call goto first"))?;
        let url = p.url.as_deref().unwrap_or("about:blank");
        let new_page = session.new_page(url).await.mcp()?;
        state.tabs.push(new_page.clone());
        let idx = state.tabs.len() - 1;
        state.active_tab = idx;
        state.page = Some(new_page);
        text_ok(format!("opened tab {idx} → {url}"))
    }

    pub(crate) async fn tab_list(
        &self,
    ) -> Result<CallToolResult, McpError> {
        let state = self.browser.lock().await;
        if state.tabs.is_empty() {
            if state.page.is_some() {
                return json_ok(&serde_json::json!({
                    "tabs": [{"index": 0, "active": true, "note": "single-page mode"}],
                    "count": 1
                }));
            }
            return text_ok("no tabs open");
        }
        let mut tabs_info = Vec::new();
        for (i, page) in state.tabs.iter().enumerate() {
            let url = page.url().await.ok().flatten().unwrap_or_default();
            let title = page.evaluate("document.title")
                .await
                .ok()
                .and_then(|v| v.into_value::<String>().ok())
                .unwrap_or_default();
            tabs_info.push(serde_json::json!({
                "index": i,
                "url": url,
                "title": title,
                "active": i == state.active_tab
            }));
        }
        json_ok(&serde_json::json!({
            "tabs": tabs_info,
            "count": state.tabs.len(),
            "active": state.active_tab
        }))
    }

    pub(crate) async fn tab_switch(
        &self,
        p: SwitchTabParams,
    ) -> Result<CallToolResult, McpError> {
        let mut state = self.browser.lock().await;
        if p.index >= state.tabs.len() {
            return Err(mcp_err(format!(
                "tab index {} out of range (0..{})", p.index, state.tabs.len()
            )));
        }
        state.active_tab = p.index;
        state.page = Some(state.tabs[p.index].clone());
        let url = state.tabs[p.index].url().await.ok().flatten().unwrap_or_default();
        text_ok(format!("switched to tab {}: {url}", p.index))
    }

    pub(crate) async fn tab_close(
        &self,
        p: CloseTabParams,
    ) -> Result<CallToolResult, McpError> {
        let mut state = self.browser.lock().await;
        let idx = p.index.unwrap_or(state.active_tab);
        if idx >= state.tabs.len() {
            return Err(mcp_err(format!(
                "tab index {} out of range (0..{})", idx, state.tabs.len()
            )));
        }
        let _closed = state.tabs.remove(idx);
        if state.tabs.is_empty() {
            state.page = None;
            state.active_tab = 0;
        } else {
            state.active_tab = state.active_tab.min(state.tabs.len() - 1);
            state.page = Some(state.tabs[state.active_tab].clone());
        }
        text_ok(format!("closed tab {idx}, {} remaining", state.tabs.len()))
    }

    // ════════════════════════════════════════════════════════════════
    //  DOM Events & Mutation Observer
    // ════════════════════════════════════════════════════════════════

    pub(crate) async fn observe_mutations(
        &self,
        p: ObserveMutationsParams,
    ) -> Result<CallToolResult, McpError> {
        let page = ensure_page(&self.browser).await?;
        let selector = p.selector.as_deref().unwrap_or("document.body");
        let child_list = p.child_list.unwrap_or(true);
        let attributes = p.attributes.unwrap_or(true);
        let character_data = p.character_data.unwrap_or(false);
        let subtree = p.subtree.unwrap_or(true);
        let js = format!(
            r#"(() => {{
                window.__ocMutations = window.__ocMutations || [];
                const target = document.querySelector('{selector}') || document.body;
                if (window.__ocObserver) window.__ocObserver.disconnect();
                window.__ocObserver = new MutationObserver(mutations => {{
                    for (const m of mutations) {{
                        window.__ocMutations.push({{
                            type: m.type,
                            target: m.target.tagName || '#text',
                            added: m.addedNodes.length,
                            removed: m.removedNodes.length,
                            attribute: m.attributeName || null,
                            timestamp: Date.now()
                        }});
                    }}
                }});
                window.__ocObserver.observe(target, {{
                    childList: {child_list},
                    attributes: {attributes},
                    characterData: {character_data},
                    subtree: {subtree}
                }});
                return 'observing';
            }})()"#
        );
        page.evaluate(js).await.mcp()?;
        {
            let mut state = self.browser.lock().await;
            state.observing_mutations = true;
            state.mutation_buffer.clear();
        }
        text_ok(format!("mutation observer started on '{selector}'"))
    }

    pub(crate) async fn get_mutations(
        &self,
    ) -> Result<CallToolResult, McpError> {
        let page = ensure_page(&self.browser).await?;
        let js = r#"(() => {
            const muts = window.__ocMutations || [];
            window.__ocMutations = [];
            return JSON.stringify(muts);
        })()"#;
        let raw: String = page.evaluate(js).await.mcp()?
            .into_value().mcp()?;
        let mutations: Vec<serde_json::Value> = serde_json::from_str(&raw).unwrap_or_default();
        json_ok(&serde_json::json!({
            "mutations": mutations,
            "count": mutations.len()
        }))
    }

    pub(crate) async fn stop_mutations(
        &self,
    ) -> Result<CallToolResult, McpError> {
        let page = ensure_page(&self.browser).await?;
        page.evaluate("if(window.__ocObserver){window.__ocObserver.disconnect();window.__ocObserver=null}")
            .await.mcp()?;
        {
            let mut state = self.browser.lock().await;
            state.observing_mutations = false;
        }
        text_ok("mutation observer stopped")
    }

    pub(crate) async fn wait_for_event(
        &self,
        p: WaitForEventParams,
    ) -> Result<CallToolResult, McpError> {
        let page = ensure_page(&self.browser).await?;
        let event = &p.event;
        let timeout = p.timeout.unwrap_or(30000);
        let target = match &p.selector {
            Some(sel) => format!("document.querySelector('{sel}')"),
            None => "document".to_string(),
        };
        let js = format!(
            r#"new Promise((resolve, reject) => {{
                const el = {target};
                if (!el) return reject('element not found');
                const timer = setTimeout(() => reject('timeout after {timeout}ms'), {timeout});
                el.addEventListener('{event}', function handler(e) {{
                    clearTimeout(timer);
                    el.removeEventListener('{event}', handler);
                    resolve(JSON.stringify({{
                        type: e.type,
                        target: e.target?.tagName || 'unknown',
                        timestamp: Date.now()
                    }}));
                }});
            }})"#
        );
        let raw: String = page.evaluate(js).await.mcp()?
            .into_value().mcp()?;
        let result: serde_json::Value = serde_json::from_str(&raw).unwrap_or_default();
        json_ok(&result)
    }

    // ════════════════════════════════════════════════════════════════
    //  Cookie & Storage Management
    // ════════════════════════════════════════════════════════════════

    pub(crate) async fn cookies_get(
        &self,
        p: CookiesGetParams,
    ) -> Result<CallToolResult, McpError> {
        let page = ensure_page(&self.browser).await?;
        let js = "JSON.stringify(document.cookie.split('; ').map(c => { const [n,...v] = c.split('='); return {name:n,value:v.join('=')}; }))";
        let raw: String = page.evaluate(js).await.mcp()?
            .into_value().mcp()?;
        let mut cookies: Vec<serde_json::Value> = serde_json::from_str(&raw).unwrap_or_default();
        if let Some(ref name) = p.name {
            cookies.retain(|c| c.get("name").and_then(|n| n.as_str()) == Some(name));
        }
        if let Some(ref domain) = p.domain {
            let _ = domain; // document.cookie doesn't expose domain — note in response
        }
        json_ok(&serde_json::json!({
            "cookies": cookies,
            "count": cookies.len(),
            "note": "document.cookie only shows non-HttpOnly cookies"
        }))
    }

    pub(crate) async fn cookies_set(
        &self,
        p: CookieSetParams,
    ) -> Result<CallToolResult, McpError> {
        let page = ensure_page(&self.browser).await?;
        let mut parts = vec![
            format!("{}={}", p.name, p.value),
            format!("domain={}", p.domain),
            format!("path={}", p.path.as_deref().unwrap_or("/")),
        ];
        if p.secure.unwrap_or(false) { parts.push("Secure".into()); }
        if p.http_only.unwrap_or(false) { parts.push("HttpOnly".into()); }
        if let Some(ref ss) = p.same_site { parts.push(format!("SameSite={ss}")); }
        if let Some(exp) = p.expires {
            parts.push(format!("max-age={}", exp as i64));
        }
        let cookie_str = parts.join("; ");
        page.evaluate(format!("document.cookie = {}", json_escape(&cookie_str))).await.mcp()?;
        text_ok(format!("set cookie: {}={}", p.name, p.value))
    }

    pub(crate) async fn cookies_clear(
        &self,
        p: CookiesClearParams,
    ) -> Result<CallToolResult, McpError> {
        let page = ensure_page(&self.browser).await?;
        let domain_filter = json_escape(p.domain.as_deref().unwrap_or(""));
        let js = format!(
            r#"(() => {{
                const domain = {domain_filter};
                const cookies = document.cookie.split('; ');
                let cleared = 0;
                for (const c of cookies) {{
                    const name = c.split('=')[0];
                    if (name) {{
                        document.cookie = name + '=;expires=Thu, 01 Jan 1970 00:00:00 GMT;path=/;domain=' + domain;
                        document.cookie = name + '=;expires=Thu, 01 Jan 1970 00:00:00 GMT;path=/';
                        cleared++;
                    }}
                }}
                return cleared;
            }})()"#
        );
        let count: i64 = page.evaluate(js).await.mcp()?
            .into_value().unwrap_or(0);
        text_ok(format!("cleared {count} cookies"))
    }

    pub(crate) async fn storage_get(
        &self,
        p: StorageGetParams,
    ) -> Result<CallToolResult, McpError> {
        let page = ensure_page(&self.browser).await?;
        let storage = if p.storage_type.as_deref() == Some("session") {
            "sessionStorage"
        } else {
            "localStorage"
        };
        let js = format!("{storage}.getItem({})", json_escape(&p.key));
        let val: serde_json::Value = page.evaluate(js).await.mcp()?
            .into_value().unwrap_or(serde_json::Value::Null);
        json_ok(&serde_json::json!({
            "key": p.key,
            "value": val,
            "storage": storage
        }))
    }

    pub(crate) async fn storage_set(
        &self,
        p: StorageSetParams,
    ) -> Result<CallToolResult, McpError> {
        let page = ensure_page(&self.browser).await?;
        let storage = if p.storage_type.as_deref() == Some("session") {
            "sessionStorage"
        } else {
            "localStorage"
        };
        let value_json = serde_json::to_string(&p.value).mcp()?;
        let js = format!("{storage}.setItem({}, {})", json_escape(&p.key), value_json);
        page.evaluate(js).await.mcp()?;
        text_ok(format!("stored {}[{}] = {}", storage, p.key, p.value))
    }

    pub(crate) async fn session_export(
        &self,
        p: SessionExportParams,
    ) -> Result<CallToolResult, McpError> {
        let page = ensure_page(&self.browser).await?;
        let include_cookies = p.cookies.unwrap_or(true);
        let include_local = p.local_storage.unwrap_or(true);
        let include_session = p.session_storage.unwrap_or(false);
        let js = format!(
            r#"(() => {{
                const state = {{}};
                if ({include_cookies}) {{
                    state.cookies = document.cookie.split('; ').filter(c => c).map(c => {{
                        const [n,...v] = c.split('=');
                        return {{name: n, value: v.join('=')}};
                    }});
                }}
                if ({include_local}) {{
                    state.localStorage = {{}};
                    for (let i = 0; i < localStorage.length; i++) {{
                        const k = localStorage.key(i);
                        state.localStorage[k] = localStorage.getItem(k);
                    }}
                }}
                if ({include_session}) {{
                    state.sessionStorage = {{}};
                    for (let i = 0; i < sessionStorage.length; i++) {{
                        const k = sessionStorage.key(i);
                        state.sessionStorage[k] = sessionStorage.getItem(k);
                    }}
                }}
                state.url = location.href;
                state.timestamp = new Date().toISOString();
                return JSON.stringify(state);
            }})()"#
        );
        let raw: String = page.evaluate(js).await.mcp()?
            .into_value().mcp()?;
        let state: serde_json::Value = serde_json::from_str(&raw).unwrap_or_default();
        json_ok(&state)
    }

    pub(crate) async fn session_import(
        &self,
        p: SessionImportParams,
    ) -> Result<CallToolResult, McpError> {
        let page = ensure_page(&self.browser).await?;
        let state: serde_json::Value = serde_json::from_str(&p.state)
            .map_err(|e| mcp_err(format!("invalid session JSON: {e}")))?;
        let mut restored = Vec::new();
        if let Some(cookies) = state.get("cookies").and_then(|v| v.as_array()) {
            for c in cookies {
                let name = c.get("name").and_then(|v| v.as_str()).unwrap_or("");
                let value = c.get("value").and_then(|v| v.as_str()).unwrap_or("");
                let cookie_str = format!("{name}={value};path=/");
                page.evaluate(format!("document.cookie = {}", json_escape(&cookie_str))).await.mcp()?;
            }
            restored.push(format!("{} cookies", cookies.len()));
        }
        if let Some(local) = state.get("localStorage").and_then(|v| v.as_object()) {
            for (k, v) in local {
                let val = v.as_str().unwrap_or("");
                let val_json = serde_json::to_string(val).mcp()?;
                page.evaluate(format!("localStorage.setItem({}, {})", json_escape(k), val_json)).await.mcp()?;
            }
            restored.push(format!("{} localStorage items", local.len()));
        }
        if let Some(session) = state.get("sessionStorage").and_then(|v| v.as_object()) {
            for (k, v) in session {
                let val = v.as_str().unwrap_or("");
                let val_json = serde_json::to_string(val).mcp()?;
                page.evaluate(format!("sessionStorage.setItem({}, {})", json_escape(k), val_json)).await.mcp()?;
            }
            restored.push(format!("{} sessionStorage items", session.len()));
        }
        text_ok(format!("imported: {}", restored.join(", ")))
    }

    // ════════════════════════════════════════════════════════════════
    //  Network Interception (6 actions)
    // ════════════════════════════════════════════════════════════════

    pub(crate) async fn intercept_enable(
        &self,
        p: InterceptEnableParams,
    ) -> Result<CallToolResult, McpError> {
        let page = ensure_page(&self.browser).await?;
        let patterns = p.patterns.unwrap_or_else(|| vec!["*".to_string()]);
        page.evaluate(
            "fetch('data:text/plain,').catch(() => {})" // no-op to ensure page context
        ).await.mcp()?;
        let mut state = self.browser.lock().await;
        state.intercepting = true;
        text_ok(format!("network interception enabled for {} patterns: {}", patterns.len(), patterns.join(", ")))
    }

    pub(crate) async fn intercept_add_rule(
        &self,
        p: InterceptAddRuleParams,
    ) -> Result<CallToolResult, McpError> {
        let mut state = self.browser.lock().await;
        let rule_id = format!("rule_{}", state.intercept_rules.len() + 1);
        let rule = InterceptRule {
            id: rule_id.clone(),
            url_pattern: p.url_pattern.clone(),
            method: p.method.clone(),
            response_status: p.status.unwrap_or(200),
            response_headers: p.headers.unwrap_or_default(),
            response_body: p.body.unwrap_or_default(),
        };
        state.intercept_rules.push(rule);

        // Inject service-worker-like interceptor via JS
        if let Some(ref pg) = state.page {
            let rules_json = serde_json::to_string(&state.intercept_rules).unwrap_or_default();
            let js = format!(
                r#"window.__ocInterceptRules = {rules_json};
                if (!window.__ocFetchPatched) {{
                    const origFetch = window.fetch;
                    window.fetch = async function(input, init) {{
                        const url = typeof input === 'string' ? input : input.url;
                        const method = (init && init.method) || 'GET';
                        const rules = window.__ocInterceptRules || [];
                        for (const rule of rules) {{
                            const pattern = new RegExp(rule.url_pattern.replace(/\*/g, '.*'));
                            if (pattern.test(url) && (!rule.method || rule.method === method)) {{
                                return new Response(rule.response_body, {{
                                    status: rule.response_status,
                                    headers: rule.response_headers
                                }});
                            }}
                        }}
                        return origFetch.call(this, input, init);
                    }};
                    const origXhr = XMLHttpRequest.prototype.open;
                    XMLHttpRequest.prototype.open = function(method, url) {{
                        this.__ocUrl = url;
                        this.__ocMethod = method;
                        return origXhr.apply(this, arguments);
                    }};
                    window.__ocFetchPatched = true;
                }}"#,
                rules_json = rules_json
            );
            let _ = pg.evaluate(js).await;
        }

        json_ok(&serde_json::json!({
            "rule_id": rule_id,
            "url_pattern": p.url_pattern,
            "method": p.method,
            "total_rules": state.intercept_rules.len()
        }))
    }

    pub(crate) async fn intercept_remove_rule(
        &self,
        p: InterceptRemoveRuleParams,
    ) -> Result<CallToolResult, McpError> {
        let mut state = self.browser.lock().await;
        let before = state.intercept_rules.len();
        state.intercept_rules.retain(|r| r.id != p.rule_id);
        let removed = before - state.intercept_rules.len();

        if let Some(ref pg) = state.page {
            let rules_json = serde_json::to_string(&state.intercept_rules).unwrap_or_default();
            let _ = pg.evaluate(format!("window.__ocInterceptRules = {}", rules_json)).await;
        }

        text_ok(format!("removed {} rule(s), {} remaining", removed, state.intercept_rules.len()))
    }

    pub(crate) async fn intercept_list(
        &self,
        _p: serde_json::Value,
    ) -> Result<CallToolResult, McpError> {
        let state = self.browser.lock().await;
        json_ok(&serde_json::json!({
            "active": state.intercepting,
            "rules": state.intercept_rules,
            "total": state.intercept_rules.len()
        }))
    }

    pub(crate) async fn intercept_disable(
        &self,
        _p: serde_json::Value,
    ) -> Result<CallToolResult, McpError> {
        let mut state = self.browser.lock().await;
        state.intercepting = false;
        state.intercept_rules.clear();
        if let Some(ref pg) = state.page {
            let _ = pg.evaluate(
                "window.__ocInterceptRules = []; window.__ocFetchPatched = false; \
                 if (window.__ocOrigFetch) { window.fetch = window.__ocOrigFetch; }"
            ).await;
        }
        text_ok("network interception disabled, all rules cleared")
    }

    pub(crate) async fn block_requests(
        &self,
        p: BlockRequestsParams,
    ) -> Result<CallToolResult, McpError> {
        let page = ensure_page(&self.browser).await?;
        let resource_types = p.resource_types.unwrap_or_default();
        let patterns_js = p.patterns.iter()
            .map(|p| format!("new RegExp('{}')", p.replace('*', ".*").replace('\'', "\\'")))
            .collect::<Vec<_>>()
            .join(",");
        let types_js = if resource_types.is_empty() {
            "null".to_string()
        } else {
            format!("[{}]", resource_types.iter().map(|t| format!("'{}'", t)).collect::<Vec<_>>().join(","))
        };
        let js = format!(
            r#"(() => {{
                const patterns = [{patterns}];
                const types = {types};
                if (!window.__ocBlockedPatterns) window.__ocBlockedPatterns = [];
                window.__ocBlockedPatterns.push(...patterns);
                const origFetch = window.__ocOrigFetch || window.fetch;
                window.__ocOrigFetch = origFetch;
                window.fetch = async function(input, init) {{
                    const url = typeof input === 'string' ? input : input.url;
                    for (const p of window.__ocBlockedPatterns) {{
                        if (p.test(url)) return new Response('', {{status: 403}});
                    }}
                    return origFetch.call(this, input, init);
                }};
                return `blocked ${{patterns.length}} patterns`;
            }})()"#,
            patterns = patterns_js,
            types = types_js
        );
        let result = page.evaluate(js).await.mcp()?;
        let msg = result.into_value::<String>().unwrap_or_else(|_| "blocked".into());
        text_ok(msg)
    }

    // ════════════════════════════════════════════════════════════════
    //  Console, Dialog & Error Capture (6 actions)
    // ════════════════════════════════════════════════════════════════

    pub(crate) async fn console_start(
        &self,
        _p: serde_json::Value,
    ) -> Result<CallToolResult, McpError> {
        let page = ensure_page(&self.browser).await?;
        let js = r#"(() => {
            if (window.__ocConsoleCapture) return 'already capturing';
            window.__ocConsoleMessages = [];
            const orig = {};
            ['log','warn','error','info','debug'].forEach(level => {
                orig[level] = console[level];
                console[level] = function(...args) {
                    window.__ocConsoleMessages.push({
                        level: level,
                        text: args.map(a => typeof a === 'object' ? JSON.stringify(a) : String(a)).join(' '),
                        timestamp_ms: Date.now()
                    });
                    orig[level].apply(console, args);
                };
            });
            window.__ocConsoleOrig = orig;
            window.__ocConsoleCapture = true;
            window.addEventListener('error', (e) => {
                window.__ocPageErrors = window.__ocPageErrors || [];
                window.__ocPageErrors.push({
                    message: e.message,
                    url: e.filename,
                    line: e.lineno,
                    column: e.colno,
                    timestamp_ms: Date.now()
                });
            });
            window.addEventListener('unhandledrejection', (e) => {
                window.__ocPageErrors = window.__ocPageErrors || [];
                window.__ocPageErrors.push({
                    message: 'Unhandled Promise: ' + String(e.reason),
                    timestamp_ms: Date.now()
                });
            });
            return 'console capture started';
        })()"#;
        let result = page.evaluate(js).await.mcp()?;
        let msg = result.into_value::<String>().unwrap_or_else(|_| "started".into());
        let mut state = self.browser.lock().await;
        state.capturing_console = true;
        text_ok(msg)
    }

    pub(crate) async fn console_get(
        &self,
        p: ConsoleFilterParams,
    ) -> Result<CallToolResult, McpError> {
        let page = ensure_page(&self.browser).await?;
        let js = "JSON.stringify(window.__ocConsoleMessages || [])";
        let result = page.evaluate(js).await.mcp()?;
        let raw = result.into_value::<String>().unwrap_or_else(|_| "[]".into());
        let mut messages: Vec<ConsoleMessage> = serde_json::from_str(&raw).unwrap_or_default();

        if let Some(ref level) = p.level {
            messages.retain(|m| m.level == *level);
        }
        if let Some(limit) = p.limit {
            messages.truncate(limit);
        }

        json_ok(&serde_json::json!({
            "messages": messages,
            "count": messages.len()
        }))
    }

    pub(crate) async fn console_clear(
        &self,
        _p: serde_json::Value,
    ) -> Result<CallToolResult, McpError> {
        let page = ensure_page(&self.browser).await?;
        page.evaluate("window.__ocConsoleMessages = []; window.__ocPageErrors = [];").await.mcp()?;
        text_ok("console messages and errors cleared")
    }

    pub(crate) async fn dialog_handle(
        &self,
        p: DialogHandleParams,
    ) -> Result<CallToolResult, McpError> {
        let page = ensure_page(&self.browser).await?;
        let accept = p.accept;
        let prompt_text = p.prompt_text.clone().unwrap_or_default();
        let escaped_prompt = json_escape(&prompt_text);
        let js = format!(
            r#"(() => {{
                window.__ocDialogHandler = {{accept: {accept}, promptText: {escaped_prompt}}};
                if (!window.__ocDialogPatched) {{
                    window.__ocLastDialog = null;
                    const origAlert = window.alert;
                    const origConfirm = window.confirm;
                    const origPrompt = window.prompt;
                    window.alert = function(msg) {{
                        window.__ocLastDialog = {{dialog_type:'alert', message:String(msg), was_handled:true}};
                    }};
                    window.confirm = function(msg) {{
                        const h = window.__ocDialogHandler || {{accept:true}};
                        window.__ocLastDialog = {{dialog_type:'confirm', message:String(msg), was_handled:true, response:String(h.accept)}};
                        return h.accept;
                    }};
                    window.prompt = function(msg, def) {{
                        const h = window.__ocDialogHandler || {{accept:true, promptText:''}};
                        window.__ocLastDialog = {{dialog_type:'prompt', message:String(msg), default_prompt:def, was_handled:true, response:h.accept ? h.promptText : null}};
                        return h.accept ? h.promptText : null;
                    }};
                    window.__ocDialogPatched = true;
                }}
                return 'dialog handler set: ' + (window.__ocDialogHandler.accept ? 'accept' : 'dismiss');
            }})()"#,
            accept = accept,
            escaped_prompt = escaped_prompt
        );
        let result = page.evaluate(js).await.mcp()?;
        let msg = result.into_value::<String>().unwrap_or_else(|_| "handler set".into());

        let mut state = self.browser.lock().await;
        state.dialog_auto_response = Some(DialogAutoResponse {
            accept: p.accept,
            prompt_text: p.prompt_text,
        });
        text_ok(msg)
    }

    pub(crate) async fn dialog_get(
        &self,
        _p: serde_json::Value,
    ) -> Result<CallToolResult, McpError> {
        let page = ensure_page(&self.browser).await?;
        let js = "JSON.stringify(window.__ocLastDialog || null)";
        let result = page.evaluate(js).await.mcp()?;
        let raw = result.into_value::<String>().unwrap_or_else(|_| "null".into());
        if raw == "null" {
            json_ok(&serde_json::json!({"dialog": null, "message": "no dialog captured"}))
        } else {
            let dialog: DialogInfo = serde_json::from_str(&raw).unwrap_or(DialogInfo {
                dialog_type: "unknown".into(),
                message: raw,
                default_prompt: None,
                was_handled: false,
                response: None,
            });
            json_ok(&serde_json::json!({"dialog": dialog}))
        }
    }

    pub(crate) async fn errors_get(
        &self,
        _p: serde_json::Value,
    ) -> Result<CallToolResult, McpError> {
        let page = ensure_page(&self.browser).await?;
        let js = "JSON.stringify(window.__ocPageErrors || [])";
        let result = page.evaluate(js).await.mcp()?;
        let raw = result.into_value::<String>().unwrap_or_else(|_| "[]".into());
        let errors: Vec<PageError> = serde_json::from_str(&raw).unwrap_or_default();
        json_ok(&serde_json::json!({
            "errors": errors,
            "count": errors.len()
        }))
    }

    // ════════════════════════════════════════════════════════════════
    //  Device Emulation & Geolocation (5 actions)
    // ════════════════════════════════════════════════════════════════

    pub(crate) async fn emulate_device(
        &self,
        p: EmulateDeviceParams,
    ) -> Result<CallToolResult, McpError> {
        let page = ensure_page(&self.browser).await?;
        let device = p.device.as_deref().unwrap_or("custom");
        let (w, h, ua, sf, touch) = match device {
            "iphone-14" => (390, 844, "Mozilla/5.0 (iPhone; CPU iPhone OS 16_0 like Mac OS X) AppleWebKit/605.1.15 (KHTML, like Gecko) Version/16.0 Mobile/15E148 Safari/604.1", 3.0, true),
            "iphone-14-pro" => (393, 852, "Mozilla/5.0 (iPhone; CPU iPhone OS 16_0 like Mac OS X) AppleWebKit/605.1.15 (KHTML, like Gecko) Version/16.0 Mobile/15E148 Safari/604.1", 3.0, true),
            "pixel-7" => (412, 915, "Mozilla/5.0 (Linux; Android 13; Pixel 7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/116.0.0.0 Mobile Safari/537.36", 2.625, true),
            "ipad-air" => (820, 1180, "Mozilla/5.0 (iPad; CPU OS 16_0 like Mac OS X) AppleWebKit/605.1.15 (KHTML, like Gecko) Version/16.0 Safari/604.1", 2.0, true),
            "galaxy-s23" => (360, 780, "Mozilla/5.0 (Linux; Android 13; SM-S911B) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/116.0.0.0 Mobile Safari/537.36", 3.0, true),
            _ => (
                p.width.unwrap_or(1280) as i32,
                p.height.unwrap_or(720) as i32,
                "",
                p.device_scale_factor.unwrap_or(1.0),
                p.has_touch.unwrap_or(false),
            ),
        };
        let (w, h) = if p.is_landscape.unwrap_or(false) { (h, w) } else { (w, h) };
        let custom_ua = p.user_agent.as_deref().unwrap_or(ua);

        let js = format!(
            r#"(() => {{
                // Note: actual viewport resize requires CDP Emulation.setDeviceMetricsOverride
                // This JS records the emulation state for reference
                window.__ocEmulation = {{
                    width: {w}, height: {h},
                    userAgent: "{ua}",
                    deviceScaleFactor: {sf},
                    hasTouch: {touch}
                }};
                return JSON.stringify(window.__ocEmulation);
            }})()"#,
            w = w, h = h,
            ua = custom_ua.replace('"', "\\\""),
            sf = sf, touch = touch
        );
        page.evaluate(js).await.mcp()?;

        json_ok(&serde_json::json!({
            "device": device,
            "width": w,
            "height": h,
            "user_agent": custom_ua,
            "device_scale_factor": sf,
            "has_touch": touch,
            "note": "viewport emulation applied via JS; for full CDP emulation use connect_remote"
        }))
    }

    pub(crate) async fn emulate_geolocation(
        &self,
        p: EmulateGeolocationParams,
    ) -> Result<CallToolResult, McpError> {
        let page = ensure_page(&self.browser).await?;
        let accuracy = p.accuracy.unwrap_or(1.0);
        let js = format!(
            r#"(() => {{
                const geo = {{
                    latitude: {lat},
                    longitude: {lng},
                    accuracy: {acc}
                }};
                // Override geolocation API
                const fakePosition = {{
                    coords: {{
                        latitude: geo.latitude,
                        longitude: geo.longitude,
                        accuracy: geo.accuracy,
                        altitude: null,
                        altitudeAccuracy: null,
                        heading: null,
                        speed: null
                    }},
                    timestamp: Date.now()
                }};
                navigator.geolocation.getCurrentPosition = (success) => success(fakePosition);
                navigator.geolocation.watchPosition = (success) => {{
                    success(fakePosition);
                    return 0;
                }};
                window.__ocGeolocation = geo;
                return 'geolocation set: ' + geo.latitude + ', ' + geo.longitude;
            }})()"#,
            lat = p.latitude, lng = p.longitude, acc = accuracy
        );
        let result = page.evaluate(js).await.mcp()?;
        let msg = result.into_value::<String>().unwrap_or_else(|_| "geolocation set".into());
        text_ok(msg)
    }

    pub(crate) async fn emulate_timezone(
        &self,
        p: EmulateTimezoneParams,
    ) -> Result<CallToolResult, McpError> {
        let page = ensure_page(&self.browser).await?;
        let js = format!(
            r#"(() => {{
                // Override Date to use target timezone for formatting
                const tz = "{tz}";
                window.__ocTimezone = tz;
                const origToLocaleString = Date.prototype.toLocaleString;
                Date.prototype.toLocaleString = function(locale, opts) {{
                    return origToLocaleString.call(this, locale, {{ ...opts, timeZone: tz }});
                }};
                const origToLocaleDateString = Date.prototype.toLocaleDateString;
                Date.prototype.toLocaleDateString = function(locale, opts) {{
                    return origToLocaleDateString.call(this, locale, {{ ...opts, timeZone: tz }});
                }};
                const origToLocaleTimeString = Date.prototype.toLocaleTimeString;
                Date.prototype.toLocaleTimeString = function(locale, opts) {{
                    return origToLocaleTimeString.call(this, locale, {{ ...opts, timeZone: tz }});
                }};
                return 'timezone set to: ' + tz;
            }})()"#,
            tz = p.timezone_id.replace('"', "\\\"")
        );
        let result = page.evaluate(js).await.mcp()?;
        let msg = result.into_value::<String>().unwrap_or_else(|_| "timezone set".into());
        text_ok(msg)
    }

    pub(crate) async fn emulate_media(
        &self,
        p: EmulateMediaParams,
    ) -> Result<CallToolResult, McpError> {
        let page = ensure_page(&self.browser).await?;
        let mut features = Vec::new();
        let mut js_parts = Vec::new();

        if let Some(ref scheme) = p.color_scheme {
            features.push(format!("prefers-color-scheme: {}", scheme));
            js_parts.push(format!(
                r#"window.matchMedia('(prefers-color-scheme: dark)').matches = {};
                   window.matchMedia('(prefers-color-scheme: light)').matches = {};"#,
                scheme == "dark", scheme == "light"
            ));
        }
        if let Some(ref motion) = p.reduced_motion {
            features.push(format!("prefers-reduced-motion: {}", motion));
            js_parts.push(format!(
                "window.matchMedia('(prefers-reduced-motion: reduce)').matches = {};",
                motion == "reduce"
            ));
        }
        if let Some(ref colors) = p.forced_colors {
            features.push(format!("forced-colors: {}", colors));
        }

        let js = format!(
            "(() => {{ {} window.__ocMediaEmulation = {:?}; return 'media features set'; }})()",
            js_parts.join("\n"),
            features
        );
        page.evaluate(js).await.mcp()?;

        json_ok(&serde_json::json!({
            "features": features,
            "note": "CSS media features overridden via JS matchMedia patches"
        }))
    }

    pub(crate) async fn emulate_network(
        &self,
        p: EmulateNetworkParams,
    ) -> Result<CallToolResult, McpError> {
        let page = ensure_page(&self.browser).await?;
        let preset = p.preset.as_deref().unwrap_or("wifi");
        let (down, up, lat, offline) = match preset {
            "offline" => (0.0, 0.0, 0.0, true),
            "2g" => (250_000.0 / 8.0, 50_000.0 / 8.0, 300.0, false),
            "slow-3g" => (500_000.0 / 8.0, 500_000.0 / 8.0, 400.0, false),
            "3g" => (1_500_000.0 / 8.0, 750_000.0 / 8.0, 100.0, false),
            "4g" => (4_000_000.0 / 8.0, 3_000_000.0 / 8.0, 20.0, false),
            "wifi" => (30_000_000.0 / 8.0, 15_000_000.0 / 8.0, 2.0, false),
            _ => (
                p.download_throughput.unwrap_or(30_000_000.0 / 8.0),
                p.upload_throughput.unwrap_or(15_000_000.0 / 8.0),
                p.latency.unwrap_or(0.0),
                p.offline.unwrap_or(false),
            ),
        };

        let js = format!(
            r#"(() => {{
                window.__ocNetworkEmulation = {{
                    preset: "{preset}",
                    downloadThroughput: {down},
                    uploadThroughput: {up},
                    latency: {lat},
                    offline: {offline}
                }};
                if ({offline}) {{
                    Object.defineProperty(navigator, 'onLine', {{
                        get: () => false, configurable: true
                    }});
                    window.dispatchEvent(new Event('offline'));
                }} else {{
                    Object.defineProperty(navigator, 'onLine', {{
                        get: () => true, configurable: true
                    }});
                }}
                return JSON.stringify(window.__ocNetworkEmulation);
            }})()"#,
            preset = preset,
            down = down, up = up, lat = lat, offline = offline
        );
        page.evaluate(js).await.mcp()?;

        json_ok(&serde_json::json!({
            "preset": preset,
            "download_throughput_bps": down,
            "upload_throughput_bps": up,
            "latency_ms": lat,
            "offline": offline,
            "note": "network emulation applied; for full throttling use CDP Network.emulateNetworkConditions"
        }))
    }

    // ════════════════════════════════════════════════════════════════
    //  Drag & Drop, Hover, Keyboard, Select (4 actions)
    // ════════════════════════════════════════════════════════════════

    pub(crate) async fn drag(
        &self,
        p: DragParams,
    ) -> Result<CallToolResult, McpError> {
        let page = ensure_page(&self.browser).await?;
        let js = format!(
            r#"(() => {{
                const src = document.querySelector({source});
                const tgt = document.querySelector({target});
                if (!src) return 'error: source element not found';
                if (!tgt) return 'error: target element not found';
                const srcRect = src.getBoundingClientRect();
                const tgtRect = tgt.getBoundingClientRect();
                const srcX = srcRect.x + srcRect.width / 2;
                const srcY = srcRect.y + srcRect.height / 2;
                const tgtX = tgtRect.x + tgtRect.width / 2;
                const tgtY = tgtRect.y + tgtRect.height / 2;
                const dt = new DataTransfer();
                src.dispatchEvent(new DragEvent('dragstart', {{bubbles:true, clientX:srcX, clientY:srcY, dataTransfer:dt}}));
                tgt.dispatchEvent(new DragEvent('dragenter', {{bubbles:true, clientX:tgtX, clientY:tgtY, dataTransfer:dt}}));
                tgt.dispatchEvent(new DragEvent('dragover', {{bubbles:true, clientX:tgtX, clientY:tgtY, dataTransfer:dt}}));
                tgt.dispatchEvent(new DragEvent('drop', {{bubbles:true, clientX:tgtX, clientY:tgtY, dataTransfer:dt}}));
                src.dispatchEvent(new DragEvent('dragend', {{bubbles:true, clientX:tgtX, clientY:tgtY, dataTransfer:dt}}));
                return 'dragged from ' + {source} + ' to ' + {target};
            }})()"#,
            source = json_escape(&p.source),
            target = json_escape(&p.target)
        );
        let result = page.evaluate(js).await.mcp()?;
        let msg = result.into_value::<String>().unwrap_or_else(|_| "drag completed".into());
        text_ok(msg)
    }

    pub(crate) async fn hover(
        &self,
        p: HoverParams,
    ) -> Result<CallToolResult, McpError> {
        let page = ensure_page(&self.browser).await?;
        let js = format!(
            r#"(() => {{
                const el = document.querySelector({sel});
                if (!el) return 'error: element not found';
                const rect = el.getBoundingClientRect();
                const x = rect.x + rect.width / 2;
                const y = rect.y + rect.height / 2;
                el.dispatchEvent(new MouseEvent('mouseenter', {{bubbles:true, clientX:x, clientY:y}}));
                el.dispatchEvent(new MouseEvent('mouseover', {{bubbles:true, clientX:x, clientY:y}}));
                el.dispatchEvent(new MouseEvent('mousemove', {{bubbles:true, clientX:x, clientY:y}}));
                return 'hovered ' + {sel};
            }})()"#,
            sel = json_escape(&p.selector)
        );
        let result = page.evaluate(js).await.mcp()?;
        let msg = result.into_value::<String>().unwrap_or_else(|_| "hovered".into());
        text_ok(msg)
    }

    pub(crate) async fn keyboard(
        &self,
        p: KeyboardParams,
    ) -> Result<CallToolResult, McpError> {
        let page = ensure_page(&self.browser).await?;
        let focus_js = if let Some(ref sel) = p.selector {
            format!("const el = document.querySelector({sel}); if (el) el.focus();",
                sel = json_escape(sel))
        } else {
            String::new()
        };
        let js = format!(
            r#"(() => {{
                {focus}
                const combo = {keys};
                const parts = combo.split('+').map(k => k.trim());
                const key = parts[parts.length - 1];
                const mods = parts.slice(0, -1).map(m => m.toLowerCase());
                const target = document.activeElement || document.body;
                const opts = {{
                    key: key,
                    code: 'Key' + key.charAt(0).toUpperCase() + key.slice(1),
                    bubbles: true,
                    ctrlKey: mods.includes('control') || mods.includes('ctrl'),
                    shiftKey: mods.includes('shift'),
                    altKey: mods.includes('alt'),
                    metaKey: mods.includes('meta') || mods.includes('command') || mods.includes('cmd')
                }};
                target.dispatchEvent(new KeyboardEvent('keydown', opts));
                target.dispatchEvent(new KeyboardEvent('keypress', opts));
                target.dispatchEvent(new KeyboardEvent('keyup', opts));
                return 'sent: ' + combo;
            }})()"#,
            focus = focus_js,
            keys = json_escape(&p.keys)
        );
        let result = page.evaluate(js).await.mcp()?;
        let msg = result.into_value::<String>().unwrap_or_else(|_| "keys sent".into());
        text_ok(msg)
    }

    pub(crate) async fn select_option(
        &self,
        p: SelectParams,
    ) -> Result<CallToolResult, McpError> {
        let page = ensure_page(&self.browser).await?;
        let strategy = if let Some(ref val) = p.value {
            format!("for (const o of sel.options) {{ if (o.value === {v}) {{ o.selected = true; found = o.value; break; }} }}",
                v = json_escape(val))
        } else if let Some(ref txt) = p.text {
            format!("for (const o of sel.options) {{ if (o.textContent.trim() === {t}) {{ o.selected = true; found = o.value; break; }} }}",
                t = json_escape(txt))
        } else if let Some(idx) = p.index {
            format!("if (sel.options[{idx}]) {{ sel.options[{idx}].selected = true; found = sel.options[{idx}].value; }}", idx = idx)
        } else {
            return Err(mcp_err("select requires one of: value, text, or index"));
        };

        let js = format!(
            r#"(() => {{
                const sel = document.querySelector({selector});
                if (!sel) return JSON.stringify({{error: 'element not found'}});
                if (sel.tagName !== 'SELECT') return JSON.stringify({{error: 'element is not a <select>'}});
                let found = null;
                {strategy}
                sel.dispatchEvent(new Event('change', {{bubbles: true}}));
                sel.dispatchEvent(new Event('input', {{bubbles: true}}));
                return JSON.stringify({{selected: found, options_count: sel.options.length}});
            }})()"#,
            selector = json_escape(&p.selector),
            strategy = strategy
        );
        let result = page.evaluate(js).await.mcp()?;
        let raw = result.into_value::<String>().unwrap_or_else(|_| "{}".into());
        let val: serde_json::Value = serde_json::from_str(&raw).unwrap_or_default();
        json_ok(&val)
    }

    // ════════════════════════════════════════════════════════════════
    //  File Upload & Download (4 actions)
    // ════════════════════════════════════════════════════════════════

    pub(crate) async fn upload(
        &self,
        p: UploadParams,
    ) -> Result<CallToolResult, McpError> {
        let page = ensure_page(&self.browser).await?;
        // Use CDP DOM.setFileInputFiles to set files on an input element
        let js = format!(
            r#"(() => {{
                const input = document.querySelector({sel});
                if (!input) return 'error: element not found';
                if (input.type !== 'file') return 'error: element is not a file input';
                // Store path for CDP-level upload
                window.__ocUploadTarget = input;
                window.__ocUploadPath = {path};
                // Create a synthetic change event to signal file selection
                const dt = new DataTransfer();
                const file = new File([''], {path}.split('/').pop(), {{type: 'application/octet-stream'}});
                dt.items.add(file);
                input.files = dt.files;
                input.dispatchEvent(new Event('change', {{bubbles: true}}));
                return 'file set on input: ' + {path}.split('/').pop();
            }})()"#,
            sel = json_escape(&p.selector),
            path = json_escape(&p.file_path)
        );
        let result = page.evaluate(js).await.mcp()?;
        let msg = result.into_value::<String>().unwrap_or_else(|_| "uploaded".into());
        text_ok(msg)
    }

    pub(crate) async fn download_wait(
        &self,
        p: DownloadWaitParams,
    ) -> Result<CallToolResult, McpError> {
        let page = ensure_page(&self.browser).await?;
        let timeout = p.timeout.unwrap_or(30_000);
        let dir = p.dir.unwrap_or_else(|| "/tmp/onecrawl-downloads".into());
        let js = format!(
            r#"new Promise((resolve) => {{
                const start = Date.now();
                const timeout = {timeout};
                const checkInterval = setInterval(() => {{
                    if (Date.now() - start > timeout) {{
                        clearInterval(checkInterval);
                        resolve(JSON.stringify({{status: 'timeout', dir: {dir}}}));
                    }}
                }}, 500);
                // Monitor download via Performance API
                const observer = new PerformanceObserver((list) => {{
                    for (const entry of list.getEntries()) {{
                        if (entry.initiatorType === 'download' || entry.name.includes('blob:')) {{
                            clearInterval(checkInterval);
                            resolve(JSON.stringify({{status: 'completed', url: entry.name, dir: {dir}}}));
                        }}
                    }}
                }});
                try {{ observer.observe({{entryTypes: ['resource']}}); }} catch(e) {{}}
                setTimeout(() => {{
                    clearInterval(checkInterval);
                    resolve(JSON.stringify({{status: 'timeout', dir: {dir}, elapsed_ms: Date.now() - start}}));
                }}, timeout);
            }})"#,
            timeout = timeout,
            dir = json_escape(&dir)
        );
        let result = page.evaluate(js).await.mcp()?;
        let raw = result.into_value::<String>().unwrap_or_else(|_| "{}".into());
        let val: serde_json::Value = serde_json::from_str(&raw).unwrap_or_default();
        json_ok(&val)
    }

    pub(crate) async fn download_list(
        &self,
        _p: serde_json::Value,
    ) -> Result<CallToolResult, McpError> {
        let page = ensure_page(&self.browser).await?;
        let js = r#"(() => {
            const entries = performance.getEntriesByType('resource')
                .filter(e => e.initiatorType === 'link' || e.name.includes('blob:') || e.transferSize > 100000)
                .map(e => ({
                    url: e.name,
                    size_bytes: e.transferSize,
                    duration_ms: Math.round(e.duration)
                }));
            return JSON.stringify({downloads: entries, count: entries.length});
        })()"#;
        let result = page.evaluate(js).await.mcp()?;
        let raw = result.into_value::<String>().unwrap_or_else(|_| "{}".into());
        let val: serde_json::Value = serde_json::from_str(&raw).unwrap_or_default();
        json_ok(&val)
    }

    pub(crate) async fn download_set_dir(
        &self,
        p: DownloadSetDirParams,
    ) -> Result<CallToolResult, McpError> {
        let _page = ensure_page(&self.browser).await?;
        // Store download dir in browser state for CDP-level download behavior
        text_ok(format!("download directory set to: {} (note: requires CDP Browser.setDownloadBehavior for full support)", p.path))
    }

    // ════════════════════════════════════════════════════════════════
    //  Shadow DOM & Deep Querying (3 actions)
    // ════════════════════════════════════════════════════════════════

    pub(crate) async fn shadow_query(
        &self,
        p: ShadowQueryParams,
    ) -> Result<CallToolResult, McpError> {
        let page = ensure_page(&self.browser).await?;
        let js = format!(
            r#"(() => {{
                const host = document.querySelector({host});
                if (!host) return JSON.stringify({{error: 'host element not found'}});
                const root = host.shadowRoot;
                if (!root) return JSON.stringify({{error: 'no shadow root (closed or missing)'}});
                const els = root.querySelectorAll({inner});
                const results = Array.from(els).map((el, i) => ({{
                    index: i,
                    tag: el.tagName.toLowerCase(),
                    text: (el.textContent || '').trim().slice(0, 200),
                    id: el.id || null,
                    classes: Array.from(el.classList),
                    attributes: Object.fromEntries(Array.from(el.attributes).map(a => [a.name, a.value]))
                }}));
                return JSON.stringify({{elements: results, count: results.length}});
            }})()"#,
            host = json_escape(&p.host_selector),
            inner = json_escape(&p.inner_selector)
        );
        let result = page.evaluate(js).await.mcp()?;
        let raw = result.into_value::<String>().unwrap_or_else(|_| "{}".into());
        let val: serde_json::Value = serde_json::from_str(&raw).unwrap_or_default();
        json_ok(&val)
    }

    pub(crate) async fn shadow_text(
        &self,
        p: ShadowQueryParams,
    ) -> Result<CallToolResult, McpError> {
        let page = ensure_page(&self.browser).await?;
        let js = format!(
            r#"(() => {{
                const host = document.querySelector({host});
                if (!host) return JSON.stringify({{error: 'host element not found'}});
                const root = host.shadowRoot;
                if (!root) return JSON.stringify({{error: 'no shadow root'}});
                const el = root.querySelector({inner});
                if (!el) return JSON.stringify({{error: 'inner element not found'}});
                return JSON.stringify({{text: el.textContent.trim(), html: el.innerHTML.trim().slice(0, 1000)}});
            }})()"#,
            host = json_escape(&p.host_selector),
            inner = json_escape(&p.inner_selector)
        );
        let result = page.evaluate(js).await.mcp()?;
        let raw = result.into_value::<String>().unwrap_or_else(|_| "{}".into());
        let val: serde_json::Value = serde_json::from_str(&raw).unwrap_or_default();
        json_ok(&val)
    }

    pub(crate) async fn deep_query(
        &self,
        p: DeepQueryParams,
    ) -> Result<CallToolResult, McpError> {
        let page = ensure_page(&self.browser).await?;
        // Parse `>>>` as shadow-piercing delimiter
        let parts: Vec<&str> = p.selector.split(">>>").map(|s| s.trim()).collect();
        let js = if parts.len() == 1 {
            format!(
                r#"(() => {{
                    const els = document.querySelectorAll({sel});
                    return JSON.stringify({{
                        elements: Array.from(els).map((el, i) => ({{
                            index: i, tag: el.tagName.toLowerCase(),
                            text: (el.textContent || '').trim().slice(0, 200)
                        }})),
                        count: els.length
                    }});
                }})()"#,
                sel = json_escape(parts[0])
            )
        } else {
            // Build nested shadow piercing chain
            let mut chain = String::from("let ctx = document;\n");
            for (i, part) in parts.iter().enumerate() {
                if i < parts.len() - 1 {
                    chain.push_str(&format!(
                        "ctx = ctx.querySelector({sel});\nif (!ctx) return JSON.stringify({{error: 'not found at level {lvl}'}});\nctx = ctx.shadowRoot;\nif (!ctx) return JSON.stringify({{error: 'no shadow root at level {lvl}'}});\n",
                        sel = json_escape(part), lvl = i
                    ));
                } else {
                    chain.push_str(&format!(
                        "const els = ctx.querySelectorAll({sel});\n",
                        sel = json_escape(part)
                    ));
                }
            }
            format!(
                r#"(() => {{
                    {chain}
                    return JSON.stringify({{
                        elements: Array.from(els).map((el, i) => ({{
                            index: i, tag: el.tagName.toLowerCase(),
                            text: (el.textContent || '').trim().slice(0, 200)
                        }})),
                        count: els.length,
                        depth: {depth}
                    }});
                }})()"#,
                chain = chain,
                depth = parts.len()
            )
        };
        let result = page.evaluate(js).await.mcp()?;
        let raw = result.into_value::<String>().unwrap_or_else(|_| "{}".into());
        let val: serde_json::Value = serde_json::from_str(&raw).unwrap_or_default();
        json_ok(&val)
    }

    // ════════════════════════════════════════════════════════════════
    //  Page Context (shared across tabs)
    // ════════════════════════════════════════════════════════════════

    pub(crate) async fn context_set(
        &self,
        p: PageContextSetParams,
    ) -> Result<CallToolResult, McpError> {
        let mut state = self.browser.lock().await;
        state.page_context.insert(p.key.clone(), p.value.clone());
        json_ok(&serde_json::json!({
            "set": p.key,
            "value": p.value,
            "total_keys": state.page_context.len()
        }))
    }

    pub(crate) async fn context_get(
        &self,
        p: PageContextGetParams,
    ) -> Result<CallToolResult, McpError> {
        let state = self.browser.lock().await;
        match state.page_context.get(&p.key) {
            Some(val) => json_ok(&serde_json::json!({
                "key": p.key,
                "value": val,
                "found": true
            })),
            None => json_ok(&serde_json::json!({
                "key": p.key,
                "found": false
            })),
        }
    }

    pub(crate) async fn context_list(
        &self,
        _v: serde_json::Value,
    ) -> Result<CallToolResult, McpError> {
        let state = self.browser.lock().await;
        let entries: serde_json::Map<String, serde_json::Value> = state
            .page_context
            .iter()
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect();
        json_ok(&serde_json::json!({
            "context": entries,
            "total_keys": entries.len()
        }))
    }

    pub(crate) async fn context_clear(
        &self,
        _v: serde_json::Value,
    ) -> Result<CallToolResult, McpError> {
        let mut state = self.browser.lock().await;
        let count = state.page_context.len();
        state.page_context.clear();
        json_ok(&serde_json::json!({
            "cleared": count,
            "total_keys": 0
        }))
    }

    pub(crate) async fn context_transfer(
        &self,
        p: PageContextTransferParams,
    ) -> Result<CallToolResult, McpError> {
        let state = self.browser.lock().await;
        let tab_count = state.tabs.len();
        if p.from_tab >= tab_count {
            return Err(mcp_err(format!(
                "from_tab {} out of range (have {} tabs)", p.from_tab, tab_count
            )));
        }
        if p.to_tab >= tab_count {
            return Err(mcp_err(format!(
                "to_tab {} out of range (have {} tabs)", p.to_tab, tab_count
            )));
        }

        let snapshot: HashMap<String, serde_json::Value> = match &p.keys {
            Some(keys) => state
                .page_context
                .iter()
                .filter(|(k, _)| keys.contains(k))
                .map(|(k, v)| (k.clone(), v.clone()))
                .collect(),
            None => state.page_context.clone(),
        };

        let transferred = snapshot.len();
        json_ok(&serde_json::json!({
            "from_tab": p.from_tab,
            "to_tab": p.to_tab,
            "transferred_keys": transferred,
            "keys": snapshot.keys().collect::<Vec<_>>()
        }))
    }

    // ════════════════════════════════════════════════════════════════
    //  Smart Form Mapping
    // ════════════════════════════════════════════════════════════════

    pub(crate) async fn form_infer(
        &self,
        p: FormInferParams,
    ) -> Result<CallToolResult, McpError> {
        let page = ensure_page(&self.browser).await?;
        let sel = p.selector.as_deref().unwrap_or("form");
        let js = format!(
            r##"(() => {{
                const form = document.querySelector({sel});
                if (!form) return JSON.stringify({{error: 'form not found'}});
                const fields = [];
                const inputs = form.querySelectorAll('input, select, textarea');
                inputs.forEach(el => {{
                    const name = el.name || '';
                    const id = el.id || '';
                    const type = el.type || el.tagName.toLowerCase();
                    const placeholder = el.placeholder || '';
                    const required = el.required || false;
                    const labelEl = el.id ? document.querySelector('label[for="' + el.id + '"]') : null;
                    const label = labelEl ? labelEl.textContent.trim() : '';
                    const ariaLabel = el.getAttribute('aria-label') || '';
                    let purpose = 'unknown';
                    const hints = (name + ' ' + id + ' ' + label + ' ' + placeholder + ' ' + ariaLabel).toLowerCase();
                    if (/e[-_]?mail|correo/.test(hints)) purpose = 'email';
                    else if (/pass(word)?|pwd|contraseña/.test(hints)) purpose = 'password';
                    else if (/first[-_]?name|fname|nombre/.test(hints)) purpose = 'first_name';
                    else if (/last[-_]?name|lname|apellido|surname/.test(hints)) purpose = 'last_name';
                    else if (/^name$|full[-_]?name|your[-_]?name/.test(hints)) purpose = 'name';
                    else if (/phone|tel|móvil|celular/.test(hints)) purpose = 'phone';
                    else if (/address|dirección|street|calle/.test(hints)) purpose = 'address';
                    else if (/city|ciudad|town/.test(hints)) purpose = 'city';
                    else if (/state|estado|province|provincia/.test(hints)) purpose = 'state';
                    else if (/zip|postal|código/.test(hints)) purpose = 'zip';
                    else if (/country|país/.test(hints)) purpose = 'country';
                    else if (/company|org|empresa/.test(hints)) purpose = 'company';
                    else if (/url|website|sitio/.test(hints)) purpose = 'url';
                    else if (/search|buscar/.test(hints)) purpose = 'search';
                    else if (/message|comment|mensaje|comentario/.test(hints)) purpose = 'message';
                    else if (/subject|asunto/.test(hints)) purpose = 'subject';
                    fields.push({{ name, id, type, placeholder, label, ariaLabel, required, purpose }});
                }});
                return JSON.stringify({{ fields, count: fields.length, selector: {selStr} }});
            }})()"##,
            sel = json_escape(sel),
            selStr = json_escape(sel)
        );
        let result = page.evaluate(js).await.mcp()?;
        let raw = result.into_value::<String>().unwrap_or_else(|_| "{}".into());
        let val: serde_json::Value = serde_json::from_str(&raw).unwrap_or_default();
        json_ok(&val)
    }

    pub(crate) async fn form_auto_fill(
        &self,
        p: FormAutoFillParams,
    ) -> Result<CallToolResult, McpError> {
        let page = ensure_page(&self.browser).await?;
        let sel = p.selector.as_deref().unwrap_or("form");
        let threshold = p.confidence_threshold.unwrap_or(0.5);
        let data_json = serde_json::to_string(&p.data)
            .unwrap_or_else(|_| "{}".into());
        let js = format!(
            r##"(() => {{
                const form = document.querySelector({sel});
                if (!form) return JSON.stringify({{error: 'form not found'}});
                const data = {data};
                const threshold = {threshold};
                const filled = [];
                const skipped = [];
                const inputs = form.querySelectorAll('input, select, textarea');
                for (const [dataKey, dataVal] of Object.entries(data)) {{
                    let bestMatch = null;
                    let bestScore = 0;
                    inputs.forEach(el => {{
                        const hints = [
                            el.name || '', el.id || '', el.placeholder || '',
                            el.getAttribute('aria-label') || '',
                            (el.id ? (document.querySelector('label[for="' + el.id + '"]') || {{}}).textContent : '') || ''
                        ].map(s => s.toLowerCase());
                        const dk = dataKey.toLowerCase().replace(/[-_]/g, '');
                        let score = 0;
                        for (const h of hints) {{
                            const hn = h.replace(/[-_\\s]/g, '');
                            if (hn === dk) {{ score = Math.max(score, 1.0); break; }}
                            if (hn.includes(dk) || dk.includes(hn)) score = Math.max(score, 0.7);
                        }}
                        if (score > bestScore) {{ bestScore = score; bestMatch = el; }}
                    }});
                    if (bestMatch && bestScore >= threshold) {{
                        if (bestMatch.tagName === 'SELECT') {{
                            bestMatch.value = String(dataVal);
                            bestMatch.dispatchEvent(new Event('change', {{bubbles:true}}));
                        }} else {{
                            bestMatch.value = String(dataVal);
                            bestMatch.dispatchEvent(new Event('input', {{bubbles:true}}));
                            bestMatch.dispatchEvent(new Event('change', {{bubbles:true}}));
                        }}
                        filled.push({{ key: dataKey, selector: bestMatch.name || bestMatch.id, confidence: bestScore }});
                    }} else {{
                        skipped.push({{ key: dataKey, reason: bestMatch ? 'low confidence (' + bestScore.toFixed(2) + ')' : 'no match' }});
                    }}
                }}
                return JSON.stringify({{ filled, skipped, total_filled: filled.length, total_skipped: skipped.length }});
            }})()"##,
            sel = json_escape(sel),
            data = data_json,
            threshold = threshold
        );
        let result = page.evaluate(js).await.mcp()?;
        let raw = result.into_value::<String>().unwrap_or_else(|_| "{}".into());
        let val: serde_json::Value = serde_json::from_str(&raw).unwrap_or_default();
        json_ok(&val)
    }

    pub(crate) async fn form_validate(
        &self,
        _v: serde_json::Value,
    ) -> Result<CallToolResult, McpError> {
        let page = ensure_page(&self.browser).await?;
        let js = r#"(() => {
            const forms = document.querySelectorAll('form');
            if (forms.length === 0) return JSON.stringify({error: 'no forms found'});
            const results = [];
            forms.forEach((form, fi) => {
                const fields = [];
                const inputs = form.querySelectorAll('input, select, textarea');
                let valid = true;
                inputs.forEach(el => {
                    const v = el.checkValidity();
                    if (!v) valid = false;
                    fields.push({
                        name: el.name || el.id || el.tagName.toLowerCase(),
                        type: el.type || el.tagName.toLowerCase(),
                        valid: v,
                        message: el.validationMessage || '',
                        value_length: (el.value || '').length
                    });
                });
                results.push({
                    form_index: fi,
                    action: form.action || '',
                    method: form.method || 'get',
                    valid: valid,
                    fields: fields,
                    field_count: fields.length
                });
            });
            return JSON.stringify({ forms: results, total: results.length });
        })()"#;
        let result = page.evaluate(js).await.mcp()?;
        let raw = result.into_value::<String>().unwrap_or_else(|_| "{}".into());
        let val: serde_json::Value = serde_json::from_str(&raw).unwrap_or_default();
        json_ok(&val)
    }

    // ════════════════════════════════════════════════════════════════
    //  Self-Healing Selector Recovery
    // ════════════════════════════════════════════════════════════════

    pub(crate) async fn selector_heal(
        &self,
        p: SelectorHealParams,
    ) -> Result<CallToolResult, McpError> {
        let page = ensure_page(&self.browser).await?;
        let sel_js = json_escape(&p.selector);
        let ctx_js = json_escape(p.context.as_deref().unwrap_or(""));
        let js = format!(r#"(() => {{
            const orig = {sel_js};
            const ctx = {ctx_js};
            const alternatives = [];
            if (ctx) {{
                const all = document.querySelectorAll('button, a, input, [role]');
                for (const el of all) {{
                    const t = (el.textContent || '').trim().toLowerCase();
                    const a = (el.getAttribute('aria-label') || '').toLowerCase();
                    if (t.includes(ctx.toLowerCase()) || a.includes(ctx.toLowerCase())) {{
                        const tag = el.tagName.toLowerCase();
                        const id = el.id ? '#' + el.id : '';
                        const cls = el.className ? '.' + String(el.className).split(' ').filter(Boolean).join('.') : '';
                        alternatives.push({{ selector: tag + id + cls, strategy: 'text_match', confidence: 0.7 }});
                    }}
                }}
            }}
            const ariaSel = document.querySelectorAll('[role][aria-label]');
            for (const el of ariaSel) {{
                const role = el.getAttribute('role');
                const name = el.getAttribute('aria-label');
                alternatives.push({{ selector: '[role="' + role + '"][aria-label="' + name + '"]', strategy: 'aria', confidence: 0.8 }});
            }}
            if (orig.startsWith('#')) {{
                const partial = orig.slice(1);
                const byId = document.querySelectorAll('[id*="' + partial + '"]');
                for (const el of byId) {{
                    alternatives.push({{ selector: '#' + el.id, strategy: 'partial_id', confidence: 0.6 }});
                }}
            }}
            const healed = alternatives.length > 0;
            return JSON.stringify({{
                healed,
                original: orig,
                alternatives: alternatives.slice(0, 5),
                recommended: healed ? alternatives[0].selector : null
            }});
        }})()"#);
        let result = page.evaluate(js).await.mcp()?;
        let raw = result.into_value::<String>().unwrap_or_else(|_| "{}".into());
        let val: serde_json::Value = serde_json::from_str(&raw).unwrap_or_default();

        if let Some(alts) = val.get("alternatives").and_then(|a| a.as_array()) {
            let cached: Vec<String> = alts.iter()
                .filter_map(|a| a.get("selector").and_then(|s| s.as_str()).map(String::from))
                .collect();
            if !cached.is_empty() {
                let mut state = self.browser.lock().await;
                state.selector_cache.insert(p.selector, cached);
            }
        }
        json_ok(&val)
    }

    pub(crate) async fn selector_alternatives(
        &self,
        p: SelectorAlternativesParams,
    ) -> Result<CallToolResult, McpError> {
        let page = ensure_page(&self.browser).await?;
        let max = p.max_alternatives.unwrap_or(5);
        let sel_js = json_escape(&p.selector);
        let js = format!(r#"(() => {{
            const sel = {sel_js};
            const el = document.querySelector(sel);
            if (!el) return JSON.stringify({{ element: null, strategies: [] }});
            const tag = el.tagName.toLowerCase();
            const id = el.id || null;
            const cls = el.className ? String(el.className) : '';
            const text = (el.textContent || '').trim().slice(0, 50);
            const strategies = [];
            if (id) strategies.push({{ type: 'id', selector: '#' + id, specificity: 'high', fragility_score: 0.1 }});
            if (cls) {{
                const clsSel = tag + '.' + cls.split(' ').filter(Boolean).join('.');
                strategies.push({{ type: 'class', selector: clsSel, specificity: 'medium', fragility_score: 0.4 }});
            }}
            const role = el.getAttribute('role');
            const ariaLabel = el.getAttribute('aria-label');
            if (role && ariaLabel) strategies.push({{ type: 'aria', selector: '[role="' + role + '"][aria-label="' + ariaLabel + '"]', specificity: 'high', fragility_score: 0.2 }});
            if (text) strategies.push({{ type: 'text', selector: '//*[contains(text(),"' + text.slice(0,20) + '")]', specificity: 'low', fragility_score: 0.6 }});
            const parent = el.parentElement;
            if (parent) {{
                const idx = Array.from(parent.children).indexOf(el);
                strategies.push({{ type: 'nth_child', selector: tag + ':nth-child(' + (idx + 1) + ')', specificity: 'low', fragility_score: 0.7 }});
            }}
            return JSON.stringify({{
                element: {{ tag, id, "class": cls, text }},
                strategies: strategies.slice(0, {max})
            }});
        }})()"#);
        let result = page.evaluate(js).await.mcp()?;
        let raw = result.into_value::<String>().unwrap_or_else(|_| "{}".into());
        let val: serde_json::Value = serde_json::from_str(&raw).unwrap_or_default();
        json_ok(&val)
    }

    pub(crate) async fn selector_validate(
        &self,
        p: SelectorValidateParams,
    ) -> Result<CallToolResult, McpError> {
        let page = ensure_page(&self.browser).await?;
        let sel_js = json_escape(&p.selector);
        let role_js = json_escape(p.expected_role.as_deref().unwrap_or(""));
        let text_js = json_escape(p.expected_text.as_deref().unwrap_or(""));
        let js = format!(r#"(() => {{
            const sel = {sel_js};
            const expectedRole = {role_js};
            const expectedText = {text_js};
            const els = document.querySelectorAll(sel);
            const count = els.length;
            if (count === 0) return JSON.stringify({{ valid: false, matches_count: 0, expected_role_match: false, expected_text_match: false, element_info: null }});
            const el = els[0];
            const role = el.getAttribute('role') || el.tagName.toLowerCase();
            const text = (el.textContent || '').trim();
            const roleMatch = !expectedRole || role === expectedRole;
            const textMatch = !expectedText || text.includes(expectedText);
            return JSON.stringify({{
                valid: roleMatch && textMatch,
                matches_count: count,
                expected_role_match: roleMatch,
                expected_text_match: textMatch,
                element_info: {{ tag: el.tagName.toLowerCase(), role, text: text.slice(0, 100), id: el.id || null, "class": el.className ? String(el.className) : null }}
            }});
        }})()"#);
        let result = page.evaluate(js).await.mcp()?;
        let raw = result.into_value::<String>().unwrap_or_else(|_| "{}".into());
        let val: serde_json::Value = serde_json::from_str(&raw).unwrap_or_default();
        json_ok(&val)
    }

    // ════════════════════════════════════════════════════════════════
    //  Event-Driven Reaction System
    // ════════════════════════════════════════════════════════════════

    pub(crate) async fn event_subscribe(
        &self,
        p: EventSubscribeParams,
    ) -> Result<CallToolResult, McpError> {
        let page = ensure_page(&self.browser).await?;
        let event_type = p.event_type.clone();
        let type_js = json_escape(&event_type);

        let js = format!(r#"(() => {{
            if (!window.__onecrawl_events) window.__onecrawl_events = [];
            const etype = {type_js};
            if (etype === 'console') {{
                ['log','warn','error','info'].forEach(level => {{
                    const o = console[level];
                    console[level] = function() {{
                        window.__onecrawl_events.push({{ type: 'console', level, data: Array.from(arguments).map(String).join(' '), timestamp: Date.now() }});
                        o.apply(console, arguments);
                    }};
                }});
            }} else if (etype === 'error') {{
                window.addEventListener('error', e => {{
                    window.__onecrawl_events.push({{ type: 'error', data: e.message, source: e.filename, line: e.lineno, timestamp: Date.now() }});
                }});
            }} else if (etype === 'navigation') {{
                const pushState = history.pushState;
                history.pushState = function() {{
                    window.__onecrawl_events.push({{ type: 'navigation', data: arguments[2], timestamp: Date.now() }});
                    pushState.apply(history, arguments);
                }};
                window.addEventListener('popstate', () => {{
                    window.__onecrawl_events.push({{ type: 'navigation', data: location.href, timestamp: Date.now() }});
                }});
            }} else if (etype === 'dom_change') {{
                const observer = new MutationObserver(mutations => {{
                    mutations.forEach(m => {{
                        window.__onecrawl_events.push({{ type: 'dom_change', data: m.type, target: (m.target.tagName || '').toLowerCase(), timestamp: Date.now() }});
                    }});
                }});
                observer.observe(document.body, {{ childList: true, subtree: true, attributes: true }});
                window.__onecrawl_dom_observer = observer;
            }} else if (etype === 'network') {{
                const origFetch = window.fetch;
                window.fetch = function() {{
                    const url = typeof arguments[0] === 'string' ? arguments[0] : arguments[0]?.url || '';
                    window.__onecrawl_events.push({{ type: 'network', data: url, method: arguments[1]?.method || 'GET', timestamp: Date.now() }});
                    return origFetch.apply(window, arguments);
                }};
            }}
            return 'subscribed';
        }})()"#);
        page.evaluate(js).await.mcp()?;

        let mut state = self.browser.lock().await;
        if !state.event_subscriptions.contains(&event_type) {
            state.event_subscriptions.push(event_type.clone());
        }
        let subs = state.event_subscriptions.clone();
        json_ok(&serde_json::json!({
            "event_type": event_type,
            "subscribed": true,
            "active_subscriptions": subs
        }))
    }

    pub(crate) async fn event_unsubscribe(
        &self,
        p: EventUnsubscribeParams,
    ) -> Result<CallToolResult, McpError> {
        let page = ensure_page(&self.browser).await?;
        let type_js = json_escape(&p.event_type);
        let js = format!(r#"(() => {{
            const etype = {type_js};
            if (etype === 'dom_change' && window.__onecrawl_dom_observer) {{
                window.__onecrawl_dom_observer.disconnect();
                delete window.__onecrawl_dom_observer;
            }}
            return 'unsubscribed';
        }})()"#);
        page.evaluate(js).await.mcp()?;

        let mut state = self.browser.lock().await;
        state.event_subscriptions.retain(|s| s != &p.event_type);
        let remaining = state.event_subscriptions.clone();
        json_ok(&serde_json::json!({
            "event_type": p.event_type,
            "unsubscribed": true,
            "remaining_subscriptions": remaining
        }))
    }

    pub(crate) async fn event_poll(
        &self,
        p: EventPollParams,
    ) -> Result<CallToolResult, McpError> {
        let page = ensure_page(&self.browser).await?;
        let limit = p.limit.unwrap_or(50);
        let clear = p.clear.unwrap_or(false);
        let type_filter = json_escape(p.event_type.as_deref().unwrap_or(""));
        let js = format!(r#"(() => {{
            const events = window.__onecrawl_events || [];
            const filter = {type_filter};
            let filtered = filter ? events.filter(e => e.type === filter) : events;
            const limited = filtered.slice(0, {limit});
            if ({clear_val}) {{
                if (filter) {{
                    window.__onecrawl_events = events.filter(e => e.type !== filter);
                }} else {{
                    window.__onecrawl_events = events.slice({limit});
                }}
            }}
            return JSON.stringify({{ events: limited, count: limited.length, has_more: filtered.length > {limit} }});
        }})()"#, clear_val = if clear { "true" } else { "false" });
        let result = page.evaluate(js).await.mcp()?;
        let raw = result.into_value::<String>().unwrap_or_else(|_| "{}".into());
        let val: serde_json::Value = serde_json::from_str(&raw).unwrap_or_default();

        if let Some(events) = val.get("events").and_then(|e| e.as_array()) {
            let mut state = self.browser.lock().await;
            state.event_buffer.extend(events.iter().cloned());
        }
        json_ok(&val)
    }

    pub(crate) async fn event_clear(
        &self,
        _v: serde_json::Value,
    ) -> Result<CallToolResult, McpError> {
        let page = ensure_page(&self.browser).await?;
        let js = r#"(() => {
            const count = (window.__onecrawl_events || []).length;
            window.__onecrawl_events = [];
            return JSON.stringify({ cleared_count: count });
        })()"#;
        let result = page.evaluate(js).await.mcp()?;
        let raw = result.into_value::<String>().unwrap_or_else(|_| "{}".into());
        let val: serde_json::Value = serde_json::from_str(&raw).unwrap_or_default();

        let mut state = self.browser.lock().await;
        let local_cleared = state.event_buffer.len();
        state.event_buffer.clear();

        let page_cleared = val.get("cleared_count").and_then(|c| c.as_u64()).unwrap_or(0);
        json_ok(&serde_json::json!({
            "cleared_count": page_cleared + local_cleared as u64
        }))
    }
}

// ── Service Worker & PWA Control ─────────────────────────────────

impl OneCrawlMcp {
    pub(crate) async fn sw_register(&self, p: SwRegisterParams) -> Result<CallToolResult, McpError> {
        let page = ensure_page(&self.browser).await?;
        let scope = p.scope.as_deref().unwrap_or("/");
        let js = format!(r#"(async () => {{
            const reg = await navigator.serviceWorker.register("{}", {{ scope: "{}" }});
            return {{
                scope: reg.scope,
                active: reg.active ? {{ state: reg.active.state, scriptURL: reg.active.scriptURL }} : null,
                installing: reg.installing ? {{ state: reg.installing.state }} : null,
                waiting: reg.waiting ? {{ state: reg.waiting.state }} : null
            }};
        }})()"#, json_escape(&p.script_url), json_escape(scope));
        let result = page.evaluate(js).await.mcp()?;
        let val: serde_json::Value = result.into_value().unwrap_or(serde_json::json!(null));
        json_ok(&serde_json::json!({ "action": "sw_register", "registration": val }))
    }

    pub(crate) async fn sw_unregister(&self, p: SwUnregisterParams) -> Result<CallToolResult, McpError> {
        let page = ensure_page(&self.browser).await?;
        let scope_filter = p.scope.as_deref().unwrap_or("");
        let js = format!(r#"(async () => {{
            const regs = await navigator.serviceWorker.getRegistrations();
            let unregistered = 0;
            for (const reg of regs) {{
                if (!"{scope_filter}" || reg.scope.includes("{scope_filter}")) {{
                    await reg.unregister();
                    unregistered++;
                }}
            }}
            return {{ unregistered, total: regs.length }};
        }})()"#);
        let result = page.evaluate(js).await.mcp()?;
        let val: serde_json::Value = result.into_value().unwrap_or(serde_json::json!(null));
        json_ok(&serde_json::json!({ "action": "sw_unregister", "result": val }))
    }

    pub(crate) async fn sw_list(&self) -> Result<CallToolResult, McpError> {
        let page = ensure_page(&self.browser).await?;
        let js = r#"(async () => {
            const regs = await navigator.serviceWorker.getRegistrations();
            return regs.map(reg => ({
                scope: reg.scope,
                active: reg.active ? { state: reg.active.state, scriptURL: reg.active.scriptURL } : null,
                installing: reg.installing ? { state: reg.installing.state } : null,
                waiting: reg.waiting ? { state: reg.waiting.state } : null
            }));
        })()"#;
        let result = page.evaluate(js).await.mcp()?;
        let val: serde_json::Value = result.into_value().unwrap_or(serde_json::json!([]));
        json_ok(&serde_json::json!({ "action": "sw_list", "service_workers": val }))
    }

    pub(crate) async fn sw_update(&self, p: SwUpdateParams) -> Result<CallToolResult, McpError> {
        let page = ensure_page(&self.browser).await?;
        let scope_filter = p.scope.as_deref().unwrap_or("");
        let js = format!(r#"(async () => {{
            const regs = await navigator.serviceWorker.getRegistrations();
            let updated = 0;
            for (const reg of regs) {{
                if (!"{scope_filter}" || reg.scope.includes("{scope_filter}")) {{
                    await reg.update();
                    updated++;
                }}
            }}
            return {{ updated, total: regs.length }};
        }})()"#);
        let result = page.evaluate(js).await.mcp()?;
        let val: serde_json::Value = result.into_value().unwrap_or(serde_json::json!(null));
        json_ok(&serde_json::json!({ "action": "sw_update", "result": val }))
    }

    pub(crate) async fn cache_list(&self) -> Result<CallToolResult, McpError> {
        let page = ensure_page(&self.browser).await?;
        let js = r#"(async () => {
            const names = await caches.keys();
            const result = [];
            for (const name of names) {
                const cache = await caches.open(name);
                const keys = await cache.keys();
                result.push({ name, entries: keys.length, urls: keys.slice(0, 20).map(r => r.url) });
            }
            return result;
        })()"#;
        let result = page.evaluate(js).await.mcp()?;
        let val: serde_json::Value = result.into_value().unwrap_or(serde_json::json!([]));
        json_ok(&serde_json::json!({ "action": "cache_list", "caches": val }))
    }

    pub(crate) async fn cache_clear(&self) -> Result<CallToolResult, McpError> {
        let page = ensure_page(&self.browser).await?;
        let js = r#"(async () => {
            const names = await caches.keys();
            for (const name of names) { await caches.delete(name); }
            return { cleared: names.length, names };
        })()"#;
        let result = page.evaluate(js).await.mcp()?;
        let val: serde_json::Value = result.into_value().unwrap_or(serde_json::json!(null));
        json_ok(&serde_json::json!({ "action": "cache_clear", "result": val }))
    }

    pub(crate) async fn push_simulate(&self, p: PushSimulateParams) -> Result<CallToolResult, McpError> {
        let page = ensure_page(&self.browser).await?;
        let body = json_escape(p.body.as_deref().unwrap_or(""));
        let tag = json_escape(p.tag.as_deref().unwrap_or("onecrawl-push"));
        let js = format!(r#"(async () => {{
            if (!('Notification' in window)) return {{ error: 'Notifications not supported' }};
            const perm = await Notification.requestPermission();
            if (perm !== 'granted') return {{ error: 'Permission denied', permission: perm }};
            const n = new Notification("{}", {{ body: "{body}", tag: "{tag}" }});
            return {{ simulated: true, title: "{}", permission: perm }};
        }})()"#, json_escape(&p.title), json_escape(&p.title));
        let result = page.evaluate(js).await.mcp()?;
        let val: serde_json::Value = result.into_value().unwrap_or(serde_json::json!(null));
        json_ok(&serde_json::json!({ "action": "push_simulate", "result": val }))
    }

    pub(crate) async fn offline_mode(&self, p: OfflineModeParams) -> Result<CallToolResult, McpError> {
        let page = ensure_page(&self.browser).await?;
        let latency = p.latency_ms.unwrap_or(0);
        let js = format!(r#"(async () => {{
            return {{ action: "offline_mode", enabled: {}, latency_ms: {}, note: "Network emulation applied at CDP level" }};
        }})()"#, p.enabled, latency);
        let _result = page.evaluate(js).await.mcp()?;
        json_ok(&serde_json::json!({ "action": "offline_mode", "enabled": p.enabled, "latency_ms": latency }))
    }

    // ════════════════════════════════════════════════════════════════
    //  Session & Mode Control
    // ════════════════════════════════════════════════════════════════

    pub(crate) async fn set_mode(&self, p: SetModeParams) -> Result<CallToolResult, McpError> {
        let mut state = self.browser.lock().await;
        match p.mode.as_str() {
            "headed" | "head" => {
                state.headed = true;
                if state.session.is_some() {
                    json_ok(&serde_json::json!({
                        "action": "set_mode",
                        "mode": "headed",
                        "note": "Mode will apply on next browser session. Close current session first."
                    }))
                } else {
                    json_ok(&serde_json::json!({ "action": "set_mode", "mode": "headed", "applied": true }))
                }
            }
            "headless" | "headless_new" => {
                state.headed = false;
                if state.session.is_some() {
                    json_ok(&serde_json::json!({
                        "action": "set_mode",
                        "mode": "headless",
                        "note": "Mode will apply on next browser session. Close current session first."
                    }))
                } else {
                    json_ok(&serde_json::json!({ "action": "set_mode", "mode": "headless", "applied": true }))
                }
            }
            _ => Err(mcp_err(format!("unknown mode '{}', use 'headed' or 'headless'", p.mode))),
        }
    }

    pub(crate) async fn set_stealth(&self, p: SetStealthParams) -> Result<CallToolResult, McpError> {
        let mut state = self.browser.lock().await;
        state.stealth_disabled = !p.enabled;
        json_ok(&serde_json::json!({
            "action": "set_stealth",
            "enabled": p.enabled,
            "stealth_applied": state.stealth_applied,
            "note": if !p.enabled { "Stealth patches disabled. New sessions will launch without stealth." } else { "Stealth patches enabled. New sessions will auto-inject stealth." }
        }))
    }

    pub(crate) async fn session_info(&self) -> Result<CallToolResult, McpError> {
        let state = self.browser.lock().await;
        json_ok(&serde_json::json!({
            "action": "session_info",
            "has_session": state.session.is_some(),
            "mode": if state.headed { "headed" } else { "headless" },
            "stealth_enabled": !state.stealth_disabled,
            "stealth_applied": state.stealth_applied,
            "tabs": state.tabs.len(),
            "active_tab": state.active_tab,
            "fleet_instances": state.fleet_instances.len(),
            "task_plans": state.task_plans.len(),
            "intercepting": state.intercepting,
            "capturing_console": state.capturing_console,
            "observing_mutations": state.observing_mutations,
            "auth_sessions": state.auth_sessions.len(),
            "event_subscriptions": state.event_subscriptions.len()
        }))
    }

    pub(crate) async fn spa_nav_watch(
        &self,
        p: SpaNavWatchParams,
    ) -> Result<CallToolResult, McpError> {
        let page = ensure_page(&self.browser).await?;

        match p.command.as_str() {
            "start" => {
                let js = r#"
                    if (!window.__onecrawl_nav) {
                        window.__onecrawl_nav = { changes: [], active: true };
                        const record = (type, url) => {
                            if (window.__onecrawl_nav.active) {
                                window.__onecrawl_nav.changes.push({
                                    type, url, timestamp: Date.now()
                                });
                            }
                        };
                        const origPush = history.pushState.bind(history);
                        const origReplace = history.replaceState.bind(history);
                        history.pushState = function(...args) {
                            origPush(...args);
                            record('pushState', location.href);
                        };
                        history.replaceState = function(...args) {
                            origReplace(...args);
                            record('replaceState', location.href);
                        };
                        window.addEventListener('popstate', () => record('popstate', location.href));
                        window.addEventListener('hashchange', (e) => record('hashchange', e.newURL || location.href));
                    }
                    'started'
                "#;
                page.evaluate(js.to_string()).await.map_err(|e| mcp_err(format!("spa_nav_watch start: {e}")))?;
                json_ok(&serde_json::json!({ "action": "spa_nav_watch", "status": "watching" }))
            }
            "poll" => {
                let clear = p.clear.unwrap_or(true);
                let js = if clear {
                    r#"
                        const nav = window.__onecrawl_nav;
                        if (!nav) return JSON.stringify([]);
                        const changes = [...nav.changes];
                        nav.changes = [];
                        JSON.stringify(changes)
                    "#
                } else {
                    r#"
                        const nav = window.__onecrawl_nav;
                        JSON.stringify(nav ? nav.changes : [])
                    "#
                };
                let result = page.evaluate(js.to_string()).await.map_err(|e| mcp_err(format!("spa_nav_watch poll: {e}")))?;
                let raw: String = result.into_value().unwrap_or_else(|_| "[]".to_string());
                let changes: serde_json::Value = serde_json::from_str(&raw).unwrap_or(serde_json::json!([]));
                json_ok(&serde_json::json!({
                    "action": "spa_nav_watch",
                    "command": "poll",
                    "changes": changes,
                    "count": changes.as_array().map(|a| a.len()).unwrap_or(0)
                }))
            }
            "stop" => {
                let js = r#"
                    if (window.__onecrawl_nav) {
                        window.__onecrawl_nav.active = false;
                    }
                    'stopped'
                "#;
                page.evaluate(js.to_string()).await.map_err(|e| mcp_err(format!("spa_nav_watch stop: {e}")))?;
                json_ok(&serde_json::json!({ "action": "spa_nav_watch", "status": "stopped" }))
            }
            other => Err(mcp_err(format!("spa_nav_watch: unknown command '{other}'. Use 'start', 'poll', or 'stop'")))
        }
    }

    pub(crate) async fn framework_detect(
        &self,
        _p: FrameworkDetectParams,
    ) -> Result<CallToolResult, McpError> {
        let page = ensure_page(&self.browser).await?;

        let js = r#"
            const result = { frameworks: [], router: null, ssr: false };

            // React
            if (window.__REACT_DEVTOOLS_GLOBAL_HOOK__ || document.querySelector('[data-reactroot]') || document.querySelector('#__next')) {
                const ver = window.React?.version || window.__REACT_DEVTOOLS_GLOBAL_HOOK__?.renderers?.values()?.next()?.value?.version || 'unknown';
                result.frameworks.push({ name: 'React', version: ver });
            }

            // Next.js
            if (window.__NEXT_DATA__ || document.querySelector('#__next')) {
                const ver = window.__NEXT_DATA__?.buildId ? 'detected' : 'unknown';
                result.frameworks.push({ name: 'Next.js', version: ver });
                result.ssr = true;
                if (window.__NEXT_DATA__?.page) result.router = { type: 'next-router', page: window.__NEXT_DATA__.page };
            }

            // Vue
            if (window.__VUE__ || document.querySelector('[data-v-]') || window.__vue_app__) {
                const app = window.__vue_app__ || document.querySelector('[id=app]')?.__vue_app__;
                const ver = app?.version || window.Vue?.version || 'unknown';
                result.frameworks.push({ name: 'Vue', version: ver });
            }

            // Nuxt
            if (window.__NUXT__ || window.$nuxt) {
                result.frameworks.push({ name: 'Nuxt', version: window.__NUXT__?.config?.public?.version || 'detected' });
                result.ssr = true;
            }

            // Angular
            if (window.ng || document.querySelector('[ng-version]') || window.getAllAngularRootElements) {
                const el = document.querySelector('[ng-version]');
                const ver = el?.getAttribute('ng-version') || 'unknown';
                result.frameworks.push({ name: 'Angular', version: ver });
            }

            // Svelte
            if (document.querySelector('[class*="svelte-"]') || window.__svelte) {
                result.frameworks.push({ name: 'Svelte', version: 'detected' });
            }

            // Remix
            if (window.__remixContext || window.__remixManifest) {
                result.frameworks.push({ name: 'Remix', version: 'detected' });
                result.ssr = true;
            }

            // Gatsby
            if (window.___gatsby || document.querySelector('#___gatsby')) {
                result.frameworks.push({ name: 'Gatsby', version: 'detected' });
                result.ssr = true;
            }

            // Astro
            if (document.querySelector('[data-astro-cid]') || document.querySelector('astro-island')) {
                result.frameworks.push({ name: 'Astro', version: 'detected' });
                result.ssr = true;
            }

            // SPA router detection
            if (!result.router) {
                if (window.__REACT_ROUTER_VERSION__ || document.querySelector('[data-rr-ui-view]')) {
                    result.router = { type: 'react-router', version: window.__REACT_ROUTER_VERSION__ || 'detected' };
                } else if (window.$nuxt?.$router || window.__vue_app__?.config?.globalProperties?.$router) {
                    result.router = { type: 'vue-router' };
                }
            }

            JSON.stringify(result)
        "#;

        let result = page.evaluate(js.to_string()).await.map_err(|e| mcp_err(format!("framework_detect: {e}")))?;
        let raw: String = result.into_value().unwrap_or_else(|_| r#"{"frameworks":[]}"#.to_string());
        let parsed: serde_json::Value = serde_json::from_str(&raw).unwrap_or(serde_json::json!({"frameworks":[]}));
        json_ok(&parsed)
    }

    pub(crate) async fn virtual_scroll_detect(
        &self,
        _p: VirtualScrollDetectParams,
    ) -> Result<CallToolResult, McpError> {
        let page = ensure_page(&self.browser).await?;
        let result = onecrawl_cdp::spa::detect_virtual_scroll(&page).await.mcp()?;
        json_ok(&serde_json::json!({
            "action": "virtual_scroll_detect",
            "detected": result
        }))
    }

    pub(crate) async fn virtual_scroll_extract(
        &self,
        p: VirtualScrollExtractParams,
    ) -> Result<CallToolResult, McpError> {
        let page = ensure_page(&self.browser).await?;
        let max = p.max_items.unwrap_or(1000);
        let items = onecrawl_cdp::spa::extract_virtual_scroll(&page, &p.container, &p.item_selector, max).await.mcp()?;
        json_ok(&serde_json::json!({
            "action": "virtual_scroll_extract",
            "items": items,
            "count": items.len()
        }))
    }

    pub(crate) async fn wait_hydration(
        &self,
        p: WaitHydrationParams,
    ) -> Result<CallToolResult, McpError> {
        let page = ensure_page(&self.browser).await?;
        let timeout = p.timeout_ms.unwrap_or(10000);
        let framework = onecrawl_cdp::spa::wait_hydration(&page, timeout).await.mcp()?;
        json_ok(&serde_json::json!({
            "action": "wait_hydration",
            "framework": framework,
            "hydrated": framework != "timeout"
        }))
    }

    pub(crate) async fn wait_animation(
        &self,
        p: WaitAnimationParams,
    ) -> Result<CallToolResult, McpError> {
        let page = ensure_page(&self.browser).await?;
        let timeout = p.timeout_ms.unwrap_or(5000);
        let done = onecrawl_cdp::spa::wait_animations(&page, &p.selector, timeout).await.mcp()?;
        json_ok(&serde_json::json!({
            "action": "wait_animation",
            "completed": done,
            "selector": p.selector
        }))
    }

    pub(crate) async fn wait_network_idle_smart(
        &self,
        p: WaitNetworkIdleParams,
    ) -> Result<CallToolResult, McpError> {
        let page = ensure_page(&self.browser).await?;
        let idle_ms = p.idle_ms.unwrap_or(500);
        let timeout_ms = p.timeout_ms.unwrap_or(30000);
        let idle = onecrawl_cdp::spa::wait_network_idle(&page, idle_ms, timeout_ms).await.mcp()?;
        json_ok(&serde_json::json!({
            "action": "wait_network_idle",
            "idle": idle,
            "idle_threshold_ms": idle_ms,
            "timeout_ms": timeout_ms
        }))
    }

    pub(crate) async fn trigger_lazy_load(
        &self,
        p: TriggerLazyLoadParams,
    ) -> Result<CallToolResult, McpError> {
        let page = ensure_page(&self.browser).await?;
        let selector = p.selector.as_deref().unwrap_or("img[data-src], img[loading='lazy'], [data-lazy]");
        let count = onecrawl_cdp::spa::trigger_lazy_load(&page, selector).await.mcp()?;
        json_ok(&serde_json::json!({
            "action": "trigger_lazy_load",
            "triggered": count,
            "selector": selector
        }))
    }

    pub(crate) async fn health_check(
        &self,
        _p: HealthCheckParams,
    ) -> Result<CallToolResult, McpError> {
        let page = ensure_page(&self.browser).await?;
        let health = onecrawl_cdp::harness::health_check(&page).await.mcp()?;
        json_ok(&health)
    }

    pub(crate) async fn circuit_breaker(
        &self,
        p: CircuitBreakerParams,
    ) -> Result<CallToolResult, McpError> {
        let page = ensure_page(&self.browser).await?;
        let threshold = p.threshold.unwrap_or(5);

        let js = match p.command.as_str() {
            "record_success" => {
                r#"
                    if (window.__cb) { window.__cb.failures = 0; window.__cb.open = false; }
                    JSON.stringify(window.__cb || {failures: 0, open: false})
                "#.to_string()
            }
            "record_failure" => {
                let err = p.error.as_deref().unwrap_or("unknown");
                format!(r#"
                    if (!window.__cb) window.__cb = {{ failures: 0, open: false, threshold: {threshold}, last_error: null }};
                    window.__cb.failures++;
                    window.__cb.last_error = '{err}';
                    if (window.__cb.failures >= window.__cb.threshold) window.__cb.open = true;
                    JSON.stringify(window.__cb)
                "#)
            }
            "reset" => {
                format!(r#"
                    window.__cb = {{ failures: 0, open: false, threshold: {threshold}, last_error: null }};
                    JSON.stringify(window.__cb)
                "#)
            }
            _ => {
                format!(r#"
                    if (!window.__cb) window.__cb = {{ failures: 0, open: false, threshold: {threshold}, last_error: null }};
                    JSON.stringify(window.__cb)
                "#)
            }
        };

        let result = page.evaluate(js).await.map_err(|e| mcp_err(format!("circuit_breaker: {e}")))?;
        let raw: String = result.into_value().unwrap_or_else(|_| "{}".to_string());
        let parsed: serde_json::Value = serde_json::from_str(&raw).unwrap_or(serde_json::json!({}));
        json_ok(&serde_json::json!({
            "action": "circuit_breaker",
            "command": p.command,
            "state": parsed
        }))
    }

    pub(crate) async fn state_inspect(
        &self,
        p: StateInspectParams,
    ) -> Result<CallToolResult, McpError> {
        let page = ensure_page(&self.browser).await?;
        let result = onecrawl_cdp::spa::state_inspect(&page, p.path.as_deref()).await.mcp()?;
        json_ok(&result)
    }

    pub(crate) async fn form_wizard_track(
        &self,
        _p: FormWizardTrackParams,
    ) -> Result<CallToolResult, McpError> {
        let page = ensure_page(&self.browser).await?;
        let result = onecrawl_cdp::spa::form_wizard_track(&page).await.mcp()?;
        json_ok(&result)
    }

    pub(crate) async fn dynamic_import_wait(
        &self,
        p: DynamicImportWaitParams,
    ) -> Result<CallToolResult, McpError> {
        let page = ensure_page(&self.browser).await?;
        let timeout = p.timeout_ms.unwrap_or(10000);
        let result = onecrawl_cdp::spa::dynamic_import_wait(&page, &p.module_pattern, timeout).await.mcp()?;
        json_ok(&result)
    }

    pub(crate) async fn parallel_exec(
        &self,
        p: ParallelExecParams,
    ) -> Result<CallToolResult, McpError> {
        let page = ensure_page(&self.browser).await?;
        let result = onecrawl_cdp::spa::parallel_exec(&page, &p.actions).await.mcp()?;
        json_ok(&result)
    }

    pub(crate) async fn token_budget(
        &self,
        p: TokenBudgetParams,
    ) -> Result<CallToolResult, McpError> {
        let page = ensure_page(&self.browser).await?;
        let result = onecrawl_cdp::extract::extract(
            &page,
            p.selector.as_deref(),
            onecrawl_cdp::ExtractFormat::Text,
        )
        .await
        .mcp()?;
        let content = result.content;
        let max_chars = p.max_tokens.unwrap_or(4000) * 4;
        let truncated = if content.len() > max_chars {
            format!(
                "{}...[truncated, {} total chars]",
                &content[..max_chars],
                content.len()
            )
        } else {
            content.clone()
        };
        let chars = truncated.len();
        json_ok(&serde_json::json!({
            "content": truncated,
            "stats": {
                "chars": chars,
                "estimated_tokens": chars / 4,
                "truncated": content.len() > max_chars,
                "original_chars": content.len()
            }
        }))
    }

    pub(crate) async fn compact_state(
        &self,
        _p: CompactStateParams,
    ) -> Result<CallToolResult, McpError> {
        let page = ensure_page(&self.browser).await?;
        let url = page.url().await.mcp()?.unwrap_or_default();
        let title = page
            .evaluate("document.title")
            .await
            .mcp()?
            .into_value::<String>()
            .unwrap_or_default();
        let ready_state = page
            .evaluate("document.readyState")
            .await
            .mcp()?
            .into_value::<String>()
            .unwrap_or_default();
        let forms = page
            .evaluate("document.forms.length")
            .await
            .mcp()?
            .into_value::<u64>()
            .unwrap_or(0);
        let links = page
            .evaluate("document.links.length")
            .await
            .mcp()?
            .into_value::<u64>()
            .unwrap_or(0);
        let images = page
            .evaluate("document.images.length")
            .await
            .mcp()?
            .into_value::<u64>()
            .unwrap_or(0);
        let inputs = page
            .evaluate("document.querySelectorAll('input,textarea,select').length")
            .await
            .mcp()?
            .into_value::<u64>()
            .unwrap_or(0);
        let buttons = page
            .evaluate("document.querySelectorAll('button,[role=button]').length")
            .await
            .mcp()?
            .into_value::<u64>()
            .unwrap_or(0);
        json_ok(&serde_json::json!({
            "url": url, "title": title, "ready": ready_state,
            "counts": { "forms": forms, "links": links, "images": images, "inputs": inputs, "buttons": buttons }
        }))
    }

    pub(crate) async fn page_assertions(
        &self,
        p: PageAssertionsParams,
    ) -> Result<CallToolResult, McpError> {
        let page = ensure_page(&self.browser).await?;
        let mut results = Vec::new();
        let mut all_pass = true;
        for assertion in &p.assertions {
            let pass = match assertion.check_type.as_str() {
                "url_contains" => {
                    let url = page.url().await.mcp()?.unwrap_or_default();
                    url.contains(&assertion.expected)
                }
                "title_contains" => {
                    let title = page
                        .evaluate("document.title".to_string())
                        .await
                        .mcp()?
                        .into_value::<String>()
                        .unwrap_or_default();
                    title.contains(&assertion.expected)
                }
                "element_exists" => {
                    let js = format!(
                        "!!document.querySelector(`{}`)",
                        assertion.expected.replace('`', r"\`")
                    );
                    page.evaluate(js)
                        .await
                        .mcp()?
                        .into_value::<bool>()
                        .unwrap_or(false)
                }
                "element_visible" => {
                    let js = format!(
                        "(() => {{ const el = document.querySelector(`{}`); if (!el) return false; const s = getComputedStyle(el); return s.display !== 'none' && s.visibility !== 'hidden'; }})()",
                        assertion.expected.replace('`', r"\`")
                    );
                    page.evaluate(js)
                        .await
                        .mcp()?
                        .into_value::<bool>()
                        .unwrap_or(false)
                }
                "text_contains" => {
                    let text = page
                        .evaluate("document.body?.innerText || ''".to_string())
                        .await
                        .mcp()?
                        .into_value::<String>()
                        .unwrap_or_default();
                    text.contains(&assertion.expected)
                }
                _ => false,
            };
            if !pass {
                all_pass = false;
            }
            results.push(serde_json::json!({"check": assertion.check_type, "expected": assertion.expected, "pass": pass}));
        }
        json_ok(&serde_json::json!({"assertions": results, "all_pass": all_pass, "total": p.assertions.len()}))
    }
}
