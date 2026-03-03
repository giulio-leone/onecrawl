use chromiumoxide::Page;
use onecrawl_core::{Error, Result};

/// Get the full page HTML content.
pub async fn get_content(page: &Page) -> Result<String> {
    let html = page
        .content()
        .await
        .map_err(|e| Error::Cdp(format!("get_content failed: {e}")))?;
    Ok(html)
}

/// Set the page HTML content.
pub async fn set_content(page: &Page, html: &str) -> Result<()> {
    page.set_content(html)
        .await
        .map_err(|e| Error::Cdp(format!("set_content failed: {e}")))?;
    Ok(())
}

/// Execute JavaScript and return result.
pub async fn evaluate_js(page: &Page, js: &str) -> Result<serde_json::Value> {
    let result = page
        .evaluate(js)
        .await
        .map_err(|e| Error::Cdp(format!("evaluate failed: {e}")))?;

    // Gracefully handle expressions that return undefined/void
    match result.into_value::<serde_json::Value>() {
        Ok(val) => Ok(val),
        Err(_) => Ok(serde_json::Value::Null),
    }
}
