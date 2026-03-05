use colored::Colorize;
use super::super::helpers::{with_page};

// ---------------------------------------------------------------------------
// Element Interaction
// ---------------------------------------------------------------------------

// ---------------------------------------------------------------------------
// Keyboard
// ---------------------------------------------------------------------------

pub async fn mouse_move(x: f64, y: f64) {
    with_page(|page| async move {
        let js = format!(
            "document.elementFromPoint({x},{y})?.dispatchEvent(new MouseEvent('mousemove', {{ clientX: {x}, clientY: {y}, bubbles: true }}))",
            x = x, y = y
        );
        page.evaluate(js).await.map_err(|e| e.to_string())?;
        println!("{} Mouse moved to ({}, {})", "✓".green(), x, y);
        Ok(())
    })
    .await;
}

pub async fn mouse_down(button: &str) {
    let btn: u8 = match button { "left" => 0, "middle" => 1, "right" => 2, _ => 0 };
    with_page(|page| async move {
        let js = format!(
            "document.activeElement?.dispatchEvent(new MouseEvent('mousedown', {{ button: {btn}, bubbles: true }}))",
            btn = btn
        );
        page.evaluate(js).await.map_err(|e| e.to_string())?;
        println!("{} Mouse {} down", "✓".green(), button);
        Ok(())
    })
    .await;
}

pub async fn mouse_up(button: &str) {
    let btn: u8 = match button { "left" => 0, "middle" => 1, "right" => 2, _ => 0 };
    with_page(|page| async move {
        let js = format!(
            "document.activeElement?.dispatchEvent(new MouseEvent('mouseup', {{ button: {btn}, bubbles: true }}))",
            btn = btn
        );
        page.evaluate(js).await.map_err(|e| e.to_string())?;
        println!("{} Mouse {} up", "✓".green(), button);
        Ok(())
    })
    .await;
}

pub async fn mouse_wheel(dy: f64, dx: f64) {
    with_page(|page| async move {
        let js = format!(
            "document.dispatchEvent(new WheelEvent('wheel', {{ deltaX: {dx}, deltaY: {dy}, bubbles: true }}))",
            dx = dx, dy = dy
        );
        page.evaluate(js).await.map_err(|e| e.to_string())?;
        println!("{} Mouse wheel dy={} dx={}", "✓".green(), dy, dx);
        Ok(())
    })
    .await;
}

// ---------------------------------------------------------------------------
// Highlight
// ---------------------------------------------------------------------------

