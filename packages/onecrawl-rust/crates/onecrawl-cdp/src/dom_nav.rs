//! Rich DOM traversal — parent, siblings, children, above, below (Scrapling-like).

use chromiumoxide::Page;
use onecrawl_core::{Error, Result};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NavElement {
    pub tag: String,
    pub text: String,
    pub html: String,
    pub attributes: std::collections::HashMap<String, String>,
}

/// Get parent element.
pub async fn get_parent(page: &Page, selector: &str) -> Result<Option<NavElement>> {
    let js = format!(
        r#"
        (() => {{
            const el = document.querySelector('{}');
            if (!el || !el.parentElement) return null;
            const p = el.parentElement;
            return {{
                tag: p.tagName.toLowerCase(),
                text: p.textContent?.substring(0, 500) || '',
                html: p.outerHTML?.substring(0, 2000) || '',
                attributes: Object.fromEntries(Array.from(p.attributes || []).map(a => [a.name, a.value]))
            }};
        }})()
    "#,
        selector.replace('\'', "\\'")
    );
    let val = page.evaluate(js).await.map_err(|e| Error::Browser(e.to_string()))?;
    let v = val.into_value().unwrap_or(serde_json::json!(null));
    if v.is_null() {
        return Ok(None);
    }
    Ok(Some(serde_json::from_value(v)?))
}

/// Get all children elements.
pub async fn get_children(page: &Page, selector: &str) -> Result<Vec<NavElement>> {
    nav_query(page, selector, "Array.from(el.children)").await
}

/// Get next sibling element.
pub async fn get_next_sibling(page: &Page, selector: &str) -> Result<Option<NavElement>> {
    let js = format!(
        r#"
        (() => {{
            const el = document.querySelector('{}');
            if (!el) return null;
            let sib = el.nextElementSibling;
            if (!sib) return null;
            return {{
                tag: sib.tagName.toLowerCase(),
                text: sib.textContent?.substring(0, 500) || '',
                html: sib.outerHTML?.substring(0, 2000) || '',
                attributes: Object.fromEntries(Array.from(sib.attributes || []).map(a => [a.name, a.value]))
            }};
        }})()
    "#,
        selector.replace('\'', "\\'")
    );
    let val = page.evaluate(js).await.map_err(|e| Error::Browser(e.to_string()))?;
    let v = val.into_value().unwrap_or(serde_json::json!(null));
    if v.is_null() {
        return Ok(None);
    }
    Ok(Some(serde_json::from_value(v)?))
}

/// Get previous sibling element.
pub async fn get_prev_sibling(page: &Page, selector: &str) -> Result<Option<NavElement>> {
    let js = format!(
        r#"
        (() => {{
            const el = document.querySelector('{}');
            if (!el) return null;
            let sib = el.previousElementSibling;
            if (!sib) return null;
            return {{
                tag: sib.tagName.toLowerCase(),
                text: sib.textContent?.substring(0, 500) || '',
                html: sib.outerHTML?.substring(0, 2000) || '',
                attributes: Object.fromEntries(Array.from(sib.attributes || []).map(a => [a.name, a.value]))
            }};
        }})()
    "#,
        selector.replace('\'', "\\'")
    );
    let val = page.evaluate(js).await.map_err(|e| Error::Browser(e.to_string()))?;
    let v = val.into_value().unwrap_or(serde_json::json!(null));
    if v.is_null() {
        return Ok(None);
    }
    Ok(Some(serde_json::from_value(v)?))
}

/// Get all sibling elements.
pub async fn get_siblings(page: &Page, selector: &str) -> Result<Vec<NavElement>> {
    nav_query(
        page,
        selector,
        "Array.from(el.parentElement?.children || []).filter(s => s !== el)",
    )
    .await
}

/// Find similar elements (like Scrapling's `find_similar`).
pub async fn find_similar(page: &Page, selector: &str) -> Result<Vec<NavElement>> {
    let js = format!(
        r#"
        (() => {{
            const el = document.querySelector('{}');
            if (!el) return [];

            const tag = el.tagName;
            const classes = Array.from(el.classList);
            const results = [];

            const candidates = document.querySelectorAll(tag);
            for (const c of candidates) {{
                if (c === el) continue;

                let score = 0;
                const cClasses = Array.from(c.classList);

                const overlap = classes.filter(cl => cClasses.includes(cl)).length;
                if (classes.length > 0) score += (overlap / classes.length) * 50;

                if (c.parentElement === el.parentElement) score += 20;

                if (Math.abs(c.attributes.length - el.attributes.length) <= 1) score += 10;

                const lenRatio = Math.min(c.textContent.length, el.textContent.length) /
                                 Math.max(c.textContent.length, el.textContent.length || 1);
                score += lenRatio * 20;

                if (score >= 40) {{
                    results.push({{
                        tag: c.tagName.toLowerCase(),
                        text: c.textContent?.substring(0, 500) || '',
                        html: c.outerHTML?.substring(0, 2000) || '',
                        attributes: Object.fromEntries(Array.from(c.attributes || []).map(a => [a.name, a.value]))
                    }});
                }}
            }}
            return results;
        }})()
    "#,
        selector.replace('\'', "\\'")
    );

    let val = page.evaluate(js).await.map_err(|e| Error::Browser(e.to_string()))?;
    let elements: Vec<NavElement> =
        serde_json::from_value(val.into_value().unwrap_or(serde_json::json!([])))?;
    Ok(elements)
}

/// Get elements above the target in the DOM flow.
pub async fn above_elements(
    page: &Page,
    selector: &str,
    limit: usize,
) -> Result<Vec<NavElement>> {
    let js = format!(
        r#"
        (() => {{
            const el = document.querySelector('{}');
            if (!el) return [];
            const rect = el.getBoundingClientRect();
            const all = document.querySelectorAll('*');
            const results = [];
            for (const other of all) {{
                if (other === el) continue;
                const otherRect = other.getBoundingClientRect();
                if (otherRect.bottom <= rect.top && otherRect.height > 0) {{
                    results.push({{
                        tag: other.tagName.toLowerCase(),
                        text: other.textContent?.substring(0, 500) || '',
                        html: other.outerHTML?.substring(0, 2000) || '',
                        attributes: Object.fromEntries(Array.from(other.attributes || []).map(a => [a.name, a.value]))
                    }});
                }}
                if (results.length >= {}) break;
            }}
            return results;
        }})()
    "#,
        selector.replace('\'', "\\'"),
        limit
    );

    let val = page.evaluate(js).await.map_err(|e| Error::Browser(e.to_string()))?;
    let elements: Vec<NavElement> =
        serde_json::from_value(val.into_value().unwrap_or(serde_json::json!([])))?;
    Ok(elements)
}

/// Get elements below the target in the DOM flow.
pub async fn below_elements(
    page: &Page,
    selector: &str,
    limit: usize,
) -> Result<Vec<NavElement>> {
    let js = format!(
        r#"
        (() => {{
            const el = document.querySelector('{}');
            if (!el) return [];
            const rect = el.getBoundingClientRect();
            const all = document.querySelectorAll('*');
            const results = [];
            for (const other of all) {{
                if (other === el) continue;
                const otherRect = other.getBoundingClientRect();
                if (otherRect.top >= rect.bottom && otherRect.height > 0) {{
                    results.push({{
                        tag: other.tagName.toLowerCase(),
                        text: other.textContent?.substring(0, 500) || '',
                        html: other.outerHTML?.substring(0, 2000) || '',
                        attributes: Object.fromEntries(Array.from(other.attributes || []).map(a => [a.name, a.value]))
                    }});
                }}
                if (results.length >= {}) break;
            }}
            return results;
        }})()
    "#,
        selector.replace('\'', "\\'"),
        limit
    );

    let val = page.evaluate(js).await.map_err(|e| Error::Browser(e.to_string()))?;
    let elements: Vec<NavElement> =
        serde_json::from_value(val.into_value().unwrap_or(serde_json::json!([])))?;
    Ok(elements)
}

async fn nav_query(page: &Page, selector: &str, collection_expr: &str) -> Result<Vec<NavElement>> {
    let js = format!(
        r#"
        (() => {{
            const el = document.querySelector('{}');
            if (!el) return [];
            const collection = {};
            return collection.map(c => ({{
                tag: c.tagName?.toLowerCase() || '#text',
                text: c.textContent?.substring(0, 500) || '',
                html: c.outerHTML?.substring(0, 2000) || '',
                attributes: Object.fromEntries(Array.from(c.attributes || []).map(a => [a.name, a.value]))
            }}));
        }})()
    "#,
        selector.replace('\'', "\\'"),
        collection_expr
    );

    let val = page.evaluate(js).await.map_err(|e| Error::Browser(e.to_string()))?;
    let elements: Vec<NavElement> =
        serde_json::from_value(val.into_value().unwrap_or(serde_json::json!([])))?;
    Ok(elements)
}
