use chromiumoxide::Page;
use onecrawl_core::{Error, Result};

/// Take a screenshot of the full page.
pub async fn screenshot_full(page: &Page) -> Result<Vec<u8>> {
    let bytes = page
        .screenshot(
            chromiumoxide::page::ScreenshotParams::builder()
                .full_page(true)
                .build(),
        )
        .await
        .map_err(|e| Error::Browser(format!("screenshot failed: {e}")))?;
    Ok(bytes)
}

/// Take a screenshot of the visible viewport.
pub async fn screenshot_viewport(page: &Page) -> Result<Vec<u8>> {
    let bytes = page
        .screenshot(chromiumoxide::page::ScreenshotParams::builder().build())
        .await
        .map_err(|e| Error::Browser(format!("screenshot failed: {e}")))?;
    Ok(bytes)
}

/// Save page as PDF.
pub async fn pdf(page: &Page) -> Result<Vec<u8>> {
    let bytes = page
        .pdf(Default::default())
        .await
        .map_err(|e| Error::Browser(format!("pdf failed: {e}")))?;
    Ok(bytes)
}
