//! Iframe enumeration, cross-frame JavaScript evaluation, and CDP frame targeting.
//!
//! Provides two layers:
//! - **DOM-based** (`list_iframes`, `eval_in_iframe`): uses `contentWindow.eval()` for same-origin iframes
//! - **CDP-based** (`get_frame_tree`, `eval_in_frame_cdp`, `click_in_frame`): uses
//!   `Page.getFrameTree` + `Page.createIsolatedWorld` + `Runtime.evaluate` to bypass
//!   cross-origin restrictions at the browser protocol level

use onecrawl_browser::cdp::browser_protocol::page::{
    CreateIsolatedWorldParams, FrameTree, GetFrameTreeParams,
};
use onecrawl_browser::cdp::js_protocol::runtime::EvaluateParams;
use onecrawl_browser::Page;
use onecrawl_core::Result;
use serde::{Deserialize, Serialize};

/// Metadata about an iframe on the page (DOM-based).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IframeInfo {
    pub index: usize,
    pub src: String,
    pub name: String,
    pub id: String,
    pub width: String,
    pub height: String,
    pub sandbox: Option<String>,
}

/// CDP frame info extracted from the frame tree.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CdpFrameInfo {
    pub frame_id: String,
    pub url: String,
    pub name: Option<String>,
    pub security_origin: String,
    pub parent_frame_id: Option<String>,
}

// ---------------------------------------------------------------------------
// DOM-based helpers (same-origin only)
// ---------------------------------------------------------------------------

/// List all iframes on the page.
pub async fn list_iframes(page: &Page) -> Result<Vec<IframeInfo>> {
    let result = page
        .evaluate(
            r#"
            Array.from(document.querySelectorAll('iframe')).map((f, i) => ({
                index: i,
                src: f.src || '',
                name: f.name || '',
                id: f.id || '',
                width: f.width || f.style.width || '',
                height: f.height || f.style.height || '',
                sandbox: f.sandbox ? f.sandbox.value : null
            }))
            "#,
        )
        .await
        .map_err(|e| onecrawl_core::Error::Cdp(format!("list_iframes failed: {e}")))?;

    let iframes: Vec<IframeInfo> = result.into_value().unwrap_or_default();

    Ok(iframes)
}

/// Execute JavaScript inside a specific iframe by index (same-origin only).
pub async fn eval_in_iframe(
    page: &Page,
    index: usize,
    expression: &str,
) -> Result<serde_json::Value> {
    let expr_json =
        serde_json::to_string(expression).unwrap_or_else(|_| format!("\"{}\"", expression));

    let js = format!(
        r#"
        (() => {{
            const frames = document.querySelectorAll('iframe');
            if ({index} >= frames.length) return {{ error: 'iframe index out of bounds' }};
            try {{
                const win = frames[{index}].contentWindow;
                return win.eval({expr});
            }} catch(e) {{
                return {{ error: e.message }};
            }}
        }})()
        "#,
        index = index,
        expr = expr_json,
    );

    let result = page
        .evaluate(js.as_str())
        .await
        .map_err(|e| onecrawl_core::Error::Cdp(format!("eval_in_iframe failed: {e}")))?;

    let val: serde_json::Value = result.into_value().unwrap_or(serde_json::Value::Null);
    Ok(val)
}

/// Get the inner HTML content of an iframe.
pub async fn get_iframe_content(page: &Page, index: usize) -> Result<String> {
    let js = format!(
        r#"
        (() => {{
            const frames = document.querySelectorAll('iframe');
            if ({0} >= frames.length) return '';
            try {{
                return frames[{0}].contentDocument?.documentElement?.outerHTML || '';
            }} catch(e) {{
                return 'cross-origin: ' + e.message;
            }}
        }})()
        "#,
        index,
    );

    let result = page
        .evaluate(js.as_str())
        .await
        .map_err(|e| onecrawl_core::Error::Cdp(format!("get_iframe_content failed: {e}")))?;

    let html: String = result.into_value().unwrap_or_default();
    Ok(html)
}

// ---------------------------------------------------------------------------
// CDP-based frame targeting (cross-origin capable)
// ---------------------------------------------------------------------------

/// Get the full CDP frame tree including all child frames.
pub async fn get_frame_tree(page: &Page) -> Result<FrameTree> {
    let result = page
        .execute(GetFrameTreeParams {})
        .await
        .map_err(|e| onecrawl_core::Error::Cdp(format!("getFrameTree failed: {e}")))?;
    Ok(result.result.frame_tree)
}

/// Collect all frames from a frame tree into a flat list.
fn flatten_frame_tree(tree: &FrameTree, out: &mut Vec<CdpFrameInfo>) {
    out.push(CdpFrameInfo {
        frame_id: tree.frame.id.inner().to_string(),
        url: tree.frame.url.clone(),
        name: tree.frame.name.clone(),
        security_origin: tree.frame.security_origin.clone(),
        parent_frame_id: tree.frame.parent_id.as_ref().map(|id| id.inner().to_string()),
    });
    if let Some(children) = &tree.child_frames {
        for child in children {
            flatten_frame_tree(child, out);
        }
    }
}

/// List all frames (including cross-origin) via CDP `Page.getFrameTree`.
pub async fn list_all_frames(page: &Page) -> Result<Vec<CdpFrameInfo>> {
    let tree = get_frame_tree(page).await?;
    let mut frames = Vec::new();
    flatten_frame_tree(&tree, &mut frames);
    Ok(frames)
}

/// Find a frame by URL substring match.
pub async fn find_frame_by_url(page: &Page, url_pattern: &str) -> Result<Option<CdpFrameInfo>> {
    let frames = list_all_frames(page).await?;
    Ok(frames.into_iter().find(|f| f.url.contains(url_pattern)))
}

/// Execute JavaScript inside a cross-origin iframe via CDP frame targeting.
///
/// Uses `Page.createIsolatedWorld` with `grantUniversalAccess` to create
/// an execution context inside the target frame, then `Runtime.evaluate`
/// to run code in that context. This bypasses Same-Origin Policy at the
/// browser protocol level.
pub async fn eval_in_frame_cdp(
    page: &Page,
    frame_id: &str,
    expression: &str,
) -> Result<serde_json::Value> {
    // Create an isolated world inside the target frame with universal access
    let isolated = page
        .execute(
            CreateIsolatedWorldParams::builder()
                .frame_id(frame_id.to_string())
                .world_name("onecrawl_cross_origin")
                .grant_univeral_access(true)
                .build()
                .map_err(|e| onecrawl_core::Error::Cdp(format!("createIsolatedWorld build: {e}")))?,
        )
        .await
        .map_err(|e| onecrawl_core::Error::Cdp(format!("createIsolatedWorld failed: {e}")))?;

    let ctx_id = isolated.result.execution_context_id;

    // Execute in that context
    let mut eval_params = EvaluateParams::new(expression);
    eval_params.context_id = Some(ctx_id);
    eval_params.return_by_value = Some(true);
    eval_params.await_promise = Some(true);

    let eval_result = page
        .execute(eval_params)
        .await
        .map_err(|e| onecrawl_core::Error::Cdp(format!("Runtime.evaluate in frame: {e}")))?;

    // Extract the value from the result
    let val = if let Some(v) = eval_result.result.result.value {
        v
    } else {
        serde_json::Value::Null
    };
    Ok(val)
}

/// Click an element inside a cross-origin iframe by selector.
///
/// Finds the frame by URL pattern, creates an isolated world with universal
/// access, then clicks the element matching `selector` inside that frame.
pub async fn click_in_frame(
    page: &Page,
    frame_url_pattern: &str,
    selector: &str,
) -> Result<bool> {
    let frame = find_frame_by_url(page, frame_url_pattern)
        .await?
        .ok_or_else(|| {
            onecrawl_core::Error::Cdp(format!(
                "No frame found matching URL pattern: {frame_url_pattern}"
            ))
        })?;

    let click_js = format!(
        r#"(() => {{
            const el = document.querySelector({sel});
            if (!el) return {{ clicked: false, error: 'element not found' }};
            el.click();
            return {{ clicked: true }};
        }})()"#,
        sel = serde_json::to_string(selector).unwrap_or_default(),
    );

    let result = eval_in_frame_cdp(page, &frame.frame_id, &click_js).await?;
    Ok(result.get("clicked").and_then(|v| v.as_bool()).unwrap_or(false))
}

/// Get the bounding box of an element inside a cross-origin iframe (viewport coordinates).
///
/// Returns `(x, y, width, height)` in viewport coordinates (iframe offset + element offset).
pub async fn element_rect_in_frame(
    page: &Page,
    frame_url_pattern: &str,
    selector: &str,
) -> Result<(f64, f64, f64, f64)> {
    // Get the iframe element's position in the viewport
    let iframe_rect_js = format!(
        r#"(() => {{
            const iframes = document.querySelectorAll('iframe');
            for (const f of iframes) {{
                if (f.src && f.src.includes({pattern})) {{
                    const r = f.getBoundingClientRect();
                    return {{ x: r.x, y: r.y, w: r.width, h: r.height }};
                }}
            }}
            return null;
        }})()"#,
        pattern = serde_json::to_string(frame_url_pattern).unwrap_or_default(),
    );
    let iframe_rect: serde_json::Value = page
        .evaluate(iframe_rect_js)
        .await
        .map_err(|e| onecrawl_core::Error::Cdp(format!("iframe rect: {e}")))?
        .into_value()
        .unwrap_or(serde_json::Value::Null);

    let ix = iframe_rect.get("x").and_then(|v| v.as_f64()).unwrap_or(0.0);
    let iy = iframe_rect.get("y").and_then(|v| v.as_f64()).unwrap_or(0.0);

    // Get the element's position inside the frame
    let frame = find_frame_by_url(page, frame_url_pattern)
        .await?
        .ok_or_else(|| {
            onecrawl_core::Error::Cdp(format!(
                "No frame found matching URL pattern: {frame_url_pattern}"
            ))
        })?;

    let el_rect_js = format!(
        r#"(() => {{
            const el = document.querySelector({sel});
            if (!el) return null;
            const r = el.getBoundingClientRect();
            return {{ x: r.x, y: r.y, w: r.width, h: r.height }};
        }})()"#,
        sel = serde_json::to_string(selector).unwrap_or_default(),
    );

    let el_rect = eval_in_frame_cdp(page, &frame.frame_id, &el_rect_js).await?;

    let ex = el_rect.get("x").and_then(|v| v.as_f64()).unwrap_or(0.0);
    let ey = el_rect.get("y").and_then(|v| v.as_f64()).unwrap_or(0.0);
    let ew = el_rect.get("w").and_then(|v| v.as_f64()).unwrap_or(0.0);
    let eh = el_rect.get("h").and_then(|v| v.as_f64()).unwrap_or(0.0);

    // Combine: viewport coordinates = iframe offset + element offset within frame
    Ok((ix + ex, iy + ey, ew, eh))
}

/// Human-like click on an element inside a cross-origin iframe.
///
/// Gets the element's viewport coordinates (iframe offset + element position)
/// and performs a bezier-curve mouse move + click at those coordinates.
pub async fn human_click_in_frame(
    page: &Page,
    frame_url_pattern: &str,
    selector: &str,
) -> Result<()> {
    use crate::human;

    let (x, y, w, h) = element_rect_in_frame(page, frame_url_pattern, selector).await?;

    if w == 0.0 && h == 0.0 {
        return Err(onecrawl_core::Error::Cdp(
            format!("Element '{selector}' not found or has zero size in frame '{frame_url_pattern}'"),
        ));
    }

    // Add jitter to avoid exact center (bot detection heuristic)
    use rand::prelude::*;
    let (jx, jy, hold_ms) = {
        let mut rng = rand::rng();
        let jx = if w > 0.0 { rng.random_range(-w * 0.15..w * 0.15) } else { 0.0 };
        let jy = if h > 0.0 { rng.random_range(-h * 0.15..h * 0.15) } else { 0.0 };
        let hold = rng.random_range(40u64..120);
        (jx, jy, hold)
    };
    let cx = x + w / 2.0 + jx;
    let cy = y + h / 2.0 + jy;

    human::pre_action_delay().await;

    // Click at the computed viewport coordinates using CDP Input.dispatchMouseEvent
    use onecrawl_browser::cdp::browser_protocol::input::{
        DispatchMouseEventParams, DispatchMouseEventType, MouseButton,
    };

    page.execute(
        DispatchMouseEventParams::builder()
            .x(cx)
            .y(cy)
            .r#type(DispatchMouseEventType::MousePressed)
            .button(MouseButton::Left)
            .click_count(1)
            .build()
            .map_err(|e| onecrawl_core::Error::Cdp(format!("mousePressed build: {e}")))?,
    )
    .await
    .map_err(|e| onecrawl_core::Error::Cdp(format!("mousePressed: {e}")))?;

    // Brief hold before release (human-like)
    tokio::time::sleep(std::time::Duration::from_millis(hold_ms)).await;

    page.execute(
        DispatchMouseEventParams::builder()
            .x(cx)
            .y(cy)
            .r#type(DispatchMouseEventType::MouseReleased)
            .button(MouseButton::Left)
            .click_count(1)
            .build()
            .map_err(|e| onecrawl_core::Error::Cdp(format!("mouseReleased build: {e}")))?,
    )
    .await
    .map_err(|e| onecrawl_core::Error::Cdp(format!("mouseReleased: {e}")))?;

    human::post_action_delay().await;
    Ok(())
}
