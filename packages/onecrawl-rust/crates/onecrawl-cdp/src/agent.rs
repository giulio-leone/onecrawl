//! Autonomous agent loop — observe → plan → act → verify cycles.

use onecrawl_browser::Page;
use onecrawl_core::{Error, Result};
use serde_json::Value;

use crate::page::evaluate_js;

/// Execute a multi-step agent loop with observation and verification.
/// Returns the step-by-step execution trace.
pub async fn agent_loop(
    page: &Page,
    goal: &str,
    max_steps: usize,
    verify_js: Option<&str>,
) -> Result<Value> {
    let mut steps = Vec::new();

    for step_num in 0..max_steps {
        // Observe: get page state
        let url = page.url().await.ok().flatten().unwrap_or_default();
        let title = evaluate_js(page, "document.title").await?;
        let title_str = title.as_str().unwrap_or("").to_string();

        // Get interactive elements count
        let elems_js = r#"
            (() => {
                const interactive = document.querySelectorAll('a, button, input, select, textarea, [role="button"], [role="link"], [tabindex]');
                const forms = document.querySelectorAll('form');
                const visible = Array.from(interactive).filter(el => {
                    const rect = el.getBoundingClientRect();
                    return rect.width > 0 && rect.height > 0;
                });
                return {
                    total_interactive: interactive.length,
                    visible_interactive: visible.length,
                    forms: forms.length,
                    body_text_length: document.body?.innerText?.length || 0
                };
            })()
        "#;
        let observation = evaluate_js(page, elems_js).await?;

        // Verify: check if goal is met
        let mut verified = false;
        let mut verify_result = Value::Null;

        if let Some(js) = verify_js {
            let vr = evaluate_js(page, js).await?;
            let vr_str = match &vr {
                Value::Bool(b) => b.to_string(),
                Value::String(s) => s.clone(),
                other => other.to_string(),
            };
            verified = vr_str == "true" || vr_str.contains("true");
            verify_result = vr;
        }

        let step = serde_json::json!({
            "step": step_num + 1,
            "url": url,
            "title": title_str,
            "observation": observation,
            "goal": goal,
            "verified": verified,
            "verify_result": verify_result
        });
        steps.push(step);

        if verified {
            return Ok(serde_json::json!({
                "status": "goal_achieved",
                "total_steps": step_num + 1,
                "goal": goal,
                "steps": steps
            }));
        }
    }

    Ok(serde_json::json!({
        "status": "max_steps_reached",
        "total_steps": max_steps,
        "goal": goal,
        "steps": steps
    }))
}

/// Semantic goal verification: check URL, title, key elements.
pub async fn goal_assert(
    page: &Page,
    assertions: &[(&str, &str)],
) -> Result<Value> {
    let url = page.url().await.ok().flatten().unwrap_or_default();
    let title_val = evaluate_js(page, "document.title").await?;
    let title = title_val.as_str().unwrap_or("").to_string();

    let mut results = Vec::new();
    let mut all_passed = true;

    for (assertion_type, value) in assertions {
        let passed = match *assertion_type {
            "url_contains" => url.contains(value),
            "url_equals" => url == *value,
            "title_contains" => title.contains(value),
            "title_equals" => title == *value,
            "element_exists" => {
                let js = format!(
                    "document.querySelector('{}') !== null",
                    value.replace('\\', "\\\\").replace('\'', "\\'")
                );
                let r = evaluate_js(page, &js).await?;
                r.as_bool().unwrap_or(false)
            }
            "text_contains" => {
                let js = format!(
                    "document.body?.innerText?.includes('{}') || false",
                    value.replace('\\', "\\\\").replace('\'', "\\'")
                );
                let r = evaluate_js(page, &js).await?;
                r.as_bool().unwrap_or(false)
            }
            "element_visible" => {
                let js = format!(
                    r#"(() => {{
                        const el = document.querySelector('{}');
                        if (!el) return false;
                        const rect = el.getBoundingClientRect();
                        return rect.width > 0 && rect.height > 0;
                    }})()"#,
                    value.replace('\\', "\\\\").replace('\'', "\\'")
                );
                let r = evaluate_js(page, &js).await?;
                r.as_bool().unwrap_or(false)
            }
            _ => false,
        };

        if !passed {
            all_passed = false;
        }

        results.push(serde_json::json!({
            "type": assertion_type,
            "value": value,
            "passed": passed
        }));
    }

    Ok(serde_json::json!({
        "all_passed": all_passed,
        "assertions": results,
        "context": {
            "url": url,
            "title": title
        }
    }))
}

/// Annotated observation: get page state with element coordinates and bounding boxes.
pub async fn annotated_observe(page: &Page) -> Result<Value> {
    let js = r#"
        (() => {
            const elements = [];
            const interactive = document.querySelectorAll(
                'a, button, input, select, textarea, [role="button"], [role="link"], [role="checkbox"], [role="radio"], [role="combobox"], [role="menuitem"], [tabindex]:not([tabindex="-1"])'
            );

            let ref_counter = 0;
            interactive.forEach(el => {
                const rect = el.getBoundingClientRect();
                if (rect.width === 0 && rect.height === 0) return;

                ref_counter++;
                const ref_id = `@e${ref_counter}`;

                const tag = el.tagName.toLowerCase();
                const role = el.getAttribute('role') || '';
                const text = (el.innerText || el.textContent || '').trim().substring(0, 100);
                const ariaLabel = el.getAttribute('aria-label') || '';
                const placeholder = el.getAttribute('placeholder') || '';
                const type = el.getAttribute('type') || '';
                const href = el.getAttribute('href') || '';
                const name = el.getAttribute('name') || '';
                const id = el.id || '';
                const value = el.value || '';
                const disabled = el.disabled || false;
                const checked = el.checked || false;

                elements.push({
                    ref: ref_id,
                    tag,
                    role,
                    text,
                    aria_label: ariaLabel,
                    placeholder,
                    type,
                    href,
                    name,
                    id,
                    value,
                    disabled,
                    checked,
                    bounds: {
                        x: Math.round(rect.x),
                        y: Math.round(rect.y),
                        width: Math.round(rect.width),
                        height: Math.round(rect.height),
                        center_x: Math.round(rect.x + rect.width / 2),
                        center_y: Math.round(rect.y + rect.height / 2)
                    }
                });
            });

            const url = window.location.href;
            const title = document.title;
            const viewport = { width: window.innerWidth, height: window.innerHeight };
            const scroll = { x: window.scrollX, y: window.scrollY, max_y: document.documentElement.scrollHeight - window.innerHeight };

            return {
                url,
                title,
                viewport,
                scroll,
                elements,
                element_count: elements.length,
                timestamp: Date.now()
            };
        })()
    "#;

    evaluate_js(page, js).await
}

/// Store/retrieve session context in page window object
pub async fn session_context(
    page: &Page,
    command: &str,
    key: Option<&str>,
    value: Option<&str>,
) -> Result<Value> {
    let js = match command {
        "set" => {
            let k = key.unwrap_or("default");
            let v = value.unwrap_or("");
            format!(r#"
                (() => {{
                    if (!window.__onecrawl_ctx) window.__onecrawl_ctx = {{}};
                    window.__onecrawl_ctx['{}'] = '{}';
                    return JSON.stringify({{ action: 'set', key: '{}', stored: true }});
                }})()
            "#, k.replace('\\', "\\\\").replace('\'', "\\'"), v.replace('\\', "\\\\").replace('\'', "\\'"), k.replace('\\', "\\\\").replace('\'', "\\'"))
        }
        "get" => {
            let k = key.unwrap_or("default");
            format!(r#"
                (() => {{
                    if (!window.__onecrawl_ctx) return JSON.stringify({{ action: 'get', key: '{}', value: null }});
                    return JSON.stringify({{ action: 'get', key: '{}', value: window.__onecrawl_ctx['{}'] || null }});
                }})()
            "#, k.replace('\\', "\\\\").replace('\'', "\\'"), k.replace('\\', "\\\\").replace('\'', "\\'"), k.replace('\\', "\\\\").replace('\'', "\\'"))
        }
        "get_all" => {
            r#"
                (() => {
                    return JSON.stringify({ action: 'get_all', context: window.__onecrawl_ctx || {} });
                })()
            "#.to_string()
        }
        "clear" => {
            r#"
                (() => {
                    window.__onecrawl_ctx = {};
                    return JSON.stringify({ action: 'clear', cleared: true });
                })()
            "#.to_string()
        }
        _ => return Ok(serde_json::json!({"error": "unknown command"})),
    };

    let result = page.evaluate(js).await
        .map_err(|e| Error::Cdp(format!("session_context: {e}")))?;
    let raw: String = result.into_value().unwrap_or_else(|_| "{}".to_string());
    Ok(serde_json::from_str(&raw).unwrap_or(serde_json::json!({})))
}

/// Execute a chain of JS actions with error recovery
pub async fn auto_chain(
    page: &Page,
    actions: &[String],
    on_error: &str,
    max_retries: usize,
) -> Result<Value> {
    let mut results = Vec::new();

    for (i, action_js) in actions.iter().enumerate() {
        let mut success = false;
        let mut last_err = String::new();
        let mut attempts = 0;

        for attempt in 0..=max_retries {
            attempts = attempt + 1;
            match page.evaluate(action_js.to_string()).await {
                Ok(val) => {
                    let r: String = val.into_value().unwrap_or_else(|_| "null".to_string());
                    results.push(serde_json::json!({
                        "step": i + 1,
                        "status": "success",
                        "result": r,
                        "attempts": attempts
                    }));
                    success = true;
                    break;
                }
                Err(e) => {
                    last_err = e.to_string();
                    if on_error != "retry" || attempt == max_retries {
                        break;
                    }
                    tokio::time::sleep(std::time::Duration::from_millis(500)).await;
                }
            }
        }

        if !success {
            match on_error {
                "skip" => {
                    results.push(serde_json::json!({
                        "step": i + 1,
                        "status": "skipped",
                        "error": last_err,
                        "attempts": attempts
                    }));
                }
                "abort" => {
                    results.push(serde_json::json!({
                        "step": i + 1,
                        "status": "aborted",
                        "error": last_err,
                        "attempts": attempts
                    }));
                    return Ok(serde_json::json!({
                        "status": "aborted",
                        "completed_steps": i,
                        "total_steps": actions.len(),
                        "results": results
                    }));
                }
                _ => {
                    results.push(serde_json::json!({
                        "step": i + 1,
                        "status": "failed",
                        "error": last_err,
                        "attempts": attempts
                    }));
                }
            }
        }
    }

    let all_ok = results.iter().all(|r| r["status"] == "success");
    Ok(serde_json::json!({
        "status": if all_ok { "all_success" } else { "partial" },
        "completed_steps": results.len(),
        "total_steps": actions.len(),
        "results": results
    }))
}

/// Structured reasoning: observe and recommend actions
pub async fn think(page: &Page) -> Result<Value> {
    let js = r#"
        (() => {
            const state = {
                url: window.location.href,
                title: document.title,
                ready: document.readyState,
                scroll: { x: window.scrollX, y: window.scrollY, maxY: document.documentElement.scrollHeight - window.innerHeight },
                viewport: { w: window.innerWidth, h: window.innerHeight }
            };

            const buttons = document.querySelectorAll('button, [role="button"]');
            const links = document.querySelectorAll('a[href]');
            const inputs = document.querySelectorAll('input, textarea, select');
            const forms = document.querySelectorAll('form');

            const ctas = Array.from(buttons).filter(b => {
                const rect = b.getBoundingClientRect();
                return rect.width > 50 && rect.height > 20 && rect.top < window.innerHeight;
            }).map(b => ({
                text: (b.innerText || '').trim().substring(0, 50),
                tag: b.tagName.toLowerCase(),
                type: b.type || '',
                disabled: b.disabled
            })).slice(0, 10);

            const emptyInputs = Array.from(inputs).filter(i => {
                return i.required && !i.value && i.getBoundingClientRect().width > 0;
            }).map(i => ({
                name: i.name || i.id || i.placeholder || i.type,
                type: i.type
            })).slice(0, 10);

            const hasLogin = !!(document.querySelector('[type="password"]') || document.querySelector('form[action*="login"]'));
            const hasSearch = !!(document.querySelector('[type="search"]') || document.querySelector('[name="q"]'));
            const hasModal = !!(document.querySelector('[role="dialog"]') || document.querySelector('.modal.show'));
            const hasCaptcha = !!(document.querySelector('[class*="captcha"]') || document.querySelector('iframe[src*="captcha"]'));
            const isLoading = !!(document.querySelector('.loading, .spinner, [aria-busy="true"]'));

            const analysis = {
                page_type: hasLogin ? 'login_page' : hasSearch ? 'search_page' : hasModal ? 'modal_open' : 'content_page',
                state,
                interactive: {
                    buttons: buttons.length,
                    links: links.length,
                    inputs: inputs.length,
                    forms: forms.length
                },
                prominent_ctas: ctas,
                empty_required: emptyInputs,
                flags: { hasLogin, hasSearch, hasModal, hasCaptcha, isLoading },
                recommendations: []
            };

            if (hasCaptcha) analysis.recommendations.push({ action: 'solve_captcha', priority: 'high', reason: 'CAPTCHA detected' });
            if (hasModal) analysis.recommendations.push({ action: 'dismiss_modal', priority: 'high', reason: 'Modal blocking interaction' });
            if (isLoading) analysis.recommendations.push({ action: 'wait', priority: 'high', reason: 'Page still loading' });
            if (emptyInputs.length > 0) analysis.recommendations.push({ action: 'fill_form', priority: 'medium', reason: `${emptyInputs.length} required inputs empty` });
            if (hasLogin) analysis.recommendations.push({ action: 'authenticate', priority: 'medium', reason: 'Login form detected' });
            if (ctas.length > 0) analysis.recommendations.push({ action: 'click_cta', priority: 'low', reason: `${ctas.length} CTAs available` });
            if (state.scroll.maxY > 0 && state.scroll.y === 0) analysis.recommendations.push({ action: 'scroll_explore', priority: 'low', reason: 'Page has scrollable content' });

            return JSON.stringify(analysis);
        })()
    "#.to_string();

    let result = page.evaluate(js).await
        .map_err(|e| Error::Cdp(format!("think: {e}")))?;
    let raw: String = result.into_value().unwrap_or_else(|_| "{}".to_string());
    Ok(serde_json::from_str(&raw).unwrap_or(serde_json::json!({})))
}

/// Click at specific viewport coordinates with element feedback
pub async fn click_at_coords(page: &Page, x: f64, y: f64) -> Result<Value> {
    let identify_js = format!(r#"
        (() => {{
            const el = document.elementFromPoint({x}, {y});
            if (!el) return JSON.stringify({{ found: false }});
            return JSON.stringify({{
                found: true,
                tag: el.tagName.toLowerCase(),
                text: (el.innerText || '').trim().substring(0, 100),
                id: el.id || '',
                className: el.className?.toString?.() || '',
                href: el.getAttribute('href') || '',
                type: el.getAttribute('type') || '',
                role: el.getAttribute('role') || ''
            }});
        }})()
    "#);

    let identify_result = page.evaluate(identify_js).await
        .map_err(|e| Error::Cdp(format!("click_at_coords identify: {e}")))?;
    let element_info: String = identify_result.into_value().unwrap_or_else(|_| r#"{{"found":false}}"#.to_string());
    let element: Value = serde_json::from_str(&element_info).unwrap_or(serde_json::json!({"found": false}));

    let click_js = format!(r#"
        (() => {{
            const el = document.elementFromPoint({x}, {y});
            if (el) {{
                el.click();
                return 'clicked';
            }}
            return 'no_element';
        }})()
    "#);

    let click_result = page.evaluate(click_js).await
        .map_err(|e| Error::Cdp(format!("click_at_coords click: {e}")))?;
    let click_status: String = click_result.into_value().unwrap_or_else(|_| "error".to_string());

    Ok(serde_json::json!({
        "action": "click_at_coords",
        "x": x,
        "y": y,
        "clicked": click_status == "clicked",
        "element": element
    }))
}

/// Replay a sequence of input events
pub async fn input_replay(
    page: &Page,
    events: &[Value],
) -> Result<Value> {
    let mut results = Vec::new();

    for (i, event) in events.iter().enumerate() {
        let event_type = event["type"].as_str().unwrap_or("unknown");
        let result = match event_type {
            "click" => {
                let selector = event["selector"].as_str().unwrap_or("body");
                let js = format!(r#"
                    (() => {{
                        const el = document.querySelector('{}');
                        if (el) {{ el.click(); return 'clicked'; }}
                        return 'not_found';
                    }})()
                "#, selector.replace('\\', "\\\\").replace('\'', "\\'"));
                page.evaluate(js).await.map(|v| {
                    let s: String = v.into_value().unwrap_or_default();
                    serde_json::json!({"status": s})
                }).unwrap_or(serde_json::json!({"status": "error"}))
            }
            "type" => {
                let selector = event["selector"].as_str().unwrap_or("input");
                let text = event["text"].as_str().unwrap_or("");
                let js = format!(r#"
                    (() => {{
                        const el = document.querySelector('{}');
                        if (el) {{
                            el.focus();
                            el.value = '{}';
                            el.dispatchEvent(new Event('input', {{ bubbles: true }}));
                            el.dispatchEvent(new Event('change', {{ bubbles: true }}));
                            return 'typed';
                        }}
                        return 'not_found';
                    }})()
                "#, selector.replace('\\', "\\\\").replace('\'', "\\'"), text.replace('\\', "\\\\").replace('\'', "\\'"));
                page.evaluate(js).await.map(|v| {
                    let s: String = v.into_value().unwrap_or_default();
                    serde_json::json!({"status": s})
                }).unwrap_or(serde_json::json!({"status": "error"}))
            }
            "scroll" => {
                let sx = event["x"].as_f64().unwrap_or(0.0);
                let sy = event["y"].as_f64().unwrap_or(0.0);
                let js = format!("window.scrollBy({}, {}); 'scrolled'", sx, sy);
                page.evaluate(js).await.map(|v| {
                    let s: String = v.into_value().unwrap_or_default();
                    serde_json::json!({"status": s})
                }).unwrap_or(serde_json::json!({"status": "error"}))
            }
            "wait" => {
                let ms = event["ms"].as_u64().unwrap_or(1000);
                tokio::time::sleep(std::time::Duration::from_millis(ms)).await;
                serde_json::json!({"status": "waited", "ms": ms})
            }
            _ => serde_json::json!({"status": "unknown_event_type"})
        };

        results.push(serde_json::json!({
            "step": i + 1,
            "type": event_type,
            "result": result
        }));
    }

    Ok(serde_json::json!({
        "action": "input_replay",
        "total_events": events.len(),
        "results": results
    }))
}
