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

pub async fn eval(expression: &str) {
    // Try proxy first
    if let Some(proxy) = super::super::super::proxy::ServerProxy::from_session().await {
        if let Ok(val) = proxy.evaluate(expression).await {
            let result = &val["result"];
            match result {
                serde_json::Value::String(s) => println!("{s}"),
                serde_json::Value::Null => println!("undefined"),
                other => println!(
                    "{}",
                    serde_json::to_string_pretty(other).unwrap_or_default()
                ),
            }
            return;
        }
    }
    with_page(|page| async move {
        let val = onecrawl_cdp::page::evaluate_js(&page, expression)
            .await
            .map_err(|e| e.to_string())?;
        match &val {
            serde_json::Value::String(s) => println!("{s}"),
            serde_json::Value::Null => println!("undefined"),
            other => println!(
                "{}",
                serde_json::to_string_pretty(other).unwrap_or_default()
            ),
        }
        Ok(())
    })
    .await;
}

