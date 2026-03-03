//! Smart CSS/XPath selectors with pseudo-elements (Scrapling-like).

use chromiumoxide::Page;
use onecrawl_core::{Error, Result};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SelectorResult {
    pub selector: String,
    pub count: usize,
    pub results: Vec<ElementData>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ElementData {
    pub tag: String,
    pub text: String,
    pub html: String,
    pub attributes: std::collections::HashMap<String, String>,
    pub index: usize,
}

/// CSS selector with Scrapy-style pseudo-elements (`::text`, `::attr(name)`).
pub async fn css_select(page: &Page, selector: &str) -> Result<SelectorResult> {
    let (real_selector, pseudo) = parse_pseudo(selector);

    let js = format!(
        r#"
        (() => {{
            const els = document.querySelectorAll('{}');
            return Array.from(els).map((el, i) => ({{
                tag: el.tagName.toLowerCase(),
                text: el.textContent || '',
                html: el.outerHTML,
                attributes: Object.fromEntries(Array.from(el.attributes).map(a => [a.name, a.value])),
                index: i
            }}));
        }})()
    "#,
        real_selector.replace('\'', "\\'")
    );

    let val = page
        .evaluate(js)
        .await
        .map_err(|e| Error::Browser(e.to_string()))?;
    let mut elements: Vec<ElementData> =
        serde_json::from_value(val.into_value().unwrap_or(serde_json::json!([])))?;

    // Apply pseudo-element extraction
    match pseudo.as_deref() {
        Some("text") => {
            for el in &mut elements {
                el.html = el.text.clone();
            }
        }
        Some(attr) if attr.starts_with("attr(") && attr.ends_with(')') => {
            let attr_name = &attr[5..attr.len() - 1];
            for el in &mut elements {
                el.text = el.attributes.get(attr_name).cloned().unwrap_or_default();
                el.html = el.text.clone();
            }
        }
        _ => {}
    }

    let count = elements.len();
    Ok(SelectorResult {
        selector: selector.to_string(),
        count,
        results: elements,
    })
}

/// XPath selector.
pub async fn xpath_select(page: &Page, expression: &str) -> Result<SelectorResult> {
    let expr_escaped = expression.replace('\\', "\\\\").replace('\'', "\\'");
    let js = format!(
        r#"
        (() => {{
            const results = [];
            const xpathResult = document.evaluate('{}', document, null, XPathResult.ORDERED_NODE_SNAPSHOT_TYPE, null);
            for (let i = 0; i < xpathResult.snapshotLength; i++) {{
                const node = xpathResult.snapshotItem(i);
                if (node.nodeType === Node.ELEMENT_NODE) {{
                    results.push({{
                        tag: node.tagName.toLowerCase(),
                        text: node.textContent || '',
                        html: node.outerHTML,
                        attributes: Object.fromEntries(Array.from(node.attributes || []).map(a => [a.name, a.value])),
                        index: i
                    }});
                }} else if (node.nodeType === Node.TEXT_NODE) {{
                    results.push({{
                        tag: '#text',
                        text: node.textContent || '',
                        html: node.textContent || '',
                        attributes: {{}},
                        index: i
                    }});
                }} else if (node.nodeType === Node.ATTRIBUTE_NODE) {{
                    results.push({{
                        tag: '#attr',
                        text: node.value || '',
                        html: node.value || '',
                        attributes: {{ name: node.name }},
                        index: i
                    }});
                }}
            }}
            return results;
        }})()
    "#,
        expr_escaped
    );

    let val = page
        .evaluate(js)
        .await
        .map_err(|e| Error::Browser(e.to_string()))?;
    let elements: Vec<ElementData> =
        serde_json::from_value(val.into_value().unwrap_or(serde_json::json!([])))?;
    let count = elements.len();

    Ok(SelectorResult {
        selector: expression.to_string(),
        count,
        results: elements,
    })
}

/// Find elements by text content (like Scrapling's `find_by_text`).
pub async fn find_by_text(page: &Page, text: &str, tag: Option<&str>) -> Result<SelectorResult> {
    let tag_filter = tag.unwrap_or("*");
    let text_escaped = text.replace('\\', "\\\\").replace('\'', "\\'");
    let js = format!(
        r#"
        (() => {{
            const results = [];
            const all = document.querySelectorAll('{}');
            let idx = 0;
            for (const el of all) {{
                if (el.textContent && el.textContent.includes('{}')) {{
                    results.push({{
                        tag: el.tagName.toLowerCase(),
                        text: el.textContent || '',
                        html: el.outerHTML,
                        attributes: Object.fromEntries(Array.from(el.attributes || []).map(a => [a.name, a.value])),
                        index: idx++
                    }});
                }}
            }}
            return results;
        }})()
    "#,
        tag_filter, text_escaped
    );

    let val = page
        .evaluate(js)
        .await
        .map_err(|e| Error::Browser(e.to_string()))?;
    let elements: Vec<ElementData> =
        serde_json::from_value(val.into_value().unwrap_or(serde_json::json!([])))?;
    let count = elements.len();

    Ok(SelectorResult {
        selector: format!("text('{}')", text),
        count,
        results: elements,
    })
}

/// Find elements by regex pattern.
pub async fn find_by_regex(
    page: &Page,
    pattern: &str,
    tag: Option<&str>,
) -> Result<SelectorResult> {
    let tag_filter = tag.unwrap_or("*");
    let pattern_escaped = pattern.replace('\\', "\\\\").replace('\'', "\\'");
    let js = format!(
        r#"
        (() => {{
            const regex = new RegExp('{}');
            const results = [];
            const all = document.querySelectorAll('{}');
            let idx = 0;
            for (const el of all) {{
                if (el.textContent && regex.test(el.textContent)) {{
                    results.push({{
                        tag: el.tagName.toLowerCase(),
                        text: el.textContent || '',
                        html: el.outerHTML,
                        attributes: Object.fromEntries(Array.from(el.attributes || []).map(a => [a.name, a.value])),
                        index: idx++
                    }});
                }}
            }}
            return results;
        }})()
    "#,
        pattern_escaped, tag_filter
    );

    let val = page
        .evaluate(js)
        .await
        .map_err(|e| Error::Browser(e.to_string()))?;
    let elements: Vec<ElementData> =
        serde_json::from_value(val.into_value().unwrap_or(serde_json::json!([])))?;
    let count = elements.len();

    Ok(SelectorResult {
        selector: format!("regex('{}')", pattern),
        count,
        results: elements,
    })
}

/// Auto-generate a unique CSS selector for an element.
pub async fn auto_selector(page: &Page, target_selector: &str) -> Result<String> {
    let js = format!(
        r#"
        (() => {{
            const el = document.querySelector('{}');
            if (!el) return '';

            function getSelector(el) {{
                if (el.id) return '#' + el.id;

                let path = [];
                let current = el;
                while (current && current !== document.body) {{
                    let selector = current.tagName.toLowerCase();
                    if (current.id) {{
                        path.unshift('#' + current.id);
                        break;
                    }}
                    if (current.className) {{
                        const classes = Array.from(current.classList).filter(c => c.trim()).join('.');
                        if (classes) selector += '.' + classes;
                    }}
                    const parent = current.parentElement;
                    if (parent) {{
                        const siblings = Array.from(parent.children).filter(s => s.tagName === current.tagName);
                        if (siblings.length > 1) {{
                            const idx = siblings.indexOf(current) + 1;
                            selector += ':nth-child(' + idx + ')';
                        }}
                    }}
                    path.unshift(selector);
                    current = current.parentElement;
                }}
                return path.join(' > ');
            }}

            return getSelector(el);
        }})()
    "#,
        target_selector.replace('\'', "\\'")
    );

    let val = page
        .evaluate(js)
        .await
        .map_err(|e| Error::Browser(e.to_string()))?;
    let s = val.into_value().unwrap_or(serde_json::json!(""));
    Ok(s.as_str().map(String::from).unwrap_or_default())
}

/// Helper to parse pseudo-elements from CSS selector.
fn parse_pseudo(selector: &str) -> (String, Option<String>) {
    if let Some(idx) = selector.rfind("::") {
        let real = selector[..idx].to_string();
        let pseudo = selector[idx + 2..].to_string();
        (real, Some(pseudo))
    } else {
        (selector.to_string(), None)
    }
}
