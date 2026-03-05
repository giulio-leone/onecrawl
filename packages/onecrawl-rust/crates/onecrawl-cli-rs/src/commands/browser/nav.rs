use colored::Colorize;
use super::helpers::{with_page};

// ---------------------------------------------------------------------------
// Navigation
// ---------------------------------------------------------------------------

// ---------------------------------------------------------------------------
// Wait
// ---------------------------------------------------------------------------

pub async fn navigate(url: &str, wait: u64, wait_cf: bool) {
    let t0 = std::time::Instant::now();
    // Try proxy first (avoids CDP reconnect overhead)
    if let Some(proxy) = super::super::proxy::ServerProxy::from_session().await {
        match proxy.navigate(url).await {
            Ok(_) => {
                if wait > 0 {
                    tokio::time::sleep(std::time::Duration::from_millis(wait)).await;
                }
                let ms = t0.elapsed().as_millis();
                println!("{} Navigated to {} {} {}", "✓".green(), url.cyan(), format!("{ms}ms").dimmed(), "(proxy)".dimmed());
                return;
            }
            Err(_) => {} // fall through to CDP
        }
    }
    with_page(|page| async move {
        onecrawl_cdp::navigation::goto(&page, url)
            .await
            .map_err(|e| e.to_string())?;
        if wait > 0 {
            onecrawl_cdp::navigation::wait_ms(wait).await;
        }
        if wait_cf {
            let passed = onecrawl_cdp::human::wait_for_cf_clearance(&page, 30_000).await;
            if passed {
                println!("{} Cloudflare challenge cleared", "✓".green());
            } else {
                eprintln!("{} Cloudflare challenge did not clear within 30s", "✗".red());
            }
        }
        let ms = t0.elapsed().as_millis();
        println!("{} Navigated to {} {}", "✓".green(), url.cyan(), format!("{ms}ms").dimmed());
        Ok(())
    })
    .await;
}

pub async fn back() {
    with_page(|page| async move {
        onecrawl_cdp::navigation::go_back(&page)
            .await
            .map_err(|e| e.to_string())?;
        println!("{} Navigated back", "✓".green());
        Ok(())
    })
    .await;
}

pub async fn forward() {
    with_page(|page| async move {
        onecrawl_cdp::navigation::go_forward(&page)
            .await
            .map_err(|e| e.to_string())?;
        println!("{} Navigated forward", "✓".green());
        Ok(())
    })
    .await;
}

pub async fn reload() {
    with_page(|page| async move {
        onecrawl_cdp::navigation::reload(&page)
            .await
            .map_err(|e| e.to_string())?;
        println!("{} Page reloaded", "✓".green());
        Ok(())
    })
    .await;
}

pub async fn wait_ms(ms: u64) {
    onecrawl_cdp::navigation::wait_ms(ms).await;
    println!("{} Waited {}ms", "✓".green(), ms);
}

pub async fn wait_for_selector(selector: &str, timeout: u64) {
    let sel = selector.to_string();
    with_page(|page| async move {
        onecrawl_cdp::navigation::wait_for_selector(&page, &sel, timeout)
            .await
            .map_err(|e| e.to_string())?;
        println!("{} Selector '{}' found", "✓".green(), sel.dimmed());
        Ok(())
    })
    .await;
}

pub async fn wait_for_url(pattern: &str, timeout: u64) {
    let pat = pattern.to_string();
    with_page(|page| async move {
        onecrawl_cdp::navigation::wait_for_url(&page, &pat, timeout)
            .await
            .map_err(|e| e.to_string())?;
        println!("{} URL matched '{}'", "✓".green(), pat.dimmed());
        Ok(())
    })
    .await;
}
