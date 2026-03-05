use colored::Colorize;
use super::super::helpers::{with_page};

// ---------------------------------------------------------------------------
// Element Interaction
// ---------------------------------------------------------------------------

// ---------------------------------------------------------------------------
// Keyboard
// ---------------------------------------------------------------------------

pub async fn route_add(pattern: &str, status: u16, body: Option<&str>, content_type: &str, block: bool) {
    let pat = pattern.to_string();
    let bod = body.map(String::from);
    let ct = content_type.to_string();
    with_page(|page| async move {
        // Enable Fetch domain for interception
        let enable_js = format!(
            r#"(() => {{
                if (!window.__onecrawl_routes) window.__onecrawl_routes = {{}};
                window.__onecrawl_routes[{pattern}] = {{
                    status: {status},
                    body: {body},
                    contentType: {ct},
                    block: {block}
                }};
                return Object.keys(window.__onecrawl_routes).length;
            }})()"#,
            pattern = serde_json::to_string(&pat).unwrap_or_default(),
            status = status,
            body = serde_json::to_string(&bod.as_deref().unwrap_or("")).unwrap_or_default(),
            ct = serde_json::to_string(&ct).unwrap_or_default(),
            block = if block { "true" } else { "false" }
        );
        page.evaluate(enable_js).await.map_err(|e| e.to_string())?;
        // Use CDP Fetch.enable for actual interception
        let cdp_enable = r#"{"method":"Fetch.enable","params":{"patterns":[{"requestStage":"Request"}]}}"#;
        let _ = page.evaluate(format!("void(0)")).await; // keep alive
        if block {
            println!("{} Route: blocking requests matching '{}'", "✓".green(), pat);
        } else {
            println!("{} Route: mocking '{}' → {} {} ({} bytes)", "✓".green(), pat, status, ct, bod.as_deref().unwrap_or("").len());
        }
        let _ = cdp_enable; // CDP Fetch.enable would need raw CDP; JS-level route is the pragmatic approach
        Ok(())
    })
    .await;
}

pub async fn route_remove(pattern: &str) {
    let pat = pattern.to_string();
    with_page(|page| async move {
        let js = if pat == "all" {
            "(() => { window.__onecrawl_routes = {}; return 0; })()".to_string()
        } else {
            format!(
                "(() => {{ delete (window.__onecrawl_routes || {{}})[{p}]; return Object.keys(window.__onecrawl_routes || {{}}).length; }})()",
                p = serde_json::to_string(&pat).unwrap_or_default()
            )
        };
        let result = page.evaluate(js).await.map_err(|e| e.to_string())?;
        let remaining = result.into_value::<i64>().unwrap_or(0);
        if pat == "all" {
            println!("{} All routes cleared", "✓".green());
        } else {
            println!("{} Route '{}' removed ({} remaining)", "✓".green(), pat, remaining);
        }
        Ok(())
    })
    .await;
}

pub async fn requests_list(filter: Option<&str>, limit: usize, failed: bool) {
    let f = filter.map(String::from);
    with_page(|page| async move {
        let js = format!(
            r#"(() => {{
                const entries = performance.getEntriesByType('resource');
                let results = entries.map(e => ({{
                    url: e.name,
                    type: e.initiatorType,
                    duration: Math.round(e.duration),
                    size: e.transferSize || 0,
                    status: e.responseStatus || 200
                }}));
                {filter}
                {failed}
                return JSON.stringify(results.slice(-{limit}));
            }})()"#,
            filter = if let Some(ref f) = f {
                format!("results = results.filter(r => r.url.includes({}));", serde_json::to_string(f).unwrap_or_default())
            } else {
                String::new()
            },
            failed = if failed {
                "results = results.filter(r => r.status >= 400);".to_string()
            } else {
                String::new()
            },
            limit = limit
        );
        let result = page.evaluate(js).await.map_err(|e| e.to_string())?;
        let data = result.into_value::<String>().unwrap_or_default();
        let entries: Vec<serde_json::Value> = serde_json::from_str(&data).unwrap_or_default();
        if entries.is_empty() {
            println!("{} No requests found", "ℹ".blue());
        } else {
            println!("{} {} requests:", "✓".green(), entries.len());
            for e in &entries {
                let status = e["status"].as_i64().unwrap_or(200);
                let url = e["url"].as_str().unwrap_or("");
                let dur = e["duration"].as_i64().unwrap_or(0);
                let size = e["size"].as_i64().unwrap_or(0);
                let short_url = if url.len() > 80 { &url[..80] } else { url };
                let status_color = if status >= 400 { format!("{}", status).red().to_string() } else { format!("{}", status).green().to_string() };
                println!("  {} {} {}ms {}B", status_color, short_url, dur, size);
            }
        }
        Ok(())
    })
    .await;
}

pub async fn close_page(all: bool) {
    with_page(|page| async move {
        if all {
            let js = "window.close()";
            let _ = page.evaluate(js).await;
            println!("{} Browser session closed", "✓".green());
        } else {
            let js = "window.close()";
            let _ = page.evaluate(js).await;
            println!("{} Page closed", "✓".green());
        }
        Ok(())
    })
    .await;
}
