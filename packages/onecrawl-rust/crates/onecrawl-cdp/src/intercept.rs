//! Request interception and response mocking via JS monkey-patching.

use chromiumoxide::Page;
use onecrawl_core::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// A rule describing how to intercept a matching request.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InterceptRule {
    /// Glob-style URL pattern, e.g. "*api/v1/*"
    pub url_pattern: String,
    /// Optional resource type filter: "Document", "Script", "Image", etc.
    pub resource_type: Option<String>,
    /// Action to take when a request matches.
    pub action: InterceptAction,
}

/// What to do with an intercepted request.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum InterceptAction {
    /// Block the request entirely.
    Block,
    /// Modify outgoing headers.
    Modify {
        headers: Option<HashMap<String, String>>,
    },
    /// Return a fake response without hitting the network.
    MockResponse {
        status: u16,
        body: String,
        headers: Option<HashMap<String, String>>,
    },
}

/// Register interception rules by monkey-patching `window.fetch` and `XMLHttpRequest`.
pub async fn set_intercept_rules(page: &Page, rules: Vec<InterceptRule>) -> Result<()> {
    let rules_json =
        serde_json::to_string(&rules).map_err(|e| onecrawl_core::Error::Browser(e.to_string()))?;

    let js = format!(
        r#"(() => {{
    // Store originals if not already saved
    if (!window.__onecrawl_orig_fetch) {{
        window.__onecrawl_orig_fetch = window.fetch.bind(window);
    }}
    if (!window.__onecrawl_orig_xhr_open) {{
        window.__onecrawl_orig_xhr_open = XMLHttpRequest.prototype.open;
        window.__onecrawl_orig_xhr_send = XMLHttpRequest.prototype.send;
    }}

    window.__onecrawl_intercept_rules = {rules_json};
    window.__onecrawl_intercepted_log = window.__onecrawl_intercepted_log || [];

    function matchPattern(url, pattern) {{
        const regex = new RegExp('^' + pattern.replace(/\*/g, '.*') + '$');
        return regex.test(url);
    }}

    function findRule(url) {{
        const rules = window.__onecrawl_intercept_rules || [];
        for (const rule of rules) {{
            if (matchPattern(url, rule.url_pattern)) return rule;
        }}
        return null;
    }}

    // Override fetch
    window.fetch = function(input, init) {{
        const url = (typeof input === 'string') ? input : (input.url || '');
        const rule = findRule(url);
        if (rule) {{
            window.__onecrawl_intercepted_log.push({{ url, type: 'fetch', action: rule.action, ts: Date.now() }});
            if (rule.action === 'Block' || (rule.action && rule.action.Block !== undefined)) {{
                return Promise.reject(new Error('Blocked by OneCrawl intercept'));
            }}
            if (rule.action.MockResponse) {{
                const mock = rule.action.MockResponse;
                const headers = new Headers(mock.headers || {{}});
                return Promise.resolve(new Response(mock.body, {{ status: mock.status, headers }}));
            }}
            if (rule.action.Modify && rule.action.Modify.headers) {{
                init = init || {{}};
                init.headers = Object.assign({{}}, init.headers || {{}}, rule.action.Modify.headers);
            }}
        }}
        return window.__onecrawl_orig_fetch(input, init);
    }};

    // Override XHR
    XMLHttpRequest.prototype.open = function(method, url, ...rest) {{
        this.__onecrawl_url = url;
        this.__onecrawl_rule = findRule(url);
        return window.__onecrawl_orig_xhr_open.call(this, method, url, ...rest);
    }};

    XMLHttpRequest.prototype.send = function(body) {{
        const rule = this.__onecrawl_rule;
        if (rule) {{
            window.__onecrawl_intercepted_log.push({{ url: this.__onecrawl_url, type: 'xhr', action: rule.action, ts: Date.now() }});
            if (rule.action === 'Block' || (rule.action && rule.action.Block !== undefined)) {{
                this.dispatchEvent(new Event('error'));
                return;
            }}
            if (rule.action.MockResponse) {{
                const mock = rule.action.MockResponse;
                Object.defineProperty(this, 'status', {{ value: mock.status, writable: false }});
                Object.defineProperty(this, 'responseText', {{ value: mock.body, writable: false }});
                Object.defineProperty(this, 'readyState', {{ value: 4, writable: false }});
                this.dispatchEvent(new Event('readystatechange'));
                this.dispatchEvent(new Event('load'));
                return;
            }}
        }}
        return window.__onecrawl_orig_xhr_send.call(this, body);
    }};
}})()"#
    );

    page.evaluate(js)
        .await
        .map_err(|e| onecrawl_core::Error::Browser(format!("set_intercept_rules: {e}")))?;
    Ok(())
}

/// Get the log of intercepted requests.
pub async fn get_intercepted_requests(page: &Page) -> Result<Vec<serde_json::Value>> {
    let val = page
        .evaluate("JSON.stringify(window.__onecrawl_intercepted_log || [])")
        .await
        .map_err(|e| onecrawl_core::Error::Browser(format!("get_intercepted_requests: {e}")))?;

    let raw = val
        .into_value::<String>()
        .unwrap_or_else(|_| "[]".to_string());

    let entries: Vec<serde_json::Value> = serde_json::from_str(&raw).unwrap_or_default();
    Ok(entries)
}

/// Clear all interception rules and restore original fetch/XHR.
pub async fn clear_intercept_rules(page: &Page) -> Result<()> {
    let js = r#"(() => {
        if (window.__onecrawl_orig_fetch) {
            window.fetch = window.__onecrawl_orig_fetch;
            delete window.__onecrawl_orig_fetch;
        }
        if (window.__onecrawl_orig_xhr_open) {
            XMLHttpRequest.prototype.open = window.__onecrawl_orig_xhr_open;
            XMLHttpRequest.prototype.send = window.__onecrawl_orig_xhr_send;
            delete window.__onecrawl_orig_xhr_open;
            delete window.__onecrawl_orig_xhr_send;
        }
        window.__onecrawl_intercept_rules = [];
        window.__onecrawl_intercepted_log = [];
    })()"#;

    page.evaluate(js)
        .await
        .map_err(|e| onecrawl_core::Error::Browser(format!("clear_intercept_rules: {e}")))?;
    Ok(())
}
