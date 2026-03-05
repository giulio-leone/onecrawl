use colored::Colorize;
use super::helpers::{with_page};

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
        if let Some(proxy) = super::super::proxy::ServerProxy::from_session().await {
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

pub async fn eval(expression: &str) {
    // Try proxy first
    if let Some(proxy) = super::super::proxy::ServerProxy::from_session().await {
        if let Ok(val) = proxy.evaluate(expression).await {
            let result = &val["result"];
            match result {
                serde_json::Value::String(s) => println!("{s}"),
                serde_json::Value::Null => println!("undefined"),
                other => println!(
                    "{}",
                    serde_json::to_string_pretty(other).unwrap_or_default()
                ),
            }
            return;
        }
    }
    with_page(|page| async move {
        let val = onecrawl_cdp::page::evaluate_js(&page, expression)
            .await
            .map_err(|e| e.to_string())?;
        match &val {
            serde_json::Value::String(s) => println!("{s}"),
            serde_json::Value::Null => println!("undefined"),
            other => println!(
                "{}",
                serde_json::to_string_pretty(other).unwrap_or_default()
            ),
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

pub async fn extract_content(format: &str, selector: Option<&str>, output: Option<&str>) {
    let format = format.to_string();
    let selector = selector.map(|s| onecrawl_cdp::accessibility::resolve_ref(s));
    let output = output.map(String::from);
    with_page(|page| async move {
        let fmt =
            onecrawl_cdp::extract::parse_extract_format(&format).map_err(|e| e.to_string())?;

        if let Some(path) = output {
            let bytes = onecrawl_cdp::extract::extract_to_file(
                &page,
                selector.as_deref(),
                std::path::Path::new(&path),
            )
            .await
            .map_err(|e| e.to_string())?;
            println!("{} Extracted {} bytes to {}", "✓".green(), bytes, path);
        } else {
            let result = onecrawl_cdp::extract::extract(&page, selector.as_deref(), fmt)
                .await
                .map_err(|e| e.to_string())?;
            println!("{}", result.content);
        }
        Ok(())
    })
    .await;
}

pub async fn extract_metadata() {
    with_page(|page| async move {
        let meta = onecrawl_cdp::extract::get_page_metadata(&page)
            .await
            .map_err(|e| e.to_string())?;
        println!(
            "{}",
            serde_json::to_string_pretty(&meta).unwrap_or_default()
        );
        Ok(())
    })
    .await;
}

pub async fn stream_extract(
    item_selector: &str,
    fields: &[String],
    paginate: Option<&str>,
    max_pages: usize,
    output: Option<&str>,
    format: &str,
) {
    let fields = fields.to_vec();
    let item_selector = onecrawl_cdp::accessibility::resolve_ref(item_selector);
    let paginate = paginate.map(String::from);
    let output = output.map(String::from);
    let format = format.to_string();

    with_page(|page| async move {
        let rules: Vec<onecrawl_cdp::ExtractionRule> = fields
            .iter()
            .map(|f| onecrawl_cdp::streaming::parse_field_spec(f).map_err(|e| e.to_string()))
            .collect::<Result<Vec<_>, _>>()?;

        let pagination = paginate.map(|sel| onecrawl_cdp::PaginationConfig {
            next_selector: sel,
            max_pages,
            delay_ms: 1000,
        });

        let schema = onecrawl_cdp::ExtractionSchema {
            item_selector,
            fields: rules,
            pagination,
        };

        let result = onecrawl_cdp::streaming::extract_with_pagination(&page, &schema)
            .await
            .map_err(|e| e.to_string())?;

        if let Some(path) = output {
            let count = match format.as_str() {
                "csv" => {
                    onecrawl_cdp::streaming::export_csv(&result.items, std::path::Path::new(&path))
                        .map_err(|e| e.to_string())?
                }
                _ => {
                    onecrawl_cdp::streaming::export_json(&result.items, std::path::Path::new(&path))
                        .map_err(|e| e.to_string())?
                }
            };
            println!("{} Exported {} items to {}", "✓".green(), count, path);
        } else {
            println!(
                "{}",
                serde_json::to_string_pretty(&result).unwrap_or_default()
            );
        }

        if !result.errors.is_empty() {
            for err in &result.errors {
                eprintln!("{} {}", "⚠".yellow(), err);
            }
        }
        Ok(())
    })
    .await;
}

pub async fn structured_extract_all() {
    with_page(|page| async move {
        let data = onecrawl_cdp::structured_data::extract_all(&page)
            .await
            .map_err(|e| e.to_string())?;
        println!(
            "{}",
            serde_json::to_string_pretty(&data).unwrap_or_default()
        );
        Ok(())
    })
    .await;
}

pub async fn structured_json_ld() {
    with_page(|page| async move {
        let data = onecrawl_cdp::structured_data::extract_json_ld(&page)
            .await
            .map_err(|e| e.to_string())?;
        println!(
            "{}",
            serde_json::to_string_pretty(&data).unwrap_or_default()
        );
        Ok(())
    })
    .await;
}

pub async fn structured_open_graph() {
    with_page(|page| async move {
        let data = onecrawl_cdp::structured_data::extract_open_graph(&page)
            .await
            .map_err(|e| e.to_string())?;
        println!(
            "{}",
            serde_json::to_string_pretty(&data).unwrap_or_default()
        );
        Ok(())
    })
    .await;
}

pub async fn structured_twitter_card() {
    with_page(|page| async move {
        let data = onecrawl_cdp::structured_data::extract_twitter_card(&page)
            .await
            .map_err(|e| e.to_string())?;
        println!(
            "{}",
            serde_json::to_string_pretty(&data).unwrap_or_default()
        );
        Ok(())
    })
    .await;
}

pub async fn structured_metadata() {
    with_page(|page| async move {
        let data = onecrawl_cdp::structured_data::extract_metadata(&page)
            .await
            .map_err(|e| e.to_string())?;
        println!(
            "{}",
            serde_json::to_string_pretty(&data).unwrap_or_default()
        );
        Ok(())
    })
    .await;
}

pub fn structured_validate(data_json: &str) {
    let data: onecrawl_cdp::StructuredDataResult = match serde_json::from_str(data_json) {
        Ok(d) => d,
        Err(e) => {
            eprintln!("{} Invalid data JSON: {e}", "✗".red());
            std::process::exit(1);
        }
    };
    let warnings = onecrawl_cdp::structured_data::validate_schema(&data);
    if warnings.is_empty() {
        println!("{} Structured data is complete", "✓".green());
    } else {
        println!("{} {} warning(s):", "⚠".yellow(), warnings.len());
        for w in &warnings {
            println!("  - {w}");
        }
    }
}

// ---------------------------------------------------------------------------
// Find (Semantic Locators)
// ---------------------------------------------------------------------------

pub async fn find_action(action: crate::cli::FindAction) {
    use crate::cli::FindAction;

    // Build the JS locator expression based on the find variant
    let (locator_js, action_name, action_value) = match &action {
        FindAction::Role { role, action, value, name, exact } => {
            let mut js = format!(
                r#"(() => {{
                    const els = document.querySelectorAll('[role="{role}"]');
                    let arr = Array.from(els);"#,
                role = role
            );
            if let Some(n) = name {
                if *exact {
                    js += &format!(
                        r#"arr = arr.filter(el => (el.getAttribute('aria-label') || el.textContent?.trim() || '') === {n});"#,
                        n = serde_json::to_string(n).unwrap_or_default()
                    );
                } else {
                    js += &format!(
                        r#"arr = arr.filter(el => (el.getAttribute('aria-label') || el.textContent?.trim() || '').includes({n}));"#,
                        n = serde_json::to_string(n).unwrap_or_default()
                    );
                }
            }
            js += "return arr[0] || null; })()";
            (js, action.clone(), value.clone())
        }
        FindAction::Text { text, action, value, exact } => {
            let js = if *exact {
                format!(
                    r#"(() => {{
                        const tw = document.createTreeWalker(document.body, NodeFilter.SHOW_TEXT);
                        while(tw.nextNode()) {{
                            if (tw.currentNode.textContent?.trim() === {t}) return tw.currentNode.parentElement;
                        }}
                        return null;
                    }})()"#,
                    t = serde_json::to_string(text).unwrap_or_default()
                )
            } else {
                format!(
                    r#"(() => {{
                        const tw = document.createTreeWalker(document.body, NodeFilter.SHOW_TEXT);
                        while(tw.nextNode()) {{
                            if (tw.currentNode.textContent?.includes({t})) return tw.currentNode.parentElement;
                        }}
                        return null;
                    }})()"#,
                    t = serde_json::to_string(text).unwrap_or_default()
                )
            };
            (js, action.clone(), value.clone())
        }
        FindAction::Label { label, action, value } => {
            let js = format!(
                r#"(() => {{
                    const lbl = Array.from(document.querySelectorAll('label')).find(l => l.textContent?.includes({l}));
                    if (!lbl) return null;
                    const forId = lbl.getAttribute('for');
                    return forId ? document.getElementById(forId) : lbl.querySelector('input,select,textarea');
                }})()"#,
                l = serde_json::to_string(label).unwrap_or_default()
            );
            (js, action.clone(), value.clone())
        }
        FindAction::Placeholder { placeholder, action, value } => {
            let js = format!(
                "document.querySelector('[placeholder*={}]')",
                serde_json::to_string(placeholder).unwrap_or_default()
            );
            (js, action.clone(), value.clone())
        }
        FindAction::Alt { alt, action } => {
            let js = format!(
                "document.querySelector('[alt*={}]')",
                serde_json::to_string(alt).unwrap_or_default()
            );
            (js, action.clone(), None)
        }
        FindAction::Title { title, action } => {
            let js = format!(
                "document.querySelector('[title*={}]')",
                serde_json::to_string(title).unwrap_or_default()
            );
            (js, action.clone(), None)
        }
        FindAction::TestId { testid, action, value } => {
            let js = format!(
                "document.querySelector('[data-testid={}]')",
                serde_json::to_string(testid).unwrap_or_default()
            );
            (js, action.clone(), value.clone())
        }
        FindAction::First { selector, action, value } => {
            let js = format!(
                "document.querySelector({})",
                serde_json::to_string(selector).unwrap_or_default()
            );
            (js, action.clone(), value.clone())
        }
        FindAction::Last { selector, action, value } => {
            let js = format!(
                "(() => {{ const a = document.querySelectorAll({}); return a[a.length-1] || null; }})()",
                serde_json::to_string(selector).unwrap_or_default()
            );
            (js, action.clone(), value.clone())
        }
        FindAction::Nth { n, selector, action, value } => {
            let js = format!(
                "document.querySelectorAll({})[{}] || null",
                serde_json::to_string(selector).unwrap_or_default(), n
            );
            (js, action.clone(), value.clone())
        }
    };

    // Execute: find element, then perform action
    with_page(|page| async move {
        // Build the combined JS that finds element and performs action
        let action_js = match action_name.as_str() {
            "click" => format!(
                r#"(() => {{ const el = {loc}; if(!el) throw new Error('Element not found'); el.click(); return 'clicked'; }})()"#,
                loc = locator_js
            ),
            "fill" => {
                let val = action_value.as_deref().ok_or("fill requires a value")?;
                format!(
                    r#"(() => {{ const el = {loc}; if(!el) throw new Error('Element not found');
                    el.focus(); el.value = ''; document.execCommand('selectAll');
                    document.execCommand('insertText', false, {v}); el.dispatchEvent(new Event('input', {{bubbles:true}}));
                    return 'filled'; }})()"#,
                    loc = locator_js, v = serde_json::to_string(val).unwrap_or_default()
                )
            }
            "type" => {
                let val = action_value.as_deref().ok_or("type requires a value")?;
                format!(
                    r#"(async () => {{ const el = {loc}; if(!el) throw new Error('Element not found');
                    el.focus(); const t = {v};
                    for (const ch of t) {{ document.execCommand('insertText', false, ch); await new Promise(r=>setTimeout(r,20)); }}
                    return 'typed'; }})()"#,
                    loc = locator_js, v = serde_json::to_string(val).unwrap_or_default()
                )
            }
            "hover" => format!(
                r#"(() => {{ const el = {loc}; if(!el) throw new Error('Element not found');
                el.dispatchEvent(new MouseEvent('mouseenter', {{bubbles:true}}));
                el.dispatchEvent(new MouseEvent('mouseover', {{bubbles:true}}));
                return 'hovered'; }})()"#,
                loc = locator_js
            ),
            "focus" => format!(
                r#"(() => {{ const el = {loc}; if(!el) throw new Error('Element not found'); el.focus(); return 'focused'; }})()"#,
                loc = locator_js
            ),
            "check" => format!(
                r#"(() => {{ const el = {loc}; if(!el) throw new Error('Element not found');
                if (!el.checked) el.click(); return 'checked'; }})()"#,
                loc = locator_js
            ),
            "uncheck" => format!(
                r#"(() => {{ const el = {loc}; if(!el) throw new Error('Element not found');
                if (el.checked) el.click(); return 'unchecked'; }})()"#,
                loc = locator_js
            ),
            "text" => format!(
                r#"(() => {{ const el = {loc}; if(!el) throw new Error('Element not found');
                return el.textContent?.trim() || ''; }})()"#,
                loc = locator_js
            ),
            other => return Err(format!("Unknown find action: {other}. Use: click, fill, type, hover, focus, check, uncheck, text")),
        };
        let v = page.evaluate(action_js).await.map_err(|e| e.to_string())?;
        let result = v.into_value::<String>().unwrap_or_default();
        println!("{result}");
        Ok(())
    })
    .await;
}
