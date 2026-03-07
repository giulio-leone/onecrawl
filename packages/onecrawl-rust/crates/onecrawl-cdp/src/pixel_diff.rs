//! In-browser pixel-level screenshot comparison using Canvas API.

use onecrawl_browser::Page;
use onecrawl_core::{Error, Result};
use serde::{Deserialize, Serialize};

/// Result of pixel-level screenshot comparison.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PixelDiffResult {
    /// Percentage of pixels that differ (0.0 - 100.0)
    pub mismatch_pct: f64,
    /// Total pixels compared
    pub total_pixels: u64,
    /// Number of different pixels
    pub diff_pixels: u64,
    /// Whether images are within threshold
    pub within_threshold: bool,
    /// Threshold used for comparison
    pub threshold: f64,
    /// Diff image as base64 PNG (red = different, dimmed = same)
    pub diff_image: Option<String>,
    /// Dimensions of compared images
    pub width: u32,
    pub height: u32,
}

/// Compare two base64-encoded images pixel by pixel in the browser.
/// Returns diff statistics and optionally a diff visualization image.
pub async fn pixel_diff(
    page: &Page,
    image_a_b64: &str,
    image_b_b64: &str,
    threshold: f64,
    generate_diff_image: bool,
) -> Result<PixelDiffResult> {
    let js = format!(
        r#"(async () => {{
            const imgA = new Image();
            const imgB = new Image();

            await Promise.all([
                new Promise((r, e) => {{ imgA.onload = r; imgA.onerror = e; imgA.src = 'data:image/png;base64,{a}'; }}),
                new Promise((r, e) => {{ imgB.onload = r; imgB.onerror = e; imgB.src = 'data:image/png;base64,{b}'; }})
            ]);

            const w = Math.max(imgA.width, imgB.width);
            const h = Math.max(imgA.height, imgB.height);

            const canvasA = document.createElement('canvas');
            canvasA.width = w; canvasA.height = h;
            const ctxA = canvasA.getContext('2d');
            ctxA.drawImage(imgA, 0, 0);
            const dataA = ctxA.getImageData(0, 0, w, h).data;

            const canvasB = document.createElement('canvas');
            canvasB.width = w; canvasB.height = h;
            const ctxB = canvasB.getContext('2d');
            ctxB.drawImage(imgB, 0, 0);
            const dataB = ctxB.getImageData(0, 0, w, h).data;

            let diffCount = 0;
            const totalPixels = w * h;
            const diffThreshold = 30;

            let diffCanvas, diffCtx, diffData;
            if ({gen_diff}) {{
                diffCanvas = document.createElement('canvas');
                diffCanvas.width = w; diffCanvas.height = h;
                diffCtx = diffCanvas.getContext('2d');
                diffData = diffCtx.createImageData(w, h);
            }}

            for (let i = 0; i < dataA.length; i += 4) {{
                const dr = Math.abs(dataA[i] - dataB[i]);
                const dg = Math.abs(dataA[i+1] - dataB[i+1]);
                const db = Math.abs(dataA[i+2] - dataB[i+2]);
                const isDiff = dr > diffThreshold || dg > diffThreshold || db > diffThreshold;

                if (isDiff) diffCount++;

                if ({gen_diff} && diffData) {{
                    const pi = i;
                    if (isDiff) {{
                        diffData.data[pi] = 255;
                        diffData.data[pi+1] = 0;
                        diffData.data[pi+2] = 0;
                        diffData.data[pi+3] = 255;
                    }} else {{
                        diffData.data[pi] = dataA[i] * 0.3;
                        diffData.data[pi+1] = dataA[i+1] * 0.3;
                        diffData.data[pi+2] = dataA[i+2] * 0.3;
                        diffData.data[pi+3] = 255;
                    }}
                }}
            }}

            let diffImage = null;
            if ({gen_diff} && diffCanvas && diffData) {{
                diffCtx.putImageData(diffData, 0, 0);
                diffImage = diffCanvas.toDataURL('image/png').replace('data:image/png;base64,', '');
            }}

            const mismatchPct = (diffCount / totalPixels) * 100;
            return JSON.stringify({{
                mismatch_pct: Math.round(mismatchPct * 100) / 100,
                total_pixels: totalPixels,
                diff_pixels: diffCount,
                within_threshold: mismatchPct <= {thresh},
                threshold: {thresh},
                diff_image: diffImage,
                width: w,
                height: h
            }});
        }})()"#,
        a = image_a_b64,
        b = image_b_b64,
        thresh = threshold,
        gen_diff = if generate_diff_image { "true" } else { "false" },
    );

    let result = page.evaluate(js).await.map_err(|e| Error::Cdp(e.to_string()))?;
    let text = result.into_value::<String>().unwrap_or_default();
    let parsed: PixelDiffResult = serde_json::from_str(&text)
        .map_err(|e| Error::Cdp(format!("parse pixel diff result: {e}")))?;
    Ok(parsed)
}

/// Compare two screenshot files pixel by pixel.
pub async fn pixel_diff_files(
    page: &Page,
    path_a: &str,
    path_b: &str,
    threshold: f64,
) -> Result<PixelDiffResult> {
    use base64::Engine;
    let bytes_a = std::fs::read(path_a).map_err(|e| Error::Cdp(format!("read {path_a}: {e}")))?;
    let bytes_b = std::fs::read(path_b).map_err(|e| Error::Cdp(format!("read {path_b}: {e}")))?;
    let b64_a = base64::engine::general_purpose::STANDARD.encode(&bytes_a);
    let b64_b = base64::engine::general_purpose::STANDARD.encode(&bytes_b);
    pixel_diff(page, &b64_a, &b64_b, threshold, true).await
}
