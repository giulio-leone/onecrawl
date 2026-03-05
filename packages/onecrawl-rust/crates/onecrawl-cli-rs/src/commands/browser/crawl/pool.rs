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
pub fn pool_add(name: &str, tags: Option<Vec<String>>) {
    let mut pool = onecrawl_cdp::SessionPool::new(onecrawl_cdp::PoolConfig::default());
    let id = onecrawl_cdp::session_pool::add_session(&mut pool, name, tags);
    println!("{} Session added: {}", "✓".green(), id);
}

pub fn pool_next() {
    let mut pool = onecrawl_cdp::SessionPool::new(onecrawl_cdp::PoolConfig::default());
    match onecrawl_cdp::session_pool::get_next(&mut pool) {
        Some(s) => println!("{}", serde_json::to_string_pretty(s).unwrap_or_default()),
        None => println!("{} No available sessions", "⚠".yellow()),
    }
}

pub fn pool_stats() {
    let pool = onecrawl_cdp::SessionPool::new(onecrawl_cdp::PoolConfig::default());
    let stats = onecrawl_cdp::session_pool::get_stats(&pool);
    println!(
        "{}",
        serde_json::to_string_pretty(&stats).unwrap_or_default()
    );
}

pub fn pool_cleanup() {
    let mut pool = onecrawl_cdp::SessionPool::new(onecrawl_cdp::PoolConfig::default());
    let n = onecrawl_cdp::session_pool::cleanup_idle(&mut pool);
    println!("{} Cleaned up {n} idle session(s)", "✓".green());
}

pub fn pool_save(path: &str) {
    let pool = onecrawl_cdp::SessionPool::new(onecrawl_cdp::PoolConfig::default());
    match onecrawl_cdp::session_pool::save_pool(&pool, std::path::Path::new(path)) {
        Ok(()) => println!("{} Pool saved to {path}", "✓".green()),
        Err(e) => eprintln!("{} Save failed: {e}", "✗".red()),
    }
}

pub fn pool_load(path: &str) {
    match onecrawl_cdp::session_pool::load_pool(std::path::Path::new(path)) {
        Ok(pool) => {
            println!(
                "{} Pool loaded: {} sessions",
                "✓".green(),
                pool.sessions.len()
            );
        }
        Err(e) => eprintln!("{} Load failed: {e}", "✗".red()),
    }
}
