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
pub fn schedule_add(
    name: &str,
    task_type: &str,
    config: &str,
    interval: u64,
    delay: u64,
    max_runs: Option<usize>,
) {
    let mut sched = onecrawl_cdp::Scheduler::new();
    let schedule = onecrawl_cdp::TaskSchedule {
        interval_ms: interval,
        delay_ms: delay,
        max_runs,
    };
    let id = onecrawl_cdp::scheduler::add_task(&mut sched, name, task_type, config, schedule);
    println!("{} Task added: {}", "✓".green(), id);
}

pub fn schedule_remove(id: &str) {
    let mut sched = onecrawl_cdp::Scheduler::new();
    if onecrawl_cdp::scheduler::remove_task(&mut sched, id) {
        println!("{} Task removed: {id}", "✓".green());
    } else {
        eprintln!("{} Task not found: {id}", "✗".red());
    }
}

pub fn schedule_pause(id: &str) {
    let mut sched = onecrawl_cdp::Scheduler::new();
    if onecrawl_cdp::scheduler::pause_task(&mut sched, id) {
        println!("{} Task paused: {id}", "✓".green());
    } else {
        eprintln!("{} Task not found: {id}", "✗".red());
    }
}

pub fn schedule_resume(id: &str) {
    let mut sched = onecrawl_cdp::Scheduler::new();
    if onecrawl_cdp::scheduler::resume_task(&mut sched, id) {
        println!("{} Task resumed: {id}", "✓".green());
    } else {
        eprintln!("{} Task not found or not paused: {id}", "✗".red());
    }
}

pub fn schedule_list() {
    let sched = onecrawl_cdp::Scheduler::new();
    println!(
        "{}",
        serde_json::to_string_pretty(&sched.tasks).unwrap_or_default()
    );
}

pub fn schedule_stats() {
    let sched = onecrawl_cdp::Scheduler::new();
    let stats = onecrawl_cdp::scheduler::get_stats(&sched);
    println!(
        "{}",
        serde_json::to_string_pretty(&stats).unwrap_or_default()
    );
}

pub fn schedule_save(path: &str) {
    let sched = onecrawl_cdp::Scheduler::new();
    match onecrawl_cdp::scheduler::save_scheduler(&sched, std::path::Path::new(path)) {
        Ok(()) => println!("{} Scheduler saved to {path}", "✓".green()),
        Err(e) => eprintln!("{} Save failed: {e}", "✗".red()),
    }
}

pub fn schedule_load(path: &str) {
    match onecrawl_cdp::scheduler::load_scheduler(std::path::Path::new(path)) {
        Ok(sched) => {
            println!(
                "{} Scheduler loaded: {} tasks",
                "✓".green(),
                sched.tasks.len()
            );
        }
        Err(e) => eprintln!("{} Load failed: {e}", "✗".red()),
    }
}

