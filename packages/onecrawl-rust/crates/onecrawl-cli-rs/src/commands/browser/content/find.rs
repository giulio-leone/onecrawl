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
