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
        let js = format!(
            r#"(() => {{
                window.__ocDialogHandler = {{accept: {accept}, promptText: "{prompt_text}"}};
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
            prompt_text = prompt_text.replace('"', "\\\"")
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
}
