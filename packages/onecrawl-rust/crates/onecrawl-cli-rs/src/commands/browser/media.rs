use colored::Colorize;
use super::helpers::{with_page};

// ---------------------------------------------------------------------------
// Screenshot / PDF
// ---------------------------------------------------------------------------

// ---------------------------------------------------------------------------
// Print (Enhanced)
// ---------------------------------------------------------------------------

#[allow(clippy::too_many_arguments)]

// ---------------------------------------------------------------------------
// Screenshot Diff
// ---------------------------------------------------------------------------

// ---------------------------------------------------------------------------
// Page Snapshot
// ---------------------------------------------------------------------------

pub async fn screenshot(
    output: &str,
    full: bool,
    element: Option<&str>,
    format: &str,
    quality: Option<u32>,
) {
    let t0 = std::time::Instant::now();
    // Proxy fast-path for simple PNG screenshots (no element selector, no custom format)
    if element.is_none() && format == "png" && quality.is_none() {
        if let Some(proxy) = super::super::proxy::ServerProxy::from_session().await {
            if let Ok(bytes) = proxy.screenshot().await {
                if std::fs::write(output, &bytes).is_ok() {
                    let ms = t0.elapsed().as_millis();
                    println!(
                        "{} Screenshot saved to {} ({} bytes) {} {}",
                        "✓".green(),
                        output.cyan(),
                        bytes.len(),
                        format!("{ms}ms").dimmed(),
                        "(proxy)".dimmed()
                    );
                    return;
                }
            }
        }
    }
    let out = output.to_string();
    let elem = element.map(String::from);
    let fmt = format.to_string();
    with_page(|page| async move {
        let bytes = if let Some(ref sel) = elem {
            onecrawl_cdp::screenshot::screenshot_element(&page, sel)
                .await
                .map_err(|e| e.to_string())?
        } else if fmt != "png" || quality.is_some() {
            let img_format = match fmt.as_str() {
                "jpeg" | "jpg" => onecrawl_cdp::ImageFormat::Jpeg,
                "webp" => onecrawl_cdp::ImageFormat::Webp,
                _ => onecrawl_cdp::ImageFormat::Png,
            };
            let opts = onecrawl_cdp::ScreenshotOptions {
                format: img_format,
                quality,
                full_page: full,
            };
            onecrawl_cdp::screenshot::screenshot_with_options(&page, &opts)
                .await
                .map_err(|e| e.to_string())?
        } else if full {
            onecrawl_cdp::screenshot::screenshot_full(&page)
                .await
                .map_err(|e| e.to_string())?
        } else {
            onecrawl_cdp::screenshot::screenshot_viewport(&page)
                .await
                .map_err(|e| e.to_string())?
        };
        std::fs::write(&out, &bytes).map_err(|e| format!("write failed: {e}"))?;
        let ms = t0.elapsed().as_millis();
        println!(
            "{} Screenshot saved to {} ({} bytes) {}",
            "✓".green(),
            out.cyan(),
            bytes.len(),
            format!("{ms}ms").dimmed()
        );
        Ok(())
    })
    .await;
}

pub async fn pdf(output: &str, landscape: bool, scale: f64) {
    let out = output.to_string();
    with_page(|page| async move {
        let bytes = if landscape || (scale - 1.0).abs() > f64::EPSILON {
            let opts = onecrawl_cdp::PdfOptions {
                landscape,
                scale,
                ..Default::default()
            };
            onecrawl_cdp::screenshot::pdf_with_options(&page, &opts)
                .await
                .map_err(|e| e.to_string())?
        } else {
            onecrawl_cdp::screenshot::pdf(&page)
                .await
                .map_err(|e| e.to_string())?
        };
        std::fs::write(&out, &bytes).map_err(|e| format!("write failed: {e}"))?;
        println!(
            "{} PDF saved to {} ({} bytes)",
            "✓".green(),
            out.cyan(),
            bytes.len()
        );
        Ok(())
    })
    .await;
}

pub async fn print_pdf(
    output: &str,
    landscape: bool,
    background: bool,
    scale: Option<f64>,
    paper_width: Option<f64>,
    paper_height: Option<f64>,
    margins: Option<&str>,
    page_ranges: Option<String>,
    header: Option<String>,
    footer: Option<String>,
) {
    let out = output.to_string();
    let (mt, mb, ml, mr) = if let Some(m) = margins {
        let parts: Vec<f64> = m.split(',').filter_map(|s| s.trim().parse().ok()).collect();
        (
            parts.first().copied(),
            parts.get(1).copied(),
            parts.get(2).copied(),
            parts.get(3).copied(),
        )
    } else {
        (None, None, None, None)
    };
    with_page(|page| async move {
        let opts = onecrawl_cdp::DetailedPdfOptions {
            landscape: if landscape { Some(true) } else { None },
            print_background: if background { Some(true) } else { None },
            scale,
            paper_width,
            paper_height,
            margin_top: mt,
            margin_bottom: mb,
            margin_left: ml,
            margin_right: mr,
            page_ranges,
            header_template: header,
            footer_template: footer,
            display_header_footer: None,
            prefer_css_page_size: None,
        };
        let bytes = onecrawl_cdp::print::print_to_pdf(&page, &opts)
            .await
            .map_err(|e| e.to_string())?;
        std::fs::write(&out, &bytes).map_err(|e| format!("write failed: {e}"))?;
        println!(
            "{} PDF saved to {} ({} bytes)",
            "✓".green(),
            out.cyan(),
            bytes.len()
        );
        Ok(())
    })
    .await;
}

pub async fn print_metrics() {
    with_page(|page| async move {
        let val = onecrawl_cdp::print::get_print_metrics(&page)
            .await
            .map_err(|e| e.to_string())?;
        println!("{}", serde_json::to_string_pretty(&val).unwrap_or_default());
        Ok(())
    })
    .await;
}

pub async fn screenshot_diff_compare(baseline: &str, current: &str) {
    let b = baseline.to_string();
    let c = current.to_string();
    with_page(|_page| async move {
        let result = onecrawl_cdp::screenshot_diff::compare_screenshot_files(
            std::path::Path::new(&b),
            std::path::Path::new(&c),
        )
        .map_err(|e| e.to_string())?;
        println!(
            "{}",
            serde_json::to_string_pretty(&result).unwrap_or_default()
        );
        Ok(())
    })
    .await;
}

pub async fn screenshot_diff_regression(baseline_path: &str) {
    let bp = baseline_path.to_string();
    with_page(|page| async move {
        let result =
            onecrawl_cdp::screenshot_diff::visual_regression(&page, std::path::Path::new(&bp))
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

pub async fn snapshot_take(output: Option<&str>) {
    let out = output.map(|s| s.to_string());
    with_page(|page| async move {
        let snap = onecrawl_cdp::snapshot::take_snapshot(&page)
            .await
            .map_err(|e| e.to_string())?;
        if let Some(path) = &out {
            onecrawl_cdp::snapshot::save_snapshot(&snap, std::path::Path::new(path))
                .map_err(|e| e.to_string())?;
            println!("{} Snapshot saved to {}", "✓".green(), path.cyan());
        } else {
            println!(
                "{}",
                serde_json::to_string_pretty(&snap).unwrap_or_default()
            );
        }
        Ok(())
    })
    .await;
}

pub fn snapshot_compare(path1: &str, path2: &str) {
    let a = onecrawl_cdp::snapshot::load_snapshot(std::path::Path::new(path1));
    let b = onecrawl_cdp::snapshot::load_snapshot(std::path::Path::new(path2));
    match (a, b) {
        (Ok(before), Ok(after)) => {
            let diff = onecrawl_cdp::snapshot::compare_snapshots(&before, &after);
            println!(
                "{}",
                serde_json::to_string_pretty(&diff).unwrap_or_default()
            );
        }
        (Err(e), _) | (_, Err(e)) => {
            eprintln!("{} {e}", "✗".red());
            std::process::exit(1);
        }
    }
}

pub async fn snapshot_watch(interval_ms: u64, selector: Option<&str>, count: usize) {
    let sel = selector.map(|s| s.to_string());
    with_page(|page| async move {
        let diffs =
            onecrawl_cdp::snapshot::watch_for_changes(&page, interval_ms, sel.as_deref(), count)
                .await
                .map_err(|e| e.to_string())?;
        for (i, diff) in diffs.iter().enumerate() {
            println!("--- Diff #{} ---", i + 1);
            println!("{}", serde_json::to_string_pretty(diff).unwrap_or_default());
        }
        println!("{} {} diffs captured", "✓".green(), diffs.len());
        Ok(())
    })
    .await;
}

pub async fn snapshot_agent(json_output: bool, interactive_only: bool) {
    with_page(|page| async move {
        let snap = onecrawl_cdp::accessibility::agent_snapshot(&page, interactive_only)
            .await
            .map_err(|e| e.to_string())?;
        if json_output {
            let out = serde_json::json!({
                "success": true,
                "data": {
                    "snapshot": snap.snapshot,
                    "refs": snap.refs,
                    "total": snap.total
                }
            });
            println!("{}", serde_json::to_string_pretty(&out).unwrap_or_default());
        } else {
            println!("{}", snap.snapshot);
            println!("\n{} {} elements tagged with @refs", "✓".green(), snap.total);
        }
        Ok(())
    })
    .await;
}
