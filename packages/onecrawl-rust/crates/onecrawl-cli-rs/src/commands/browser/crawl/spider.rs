use colored::Colorize;
use super::super::helpers::{with_page};

// Rate Limiter (standalone — no Page required)
// Retry Queue (standalone — no Page required)
// Task Scheduler (standalone — no Page required)
// Session Pool (standalone — no Page required)

#[allow(clippy::too_many_arguments)]
pub async fn spider_crawl(
    start_url: &str,
    max_depth: usize,
    max_pages: usize,
    concurrency: usize,
    delay: u64,
    same_domain: bool,
    selector: Option<&str>,
    format: &str,
    output: Option<&str>,
    output_format: &str,
) {
    with_page(|page| async move {
        let config = onecrawl_cdp::SpiderConfig {
            start_urls: vec![start_url.to_string()],
            max_depth,
            max_pages,
            concurrency,
            delay_ms: delay,
            follow_links: true,
            same_domain_only: same_domain,
            extract_selector: selector.map(String::from),
            extract_format: format.to_string(),
            ..Default::default()
        };
        println!(
            "{} Starting crawl from {} (depth={}, max_pages={})",
            "→".cyan(),
            start_url,
            max_depth,
            max_pages
        );
        let results = onecrawl_cdp::spider::crawl(&page, config)
            .await
            .map_err(|e| e.to_string())?;
        let summary = onecrawl_cdp::spider::summarize(&results);
        println!(
            "{} Crawl complete: {} pages ({} ok, {} failed) in {:.0}ms ({:.2} p/s)",
            "✓".green(),
            summary.total_pages,
            summary.successful,
            summary.failed,
            summary.total_duration_ms,
            summary.pages_per_second,
        );
        if let Some(path) = output {
            let p = std::path::Path::new(path);
            let count = match output_format {
                "jsonl" => onecrawl_cdp::spider::export_results_jsonl(&results, p),
                _ => onecrawl_cdp::spider::export_results(&results, p),
            }
            .map_err(|e| e.to_string())?;
            println!("{} Saved {} results to {}", "✓".green(), count, path);
        } else {
            println!(
                "{}",
                serde_json::to_string_pretty(&results).unwrap_or_default()
            );
        }
        Ok(())
    })
    .await;
}

pub async fn spider_resume(state_file: &str) {
    let state = match onecrawl_cdp::spider::load_state(std::path::Path::new(state_file)) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("{} Failed to load state: {}", "✗".red(), e);
            return;
        }
    };
    println!(
        "{} Resuming crawl: {} visited, {} pending",
        "→".cyan(),
        state.visited.len(),
        state.pending.len(),
    );
    let mut config = state.config.clone();
    config.start_urls = state.pending.iter().map(|(u, _)| u.clone()).collect();
    with_page(|page| async move {
        let results = onecrawl_cdp::spider::crawl(&page, config)
            .await
            .map_err(|e| e.to_string())?;
        let summary = onecrawl_cdp::spider::summarize(&results);
        println!(
            "{} Resume complete: {} pages ({} ok, {} failed)",
            "✓".green(),
            summary.total_pages,
            summary.successful,
            summary.failed,
        );
        println!(
            "{}",
            serde_json::to_string_pretty(&results).unwrap_or_default()
        );
        Ok(())
    })
    .await;
}

pub fn spider_summary(results_file: &str) {
    let data = match std::fs::read_to_string(results_file) {
        Ok(d) => d,
        Err(e) => {
            eprintln!("{} Failed to read file: {}", "✗".red(), e);
            return;
        }
    };
    let results: Vec<onecrawl_cdp::CrawlResult> = match serde_json::from_str(&data) {
        Ok(r) => r,
        Err(e) => {
            eprintln!("{} Invalid JSON: {}", "✗".red(), e);
            return;
        }
    };
    let summary = onecrawl_cdp::spider::summarize(&results);
    println!(
        "{}",
        serde_json::to_string_pretty(&summary).unwrap_or_default()
    );
}

