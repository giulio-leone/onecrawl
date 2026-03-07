//! Network request/response logging via JS PerformanceObserver + fetch/XHR intercept.
//!
//! Captures all network requests with timing, headers, status, and size info
//! into `window.__onecrawl_network_entries`.

use onecrawl_browser::Page;
use onecrawl_core::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// A captured network request/response entry.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkEntry {
    pub url: String,
    pub method: String,
    pub status: u16,
    pub status_text: String,
    pub resource_type: String,
    pub mime_type: String,
    pub request_headers: HashMap<String, String>,
    pub response_headers: HashMap<String, String>,
    pub response_size: u64,
    pub duration_ms: f64,
    pub timestamp: f64,
    pub initiator: String,
    pub is_from_cache: bool,
}

/// Summary statistics for captured network entries.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkSummary {
    pub total_requests: usize,
    pub total_size_bytes: u64,
    pub by_type: HashMap<String, usize>,
    pub by_status: HashMap<String, usize>,
    pub errors: Vec<String>,
    pub slowest: Vec<String>,
}

/// Start logging network requests via PerformanceObserver + fetch/XHR monkey-patch.
pub async fn start_network_log(page: &Page) -> Result<()> {
    let js = r#"
        (() => {
            if (window.__onecrawl_netlog_active) return 'already';
            window.__onecrawl_netlog_active = true;
            window.__onecrawl_network_entries = [];

            // ── PerformanceObserver for resource timing ─────────────
            try {
                window.__onecrawl_perf_observer = new PerformanceObserver((list) => {
                    for (const entry of list.getEntries()) {
                        if (entry.entryType === 'resource') {
                            const existing = window.__onecrawl_network_entries.find(
                                e => e.url === entry.name && !e._perf_merged
                            );
                            if (existing) {
                                existing.duration_ms = entry.duration;
                                existing.response_size = entry.transferSize || existing.response_size;
                                existing.is_from_cache = entry.transferSize === 0 && entry.decodedBodySize > 0;
                                existing.resource_type = existing.resource_type || entry.initiatorType || '';
                                existing._perf_merged = true;
                            }
                        }
                    }
                });
                window.__onecrawl_perf_observer.observe({ type: 'resource', buffered: true });
            } catch(_) {}

            // ── Fetch monkey-patch ──────────────────────────────────
            const origFetch = window.fetch.bind(window);
            window.__onecrawl_orig_fetch = origFetch;
            window.fetch = async function(...args) {
                const req = new Request(...args);
                const method = req.method || 'GET';
                const url = req.url;
                const reqHeaders = {};
                req.headers.forEach((v, k) => { reqHeaders[k] = v; });
                const start = performance.now();
                const ts = Date.now();
                try {
                    const resp = await origFetch(...args);
                    const respHeaders = {};
                    resp.headers.forEach((v, k) => { respHeaders[k] = v; });
                    const clone = resp.clone();
                    let size = 0;
                    try {
                        const buf = await clone.arrayBuffer();
                        size = buf.byteLength;
                    } catch(_) {}
                    window.__onecrawl_network_entries.push({
                        url,
                        method,
                        status: resp.status,
                        status_text: resp.statusText || '',
                        resource_type: 'fetch',
                        mime_type: resp.headers.get('content-type') || '',
                        request_headers: reqHeaders,
                        response_headers: respHeaders,
                        response_size: size,
                        duration_ms: performance.now() - start,
                        timestamp: ts,
                        initiator: 'fetch',
                        is_from_cache: false
                    });
                    return resp;
                } catch(err) {
                    window.__onecrawl_network_entries.push({
                        url,
                        method,
                        status: 0,
                        status_text: err.message || 'fetch error',
                        resource_type: 'fetch',
                        mime_type: '',
                        request_headers: reqHeaders,
                        response_headers: {},
                        response_size: 0,
                        duration_ms: performance.now() - start,
                        timestamp: ts,
                        initiator: 'fetch',
                        is_from_cache: false
                    });
                    throw err;
                }
            };

            // ── XHR monkey-patch ────────────────────────────────────
            const OrigXHR = window.XMLHttpRequest;
            window.__onecrawl_orig_xhr = OrigXHR;
            window.XMLHttpRequest = function() {
                const xhr = new OrigXHR();
                let method = 'GET';
                let url = '';
                let reqHeaders = {};
                let start = 0;
                let ts = 0;
                const origOpen = xhr.open.bind(xhr);
                xhr.open = function(m, u, ...rest) {
                    method = m || 'GET';
                    url = String(u);
                    start = performance.now();
                    ts = Date.now();
                    return origOpen(m, u, ...rest);
                };
                const origSetHeader = xhr.setRequestHeader.bind(xhr);
                xhr.setRequestHeader = function(k, v) {
                    reqHeaders[k] = v;
                    return origSetHeader(k, v);
                };
                xhr.addEventListener('loadend', function() {
                    const respHeaders = {};
                    try {
                        const raw = xhr.getAllResponseHeaders() || '';
                        raw.split('\r\n').forEach(line => {
                            const idx = line.indexOf(':');
                            if (idx > 0) respHeaders[line.slice(0,idx).trim()] = line.slice(idx+1).trim();
                        });
                    } catch(_) {}
                    window.__onecrawl_network_entries.push({
                        url,
                        method,
                        status: xhr.status || 0,
                        status_text: xhr.statusText || '',
                        resource_type: 'xhr',
                        mime_type: xhr.getResponseHeader('content-type') || '',
                        request_headers: reqHeaders,
                        response_headers: respHeaders,
                        response_size: xhr.response ? (xhr.response.byteLength || xhr.responseText?.length || 0) : 0,
                        duration_ms: performance.now() - start,
                        timestamp: ts,
                        initiator: 'xmlhttprequest',
                        is_from_cache: false
                    });
                });
                return xhr;
            };

            return 'installed';
        })()
    "#;

    page.evaluate(js)
        .await
        .map_err(|e| onecrawl_core::Error::Cdp(format!("start_network_log failed: {e}")))?;

    Ok(())
}

/// Drain all logged network entries from the page and return them.
pub async fn drain_network_log(page: &Page) -> Result<Vec<NetworkEntry>> {
    let result = page
        .evaluate(
            r#"
            (() => {
                const entries = (window.__onecrawl_network_entries || []).map(e => {
                    const copy = Object.assign({}, e);
                    delete copy._perf_merged;
                    return copy;
                });
                window.__onecrawl_network_entries = [];
                return entries;
            })()
            "#,
        )
        .await
        .map_err(|e| onecrawl_core::Error::Cdp(format!("drain_network_log failed: {e}")))?;

    let entries: Vec<NetworkEntry> = result.into_value().unwrap_or_default();
    Ok(entries)
}

/// Get summary statistics for current network entries (without draining them).
pub async fn get_network_summary(page: &Page) -> Result<NetworkSummary> {
    let result = page
        .evaluate(
            r#"
            (() => {
                const entries = window.__onecrawl_network_entries || [];
                const byType = {};
                const byStatus = {};
                const errors = [];
                let totalSize = 0;

                entries.forEach(e => {
                    const t = e.resource_type || 'unknown';
                    byType[t] = (byType[t] || 0) + 1;
                    const s = String(e.status);
                    byStatus[s] = (byStatus[s] || 0) + 1;
                    totalSize += (e.response_size || 0);
                    if (e.status === 0 || e.status >= 400) {
                        errors.push(e.method + ' ' + e.url + ' → ' + e.status + ' ' + e.status_text);
                    }
                });

                const sorted = entries.slice().sort((a,b) => b.duration_ms - a.duration_ms);
                const slowest = sorted.slice(0, 5).map(e =>
                    e.method + ' ' + e.url + ' (' + e.duration_ms.toFixed(1) + 'ms)'
                );

                return {
                    total_requests: entries.length,
                    total_size_bytes: totalSize,
                    by_type: byType,
                    by_status: byStatus,
                    errors: errors,
                    slowest: slowest
                };
            })()
            "#,
        )
        .await
        .map_err(|e| onecrawl_core::Error::Cdp(format!("get_network_summary failed: {e}")))?;

    let summary: NetworkSummary = result.into_value().unwrap_or(NetworkSummary {
        total_requests: 0,
        total_size_bytes: 0,
        by_type: HashMap::new(),
        by_status: HashMap::new(),
        errors: Vec::new(),
        slowest: Vec::new(),
    });

    Ok(summary)
}

/// Stop logging and restore original fetch/XHR.
pub async fn stop_network_log(page: &Page) -> Result<()> {
    page.evaluate(
        r#"
        (() => {
            if (window.__onecrawl_perf_observer) {
                window.__onecrawl_perf_observer.disconnect();
                window.__onecrawl_perf_observer = null;
            }
            if (window.__onecrawl_orig_fetch) {
                window.fetch = window.__onecrawl_orig_fetch;
                delete window.__onecrawl_orig_fetch;
            }
            if (window.__onecrawl_orig_xhr) {
                window.XMLHttpRequest = window.__onecrawl_orig_xhr;
                delete window.__onecrawl_orig_xhr;
            }
            window.__onecrawl_netlog_active = false;
        })()
        "#,
    )
    .await
    .map_err(|e| onecrawl_core::Error::Cdp(format!("stop_network_log failed: {e}")))?;

    Ok(())
}

/// Export all logged network entries to a JSON file at the given path.
pub async fn export_network_log(page: &Page, path: &str) -> Result<()> {
    let entries = drain_network_log(page).await?;
    let json = serde_json::to_string_pretty(&entries)
        .map_err(|e| onecrawl_core::Error::Cdp(format!("serialize network log: {e}")))?;
    std::fs::write(path, json)?;
    Ok(())
}
