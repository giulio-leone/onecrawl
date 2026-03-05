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
pub fn ratelimit_set(preset: Option<&str>, config_json: Option<&str>) {
    let cfg = if let Some(name) = preset {
        let presets = onecrawl_cdp::rate_limiter::presets();
        match presets.get(name) {
            Some(c) => c.clone(),
            None => {
                eprintln!(
                    "{} Unknown preset: {}. Use: conservative, moderate, aggressive, unlimited",
                    "✗".red(),
                    name
                );
                std::process::exit(1);
            }
        }
    } else if let Some(json) = config_json {
        match serde_json::from_str::<onecrawl_cdp::RateLimitConfig>(json) {
            Ok(c) => c,
            Err(e) => {
                eprintln!("{} Invalid config JSON: {e}", "✗".red());
                std::process::exit(1);
            }
        }
    } else {
        onecrawl_cdp::RateLimitConfig::default()
    };
    let state = onecrawl_cdp::RateLimitState::new(cfg);
    let stats = onecrawl_cdp::rate_limiter::get_stats(&state);
    println!("{} Rate limiter configured", "✓".green());
    println!(
        "{}",
        serde_json::to_string_pretty(&stats).unwrap_or_default()
    );
}

pub fn ratelimit_stats() {
    let state = onecrawl_cdp::RateLimitState::new(onecrawl_cdp::RateLimitConfig::default());
    let stats = onecrawl_cdp::rate_limiter::get_stats(&state);
    println!(
        "{}",
        serde_json::to_string_pretty(&stats).unwrap_or_default()
    );
}

pub fn ratelimit_reset() {
    println!("{} Rate limiter reset", "✓".green());
}

