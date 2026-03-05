use colored::Colorize;
use super::helpers::{with_page};

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

pub async fn proxy_create_pool(json: &str) {
    match onecrawl_cdp::ProxyPool::from_json(json) {
        Ok(pool) => match pool.to_json() {
            Ok(out) => println!("{out}"),
            Err(e) => {
                eprintln!("{} {e}", "✗".red());
                std::process::exit(1);
            }
        },
        Err(e) => {
            eprintln!("{} {e}", "✗".red());
            std::process::exit(1);
        }
    }
}

pub async fn proxy_chrome_args(json: &str) {
    match onecrawl_cdp::ProxyPool::from_json(json) {
        Ok(pool) => {
            let args = pool.chrome_args();
            println!("{}", args.join(" "));
        }
        Err(e) => {
            eprintln!("{} {e}", "✗".red());
            std::process::exit(1);
        }
    }
}

pub async fn proxy_next(json: &str) {
    match onecrawl_cdp::ProxyPool::from_json(json) {
        Ok(mut pool) => {
            pool.next_proxy();
            match pool.to_json() {
                Ok(out) => println!("{out}"),
                Err(e) => {
                    eprintln!("{} {e}", "✗".red());
                    std::process::exit(1);
                }
            }
        }
        Err(e) => {
            eprintln!("{} {e}", "✗".red());
            std::process::exit(1);
        }
    }
}

pub async fn intercept_set(rules_json: &str) {
    let rules: Vec<onecrawl_cdp::InterceptRule> = match serde_json::from_str(rules_json) {
        Ok(r) => r,
        Err(e) => {
            eprintln!("{} Invalid rules JSON: {e}", "✗".red());
            std::process::exit(1);
        }
    };
    with_page(|page| async move {
        onecrawl_cdp::intercept::set_intercept_rules(&page, rules)
            .await
            .map_err(|e| e.to_string())?;
        println!("{} Intercept rules set", "✓".green());
        Ok(())
    })
    .await;
}

pub async fn intercept_log() {
    with_page(|page| async move {
        let log = onecrawl_cdp::intercept::get_intercepted_requests(&page)
            .await
            .map_err(|e| e.to_string())?;
        println!("{}", serde_json::to_string_pretty(&log).unwrap_or_default());
        Ok(())
    })
    .await;
}

pub async fn intercept_clear() {
    with_page(|page| async move {
        onecrawl_cdp::intercept::clear_intercept_rules(&page)
            .await
            .map_err(|e| e.to_string())?;
        println!("{} Intercept rules cleared", "✓".green());
        Ok(())
    })
    .await;
}

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

pub async fn geo_apply(profile: &str) {
    let profile = profile.to_string();
    with_page(|page| async move {
        let geo: onecrawl_cdp::GeoProfile =
            if let Some(p) = onecrawl_cdp::geofencing::get_preset(&profile) {
                p
            } else {
                serde_json::from_str(&profile)
                    .map_err(|e| format!("Invalid profile name or JSON: {e}"))?
            };
        onecrawl_cdp::geofencing::apply_geo_profile(&page, &geo)
            .await
            .map_err(|e| e.to_string())?;
        println!(
            "{} Geo profile '{}' applied (lat={}, lng={})",
            "✓".green(),
            geo.name,
            geo.latitude,
            geo.longitude
        );
        Ok(())
    })
    .await;
}

pub async fn geo_presets() {
    let presets = onecrawl_cdp::geofencing::list_presets();
    for name in &presets {
        if let Some(p) = onecrawl_cdp::geofencing::get_preset(name) {
            println!(
                "  {} — lat={:.4}, lng={:.4}, tz={}",
                name.green(),
                p.latitude,
                p.longitude,
                p.timezone
            );
        }
    }
}

pub async fn geo_current() {
    with_page(|page| async move {
        let val = onecrawl_cdp::geofencing::get_current_geo(&page)
            .await
            .map_err(|e| e.to_string())?;
        println!("{}", serde_json::to_string_pretty(&val).unwrap_or_default());
        Ok(())
    })
    .await;
}

pub async fn request_execute(json: &str) {
    let json = json.to_string();
    with_page(|page| async move {
        let req: onecrawl_cdp::QueuedRequest =
            serde_json::from_str(&json).map_err(|e| format!("Invalid request JSON: {e}"))?;
        let result = onecrawl_cdp::request_queue::execute_request(&page, &req)
            .await
            .map_err(|e| e.to_string())?;
        println!(
            "{}",
            serde_json::to_string_pretty(&result).unwrap_or_default()
        );
        Ok(())
    })
    .await;
}

pub async fn request_batch(json: &str, concurrency: usize, delay: u64) {
    let json = json.to_string();
    with_page(|page| async move {
        let reqs: Vec<onecrawl_cdp::QueuedRequest> =
            serde_json::from_str(&json).map_err(|e| format!("Invalid requests JSON: {e}"))?;
        let config = onecrawl_cdp::QueueConfig {
            concurrency,
            delay_between_ms: delay,
            ..Default::default()
        };
        let results = onecrawl_cdp::request_queue::execute_batch(&page, &reqs, &config)
            .await
            .map_err(|e| e.to_string())?;
        println!(
            "{}",
            serde_json::to_string_pretty(&results).unwrap_or_default()
        );
        Ok(())
    })
    .await;
}

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

pub async fn robots_parse(source: &str) {
    // If it looks like a URL, fetch via browser; otherwise read as file
    if source.starts_with("http://") || source.starts_with("https://") {
        with_page(|page| async move {
            let robots = onecrawl_cdp::robots::fetch_robots(&page, source)
                .await
                .map_err(|e| e.to_string())?;
            println!(
                "{}",
                serde_json::to_string_pretty(&robots).unwrap_or_default()
            );
            Ok(())
        })
        .await;
    } else {
        let content = match std::fs::read_to_string(source) {
            Ok(c) => c,
            Err(e) => {
                eprintln!("{} Failed to read file: {}", "✗".red(), e);
                return;
            }
        };
        let robots = onecrawl_cdp::robots::parse_robots(&content);
        println!(
            "{}",
            serde_json::to_string_pretty(&robots).unwrap_or_default()
        );
    }
}

pub async fn robots_check(url: &str, path: &str, user_agent: &str) {
    with_page(|page| async move {
        let robots = onecrawl_cdp::robots::fetch_robots(&page, url)
            .await
            .map_err(|e| e.to_string())?;
        let allowed = onecrawl_cdp::robots::is_allowed(&robots, user_agent, path);
        if allowed {
            println!(
                "{} Path \"{}\" is {} for {}",
                "✓".green(),
                path,
                "ALLOWED".green(),
                user_agent
            );
        } else {
            println!(
                "{} Path \"{}\" is {} for {}",
                "✗".red(),
                path,
                "DISALLOWED".red(),
                user_agent
            );
        }
        Ok(())
    })
    .await;
}

pub async fn robots_sitemaps(url: &str) {
    with_page(|page| async move {
        let robots = onecrawl_cdp::robots::fetch_robots(&page, url)
            .await
            .map_err(|e| e.to_string())?;
        let sitemaps = onecrawl_cdp::robots::get_sitemaps(&robots);
        if sitemaps.is_empty() {
            println!("{} No sitemaps declared", "→".cyan());
        } else {
            for s in &sitemaps {
                println!("  {s}");
            }
        }
        Ok(())
    })
    .await;
}

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

pub fn pipeline_run(pipeline_path: &str, data_path: &str, output: Option<&str>, format: &str) {
    let pipeline =
        match onecrawl_cdp::data_pipeline::load_pipeline(std::path::Path::new(pipeline_path)) {
            Ok(p) => p,
            Err(e) => {
                eprintln!("{} Failed to load pipeline: {e}", "✗".red());
                std::process::exit(1);
            }
        };

    let data_str = match std::fs::read_to_string(data_path) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("{} Failed to read data: {e}", "✗".red());
            std::process::exit(1);
        }
    };

    let items: Vec<std::collections::HashMap<String, String>> =
        match serde_json::from_str(&data_str) {
            Ok(v) => v,
            Err(e) => {
                eprintln!("{} Invalid data JSON: {e}", "✗".red());
                std::process::exit(1);
            }
        };

    let result = onecrawl_cdp::data_pipeline::execute_pipeline(&pipeline, items);
    println!(
        "{} Pipeline '{}': {} → {} items ({} filtered, {} deduplicated)",
        "✓".green(),
        pipeline.name,
        result.input_count,
        result.output_count,
        result.filtered_count,
        result.deduplicated_count,
    );
    for err in &result.errors {
        eprintln!("  {} {err}", "⚠".yellow());
    }

    if let Some(out) = output {
        match onecrawl_cdp::data_pipeline::export_processed(
            &result,
            std::path::Path::new(out),
            format,
        ) {
            Ok(n) => println!("{} Exported {n} items to {}", "✓".green(), out.cyan()),
            Err(e) => {
                eprintln!("{} Export failed: {e}", "✗".red());
                std::process::exit(1);
            }
        }
    } else {
        println!(
            "{}",
            serde_json::to_string_pretty(&result).unwrap_or_default()
        );
    }
}

pub fn pipeline_validate(pipeline_path: &str) {
    let pipeline =
        match onecrawl_cdp::data_pipeline::load_pipeline(std::path::Path::new(pipeline_path)) {
            Ok(p) => p,
            Err(e) => {
                eprintln!("{} Failed to load pipeline: {e}", "✗".red());
                std::process::exit(1);
            }
        };

    let errors = onecrawl_cdp::data_pipeline::validate_pipeline(&pipeline);
    if errors.is_empty() {
        println!("{} Pipeline '{}' is valid", "✓".green(), pipeline.name);
    } else {
        eprintln!(
            "{} Pipeline '{}' has {} error(s):",
            "✗".red(),
            pipeline.name,
            errors.len()
        );
        for err in &errors {
            eprintln!("  - {err}");
        }
        std::process::exit(1);
    }
}

pub fn pipeline_save_file(pipeline_json: &str, path: &str) {
    let pipeline: onecrawl_cdp::Pipeline = match serde_json::from_str(pipeline_json) {
        Ok(p) => p,
        Err(e) => {
            eprintln!("{} Invalid pipeline JSON: {e}", "✗".red());
            std::process::exit(1);
        }
    };
    match onecrawl_cdp::data_pipeline::save_pipeline(&pipeline, std::path::Path::new(path)) {
        Ok(()) => println!("{} Pipeline saved to {}", "✓".green(), path.cyan()),
        Err(e) => {
            eprintln!("{} Save failed: {e}", "✗".red());
            std::process::exit(1);
        }
    }
}

pub fn pipeline_load_file(path: &str) {
    match onecrawl_cdp::data_pipeline::load_pipeline(std::path::Path::new(path)) {
        Ok(pipeline) => {
            println!(
                "{}",
                serde_json::to_string_pretty(&pipeline).unwrap_or_default()
            );
        }
        Err(e) => {
            eprintln!("{} Failed to load pipeline: {e}", "✗".red());
            std::process::exit(1);
        }
    }
}

pub async fn proxy_health_check(proxy: &str, test_url: Option<&str>, timeout: u64) {
    let proxy = proxy.to_string();
    let mut config = onecrawl_cdp::ProxyHealthConfig::default();
    if let Some(url) = test_url {
        config.test_url = url.to_string();
    }
    config.timeout_ms = timeout;
    with_page(|page| async move {
        let result = onecrawl_cdp::proxy_health::check_proxy(&page, &proxy, &config)
            .await
            .map_err(|e| e.to_string())?;
        println!(
            "{}",
            serde_json::to_string_pretty(&result).unwrap_or_default()
        );
        Ok(())
    })
    .await;
}

pub async fn proxy_health_check_all(proxies_json: &str) {
    let proxies: Vec<String> = match serde_json::from_str(proxies_json) {
        Ok(p) => p,
        Err(e) => {
            eprintln!("{} Invalid proxies JSON: {e}", "✗".red());
            std::process::exit(1);
        }
    };
    let config = onecrawl_cdp::ProxyHealthConfig::default();
    with_page(|page| async move {
        let results = onecrawl_cdp::proxy_health::check_proxies(&page, &proxies, &config)
            .await
            .map_err(|e| e.to_string())?;
        println!(
            "{}",
            serde_json::to_string_pretty(&results).unwrap_or_default()
        );
        Ok(())
    })
    .await;
}

pub fn proxy_health_rank(results_json: &str) {
    let results: Vec<onecrawl_cdp::ProxyHealthResult> = match serde_json::from_str(results_json) {
        Ok(r) => r,
        Err(e) => {
            eprintln!("{} Invalid results JSON: {e}", "✗".red());
            std::process::exit(1);
        }
    };
    let ranked = onecrawl_cdp::proxy_health::rank_proxies(&results);
    println!(
        "{}",
        serde_json::to_string_pretty(&ranked).unwrap_or_default()
    );
}

pub fn proxy_health_filter(results_json: &str, min_score: u32) {
    let results: Vec<onecrawl_cdp::ProxyHealthResult> = match serde_json::from_str(results_json) {
        Ok(r) => r,
        Err(e) => {
            eprintln!("{} Invalid results JSON: {e}", "✗".red());
            std::process::exit(1);
        }
    };
    let filtered = onecrawl_cdp::proxy_health::filter_healthy(&results, min_score);
    println!(
        "{}",
        serde_json::to_string_pretty(&filtered).unwrap_or_default()
    );
}

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
