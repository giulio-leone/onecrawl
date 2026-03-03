//! HTTP client that uses the browser's `fetch` API.
//!
//! Inherits cookies, headers, and session from the browser context.

use chromiumoxide::Page;
use onecrawl_core::{Error, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HttpRequest {
    pub url: String,
    pub method: String,
    pub headers: HashMap<String, String>,
    pub body: Option<String>,
    pub timeout_ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HttpResponse {
    pub status: u16,
    pub status_text: String,
    pub headers: HashMap<String, String>,
    pub body: String,
    pub url: String,
    pub redirected: bool,
    pub duration_ms: f64,
}

fn build_fetch_js(request: &HttpRequest) -> String {
    let headers_json = serde_json::to_string(&request.headers).unwrap_or_else(|_| "{}".to_string());
    let body_part = match &request.body {
        Some(b) => format!(
            ", body: {}",
            serde_json::to_string(b).unwrap_or_else(|_| "null".to_string())
        ),
        None => String::new(),
    };

    format!(
        r#"(async () => {{
            const start = performance.now();
            const controller = new AbortController();
            const timer = setTimeout(() => controller.abort(), {timeout});
            try {{
                const resp = await fetch('{url}', {{
                    method: '{method}',
                    headers: {headers},
                    signal: controller.signal{body}
                }});
                clearTimeout(timer);
                const elapsed = performance.now() - start;
                const text = await resp.text();
                const hdrs = {{}};
                resp.headers.forEach((v, k) => {{ hdrs[k] = v; }});
                return {{
                    status: resp.status,
                    status_text: resp.statusText,
                    headers: hdrs,
                    body: text,
                    url: resp.url,
                    redirected: resp.redirected,
                    duration_ms: elapsed
                }};
            }} catch (err) {{
                clearTimeout(timer);
                return {{ error: err.message || String(err) }};
            }}
        }})()"#,
        url = request.url.replace('\'', "\\'"),
        method = request.method.replace('\'', "\\'"),
        headers = headers_json,
        timeout = request.timeout_ms,
        body = body_part,
    )
}

fn parse_response(raw: serde_json::Value) -> Result<HttpResponse> {
    if let Some(err) = raw.get("error").and_then(|v| v.as_str()) {
        return Err(Error::Browser(format!("fetch failed: {err}")));
    }
    Ok(HttpResponse {
        status: raw.get("status").and_then(|v| v.as_u64()).unwrap_or(0) as u16,
        status_text: raw
            .get("status_text")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string(),
        headers: serde_json::from_value(
            raw.get("headers").cloned().unwrap_or(serde_json::json!({})),
        )
        .unwrap_or_default(),
        body: raw
            .get("body")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string(),
        url: raw
            .get("url")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string(),
        redirected: raw
            .get("redirected")
            .and_then(|v| v.as_bool())
            .unwrap_or(false),
        duration_ms: raw
            .get("duration_ms")
            .and_then(|v| v.as_f64())
            .unwrap_or(0.0),
    })
}

/// Execute an HTTP request via the browser's fetch API.
pub async fn fetch(page: &Page, request: &HttpRequest) -> Result<HttpResponse> {
    let js = build_fetch_js(request);
    let val = page
        .evaluate(js)
        .await
        .map_err(|e| Error::Browser(e.to_string()))?;
    let raw = val.into_value().unwrap_or(serde_json::json!({}));
    parse_response(raw)
}

/// Convenience GET request.
pub async fn get(
    page: &Page,
    url: &str,
    headers: Option<HashMap<String, String>>,
) -> Result<HttpResponse> {
    let request = HttpRequest {
        url: url.to_string(),
        method: "GET".to_string(),
        headers: headers.unwrap_or_default(),
        body: None,
        timeout_ms: 30000,
    };
    fetch(page, &request).await
}

/// Convenience POST request.
pub async fn post(
    page: &Page,
    url: &str,
    body: &str,
    content_type: &str,
    headers: Option<HashMap<String, String>>,
) -> Result<HttpResponse> {
    let mut hdrs = headers.unwrap_or_default();
    hdrs.insert("Content-Type".to_string(), content_type.to_string());
    let request = HttpRequest {
        url: url.to_string(),
        method: "POST".to_string(),
        headers: hdrs,
        body: Some(body.to_string()),
        timeout_ms: 30000,
    };
    fetch(page, &request).await
}

/// HEAD request.
pub async fn head(page: &Page, url: &str) -> Result<HttpResponse> {
    let request = HttpRequest {
        url: url.to_string(),
        method: "HEAD".to_string(),
        headers: HashMap::new(),
        body: None,
        timeout_ms: 30000,
    };
    fetch(page, &request).await
}

/// GET and parse the response body as JSON.
pub async fn fetch_json(page: &Page, url: &str) -> Result<serde_json::Value> {
    let mut headers = HashMap::new();
    headers.insert("Accept".to_string(), "application/json".to_string());
    let resp = get(page, url, Some(headers)).await?;
    if resp.status >= 400 {
        return Err(Error::Browser(format!(
            "HTTP {} {} for {}",
            resp.status, resp.status_text, url
        )));
    }
    let val: serde_json::Value = serde_json::from_str(&resp.body)?;
    Ok(val)
}
