use colored::Colorize;
use super::super::helpers::{with_page};

// ---------------------------------------------------------------------------
// Proxy
// ---------------------------------------------------------------------------

// ---------------------------------------------------------------------------
// Request Interception
// ---------------------------------------------------------------------------

// ---------------------------------------------------------------------------
// Benchmark
// ---------------------------------------------------------------------------

// ---------------------------------------------------------------------------
// Geofencing
// ---------------------------------------------------------------------------

// ---------------------------------------------------------------------------
// Request Queue
// ---------------------------------------------------------------------------

// ── Spider / Crawl ─────────────────────────────────────────────────

// ---------------------------------------------------------------------------
// Robots.txt
// ---------------------------------------------------------------------------
// ---------------------------------------------------------------------------
// Link Graph
// ---------------------------------------------------------------------------
// ---------------------------------------------------------------------------
// Rate Limiter (standalone — no Page required)
// ---------------------------------------------------------------------------
// ---------------------------------------------------------------------------
// Retry Queue (standalone — no Page required)
// ---------------------------------------------------------------------------
// ──────────────── Data Pipeline ────────────────
// ---------------------------------------------------------------------------
// Proxy Health
// ---------------------------------------------------------------------------
// ---------------------------------------------------------------------------
// Task Scheduler (standalone — no Page required)
// ---------------------------------------------------------------------------
// ---------------------------------------------------------------------------
// Session Pool (standalone — no Page required)
// ---------------------------------------------------------------------------
pub async fn bench_run(iterations: u32, _module: Option<&str>) {
    with_page(|page| async move {
        println!(
            "{} Running CDP benchmarks ({iterations} iterations)…",
            "⏱".yellow()
        );
        let suite = onecrawl_cdp::benchmark::run_cdp_benchmarks(&page, iterations).await;
        let table = onecrawl_cdp::benchmark::format_results(&suite);
        println!("{table}");

        // Save JSON report
        let dir = std::path::PathBuf::from("reports");
        let _ = std::fs::create_dir_all(&dir);
        let json_path = dir.join("cdp-bench.json");
        if let Ok(json) = serde_json::to_string_pretty(&suite) {
            let _ = std::fs::write(&json_path, &json);
            println!("{} Report saved to {}", "✓".green(), json_path.display());
        }
        Ok(())
    })
    .await;
}

pub async fn bench_report(format: &str) {
    let json_path = std::path::PathBuf::from("reports").join("cdp-bench.json");

    let data = match std::fs::read_to_string(&json_path) {
        Ok(d) => d,
        Err(_) => {
            eprintln!(
                "{} No benchmark data found. Run `onecrawl bench run` first.",
                "✗".red()
            );
            std::process::exit(1);
        }
    };

    match format {
        "json" => println!("{data}"),
        _ => {
            if let Ok(suite) = serde_json::from_str::<onecrawl_cdp::BenchmarkSuite>(&data) {
                println!("{}", onecrawl_cdp::benchmark::format_results(&suite));
            } else {
                eprintln!("{} Failed to parse benchmark data", "✗".red());
                std::process::exit(1);
            }
        }
    }
}

