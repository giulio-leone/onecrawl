use colored::Colorize;
use super::super::helpers::{with_page};

// ---------------------------------------------------------------------------
// Content
// ---------------------------------------------------------------------------

// ---------------------------------------------------------------------------
// Content Extraction
// ---------------------------------------------------------------------------

// ---------------------------------------------------------------------------
// Streaming Extractor
// ---------------------------------------------------------------------------

// ──────────────── Structured Data ────────────────

pub async fn get(what: &str, selector: Option<&str>, arg: Option<&str>) {
    let selector = selector.map(|s| onecrawl_cdp::accessibility::resolve_ref(s));
    let selector = selector.as_deref();
    // Proxy fast-path for simple content retrieval (no selector)
    if selector.is_none() {
        if let Some(proxy) = super::super::super::proxy::ServerProxy::from_session().await {
            match what {
                "text" => {
                    if let Ok(text) = proxy.get_text().await {
                        println!("{text}");
                        return;
                    }
                }
                "url" => {
                    if let Ok(val) = proxy.evaluate("window.location.href").await {
                        let url = val["result"].as_str().unwrap_or("");
                        println!("{url}");
                        return;
                    }
                }
                "title" => {
                    if let Ok(val) = proxy.evaluate("document.title").await {
                        let title = val["result"].as_str().unwrap_or("");
                        println!("{title}");
                        return;
                    }
                }
                "html" => {
                    if let Ok(val) = proxy.evaluate("document.documentElement.outerHTML").await {
                        let html = val["result"].as_str().unwrap_or("");
                        println!("{html}");
                        return;
                    }
                }
                _ => {}
            }
        }
    }
    with_page(|page| async move {
        match what {
            "url" => {
                let url = onecrawl_cdp::navigation::get_url(&page)
                    .await
                    .map_err(|e| e.to_string())?;
                println!("{url}");
            }
            "title" => {
                let title = onecrawl_cdp::navigation::get_title(&page)
                    .await
                    .map_err(|e| e.to_string())?;
                println!("{title}");
            }
            "html" => {
                if let Some(sel) = selector {
                    let val = onecrawl_cdp::page::evaluate_js(
                        &page,
                        &format!(
                            "document.querySelector('{}')?.outerHTML || ''",
                            sel.replace('\'', "\\'")
                        ),
                    )
                    .await
                    .map_err(|e| e.to_string())?;
                    println!("{}", val.as_str().unwrap_or(&val.to_string()));
                } else {
                    let html = onecrawl_cdp::page::get_content(&page)
                        .await
                        .map_err(|e| e.to_string())?;
                    println!("{html}");
                }
            }
            "text" => {
                if let Some(sel) = selector {
                    let text = onecrawl_cdp::element::get_text(&page, sel)
                        .await
                        .map_err(|e| e.to_string())?;
                    println!("{text}");
                } else {
                    let val =
                        onecrawl_cdp::page::evaluate_js(&page, "document.body?.innerText || ''")
                            .await
                            .map_err(|e| e.to_string())?;
                    println!("{}", val.as_str().unwrap_or(&val.to_string()));
                }
            }
            "value" => {
                let sel = selector.ok_or("get value requires a selector")?;
                let js = format!(
                    "document.querySelector('{}')?.value ?? ''",
                    sel.replace('\'', "\\'")
                );
                let val = onecrawl_cdp::page::evaluate_js(&page, &js)
                    .await.map_err(|e| e.to_string())?;
                println!("{}", val.as_str().unwrap_or(&val.to_string()));
            }
            "attr" => {
                let sel = selector.ok_or("get attr requires a selector")?;
                let attr_name = arg.ok_or("get attr requires an attribute name (3rd argument)")?;
                let js = format!(
                    "document.querySelector('{}')?.getAttribute('{}') ?? ''",
                    sel.replace('\'', "\\'"),
                    attr_name.replace('\'', "\\'")
                );
                let val = onecrawl_cdp::page::evaluate_js(&page, &js)
                    .await.map_err(|e| e.to_string())?;
                println!("{}", val.as_str().unwrap_or(&val.to_string()));
            }
            "count" => {
                let sel = selector.ok_or("get count requires a selector")?;
                let js = format!(
                    "document.querySelectorAll('{}').length",
                    sel.replace('\'', "\\'")
                );
                let val = onecrawl_cdp::page::evaluate_js(&page, &js)
                    .await.map_err(|e| e.to_string())?;
                println!("{}", val);
            }
            "styles" => {
                let sel = selector.ok_or("get styles requires a selector")?;
                let js = format!(
                    r#"(() => {{
                        const el = document.querySelector('{}');
                        if (!el) return '{{}}';
                        const s = getComputedStyle(el);
                        const o = {{}};
                        for (let i = 0; i < s.length; i++) {{
                            const p = s[i];
                            o[p] = s.getPropertyValue(p);
                        }}
                        return JSON.stringify(o);
                    }})()"#,
                    sel.replace('\'', "\\'")
                );
                let val = onecrawl_cdp::page::evaluate_js(&page, &js)
                    .await.map_err(|e| e.to_string())?;
                println!("{}", val.as_str().unwrap_or(&val.to_string()));
            }
            "box" => {
                let sel = selector.ok_or("get box requires a selector")?;
                let js = format!(
                    r#"(() => {{
                        const el = document.querySelector('{}');
                        if (!el) return 'null';
                        const r = el.getBoundingClientRect();
                        return JSON.stringify({{ x: r.x, y: r.y, width: r.width, height: r.height }});
                    }})()"#,
                    sel.replace('\'', "\\'")
                );
                let val = onecrawl_cdp::page::evaluate_js(&page, &js)
                    .await.map_err(|e| e.to_string())?;
                println!("{}", val.as_str().unwrap_or(&val.to_string()));
            }
            other => {
                return Err(format!(
                    "Unknown target: {other}. Use: text, html, url, title, value, attr, count, styles, box"
                ));
            }
        }
        Ok(())
    })
    .await;
}

pub async fn set_content(html: &str) {
    let html = html.to_string();
    with_page(|page| async move {
        onecrawl_cdp::page::set_content(&page, &html)
            .await
            .map_err(|e| e.to_string())?;
        println!("{} Content set", "✓".green());
        Ok(())
    })
    .await;
}

