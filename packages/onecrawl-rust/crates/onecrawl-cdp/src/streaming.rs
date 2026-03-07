//! Streaming / pipeline-style structured data extractor.
//!
//! Define extraction rules and get structured data as items are found.

use onecrawl_browser::Page;
use onecrawl_core::{Error, Result};
use serde::{Deserialize, Serialize};
use std::borrow::Cow;
use std::collections::HashMap;
use std::fmt::Write;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtractionRule {
    pub name: String,
    pub selector: String,
    /// "text", "html", "attr:<name>", "href", "src"
    pub extract: String,
    /// "trim", "lowercase", "uppercase", "strip_tags"
    pub transform: Option<String>,
    pub required: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtractionSchema {
    /// CSS selector for each item container.
    pub item_selector: String,
    pub fields: Vec<ExtractionRule>,
    pub pagination: Option<PaginationConfig>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaginationConfig {
    /// CSS selector for "next page" link/button.
    pub next_selector: String,
    pub max_pages: usize,
    pub delay_ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtractedItem {
    pub index: usize,
    pub page: usize,
    pub fields: HashMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtractionResult {
    pub items: Vec<ExtractedItem>,
    pub total_items: usize,
    pub pages_scraped: usize,
    pub errors: Vec<String>,
}

/// Build JS expression for a single field extraction. Shared between multi-item and single-item.
fn write_field_extractor(
    buf: &mut String,
    f: &ExtractionRule,
    root: &str, // "el" for multi-item, "document" for single
    target: &str, // "fields" or "result"
    include_required_check: bool,
) {
    let escaped_selector = f.selector.replace('\\', "\\\\").replace('\'', "\\'");
    let escaped_name = f.name.replace('\\', "\\\\").replace('\'', "\\'");

    let property = if f.extract == "text" {
        "textContent"
    } else if f.extract == "html" {
        "innerHTML"
    } else if f.extract == "href" {
        "href"
    } else if f.extract == "src" {
        "src"
    } else {
        "textContent"
    };

    if let Some(attr_name) = f.extract.strip_prefix("attr:") {
        let escaped_attr = attr_name.replace('\\', "\\\\").replace('\'', "\\'");
        let _ = write!(
            buf,
            "{target}['{escaped_name}'] = ({root}.querySelector('{escaped_selector}')?.getAttribute('{escaped_attr}') || '')"
        );
    } else {
        let _ = write!(
            buf,
            "{target}['{escaped_name}'] = ({root}.querySelector('{escaped_selector}')?.{property} || '')"
        );
    }

    match f.transform.as_deref() {
        Some("trim") => buf.push_str(".trim()"),
        Some("lowercase") => buf.push_str(".trim().toLowerCase()"),
        Some("uppercase") => buf.push_str(".trim().toUpperCase()"),
        Some("strip_tags") => buf.push_str(".replace(/<[^>]*>/g, '').trim()"),
        _ => {}
    }
    buf.push_str(";\n                ");

    if include_required_check && f.required {
        let _ = write!(
            buf,
            "if (!{target}['{escaped_name}']) {{ errors.push('missing required field: {escaped_name}'); }}\n                "
        );
    }
}

fn build_extract_js(fields: &[ExtractionRule], item_selector: &str, page_num: usize) -> String {
    let escaped_item = item_selector.replace('\\', "\\\\").replace('\'', "\\'");
    let mut js = String::with_capacity(256 + fields.len() * 128);
    let _ = write!(
        js,
        r#"(() => {{
            const items = [];
            const errors = [];
            const containers = document.querySelectorAll('{escaped_item}');
            let idx = 0;
            for (const el of containers) {{
                const fields = {{}};
                "#
    );

    for f in fields {
        write_field_extractor(&mut js, f, "el", "fields", true);
    }

    let _ = write!(
        js,
        r#"items.push({{ index: idx, page: {page_num}, fields }});
                idx++;
            }}
            return {{ items, errors }};
        }})()"#
    );
    js
}

fn build_single_extract_js(rules: &[ExtractionRule]) -> String {
    let mut js = String::with_capacity(128 + rules.len() * 128);
    js.push_str(
        r#"(() => {
            const result = {};
            "#,
    );

    for f in rules {
        write_field_extractor(&mut js, f, "document", "result", false);
    }

    js.push_str(
        r#"return result;
        })()"#,
    );
    js
}

/// Extract all items from the current page using a schema.
pub async fn extract_items(page: &Page, schema: &ExtractionSchema) -> Result<ExtractionResult> {
    let js = build_extract_js(&schema.fields, &schema.item_selector, 1);
    let val = page
        .evaluate(js)
        .await
        .map_err(|e| Error::Cdp(e.to_string()))?;
    let raw = val.into_value().unwrap_or(serde_json::json!({}));

    let items: Vec<ExtractedItem> =
        serde_json::from_value(raw.get("items").cloned().unwrap_or(serde_json::json!([])))
            .unwrap_or_default();
    let errors: Vec<String> =
        serde_json::from_value(raw.get("errors").cloned().unwrap_or(serde_json::json!([])))
            .unwrap_or_default();

    let total = items.len();
    Ok(ExtractionResult {
        items,
        total_items: total,
        pages_scraped: 1,
        errors,
    })
}

/// Paginate and extract across multiple pages.
pub async fn extract_with_pagination(
    page: &Page,
    schema: &ExtractionSchema,
) -> Result<ExtractionResult> {
    let pagination = match &schema.pagination {
        Some(p) => p,
        None => return extract_items(page, schema).await,
    };

    let mut all_items: Vec<ExtractedItem> = Vec::new();
    let mut all_errors: Vec<String> = Vec::new();
    let mut pages_scraped = 0usize;

    for page_num in 1..=pagination.max_pages {
        let js = build_extract_js(&schema.fields, &schema.item_selector, page_num);
        let val = page
            .evaluate(js)
            .await
            .map_err(|e| Error::Cdp(e.to_string()))?;
        let raw = val.into_value().unwrap_or(serde_json::json!({}));

        let mut items: Vec<ExtractedItem> =
            serde_json::from_value(raw.get("items").cloned().unwrap_or(serde_json::json!([])))
                .unwrap_or_default();
        let errors: Vec<String> =
            serde_json::from_value(raw.get("errors").cloned().unwrap_or(serde_json::json!([])))
                .unwrap_or_default();

        // Re-index items globally
        for item in &mut items {
            item.index += all_items.len();
        }

        all_items.append(&mut items);
        all_errors.extend(errors);
        pages_scraped = page_num;

        // Try clicking next
        if page_num < pagination.max_pages {
            let click_js = format!(
                r#"(() => {{
                    const btn = document.querySelector('{}');
                    if (btn) {{ btn.click(); return true; }}
                    return false;
                }})()"#,
                pagination.next_selector.replace('\\', "\\\\").replace('\'', "\\'")
            );
            let click_val = page
                .evaluate(click_js)
                .await
                .map_err(|e| Error::Cdp(e.to_string()))?;
            let clicked: bool = click_val.into_value().unwrap_or(false);
            if !clicked {
                break;
            }
            if pagination.delay_ms > 0 {
                tokio::time::sleep(std::time::Duration::from_millis(pagination.delay_ms)).await;
            }
        }
    }

    let total = all_items.len();
    Ok(ExtractionResult {
        items: all_items,
        total_items: total,
        pages_scraped,
        errors: all_errors,
    })
}

/// Extract a single item from the page (no item_selector).
pub async fn extract_single(
    page: &Page,
    rules: &[ExtractionRule],
) -> Result<HashMap<String, String>> {
    let js = build_single_extract_js(rules);
    let val = page
        .evaluate(js)
        .await
        .map_err(|e| Error::Cdp(e.to_string()))?;
    let raw = val.into_value().unwrap_or(serde_json::json!({}));
    let map: HashMap<String, String> = serde_json::from_value(raw).unwrap_or_default();
    Ok(map)
}

fn escape_csv_field<'a>(s: &'a str) -> Cow<'a, str> {
    if s.contains(',') || s.contains('"') || s.contains('\n') || s.contains('\r') {
        let mut buf = String::with_capacity(s.len() + 4);
        buf.push('"');
        for ch in s.chars() {
            if ch == '"' {
                buf.push_str("\"\"");
            } else {
                buf.push(ch);
            }
        }
        buf.push('"');
        Cow::Owned(buf)
    } else {
        Cow::Borrowed(s)
    }
}

/// Export items as CSV.
pub fn export_csv(items: &[ExtractedItem], path: &std::path::Path) -> Result<usize> {
    if items.is_empty() {
        std::fs::write(path, "")?;
        return Ok(0);
    }

    // Collect all field names using a set for O(1) lookup
    let mut col_set = std::collections::HashSet::with_capacity(16);
    let mut columns: Vec<String> = Vec::new();
    for item in items {
        for key in item.fields.keys() {
            if col_set.insert(key.clone()) {
                columns.push(key.clone());
            }
        }
    }
    columns.sort();

    let mut csv = String::with_capacity(items.len() * columns.len() * 16);
    // Header
    for (i, c) in columns.iter().enumerate() {
        if i > 0 {
            csv.push(',');
        }
        csv.push_str(&escape_csv_field(c));
    }
    csv.push('\n');

    // Rows
    for item in items {
        for (i, col) in columns.iter().enumerate() {
            if i > 0 {
                csv.push(',');
            }
            let val = item.fields.get(col).map(|s| s.as_str()).unwrap_or("");
            csv.push_str(&escape_csv_field(val));
        }
        csv.push('\n');
    }

    std::fs::write(path, &csv)?;
    Ok(items.len())
}

/// Export items as JSON.
pub fn export_json(items: &[ExtractedItem], path: &std::path::Path) -> Result<usize> {
    let json = serde_json::to_string_pretty(items)?;
    std::fs::write(path, json)?;
    Ok(items.len())
}

/// Parse a CLI --field spec like "name:css:h2:text" into an ExtractionRule.
pub fn parse_field_spec(spec: &str) -> Result<ExtractionRule> {
    let parts: Vec<&str> = spec.splitn(4, ':').collect();
    if parts.len() < 4 {
        return Err(Error::Config(format!(
            "Invalid field spec: '{spec}'. Expected format: name:css:selector:extract"
        )));
    }
    Ok(ExtractionRule {
        name: parts[0].to_string(),
        selector: parts[2].to_string(),
        extract: parts[3].to_string(),
        transform: None,
        required: false,
    })
}
