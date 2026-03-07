//! Human-like behavior simulation and Cloudflare challenge detection.
//!
//! Provides:
//! - Bezier-curve mouse movement with easing (acceleration/deceleration)
//! - Micro-hesitations during mouse travel for realism
//! - Human-like scrolling with momentum and deceleration
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

/// Easing function: slow start → fast middle → slow end (ease-in-out cubic).
fn ease_in_out(t: f64) -> f64 {
    if t < 0.5 {
        4.0 * t * t * t
    } else {
        1.0 - (-2.0 * t + 2.0).powi(3) / 2.0
    }
}

/// Move the mouse from `(x0,y0)` to `(x1,y1)` along a bezier curve.
///
/// Uses easing for speed variation (slow start/end, fast middle) and
/// occasional micro-hesitations (~5% chance per step) for realism.
pub async fn mouse_move_bezier(
    page: &Page,
    x0: f64,
    y0: f64,
    x1: f64,
    y1: f64,
) -> Result<()> {
    let path = {
        let mut rng = rand::rng();
        let steps: usize = rng.random_range(15..28);
        bezier_path(x0, y0, x1, y1, steps)
    };
    let total = path.len().max(1) as f64;

    // Pre-compute all per-step random values to avoid holding !Send RNG across .await
    let step_randoms: Vec<(f64, bool, u64)> = {
        let mut rng = rand::rng();
        path.iter()
            .enumerate()
            .map(|(i, _)| {
                let jitter = rng.random_range(-2.0f64..2.0);
                let do_hesitate = rng.random_range(0u32..100) < 5;
                let hesitate_ms = rng.random_range(30u64..80);
                (jitter, do_hesitate, hesitate_ms)
            })
            .collect()
    };

    for (i, (x, y)) in path.into_iter().enumerate() {
        page.move_mouse(Point { x, y })
            .await
            .map_err(|e| Error::Cdp(format!("mouse_move_bezier: {e}")))?;

        let progress = i as f64 / total;
        let speed_factor = ease_in_out(progress);
        let base_delay = 6.0 + (1.0 - speed_factor) * 18.0;
        let (jitter, do_hesitate, hesitate_ms) = step_randoms[i];
        let delay_ms = (base_delay + jitter).max(4.0) as u64;
        sleep(Duration::from_millis(delay_ms)).await;

        if do_hesitate {
            sleep(Duration::from_millis(hesitate_ms)).await;
        }
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// Human-like scrolling
// ---------------------------------------------------------------------------

/// Scroll with momentum and deceleration (like a real mouse wheel).
///
/// Divides total `pixels` into a series of decreasing increments with
/// variable timing, simulating flick + coast + stop physics.
pub async fn human_scroll(page: &Page, dx_total: i64, dy_total: i64) -> Result<()> {
    let total = ((dx_total as f64).powi(2) + (dy_total as f64).powi(2)).sqrt().max(1.0);
    let direction_x = dx_total as f64 / total;
    let direction_y = dy_total as f64 / total;

    // Pre-compute random values (max ~200 steps is more than enough for any scroll)
    let (initial_velocity, decel_factors, delays) = {
        let mut rng = rand::rng();
        let vel = rng.random_range(0.4..0.7);
        let decels: Vec<f64> = (0..200).map(|_| rng.random_range(0.75..0.90)).collect();
        let dls: Vec<u64> = (0..200).map(|_| rng.random_range(12u64..30)).collect();
        (vel, decels, dls)
    };

    let mut remaining = total;
    let mut velocity = initial_velocity;
    let mut step_idx = 0usize;

    while remaining > 1.0 {
        let step = (remaining * velocity).max(1.0).min(remaining);
        let step_dx = (direction_x * step).round() as i64;
        let step_dy = (direction_y * step).round() as i64;

        if step_dx != 0 || step_dy != 0 {
            let js = format!("window.scrollBy({step_dx},{step_dy})");
            page.evaluate(js)
                .await
                .map_err(|e| Error::Cdp(format!("human_scroll: {e}")))?;
        }

        remaining -= step;
        let idx = step_idx.min(199);
        velocity *= decel_factors[idx];
        sleep(Duration::from_millis(delays[idx])).await;
        step_idx += 1;
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
    // Try to get element center; fall back gracefully.
    let bbox = crate::input::bounding_box(page, selector).await;
    let (x, y, w, h) = match bbox {
        Ok(v) => v,
        Err(_) => {
            // Can't get bounding box — skip mouse move, click directly.
            return crate::element::click(page, selector).await;
        }
    };

    // Compute all random values in a sync block to avoid !Send RNG across .await
    let (cx, cy, start_x, start_y) = {
        let mut rng = rand::rng();
        let jx = x + w / 2.0 + if w > 0.0 { rng.random_range(-w * 0.15..w * 0.15) } else { 0.0 };
        let jy = y + h / 2.0 + if h > 0.0 { rng.random_range(-h * 0.15..h * 0.15) } else { 0.0 };
        let sx = jx + rng.random_range(-250.0f64..250.0);
        let sy = jy + rng.random_range(-150.0f64..150.0);
        (jx, jy, sx, sy)
    };

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
