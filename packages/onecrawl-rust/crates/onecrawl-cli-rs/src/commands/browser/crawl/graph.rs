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

#[allow(clippy::too_many_arguments)]

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

pub async fn graph_extract(base_url: Option<&str>) {
    with_page(|page| async move {
        let current_url: String = page
            .evaluate("window.location.href")
            .await
            .ok()
            .and_then(|v| v.into_value::<String>().ok())
            .unwrap_or_default();
        let base = base_url.unwrap_or(&current_url);
        let edges = onecrawl_cdp::link_graph::extract_links(&page, base)
            .await
            .map_err(|e| e.to_string())?;
        println!(
            "{}",
            serde_json::to_string_pretty(&edges).unwrap_or_default()
        );
        Ok(())
    })
    .await;
}

pub fn graph_build(edges_file: &str) {
    let data = match std::fs::read_to_string(edges_file) {
        Ok(d) => d,
        Err(e) => {
            eprintln!("{} Failed to read file: {}", "✗".red(), e);
            return;
        }
    };
    let edges: Vec<onecrawl_cdp::LinkEdge> = match serde_json::from_str(&data) {
        Ok(e) => e,
        Err(e) => {
            eprintln!("{} Invalid JSON: {}", "✗".red(), e);
            return;
        }
    };
    let graph = onecrawl_cdp::link_graph::build_graph(&edges);
    println!(
        "{}",
        serde_json::to_string_pretty(&graph).unwrap_or_default()
    );
}

pub fn graph_analyze(graph_file: &str) {
    let data = match std::fs::read_to_string(graph_file) {
        Ok(d) => d,
        Err(e) => {
            eprintln!("{} Failed to read file: {}", "✗".red(), e);
            return;
        }
    };
    let graph: onecrawl_cdp::LinkGraph = match serde_json::from_str(&data) {
        Ok(g) => g,
        Err(e) => {
            eprintln!("{} Invalid JSON: {}", "✗".red(), e);
            return;
        }
    };
    let stats = onecrawl_cdp::link_graph::analyze_graph(&graph);
    println!(
        "{}",
        serde_json::to_string_pretty(&stats).unwrap_or_default()
    );
}

pub fn graph_export(graph_file: &str, output: &str) {
    let data = match std::fs::read_to_string(graph_file) {
        Ok(d) => d,
        Err(e) => {
            eprintln!("{} Failed to read file: {}", "✗".red(), e);
            return;
        }
    };
    let graph: onecrawl_cdp::LinkGraph = match serde_json::from_str(&data) {
        Ok(g) => g,
        Err(e) => {
            eprintln!("{} Invalid JSON: {}", "✗".red(), e);
            return;
        }
    };
    match onecrawl_cdp::link_graph::export_graph_json(&graph, std::path::Path::new(output)) {
        Ok(()) => println!("{} Graph exported to {}", "✓".green(), output),
        Err(e) => eprintln!("{} Export failed: {}", "✗".red(), e),
    }
}

