//! Handler implementations for the `perf` super-tool.

use rmcp::{ErrorData as McpError, model::*};
use crate::helpers::{mcp_err, ensure_page, json_ok, McpResult};
use crate::types::*;
use crate::OneCrawlMcp;

impl OneCrawlMcp {

    // ════════════════════════════════════════════════════════════════
    //  Visual Regression Testing tools
    // ════════════════════════════════════════════════════════════════

    pub(crate) async fn vrt_run(
        &self,
        p: VrtRunParams,
    ) -> Result<CallToolResult, McpError> {
        let suite = if p.suite.trim().starts_with('{') {
            serde_json::from_str::<onecrawl_cdp::VrtSuite>(&p.suite)
                .map_err(|e| mcp_err(format!("invalid VRT suite: {e}")))?
        } else {
            onecrawl_cdp::vrt::load_suite(&p.suite)
                .mcp()?
        };

        let errors = onecrawl_cdp::vrt::validate_suite(&suite);
        if !errors.is_empty() {
            return json_ok(&serde_json::json!({ "valid": false, "errors": errors }));
        }

        let page = ensure_page(&self.browser).await?;
        let start = std::time::Instant::now();
        let mut results = Vec::new();
        let mut passed = 0usize;
        let mut failed = 0usize;
        let mut new_baselines = 0usize;
        let mut error_count = 0usize;

        for test in &suite.tests {
            onecrawl_cdp::navigation::goto(&page, &test.url)
                .await
                .mcp()?;

            if test.delay_ms > 0 {
                tokio::time::sleep(tokio::time::Duration::from_millis(test.delay_ms)).await;
            }

            if let Some(ref wait) = test.wait_for {
                let _ = onecrawl_cdp::element::evaluate(
                    &page,
                    &format!(
                        "await new Promise(r => {{ const i = setInterval(() => {{ if (document.querySelector('{}')) {{ clearInterval(i); r(); }} }}, 100); setTimeout(() => {{ clearInterval(i); r(); }}, 10000); }})",
                        wait.replace('\'', "\\'")
                    ),
                ).await;
            }

            let screenshot_data = if test.full_page {
                onecrawl_cdp::screenshot::screenshot_full(&page)
                    .await
                    .mcp()?
            } else {
                onecrawl_cdp::screenshot::screenshot_viewport(&page)
                    .await
                    .mcp()?
            };

            let result = onecrawl_cdp::vrt::compare_test(
                test,
                &screenshot_data,
                &suite.baseline_dir,
                &suite.output_dir,
                &suite.diff_dir,
                suite.threshold,
            );

            match result.status {
                onecrawl_cdp::VrtStatus::Passed => passed += 1,
                onecrawl_cdp::VrtStatus::Failed => failed += 1,
                onecrawl_cdp::VrtStatus::NewBaseline => new_baselines += 1,
                onecrawl_cdp::VrtStatus::Error => error_count += 1,
            }
            results.push(result);
        }

        let suite_result = onecrawl_cdp::VrtSuiteResult {
            suite_name: suite.name.clone(),
            total: suite.tests.len(),
            passed,
            failed,
            new_baselines,
            errors: error_count,
            results,
            duration_ms: start.elapsed().as_millis() as u64,
        };

        let junit = onecrawl_cdp::vrt::generate_junit_report(&suite_result);

        json_ok(&serde_json::json!({
            "suite_name": suite_result.suite_name,
            "total": suite_result.total,
            "passed": suite_result.passed,
            "failed": suite_result.failed,
            "new_baselines": suite_result.new_baselines,
            "errors": suite_result.errors,
            "duration_ms": suite_result.duration_ms,
            "results": suite_result.results,
            "junit_xml": junit,
        }))
    }


    pub(crate) async fn vrt_compare(
        &self,
        p: VrtCompareParams,
    ) -> Result<CallToolResult, McpError> {
        let page = ensure_page(&self.browser).await?;
        onecrawl_cdp::navigation::goto(&page, &p.url)
            .await
            .mcp()?;

        let screenshot_data = if p.full_page.unwrap_or(false) {
            onecrawl_cdp::screenshot::screenshot_full(&page)
                .await
                .mcp()?
        } else {
            onecrawl_cdp::screenshot::screenshot_viewport(&page)
                .await
                .mcp()?
        };

        let test = onecrawl_cdp::VrtTestCase {
            name: p.name.clone(),
            url: p.url.clone(),
            selector: p.selector,
            full_page: p.full_page.unwrap_or(false),
            threshold: p.threshold.unwrap_or(0.1),
            viewport: None,
            wait_for: None,
            hide_selectors: vec![],
            delay_ms: 0,
        };

        let baseline_dir = p.baseline_dir.as_deref().unwrap_or(".vrt/baselines");
        let result = onecrawl_cdp::vrt::compare_test(
            &test,
            &screenshot_data,
            baseline_dir,
            ".vrt/current",
            ".vrt/diffs",
            p.threshold.unwrap_or(0.1),
        );

        json_ok(&serde_json::to_value(&result).mcp()?)
    }


    pub(crate) async fn vrt_update_baseline(
        &self,
        p: VrtUpdateBaselineParams,
    ) -> Result<CallToolResult, McpError> {
        let baseline_dir = p.baseline_dir.as_deref().unwrap_or(".vrt/baselines");
        let current_dir = ".vrt/current";
        let current = onecrawl_cdp::vrt::load_baseline(current_dir, &p.test_name);

        match current {
            Some(data) => {
                let path =
                    onecrawl_cdp::vrt::save_baseline(baseline_dir, &p.test_name, &data)
                        .mcp()?;
                json_ok(&serde_json::json!({
                    "updated": true,
                    "test_name": p.test_name,
                    "baseline_path": path.to_string_lossy(),
                    "bytes": data.len(),
                }))
            }
            None => json_ok(&serde_json::json!({
                "updated": false,
                "error": format!("no current screenshot found for '{}'", p.test_name),
            })),
        }
    }

    // ════════════════════════════════════════════════════════════════
    //  AI Task Planner tools
    // ════════════════════════════════════════════════════════════════


    // ════════════════════════════════════════════════════════════════
    //  Performance Monitor tools
    // ════════════════════════════════════════════════════════════════

    pub(crate) async fn perf_audit(
        &self,
        p: PerfAuditParams,
    ) -> Result<CallToolResult, McpError> {
        let page = ensure_page(&self.browser).await?;

        if let Some(url) = &p.url {
            onecrawl_cdp::navigation::goto(&page, url)
                .await
                .mcp()?;
            tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;
        }

        let js = onecrawl_cdp::perf_monitor::metrics_collection_js();
        let result = page.evaluate(js).await.mcp()?;
        let metrics: serde_json::Value = result.into_value().unwrap_or(serde_json::Value::Null);

        let url = onecrawl_cdp::navigation::get_url(&page).await.unwrap_or_default();
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        let vitals: onecrawl_cdp::CoreWebVitals = serde_json::from_value(
            metrics.get("vitals").cloned().unwrap_or_default()
        ).unwrap_or_default();

        let ratings = onecrawl_cdp::perf_monitor::rate_vitals(&vitals);

        json_ok(&serde_json::json!({
            "url": url,
            "timestamp": now,
            "vitals": metrics.get("vitals"),
            "ratings": ratings,
            "navigation_timing": metrics.get("navigation_timing"),
            "resource_count": metrics.get("resource_count"),
            "memory": metrics.get("memory"),
        }))
    }


    pub(crate) async fn perf_budget(
        &self,
        p: PerfBudgetCheckParams,
    ) -> Result<CallToolResult, McpError> {
        let budget: onecrawl_cdp::PerfBudget = serde_json::from_str(&p.budget)
            .map_err(|e| mcp_err(format!("invalid budget: {e}")))?;

        let page = ensure_page(&self.browser).await?;

        if let Some(url) = &p.url {
            onecrawl_cdp::navigation::goto(&page, url)
                .await
                .mcp()?;
            tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;
        }

        let js = onecrawl_cdp::perf_monitor::metrics_collection_js();
        let result = page.evaluate(js).await.mcp()?;
        let metrics: serde_json::Value = result.into_value().unwrap_or(serde_json::Value::Null);

        let snapshot = onecrawl_cdp::PerfSnapshot {
            url: onecrawl_cdp::navigation::get_url(&page).await.unwrap_or_default(),
            timestamp: std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap_or_default().as_secs(),
            vitals: serde_json::from_value(metrics.get("vitals").cloned().unwrap_or_default()).unwrap_or_default(),
            navigation_timing: serde_json::from_value(metrics.get("navigation_timing").cloned().unwrap_or_default()).unwrap_or_default(),
            resource_count: serde_json::from_value(metrics.get("resource_count").cloned().unwrap_or_default()).unwrap_or_default(),
            memory: None,
            js_heap_size: None,
        };

        let budget_result = onecrawl_cdp::perf_monitor::check_budget(&snapshot, &budget);
        json_ok(&serde_json::to_value(&budget_result).mcp()?)
    }


    pub(crate) async fn perf_compare(
        &self,
        p: PerfCompareParams,
    ) -> Result<CallToolResult, McpError> {
        let baseline: onecrawl_cdp::PerfSnapshot = serde_json::from_str(&p.baseline)
            .map_err(|e| mcp_err(format!("invalid baseline: {e}")))?;
        let current: onecrawl_cdp::PerfSnapshot = serde_json::from_str(&p.current)
            .map_err(|e| mcp_err(format!("invalid current: {e}")))?;

        let threshold = p.threshold_pct.unwrap_or(10.0);
        let regressions = onecrawl_cdp::perf_monitor::detect_regressions(&baseline, &current, threshold);

        json_ok(&serde_json::json!({
            "baseline_url": baseline.url,
            "current_url": current.url,
            "threshold_pct": threshold,
            "regressions": regressions,
            "regressed": !regressions.is_empty(),
            "count": regressions.len(),
        }))
    }


    pub(crate) async fn pixel_diff(
        &self,
        p: PixelDiffParams,
    ) -> Result<CallToolResult, McpError> {
        let page = ensure_page(&self.browser).await?;
        let result = onecrawl_cdp::pixel_diff::pixel_diff(
            &page,
            &p.image_a,
            &p.image_b,
            p.threshold.unwrap_or(5.0),
            p.generate_diff.unwrap_or(true),
        )
        .await
        .mcp()?;
        json_ok(&serde_json::to_value(&result).mcp()?)
    }

    pub(crate) async fn perf_trace(
        &self,
        p: PerfTraceParams,
    ) -> Result<CallToolResult, McpError> {
        let page = ensure_page(&self.browser).await?;
        let start = std::time::Instant::now();

        onecrawl_cdp::navigation::goto(&page, &p.url)
            .await
            .mcp()?;

        let settle = p.settle_ms.unwrap_or(3000);
        tokio::time::sleep(tokio::time::Duration::from_millis(settle)).await;

        let js = onecrawl_cdp::perf_monitor::metrics_collection_js();
        let result = page.evaluate(js).await.mcp()?;
        let metrics: serde_json::Value = result.into_value().unwrap_or(serde_json::Value::Null);

        let vitals: onecrawl_cdp::CoreWebVitals = serde_json::from_value(
            metrics.get("vitals").cloned().unwrap_or_default()
        ).unwrap_or_default();
        let ratings = onecrawl_cdp::perf_monitor::rate_vitals(&vitals);

        let trace_duration = start.elapsed().as_millis() as u64;

        json_ok(&serde_json::json!({
            "url": p.url,
            "trace_duration_ms": trace_duration,
            "settle_ms": settle,
            "vitals": metrics.get("vitals"),
            "ratings": ratings,
            "navigation_timing": metrics.get("navigation_timing"),
            "resource_count": metrics.get("resource_count"),
            "memory": metrics.get("memory"),
        }))
    }

}
