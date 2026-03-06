//! Smart element resolution — multi-strategy element discovery and interaction.
//!
//! Finds the best matching DOM element using exact text, fuzzy text,
//! ARIA roles, attribute matching, and CSS selector strategies.

use chromiumoxide::Page;
use onecrawl_core::{Error, Result};
use serde::{Deserialize, Serialize};

/// A single element match with confidence score and resolution strategy.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SmartMatch {
    pub selector: String,
    pub confidence: f64,
    pub strategy: String,
    pub ref_id: Option<String>,
}

/// Smart element resolution — finds the best match using multiple strategies.
pub async fn smart_find(page: &Page, query: &str) -> Result<Vec<SmartMatch>> {
    let js = format!(
        r#"
        (() => {{
            const query = {query_json};
            const results = [];

            // Strategy 1: Exact text match (highest confidence)
            const exactText = Array.from(document.querySelectorAll(
                'button, a, [role="button"], [role="link"], input[type="submit"]'
            )).filter(el => {{
                const text = (el.textContent || el.value || el.getAttribute('aria-label') || '').trim();
                return text.toLowerCase() === query.toLowerCase();
            }});
            exactText.forEach(el => {{
                results.push({{
                    selector: buildSelector(el),
                    confidence: 1.0,
                    strategy: 'exact_text',
                    ref_id: el.getAttribute('data-onecrawl-ref') || null
                }});
            }});

            // Strategy 2: Fuzzy text match (contains)
            if (results.length === 0) {{
                const fuzzy = Array.from(document.querySelectorAll(
                    'button, a, [role="button"], [role="link"], label, input, select, textarea'
                )).filter(el => {{
                    const text = (el.textContent || el.value || el.getAttribute('aria-label')
                        || el.getAttribute('placeholder') || '').trim().toLowerCase();
                    return text.includes(query.toLowerCase()) || query.toLowerCase().includes(text);
                }});
                fuzzy.forEach(el => {{
                    const text = (el.textContent || el.value || el.getAttribute('aria-label') || '').trim();
                    const similarity = Math.min(query.length, text.length) / Math.max(query.length, text.length);
                    results.push({{
                        selector: buildSelector(el),
                        confidence: 0.5 + (similarity * 0.3),
                        strategy: 'fuzzy_text',
                        ref_id: el.getAttribute('data-onecrawl-ref') || null
                    }});
                }});
            }}

            // Strategy 3: ARIA role match
            const ariaRoles = ['button', 'link', 'textbox', 'checkbox', 'radio',
                               'combobox', 'tab', 'menuitem', 'switch'];
            if (ariaRoles.includes(query.toLowerCase())) {{
                const roleEls = document.querySelectorAll(
                    query.toLowerCase() + ', [role="' + query.toLowerCase() + '"]'
                );
                roleEls.forEach(el => {{
                    results.push({{
                        selector: buildSelector(el),
                        confidence: 0.6,
                        strategy: 'aria_role',
                        ref_id: el.getAttribute('data-onecrawl-ref') || null
                    }});
                }});
            }}

            // Strategy 4: Attribute match (placeholder, name, id, title, alt)
            if (results.length === 0) {{
                const attrs = ['placeholder', 'name', 'id', 'title', 'alt', 'aria-label'];
                attrs.forEach(attr => {{
                    const els = document.querySelectorAll('[' + attr + ']');
                    els.forEach(el => {{
                        const val = (el.getAttribute(attr) || '').toLowerCase();
                        if (val.includes(query.toLowerCase()) || query.toLowerCase().includes(val)) {{
                            const similarity = Math.min(query.length, val.length) / Math.max(query.length, val.length);
                            results.push({{
                                selector: buildSelector(el),
                                confidence: 0.4 + (similarity * 0.3),
                                strategy: 'attribute_match',
                                ref_id: el.getAttribute('data-onecrawl-ref') || null
                            }});
                        }}
                    }});
                }});
            }}

            // Strategy 5: CSS selector (if query looks like a selector)
            if (query.startsWith('.') || query.startsWith('#') || query.includes('[')) {{
                try {{
                    const el = document.querySelector(query);
                    if (el) {{
                        results.push({{
                            selector: query,
                            confidence: 0.95,
                            strategy: 'css_selector',
                            ref_id: el.getAttribute('data-onecrawl-ref') || null
                        }});
                    }}
                }} catch(_e) {{}}
            }}

            function buildSelector(el) {{
                if (el.id) return '#' + el.id;
                const testId = el.getAttribute('data-testid');
                if (testId) return '[data-testid="' + testId + '"]';
                const ref_val = el.getAttribute('data-onecrawl-ref');
                if (ref_val) return '[data-onecrawl-ref="' + ref_val + '"]';
                const tag = el.tagName.toLowerCase();
                const text = (el.textContent || '').trim().substring(0, 30);
                if (text) return tag + ':has-text("' + text + '")';
                return tag;
            }}

            // Deduplicate and sort by confidence
            const seen = new Set();
            return results
                .filter(r => {{
                    if (seen.has(r.selector)) return false;
                    seen.add(r.selector);
                    return true;
                }})
                .sort((a, b) => b.confidence - a.confidence)
                .slice(0, 10);
        }})()
        "#,
        query_json = serde_json::to_string(query).unwrap_or_default()
    );

    let result = page
        .evaluate(js)
        .await
        .map_err(|e| Error::Cdp(format!("smart_find failed: {e}")))?;

    result
        .into_value::<Vec<SmartMatch>>()
        .map_err(|e| Error::Cdp(format!("smart_find parse failed: {e}")))
}

/// Smart click — tries multiple strategies to click the right element.
pub async fn smart_click(page: &Page, query: &str) -> Result<SmartMatch> {
    let matches = smart_find(page, query).await?;
    if matches.is_empty() {
        return Err(Error::Cdp(format!(
            "smart_click: no match found for '{}'. Try a more specific query.",
            query
        )));
    }

    let best = &matches[0];
    let resolved = if let Some(ref r) = best.ref_id {
        crate::accessibility::resolve_ref(&format!("@{r}"))
    } else {
        best.selector.clone()
    };

    crate::element::click(page, &resolved).await?;
    Ok(best.clone())
}

/// Smart fill — finds an input and types into it.
pub async fn smart_fill(page: &Page, query: &str, value: &str) -> Result<SmartMatch> {
    let matches = smart_find(page, query).await?;
    if matches.is_empty() {
        return Err(Error::Cdp(format!(
            "smart_fill: no input found for '{}'. Try a more specific query.",
            query
        )));
    }

    let best = &matches[0];
    let resolved = if let Some(ref r) = best.ref_id {
        crate::accessibility::resolve_ref(&format!("@{r}"))
    } else {
        best.selector.clone()
    };

    crate::element::type_text(page, &resolved, value).await?;
    Ok(best.clone())
}
