use colored::Colorize;
use super::super::helpers::{with_page};

// ---------------------------------------------------------------------------
// Keyboard
// ---------------------------------------------------------------------------

pub async fn keyboard_shortcut(keys: &str) {
    let ks = keys.to_string();
    with_page(|page| async move {
        onecrawl_cdp::keyboard::keyboard_shortcut(&page, &ks)
            .await
            .map_err(|e| e.to_string())?;
        println!("{} Shortcut: {}", "✓".green(), ks.dimmed());
        Ok(())
    })
    .await;
}

// Keyboard (focus-based, no selector)

pub async fn keyboard_type(text: &str) {
    let text = text.to_string();
    with_page(|page| async move {
        let js = format!(
            r#"(async () => {{
                const el = document.activeElement;
                if (!el) throw new Error('No focused element');
                const text = {text};
                for (const ch of text) {{
                    el.dispatchEvent(new KeyboardEvent('keydown', {{ key: ch, bubbles: true }}));
                    el.dispatchEvent(new KeyboardEvent('keypress', {{ key: ch, bubbles: true }}));
                    if (el.tagName === 'INPUT' || el.tagName === 'TEXTAREA' || el.isContentEditable) {{
                        document.execCommand('insertText', false, ch);
                    }}
                    el.dispatchEvent(new KeyboardEvent('keyup', {{ key: ch, bubbles: true }}));
                    await new Promise(r => setTimeout(r, 10 + Math.random() * 30));
                }}
                return text.length;
            }})()"#,
            text = serde_json::to_string(&text).unwrap_or_default()
        );
        let v = page.evaluate(js).await.map_err(|e| e.to_string())?;
        println!("{} Typed {} chars at focus", "✓".green(),
            v.into_value::<serde_json::Value>().unwrap_or_default());
        Ok(())
    })
    .await;
}

pub async fn keyboard_insert_text(text: &str) {
    let text = text.to_string();
    with_page(|page| async move {
        let js = format!(
            "document.execCommand('insertText', false, {})",
            serde_json::to_string(&text).unwrap_or_default()
        );
        page.evaluate(js).await.map_err(|e| e.to_string())?;
        println!("{} Inserted text at focus", "✓".green());
        Ok(())
    })
    .await;
}

// Scroll (directional)
