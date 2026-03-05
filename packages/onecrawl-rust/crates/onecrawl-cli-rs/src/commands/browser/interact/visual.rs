use colored::Colorize;
use super::super::helpers::{with_page};

// ---------------------------------------------------------------------------
// Keyboard
// ---------------------------------------------------------------------------

pub async fn highlight(selector: &str) {
    let sel = selector.to_string();
    with_page(|page| async move {
        let js = format!(
            r#"(() => {{
                const el = document.querySelector({sel});
                if (!el) throw new Error('Element not found');
                el.style.outline = '3px solid red';
                el.style.outlineOffset = '2px';
                setTimeout(() => {{ el.style.outline = ''; el.style.outlineOffset = ''; }}, 3000);
                return true;
            }})()"#,
            sel = serde_json::to_string(&sel).unwrap_or_default()
        );
        page.evaluate(js).await.map_err(|e| e.to_string())?;
        println!("{} Highlighted {} (3s)", "✓".green(), sel.dimmed());
        Ok(())
    })
    .await;
}

// ---------------------------------------------------------------------------
// Page Errors
// ---------------------------------------------------------------------------

pub async fn page_errors(clear: bool) {
    with_page(|page| async move {
        if clear {
            page.evaluate("window.__onecrawl_errors = []").await.map_err(|e| e.to_string())?;
            println!("{} Errors cleared", "✓".green());
        } else {
            let js = r#"(() => {
                if (!window.__onecrawl_errors) {
                    window.__onecrawl_errors = [];
                    window.addEventListener('error', e => window.__onecrawl_errors.push({
                        message: e.message, filename: e.filename, lineno: e.lineno, colno: e.colno, ts: Date.now()
                    }));
                    window.addEventListener('unhandledrejection', e => window.__onecrawl_errors.push({
                        message: String(e.reason), filename: '', lineno: 0, colno: 0, ts: Date.now()
                    }));
                }
                return JSON.stringify(window.__onecrawl_errors);
            })()"#;
            let v = page.evaluate(js).await.map_err(|e| e.to_string())?;
            let text = v.into_value::<String>().unwrap_or_else(|_| "[]".to_string());
            if text == "[]" {
                println!("No errors captured");
            } else {
                println!("{text}");
            }
        }
        Ok(())
    })
    .await;
}

