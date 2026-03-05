use colored::Colorize;
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

