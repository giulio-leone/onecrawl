//! Dialog auto-handling via JS monkey-patching.
//!
//! Overrides window.alert, window.confirm, and window.prompt to
//! auto-handle them and record a history of dialog events.

use onecrawl_browser::Page;
use onecrawl_core::Result;
use serde::{Deserialize, Serialize};

/// A recorded dialog event.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DialogEvent {
    pub dialog_type: String,
    pub message: String,
    pub accepted: bool,
    pub response: Option<String>,
    pub timestamp: f64,
}

/// Auto-handle dialogs with the specified behavior.
pub async fn set_dialog_handler(
    page: &Page,
    accept: bool,
    prompt_text: Option<&str>,
) -> Result<()> {
    let prompt_val = match prompt_text {
        Some(t) => format!("'{}'", t.replace('\\', "\\\\").replace('\'', "\\'")),
        None => "null".to_string(),
    };
    let js = format!(
        r#"
        (() => {{
            window.__onecrawl_dialog_history = window.__onecrawl_dialog_history || [];
            const accept = {accept};
            const promptText = {prompt_val};

            window.alert = function(msg) {{
                window.__onecrawl_dialog_history.push({{
                    dialog_type: 'alert',
                    message: String(msg || ''),
                    accepted: true,
                    response: null,
                    timestamp: Date.now()
                }});
            }};

            window.confirm = function(msg) {{
                window.__onecrawl_dialog_history.push({{
                    dialog_type: 'confirm',
                    message: String(msg || ''),
                    accepted: accept,
                    response: null,
                    timestamp: Date.now()
                }});
                return accept;
            }};

            window.prompt = function(msg, defaultVal) {{
                const resp = accept ? (promptText !== null ? promptText : (defaultVal || '')) : null;
                window.__onecrawl_dialog_history.push({{
                    dialog_type: 'prompt',
                    message: String(msg || ''),
                    accepted: accept,
                    response: resp,
                    timestamp: Date.now()
                }});
                return resp;
            }};

            window.addEventListener('beforeunload', function(evt) {{
                window.__onecrawl_dialog_history.push({{
                    dialog_type: 'beforeunload',
                    message: evt.returnValue || '',
                    accepted: accept,
                    response: null,
                    timestamp: Date.now()
                }});
            }});

            return 'installed';
        }})()
        "#,
        accept = accept,
        prompt_val = prompt_val,
    );

    page.evaluate(js.as_str())
        .await
        .map_err(|e| onecrawl_core::Error::Cdp(format!("set_dialog_handler failed: {e}")))?;

    Ok(())
}

/// Get history of dialogs that have been handled.
pub async fn get_dialog_history(page: &Page) -> Result<Vec<DialogEvent>> {
    let result = page
        .evaluate("window.__onecrawl_dialog_history || []")
        .await
        .map_err(|e| onecrawl_core::Error::Cdp(format!("get_dialog_history failed: {e}")))?;

    let events: Vec<DialogEvent> = result.into_value().unwrap_or_default();

    Ok(events)
}

/// Clear dialog history.
pub async fn clear_dialog_history(page: &Page) -> Result<()> {
    page.evaluate("window.__onecrawl_dialog_history = []")
        .await
        .map_err(|e| onecrawl_core::Error::Cdp(format!("clear_dialog_history failed: {e}")))?;

    Ok(())
}
