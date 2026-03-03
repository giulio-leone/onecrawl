use chromiumoxide::Page;
use chromiumoxide::cdp::browser_protocol::page::CaptureScreenshotFormat;
use onecrawl_core::{Error, Result};
use serde::{Deserialize, Serialize};

/// Supported screenshot image formats.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ImageFormat {
    Png,
    Jpeg,
    Webp,
}

impl Default for ImageFormat {
    fn default() -> Self {
        Self::Png
    }
}

/// Options for taking a screenshot.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScreenshotOptions {
    /// Image format (default: PNG).
    pub format: ImageFormat,
    /// JPEG/WebP quality (0–100). Ignored for PNG.
    pub quality: Option<u32>,
    /// Capture the full scrollable page instead of just the viewport.
    pub full_page: bool,
}

impl Default for ScreenshotOptions {
    fn default() -> Self {
        Self {
            format: ImageFormat::Png,
            quality: None,
            full_page: false,
        }
    }
}

/// Options for generating a PDF.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PdfOptions {
    /// Landscape orientation.
    pub landscape: bool,
    /// Scale of the page rendering (default: 1.0).
    pub scale: f64,
    /// Paper width in inches (default: 8.5 — US Letter).
    pub paper_width: f64,
    /// Paper height in inches (default: 11.0 — US Letter).
    pub paper_height: f64,
}

impl Default for PdfOptions {
    fn default() -> Self {
        Self {
            landscape: false,
            scale: 1.0,
            paper_width: 8.5,
            paper_height: 11.0,
        }
    }
}

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

/// Take a screenshot with custom options.
pub async fn screenshot_with_options(page: &Page, opts: &ScreenshotOptions) -> Result<Vec<u8>> {
    let cdp_format = match opts.format {
        ImageFormat::Png => CaptureScreenshotFormat::Png,
        ImageFormat::Jpeg => CaptureScreenshotFormat::Jpeg,
        ImageFormat::Webp => CaptureScreenshotFormat::Webp,
    };

    let mut builder = chromiumoxide::page::ScreenshotParams::builder()
        .full_page(opts.full_page)
        .format(cdp_format);

    if let Some(q) = opts.quality {
        builder = builder.quality(q as i64);
    }

    let bytes = page
        .screenshot(builder.build())
        .await
        .map_err(|e| Error::Browser(format!("screenshot failed: {e}")))?;
    Ok(bytes)
}

/// Take a screenshot of a specific element identified by CSS selector.
///
/// Scrolls the element into view, then captures its bounding box.
pub async fn screenshot_element(page: &Page, selector: &str) -> Result<Vec<u8>> {
    let el = page
        .find_element(selector)
        .await
        .map_err(|e| Error::Browser(format!("element not found: {e}")))?;
    let bytes = el
        .screenshot(CaptureScreenshotFormat::Png)
        .await
        .map_err(|e| Error::Browser(format!("element screenshot failed: {e}")))?;
    Ok(bytes)
}

/// Take a full-page screenshot (alias for `screenshot_full`).
pub async fn take_full_page_screenshot(page: &Page) -> Result<Vec<u8>> {
    screenshot_full(page).await
}

/// Save page as PDF.
pub async fn pdf(page: &Page) -> Result<Vec<u8>> {
    let bytes = page
        .pdf(Default::default())
        .await
        .map_err(|e| Error::Browser(format!("pdf failed: {e}")))?;
    Ok(bytes)
}

/// Save page as PDF with custom options.
pub async fn pdf_with_options(page: &Page, opts: &PdfOptions) -> Result<Vec<u8>> {
    let params = chromiumoxide::cdp::browser_protocol::page::PrintToPdfParams::builder()
        .landscape(opts.landscape)
        .scale(opts.scale)
        .paper_width(opts.paper_width)
        .paper_height(opts.paper_height)
        .build();
    let bytes = page
        .pdf(params)
        .await
        .map_err(|e| Error::Browser(format!("pdf failed: {e}")))?;
    Ok(bytes)
}
