use colored::Colorize;
use super::helpers::{with_page};

// ---------------------------------------------------------------------------
// Wait
// ---------------------------------------------------------------------------

pub async fn navigate(url: &str, wait: u64, wait_cf: bool) {
    let t0 = std::time::Instant::now();
    // Try proxy first (avoids CDP reconnect overhead)
    if let Some(proxy) = super::super::proxy::ServerProxy::from_session().await
        && proxy.navigate(url).await.is_ok()
    {
        if wait > 0 {
            tokio::time::sleep(std::time::Duration::from_millis(wait)).await;
        }
        let ms = t0.elapsed().as_millis();
        println!("{} Navigated to {} {} {}", "✓".green(), url.cyan(), format!("{ms}ms").dimmed(), "(proxy)".dimmed());
        return;
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

pub async fn wait_for_text(text: &str, timeout: u64) {
    let text = text.to_string();
    with_page(|page| async move {
        let js = format!(
            r#"new Promise((resolve, reject) => {{
                const t = setTimeout(() => reject(new Error('timeout')), {timeout});
                const check = () => {{
                    if (document.body && document.body.innerText.includes({text})) {{
                        clearTimeout(t); resolve(true);
                    }} else {{ requestAnimationFrame(check); }}
                }};
                check();
            }})"#,
            timeout = timeout,
            text = serde_json::to_string(&text).unwrap_or_default()
        );
        page.evaluate(js).await.map_err(|e| e.to_string())?;
        println!("{} Text found: '{}'", "✓".green(), &text[..text.len().min(40)]);
        Ok(())
    })
    .await;
}

pub async fn wait_for_load(state: &str, timeout: u64) {
    let state = state.to_string();
    with_page(|page| async move {
        let js = match state.as_str() {
            "networkidle" => format!(
                r#"new Promise((resolve, reject) => {{
                    const t = setTimeout(() => reject(new Error('timeout')), {timeout});
                    let idle = setTimeout(() => {{ clearTimeout(t); resolve(true); }}, 500);
                    const obs = new PerformanceObserver(() => {{ clearTimeout(idle); idle = setTimeout(() => {{ clearTimeout(t); resolve(true); }}, 500); }});
                    try {{ obs.observe({{ type: 'resource', buffered: false }}); }} catch(e) {{}}
                }})"#,
                timeout = timeout
            ),
            "load" | "domcontentloaded" => {
                let event = if state == "load" { "load" } else { "DOMContentLoaded" };
                format!(
                    r#"new Promise((resolve, reject) => {{
                        const t = setTimeout(() => reject(new Error('timeout')), {timeout});
                        if (document.readyState === 'complete' || ('{event}' === 'DOMContentLoaded' && document.readyState !== 'loading')) {{
                            clearTimeout(t); resolve(true);
                        }} else {{
                            window.addEventListener('{event}', () => {{ clearTimeout(t); resolve(true); }}, {{ once: true }});
                        }}
                    }})"#,
                    timeout = timeout, event = event
                )
            }
            _ => { eprintln!("❌ Unknown load state: {state}. Use: load, domcontentloaded, networkidle"); return Ok(()); }
        };
        page.evaluate(js).await.map_err(|e| e.to_string())?;
        println!("{} Load state '{}' reached", "✓".green(), state);
        Ok(())
    })
    .await;
}

pub async fn wait_for_function(expression: &str, timeout: u64) {
    let expr = expression.to_string();
    with_page(|page| async move {
        let js = format!(
            r#"new Promise((resolve, reject) => {{
                const t = setTimeout(() => reject(new Error('timeout')), {timeout});
                const check = () => {{
                    try {{ if ({expr}) {{ clearTimeout(t); resolve(true); return; }} }} catch(e) {{}}
                    requestAnimationFrame(check);
                }};
                check();
            }})"#,
            timeout = timeout, expr = expr
        );
        page.evaluate(js).await.map_err(|e| e.to_string())?;
        println!("{} JS condition met", "✓".green());
        Ok(())
    })
    .await;
}
