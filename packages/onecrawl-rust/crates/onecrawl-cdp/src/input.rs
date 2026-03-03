//! Advanced input: drag-and-drop, file upload via CDP.

use chromiumoxide::Page;
use onecrawl_core::{Error, Result};

/// Drag an element and drop it onto another element.
/// Both `source` and `target` are CSS selectors.
pub async fn drag_and_drop(page: &Page, source: &str, target: &str) -> Result<()> {
    let js = format!(
        r#"
        (async () => {{
            const src = document.querySelector('{source}');
            const tgt = document.querySelector('{target}');
            if (!src) throw new Error('source not found: {source}');
            if (!tgt) throw new Error('target not found: {target}');

            const srcRect = src.getBoundingClientRect();
            const tgtRect = tgt.getBoundingClientRect();

            const srcX = srcRect.left + srcRect.width / 2;
            const srcY = srcRect.top + srcRect.height / 2;
            const tgtX = tgtRect.left + tgtRect.width / 2;
            const tgtY = tgtRect.top + tgtRect.height / 2;

            const dt = new DataTransfer();

            src.dispatchEvent(new DragEvent('dragstart', {{
                bubbles: true, cancelable: true, clientX: srcX, clientY: srcY, dataTransfer: dt
            }}));
            await new Promise(r => setTimeout(r, 50));

            tgt.dispatchEvent(new DragEvent('dragenter', {{
                bubbles: true, cancelable: true, clientX: tgtX, clientY: tgtY, dataTransfer: dt
            }}));
            tgt.dispatchEvent(new DragEvent('dragover', {{
                bubbles: true, cancelable: true, clientX: tgtX, clientY: tgtY, dataTransfer: dt
            }}));
            await new Promise(r => setTimeout(r, 50));

            tgt.dispatchEvent(new DragEvent('drop', {{
                bubbles: true, cancelable: true, clientX: tgtX, clientY: tgtY, dataTransfer: dt
            }}));
            src.dispatchEvent(new DragEvent('dragend', {{
                bubbles: true, cancelable: true, clientX: tgtX, clientY: tgtY, dataTransfer: dt
            }}));

            return 'ok';
        }})()
        "#,
        source = source.replace('\'', "\\'"),
        target = target.replace('\'', "\\'"),
    );
    page.evaluate(js)
        .await
        .map_err(|e| Error::Browser(format!("drag_and_drop failed: {e}")))?;
    Ok(())
}

/// Set files on a `<input type="file">` element via CDP DOM.setFileInputFiles.
pub async fn set_file_input(page: &Page, selector: &str, file_paths: &[String]) -> Result<()> {
    let el = page
        .find_element(selector)
        .await
        .map_err(|e| Error::Browser(format!("file input not found: {e}")))?;

    let backend_node_id = el.backend_node_id;

    let set_files = chromiumoxide::cdp::browser_protocol::dom::SetFileInputFilesParams::builder()
        .files(file_paths.to_vec())
        .backend_node_id(backend_node_id)
        .build()
        .map_err(|e| Error::Browser(format!("SetFileInputFilesParams build failed: {e}")))?;

    page.execute(set_files)
        .await
        .map_err(|e| Error::Browser(format!("DOM.setFileInputFiles failed: {e}")))?;

    Ok(())
}

/// Get the bounding box of an element. Returns (x, y, width, height).
pub async fn bounding_box(page: &Page, selector: &str) -> Result<(f64, f64, f64, f64)> {
    let val = page
        .evaluate(format!(
            r#"
            (() => {{
                const el = document.querySelector('{selector}');
                if (!el) throw new Error('not found: {selector}');
                const r = el.getBoundingClientRect();
                return JSON.stringify({{ x: r.x, y: r.y, width: r.width, height: r.height }});
            }})()
            "#,
            selector = selector.replace('\'', "\\'"),
        ))
        .await
        .map_err(|e| Error::Browser(format!("bounding_box eval failed: {e}")))?
        .into_value::<serde_json::Value>()
        .map_err(|e| Error::Browser(format!("bounding_box parse failed: {e}")))?;

    let s = match val {
        serde_json::Value::String(s) => s,
        other => other.to_string(),
    };

    let obj: serde_json::Value =
        serde_json::from_str(&s).map_err(|e| Error::Browser(format!("bounding_box json: {e}")))?;

    let x = obj["x"].as_f64().unwrap_or(0.0);
    let y = obj["y"].as_f64().unwrap_or(0.0);
    let w = obj["width"].as_f64().unwrap_or(0.0);
    let h = obj["height"].as_f64().unwrap_or(0.0);

    Ok((x, y, w, h))
}

/// Tap an element (for touch simulation). Dispatches touchstart + touchend.
pub async fn tap(page: &Page, selector: &str) -> Result<()> {
    let js = format!(
        r#"
        (() => {{
            const el = document.querySelector('{selector}');
            if (!el) throw new Error('not found: {selector}');
            const rect = el.getBoundingClientRect();
            const x = rect.left + rect.width / 2;
            const y = rect.top + rect.height / 2;
            const touch = new Touch({{ identifier: 1, target: el, clientX: x, clientY: y }});
            el.dispatchEvent(new TouchEvent('touchstart', {{ touches: [touch], bubbles: true }}));
            el.dispatchEvent(new TouchEvent('touchend', {{ changedTouches: [touch], bubbles: true }}));
        }})()
        "#,
        selector = selector.replace('\'', "\\'"),
    );
    page.evaluate(js)
        .await
        .map_err(|e| Error::Browser(format!("tap failed: {e}")))?;
    Ok(())
}
