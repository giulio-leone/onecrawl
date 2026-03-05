use super::super::helpers::{with_page};

// ---------------------------------------------------------------------------
// Keyboard
// ---------------------------------------------------------------------------

pub async fn is_check(check: &str, selector: &str) {
    let check = check.to_string();
    let sel = selector.to_string();
    with_page(|page| async move {
        let js = match check.as_str() {
            "visible" => format!(
                r#"(() => {{
                    const el = document.querySelector({sel});
                    if (!el) return false;
                    const r = el.getBoundingClientRect();
                    const s = getComputedStyle(el);
                    return r.width > 0 && r.height > 0 && s.visibility !== 'hidden' && s.display !== 'none' && s.opacity !== '0';
                }})()"#,
                sel = serde_json::to_string(&sel).unwrap_or_default()
            ),
            "enabled" => format!(
                "!document.querySelector({}).disabled",
                serde_json::to_string(&sel).unwrap_or_default()
            ),
            "checked" => format!(
                "document.querySelector({}).checked === true",
                serde_json::to_string(&sel).unwrap_or_default()
            ),
            _ => { eprintln!("❌ Unknown check: {check}. Use: visible, enabled, checked"); return Ok(()); }
        };
        let v = page.evaluate(js).await.map_err(|e| e.to_string())?;
        let result = v.into_value::<bool>().unwrap_or(false);
        println!("{result}");
        Ok(())
    })
    .await;
}

