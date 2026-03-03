use chromiumoxide::Page;
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
        .map_err(|e| Error::Browser(format!("read baseline failed: {e}")))?;
    let current = std::fs::read(current_path)
        .map_err(|e| Error::Browser(format!("read current failed: {e}")))?;
    compare_screenshots(&baseline, &current)
}

/// Take a screenshot and compare with a baseline file.
/// If baseline doesn't exist, saves current screenshot as the new baseline.
pub async fn visual_regression(page: &Page, baseline_path: &Path) -> Result<DiffResult> {
    let current = page
        .screenshot(chromiumoxide::page::ScreenshotParams::builder().build())
        .await
        .map_err(|e| Error::Browser(format!("visual_regression screenshot failed: {e}")))?;

    if !baseline_path.exists() {
        std::fs::write(baseline_path, &current)
            .map_err(|e| Error::Browser(format!("save baseline failed: {e}")))?;
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
        .map_err(|e| Error::Browser(format!("read baseline failed: {e}")))?;
    compare_screenshots(&baseline, &current)
}
