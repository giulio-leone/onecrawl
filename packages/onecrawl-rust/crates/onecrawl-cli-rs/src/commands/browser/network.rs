use colored::Colorize;
use super::helpers::{with_page};

// ---------------------------------------------------------------------------
// HTTP Client
// ---------------------------------------------------------------------------

pub async fn network_block(types: &str) {
    let types = types.to_string();
    with_page(|page| async move {
        let resource_types: Vec<onecrawl_cdp::ResourceType> = types
            .split(',')
            .filter_map(|t| match t.trim().to_lowercase().as_str() {
                "image" | "images" => Some(onecrawl_cdp::ResourceType::Image),
                "stylesheet" | "css" => Some(onecrawl_cdp::ResourceType::Stylesheet),
                "font" | "fonts" => Some(onecrawl_cdp::ResourceType::Font),
                "script" | "js" => Some(onecrawl_cdp::ResourceType::Script),
                "media" => Some(onecrawl_cdp::ResourceType::Media),
                "xhr" => Some(onecrawl_cdp::ResourceType::Xhr),
                "fetch" => Some(onecrawl_cdp::ResourceType::Fetch),
                "websocket" | "ws" => Some(onecrawl_cdp::ResourceType::WebSocket),
                "document" => Some(onecrawl_cdp::ResourceType::Document),
                _ => None,
            })
            .collect();
        if resource_types.is_empty() {
            return Err("No valid resource types. Use: image,stylesheet,font,script,media,xhr,fetch,websocket".into());
        }
        onecrawl_cdp::network::block_resources(&page, &resource_types)
            .await
            .map_err(|e| e.to_string())?;
        println!(
            "{} Blocking {} resource type(s)",
            "✓".green(),
            resource_types.len()
        );
        Ok(())
    })
    .await;
}

pub async fn har_start() {
    with_page(|page| async move {
        let recorder = onecrawl_cdp::HarRecorder::new();
        onecrawl_cdp::har::start_har_recording(&page, &recorder)
            .await
            .map_err(|e| e.to_string())?;
        println!("{} HAR recording started", "✓".green());
        Ok(())
    })
    .await;
}

pub async fn har_drain() {
    with_page(|page| async move {
        let recorder = onecrawl_cdp::HarRecorder::new();
        let count = onecrawl_cdp::har::drain_har_entries(&page, &recorder)
            .await
            .map_err(|e| e.to_string())?;
        println!("{} Drained {} HAR entries", "✓".green(), count);
        Ok(())
    })
    .await;
}

pub async fn har_export(output: &str) {
    let out = output.to_string();
    with_page(|page| async move {
        let recorder = onecrawl_cdp::HarRecorder::new();
        // Start + drain to capture current entries
        let _ = onecrawl_cdp::har::start_har_recording(&page, &recorder).await;
        tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
        let _ = onecrawl_cdp::har::drain_har_entries(&page, &recorder).await;
        let url = onecrawl_cdp::navigation::get_url(&page)
            .await
            .unwrap_or_default();
        let har = onecrawl_cdp::har::export_har(&recorder, &url)
            .await
            .map_err(|e| e.to_string())?;
        let json = serde_json::to_string_pretty(&har).unwrap_or_default();
        std::fs::write(&out, &json).map_err(|e| format!("write failed: {e}"))?;
        println!("{} HAR exported to {}", "✓".green(), out.cyan());
        Ok(())
    })
    .await;
}

pub async fn ws_start() {
    with_page(|page| async move {
        let recorder = onecrawl_cdp::WsRecorder::new();
        onecrawl_cdp::websocket::start_ws_recording(&page, &recorder)
            .await
            .map_err(|e| e.to_string())?;
        println!("{} WebSocket recording started", "✓".green());
        Ok(())
    })
    .await;
}

pub async fn ws_drain() {
    with_page(|page| async move {
        let recorder = onecrawl_cdp::WsRecorder::new();
        let count = onecrawl_cdp::websocket::drain_ws_frames(&page, &recorder)
            .await
            .map_err(|e| e.to_string())?;
        println!("{} Drained {} WebSocket frames", "✓".green(), count);
        Ok(())
    })
    .await;
}

pub async fn ws_export(output: &str) {
    let out = output.to_string();
    with_page(|page| async move {
        let recorder = onecrawl_cdp::WsRecorder::new();
        let _ = onecrawl_cdp::websocket::start_ws_recording(&page, &recorder).await;
        tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
        let _ = onecrawl_cdp::websocket::drain_ws_frames(&page, &recorder).await;
        let frames = onecrawl_cdp::websocket::export_ws_frames(&recorder)
            .await
            .map_err(|e| e.to_string())?;
        let json = serde_json::to_string_pretty(&frames).unwrap_or_default();
        std::fs::write(&out, &json).map_err(|e| format!("write failed: {e}"))?;
        println!(
            "{} WebSocket frames exported to {}",
            "✓".green(),
            out.cyan()
        );
        Ok(())
    })
    .await;
}

pub async fn ws_connections() {
    with_page(|page| async move {
        let count = onecrawl_cdp::websocket::active_ws_connections(&page)
            .await
            .map_err(|e| e.to_string())?;
        println!("{count}");
        Ok(())
    })
    .await;
}

fn cli_parse_network_profile(name: &str) -> Result<onecrawl_cdp::NetworkProfile, String> {
    match name.to_lowercase().as_str() {
        "fast3g" | "fast-3g" => Ok(onecrawl_cdp::NetworkProfile::Fast3G),
        "slow3g" | "slow-3g" => Ok(onecrawl_cdp::NetworkProfile::Slow3G),
        "offline" => Ok(onecrawl_cdp::NetworkProfile::Offline),
        "regular4g" | "4g" => Ok(onecrawl_cdp::NetworkProfile::Regular4G),
        "wifi" => Ok(onecrawl_cdp::NetworkProfile::WiFi),
        _ => Err(format!(
            "Unknown profile: {name}. Use: fast3g, slow3g, offline, regular4g, wifi"
        )),
    }
}

pub async fn throttle_set(profile: &str) {
    let p = match cli_parse_network_profile(profile) {
        Ok(p) => p,
        Err(e) => {
            eprintln!("{} {e}", "✗".red());
            std::process::exit(1);
        }
    };
    let desc = onecrawl_cdp::throttle::describe_profile(&p);
    with_page(|page| async move {
        onecrawl_cdp::throttle::set_network_conditions(&page, p)
            .await
            .map_err(|e| e.to_string())?;
        println!("{} Network throttle set: {}", "✓".green(), desc);
        Ok(())
    })
    .await;
}

pub async fn throttle_custom(download_kbps: f64, upload_kbps: f64, latency_ms: f64) {
    let profile = onecrawl_cdp::NetworkProfile::Custom {
        download_kbps,
        upload_kbps,
        latency_ms,
    };
    with_page(|page| async move {
        onecrawl_cdp::throttle::set_network_conditions(&page, profile)
            .await
            .map_err(|e| e.to_string())?;
        println!(
            "{} Custom throttle set: ↓{}kbps ↑{}kbps ~{}ms",
            "✓".green(),
            download_kbps,
            upload_kbps,
            latency_ms
        );
        Ok(())
    })
    .await;
}

pub async fn throttle_clear() {
    with_page(|page| async move {
        onecrawl_cdp::throttle::clear_network_conditions(&page)
            .await
            .map_err(|e| e.to_string())?;
        println!("{} Network throttle cleared", "✓".green());
        Ok(())
    })
    .await;
}

pub async fn network_log_start() {
    with_page(|page| async move {
        onecrawl_cdp::network_log::start_network_log(&page)
            .await
            .map_err(|e| e.to_string())?;
        println!("{} Network logging started", "✓".green());
        Ok(())
    })
    .await;
}

pub async fn network_log_drain() {
    with_page(|page| async move {
        let entries = onecrawl_cdp::network_log::drain_network_log(&page)
            .await
            .map_err(|e| e.to_string())?;
        println!(
            "{}",
            serde_json::to_string_pretty(&entries).unwrap_or_default()
        );
        Ok(())
    })
    .await;
}

pub async fn network_log_summary() {
    with_page(|page| async move {
        let summary = onecrawl_cdp::network_log::get_network_summary(&page)
            .await
            .map_err(|e| e.to_string())?;
        println!(
            "{}",
            serde_json::to_string_pretty(&summary).unwrap_or_default()
        );
        Ok(())
    })
    .await;
}

pub async fn network_log_stop() {
    with_page(|page| async move {
        onecrawl_cdp::network_log::stop_network_log(&page)
            .await
            .map_err(|e| e.to_string())?;
        println!("{} Network logging stopped", "✓".green());
        Ok(())
    })
    .await;
}

pub async fn network_log_export(path: &str) {
    let p = path.to_string();
    with_page(|page| async move {
        onecrawl_cdp::network_log::export_network_log(&page, &p)
            .await
            .map_err(|e| e.to_string())?;
        println!("{} Network log exported to {}", "✓".green(), p.cyan());
        Ok(())
    })
    .await;
}

pub async fn domain_block(domains: &[String]) {
    with_page(|page| async move {
        let count = onecrawl_cdp::domain_blocker::block_domains(&page, domains)
            .await
            .map_err(|e| e.to_string())?;
        println!(
            "{} Blocked {} domain(s) — {} total on blocklist",
            "✓".green(),
            domains.len(),
            count
        );
        Ok(())
    })
    .await;
}

pub async fn domain_block_category(category: &str) {
    let cat = category.to_string();
    with_page(|page| async move {
        let count = onecrawl_cdp::domain_blocker::block_category(&page, &cat)
            .await
            .map_err(|e| e.to_string())?;
        println!(
            "{} Category '{}' blocked — {} total on blocklist",
            "✓".green(),
            cat.cyan(),
            count
        );
        Ok(())
    })
    .await;
}

pub async fn domain_unblock() {
    with_page(|page| async move {
        onecrawl_cdp::domain_blocker::clear_blocks(&page)
            .await
            .map_err(|e| e.to_string())?;
        println!("{} All domain blocks cleared", "✓".green());
        Ok(())
    })
    .await;
}

pub async fn domain_stats() {
    with_page(|page| async move {
        let stats = onecrawl_cdp::domain_blocker::block_stats(&page)
            .await
            .map_err(|e| e.to_string())?;
        println!(
            "{}",
            serde_json::to_string_pretty(&stats).unwrap_or_default()
        );
        Ok(())
    })
    .await;
}

pub async fn domain_list() {
    with_page(|page| async move {
        let domains = onecrawl_cdp::domain_blocker::list_blocked(&page)
            .await
            .map_err(|e| e.to_string())?;
        if domains.is_empty() {
            println!("No domains currently blocked.");
        } else {
            for d in &domains {
                println!("  • {}", d);
            }
            println!("\n{} domain(s) blocked", domains.len());
        }
        Ok(())
    })
    .await;
}

pub fn domain_categories() {
    let cats = onecrawl_cdp::domain_blocker::available_categories();
    for (name, count) in &cats {
        println!("  {:<12} {} domains", name.cyan(), count);
    }
}

pub async fn http_get(url: &str) {
    let url = url.to_string();
    with_page(|page| async move {
        let resp = onecrawl_cdp::http_client::get(&page, &url, None)
            .await
            .map_err(|e| e.to_string())?;
        println!(
            "{}",
            serde_json::to_string_pretty(&resp).unwrap_or_default()
        );
        Ok(())
    })
    .await;
}

pub async fn http_post(url: &str, body: &str, content_type: &str) {
    let url = url.to_string();
    let body = body.to_string();
    let content_type = content_type.to_string();
    with_page(|page| async move {
        let resp = onecrawl_cdp::http_client::post(&page, &url, &body, &content_type, None)
            .await
            .map_err(|e| e.to_string())?;
        println!(
            "{}",
            serde_json::to_string_pretty(&resp).unwrap_or_default()
        );
        Ok(())
    })
    .await;
}

pub async fn http_head(url: &str) {
    let url = url.to_string();
    with_page(|page| async move {
        let resp = onecrawl_cdp::http_client::head(&page, &url)
            .await
            .map_err(|e| e.to_string())?;
        println!(
            "{}",
            serde_json::to_string_pretty(&resp).unwrap_or_default()
        );
        Ok(())
    })
    .await;
}

pub async fn http_fetch(json: &str) {
    let json = json.to_string();
    with_page(|page| async move {
        let request: onecrawl_cdp::HttpRequest =
            serde_json::from_str(&json).map_err(|e| e.to_string())?;
        let resp = onecrawl_cdp::http_client::fetch(&page, &request)
            .await
            .map_err(|e| e.to_string())?;
        println!(
            "{}",
            serde_json::to_string_pretty(&resp).unwrap_or_default()
        );
        Ok(())
    })
    .await;
}

pub async fn http_adaptive(url: &str, retries: u32, no_escalate: bool, user_agent: Option<&str>) {
    let url = url.to_string();
    let ua = user_agent.map(|s| s.to_string());
    with_page(|page| async move {
        let config = onecrawl_cdp::adaptive_fetch::AdaptiveFetchConfig {
            max_retries: retries,
            escalate_to_cdp: !no_escalate,
            user_agent: ua,
            ..Default::default()
        };
        let result = onecrawl_cdp::adaptive_fetch::adaptive_get(&page, &url, Some(config))
            .await
            .map_err(|e| e.to_string())?;

        let method_label = if result.was_escalated {
            "CDP (escalated)".yellow().to_string()
        } else {
            "HTTP (direct)".green().to_string()
        };
        eprintln!(
            "{} {} — {} {} in {}ms ({} attempt{})",
            "✓".green(),
            method_label,
            result.status,
            result.url,
            result.duration_ms as u64,
            result.attempts,
            if result.attempts > 1 { "s" } else { "" }
        );

        println!("{}", result.body);
        Ok(())
    })
    .await;
}
