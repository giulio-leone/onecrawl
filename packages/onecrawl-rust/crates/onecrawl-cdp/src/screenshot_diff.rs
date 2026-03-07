use onecrawl_browser::Page;
use onecrawl_core::{Error, Result};
use serde::{Deserialize, Serialize};
use std::path::Path;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiffResult {
    pub total_pixels: u64,
    pub different_pixels: u64,
    pub difference_percentage: f64,
    pub is_identical: bool,
    pub width: u32,
    pub height: u32,
}

/// Result of an in-browser pixel-level visual comparison with diff image output.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PixelDiffResult {
    /// PNG bytes of the diff visualisation (red = different, dimmed = same)
    pub diff_image: Vec<u8>,
    /// Total pixel count
    pub total_pixels: u64,
    /// Number of pixels that differ beyond the threshold
    pub different_pixels: u64,
    /// Percentage of pixels that differ (0.0–100.0)
    pub difference_percentage: f64,
    /// Per-channel threshold used for comparison (0.0–1.0)
    pub threshold: f64,
}

/// Compare two PNG screenshots byte-by-byte.
pub fn compare_screenshots(baseline: &[u8], current: &[u8]) -> Result<DiffResult> {
    let baseline_len = baseline.len();
    let current_len = current.len();
    let min_len = baseline_len.min(current_len);
    let max_len = baseline_len.max(current_len);

    let mut diff_bytes = 0u64;
    for i in 0..min_len {
        if baseline[i] != current[i] {
            diff_bytes += 1;
        }
    }
    diff_bytes += (max_len - min_len) as u64;

    let total = max_len as u64;
    let pct = if total > 0 {
        (diff_bytes as f64 / total as f64) * 100.0
    } else {
        0.0
    };

    // Estimate pixel count assuming RGBA (4 bytes per pixel)
    let pixel_divisor = 4u64;
    Ok(DiffResult {
        total_pixels: total / pixel_divisor,
        different_pixels: diff_bytes / pixel_divisor,
        difference_percentage: pct,
        is_identical: diff_bytes == 0,
        width: 0,
        height: 0,
    })
}

/// Compare two screenshot files on disk.
pub fn compare_screenshot_files(baseline_path: &Path, current_path: &Path) -> Result<DiffResult> {
    let baseline = std::fs::read(baseline_path)
        .map_err(|e| Error::Cdp(format!("read baseline failed: {e}")))?;
    let current = std::fs::read(current_path)
        .map_err(|e| Error::Cdp(format!("read current failed: {e}")))?;
    compare_screenshots(&baseline, &current)
}

/// Take a screenshot and compare with a baseline file.
/// If baseline doesn't exist, saves current screenshot as the new baseline.
pub async fn visual_regression(page: &Page, baseline_path: &Path) -> Result<DiffResult> {
    let current = page
        .screenshot(onecrawl_browser::page::ScreenshotParams::builder().build())
        .await
        .map_err(|e| Error::Cdp(format!("visual_regression screenshot failed: {e}")))?;

    if !baseline_path.exists() {
        std::fs::write(baseline_path, &current)
            .map_err(|e| Error::Cdp(format!("save baseline failed: {e}")))?;
        return Ok(DiffResult {
            total_pixels: current.len() as u64 / 4,
            different_pixels: 0,
            difference_percentage: 0.0,
            is_identical: true,
            width: 0,
            height: 0,
        });
    }

    let baseline = std::fs::read(baseline_path)
        .map_err(|e| Error::Cdp(format!("read baseline failed: {e}")))?;
    compare_screenshots(&baseline, &current)
}

/// Perform in-browser pixel-level comparison of two PNG images via Canvas.
///
/// Injects JS that decodes both images, draws them onto off-screen canvases,
/// iterates every pixel, and produces a diff image (red = different, dimmed = same).
/// `threshold` controls per-channel sensitivity (0.0 = exact, default 0.05).
pub async fn pixel_diff(
    page: &Page,
    baseline: &[u8],
    current: &[u8],
    threshold: Option<f64>,
) -> Result<PixelDiffResult> {
    use base64::{Engine as _, engine::general_purpose::STANDARD as B64};

    let baseline_b64 = B64.encode(baseline);
    let current_b64 = B64.encode(current);
    let thresh = threshold.unwrap_or(0.05);

    let js = format!(
        r#"(async () => {{
    const thresh = {thresh};
    function b64ToImg(b64) {{
        return new Promise((resolve, reject) => {{
            const img = new Image();
            img.onload = () => resolve(img);
            img.onerror = reject;
            img.src = 'data:image/png;base64,' + b64;
        }});
    }}
    const [imgA, imgB] = await Promise.all([
        b64ToImg("{baseline_b64}"),
        b64ToImg("{current_b64}")
    ]);
    const w = Math.max(imgA.width, imgB.width);
    const h = Math.max(imgA.height, imgB.height);
    const cA = new OffscreenCanvas(w, h);
    const cB = new OffscreenCanvas(w, h);
    const cD = new OffscreenCanvas(w, h);
    const ctxA = cA.getContext('2d');
    const ctxB = cB.getContext('2d');
    const ctxD = cD.getContext('2d');
    ctxA.drawImage(imgA, 0, 0);
    ctxB.drawImage(imgB, 0, 0);
    const dA = ctxA.getImageData(0, 0, w, h);
    const dB = ctxB.getImageData(0, 0, w, h);
    const dD = ctxD.createImageData(w, h);
    const t = thresh * 255;
    let diffCount = 0;
    const total = w * h;
    for (let i = 0; i < dA.data.length; i += 4) {{
        const dr = Math.abs(dA.data[i] - dB.data[i]);
        const dg = Math.abs(dA.data[i+1] - dB.data[i+1]);
        const db = Math.abs(dA.data[i+2] - dB.data[i+2]);
        if (dr > t || dg > t || db > t) {{
            dD.data[i]   = 255;
            dD.data[i+1] = 0;
            dD.data[i+2] = 0;
            dD.data[i+3] = 255;
            diffCount++;
        }} else {{
            dD.data[i]   = Math.round(dA.data[i] * 0.3);
            dD.data[i+1] = Math.round(dA.data[i+1] * 0.3);
            dD.data[i+2] = Math.round(dA.data[i+2] * 0.3);
            dD.data[i+3] = 255;
        }}
    }}
    ctxD.putImageData(dD, 0, 0);
    const blob = await cD.convertToBlob({{ type: 'image/png' }});
    const buf = await blob.arrayBuffer();
    const arr = new Uint8Array(buf);
    let b = '';
    const chunk = 8192;
    for (let i = 0; i < arr.length; i += chunk) {{
        b += String.fromCharCode.apply(null, arr.subarray(i, i + chunk));
    }}
    const diffB64 = btoa(b);
    return JSON.stringify({{
        diff_b64: diffB64,
        total_pixels: total,
        different_pixels: diffCount,
        difference_percentage: total > 0 ? (diffCount / total) * 100 : 0
    }});
}})()"#
    );

    let val = page
        .evaluate(js)
        .await
        .map_err(|e| Error::Cdp(format!("pixel_diff JS failed: {e}")))?;

    let json_str: String =
        serde_json::from_value(val.into_value().unwrap_or(serde_json::json!("")))
            .unwrap_or_default();

    #[derive(Deserialize)]
    struct JsResult {
        diff_b64: String,
        total_pixels: u64,
        different_pixels: u64,
        difference_percentage: f64,
    }

    let parsed: JsResult = serde_json::from_str(&json_str)
        .map_err(|e| Error::Cdp(format!("pixel_diff parse result: {e}")))?;

    let diff_image = B64
        .decode(&parsed.diff_b64)
        .map_err(|e| Error::Cdp(format!("pixel_diff decode image: {e}")))?;

    Ok(PixelDiffResult {
        diff_image,
        total_pixels: parsed.total_pixels,
        different_pixels: parsed.different_pixels,
        difference_percentage: parsed.difference_percentage,
        threshold: thresh,
    })
}
