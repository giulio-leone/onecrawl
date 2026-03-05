use colored::Colorize;
use super::super::helpers::{with_page};

// ---------------------------------------------------------------------------
// Keyboard
// ---------------------------------------------------------------------------

pub async fn window_new() {
    with_page(|page| async move {
        page.evaluate("window.open('about:blank', '_blank')").await.map_err(|e| e.to_string())?;
        println!("{} New window opened", "✓".green());
        Ok(())
    })
    .await;
}

// Set (offline, headers, credentials)
