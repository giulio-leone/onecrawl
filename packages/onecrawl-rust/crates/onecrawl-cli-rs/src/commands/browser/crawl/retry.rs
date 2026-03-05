use colored::Colorize;

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

pub fn retry_enqueue(url: &str, operation: &str, payload: Option<&str>) {
    let mut queue = onecrawl_cdp::RetryQueue::new(onecrawl_cdp::RetryConfig::default());
    let id = onecrawl_cdp::retry_queue::enqueue(&mut queue, url, operation, payload);
    println!("{} Enqueued: {} ({})", "✓".green(), id, operation.cyan());
}

pub fn retry_next() {
    let mut queue = onecrawl_cdp::RetryQueue::new(onecrawl_cdp::RetryConfig::default());
    match onecrawl_cdp::retry_queue::get_next(&mut queue) {
        Some(item) => println!("{}", serde_json::to_string_pretty(item).unwrap_or_default()),
        None => println!("No items due for retry"),
    }
}

pub fn retry_success(id: &str) {
    println!("{} Marked {} as success", "✓".green(), id.cyan());
}

pub fn retry_fail(id: &str, error: &str) {
    println!("{} Marked {} as failed: {}", "✓".green(), id.cyan(), error);
}

pub fn retry_stats() {
    let queue = onecrawl_cdp::RetryQueue::new(onecrawl_cdp::RetryConfig::default());
    let stats = onecrawl_cdp::retry_queue::get_stats(&queue);
    println!(
        "{}",
        serde_json::to_string_pretty(&stats).unwrap_or_default()
    );
}

pub fn retry_clear() {
    println!("{} Completed items cleared", "✓".green());
}

pub fn retry_save(path: &str) {
    let queue = onecrawl_cdp::RetryQueue::new(onecrawl_cdp::RetryConfig::default());
    match onecrawl_cdp::retry_queue::save_queue(&queue, std::path::Path::new(path)) {
        Ok(()) => println!("{} Queue saved to {}", "✓".green(), path.cyan()),
        Err(e) => {
            eprintln!("{} Save failed: {e}", "✗".red());
            std::process::exit(1);
        }
    }
}

pub fn retry_load(path: &str) {
    match onecrawl_cdp::retry_queue::load_queue(std::path::Path::new(path)) {
        Ok(queue) => {
            let stats = onecrawl_cdp::retry_queue::get_stats(&queue);
            println!("{} Queue loaded from {}", "✓".green(), path.cyan());
            println!(
                "{}",
                serde_json::to_string_pretty(&stats).unwrap_or_default()
            );
        }
        Err(e) => {
            eprintln!("{} Load failed: {e}", "✗".red());
            std::process::exit(1);
        }
    }
}

