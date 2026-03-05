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
