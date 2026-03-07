//! Benchmark utilities for measuring CDP operation performance.
//!
//! Uses `std::time::Instant` only — no external benchmark crate dependencies.

use serde::{Deserialize, Serialize};
use std::time::{Duration, Instant};

/// Result of a single benchmark run.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BenchmarkResult {
    pub name: String,
    pub iterations: u32,
    pub total_ms: f64,
    pub avg_ms: f64,
    pub min_ms: f64,
    pub max_ms: f64,
    pub p50_ms: f64,
    pub p95_ms: f64,
    pub p99_ms: f64,
    pub ops_per_sec: f64,
}

/// Collection of benchmark results.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BenchmarkSuite {
    pub results: Vec<BenchmarkResult>,
    pub total_duration_ms: f64,
    pub timestamp: String,
}

/// Run a benchmark of an async function.
pub async fn bench_async<F, Fut>(name: &str, iterations: u32, f: F) -> BenchmarkResult
where
    F: Fn() -> Fut,
    Fut: std::future::Future<
            Output = std::result::Result<(), Box<dyn std::error::Error + Send + Sync>>,
        >,
{
    let mut timings = Vec::with_capacity(iterations as usize);

    // Warmup (10% of iterations, min 1)
    let warmup = (iterations / 10).max(1);
    for _ in 0..warmup {
        let _ = f().await;
    }

    let total_start = Instant::now();
    for _ in 0..iterations {
        let start = Instant::now();
        let _ = f().await;
        timings.push(start.elapsed());
    }
    let total = total_start.elapsed();

    timings.sort();
    build_result(name, iterations, total, &timings)
}

/// Run a benchmark of a sync function.
pub fn bench_sync<F>(name: &str, iterations: u32, f: F) -> BenchmarkResult
where
    F: Fn() -> std::result::Result<(), Box<dyn std::error::Error + Send + Sync>>,
{
    let mut timings = Vec::with_capacity(iterations as usize);

    let warmup = (iterations / 10).max(1);
    for _ in 0..warmup {
        let _ = f();
    }

    let total_start = Instant::now();
    for _ in 0..iterations {
        let start = Instant::now();
        let _ = f();
        timings.push(start.elapsed());
    }
    let total = total_start.elapsed();

    timings.sort();
    build_result(name, iterations, total, &timings)
}

fn build_result(
    name: &str,
    iterations: u32,
    total: Duration,
    timings: &[Duration],
) -> BenchmarkResult {
    let total_ms = total.as_secs_f64() * 1000.0;
    let avg_ms = total_ms / iterations as f64;
    let min_ms = timings
        .first()
        .map(|d| d.as_secs_f64() * 1000.0)
        .unwrap_or(0.0);
    let max_ms = timings
        .last()
        .map(|d| d.as_secs_f64() * 1000.0)
        .unwrap_or(0.0);
    let p50_ms = percentile(timings, 50.0);
    let p95_ms = percentile(timings, 95.0);
    let p99_ms = percentile(timings, 99.0);
    let ops_per_sec = if total_ms > 0.0 {
        (iterations as f64 / total_ms) * 1000.0
    } else {
        0.0
    };

    BenchmarkResult {
        name: name.to_string(),
        iterations,
        total_ms,
        avg_ms,
        min_ms,
        max_ms,
        p50_ms,
        p95_ms,
        p99_ms,
        ops_per_sec,
    }
}

fn percentile(sorted: &[Duration], pct: f64) -> f64 {
    if sorted.is_empty() {
        return 0.0;
    }
    let idx = ((pct / 100.0) * (sorted.len() - 1) as f64).round() as usize;
    let idx = idx.min(sorted.len() - 1);
    sorted[idx].as_secs_f64() * 1000.0
}

/// Format results as a table string.
pub fn format_results(suite: &BenchmarkSuite) -> String {
    let mut out = String::new();
    out.push_str(&format!(
        "OneCrawl CDP Benchmark Suite — {}\n",
        suite.timestamp
    ));
    out.push_str(&format!(
        "Total duration: {:.1}ms\n\n",
        suite.total_duration_ms
    ));
    out.push_str(&format!(
        "{:<35} {:>6} {:>10} {:>10} {:>10} {:>10} {:>10} {:>12}\n",
        "Benchmark", "Iters", "Avg(ms)", "P50(ms)", "P95(ms)", "P99(ms)", "Min(ms)", "Ops/sec"
    ));
    out.push_str(&"-".repeat(115));
    out.push('\n');

    for r in &suite.results {
        out.push_str(&format!(
            "{:<35} {:>6} {:>10.3} {:>10.3} {:>10.3} {:>10.3} {:>10.3} {:>12.1}\n",
            r.name, r.iterations, r.avg_ms, r.p50_ms, r.p95_ms, r.p99_ms, r.min_ms, r.ops_per_sec
        ));
    }
    out
}

/// Run the full CDP benchmark suite against a live page.
pub async fn run_cdp_benchmarks(page: &onecrawl_browser::Page, iterations: u32) -> BenchmarkSuite {
    let suite_start = Instant::now();
    let mut results = Vec::new();

    // 1. Navigation
    {
        let p = page.clone();
        results.push(
            bench_async("navigation::goto (data URL)", iterations, || {
                let p = p.clone();
                async move {
                    crate::navigation::goto(&p, "data:text/html,<h1>bench</h1>")
                        .await
                        .map_err(|e| Box::new(e) as Box<dyn std::error::Error + Send + Sync>)
                }
            })
            .await,
        );
    }

    // 2. JS evaluation
    {
        let p = page.clone();
        results.push(
            bench_async("page::evaluate_js (1+1)", iterations, || {
                let p = p.clone();
                async move {
                    crate::page::evaluate_js(&p, "1+1")
                        .await
                        .map(|_| ())
                        .map_err(|e| Box::new(e) as Box<dyn std::error::Error + Send + Sync>)
                }
            })
            .await,
        );
    }

    // 3. DOM query (querySelector)
    {
        let p = page.clone();
        // Ensure a page with elements
        let _ =
            crate::navigation::goto(&p, "data:text/html,<h1 id='t'>hello</h1><p>world</p>").await;
        results.push(
            bench_async("element::get_text (h1)", iterations, || {
                let p = p.clone();
                async move {
                    crate::element::get_text(&p, "h1")
                        .await
                        .map(|_| ())
                        .map_err(|e| Box::new(e) as Box<dyn std::error::Error + Send + Sync>)
                }
            })
            .await,
        );
    }

    // 4. Screenshot capture
    {
        let p = page.clone();
        results.push(
            bench_async("screenshot::screenshot_viewport", iterations, || {
                let p = p.clone();
                async move {
                    crate::screenshot::screenshot_viewport(&p)
                        .await
                        .map(|_| ())
                        .map_err(|e| Box::new(e) as Box<dyn std::error::Error + Send + Sync>)
                }
            })
            .await,
        );
    }

    // 5. Console capture start/drain
    {
        let p = page.clone();
        results.push(
            bench_async("console::start+drain", iterations, || {
                let p = p.clone();
                async move {
                    crate::console::start_console_capture(&p)
                        .await
                        .map_err(|e| Box::new(e) as Box<dyn std::error::Error + Send + Sync>)?;
                    crate::console::drain_console_entries(&p)
                        .await
                        .map(|_| ())
                        .map_err(|e| Box::new(e) as Box<dyn std::error::Error + Send + Sync>)
                }
            })
            .await,
        );
    }

    // 6. Cookie operations (set/get/delete)
    {
        let p = page.clone();
        // Navigate to a real origin so cookies work
        let _ = crate::navigation::goto(&p, "data:text/html,<h1>cookie-bench</h1>").await;
        results.push(
            bench_async("cookie::set+get+clear", iterations, || {
                let p = p.clone();
                async move {
                    let params = crate::cookie::SetCookieParams {
                        name: "bench".into(),
                        value: "val".into(),
                        domain: Some("localhost".into()),
                        path: Some("/".into()),
                        expires: None,
                        http_only: None,
                        secure: None,
                        same_site: None,
                        url: None,
                    };
                    crate::cookie::set_cookie(&p, &params)
                        .await
                        .map_err(|e| Box::new(e) as Box<dyn std::error::Error + Send + Sync>)?;
                    crate::cookie::get_cookies(&p)
                        .await
                        .map(|_| ())
                        .map_err(|e| Box::new(e) as Box<dyn std::error::Error + Send + Sync>)?;
                    crate::cookie::clear_cookies(&p)
                        .await
                        .map_err(|e| Box::new(e) as Box<dyn std::error::Error + Send + Sync>)
                }
            })
            .await,
        );
    }

    // 7. WebStorage operations (set/get/clear)
    {
        let p = page.clone();
        results.push(
            bench_async("web_storage::local set+get+clear", iterations, || {
                let p = p.clone();
                async move {
                    crate::web_storage::set_local_storage(&p, "bench_key", "bench_val")
                        .await
                        .map_err(|e| Box::new(e) as Box<dyn std::error::Error + Send + Sync>)?;
                    crate::web_storage::get_local_storage(&p)
                        .await
                        .map(|_| ())
                        .map_err(|e| Box::new(e) as Box<dyn std::error::Error + Send + Sync>)?;
                    crate::web_storage::clear_local_storage(&p)
                        .await
                        .map_err(|e| Box::new(e) as Box<dyn std::error::Error + Send + Sync>)
                }
            })
            .await,
        );
    }

    // 8. DOM snapshot
    {
        let p = page.clone();
        results.push(
            bench_async("dom_observer::get_dom_snapshot", iterations, || {
                let p = p.clone();
                async move {
                    crate::dom_observer::get_dom_snapshot(&p, None)
                        .await
                        .map(|_| ())
                        .map_err(|e| Box::new(e) as Box<dyn std::error::Error + Send + Sync>)
                }
            })
            .await,
        );
    }

    // 9. Accessibility tree
    {
        let p = page.clone();
        results.push(
            bench_async("accessibility::get_tree", iterations, || {
                let p = p.clone();
                async move {
                    crate::accessibility::get_accessibility_tree(&p)
                        .await
                        .map(|_| ())
                        .map_err(|e| Box::new(e) as Box<dyn std::error::Error + Send + Sync>)
                }
            })
            .await,
        );
    }

    let total_duration_ms = suite_start.elapsed().as_secs_f64() * 1000.0;
    let timestamp = chrono_lite_now();

    BenchmarkSuite {
        results,
        total_duration_ms,
        timestamp,
    }
}

/// Simple ISO-8601 timestamp without chrono dependency.
fn chrono_lite_now() -> String {
    use std::process::Command;
    Command::new("date")
        .arg("+%Y-%m-%dT%H:%M:%S%z")
        .output()
        .ok()
        .and_then(|o| String::from_utf8(o.stdout).ok())
        .map(|s| s.trim().to_string())
        .unwrap_or_else(|| "unknown".into())
}
