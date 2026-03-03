//! Streaming / pipeline-style structured data extractor.
//!
//! Define extraction rules and get structured data as items are found.

use chromiumoxide::Page;
use onecrawl_core::{Error, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

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

fn build_extract_js(fields: &[ExtractionRule], item_selector: &str, page_num: usize) -> String {
    let field_extractors: Vec<String> = fields
        .iter()
        .map(|f| {
            let extract_expr = if f.extract == "text" {
                format!(
                    "el.querySelector('{}')?.textContent || ''",
                    f.selector.replace('\'', "\\'")
                )
            } else if f.extract == "html" {
                format!(
                    "el.querySelector('{}')?.innerHTML || ''",
                    f.selector.replace('\'', "\\'")
                )
            } else if f.extract == "href" {
                format!(
                    "el.querySelector('{}')?.href || ''",
                    f.selector.replace('\'', "\\'")
                )
            } else if f.extract == "src" {
                format!(
                    "el.querySelector('{}')?.src || ''",
                    f.selector.replace('\'', "\\'")
                )
            } else if let Some(attr_name) = f.extract.strip_prefix("attr:") {
                format!(
                    "el.querySelector('{}')?.getAttribute('{}') || ''",
                    f.selector.replace('\'', "\\'"),
                    attr_name.replace('\'', "\\'")
                )
            } else {
                format!(
                    "el.querySelector('{}')?.textContent || ''",
                    f.selector.replace('\'', "\\'")
                )
            };

            let transform_expr = match f.transform.as_deref() {
                Some("trim") => ".trim()",
                Some("lowercase") => ".trim().toLowerCase()",
                Some("uppercase") => ".trim().toUpperCase()",
                Some("strip_tags") => ".replace(/<[^>]*>/g, '').trim()",
                _ => "",
            };

            format!(
                "fields['{}'] = ({}){}; if ({} && !fields['{}']) {{ errors.push('missing required field: {}'); }}",
                f.name.replace('\'', "\\'"),
                extract_expr,
                transform_expr,
                if f.required { "true" } else { "false" },
                f.name.replace('\'', "\\'"),
                f.name.replace('\'', "\\'"),
            )
        })
        .collect();

    format!(
        r#"(() => {{
            const items = [];
            const errors = [];
            const containers = document.querySelectorAll('{}');
            let idx = 0;
            for (const el of containers) {{
                const fields = {{}};
                {}
                items.push({{ index: idx, page: {}, fields }});
                idx++;
            }}
            return {{ items, errors }};
        }})()"#,
        item_selector.replace('\'', "\\'"),
        field_extractors.join("\n                "),
        page_num,
    )
}

fn build_single_extract_js(rules: &[ExtractionRule]) -> String {
    let field_extractors: Vec<String> = rules
        .iter()
        .map(|f| {
            let extract_expr = if f.extract == "text" {
                format!(
                    "document.querySelector('{}')?.textContent || ''",
                    f.selector.replace('\'', "\\'")
                )
            } else if f.extract == "html" {
                format!(
                    "document.querySelector('{}')?.innerHTML || ''",
                    f.selector.replace('\'', "\\'")
                )
            } else if f.extract == "href" {
                format!(
                    "document.querySelector('{}')?.href || ''",
                    f.selector.replace('\'', "\\'")
                )
            } else if f.extract == "src" {
                format!(
                    "document.querySelector('{}')?.src || ''",
                    f.selector.replace('\'', "\\'")
                )
            } else if let Some(attr_name) = f.extract.strip_prefix("attr:") {
                format!(
                    "document.querySelector('{}')?.getAttribute('{}') || ''",
                    f.selector.replace('\'', "\\'"),
                    attr_name.replace('\'', "\\'")
                )
            } else {
                format!(
                    "document.querySelector('{}')?.textContent || ''",
                    f.selector.replace('\'', "\\'")
                )
            };

            let transform_expr = match f.transform.as_deref() {
                Some("trim") => ".trim()",
                Some("lowercase") => ".trim().toLowerCase()",
                Some("uppercase") => ".trim().toUpperCase()",
                Some("strip_tags") => ".replace(/<[^>]*>/g, '').trim()",
                _ => "",
            };

            format!(
                "result['{}'] = ({}){};",
                f.name.replace('\'', "\\'"),
                extract_expr,
                transform_expr,
            )
        })
        .collect();

    format!(
        r#"(() => {{
            const result = {{}};
            {}
            return result;
        }})()"#,
        field_extractors.join("\n            "),
    )
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
                pagination.next_selector.replace('\'', "\\'")
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

fn escape_csv_field(s: &str) -> String {
    if s.contains(',') || s.contains('"') || s.contains('\n') || s.contains('\r') {
        format!("\"{}\"", s.replace('"', "\"\""))
    } else {
        s.to_string()
    }
}

/// Export items as CSV.
pub fn export_csv(items: &[ExtractedItem], path: &std::path::Path) -> Result<usize> {
    if items.is_empty() {
        std::fs::write(path, "")?;
        return Ok(0);
    }

    // Collect all field names from all items for consistent columns
    let mut columns: Vec<String> = Vec::new();
    for item in items {
        for key in item.fields.keys() {
            if !columns.contains(key) {
                columns.push(key.clone());
            }
        }
    }
    columns.sort();

    let mut csv = String::new();
    // Header
    csv.push_str(
        &columns
            .iter()
            .map(|c| escape_csv_field(c))
            .collect::<Vec<_>>()
            .join(","),
    );
    csv.push('\n');

    // Rows
    for item in items {
        let row: Vec<String> = columns
            .iter()
            .map(|col| escape_csv_field(item.fields.get(col).map(|s| s.as_str()).unwrap_or("")))
            .collect();
        csv.push_str(&row.join(","));
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
