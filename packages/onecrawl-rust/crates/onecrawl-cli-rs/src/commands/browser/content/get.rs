use colored::Colorize;
use super::super::helpers::{with_page};

// ---------------------------------------------------------------------------
// Streaming Extractor
// ---------------------------------------------------------------------------

pub async fn get(what: &str, selector: Option<&str>, arg: Option<&str>) {
    let selector = selector.map(onecrawl_cdp::accessibility::resolve_ref);
    let selector = selector.as_deref();
    if selector.is_none()
        && let Some(proxy) = super::super::super::proxy::ServerProxy::from_session().await
        && let Some(result) = try_proxy_get(what, &proxy).await {
            println!("{result}");
            return;
        }
    with_page(|page| async move {
        let output = get_from_page(what, selector, arg, &page).await?;
        println!("{output}");
        Ok(())
    })
    .await;
}

async fn try_proxy_get(what: &str, proxy: &super::super::super::proxy::ServerProxy) -> Option<String> {
    match what {
        "text" => proxy.get_text().await.ok(),
        "url" => proxy.evaluate("window.location.href").await.ok()
            .and_then(|v| v["result"].as_str().map(String::from)),
        "title" => proxy.evaluate("document.title").await.ok()
            .and_then(|v| v["result"].as_str().map(String::from)),
        "html" => proxy.evaluate("document.documentElement.outerHTML").await.ok()
            .and_then(|v| v["result"].as_str().map(String::from)),
        _ => None,
    }
}

async fn get_from_page(
    what: &str,
    selector: Option<&str>,
    arg: Option<&str>,
    page: &onecrawl_cdp::Page,
) -> std::result::Result<String, String> {
    match what {
        "url" => onecrawl_cdp::navigation::get_url(page).await.map_err(|e| e.to_string()),
        "title" => onecrawl_cdp::navigation::get_title(page).await.map_err(|e| e.to_string()),
        "html" => {
            if let Some(sel) = selector {
                let js = format!("document.querySelector('{}')?.outerHTML || ''", sel.replace('\'', "\\'"));
                let val = onecrawl_cdp::page::evaluate_js(page, &js).await.map_err(|e| e.to_string())?;
                Ok(val.as_str().unwrap_or(&val.to_string()).to_string())
            } else {
                onecrawl_cdp::page::get_content(page).await.map_err(|e| e.to_string())
            }
        }
        "text" => {
            if let Some(sel) = selector {
                onecrawl_cdp::element::get_text(page, sel).await.map_err(|e| e.to_string())
            } else {
                let val = onecrawl_cdp::page::evaluate_js(page, "document.body?.innerText || ''")
                    .await.map_err(|e| e.to_string())?;
                Ok(val.as_str().unwrap_or(&val.to_string()).to_string())
            }
        }
        "value" | "attr" | "count" | "styles" | "box" => {
            get_element_property(what, selector, arg, page).await
        }
        other => Err(format!("Unknown target: {other}. Use: text, html, url, title, value, attr, count, styles, box")),
    }
}

async fn get_element_property(
    what: &str,
    selector: Option<&str>,
    arg: Option<&str>,
    page: &onecrawl_cdp::Page,
) -> std::result::Result<String, String> {
    let js = match what {
        "value" => {
            let sel = selector.ok_or("get value requires a selector")?;
            format!("document.querySelector('{}')?.value ?? ''", sel.replace('\'', "\\'"))
        }
        "attr" => {
            let sel = selector.ok_or("get attr requires a selector")?;
            let attr_name = arg.ok_or("get attr requires an attribute name (3rd argument)")?;
            format!("document.querySelector('{}')?.getAttribute('{}') ?? ''",
                sel.replace('\'', "\\'"), attr_name.replace('\'', "\\'"))
        }
        "count" => {
            let sel = selector.ok_or("get count requires a selector")?;
            format!("document.querySelectorAll('{}').length", sel.replace('\'', "\\'"))
        }
        "styles" => {
            let sel = selector.ok_or("get styles requires a selector")?;
            format!(
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
            )
        }
        "box" => {
            let sel = selector.ok_or("get box requires a selector")?;
            format!(
                r#"(() => {{
                    const el = document.querySelector('{}');
                    if (!el) return 'null';
                    const r = el.getBoundingClientRect();
                    return JSON.stringify({{ x: r.x, y: r.y, width: r.width, height: r.height }});
                }})()"#,
                sel.replace('\'', "\\'")
            )
        }
        _ => unreachable!(),
    };
    let val = onecrawl_cdp::page::evaluate_js(page, &js).await.map_err(|e| e.to_string())?;
    Ok(val.as_str().unwrap_or(&val.to_string()).to_string())
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

