use chromiumoxide::Page;
use onecrawl_core::{Error, Result};

/// Navigate to a URL and wait for load.
pub async fn goto(page: &Page, url: &str) -> Result<()> {
    page.goto(url)
        .await
        .map_err(|e| Error::Browser(format!("goto failed: {e}")))?;
    Ok(())
}

/// Go back in history.
pub async fn go_back(page: &Page) -> Result<()> {
    page.evaluate("window.history.back()")
        .await
        .map_err(|e| Error::Browser(format!("go_back failed: {e}")))?;
    Ok(())
}

/// Go forward in history.
pub async fn go_forward(page: &Page) -> Result<()> {
    page.evaluate("window.history.forward()")
        .await
        .map_err(|e| Error::Browser(format!("go_forward failed: {e}")))?;
    Ok(())
}

/// Reload the page.
pub async fn reload(page: &Page) -> Result<()> {
    page.evaluate("window.location.reload()")
        .await
        .map_err(|e| Error::Browser(format!("reload failed: {e}")))?;
    Ok(())
}

/// Get the current URL.
pub async fn get_url(page: &Page) -> Result<String> {
    let url = page
        .url()
        .await
        .map_err(|e| Error::Browser(format!("get_url failed: {e}")))?
        .unwrap_or_default()
        .to_string();
    Ok(url)
}

/// Get the page title.
pub async fn get_title(page: &Page) -> Result<String> {
    let title = page
        .evaluate("document.title")
        .await
        .map_err(|e| Error::Browser(format!("get_title failed: {e}")))?
        .into_value::<String>()
        .map_err(|e| Error::Browser(format!("parse title failed: {e}")))?;
    Ok(title)
}

/// Wait for a specific number of milliseconds.
pub async fn wait_ms(ms: u64) {
    tokio::time::sleep(tokio::time::Duration::from_millis(ms)).await;
}
