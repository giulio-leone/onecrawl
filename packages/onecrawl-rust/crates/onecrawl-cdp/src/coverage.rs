//! JS/CSS code coverage via CDP Profiler domain.
//!
//! Tracks which parts of JavaScript and CSS are actually executed/used.

use chromiumoxide::Page;
use onecrawl_core::{Error, Result};
use serde::{Deserialize, Serialize};

/// A coverage range within a source file.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CoverageRange {
    pub start_offset: i64,
    pub end_offset: i64,
    pub count: i64,
}

/// Coverage data for a single script/file.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScriptCoverage {
    pub url: String,
    pub ranges: Vec<CoverageRange>,
    pub total_bytes: i64,
    pub used_bytes: i64,
    pub coverage_percent: f64,
}

/// Aggregated coverage report.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CoverageReport {
    pub scripts: Vec<ScriptCoverage>,
    pub total_bytes: i64,
    pub used_bytes: i64,
    pub overall_percent: f64,
}

/// Start JS code coverage collection via CDP Profiler.
pub async fn start_js_coverage(page: &Page) -> Result<()> {
    use chromiumoxide::cdp::js_protocol::profiler::{
        EnableParams, StartPreciseCoverageParams,
    };

    page.execute(EnableParams::default())
        .await
        .map_err(|e| Error::Browser(format!("Profiler.enable failed: {e}")))?;

    let params: StartPreciseCoverageParams = StartPreciseCoverageParams::builder()
        .call_count(true)
        .detailed(true)
        .build();

    page.execute(params)
        .await
        .map_err(|e| Error::Browser(format!("StartPreciseCoverage failed: {e}")))?;

    Ok(())
}

/// Stop JS coverage and return the report.
pub async fn stop_js_coverage(page: &Page) -> Result<CoverageReport> {
    use chromiumoxide::cdp::js_protocol::profiler::{
        TakePreciseCoverageParams, TakePreciseCoverageReturns,
        StopPreciseCoverageParams, DisableParams,
    };

    let resp = page
        .execute(TakePreciseCoverageParams::default())
        .await
        .map_err(|e| Error::Browser(format!("TakePreciseCoverage failed: {e}")))?;
    let result: &TakePreciseCoverageReturns = &resp;

    let mut scripts = Vec::new();
    let mut total_bytes: i64 = 0;
    let mut used_bytes: i64 = 0;

    for script in &result.result {
        let url = script.url.clone();
        if url.is_empty() || url.starts_with("internal:") {
            continue;
        }

        let mut script_ranges = Vec::new();
        let mut script_total: i64 = 0;
        let mut script_used: i64 = 0;

        for func in &script.functions {
            for range in &func.ranges {
                let size = range.end_offset - range.start_offset;
                script_total += size;
                if range.count > 0 {
                    script_used += size;
                }
                script_ranges.push(CoverageRange {
                    start_offset: range.start_offset,
                    end_offset: range.end_offset,
                    count: range.count,
                });
            }
        }

        let coverage_percent = if script_total > 0 {
            (script_used as f64 / script_total as f64) * 100.0
        } else {
            0.0
        };

        total_bytes += script_total;
        used_bytes += script_used;

        scripts.push(ScriptCoverage {
            url,
            ranges: script_ranges,
            total_bytes: script_total,
            used_bytes: script_used,
            coverage_percent,
        });
    }

    // Stop and disable profiler
    let _ = page.execute(StopPreciseCoverageParams::default()).await;
    let _ = page.execute(DisableParams::default()).await;

    let overall = if total_bytes > 0 {
        (used_bytes as f64 / total_bytes as f64) * 100.0
    } else {
        0.0
    };

    Ok(CoverageReport {
        scripts,
        total_bytes,
        used_bytes,
        overall_percent: overall,
    })
}

/// Start CSS coverage collection.
pub async fn start_css_coverage(page: &Page) -> Result<()> {
    use chromiumoxide::cdp::browser_protocol::dom::EnableParams as DomEnableParams;
    use chromiumoxide::cdp::browser_protocol::css::EnableParams;

    page.execute(DomEnableParams::default())
        .await
        .map_err(|e| Error::Browser(format!("DOM.enable failed: {e}")))?;
    page.execute(EnableParams::default())
        .await
        .map_err(|e| Error::Browser(format!("CSS.enable failed: {e}")))?;

    let js = r#"
        (() => {
            if (window.__onecrawl_css_coverage) return 'already';
            window.__onecrawl_css_coverage = true;
            window.__onecrawl_css_used = new Set();

            // Track used CSS rules via getComputedStyle sampling
            const observer = new MutationObserver(() => {
                document.querySelectorAll('*').forEach(el => {
                    const styles = getComputedStyle(el);
                    for (let i = 0; i < styles.length; i++) {
                        window.__onecrawl_css_used.add(styles[i]);
                    }
                });
            });
            observer.observe(document.documentElement, { childList: true, subtree: true });

            // Initial scan
            document.querySelectorAll('*').forEach(el => {
                const styles = getComputedStyle(el);
                for (let i = 0; i < styles.length; i++) {
                    window.__onecrawl_css_used.add(styles[i]);
                }
            });

            return 'installed';
        })()
    "#;
    page.evaluate(js)
        .await
        .map_err(|e| Error::Browser(format!("start_css_coverage JS failed: {e}")))?;

    Ok(())
}

/// Get CSS coverage summary.
pub async fn get_css_coverage(page: &Page) -> Result<serde_json::Value> {
    let result = page
        .evaluate(
            r#"
            (() => {
                const used = window.__onecrawl_css_used || new Set();
                const sheets = Array.from(document.styleSheets).map(s => {
                    try {
                        return {
                            href: s.href || 'inline',
                            rules: s.cssRules ? s.cssRules.length : 0
                        };
                    } catch(e) {
                        return { href: s.href || 'cross-origin', rules: -1 };
                    }
                });
                return {
                    used_properties: used.size,
                    stylesheets: sheets,
                    total_stylesheets: sheets.length
                };
            })()
            "#,
        )
        .await
        .map_err(|e| Error::Browser(format!("get_css_coverage failed: {e}")))?;

    result
        .into_value::<serde_json::Value>()
        .map_err(|e| Error::Browser(format!("parse css coverage: {e}")))
}
