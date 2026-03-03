use chromiumoxide::Page;
use onecrawl_core::{Error, Result};

/// Click an element by CSS selector.
pub async fn click(page: &Page, selector: &str) -> Result<()> {
    let el = page
        .find_element(selector)
        .await
        .map_err(|e| Error::Browser(format!("element not found: {e}")))?;
    el.click()
        .await
        .map_err(|e| Error::Browser(format!("click failed: {e}")))?;
    Ok(())
}

/// Type text into a focused element (key-by-key).
pub async fn type_text(page: &Page, selector: &str, text: &str) -> Result<()> {
    let el = page
        .find_element(selector)
        .await
        .map_err(|e| Error::Browser(format!("element not found: {e}")))?;
    el.click()
        .await
        .map_err(|e| Error::Browser(format!("focus failed: {e}")))?;
    el.type_str(text)
        .await
        .map_err(|e| Error::Browser(format!("type failed: {e}")))?;
    Ok(())
}

/// Focus an element.
pub async fn focus(page: &Page, selector: &str) -> Result<()> {
    let el = page
        .find_element(selector)
        .await
        .map_err(|e| Error::Browser(format!("element not found: {e}")))?;
    el.click()
        .await
        .map_err(|e| Error::Browser(format!("focus failed: {e}")))?;
    Ok(())
}

/// Hover over an element.
pub async fn hover(page: &Page, selector: &str) -> Result<()> {
    page.evaluate(format!(
        "document.querySelector('{}')?.dispatchEvent(new MouseEvent('mouseover', {{bubbles: true}}))",
        selector.replace('\'', "\\'")
    ))
    .await
    .map_err(|e| Error::Browser(format!("hover failed: {e}")))?;
    Ok(())
}

/// Scroll to an element.
pub async fn scroll_into_view(page: &Page, selector: &str) -> Result<()> {
    page.evaluate(format!(
        "document.querySelector('{}')?.scrollIntoView({{behavior: 'smooth', block: 'center'}})",
        selector.replace('\'', "\\'")
    ))
    .await
    .map_err(|e| Error::Browser(format!("scroll_into_view failed: {e}")))?;
    Ok(())
}

/// Get text content of an element.
pub async fn get_text(page: &Page, selector: &str) -> Result<String> {
    let el = page
        .find_element(selector)
        .await
        .map_err(|e| Error::Browser(format!("element not found: {e}")))?;
    let text = el
        .inner_text()
        .await
        .map_err(|e| Error::Browser(format!("get_text failed: {e}")))?
        .unwrap_or_default();
    Ok(text)
}

/// Evaluate JavaScript in the page context.
pub async fn evaluate(page: &Page, expression: &str) -> Result<serde_json::Value> {
    let result = page
        .evaluate(expression)
        .await
        .map_err(|e| Error::Browser(format!("eval failed: {e}")))?
        .into_value::<serde_json::Value>()
        .map_err(|e| Error::Browser(format!("parse result failed: {e}")))?;
    Ok(result)
}
