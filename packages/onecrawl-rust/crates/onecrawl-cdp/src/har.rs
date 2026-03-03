//! HAR (HTTP Archive) capture via CDP Network domain.
//!
//! Records network requests/responses and exports them in HAR 1.2 format.

use chromiumoxide::Page;
use onecrawl_core::{Error, Result};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::Mutex;

/// A single HAR entry (request + response pair).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HarEntry {
    pub request_id: String,
    pub url: String,
    pub method: String,
    pub status: i64,
    pub status_text: String,
    pub mime_type: String,
    pub request_headers: serde_json::Value,
    pub response_headers: serde_json::Value,
    pub request_body_size: i64,
    pub response_body_size: f64,
    pub started: f64,
    pub duration_ms: f64,
    pub resource_type: String,
    pub from_cache: bool,
    pub remote_address: String,
    pub protocol: String,
}

/// Collects HAR entries during a page session.
#[derive(Clone)]
pub struct HarRecorder {
    entries: Arc<Mutex<Vec<HarEntry>>>,
}

impl Default for HarRecorder {
    fn default() -> Self {
        Self::new()
    }
}

impl HarRecorder {
    pub fn new() -> Self {
        Self {
            entries: Arc::new(Mutex::new(Vec::new())),
        }
    }

    /// Get completed HAR entries.
    pub async fn entries(&self) -> Vec<HarEntry> {
        self.entries.lock().await.clone()
    }

    /// Clear all entries.
    pub async fn clear(&self) {
        self.entries.lock().await.clear();
    }

    /// Number of recorded entries.
    pub async fn len(&self) -> usize {
        self.entries.lock().await.len()
    }

    /// Returns true if no entries have been recorded.
    pub async fn is_empty(&self) -> bool {
        self.entries.lock().await.is_empty()
    }
}

/// Start HAR recording by injecting JS-based network observers.
pub async fn start_har_recording(page: &Page, recorder: &HarRecorder) -> Result<()> {
    // Install Performance Observer to track requests
    let js = r#"
        (() => {
            if (window.__onecrawl_har_active) return 'already';
            window.__onecrawl_har_active = true;
            window.__onecrawl_har_entries = [];

            const observer = new PerformanceObserver((list) => {
                for (const entry of list.getEntries()) {
                    if (entry.entryType === 'resource' || entry.entryType === 'navigation') {
                        window.__onecrawl_har_entries.push({
                            name: entry.name,
                            entryType: entry.entryType,
                            startTime: entry.startTime,
                            duration: entry.duration,
                            transferSize: entry.transferSize || 0,
                            encodedBodySize: entry.encodedBodySize || 0,
                            decodedBodySize: entry.decodedBodySize || 0,
                            initiatorType: entry.initiatorType || '',
                            nextHopProtocol: entry.nextHopProtocol || '',
                            responseStatus: entry.responseStatus || 0,
                            timestamp: Date.now()
                        });
                    }
                }
            });
            observer.observe({ entryTypes: ['resource', 'navigation'] });
            return 'installed';
        })()
    "#;

    page.evaluate(js)
        .await
        .map_err(|e| Error::Cdp(format!("start_har_recording failed: {e}")))?;

    // Also capture existing resource entries
    let existing_js = r#"
        (() => {
            const entries = performance.getEntriesByType('resource')
                .concat(performance.getEntriesByType('navigation'))
                .map(e => ({
                    name: e.name,
                    entryType: e.entryType,
                    startTime: e.startTime,
                    duration: e.duration,
                    transferSize: e.transferSize || 0,
                    encodedBodySize: e.encodedBodySize || 0,
                    decodedBodySize: e.decodedBodySize || 0,
                    initiatorType: e.initiatorType || '',
                    nextHopProtocol: e.nextHopProtocol || '',
                    responseStatus: e.responseStatus || 0,
                    timestamp: Date.now()
                }));
            return entries;
        })()
    "#;

    let result = page
        .evaluate(existing_js)
        .await
        .map_err(|e| Error::Cdp(format!("get existing entries: {e}")))?;

    if let Ok(entries) = result.into_value::<Vec<serde_json::Value>>() {
        let mut har_entries = recorder.entries.lock().await;
        for entry in entries {
            let har = perf_entry_to_har(&entry);
            har_entries.push(har);
        }
    }

    Ok(())
}

/// Drain new HAR entries from the page.
pub async fn drain_har_entries(page: &Page, recorder: &HarRecorder) -> Result<usize> {
    let result = page
        .evaluate(
            r#"
            (() => {
                const entries = window.__onecrawl_har_entries || [];
                window.__onecrawl_har_entries = [];
                return entries;
            })()
            "#,
        )
        .await
        .map_err(|e| Error::Cdp(format!("drain_har failed: {e}")))?;

    let entries: Vec<serde_json::Value> = match result.into_value() {
        Ok(v) => v,
        Err(_) => return Ok(0),
    };

    let count = entries.len();
    let mut har_entries = recorder.entries.lock().await;
    for entry in entries {
        har_entries.push(perf_entry_to_har(&entry));
    }

    Ok(count)
}

/// Export all recorded entries as HAR 1.2 JSON.
pub async fn export_har(recorder: &HarRecorder, page_url: &str) -> Result<serde_json::Value> {
    let entries = recorder.entries.lock().await;

    let har_entries: Vec<serde_json::Value> = entries
        .iter()
        .map(|e| {
            serde_json::json!({
                "startedDateTime": format!("{}Z", chrono_like_timestamp(e.started)),
                "time": e.duration_ms,
                "request": {
                    "method": e.method,
                    "url": e.url,
                    "httpVersion": e.protocol,
                    "headers": e.request_headers,
                    "queryString": [],
                    "headersSize": -1,
                    "bodySize": e.request_body_size,
                },
                "response": {
                    "status": e.status,
                    "statusText": e.status_text,
                    "httpVersion": e.protocol,
                    "headers": e.response_headers,
                    "content": {
                        "size": e.response_body_size,
                        "mimeType": e.mime_type,
                    },
                    "redirectURL": "",
                    "headersSize": -1,
                    "bodySize": e.response_body_size as i64,
                },
                "cache": {},
                "timings": {
                    "send": 0,
                    "wait": e.duration_ms,
                    "receive": 0,
                },
                "serverIPAddress": e.remote_address,
            })
        })
        .collect();

    Ok(serde_json::json!({
        "log": {
            "version": "1.2",
            "creator": {
                "name": "OneCrawl",
                "version": "0.1.0"
            },
            "pages": [{
                "startedDateTime": chrono_like_timestamp(0.0),
                "id": "page_1",
                "title": page_url,
                "pageTimings": {}
            }],
            "entries": har_entries
        }
    }))
}

fn perf_entry_to_har(entry: &serde_json::Value) -> HarEntry {
    HarEntry {
        request_id: String::new(),
        url: entry["name"].as_str().unwrap_or("").to_string(),
        method: "GET".to_string(),
        status: entry["responseStatus"].as_i64().unwrap_or(200),
        status_text: String::new(),
        mime_type: String::new(),
        request_headers: serde_json::Value::Array(vec![]),
        response_headers: serde_json::Value::Array(vec![]),
        request_body_size: 0,
        response_body_size: entry["decodedBodySize"].as_f64().unwrap_or(0.0),
        started: entry["timestamp"].as_f64().unwrap_or(0.0),
        duration_ms: entry["duration"].as_f64().unwrap_or(0.0),
        resource_type: entry["initiatorType"]
            .as_str()
            .unwrap_or("other")
            .to_string(),
        from_cache: entry["transferSize"].as_f64().unwrap_or(0.0) == 0.0
            && entry["decodedBodySize"].as_f64().unwrap_or(0.0) > 0.0,
        remote_address: String::new(),
        protocol: entry["nextHopProtocol"].as_str().unwrap_or("").to_string(),
    }
}

fn chrono_like_timestamp(ms: f64) -> String {
    let secs = (ms / 1000.0) as i64;
    let nanos = ((ms % 1000.0) * 1_000_000.0) as u32;
    // Simple ISO-like format without chrono dependency
    if secs == 0 && nanos == 0 {
        return "1970-01-01T00:00:00.000".to_string();
    }
    format!("{}.{:03}", secs, nanos / 1_000_000)
}
