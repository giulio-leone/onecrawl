use colored::Colorize;
use super::super::helpers::{with_page};

// Print (Enhanced)
// ---------------------------------------------------------------------------
// Page Snapshot
// ---------------------------------------------------------------------------

pub async fn diff_snapshot(name: Option<&str>) {
    let baseline_name = name.unwrap_or("default").to_string();
    let path = format!("/tmp/onecrawl-diff-snap-{baseline_name}.json");
    with_page(|page| async move {
        let snap = onecrawl_cdp::accessibility::agent_snapshot(&page, &onecrawl_cdp::accessibility::AgentSnapshotOptions { interactive_only: true, ..Default::default() })
            .await
            .map_err(|e| e.to_string())?;
        if std::path::Path::new(&path).exists() {
            let saved = std::fs::read_to_string(&path).map_err(|e| e.to_string())?;
            let old: serde_json::Value = serde_json::from_str(&saved).unwrap_or_default();
            let old_text = old["snapshot"].as_str().unwrap_or("");
            let new_text = &snap.snapshot;

            let diff_result = onecrawl_cdp::snapshot_diff::diff_snapshots(old_text, new_text);

            if !diff_result.changed {
                println!("{} No diff — snapshot unchanged", "✓".green());
            } else {
                println!("{} Snapshot changed ({}+ {}− {}=):",
                    "⚡".yellow(),
                    format!("{}", diff_result.additions).green(),
                    format!("{}", diff_result.removals).red(),
                    diff_result.unchanged,
                );
                for line in diff_result.diff.lines() {
                    if let Some(rest) = line.strip_prefix('+') {
                        println!("{}", format!("+{rest}").green());
                    } else if let Some(rest) = line.strip_prefix('-') {
                        println!("{}", format!("-{rest}").red());
                    } else {
                        println!("{line}");
                    }
                }
            }

            let new = serde_json::json!({ "snapshot": snap.snapshot, "refs": snap.refs });
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
        let snap1 = onecrawl_cdp::accessibility::agent_snapshot(&page, &onecrawl_cdp::accessibility::AgentSnapshotOptions { interactive_only: true, ..Default::default() })
            .await.map_err(|e| e.to_string())?;
        page.evaluate(format!("window.location.href = {}", serde_json::to_string(&u2).unwrap_or_default()))
            .await.map_err(|e| e.to_string())?;
        tokio::time::sleep(std::time::Duration::from_secs(2)).await;
        let snap2 = onecrawl_cdp::accessibility::agent_snapshot(&page, &onecrawl_cdp::accessibility::AgentSnapshotOptions { interactive_only: true, ..Default::default() })
            .await.map_err(|e| e.to_string())?;

        let diff_result = onecrawl_cdp::snapshot_diff::diff_snapshots(&snap1.snapshot, &snap2.snapshot);

        if !diff_result.changed {
            println!("{} Pages are identical", "✓".green());
        } else {
            println!("{} Pages differ ({}+ {}− {}=):",
                "⚡".yellow(),
                format!("{}", diff_result.additions).green(),
                format!("{}", diff_result.removals).red(),
                diff_result.unchanged,
            );
            for line in diff_result.diff.lines() {
                if let Some(rest) = line.strip_prefix('+') {
                    println!("{}", format!("+{rest}").green());
                } else if let Some(rest) = line.strip_prefix('-') {
                    println!("{}", format!("-{rest}").red());
                } else {
                    println!("{line}");
                }
            }
        }
        Ok(())
    })
    .await;
}

