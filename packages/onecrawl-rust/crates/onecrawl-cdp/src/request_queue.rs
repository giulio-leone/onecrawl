//! Queued request execution with retry logic.

use chromiumoxide::Page;
use onecrawl_core::{Error, Result};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueuedRequest {
    pub id: String,
    pub url: String,
    pub method: String,
    pub headers: Option<std::collections::HashMap<String, String>>,
    pub body: Option<String>,
    pub max_retries: u32,
    pub retry_delay_ms: u64,
    pub timeout_ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RequestResult {
    pub id: String,
    pub url: String,
    pub status: u16,
    pub ok: bool,
    pub body: String,
    pub headers: std::collections::HashMap<String, String>,
    pub attempts: u32,
    pub duration_ms: f64,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueueConfig {
    pub concurrency: usize,
    pub delay_between_ms: u64,
    pub default_timeout_ms: u64,
    pub default_max_retries: u32,
    pub default_retry_delay_ms: u64,
}

impl Default for QueueConfig {
    fn default() -> Self {
        Self {
            concurrency: 3,
            delay_between_ms: 100,
            default_timeout_ms: 30000,
            default_max_retries: 3,
            default_retry_delay_ms: 1000,
        }
    }
}

/// Execute a single request with retry logic (via page's JS fetch).
pub async fn execute_request(page: &Page, request: &QueuedRequest) -> Result<RequestResult> {
    let req_json = serde_json::to_string(request)
        .map_err(|e| Error::Cdp(format!("serialize request failed: {e}")))?;
    let js = format!(
        r#"
        (async () => {{
            const req = {req_json};
            let lastError = null;
            let attempts = 0;
            const startTime = Date.now();

            for (let i = 0; i <= req.max_retries; i++) {{
                attempts++;
                try {{
                    const controller = new AbortController();
                    const timeout = setTimeout(() => controller.abort(), req.timeout_ms);

                    const options = {{
                        method: req.method,
                        signal: controller.signal,
                        headers: req.headers || {{}}
                    }};
                    if (req.body && req.method !== 'GET') {{
                        options.body = req.body;
                    }}

                    const resp = await fetch(req.url, options);
                    clearTimeout(timeout);

                    const body = await resp.text();
                    const headers = {{}};
                    resp.headers.forEach((v, k) => {{ headers[k] = v; }});

                    return {{
                        id: req.id,
                        url: req.url,
                        status: resp.status,
                        ok: resp.ok,
                        body: body,
                        headers: headers,
                        attempts: attempts,
                        duration_ms: Date.now() - startTime,
                        error: null
                    }};
                }} catch(e) {{
                    lastError = e.message;
                    if (i < req.max_retries) {{
                        await new Promise(r => setTimeout(r, req.retry_delay_ms));
                    }}
                }}
            }}

            return {{
                id: req.id,
                url: req.url,
                status: 0,
                ok: false,
                body: '',
                headers: {{}},
                attempts: attempts,
                duration_ms: Date.now() - startTime,
                error: lastError
            }};
        }})()
    "#
    );
    let val = page
        .evaluate(js)
        .await
        .map_err(|e| Error::Cdp(format!("execute_request failed: {e}")))?;
    let result: RequestResult =
        serde_json::from_value(val.into_value().unwrap_or(serde_json::json!({})))
            .map_err(|e| Error::Cdp(format!("parse request result failed: {e}")))?;
    Ok(result)
}

/// Execute a batch of requests with concurrency control.
pub async fn execute_batch(
    page: &Page,
    requests: &[QueuedRequest],
    config: &QueueConfig,
) -> Result<Vec<RequestResult>> {
    let reqs_json = serde_json::to_string(requests)
        .map_err(|e| Error::Cdp(format!("serialize batch failed: {e}")))?;
    let concurrency = config.concurrency;
    let delay = config.delay_between_ms;

    let js = format!(
        r#"
        (async () => {{
            const reqs = {reqs_json};
            const concurrency = {concurrency};
            const delay = {delay};
            const results = [];

            async function executeOne(req) {{
                let lastError = null;
                let attempts = 0;
                const startTime = Date.now();

                for (let i = 0; i <= req.max_retries; i++) {{
                    attempts++;
                    try {{
                        const controller = new AbortController();
                        const timeout = setTimeout(() => controller.abort(), req.timeout_ms);

                        const options = {{
                            method: req.method,
                            signal: controller.signal,
                            headers: req.headers || {{}}
                        }};
                        if (req.body && req.method !== 'GET') options.body = req.body;

                        const resp = await fetch(req.url, options);
                        clearTimeout(timeout);

                        const body = await resp.text();
                        const headers = {{}};
                        resp.headers.forEach((v, k) => {{ headers[k] = v; }});

                        return {{
                            id: req.id, url: req.url, status: resp.status,
                            ok: resp.ok, body, headers, attempts,
                            duration_ms: Date.now() - startTime, error: null
                        }};
                    }} catch(e) {{
                        lastError = e.message;
                        if (i < req.max_retries) {{
                            await new Promise(r => setTimeout(r, req.retry_delay_ms));
                        }}
                    }}
                }}

                return {{
                    id: req.id, url: req.url, status: 0, ok: false,
                    body: '', headers: {{}}, attempts,
                    duration_ms: Date.now() - startTime, error: lastError
                }};
            }}

            // Process with concurrency limit
            let active = 0;
            let index = 0;

            await new Promise((resolve) => {{
                function next() {{
                    if (index >= reqs.length && active === 0) {{
                        resolve();
                        return;
                    }}
                    while (active < concurrency && index < reqs.length) {{
                        active++;
                        const req = reqs[index++];
                        executeOne(req).then(result => {{
                            results.push(result);
                            active--;
                            if (delay > 0) {{
                                setTimeout(next, delay);
                            }} else {{
                                next();
                            }}
                        }});
                    }}
                }}
                next();
            }});

            return results;
        }})()
    "#
    );
    let val = page
        .evaluate(js)
        .await
        .map_err(|e| Error::Cdp(format!("execute_batch failed: {e}")))?;
    let results: Vec<RequestResult> =
        serde_json::from_value(val.into_value().unwrap_or(serde_json::json!([])))
            .map_err(|e| Error::Cdp(format!("parse batch results failed: {e}")))?;
    Ok(results)
}

/// Create a simple GET request.
pub fn get_request(id: &str, url: &str) -> QueuedRequest {
    QueuedRequest {
        id: id.to_string(),
        url: url.to_string(),
        method: "GET".to_string(),
        headers: None,
        body: None,
        max_retries: 3,
        retry_delay_ms: 1000,
        timeout_ms: 30000,
    }
}

/// Create a POST request.
pub fn post_request(id: &str, url: &str, body: &str) -> QueuedRequest {
    QueuedRequest {
        id: id.to_string(),
        url: url.to_string(),
        method: "POST".to_string(),
        headers: Some({
            let mut h = std::collections::HashMap::new();
            h.insert("Content-Type".to_string(), "application/json".to_string());
            h
        }),
        body: Some(body.to_string()),
        max_retries: 3,
        retry_delay_ms: 1000,
        timeout_ms: 30000,
    }
}
