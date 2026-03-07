//! Live browser streaming via CDP Page.startScreencast / Page.stopScreencast.

use onecrawl_browser::cdp::browser_protocol::page::{
    CaptureScreenshotFormat, CaptureScreenshotParams, StartScreencastFormat,
    StartScreencastParams, StopScreencastParams,
};
use onecrawl_browser::Page;
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

/// Capture N frames at a specified interval, returning all as raw bytes.
pub async fn capture_frames_burst(
    page: &Page,
    opts: &ScreencastOptions,
    count: usize,
    interval_ms: u64,
) -> Result<Vec<Vec<u8>>> {
    let mut frames = Vec::with_capacity(count);
    for i in 0..count {
        let frame = capture_frame(page, opts).await?;
        frames.push(frame);
        if interval_ms > 0 && i < count - 1 {
            tokio::time::sleep(std::time::Duration::from_millis(interval_ms)).await;
        }
    }
    Ok(frames)
}

/// Stream frames to a directory, returning metadata about saved frames.
pub async fn stream_to_disk(
    page: &Page,
    opts: &ScreencastOptions,
    output_dir: &str,
    count: usize,
    interval_ms: u64,
) -> Result<StreamResult> {
    std::fs::create_dir_all(output_dir)
        .map_err(|e| Error::Cdp(format!("mkdir: {e}")))?;
    let mut saved = Vec::new();
    let start = std::time::Instant::now();
    let ext = if opts.format == "png" { "png" } else { "jpg" };

    for i in 0..count {
        let bytes = capture_frame(page, opts).await?;
        let filename = format!("frame_{:04}.{ext}", i + 1);
        let path = format!("{output_dir}/{filename}");
        std::fs::write(&path, &bytes)
            .map_err(|e| Error::Cdp(format!("write: {e}")))?;
        saved.push(filename);

        if interval_ms > 0 && i < count - 1 {
            tokio::time::sleep(std::time::Duration::from_millis(interval_ms)).await;
        }
    }

    Ok(StreamResult {
        frames_captured: saved.len(),
        output_dir: output_dir.to_string(),
        files: saved,
        duration_ms: start.elapsed().as_millis() as u64,
    })
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StreamResult {
    pub frames_captured: usize,
    pub output_dir: String,
    pub files: Vec<String>,
    pub duration_ms: u64,
}
