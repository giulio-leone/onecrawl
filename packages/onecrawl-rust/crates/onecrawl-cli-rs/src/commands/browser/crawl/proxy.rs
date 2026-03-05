use colored::Colorize;
use super::super::helpers::{with_page};

// Rate Limiter (standalone — no Page required)
// Retry Queue (standalone — no Page required)
// Task Scheduler (standalone — no Page required)
// Session Pool (standalone — no Page required)

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

