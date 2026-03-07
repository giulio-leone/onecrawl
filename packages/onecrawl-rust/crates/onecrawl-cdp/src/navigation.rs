use chromiumoxide::cdp::browser_protocol::page::HandleJavaScriptDialogParams;
use chromiumoxide::Page;
use onecrawl_core::{Error, Result};

/// Navigate to a URL and wait for load.
///
/// Uses JS-based navigation (`window.location.href = url`) to bypass chromiumoxide's
/// navigation watcher, which can timeout on SPAs (like x.com) that keep background
/// connections open and never truly fire the "load" event.
///
/// Dismisses any `beforeunload` dialog that may block navigation, then polls until
/// the URL changes to the target domain or 60 seconds elapse.
pub async fn goto(page: &Page, url: &str) -> Result<()> {
    let safe_url = url.replace('\\', "\\\\").replace('\'', "\\'");

    // Dismiss any beforeunload dialog that could block navigation.
    // We clear onbeforeunload first, then trigger navigation.
    page.evaluate("window.onbeforeunload = null")
        .await
        .ok();

    // Trigger navigation via JS with a timeout — if a dialog blocks evaluate(),
    // the timeout prevents an indefinite hang.
    let nav_js = format!("window.location.href = '{safe_url}'");
    let nav_result = tokio::time::timeout(
        tokio::time::Duration::from_secs(5),
        page.evaluate(nav_js),
    )
    .await;

    // If evaluate timed out, a dialog is likely blocking — dismiss it and retry.
    if nav_result.is_err() {
        let _ = page
            .execute(HandleJavaScriptDialogParams::new(true))
            .await;
        tokio::time::sleep(tokio::time::Duration::from_millis(200)).await;
        page.evaluate(format!("window.location.href = '{safe_url}'"))
            .await
            .map_err(|e| Error::Cdp(format!("goto failed after dialog dismiss: {e}")))?;
    } else if let Ok(Err(e)) = nav_result {
        return Err(Error::Cdp(format!("goto failed: {e}")));
    }

    // Poll until URL changes to the target domain (or 60s elapsed)
    let target_domain = url
        .trim_start_matches("https://")
        .trim_start_matches("http://")
        .split('/')
        .next()
        .unwrap_or("");
    let deadline = tokio::time::Instant::now() + tokio::time::Duration::from_secs(60);
    loop {
        tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
        let current = page.url().await.ok().flatten().unwrap_or_default();
        if !current.is_empty() && current != "about:blank" && (target_domain.is_empty() || current.contains(target_domain)) {
            break;
        }
        if tokio::time::Instant::now() >= deadline {
            break;
        }
    }
    Ok(())
}

/// Go back in history.
pub async fn go_back(page: &Page) -> Result<()> {
    page.evaluate("window.history.back()")
        .await
        .map_err(|e| Error::Cdp(format!("go_back failed: {e}")))?;
    Ok(())
}

/// Go forward in history.
pub async fn go_forward(page: &Page) -> Result<()> {
    page.evaluate("window.history.forward()")
        .await
        .map_err(|e| Error::Cdp(format!("go_forward failed: {e}")))?;
    Ok(())
}

/// Reload the page.
pub async fn reload(page: &Page) -> Result<()> {
    page.evaluate("window.location.reload()")
        .await
        .map_err(|e| Error::Cdp(format!("reload failed: {e}")))?;
    Ok(())
}

/// Get the current URL.
pub async fn get_url(page: &Page) -> Result<String> {
    let url = page
        .url()
        .await
        .map_err(|e| Error::Cdp(format!("get_url failed: {e}")))?
        .unwrap_or_default()
        .to_string();
    Ok(url)
}

/// Get the page title.
pub async fn get_title(page: &Page) -> Result<String> {
    let title = page
        .evaluate("document.title")
        .await
        .map_err(|e| Error::Cdp(format!("get_title failed: {e}")))?
        .into_value::<String>()
        .map_err(|e| Error::Cdp(format!("parse title failed: {e}")))?;
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
        selector.replace('\\', "\\\\").replace('\'', "\\'")
    );
    loop {
        let found = page
            .evaluate(js.as_str())
            .await
            .map_err(|e| Error::Cdp(format!("wait_for_selector eval failed: {e}")))?
            .into_value::<bool>()
            .unwrap_or(false);
        if found {
            return Ok(());
        }
        if tokio::time::Instant::now() >= deadline {
            return Err(Error::Cdp(format!(
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
            return Err(Error::Cdp(format!(
                "wait_for_url timed out after {timeout_ms}ms waiting for '{url_pattern}'"
            )));
        }
        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
    }
}
