//! Iframe enumeration and cross-frame JavaScript evaluation.
//!
//! Uses `document.querySelectorAll('iframe')` to list frames and
//! `contentWindow.eval()` for same-origin script execution.

use chromiumoxide::Page;
use onecrawl_core::Result;
use serde::{Deserialize, Serialize};

/// Metadata about an iframe on the page.
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

/// Execute JavaScript inside a specific iframe by index.
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
