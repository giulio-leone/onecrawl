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

/// Wait for an element matching `selector` to appear in the DOM.
///
/// Polls every 100ms until the selector matches or `timeout_ms` elapses.
pub async fn wait_for_selector(page: &Page, selector: &str, timeout_ms: u64) -> Result<()> {
    let deadline = tokio::time::Instant::now() + tokio::time::Duration::from_millis(timeout_ms);
    let js = format!(
        "!!document.querySelector('{}')",
        selector.replace('\'', "\\'")
    );
    loop {
        let found = page
            .evaluate(js.as_str())
            .await
            .map_err(|e| Error::Browser(format!("wait_for_selector eval failed: {e}")))?
            .into_value::<bool>()
            .unwrap_or(false);
        if found {
            return Ok(());
        }
        if tokio::time::Instant::now() >= deadline {
            return Err(Error::Browser(format!(
                "wait_for_selector timed out after {timeout_ms}ms for '{selector}'"
            )));
        }
        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
    }
}

/// Wait for the page URL to match `url_pattern` (substring match).
///
/// Polls every 100ms until the URL contains the pattern or `timeout_ms` elapses.
pub async fn wait_for_url(page: &Page, url_pattern: &str, timeout_ms: u64) -> Result<()> {
    let deadline = tokio::time::Instant::now() + tokio::time::Duration::from_millis(timeout_ms);
    loop {
        let current = get_url(page).await?;
        if current.contains(url_pattern) {
            return Ok(());
        }
        if tokio::time::Instant::now() >= deadline {
            return Err(Error::Browser(format!(
                "wait_for_url timed out after {timeout_ms}ms waiting for '{url_pattern}'"
            )));
        }
        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
    }
}
