use colored::Colorize;
use super::helpers::{with_page};

// ---------------------------------------------------------------------------
// DOM Navigation
// ---------------------------------------------------------------------------

pub async fn adaptive_fingerprint(selector: &str) {
    let sel = onecrawl_cdp::accessibility::resolve_ref(selector);
    with_page(|page| async move {
        let fp = onecrawl_cdp::adaptive::fingerprint_element(&page, &sel)
            .await
            .map_err(|e| e.to_string())?;
        println!("{}", serde_json::to_string_pretty(&fp).unwrap_or_default());
        Ok(())
    })
    .await;
}

pub async fn adaptive_relocate(fingerprint_json: &str) {
    let fp: onecrawl_cdp::ElementFingerprint = match serde_json::from_str(fingerprint_json) {
        Ok(fp) => fp,
        Err(e) => {
            eprintln!("{} Invalid fingerprint JSON: {}", "✗".red(), e);
            std::process::exit(1);
        }
    };
    with_page(|page| async move {
        let matches = onecrawl_cdp::adaptive::relocate_element(&page, &fp)
            .await
            .map_err(|e| e.to_string())?;
        println!(
            "{}",
            serde_json::to_string_pretty(&matches).unwrap_or_default()
        );
        Ok(())
    })
    .await;
}

pub async fn adaptive_track(selectors: &str, save_path: Option<&str>) {
    let sels: Vec<String> = match serde_json::from_str(selectors) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("{} Invalid selectors JSON: {}", "✗".red(), e);
            std::process::exit(1);
        }
    };
    let sel_refs: Vec<&str> = sels.iter().map(|s| s.as_str()).collect();
    let path_buf = save_path.map(std::path::PathBuf::from);
    with_page(|page| async move {
        let fps = onecrawl_cdp::adaptive::track_elements(&page, &sel_refs, path_buf.as_deref())
            .await
            .map_err(|e| e.to_string())?;
        println!("{}", serde_json::to_string_pretty(&fps).unwrap_or_default());
        Ok(())
    })
    .await;
}

pub async fn adaptive_relocate_all(fingerprints_json: &str) {
    let fps: Vec<onecrawl_cdp::ElementFingerprint> = match serde_json::from_str(fingerprints_json) {
        Ok(fps) => fps,
        Err(e) => {
            eprintln!("{} Invalid fingerprints JSON: {}", "✗".red(), e);
            std::process::exit(1);
        }
    };
    with_page(|page| async move {
        let results = onecrawl_cdp::adaptive::relocate_all(&page, &fps)
            .await
            .map_err(|e| e.to_string())?;
        println!(
            "{}",
            serde_json::to_string_pretty(&results).unwrap_or_default()
        );
        Ok(())
    })
    .await;
}

pub async fn adaptive_save(fingerprints: &str, path: &str) {
    let fps: Vec<onecrawl_cdp::ElementFingerprint> = match serde_json::from_str(fingerprints) {
        Ok(fps) => fps,
        Err(e) => {
            eprintln!("{} Invalid fingerprints JSON: {}", "✗".red(), e);
            std::process::exit(1);
        }
    };
    match onecrawl_cdp::adaptive::save_fingerprints(&fps, std::path::Path::new(path)) {
        Ok(_) => println!(
            "{} Saved {} fingerprints to {}",
            "✓".green(),
            fps.len(),
            path.cyan()
        ),
        Err(e) => {
            eprintln!("{} {}", "✗".red(), e);
            std::process::exit(1);
        }
    }
}

pub async fn adaptive_load(path: &str) {
    match onecrawl_cdp::adaptive::load_fingerprints(std::path::Path::new(path)) {
        Ok(fps) => {
            println!("{}", serde_json::to_string_pretty(&fps).unwrap_or_default());
        }
        Err(e) => {
            eprintln!("{} {}", "✗".red(), e);
            std::process::exit(1);
        }
    }
}

pub async fn dom_observe(selector: Option<&str>) {
    let sel = selector.map(onecrawl_cdp::accessibility::resolve_ref);
    with_page(|page| async move {
        onecrawl_cdp::dom_observer::start_dom_observer(&page, sel.as_deref())
            .await
            .map_err(|e| e.to_string())?;
        println!("{} DOM observer started", "✓".green());
        Ok(())
    })
    .await;
}

pub async fn dom_mutations() {
    with_page(|page| async move {
        let mutations = onecrawl_cdp::dom_observer::drain_dom_mutations(&page)
            .await
            .map_err(|e| e.to_string())?;
        println!(
            "{}",
            serde_json::to_string_pretty(&mutations).unwrap_or_default()
        );
        Ok(())
    })
    .await;
}

pub async fn dom_stop() {
    with_page(|page| async move {
        onecrawl_cdp::dom_observer::stop_dom_observer(&page)
            .await
            .map_err(|e| e.to_string())?;
        println!("{} DOM observer stopped", "✓".green());
        Ok(())
    })
    .await;
}

pub async fn dom_snapshot(selector: Option<&str>) {
    let sel = selector.map(onecrawl_cdp::accessibility::resolve_ref);
    with_page(|page| async move {
        let html = onecrawl_cdp::dom_observer::get_dom_snapshot(&page, sel.as_deref())
            .await
            .map_err(|e| e.to_string())?;
        println!("{html}");
        Ok(())
    })
    .await;
}

pub async fn iframe_list() {
    with_page(|page| async move {
        let iframes = onecrawl_cdp::iframe::list_iframes(&page)
            .await
            .map_err(|e| e.to_string())?;
        println!(
            "{}",
            serde_json::to_string_pretty(&iframes).unwrap_or_default()
        );
        Ok(())
    })
    .await;
}

pub async fn iframe_eval(index: usize, expression: &str) {
    let expr = expression.to_string();
    with_page(|page| async move {
        let val = onecrawl_cdp::iframe::eval_in_iframe(&page, index, &expr)
            .await
            .map_err(|e| e.to_string())?;
        println!("{}", serde_json::to_string_pretty(&val).unwrap_or_default());
        Ok(())
    })
    .await;
}

pub async fn iframe_content(index: usize) {
    with_page(|page| async move {
        let html = onecrawl_cdp::iframe::get_iframe_content(&page, index)
            .await
            .map_err(|e| e.to_string())?;
        println!("{html}");
        Ok(())
    })
    .await;
}

pub async fn select_css(selector: &str) {
    let selector = onecrawl_cdp::accessibility::resolve_ref(selector);
    with_page(|page| async move {
        let result = onecrawl_cdp::selectors::css_select(&page, &selector)
            .await
            .map_err(|e| e.to_string())?;
        println!(
            "{}",
            serde_json::to_string_pretty(&result).unwrap_or_default()
        );
        Ok(())
    })
    .await;
}

pub async fn select_xpath(expression: &str) {
    let expression = expression.to_string();
    with_page(|page| async move {
        let result = onecrawl_cdp::selectors::xpath_select(&page, &expression)
            .await
            .map_err(|e| e.to_string())?;
        println!(
            "{}",
            serde_json::to_string_pretty(&result).unwrap_or_default()
        );
        Ok(())
    })
    .await;
}

pub async fn select_text(text: &str, tag: Option<&str>) {
    let text = text.to_string();
    let tag = tag.map(String::from);
    with_page(|page| async move {
        let result = onecrawl_cdp::selectors::find_by_text(&page, &text, tag.as_deref())
            .await
            .map_err(|e| e.to_string())?;
        println!(
            "{}",
            serde_json::to_string_pretty(&result).unwrap_or_default()
        );
        Ok(())
    })
    .await;
}

pub async fn select_regex(pattern: &str, tag: Option<&str>) {
    let pattern = pattern.to_string();
    let tag = tag.map(String::from);
    with_page(|page| async move {
        let result = onecrawl_cdp::selectors::find_by_regex(&page, &pattern, tag.as_deref())
            .await
            .map_err(|e| e.to_string())?;
        println!(
            "{}",
            serde_json::to_string_pretty(&result).unwrap_or_default()
        );
        Ok(())
    })
    .await;
}

pub async fn select_auto(selector: &str) {
    let selector = onecrawl_cdp::accessibility::resolve_ref(selector);
    with_page(|page| async move {
        let result = onecrawl_cdp::selectors::auto_selector(&page, &selector)
            .await
            .map_err(|e| e.to_string())?;
        println!("{result}");
        Ok(())
    })
    .await;
}

pub async fn nav_parent(selector: &str) {
    let selector = onecrawl_cdp::accessibility::resolve_ref(selector);
    with_page(|page| async move {
        let result = onecrawl_cdp::dom_nav::get_parent(&page, &selector)
            .await
            .map_err(|e| e.to_string())?;
        println!(
            "{}",
            serde_json::to_string_pretty(&result).unwrap_or_default()
        );
        Ok(())
    })
    .await;
}

pub async fn nav_children(selector: &str) {
    let selector = onecrawl_cdp::accessibility::resolve_ref(selector);
    with_page(|page| async move {
        let result = onecrawl_cdp::dom_nav::get_children(&page, &selector)
            .await
            .map_err(|e| e.to_string())?;
        println!(
            "{}",
            serde_json::to_string_pretty(&result).unwrap_or_default()
        );
        Ok(())
    })
    .await;
}

pub async fn nav_next_sibling(selector: &str) {
    let selector = onecrawl_cdp::accessibility::resolve_ref(selector);
    with_page(|page| async move {
        let result = onecrawl_cdp::dom_nav::get_next_sibling(&page, &selector)
            .await
            .map_err(|e| e.to_string())?;
        println!(
            "{}",
            serde_json::to_string_pretty(&result).unwrap_or_default()
        );
        Ok(())
    })
    .await;
}

pub async fn nav_prev_sibling(selector: &str) {
    let selector = onecrawl_cdp::accessibility::resolve_ref(selector);
    with_page(|page| async move {
        let result = onecrawl_cdp::dom_nav::get_prev_sibling(&page, &selector)
            .await
            .map_err(|e| e.to_string())?;
        println!(
            "{}",
            serde_json::to_string_pretty(&result).unwrap_or_default()
        );
        Ok(())
    })
    .await;
}

pub async fn nav_siblings(selector: &str) {
    let selector = onecrawl_cdp::accessibility::resolve_ref(selector);
    with_page(|page| async move {
        let result = onecrawl_cdp::dom_nav::get_siblings(&page, &selector)
            .await
            .map_err(|e| e.to_string())?;
        println!(
            "{}",
            serde_json::to_string_pretty(&result).unwrap_or_default()
        );
        Ok(())
    })
    .await;
}

pub async fn nav_similar(selector: &str) {
    let selector = onecrawl_cdp::accessibility::resolve_ref(selector);
    with_page(|page| async move {
        let result = onecrawl_cdp::dom_nav::find_similar(&page, &selector)
            .await
            .map_err(|e| e.to_string())?;
        println!(
            "{}",
            serde_json::to_string_pretty(&result).unwrap_or_default()
        );
        Ok(())
    })
    .await;
}

pub async fn nav_above(selector: &str, limit: usize) {
    let selector = onecrawl_cdp::accessibility::resolve_ref(selector);
    with_page(|page| async move {
        let result = onecrawl_cdp::dom_nav::above_elements(&page, &selector, limit)
            .await
            .map_err(|e| e.to_string())?;
        println!(
            "{}",
            serde_json::to_string_pretty(&result).unwrap_or_default()
        );
        Ok(())
    })
    .await;
}

pub async fn nav_below(selector: &str, limit: usize) {
    let selector = onecrawl_cdp::accessibility::resolve_ref(selector);
    with_page(|page| async move {
        let result = onecrawl_cdp::dom_nav::below_elements(&page, &selector, limit)
            .await
            .map_err(|e| e.to_string())?;
        println!(
            "{}",
            serde_json::to_string_pretty(&result).unwrap_or_default()
        );
        Ok(())
    })
    .await;
}
