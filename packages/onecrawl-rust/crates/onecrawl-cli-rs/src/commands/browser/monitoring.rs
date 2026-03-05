use colored::Colorize;
use super::helpers::{with_page};

// ---------------------------------------------------------------------------
// Page Watcher
// ---------------------------------------------------------------------------

pub async fn coverage_js_start() {
    with_page(|page| async move {
        onecrawl_cdp::coverage::start_js_coverage(&page)
            .await
            .map_err(|e| e.to_string())?;
        println!("{} JS coverage started", "✓".green());
        Ok(())
    })
    .await;
}

pub async fn coverage_js_stop() {
    with_page(|page| async move {
        let report = onecrawl_cdp::coverage::stop_js_coverage(&page)
            .await
            .map_err(|e| e.to_string())?;
        println!(
            "{}",
            serde_json::to_string_pretty(&report).unwrap_or_default()
        );
        Ok(())
    })
    .await;
}

pub async fn coverage_css_start() {
    with_page(|page| async move {
        onecrawl_cdp::coverage::start_css_coverage(&page)
            .await
            .map_err(|e| e.to_string())?;
        println!("{} CSS coverage started", "✓".green());
        Ok(())
    })
    .await;
}

pub async fn coverage_css_report() {
    with_page(|page| async move {
        let report = onecrawl_cdp::coverage::get_css_coverage(&page)
            .await
            .map_err(|e| e.to_string())?;
        println!(
            "{}",
            serde_json::to_string_pretty(&report).unwrap_or_default()
        );
        Ok(())
    })
    .await;
}

pub async fn a11y_tree() {
    with_page(|page| async move {
        let result = onecrawl_cdp::accessibility::get_accessibility_tree(&page)
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

pub async fn a11y_element(selector: &str) {
    let sel = onecrawl_cdp::accessibility::resolve_ref(selector);
    with_page(|page| async move {
        let result = onecrawl_cdp::accessibility::get_element_accessibility(&page, &sel)
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

pub async fn a11y_audit() {
    with_page(|page| async move {
        let result = onecrawl_cdp::accessibility::audit_accessibility(&page)
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

pub async fn perf_trace_start() {
    with_page(|page| async move {
        onecrawl_cdp::tracing_cdp::start_tracing(&page)
            .await
            .map_err(|e| e.to_string())?;
        println!("{} Tracing started", "✓".green());
        Ok(())
    })
    .await;
}

pub async fn perf_trace_stop() {
    with_page(|page| async move {
        let result = onecrawl_cdp::tracing_cdp::stop_tracing(&page)
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

pub async fn perf_metrics() {
    with_page(|page| async move {
        let result = onecrawl_cdp::tracing_cdp::get_performance_metrics(&page)
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

pub async fn perf_timing() {
    with_page(|page| async move {
        let result = onecrawl_cdp::tracing_cdp::get_navigation_timing(&page)
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

pub async fn perf_resources() {
    with_page(|page| async move {
        let result = onecrawl_cdp::tracing_cdp::get_resource_timing(&page)
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

pub async fn console_start() {
    with_page(|page| async move {
        onecrawl_cdp::console::start_console_capture(&page)
            .await
            .map_err(|e| e.to_string())?;
        println!("{} Console capture started", "✓".green());
        Ok(())
    })
    .await;
}

pub async fn console_drain() {
    with_page(|page| async move {
        let entries = onecrawl_cdp::console::drain_console_entries(&page)
            .await
            .map_err(|e| e.to_string())?;
        println!(
            "{}",
            serde_json::to_string_pretty(&entries).unwrap_or_default()
        );
        Ok(())
    })
    .await;
}

pub async fn console_clear() {
    with_page(|page| async move {
        onecrawl_cdp::console::clear_console(&page)
            .await
            .map_err(|e| e.to_string())?;
        println!("{} Console buffer cleared", "✓".green());
        Ok(())
    })
    .await;
}

pub async fn dialog_set_handler(accept: bool, prompt_text: Option<&str>) {
    let pt = prompt_text.map(String::from);
    with_page(|page| async move {
        onecrawl_cdp::dialog::set_dialog_handler(&page, accept, pt.as_deref())
            .await
            .map_err(|e| e.to_string())?;
        println!("{} Dialog handler set (accept={})", "✓".green(), accept);
        Ok(())
    })
    .await;
}

pub async fn dialog_history() {
    with_page(|page| async move {
        let events = onecrawl_cdp::dialog::get_dialog_history(&page)
            .await
            .map_err(|e| e.to_string())?;
        println!(
            "{}",
            serde_json::to_string_pretty(&events).unwrap_or_default()
        );
        Ok(())
    })
    .await;
}

pub async fn dialog_clear() {
    with_page(|page| async move {
        onecrawl_cdp::dialog::clear_dialog_history(&page)
            .await
            .map_err(|e| e.to_string())?;
        println!("{} Dialog history cleared", "✓".green());
        Ok(())
    })
    .await;
}

pub async fn worker_list() {
    with_page(|page| async move {
        let workers = onecrawl_cdp::workers::get_service_workers(&page)
            .await
            .map_err(|e| e.to_string())?;
        println!(
            "{}",
            serde_json::to_string_pretty(&workers).unwrap_or_default()
        );
        Ok(())
    })
    .await;
}

pub async fn worker_unregister() {
    with_page(|page| async move {
        let count = onecrawl_cdp::workers::unregister_service_workers(&page)
            .await
            .map_err(|e| e.to_string())?;
        println!("{} Unregistered {} service worker(s)", "✓".green(), count);
        Ok(())
    })
    .await;
}

pub async fn worker_info() {
    with_page(|page| async move {
        let info = onecrawl_cdp::workers::get_worker_info(&page)
            .await
            .map_err(|e| e.to_string())?;
        println!(
            "{}",
            serde_json::to_string_pretty(&info).unwrap_or_default()
        );
        Ok(())
    })
    .await;
}

pub async fn page_watcher_start() {
    with_page(|page| async move {
        onecrawl_cdp::page_watcher::start_page_watcher(&page)
            .await
            .map_err(|e| e.to_string())?;
        println!("{} Page watcher started", "✓".green());
        Ok(())
    })
    .await;
}

pub async fn page_watcher_drain() {
    with_page(|page| async move {
        let changes = onecrawl_cdp::page_watcher::drain_page_changes(&page)
            .await
            .map_err(|e| e.to_string())?;
        println!(
            "{}",
            serde_json::to_string_pretty(&changes).unwrap_or_default()
        );
        Ok(())
    })
    .await;
}

pub async fn page_watcher_stop() {
    with_page(|page| async move {
        onecrawl_cdp::page_watcher::stop_page_watcher(&page)
            .await
            .map_err(|e| e.to_string())?;
        println!("{} Page watcher stopped", "✓".green());
        Ok(())
    })
    .await;
}

pub async fn page_watcher_state() {
    with_page(|page| async move {
        let state = onecrawl_cdp::page_watcher::get_page_state(&page)
            .await
            .map_err(|e| e.to_string())?;
        println!(
            "{}",
            serde_json::to_string_pretty(&state).unwrap_or_default()
        );
        Ok(())
    })
    .await;
}
