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
    annotate: bool,
) {
    let t0 = std::time::Instant::now();
    // Proxy fast-path for simple PNG screenshots (no element selector, no custom format, no annotate)
    if element.is_none() && format == "png" && quality.is_none() && !annotate {
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

pub async fn snapshot_agent(
    json_output: bool,
    interactive_only: bool,
    _cursor: bool,
    _compact: bool,
    _depth: Option<usize>,
    scope_selector: Option<&str>,
) {
    let scope = scope_selector.map(|s| s.to_string());
    with_page(|page| async move {
        // If scope selector provided, scope to that element first
        if let Some(ref sel) = scope {
            let js = format!(
                "document.querySelector({}) ? true : false",
                serde_json::to_string(sel).unwrap_or_default()
            );
            let exists = page.evaluate(js).await.map_err(|e| e.to_string())?;
            if !exists.into_value::<bool>().unwrap_or(false) {
                return Err(format!("Scope selector not found: {sel}"));
            }
        }
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

// ── Diff ──────────────────────────────────────────────────────────────

pub async fn diff_snapshot(name: Option<&str>) {
    let baseline_name = name.unwrap_or("default").to_string();
    let path = format!("/tmp/onecrawl-diff-snap-{baseline_name}.json");
    with_page(|page| async move {
        let snap = onecrawl_cdp::accessibility::agent_snapshot(&page, true)
            .await
            .map_err(|e| e.to_string())?;
        if std::path::Path::new(&path).exists() {
            let saved = std::fs::read_to_string(&path).map_err(|e| e.to_string())?;
            let old: serde_json::Value = serde_json::from_str(&saved).unwrap_or_default();
            let new = serde_json::json!({ "snapshot": snap.snapshot, "refs": snap.refs });
            if old == new {
                println!("{} No diff — snapshot unchanged", "✓".green());
            } else {
                println!("{} Snapshot changed:", "⚡".yellow());
                println!("--- baseline ---\n{}", old["snapshot"].as_str().unwrap_or(""));
                println!("--- current ---\n{}", snap.snapshot);
            }
            std::fs::write(&path, serde_json::to_string(&new).unwrap_or_default())
                .map_err(|e| format!("write failed: {e}"))?;
        } else {
            let data = serde_json::json!({ "snapshot": snap.snapshot, "refs": snap.refs });
            std::fs::write(&path, serde_json::to_string(&data).unwrap_or_default())
                .map_err(|e| format!("write failed: {e}"))?;
            println!("{} Baseline snapshot saved as '{baseline_name}'", "✓".green());
        }
        Ok(())
    })
    .await;
}

pub async fn diff_screenshot(name: Option<&str>) {
    let baseline_name = name.unwrap_or("default").to_string();
    let path = format!("/tmp/onecrawl-diff-ss-{baseline_name}.png");
    with_page(|page| async move {
        let bytes = onecrawl_cdp::screenshot::screenshot_viewport(&page)
            .await
            .map_err(|e| e.to_string())?;
        if std::path::Path::new(&path).exists() {
            let old_bytes = std::fs::read(&path).map_err(|e| e.to_string())?;
            if old_bytes == bytes {
                println!("{} No diff — screenshot identical", "✓".green());
            } else {
                let diff_pct = bytes.iter().zip(old_bytes.iter())
                    .filter(|(a, b)| a != b).count() as f64 / bytes.len().max(1) as f64 * 100.0;
                println!("{} Screenshot changed (~{:.1}% byte diff)", "⚡".yellow(), diff_pct);
            }
            std::fs::write(&path, &bytes).map_err(|e| format!("write failed: {e}"))?;
        } else {
            std::fs::write(&path, &bytes).map_err(|e| format!("write failed: {e}"))?;
            println!("{} Baseline screenshot saved as '{baseline_name}'", "✓".green());
        }
        Ok(())
    })
    .await;
}

pub async fn diff_url(url1: &str, url2: &str) {
    let u1 = url1.to_string();
    let u2 = url2.to_string();
    with_page(|page| async move {
        page.evaluate(format!("window.location.href = {}", serde_json::to_string(&u1).unwrap_or_default()))
            .await.map_err(|e| e.to_string())?;
        tokio::time::sleep(std::time::Duration::from_secs(2)).await;
        let snap1 = onecrawl_cdp::accessibility::agent_snapshot(&page, true)
            .await.map_err(|e| e.to_string())?;
        page.evaluate(format!("window.location.href = {}", serde_json::to_string(&u2).unwrap_or_default()))
            .await.map_err(|e| e.to_string())?;
        tokio::time::sleep(std::time::Duration::from_secs(2)).await;
        let snap2 = onecrawl_cdp::accessibility::agent_snapshot(&page, true)
            .await.map_err(|e| e.to_string())?;
        if snap1.snapshot == snap2.snapshot {
            println!("{} Pages are identical", "✓".green());
        } else {
            println!("{} Pages differ:", "⚡".yellow());
            println!("--- {} ---\n{}", u1, snap1.snapshot);
            println!("--- {} ---\n{}", u2, snap2.snapshot);
        }
        Ok(())
    })
    .await;
}

// ── Auth State ───────────────────────────────────────────────────────

fn auth_state_dir() -> std::path::PathBuf {
    let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
    let dir = std::path::PathBuf::from(home).join(".onecrawl").join("auth-states");
    let _ = std::fs::create_dir_all(&dir);
    dir
}

pub async fn auth_state_save(name: &str) {
    let n = name.to_string();
    with_page(|page| async move {
        let js = r#"(() => {
            const cookies = document.cookie;
            const ls = {};
            for (let i = 0; i < localStorage.length; i++) {
                const k = localStorage.key(i);
                ls[k] = localStorage.getItem(k);
            }
            const ss = {};
            for (let i = 0; i < sessionStorage.length; i++) {
                const k = sessionStorage.key(i);
                ss[k] = sessionStorage.getItem(k);
            }
            return JSON.stringify({ url: location.href, cookies, localStorage: ls, sessionStorage: ss });
        })()"#;
        let result = page.evaluate(js).await.map_err(|e| e.to_string())?;
        let data = result.into_value::<String>().unwrap_or_default();
        let path = auth_state_dir().join(format!("{n}.json"));
        std::fs::write(&path, &data).map_err(|e| format!("write failed: {e}"))?;
        println!("{} Auth state saved as '{}' ({})", "✓".green(), n, path.display());
        Ok(())
    })
    .await;
}

pub async fn auth_state_load(name: &str) {
    let n = name.to_string();
    with_page(|page| async move {
        let path = auth_state_dir().join(format!("{n}.json"));
        let data = std::fs::read_to_string(&path).map_err(|e| format!("read failed: {e}"))?;
        let parsed: serde_json::Value = serde_json::from_str(&data).unwrap_or_default();
        if let Some(ls) = parsed.get("localStorage").and_then(|v| v.as_object()) {
            for (k, v) in ls {
                let js = format!("localStorage.setItem({}, {})",
                    serde_json::to_string(k).unwrap_or_default(),
                    serde_json::to_string(&v.as_str().unwrap_or("")).unwrap_or_default());
                let _ = page.evaluate(js).await;
            }
        }
        if let Some(ss) = parsed.get("sessionStorage").and_then(|v| v.as_object()) {
            for (k, v) in ss {
                let js = format!("sessionStorage.setItem({}, {})",
                    serde_json::to_string(k).unwrap_or_default(),
                    serde_json::to_string(&v.as_str().unwrap_or("")).unwrap_or_default());
                let _ = page.evaluate(js).await;
            }
        }
        if let Some(cookies) = parsed.get("cookies").and_then(|v| v.as_str()) {
            for cookie in cookies.split(';') {
                let c = cookie.trim();
                if !c.is_empty() {
                    let _ = page.evaluate(format!("document.cookie = {}", serde_json::to_string(c).unwrap_or_default())).await;
                }
            }
        }
        println!("{} Auth state '{}' loaded", "✓".green(), n);
        Ok(())
    })
    .await;
}

pub async fn auth_state_list() {
    let dir = auth_state_dir();
    let entries: Vec<String> = std::fs::read_dir(&dir)
        .into_iter()
        .flat_map(|rd| rd.into_iter())
        .filter_map(|e| e.ok())
        .filter(|e| e.path().extension().map_or(false, |ext| ext == "json"))
        .map(|e| {
            let name = e.path().file_stem().map(|s| s.to_string_lossy().to_string()).unwrap_or_default();
            let meta = e.metadata().ok();
            let size = meta.as_ref().map(|m| m.len()).unwrap_or(0);
            format!("  {} ({} bytes)", name, size)
        })
        .collect();
    if entries.is_empty() {
        println!("{} No saved auth states", "ℹ".blue());
    } else {
        println!("{} Saved auth states:", "✓".green());
        for e in &entries { println!("{e}"); }
    }
}

pub async fn auth_state_show(name: &str) {
    let path = auth_state_dir().join(format!("{name}.json"));
    match std::fs::read_to_string(&path) {
        Ok(data) => {
            let parsed: serde_json::Value = serde_json::from_str(&data).unwrap_or_default();
            println!("{}", serde_json::to_string_pretty(&parsed).unwrap_or(data));
        }
        Err(e) => eprintln!("{} State '{}' not found: {e}", "✗".red(), name),
    }
}

pub async fn auth_state_rename(from: &str, to: &str) {
    let dir = auth_state_dir();
    let src = dir.join(format!("{from}.json"));
    let dst = dir.join(format!("{to}.json"));
    match std::fs::rename(&src, &dst) {
        Ok(()) => println!("{} Renamed '{}' → '{}'", "✓".green(), from, to),
        Err(e) => eprintln!("{} Rename failed: {e}", "✗".red()),
    }
}

pub async fn auth_state_clear(name: &str) {
    let path = auth_state_dir().join(format!("{name}.json"));
    match std::fs::remove_file(&path) {
        Ok(()) => println!("{} Removed auth state '{}'", "✓".green(), name),
        Err(e) => eprintln!("{} Remove failed: {e}", "✗".red()),
    }
}

pub async fn auth_state_clean() {
    let dir = auth_state_dir();
    let mut count = 0;
    if let Ok(rd) = std::fs::read_dir(&dir) {
        for entry in rd.flatten() {
            if entry.path().extension().map_or(false, |ext| ext == "json") {
                let _ = std::fs::remove_file(entry.path());
                count += 1;
            }
        }
    }
    println!("{} Cleaned {} auth state(s)", "✓".green(), count);
}
