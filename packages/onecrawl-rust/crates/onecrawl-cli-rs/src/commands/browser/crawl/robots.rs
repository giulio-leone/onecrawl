use colored::Colorize;
use super::super::helpers::{with_page};

// Rate Limiter (standalone — no Page required)
// Retry Queue (standalone — no Page required)
// Task Scheduler (standalone — no Page required)
// Session Pool (standalone — no Page required)

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

