use chromiumoxide::cdp::browser_protocol::input::InsertTextParams;
use chromiumoxide::Page;
use onecrawl_core::{Error, Result};

/// Click an element by CSS selector.
///
/// Uses CDP `Input.dispatchMouseEvent` (mousePressed + mouseReleased) followed by a
/// JavaScript synthetic `click` event.  The synthetic event is required for React /
/// SPA frameworks because CDP mouse events do NOT automatically fire the browser's
/// `click` event when dispatched via the DevTools protocol.
pub async fn click(page: &Page, selector: &str) -> Result<()> {
    let el = page
        .find_element(selector)
        .await
        .map_err(|e| Error::Cdp(format!("element not found: {e}")))?;
    el.click()
        .await
        .map_err(|e| Error::Cdp(format!("click failed: {e}")))?;
    // Also dispatch a synthetic click so React / SPA synthetic event handlers fire.
    let esc = selector.replace('\\', "\\\\").replace('`', "\\`");
    page.evaluate(format!(
        "document.querySelector(`{esc}`)?.dispatchEvent(new MouseEvent('click', {{bubbles:true, cancelable:true, view:window}}))"
    ))
    .await
    .map_err(|e| Error::Cdp(format!("synthetic click failed: {e}")))?;
    Ok(())
}

/// Type text into a focused element (key-by-key).
///
/// ASCII characters are dispatched via CDP key events (realistic keyDown/keyUp).
/// Non-ASCII characters (emoji, CJK, accented, etc.) use `Input.insertText` which
/// emulates IME/emoji-keyboard input — the same path a real user would take.
pub async fn type_text(page: &Page, selector: &str, text: &str) -> Result<()> {
    let el = page
        .find_element(selector)
        .await
        .map_err(|e| Error::Cdp(format!("element not found: {e}")))?;
    el.click()
        .await
        .map_err(|e| Error::Cdp(format!("focus failed: {e}")))?;

    let mut ascii_buf = String::new();
    for ch in text.chars() {
        if ch.is_ascii() {
            ascii_buf.push(ch);
        } else {
            if !ascii_buf.is_empty() {
                el.type_str(&ascii_buf)
                    .await
                    .map_err(|e| Error::Cdp(format!("type failed: {e}")))?;
                ascii_buf.clear();
            }
            let s = ch.to_string();
            page.execute(InsertTextParams::from(s))
                .await
                .map_err(|e| Error::Cdp(format!("insertText failed: {e}")))?;
        }
    }
    if !ascii_buf.is_empty() {
        el.type_str(&ascii_buf)
            .await
            .map_err(|e| Error::Cdp(format!("type failed: {e}")))?;
    }
    Ok(())
}

/// Focus an element.
pub async fn focus(page: &Page, selector: &str) -> Result<()> {
    let el = page
        .find_element(selector)
        .await
        .map_err(|e| Error::Cdp(format!("element not found: {e}")))?;
    el.click()
        .await
        .map_err(|e| Error::Cdp(format!("focus failed: {e}")))?;
    Ok(())
}

/// Hover over an element.
pub async fn hover(page: &Page, selector: &str) -> Result<()> {
    page.evaluate(format!(
        "document.querySelector('{}')?.dispatchEvent(new MouseEvent('mouseover', {{bubbles: true}}))",
        selector.replace('\'', "\\'")
    ))
    .await
    .map_err(|e| Error::Cdp(format!("hover failed: {e}")))?;
    Ok(())
}

/// Scroll to an element.
pub async fn scroll_into_view(page: &Page, selector: &str) -> Result<()> {
    page.evaluate(format!(
        "document.querySelector('{}')?.scrollIntoView({{behavior: 'smooth', block: 'center'}})",
        selector.replace('\'', "\\'")
    ))
    .await
    .map_err(|e| Error::Cdp(format!("scroll_into_view failed: {e}")))?;
    Ok(())
}

/// Get text content of an element.
pub async fn get_text(page: &Page, selector: &str) -> Result<String> {
    let el = page
        .find_element(selector)
        .await
        .map_err(|e| Error::Cdp(format!("element not found: {e}")))?;
    let text = el
        .inner_text()
        .await
        .map_err(|e| Error::Cdp(format!("get_text failed: {e}")))?
        .unwrap_or_default();
    Ok(text)
}

/// Evaluate JavaScript in the page context.
pub async fn evaluate(page: &Page, expression: &str) -> Result<serde_json::Value> {
    let result = page
        .evaluate(expression)
        .await
        .map_err(|e| Error::Cdp(format!("eval failed: {e}")))?
        .into_value::<serde_json::Value>()
        .map_err(|e| Error::Cdp(format!("parse result failed: {e}")))?;
    Ok(result)
}

/// Double-click an element by CSS selector.
pub async fn double_click(page: &Page, selector: &str) -> Result<()> {
    page.evaluate(format!(
        "(() => {{ const el = document.querySelector('{}'); \
         if (!el) throw new Error('not found'); \
         el.dispatchEvent(new MouseEvent('dblclick', {{bubbles: true}})); \
         }})()",
        selector.replace('\'', "\\'")
    ))
    .await
    .map_err(|e| Error::Cdp(format!("double_click failed: {e}")))?;
    Ok(())
}

/// Check a checkbox element (sets `checked = true`).
pub async fn check(page: &Page, selector: &str) -> Result<()> {
    page.evaluate(format!(
        "(() => {{ const el = document.querySelector('{}'); \
         if (!el) throw new Error('not found'); \
         if (!el.checked) {{ el.checked = true; \
         el.dispatchEvent(new Event('change', {{bubbles: true}})); }} \
         }})()",
        selector.replace('\'', "\\'")
    ))
    .await
    .map_err(|e| Error::Cdp(format!("check failed: {e}")))?;
    Ok(())
}

/// Uncheck a checkbox element (sets `checked = false`).
pub async fn uncheck(page: &Page, selector: &str) -> Result<()> {
    page.evaluate(format!(
        "(() => {{ const el = document.querySelector('{}'); \
         if (!el) throw new Error('not found'); \
         if (el.checked) {{ el.checked = false; \
         el.dispatchEvent(new Event('change', {{bubbles: true}})); }} \
         }})()",
        selector.replace('\'', "\\'")
    ))
    .await
    .map_err(|e| Error::Cdp(format!("uncheck failed: {e}")))?;
    Ok(())
}

/// Get the value of an attribute on an element.
pub async fn get_attribute(page: &Page, selector: &str, attribute: &str) -> Result<Option<String>> {
    let val = page
        .evaluate(format!(
            "document.querySelector('{}')?.getAttribute('{}')",
            selector.replace('\'', "\\'"),
            attribute.replace('\'', "\\'")
        ))
        .await
        .map_err(|e| Error::Cdp(format!("get_attribute failed: {e}")))?
        .into_value::<serde_json::Value>()
        .map_err(|e| Error::Cdp(format!("parse attribute failed: {e}")))?;
    match val {
        serde_json::Value::String(s) => Ok(Some(s)),
        serde_json::Value::Null => Ok(None),
        other => Ok(Some(other.to_string())),
    }
}

/// Select an option in a `<select>` element by its value attribute.
pub async fn select_option(page: &Page, selector: &str, value: &str) -> Result<()> {
    page.evaluate(format!(
        "(() => {{ const el = document.querySelector('{}'); \
         if (!el) throw new Error('not found'); \
         el.value = '{}'; \
         el.dispatchEvent(new Event('change', {{bubbles: true}})); \
         }})()",
        selector.replace('\'', "\\'"),
        value.replace('\'', "\\'")
    ))
    .await
    .map_err(|e| Error::Cdp(format!("select_option failed: {e}")))?;
    Ok(())
}
