use colored::Colorize;
use super::super::helpers::{with_page};

// ---------------------------------------------------------------------------
// Keyboard
// ---------------------------------------------------------------------------

pub async fn scroll(direction: &str, pixels: i64, selector: Option<&str>) {
    let direction = direction.to_string();
    let sel = selector.map(|s| s.to_string());
    with_page(|page| async move {
        let (dx, dy) = match direction.as_str() {
            "up" => (0, -pixels),
            "down" => (0, pixels),
            "left" => (-pixels, 0),
            "right" => (pixels, 0),
            _ => { eprintln!("❌ Unknown direction: {direction}. Use: up, down, left, right"); return Ok(()); }
        };
        if sel.is_some() {
            // Element-scoped scroll: direct JS (human scroll is viewport-only)
            let s = sel.as_deref().unwrap();
            let js = format!(
                "{{ const el = document.querySelector({}); if(el) el.scrollBy({},{}) ; else throw new Error('not found'); }}",
                serde_json::to_string(s).unwrap_or_default(), dx, dy
            );
            page.evaluate(js).await.map_err(|e| e.to_string())?;
        } else {
            // Viewport scroll: use human-like momentum scroll
            onecrawl_cdp::human::human_scroll(&page, dx, dy)
                .await
                .map_err(|e| e.to_string())?;
        }
        println!("{} Scrolled {} {}px", "✓".green(), direction, pixels);
        Ok(())
    })
    .await;
}

// State Checks (is visible/enabled/checked)
