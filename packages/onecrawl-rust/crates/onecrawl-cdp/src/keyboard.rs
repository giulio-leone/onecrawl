//! Keyboard input via CDP Input domain.

use chromiumoxide::Page;
use onecrawl_core::{Error, Result};

/// Press a single key (keyDown + keyUp). Supports special keys:
/// `Enter`, `Tab`, `Escape`, `Backspace`, `Delete`, `ArrowUp`, `ArrowDown`,
/// `ArrowLeft`, `ArrowRight`, `Home`, `End`, `PageUp`, `PageDown`,
/// `F1`-`F12`, and single characters.
pub async fn press_key(page: &Page, key: &str) -> Result<()> {
    let js = format!(
        r#"
        (() => {{
            const key = '{key}';
            const opts = {{ key, code: key, bubbles: true, cancelable: true }};
            document.activeElement.dispatchEvent(new KeyboardEvent('keydown', opts));
            document.activeElement.dispatchEvent(new KeyboardEvent('keyup', opts));
        }})()
        "#,
        key = key.replace('\'', "\\'")
    );
    page.evaluate(js)
        .await
        .map_err(|e| Error::Browser(format!("press_key failed: {e}")))?;
    Ok(())
}

/// Send a keyboard shortcut (e.g., "Control+a", "Meta+c", "Shift+Enter").
/// Modifier keys: `Control`, `Meta`, `Alt`, `Shift`.
pub async fn keyboard_shortcut(page: &Page, shortcut: &str) -> Result<()> {
    let parts: Vec<&str> = shortcut.split('+').collect();
    if parts.is_empty() {
        return Err(Error::Browser("empty shortcut".into()));
    }

    let key = parts.last().unwrap();
    let modifiers: Vec<&str> = parts[..parts.len() - 1].to_vec();

    let ctrl = modifiers.contains(&"Control") || modifiers.contains(&"Ctrl");
    let meta = modifiers.contains(&"Meta") || modifiers.contains(&"Cmd");
    let alt = modifiers.contains(&"Alt");
    let shift = modifiers.contains(&"Shift");

    let js = format!(
        r#"
        (() => {{
            const opts = {{
                key: '{key}',
                code: '{key}',
                ctrlKey: {ctrl},
                metaKey: {meta},
                altKey: {alt},
                shiftKey: {shift},
                bubbles: true,
                cancelable: true,
            }};
            const el = document.activeElement || document.body;
            el.dispatchEvent(new KeyboardEvent('keydown', opts));
            el.dispatchEvent(new KeyboardEvent('keyup', opts));
        }})()
        "#,
        key = key.replace('\'', "\\'"),
        ctrl = ctrl,
        meta = meta,
        alt = alt,
        shift = shift,
    );
    page.evaluate(js)
        .await
        .map_err(|e| Error::Browser(format!("keyboard_shortcut failed: {e}")))?;
    Ok(())
}

/// Hold a key down (fires keydown only).
pub async fn key_down(page: &Page, key: &str) -> Result<()> {
    let js = format!(
        r#"
        (() => {{
            const el = document.activeElement || document.body;
            el.dispatchEvent(new KeyboardEvent('keydown', {{
                key: '{key}', code: '{key}', bubbles: true, cancelable: true
            }}));
        }})()
        "#,
        key = key.replace('\'', "\\'")
    );
    page.evaluate(js)
        .await
        .map_err(|e| Error::Browser(format!("key_down failed: {e}")))?;
    Ok(())
}

/// Release a key (fires keyup only).
pub async fn key_up(page: &Page, key: &str) -> Result<()> {
    let js = format!(
        r#"
        (() => {{
            const el = document.activeElement || document.body;
            el.dispatchEvent(new KeyboardEvent('keyup', {{
                key: '{key}', code: '{key}', bubbles: true, cancelable: true
            }}));
        }})()
        "#,
        key = key.replace('\'', "\\'")
    );
    page.evaluate(js)
        .await
        .map_err(|e| Error::Browser(format!("key_up failed: {e}")))?;
    Ok(())
}

/// Fill an input field: clears existing value, then types new text.
pub async fn fill(page: &Page, selector: &str, value: &str) -> Result<()> {
    let js = format!(
        r#"
        (() => {{
            const el = document.querySelector('{selector}');
            if (!el) throw new Error('element not found: {selector}');
            el.focus();
            el.value = '';
            el.dispatchEvent(new Event('input', {{ bubbles: true }}));
            el.value = '{value}';
            el.dispatchEvent(new Event('input', {{ bubbles: true }}));
            el.dispatchEvent(new Event('change', {{ bubbles: true }}));
        }})()
        "#,
        selector = selector.replace('\'', "\\'"),
        value = value.replace('\'', "\\'"),
    );
    page.evaluate(js)
        .await
        .map_err(|e| Error::Browser(format!("fill failed: {e}")))?;
    Ok(())
}
