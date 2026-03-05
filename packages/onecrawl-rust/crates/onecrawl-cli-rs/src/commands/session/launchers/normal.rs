use super::probe::kill_process;
use super::profile::sync_chrome_profile_essential;
use colored::Colorize;
use std::process::Stdio;

use super::super::core::{find_free_port};

/// Probe a CDP debugging port. Returns `(ws_url, user_agent_string)` if reachable.
/// Kill a process by PID.
/// Start the proxy server as a child process and create an instance + default tab.
/// Returns (port, server_pid, instance_id, tab_id, ws_url).
/// Copy only the session-critical files from a real Chrome profile to a
/// non-default destination directory.  Cache, GPU cache, and code cache are
/// intentionally skipped to keep the copy fast (< 1 s for typical profiles).
///
/// Cookies are encrypted with the macOS Keychain "Chrome Safe Storage" key.
/// Since we copy to a path on the same machine under the same OS user, Chrome
/// decrypts them identically — login sessions survive the copy.
/// Launch the system Chrome browser with the user's real profile and no automation flags.
///
/// Strategy (in order):
///   1. Read `DevToolsActivePort` from the onecrawl Chrome profile dir → attach if live.
///   2. Scan Chrome process args for `--remote-debugging-port=N` → try each (non-headless).
///   3. Dedicated profile is in use (no debug port) → wait up to 60s for user to close it.
///   4. Profile not in use → launch Chrome via direct spawn with `--remote-debugging-port`
///      and no automation flags.
///
/// Default profile: `~/.onecrawl/chrome-profile/` (persists between sessions; avoids
/// macOS Chrome singleton conflicts with the user's own Chrome instance).
/// Launch Chrome in `--headless=new` mode with the dedicated onecrawl profile.
///
/// Chrome runs as a detached process so it survives after `session start` exits.
/// A stealth init script (webdriver=undefined, UA spoof) is injected on every page.
pub(crate) async fn launch_normal_chrome(
    chrome_profile: Option<&str>,
) -> Result<(String, Option<u32>), String> {
    let home = std::env::var("HOME").map_err(|_| "HOME not set".to_string())?;

    // The macOS default Chrome profile directory.  Chrome refuses to enable CDP
    // on this exact path (security policy).  When the user passes this path we
    // automatically sync the essential session files to a non-default directory
    // so that CDP works while the real cookies/login state are preserved.
    // macOS Keychain decrypts the copied cookies identically because the same
    // Chrome app and the same OS user are used.
    #[cfg(target_os = "macos")]
    let default_chrome_profile = format!("{home}/Library/Application Support/Google/Chrome");
    #[cfg(not(target_os = "macos"))]
    let default_chrome_profile = format!("{home}/.config/google-chrome");

    let user_data_dir = if let Some(dir) = chrome_profile {
        let canonical = std::path::Path::new(dir)
            .canonicalize()
            .map(|p| p.to_string_lossy().to_string())
            .unwrap_or_else(|_| dir.to_string());
        let canonical_default = std::path::Path::new(&default_chrome_profile)
            .canonicalize()
            .map(|p| p.to_string_lossy().to_string())
            .unwrap_or_else(|_| default_chrome_profile.clone());

        if canonical == canonical_default {
            // Real default profile → sync to dedicated location that CDP allows.
            let synced = format!("{home}/.onecrawl/chrome-profile-synced");
            println!(
                "{} Syncing real Chrome profile to non-default path (CDP policy)...",
                "→".blue()
            );
            sync_chrome_profile_essential(dir, &synced)?;
            println!("  Synced: {}", synced.dimmed());
            synced
        } else {
            dir.to_string()
        }
    } else {
        // Use a dedicated onecrawl profile so we never interfere with (or require
        // closing) the user's main Chrome.  The profile persists between sessions,
        // so cookies/login state are preserved after the first login.
        format!("{home}/.onecrawl/chrome-profile")
    };

    // Ensure the profile directory exists before Chrome tries to open it.
    std::fs::create_dir_all(&user_data_dir)
        .map_err(|e| format!("Cannot create Chrome profile dir {user_data_dir}: {e}"))?;

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(2))
        .build()
        .unwrap();

    // Chrome's canonical file indicating the active debug port.
    // onecrawl NEVER writes to this file — Chrome manages it exclusively.
    let active_port_file = format!("{user_data_dir}/DevToolsActivePort");

    // Probe a CDP port: returns (ws_url, is_headless) or None if not reachable.
    let probe = async |port: u16| -> Option<(String, bool)> {
        let resp = client
            .get(format!("http://127.0.0.1:{port}/json/version"))
            .send()
            .await
            .ok()?;
        if !resp.status().is_success() {
            return None;
        }
        let body: serde_json::Value = resp.json().await.ok()?;
        let ws_url = body
            .get("webSocketDebuggerUrl")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string())?;
        // Headless Chrome reports "HeadlessChrome" in its User-Agent string.
        let headless = body
            .get("User-Agent")
            .and_then(|v| v.as_str())
            .map(|ua| ua.contains("HeadlessChrome"))
            .unwrap_or(false);
        Some((ws_url, headless))
    };

    // Count real HTTP tabs (http:// / https://) on a given port.
    let count_http_tabs = async |port: u16| -> usize {
        (async {
            let resp = client
                .get(format!("http://127.0.0.1:{port}/json/list"))
                .send()
                .await
                .ok()?;
            if !resp.status().is_success() {
                return None;
            }
            let arr: serde_json::Value = resp.json().await.ok()?;
            let tabs = arr.as_array()?;
            Some(
                tabs.iter()
                    .filter(|t| {
                        t.get("url")
                            .and_then(|u| u.as_str())
                            .map(|u| u.starts_with("http://") || u.starts_with("https://"))
                            .unwrap_or(false)
                    })
                    .count(),
            )
        })
        .await
        .unwrap_or(0)
    };

    // The file contains "<port>\n<ws-path>" (Chrome 144+) or just "<port>" (older).
    if let Ok(content) = std::fs::read_to_string(&active_port_file) {
        // Only trust the file if it looks like a valid 2-line Chrome file (has ws-path).
        let mut lines = content.lines();
        if let (Some(port_str), Some(_ws_path)) = (lines.next(), lines.next())
            && let Ok(port) = port_str.trim().parse::<u16>()
                && let Some((ws_url, _)) = probe(port).await {
                    println!(
                        "{} Attached to Chrome on port {} (DevToolsActivePort)",
                        "✓".green(), port
                    );
                    return Ok((ws_url, None));
                }
                // Port in file is stale / not reachable — fall through to process scan.
    }

    let profile_dir_clone = user_data_dir.clone();
    let raw_proc_output = std::process::Command::new("sh")
        .args([
            "-c",
            r#"ps -eo pid,args | grep 'Google Chrome\|Chromium\|chromium' | grep 'remote-debugging-port=[1-9]' | grep -v grep"#,
        ])
        .output()
        .map(|o| o.stdout)
        .unwrap_or_default();

    // Parse: extract (port, uses_real_profile) from process list.
    // Headless detection is done via CDP /json/version (more reliable than ps truncation).
    let mut candidate_ports: Vec<(u16, bool)> = vec![]; // (port, real_profile)
    for line in String::from_utf8_lossy(&raw_proc_output).lines() {
        let port: u16 = line
            .split_whitespace()
            .find_map(|a| {
                a.strip_prefix("--remote-debugging-port=")
                    .and_then(|p| p.parse().ok())
                    .filter(|&p: &u16| p > 0)
            })
            .unwrap_or(0);
        if port == 0 {
            continue;
        }
        let real_profile = line.contains(&profile_dir_clone);
        candidate_ports.push((port, real_profile));
    }
    candidate_ports.dedup_by_key(|p| p.0);

    // Priority: real-profile headed > headed > headless.
    // Headless detection via CDP User-Agent (contains "HeadlessChrome").
    // Within same tier, prefer the port with the most HTTP tabs.
    let mut best_ws: Option<String> = None;
    let mut best_port: u16 = 0;
    let mut best_score: usize = 0;
    let mut best_tier: u8 = 0; // 3=real+headed, 2=headed, 1=headless

    for (port, real_profile) in &candidate_ports {
        let port = *port;
        let (ws_url, cdp_headless) = match probe(port).await {
            Some(pair) => pair,
            None => continue,
        };
        let tier: u8 = if *real_profile && !cdp_headless {
            3
        } else if !cdp_headless {
            2
        } else {
            1
        };
        let tab_score = count_http_tabs(port).await;
        if tier > best_tier || (tier == best_tier && tab_score > best_score) {
            best_ws = Some(ws_url);
            best_port = port;
            best_score = tab_score;
            best_tier = tier;
        }
    }

    if let Some(ws_url) = best_ws
        && best_tier >= 2 {
            // Headed (or real-profile headed) Chrome found — use it.
            let mode = if best_tier == 3 { "real profile" } else { "headed" };
            println!(
                "{} Attached to existing Chrome on port {} ({mode}, {} HTTP tabs)",
                "✓".green(), best_port, best_score
            );
            return Ok((ws_url, None));
        }
        // All found instances are headless automation browsers — skip them.
        // Fall through to Step 3/4 to find or launch a real Chrome.

    // scan_ports: re-scan for poll loop (Step 3); only non-headless processes.
    let _scan_ports = move || -> Vec<u16> {
        let stdout_bytes = std::process::Command::new("sh")
            .args([
                "-c",
                r#"ps -eo pid,args | grep 'Google Chrome\|Chromium\|chromium' | grep 'remote-debugging-port=[1-9]' | grep -v headless | grep -v grep | grep -o -- '--remote-debugging-port=[0-9]*' | sort -u | sed 's/--remote-debugging-port=//'"#,
            ])
            .output()
            .map(|o| o.stdout)
            .unwrap_or_default();
        String::from_utf8_lossy(&stdout_bytes)
            .lines()
            .filter_map(|l| l.trim().parse::<u16>().ok())
            .collect()
    };

    // Playwright/Puppeteer always use temp profiles (/tmp, /var/folders) — skip those.
    // Only match the Chrome main process: must have --user-data-dir=<path>.
    // Crashpad handlers use --database= and --metrics-dir= with the same path but never
    // --user-data-dir=, so they are excluded by the tighter grep pattern.
    let real_chrome_running = std::process::Command::new("sh")
        .args(["-c", &format!(
            r#"ps -eo args | grep -v grep | grep -F 'Google Chrome' | grep -qF -- '--user-data-dir={}'"#,
            user_data_dir
        )])
        .status()
        .map(|s| s.success())
        .unwrap_or(false);

    if real_chrome_running {
        // Remote debugging cannot be added to an already-running Chrome process.
        // The only way is to quit Chrome and relaunch it with --remote-debugging-port.
        println!("{} Chrome is running with your profile but remote debugging is not enabled.", "⚠".yellow());
        println!("  {} Please {} Chrome completely (Cmd+Q on macOS).", "→".blue(), "quit".bold());
        println!("  {} onecrawl will then relaunch Chrome with debug mode enabled.", "→".blue());
        println!("  (Press Ctrl+C to abort)");

        // Poll up to 60s for Chrome to quit (check that the profile is no longer in ps args)
        let quit_check = format!(
            r#"ps -eo args | grep -v grep | grep -F 'Google Chrome' | grep -qF -- '--user-data-dir={}'"#,
            user_data_dir
        );
        for _ in 0..120 {
            tokio::time::sleep(std::time::Duration::from_millis(500)).await;
            let still_running = std::process::Command::new("sh")
                .args(["-c", &quit_check])
                .status()
                .map(|s| s.success())
                .unwrap_or(false);
            if !still_running {
                println!("{} Chrome closed. Relaunching with remote debugging...", "✓".green());
                // Fall through to Step 4 by breaking out of the loop
                break;
            }
        }

        // Re-check; if still running after 60s, bail out
        let still_running = std::process::Command::new("sh")
            .args(["-c", &quit_check])
            .status()
            .map(|s| s.success())
            .unwrap_or(false);
        if still_running {
            return Err(
                "Timed out (60s) waiting for Chrome to quit.\n\
                 Please quit Chrome (Cmd+Q) and run the command again."
                    .to_string(),
            );
        }
        // Small delay to let Chrome fully release its files before we relaunch
        tokio::time::sleep(std::time::Duration::from_millis(800)).await;
    }

    //        We intentionally do NOT capture the PID via 'open'; Chrome will persist
    //        after onecrawl exits so the user can keep browsing.
    // Linux: direct binary spawn (no macOS singleton issue).
    let port = find_free_port().map_err(|e| format!("find port: {e}"))?;
    println!("{} Launching Chrome on port {} (dedicated onecrawl profile)...", "→".blue(), port);
    println!("  Profile: {}", user_data_dir.dimmed());

    #[cfg(target_os = "macos")]
    {
        // Resolve the Chrome binary path directly inside the .app bundle.
        // This avoids `open -na` which goes through macOS Launch Services and can
        // fail to forward --remote-debugging-port to a profile that has pre-existing
        // windows/extensions (real user profile).  Direct spawn gives us the PID,
        // deterministic flag forwarding, and no Launch Services overhead.
        let chrome_bin = if std::path::Path::new(
            "/Applications/Google Chrome.app/Contents/MacOS/Google Chrome",
        )
        .exists()
        {
            "/Applications/Google Chrome.app/Contents/MacOS/Google Chrome"
        } else if std::path::Path::new(
            "/Applications/Chromium.app/Contents/MacOS/Chromium",
        )
        .exists()
        {
            "/Applications/Chromium.app/Contents/MacOS/Chromium"
        } else {
            return Err(
                "Chrome not found in /Applications/. Install Google Chrome first.".to_string(),
            );
        };

        let child = std::process::Command::new(chrome_bin)
            .arg(format!("--remote-debugging-port={port}"))
            .arg(format!("--user-data-dir={user_data_dir}"))
            .arg("--no-first-run")
            .arg("--no-default-browser-check")
            // Suppress the "Restore session?" dialog that blocks CDP initialisation
            // when launching a profile that previously crashed or had open tabs.
            .arg("--restore-last-session")
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .map_err(|e| format!("spawn Chrome: {e}"))?;

        let chrome_pid = child.id();

        // Poll up to 60 s (120 × 500 ms) for CDP to become available.
        let mut ws_debugger_url: Option<String> = None;
        for attempt in 0..120 {
            tokio::time::sleep(std::time::Duration::from_millis(500)).await;
            if let Some((ws, _)) = probe(port).await {
                ws_debugger_url = Some(ws);
                break;
            }
            if attempt % 10 == 9 {
                println!("  Waiting for Chrome to start ({}/60)...", attempt / 10 + 1);
            }
        }

        let ws_url = ws_debugger_url.ok_or_else(|| {
            let _ = kill_process(chrome_pid);
            format!("Chrome did not expose CDP on port {port} within 60s")
        })?;
        println!("{} Chrome ready on port {}", "✓".green(), port);
        Ok((ws_url, Some(chrome_pid)))// track PID so 'session close' can kill it
    }

    #[cfg(target_os = "linux")]
    {
        let output = std::process::Command::new("sh")
            .args(["-c", "which google-chrome google-chrome-stable chromium-browser chromium 2>/dev/null | head -1"])
            .output()
            .map_err(|e| format!("which: {e}"))?;
        let chrome_bin = String::from_utf8_lossy(&output.stdout).trim().to_string();
        if chrome_bin.is_empty() {
            return Err("Chrome/Chromium not found in PATH".to_string());
        }

        use std::os::unix::process::CommandExt;
        let child = unsafe {
            std::process::Command::new(&chrome_bin)
                .arg(format!("--remote-debugging-port={port}"))
                .arg(format!("--user-data-dir={user_data_dir}"))
                .arg("--no-first-run")
                .arg("--no-default-browser-check")
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .pre_exec(|| { libc::setsid(); Ok(()) })
                .spawn()
                .map_err(|e| format!("spawn Chrome: {e}"))?
        };
        let chrome_pid = child.id();

        let mut ws_debugger_url: Option<String> = None;
        for attempt in 0..60 {
            tokio::time::sleep(std::time::Duration::from_millis(500)).await;
            if let Some((ws, _)) = probe(port).await {
                ws_debugger_url = Some(ws);
                break;
            }
            if attempt % 10 == 9 {
                println!("  Waiting for Chrome to start ({}/30)...", attempt / 10 + 1);
            }
        }

        let ws_url = ws_debugger_url.ok_or_else(|| {
            let _ = kill_process(chrome_pid);
            format!("Chrome did not expose CDP on port {port} within 30s")
        })?;

        println!("{} Chrome ready on port {}", "✓".green(), port);
        return Ok((ws_url, Some(chrome_pid)));
    }

    #[cfg(not(any(target_os = "macos", target_os = "linux")))]
    return Err("--normal-chrome is only supported on macOS and Linux".to_string());
}

