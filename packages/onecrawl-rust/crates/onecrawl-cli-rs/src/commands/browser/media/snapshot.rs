use colored::Colorize;
use super::super::helpers::{with_page};

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

