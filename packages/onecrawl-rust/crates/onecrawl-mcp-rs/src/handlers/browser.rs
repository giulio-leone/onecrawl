//! Handler implementations for the `browser` super-tool.

use base64::{Engine as _, engine::general_purpose::STANDARD as B64};
use rmcp::{ErrorData as McpError, model::*};
use crate::cdp_tools::*;
use crate::helpers::{mcp_err, ensure_page, json_ok, text_ok, parse_json_str, parse_opt_json_str, McpResult};
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
        page.evaluate(format!("document.cookie = '{cookie_str}'")).await.mcp()?;
        text_ok(format!("set cookie: {}={}", p.name, p.value))
    }

    pub(crate) async fn cookies_clear(
        &self,
        p: CookiesClearParams,
    ) -> Result<CallToolResult, McpError> {
        let page = ensure_page(&self.browser).await?;
        let domain_filter = p.domain.as_deref().unwrap_or("");
        let js = format!(
            r#"(() => {{
                const cookies = document.cookie.split('; ');
                let cleared = 0;
                for (const c of cookies) {{
                    const name = c.split('=')[0];
                    if (name) {{
                        document.cookie = name + '=;expires=Thu, 01 Jan 1970 00:00:00 GMT;path=/;domain={domain_filter}';
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
        let js = format!("{storage}.getItem('{}')", p.key);
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
        let js = format!("{storage}.setItem('{}', {})", p.key, value_json);
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
                page.evaluate(format!("document.cookie = '{name}={value};path=/'")).await.mcp()?;
            }
            restored.push(format!("{} cookies", cookies.len()));
        }
        if let Some(local) = state.get("localStorage").and_then(|v| v.as_object()) {
            for (k, v) in local {
                let val = v.as_str().unwrap_or("");
                let val_json = serde_json::to_string(val).mcp()?;
                page.evaluate(format!("localStorage.setItem('{}', {})", k, val_json)).await.mcp()?;
            }
            restored.push(format!("{} localStorage items", local.len()));
        }
        if let Some(session) = state.get("sessionStorage").and_then(|v| v.as_object()) {
            for (k, v) in session {
                let val = v.as_str().unwrap_or("");
                let val_json = serde_json::to_string(val).mcp()?;
                page.evaluate(format!("sessionStorage.setItem('{}', {})", k, val_json)).await.mcp()?;
            }
            restored.push(format!("{} sessionStorage items", session.len()));
        }
        text_ok(format!("imported: {}", restored.join(", ")))
    }
}
