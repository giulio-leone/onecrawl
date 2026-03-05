use colored::Colorize;
use super::super::helpers::{with_page};

// ---------------------------------------------------------------------------
// Streaming Extractor
// ---------------------------------------------------------------------------

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

// Find (Semantic Locators)
