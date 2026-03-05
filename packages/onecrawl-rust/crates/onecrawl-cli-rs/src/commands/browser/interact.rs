use colored::Colorize;
use super::helpers::{with_page};

// ---------------------------------------------------------------------------
// Element Interaction
// ---------------------------------------------------------------------------

// ---------------------------------------------------------------------------
// Keyboard
// ---------------------------------------------------------------------------

pub async fn click(selector: &str) {
    let sel = onecrawl_cdp::accessibility::resolve_ref(selector);
    with_page(|page| async move {
        onecrawl_cdp::human::human_click(&page, &sel)
            .await
            .map_err(|e| e.to_string())?;
        println!("{} Clicked {}", "✓".green(), sel.dimmed());
        Ok(())
    })
    .await;
}

pub async fn dblclick(selector: &str) {
    let sel = onecrawl_cdp::accessibility::resolve_ref(selector);
    with_page(|page| async move {
        onecrawl_cdp::element::double_click(&page, &sel)
            .await
            .map_err(|e| e.to_string())?;
        println!("{} Double-clicked {}", "✓".green(), sel.dimmed());
        Ok(())
    })
    .await;
}

pub async fn type_text(selector: &str, text: &str) {
    let sel = onecrawl_cdp::accessibility::resolve_ref(selector);
    let txt = text.to_string();
    with_page(|page| async move {
        onecrawl_cdp::element::type_text(&page, &sel, &txt)
            .await
            .map_err(|e| e.to_string())?;
        println!("{} Typed into {}", "✓".green(), sel.dimmed());
        Ok(())
    })
    .await;
}

pub async fn fill(selector: &str, text: &str) {
    let sel = onecrawl_cdp::accessibility::resolve_ref(selector);
    let txt = text.to_string();
    with_page(|page| async move {
        onecrawl_cdp::keyboard::fill(&page, &sel, &txt)
            .await
            .map_err(|e| e.to_string())?;
        println!("{} Filled {}", "✓".green(), sel.dimmed());
        Ok(())
    })
    .await;
}

pub async fn focus(selector: &str) {
    let sel = onecrawl_cdp::accessibility::resolve_ref(selector);
    with_page(|page| async move {
        onecrawl_cdp::element::focus(&page, &sel)
            .await
            .map_err(|e| e.to_string())?;
        println!("{} Focused {}", "✓".green(), sel.dimmed());
        Ok(())
    })
    .await;
}

pub async fn hover(selector: &str) {
    let sel = onecrawl_cdp::accessibility::resolve_ref(selector);
    with_page(|page| async move {
        onecrawl_cdp::element::hover(&page, &sel)
            .await
            .map_err(|e| e.to_string())?;
        println!("{} Hovered {}", "✓".green(), sel.dimmed());
        Ok(())
    })
    .await;
}

pub async fn scroll_into_view(selector: &str) {
    let sel = onecrawl_cdp::accessibility::resolve_ref(selector);
    with_page(|page| async move {
        onecrawl_cdp::element::scroll_into_view(&page, &sel)
            .await
            .map_err(|e| e.to_string())?;
        println!("{} Scrolled to {}", "✓".green(), sel.dimmed());
        Ok(())
    })
    .await;
}

pub async fn check(selector: &str) {
    let sel = onecrawl_cdp::accessibility::resolve_ref(selector);
    with_page(|page| async move {
        onecrawl_cdp::element::check(&page, &sel)
            .await
            .map_err(|e| e.to_string())?;
        println!("{} Checked {}", "✓".green(), sel.dimmed());
        Ok(())
    })
    .await;
}

pub async fn uncheck(selector: &str) {
    let sel = onecrawl_cdp::accessibility::resolve_ref(selector);
    with_page(|page| async move {
        onecrawl_cdp::element::uncheck(&page, &sel)
            .await
            .map_err(|e| e.to_string())?;
        println!("{} Unchecked {}", "✓".green(), sel.dimmed());
        Ok(())
    })
    .await;
}

pub async fn select_option(selector: &str, value: &str) {
    let sel = onecrawl_cdp::accessibility::resolve_ref(selector);
    let val = value.to_string();
    with_page(|page| async move {
        onecrawl_cdp::element::select_option(&page, &sel, &val)
            .await
            .map_err(|e| e.to_string())?;
        println!("{} Selected '{}' in {}", "✓".green(), val, sel.dimmed());
        Ok(())
    })
    .await;
}

pub async fn tap(selector: &str) {
    let sel = onecrawl_cdp::accessibility::resolve_ref(selector);
    with_page(|page| async move {
        onecrawl_cdp::input::tap(&page, &sel)
            .await
            .map_err(|e| e.to_string())?;
        println!("{} Tapped {}", "✓".green(), sel.dimmed());
        Ok(())
    })
    .await;
}

pub async fn drag(from: &str, to: &str) {
    let f = from.to_string();
    let t = to.to_string();
    with_page(|page| async move {
        onecrawl_cdp::input::drag_and_drop(&page, &f, &t)
            .await
            .map_err(|e| e.to_string())?;
        println!("{} Dragged {} → {}", "✓".green(), f.dimmed(), t.dimmed());
        Ok(())
    })
    .await;
}

pub async fn upload(selector: &str, file_path: &str) {
    let sel = onecrawl_cdp::accessibility::resolve_ref(selector);
    let fp = file_path.to_string();
    with_page(|page| async move {
        onecrawl_cdp::input::set_file_input(&page, &sel, std::slice::from_ref(&fp))
            .await
            .map_err(|e| e.to_string())?;
        println!(
            "{} Uploaded {} to {}",
            "✓".green(),
            fp.dimmed(),
            sel.dimmed()
        );
        Ok(())
    })
    .await;
}

pub async fn bounding_box(selector: &str) {
    let sel = onecrawl_cdp::accessibility::resolve_ref(selector);
    with_page(|page| async move {
        let (x, y, w, h) = onecrawl_cdp::input::bounding_box(&page, &sel)
            .await
            .map_err(|e| e.to_string())?;
        println!(
            "{}",
            serde_json::json!({"x": x, "y": y, "width": w, "height": h})
        );
        Ok(())
    })
    .await;
}

pub async fn press_key(key: &str) {
    let k = key.to_string();
    with_page(|page| async move {
        onecrawl_cdp::keyboard::press_key(&page, &k)
            .await
            .map_err(|e| e.to_string())?;
        println!("{} Pressed {}", "✓".green(), k.dimmed());
        Ok(())
    })
    .await;
}

pub async fn key_down(key: &str) {
    let k = key.to_string();
    with_page(|page| async move {
        onecrawl_cdp::keyboard::key_down(&page, &k)
            .await
            .map_err(|e| e.to_string())?;
        println!("{} Key down: {}", "✓".green(), k.dimmed());
        Ok(())
    })
    .await;
}

pub async fn key_up(key: &str) {
    let k = key.to_string();
    with_page(|page| async move {
        onecrawl_cdp::keyboard::key_up(&page, &k)
            .await
            .map_err(|e| e.to_string())?;
        println!("{} Key up: {}", "✓".green(), k.dimmed());
        Ok(())
    })
    .await;
}

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
