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



