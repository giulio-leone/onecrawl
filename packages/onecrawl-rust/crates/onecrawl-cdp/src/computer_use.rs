//! AI Computer Use Protocol — structured observation-action loop for autonomous browser control.
//!
//! Implements an Anthropic/OpenAI computer-use-style interface that AI agents
//! can use to observe browser state and execute actions in a tight loop.

use onecrawl_browser::Page;
use onecrawl_core::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Observation from the browser state.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Observation {
    /// Current page URL.
    pub url: String,
    /// Page title.
    pub title: String,
    /// Accessibility snapshot (compact).
    pub snapshot: String,
    /// Number of interactive elements.
    pub interactive_count: usize,
    /// Screenshot as base64 (if requested).
    pub screenshot: Option<String>,
    /// Any error from the last action.
    pub last_error: Option<String>,
    /// Cursor position (if tracked).
    pub cursor: Option<(f64, f64)>,
    /// Page dimensions.
    pub viewport: Viewport,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Viewport {
    pub width: u32,
    pub height: u32,
}

/// Action that an agent can take.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum AgentAction {
    /// Click at coordinates or on element.
    #[serde(rename = "click")]
    Click {
        #[serde(flatten)]
        target: ClickTarget,
        #[serde(default)]
        button: Option<String>,
    },
    /// Type text into the focused element.
    #[serde(rename = "type")]
    Type { text: String },
    /// Press a key (Enter, Tab, Escape, Backspace, etc.).
    #[serde(rename = "key")]
    Key { key: String },
    /// Scroll the page.
    #[serde(rename = "scroll")]
    Scroll {
        x: i32,
        y: i32,
        #[serde(default)]
        delta_x: Option<i32>,
        #[serde(default)]
        delta_y: Option<i32>,
    },
    /// Navigate to a URL.
    #[serde(rename = "navigate")]
    Navigate { url: String },
    /// Wait for a duration.
    #[serde(rename = "wait")]
    Wait {
        #[serde(default = "default_wait_ms")]
        ms: u64,
    },
    /// Take a screenshot (for visual reasoning).
    #[serde(rename = "screenshot")]
    Screenshot,
    /// Get page snapshot (text observation).
    #[serde(rename = "observe")]
    Observe {
        #[serde(default)]
        include_screenshot: bool,
    },
    /// Execute JavaScript.
    #[serde(rename = "evaluate")]
    Evaluate { expression: String },
    /// Fill a form field.
    #[serde(rename = "fill")]
    Fill { selector: String, value: String },
    /// Select from dropdown.
    #[serde(rename = "select")]
    Select { selector: String, value: String },
    /// Drag from one point to another.
    #[serde(rename = "drag")]
    Drag {
        from_x: f64,
        from_y: f64,
        to_x: f64,
        to_y: f64,
    },
    /// Mark task as done (agent signals completion).
    #[serde(rename = "done")]
    Done { result: Option<String> },
    /// Mark task as failed.
    #[serde(rename = "fail")]
    Fail { reason: String },
}

fn default_wait_ms() -> u64 {
    1000
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum ClickTarget {
    Coordinates { x: f64, y: f64 },
    Selector { selector: String },
    Ref {
        #[serde(rename = "ref")]
        ref_id: String,
    },
}

/// Result of executing an action.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActionResult {
    pub success: bool,
    pub observation: Observation,
    pub action_index: usize,
    pub elapsed_ms: u64,
}

/// Execute a single action and return the resulting observation.
pub async fn execute_action(
    page: &Page,
    action: &AgentAction,
    action_index: usize,
) -> Result<ActionResult> {
    let start = std::time::Instant::now();
    let mut error: Option<String> = None;

    match action {
        AgentAction::Click { target, button: _ } => match target {
            ClickTarget::Coordinates { x, y } => {
                let js = format!("document.elementFromPoint({x}, {y})?.click()");
                if let Err(e) = page.evaluate(js).await {
                    error = Some(format!("Click at ({x},{y}) failed: {e}"));
                }
            }
            ClickTarget::Selector { selector } => {
                let resolved = crate::accessibility::resolve_ref(selector);
                if let Err(e) = crate::element::click(page, &resolved).await {
                    error = Some(format!("Click '{selector}' failed: {e}"));
                }
            }
            ClickTarget::Ref { ref_id } => {
                let resolved = crate::accessibility::resolve_ref(ref_id);
                if let Err(e) = crate::element::click(page, &resolved).await {
                    error = Some(format!("Click ref '{ref_id}' failed: {e}"));
                }
            }
        },
        AgentAction::Type { text } => {
            let text_json = serde_json::to_string(text).unwrap_or_default();
            let js = format!(
                "document.activeElement?.value !== undefined \
                 ? (document.activeElement.value += {text_json}, \
                    document.activeElement.dispatchEvent(\
                      new Event('input', {{bubbles: true}}))) \
                 : null"
            );
            if let Err(e) = page.evaluate(js).await {
                error = Some(format!("Type failed: {e}"));
            }
        }
        AgentAction::Key { key } => {
            let key_json = serde_json::to_string(key).unwrap_or_default();
            let js = format!(
                "document.activeElement?.dispatchEvent(\
                   new KeyboardEvent('keydown', {{key: {key_json}, bubbles: true}})); \
                 document.activeElement?.dispatchEvent(\
                   new KeyboardEvent('keyup', {{key: {key_json}, bubbles: true}}))"
            );
            if let Err(e) = page.evaluate(js).await {
                error = Some(format!("Key '{key}' failed: {e}"));
            }
        }
        AgentAction::Scroll {
            x: _,
            y: _,
            delta_x,
            delta_y,
        } => {
            let dx = delta_x.unwrap_or(0);
            let dy = delta_y.unwrap_or(-300);
            let js = format!("window.scrollBy({dx}, {dy})");
            if let Err(e) = page.evaluate(js).await {
                error = Some(format!("Scroll failed: {e}"));
            }
        }
        AgentAction::Navigate { url } => {
            if let Err(e) = crate::navigation::goto(page, url).await {
                error = Some(format!("Navigate to '{url}' failed: {e}"));
            }
        }
        AgentAction::Wait { ms } => {
            tokio::time::sleep(tokio::time::Duration::from_millis(*ms)).await;
        }
        AgentAction::Screenshot | AgentAction::Observe { .. } => {
            // Handled via observation below.
        }
        AgentAction::Evaluate { expression } => {
            if let Err(e) = page.evaluate(expression.clone()).await {
                error = Some(format!("Evaluate failed: {e}"));
            }
        }
        AgentAction::Fill { selector, value } => {
            let resolved = crate::accessibility::resolve_ref(selector);
            if let Err(e) = crate::element::type_text(page, &resolved, value).await {
                error = Some(format!("Fill '{selector}' failed: {e}"));
            }
        }
        AgentAction::Select { selector, value } => {
            let resolved = crate::accessibility::resolve_ref(selector);
            let sel_json = serde_json::to_string(&resolved).unwrap_or_default();
            let val_json = serde_json::to_string(value).unwrap_or_default();
            let js = format!(
                "(() => {{ \
                   const el = document.querySelector({sel_json}); \
                   if (el) {{ el.value = {val_json}; \
                     el.dispatchEvent(new Event('change', {{bubbles: true}})); \
                     return true; }} \
                   return false; \
                 }})()"
            );
            match page.evaluate(js).await {
                Ok(r) => {
                    if !r.into_value::<bool>().unwrap_or(false) {
                        error = Some(format!("Select '{selector}' not found"));
                    }
                }
                Err(e) => error = Some(format!("Select failed: {e}")),
            }
        }
        AgentAction::Drag {
            from_x,
            from_y,
            to_x,
            to_y,
        } => {
            let js = format!(
                "(() => {{ \
                   const el = document.elementFromPoint({from_x}, {from_y}); \
                   if (!el) return false; \
                   el.dispatchEvent(new MouseEvent('mousedown', \
                     {{clientX: {from_x}, clientY: {from_y}, bubbles: true}})); \
                   el.dispatchEvent(new MouseEvent('mousemove', \
                     {{clientX: {to_x}, clientY: {to_y}, bubbles: true}})); \
                   el.dispatchEvent(new MouseEvent('mouseup', \
                     {{clientX: {to_x}, clientY: {to_y}, bubbles: true}})); \
                   return true; \
                 }})()"
            );
            if let Err(e) = page.evaluate(js).await {
                error = Some(format!("Drag failed: {e}"));
            }
        }
        AgentAction::Done { .. } | AgentAction::Fail { .. } => {
            // Terminal actions — just observe.
        }
    }

    let include_screenshot = matches!(
        action,
        AgentAction::Screenshot | AgentAction::Observe { include_screenshot: true }
    );
    let obs = observe(page, error.clone(), include_screenshot).await?;

    Ok(ActionResult {
        success: error.is_none(),
        observation: obs,
        action_index,
        elapsed_ms: start.elapsed().as_millis() as u64,
    })
}

/// Get current page observation without taking an action.
pub async fn observe(
    page: &Page,
    last_error: Option<String>,
    include_screenshot: bool,
) -> Result<Observation> {
    let url = page
        .evaluate("window.location.href")
        .await
        .map(|v| v.into_value::<String>().unwrap_or_default())
        .unwrap_or_default();

    let title = page
        .evaluate("document.title || ''")
        .await
        .map(|v| v.into_value::<String>().unwrap_or_default())
        .unwrap_or_default();

    let opts = crate::accessibility::AgentSnapshotOptions {
        interactive_only: false,
        cursor: true,
        compact: true,
        depth: Some(10),
        selector: None,
    };
    let snap = crate::accessibility::agent_snapshot(page, &opts)
        .await
        .unwrap_or_else(|_| crate::accessibility::AgentSnapshot {
            snapshot: String::new(),
            refs: HashMap::new(),
            total: 0,
            interactive_count: 0,
        });

    let screenshot = if include_screenshot {
        match crate::screenshot::screenshot_viewport(page).await {
            Ok(bytes) => {
                use base64::Engine as _;
                Some(base64::engine::general_purpose::STANDARD.encode(&bytes))
            }
            Err(_) => None,
        }
    } else {
        None
    };

    let vp_js = "JSON.stringify({width: window.innerWidth, height: window.innerHeight})";
    let viewport = page
        .evaluate(vp_js)
        .await
        .ok()
        .and_then(|v| v.into_value::<String>().ok())
        .and_then(|s| serde_json::from_str::<Viewport>(&s).ok())
        .unwrap_or(Viewport {
            width: 1280,
            height: 720,
        });

    Ok(Observation {
        url,
        title,
        snapshot: snap.snapshot,
        interactive_count: snap.interactive_count,
        screenshot,
        last_error,
        cursor: None,
        viewport,
    })
}
