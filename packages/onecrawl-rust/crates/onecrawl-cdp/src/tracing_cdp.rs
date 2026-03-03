//! Performance tracing via CDP Tracing/Performance domains and JS Performance API.

use chromiumoxide::Page;
use onecrawl_core::{Error, Result};
use serde::{Deserialize, Serialize};

/// A single performance metric from the CDP Performance domain.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceMetric {
    pub name: String,
    pub value: f64,
}

/// Start performance tracing via CDP Tracing domain.
pub async fn start_tracing(page: &Page) -> Result<()> {
    use chromiumoxide::cdp::browser_protocol::tracing::StartParams;

    let params = StartParams::default();
    page.execute(params)
        .await
        .map_err(|e| Error::Browser(format!("Tracing.start failed: {e}")))?;

    Ok(())
}

/// Stop tracing and return trace data as JSON.
///
/// Note: CDP `Tracing.end` does not directly return trace data in the response;
/// data is delivered via `Tracing.dataCollected` events. This implementation
/// issues the end command and returns the response. For full trace capture,
/// consider using the JS Performance API via `get_navigation_timing` and
/// `get_resource_timing`.
pub async fn stop_tracing(page: &Page) -> Result<serde_json::Value> {
    use chromiumoxide::cdp::browser_protocol::tracing::EndParams;

    page.execute(EndParams::default())
        .await
        .map_err(|e| Error::Browser(format!("Tracing.end failed: {e}")))?;

    // Since trace data comes via events and not the response,
    // collect a performance summary via JS as a practical fallback.
    let js = r#"
        (() => {
            const entries = performance.getEntries();
            return {
                tracing_stopped: true,
                entry_count: entries.length,
                entries: entries.slice(0, 500).map(e => ({
                    name: e.name,
                    entryType: e.entryType,
                    startTime: e.startTime,
                    duration: e.duration
                }))
            };
        })()
    "#;

    let result = page
        .evaluate(js)
        .await
        .map_err(|e| Error::Browser(format!("stop_tracing JS fallback failed: {e}")))?;

    match result.into_value::<serde_json::Value>() {
        Ok(v) => Ok(v),
        Err(_) => Ok(serde_json::json!({ "tracing_stopped": true, "entries": [] })),
    }
}

/// Get performance metrics via CDP Performance domain.
pub async fn get_performance_metrics(page: &Page) -> Result<Vec<PerformanceMetric>> {
    use chromiumoxide::cdp::browser_protocol::performance::{
        EnableParams, GetMetricsParams, GetMetricsReturns,
    };

    page.execute(EnableParams::default())
        .await
        .map_err(|e| Error::Browser(format!("Performance.enable failed: {e}")))?;

    let resp = page
        .execute(GetMetricsParams::default())
        .await
        .map_err(|e| Error::Browser(format!("Performance.getMetrics failed: {e}")))?;
    let result: &GetMetricsReturns = &resp;

    let metrics = result
        .metrics
        .iter()
        .map(|m| PerformanceMetric {
            name: m.name.clone(),
            value: m.value,
        })
        .collect();

    Ok(metrics)
}

/// Get navigation timing via JS Performance API.
pub async fn get_navigation_timing(page: &Page) -> Result<serde_json::Value> {
    let js = r#"
        (() => {
            const nav = performance.getEntriesByType('navigation');
            if (nav.length === 0) return null;
            const t = nav[0];
            return {
                name: t.name,
                entryType: t.entryType,
                startTime: t.startTime,
                duration: t.duration,
                redirectStart: t.redirectStart,
                redirectEnd: t.redirectEnd,
                fetchStart: t.fetchStart,
                domainLookupStart: t.domainLookupStart,
                domainLookupEnd: t.domainLookupEnd,
                connectStart: t.connectStart,
                connectEnd: t.connectEnd,
                secureConnectionStart: t.secureConnectionStart,
                requestStart: t.requestStart,
                responseStart: t.responseStart,
                responseEnd: t.responseEnd,
                domInteractive: t.domInteractive,
                domContentLoadedEventStart: t.domContentLoadedEventStart,
                domContentLoadedEventEnd: t.domContentLoadedEventEnd,
                domComplete: t.domComplete,
                loadEventStart: t.loadEventStart,
                loadEventEnd: t.loadEventEnd,
                transferSize: t.transferSize,
                encodedBodySize: t.encodedBodySize,
                decodedBodySize: t.decodedBodySize
            };
        })()
    "#;

    let result = page
        .evaluate(js)
        .await
        .map_err(|e| Error::Browser(format!("get_navigation_timing failed: {e}")))?;

    match result.into_value::<serde_json::Value>() {
        Ok(v) => Ok(v),
        Err(_) => Ok(serde_json::Value::Null),
    }
}

/// Get resource timing entries via JS Performance API.
pub async fn get_resource_timing(page: &Page) -> Result<Vec<serde_json::Value>> {
    let js = r#"
        (() => {
            return performance.getEntriesByType('resource').map(r => ({
                name: r.name,
                entryType: r.entryType,
                startTime: r.startTime,
                duration: r.duration,
                initiatorType: r.initiatorType,
                transferSize: r.transferSize,
                encodedBodySize: r.encodedBodySize,
                decodedBodySize: r.decodedBodySize,
                responseStart: r.responseStart,
                responseEnd: r.responseEnd
            }));
        })()
    "#;

    let result = page
        .evaluate(js)
        .await
        .map_err(|e| Error::Browser(format!("get_resource_timing failed: {e}")))?;

    match result.into_value::<Vec<serde_json::Value>>() {
        Ok(v) => Ok(v),
        Err(_) => Ok(Vec::new()),
    }
}
