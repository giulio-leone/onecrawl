//! Autonomous agent loop — observe → plan → act → verify cycles.

use chromiumoxide::Page;
use onecrawl_core::Result;
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
                    value.replace('\'', "\\'")
                );
                let r = evaluate_js(page, &js).await?;
                r.as_bool().unwrap_or(false)
            }
            "text_contains" => {
                let js = format!(
                    "document.body?.innerText?.includes('{}') || false",
                    value.replace('\'', "\\'")
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
                    value.replace('\'', "\\'")
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
