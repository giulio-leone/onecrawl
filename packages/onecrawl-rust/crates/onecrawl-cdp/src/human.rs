//! Human-like behavior simulation and Cloudflare challenge detection.
//!
//! Provides:
//! - Bezier-curve mouse movement before clicks
//! - Random pre/post-action delays matching human reaction times
//! - Cloudflare Bot Management challenge detection
//! - Auto-wait for CF clearance (up to configurable timeout)

use chromiumoxide::layout::Point;
use chromiumoxide::Page;
use onecrawl_core::{Error, Result};
use rand::prelude::*;
use std::time::Duration;
use tokio::time::sleep;

// ---------------------------------------------------------------------------
// Mouse movement
// ---------------------------------------------------------------------------

/// Generate a quadratic bezier curve path from `(x0,y0)` to `(x1,y1)`.
/// A random control point adds a natural curve (not a straight line).
fn bezier_path(x0: f64, y0: f64, x1: f64, y1: f64, steps: usize) -> Vec<(f64, f64)> {
    let mut rng = rand::rng();
    // Control point offset: ±30% of the distance for a gentle curve
    let dist = ((x1 - x0).powi(2) + (y1 - y0).powi(2)).sqrt().max(50.0);
    let cx = (x0 + x1) / 2.0 + rng.random_range(-dist * 0.35..dist * 0.35);
    let cy = (y0 + y1) / 2.0 + rng.random_range(-dist * 0.35..dist * 0.35);

    (0..=steps)
        .map(|i| {
            let t = i as f64 / steps as f64;
            let x = (1.0 - t).powi(2) * x0 + 2.0 * (1.0 - t) * t * cx + t.powi(2) * x1;
            let y = (1.0 - t).powi(2) * y0 + 2.0 * (1.0 - t) * t * cy + t.powi(2) * y1;
            (x, y)
        })
        .collect()
}

/// Move the mouse from `(x0,y0)` to `(x1,y1)` along a bezier curve.
/// Dispatches CDP `mouseMoved` events at 60-120 fps timing.
pub async fn mouse_move_bezier(
    page: &Page,
    x0: f64,
    y0: f64,
    x1: f64,
    y1: f64,
) -> Result<()> {
    let mut rng = rand::rng();
    let steps: usize = rng.random_range(15..28);
    let path = bezier_path(x0, y0, x1, y1, steps);

    for (x, y) in path {
        page.move_mouse(Point { x, y })
            .await
            .map_err(|e| Error::Cdp(format!("mouse_move_bezier: {e}")))?;
        // 8–16 ms between moves (simulates ~62–125 fps cursor update rate)
        sleep(Duration::from_millis(rng.random_range(8..17))).await;
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// Pre-action delays
// ---------------------------------------------------------------------------

/// Random pre-action pause simulating human reaction time (80–350 ms).
pub async fn pre_action_delay() {
    let ms = rand::rng().random_range(80u64..350);
    sleep(Duration::from_millis(ms)).await;
}

/// Short post-action pause after a click/fill (50–150 ms).
pub async fn post_action_delay() {
    let ms = rand::rng().random_range(50u64..150);
    sleep(Duration::from_millis(ms)).await;
}

// ---------------------------------------------------------------------------
// Human-like click
// ---------------------------------------------------------------------------

/// Human-like click: bezier mouse move to element center, pre-action delay, then click.
///
/// The element's bounding box is retrieved to compute the center.
/// Jitter of ±15% of element size is added so the cursor never lands
/// on the exact geometric center (a known bot-detection heuristic).
///
/// Falls back to `crate::element::click` directly if bounding box fails.
pub async fn human_click(page: &Page, selector: &str) -> Result<()> {
    let mut rng = rand::rng();

    // Try to get element center; fall back gracefully.
    let (cx, cy) = match crate::input::bounding_box(page, selector).await {
        Ok((x, y, w, h)) => {
            let jx = x + w / 2.0 + rng.random_range(-w * 0.15..w * 0.15);
            let jy = y + h / 2.0 + rng.random_range(-h * 0.15..h * 0.15);
            (jx, jy)
        }
        Err(_) => {
            // Can't get bounding box — skip mouse move, click directly.
            return crate::element::click(page, selector).await;
        }
    };

    // Move from a random near-by starting position.
    let start_x = cx + rng.random_range(-250.0f64..250.0);
    let start_y = cy + rng.random_range(-150.0f64..150.0);

    mouse_move_bezier(page, start_x, start_y, cx, cy).await?;
    pre_action_delay().await;
    crate::element::click(page, selector).await?;
    post_action_delay().await;
    Ok(())
}

// ---------------------------------------------------------------------------
// Cloudflare challenge detection
// ---------------------------------------------------------------------------

const CF_DETECT_JS: &str = r#"
(() => {
    const title = (document.title || '').toLowerCase();
    const body  = (document.body?.innerText || '').toLowerCase();
    return (
        title.includes('just a moment')           ||
        title.includes('attention required')       ||
        title.includes('one more step')            ||
        body.includes('verifying you are human')   ||
        body.includes('checking your browser')     ||
        (body.includes('please wait') && body.includes('cloudflare')) ||
        document.querySelector('#challenge-form')            !== null  ||
        document.querySelector('.cf-browser-verification')   !== null  ||
        document.querySelector('[data-translate="checking_browser"]') !== null ||
        window.location.hostname === 'challenges.cloudflare.com'
    );
})()
"#;

/// Returns `true` if the current page is showing a Cloudflare bot challenge.
pub async fn is_cloudflare_challenge(page: &Page) -> bool {
    page.evaluate(CF_DETECT_JS)
        .await
        .ok()
        .and_then(|v| v.into_value::<bool>().ok())
        .unwrap_or(false)
}

/// Wait until the Cloudflare challenge clears or `timeout_ms` elapses.
///
/// Polls every 500 ms.  Returns `true` if the challenge passed before the
/// timeout, `false` otherwise.
pub async fn wait_for_cf_clearance(page: &Page, timeout_ms: u64) -> bool {
    let start = std::time::Instant::now();
    let poll = Duration::from_millis(500);

    while start.elapsed().as_millis() < timeout_ms as u128 {
        if !is_cloudflare_challenge(page).await {
            return true;
        }
        sleep(poll).await;
    }

    // Final check after timeout
    !is_cloudflare_challenge(page).await
}
