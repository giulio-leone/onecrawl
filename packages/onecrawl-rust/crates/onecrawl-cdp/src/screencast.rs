//! Live browser streaming via CDP Page.startScreencast / Page.stopScreencast.

use chromiumoxide::cdp::browser_protocol::page::{
    CaptureScreenshotFormat, CaptureScreenshotParams, StartScreencastFormat,
    StartScreencastParams, StopScreencastParams,
};
use chromiumoxide::Page;
use onecrawl_core::{Error, Result};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScreencastOptions {
    pub format: String,
    pub quality: Option<u32>,
    pub max_width: Option<u32>,
    pub max_height: Option<u32>,
    pub every_nth_frame: Option<u32>,
}

impl Default for ScreencastOptions {
    fn default() -> Self {
        Self {
            format: "jpeg".to_string(),
            quality: Some(60),
            max_width: Some(1280),
            max_height: Some(720),
            every_nth_frame: Some(1),
        }
    }
}

/// Start screencast — enables CDP `Page.startScreencast`.
/// Frames arrive via CDP `Page.screencastFrame` events.
pub async fn start_screencast(page: &Page, opts: &ScreencastOptions) -> Result<()> {
    let format = if opts.format == "png" {
        StartScreencastFormat::Png
    } else {
        StartScreencastFormat::Jpeg
    };

    let mut builder = StartScreencastParams::builder().format(format);

    if let Some(q) = opts.quality {
        builder = builder.quality(q as i64);
    }
    if let Some(w) = opts.max_width {
        builder = builder.max_width(w as i64);
    }
    if let Some(h) = opts.max_height {
        builder = builder.max_height(h as i64);
    }
    if let Some(n) = opts.every_nth_frame {
        builder = builder.every_nth_frame(n as i64);
    }

    page.execute(builder.build())
        .await
        .map_err(|e| Error::Cdp(format!("start_screencast failed: {e}")))?;
    Ok(())
}

/// Stop screencast.
pub async fn stop_screencast(page: &Page) -> Result<()> {
    page.execute(StopScreencastParams {})
        .await
        .map_err(|e| Error::Cdp(format!("stop_screencast failed: {e}")))?;
    Ok(())
}

/// Capture a single frame using CDP `Page.captureScreenshot`.
pub async fn capture_frame(page: &Page, opts: &ScreencastOptions) -> Result<Vec<u8>> {
    let format = if opts.format == "png" {
        CaptureScreenshotFormat::Png
    } else {
        CaptureScreenshotFormat::Jpeg
    };

    let mut builder = CaptureScreenshotParams::builder().format(format);
    if let Some(q) = opts.quality {
        builder = builder.quality(q as i64);
    }

    let resp = page
        .execute(builder.build())
        .await
        .map_err(|e| Error::Cdp(format!("capture_frame failed: {e}")))?;

    use base64::Engine;
    let bytes = base64::engine::general_purpose::STANDARD
        .decode(resp.data.as_ref() as &str)
        .map_err(|e| Error::Cdp(format!("base64 decode failed: {e}")))?;
    Ok(bytes)
}
