use colored::Colorize;
use super::super::helpers::{with_page};

// ---------------------------------------------------------------------------
// Screenshot / PDF
// ---------------------------------------------------------------------------

// ---------------------------------------------------------------------------
// Print (Enhanced)
// ---------------------------------------------------------------------------

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
    annotate: bool,
) {
    let t0 = std::time::Instant::now();
    // Proxy fast-path for simple PNG screenshots (no element selector, no custom format, no annotate)
    if element.is_none() && format == "png" && quality.is_none() && !annotate
        && let Some(proxy) = super::super::super::proxy::ServerProxy::from_session().await
            && let Ok(bytes) = proxy.screenshot().await
                && std::fs::write(output, &bytes).is_ok() {
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
    let out = output.to_string();
    let elem = element.map(String::from);
    let fmt = format.to_string();
    with_page(|page| async move {
        // If annotate, first inject labels on interactive elements
        if annotate {
            let label_js = r#"(() => {
                const sels = 'a,button,input,select,textarea,[role="button"],[role="link"],[tabindex]';
                const els = document.querySelectorAll(sels);
                const refs = []; let n = 1;
                for (const el of els) {
                    if (n > 200) break;
                    const r = el.getBoundingClientRect();
                    if (r.width < 1 || r.height < 1) continue;
                    const lbl = document.createElement('div');
                    lbl.className = '__oc_lbl';
                    lbl.textContent = String(n);
                    Object.assign(lbl.style, {
                        position:'absolute', left:(r.left+scrollX)+'px', top:(r.top+scrollY-14)+'px',
                        background:'#e00', color:'#fff', font:'bold 10px monospace', padding:'1px 3px',
                        zIndex:'999999', borderRadius:'2px', pointerEvents:'none'
                    });
                    document.body.appendChild(lbl);
                    refs.push('[' + n + '] ' + (el.tagName||'').toLowerCase());
                    n++;
                }
                return refs.join('\n');
            })()"#;
            let label_result = page.evaluate(label_js.to_string()).await;
            match label_result {
                Ok(v) => {
                    let labels = v.into_value::<String>().unwrap_or_default();
                    // Take screenshot with labels
                    let bytes = if full {
                        onecrawl_cdp::screenshot::screenshot_full(&page).await.map_err(|e| e.to_string())?
                    } else {
                        onecrawl_cdp::screenshot::screenshot_viewport(&page).await.map_err(|e| e.to_string())?
                    };
                    std::fs::write(&out, &bytes).map_err(|e| format!("write failed: {e}"))?;
                    // Clean up labels
                    let _ = page.evaluate("document.querySelectorAll('.__oc_lbl').forEach(el => el.remove())".to_string()).await;
                    let ms = t0.elapsed().as_millis();
                    println!("{} Annotated screenshot saved to {} ({} bytes) {}",
                        "✓".green(), out.cyan(), bytes.len(), format!("{ms}ms").dimmed());
                    if !labels.is_empty() { println!("{labels}"); }
                }
                Err(e) => return Err(e.to_string()),
            }
        } else {
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
        }
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

