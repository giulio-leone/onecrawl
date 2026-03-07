//! Annotated screenshot and adaptive retry for computer-use workflows.

use chromiumoxide::Page;
use onecrawl_core::{Error, Result};
use serde_json::Value;

/// Take a screenshot with numbered overlays on interactive elements.
/// Returns the element map with coordinates + base64 screenshot.
pub async fn annotated_screenshot(page: &Page) -> Result<Value> {
    // First, inject numbered overlays
    let inject_js = r#"
        (() => {
            // Remove any previous annotations
            document.querySelectorAll('[data-onecrawl-annotation]').forEach(el => el.remove());
            
            const interactive = document.querySelectorAll(
                'a, button, input, select, textarea, [role="button"], [role="link"], [role="checkbox"], [role="radio"], [tabindex]:not([tabindex="-1"])'
            );
            
            const elements = [];
            let counter = 0;
            
            interactive.forEach(el => {
                const rect = el.getBoundingClientRect();
                if (rect.width === 0 || rect.height === 0) return;
                if (rect.top > window.innerHeight || rect.bottom < 0) return;
                if (rect.left > window.innerWidth || rect.right < 0) return;
                
                counter++;
                
                // Create annotation overlay
                const badge = document.createElement('div');
                badge.setAttribute('data-onecrawl-annotation', counter);
                badge.style.cssText = `
                    position: fixed;
                    left: ${rect.left - 2}px;
                    top: ${rect.top - 2}px;
                    width: ${rect.width + 4}px;
                    height: ${rect.height + 4}px;
                    border: 2px solid rgba(255, 0, 0, 0.7);
                    border-radius: 3px;
                    pointer-events: none;
                    z-index: 999999;
                    box-sizing: border-box;
                `;
                
                const label = document.createElement('div');
                label.setAttribute('data-onecrawl-annotation', counter);
                label.textContent = counter;
                label.style.cssText = `
                    position: fixed;
                    left: ${rect.left - 2}px;
                    top: ${rect.top - 18}px;
                    background: rgba(255, 0, 0, 0.85);
                    color: white;
                    font-size: 11px;
                    font-weight: bold;
                    font-family: monospace;
                    padding: 1px 4px;
                    border-radius: 2px;
                    pointer-events: none;
                    z-index: 999999;
                    line-height: 14px;
                `;
                
                document.body.appendChild(badge);
                document.body.appendChild(label);
                
                const tag = el.tagName.toLowerCase();
                const text = (el.innerText || el.textContent || '').trim().substring(0, 80);
                const ariaLabel = el.getAttribute('aria-label') || '';
                const type = el.getAttribute('type') || '';
                const href = el.getAttribute('href') || '';
                
                elements.push({
                    number: counter,
                    tag,
                    text: text || ariaLabel || type || href || `[${tag}]`,
                    bounds: {
                        x: Math.round(rect.x),
                        y: Math.round(rect.y),
                        width: Math.round(rect.width),
                        height: Math.round(rect.height),
                        center_x: Math.round(rect.x + rect.width / 2),
                        center_y: Math.round(rect.y + rect.height / 2)
                    }
                });
            });
            
            return JSON.stringify({ elements, count: counter });
        })()
    "#.to_string();

    let result = page.evaluate(inject_js).await
        .map_err(|e| Error::Cdp(format!("annotated_screenshot inject failed: {e}")))?;
    let map_str: String = result
        .into_value()
        .unwrap_or_else(|_| r#"{"elements":[],"count":0}"#.to_string());
    let element_map: Value =
        serde_json::from_str(&map_str).unwrap_or(serde_json::json!({}));

    // Take screenshot with annotations visible
    let screenshot_bytes =
        crate::screenshot::screenshot_viewport(page).await?;

    // Remove annotations
    let cleanup_js = r#"
        document.querySelectorAll('[data-onecrawl-annotation]').forEach(el => el.remove());
    "#.to_string();
    let _ = page.evaluate(cleanup_js).await;

    // Encode screenshot as base64
    use base64::Engine as _;
    let b64 =
        base64::engine::general_purpose::STANDARD.encode(&screenshot_bytes);

    Ok(serde_json::json!({
        "action": "annotated_screenshot",
        "screenshot_base64": b64,
        "mime_type": "image/png",
        "element_map": element_map
    }))
}

/// Adaptive retry: try an action, if it fails try alternative strategies.
pub async fn adaptive_retry(
    page: &Page,
    action_js: &str,
    max_retries: usize,
    strategies: &[String],
) -> Result<Value> {
    let mut attempts = Vec::new();

    // Try main action first
    match page.evaluate(action_js.to_string()).await {
        Ok(val) => {
            let result: String = val
                .into_value()
                .unwrap_or_else(|_| "null".to_string());
            if result != "null" && result != "false" && result != "undefined" {
                return Ok(serde_json::json!({
                    "status": "success",
                    "strategy": "primary",
                    "attempt": 1,
                    "result": result,
                    "attempts": [{
                        "strategy": "primary",
                        "success": true,
                        "result": result
                    }]
                }));
            }
            attempts.push(serde_json::json!({
                "strategy": "primary",
                "success": false,
                "result": result
            }));
        }
        Err(e) => {
            attempts.push(serde_json::json!({
                "strategy": "primary",
                "success": false,
                "error": e.to_string()
            }));
        }
    }

    // Try alternative strategies
    for (i, strategy_js) in strategies.iter().enumerate().take(max_retries) {
        // Small delay between retries
        tokio::time::sleep(std::time::Duration::from_millis(500)).await;

        match page.evaluate(strategy_js.to_string()).await {
            Ok(val) => {
                let result: String = val
                    .into_value()
                    .unwrap_or_else(|_| "null".to_string());
                if result != "null"
                    && result != "false"
                    && result != "undefined"
                {
                    attempts.push(serde_json::json!({
                        "strategy": format!("alternative_{}", i + 1),
                        "success": true,
                        "result": result
                    }));
                    return Ok(serde_json::json!({
                        "status": "success",
                        "strategy": format!("alternative_{}", i + 1),
                        "attempt": i + 2,
                        "result": result,
                        "attempts": attempts
                    }));
                }
                attempts.push(serde_json::json!({
                    "strategy": format!("alternative_{}", i + 1),
                    "success": false,
                    "result": result
                }));
            }
            Err(e) => {
                attempts.push(serde_json::json!({
                    "strategy": format!("alternative_{}", i + 1),
                    "success": false,
                    "error": e.to_string()
                }));
            }
        }
    }

    Ok(serde_json::json!({
        "status": "all_failed",
        "total_attempts": attempts.len(),
        "attempts": attempts
    }))
}
