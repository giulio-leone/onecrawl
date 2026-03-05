use super::probe::{cdp_probe, kill_process};
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
pub(crate) async fn launch_stealth_headless(
    chrome_profile: Option<&str>,
) -> Result<(String, Option<u32>), String> {
    let user_data_dir = if let Some(dir) = chrome_profile {
        dir.to_string()
    } else {
        let home = std::env::var("HOME").unwrap_or_else(|_| "/tmp".into());
        format!("{home}/.onecrawl/chrome-profile")
    };

    // Ensure the profile directory exists
    std::fs::create_dir_all(&user_data_dir)
        .map_err(|e| format!("create profile dir: {e}"))?;

    // --- Step 1: Reuse running headless Chrome on our profile (DevToolsActivePort) ---
    let port_file = format!("{user_data_dir}/DevToolsActivePort");
    if let Ok(contents) = std::fs::read_to_string(&port_file) {
        let port_str = contents.lines().next().unwrap_or("").trim();
        if let Ok(port) = port_str.parse::<u16>()
            && let Some((ws, ua)) = cdp_probe(port).await
                && ua.contains("HeadlessChrome") {
                    println!("{} Reusing running headless Chrome on port {}", "✓".green(), port);
                    return Ok((ws, None));
                }
    }

    let port = find_free_port().map_err(|e| format!("find port: {e}"))?;
    println!("{} Launching headless Chrome (--headless=new) on port {}...", "→".blue(), port);
    println!("  Profile: {}", user_data_dir.dimmed());

    // Stealth args: UA override removes HeadlessChrome from navigator.userAgent
    let stealth_ua = "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/136.0.0.0 Safari/537.36";

    #[cfg(target_os = "macos")]
    {
        let chrome_bin = "/Applications/Google Chrome.app/Contents/MacOS/Google Chrome";
        if !std::path::Path::new(chrome_bin).exists() {
            let chromium_bin = "/Applications/Chromium.app/Contents/MacOS/Chromium";
            if !std::path::Path::new(chromium_bin).exists() {
                return Err("Chrome not found in /Applications/. Install Google Chrome first.".to_string());
            }
        }
        let chrome_bin = if std::path::Path::new(chrome_bin).exists() {
            chrome_bin
        } else {
            "/Applications/Chromium.app/Contents/MacOS/Chromium"
        };

        // For headless mode, spawn the binary directly (no `open -na` needed — no GUI singleton issue).
        use std::os::unix::process::CommandExt;
        let child = unsafe {
            std::process::Command::new(chrome_bin)
                .arg("--headless=new")
                .arg(format!("--remote-debugging-port={port}"))
                .arg(format!("--user-data-dir={user_data_dir}"))
                .arg(format!("--user-agent={stealth_ua}"))
                .arg("--no-first-run")
                .arg("--no-default-browser-check")
                .arg("--disable-blink-features=AutomationControlled")
                .arg("--window-size=1920,1080")
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .pre_exec(|| { libc::setsid(); Ok(()) })
                .spawn()
                .map_err(|e| format!("spawn headless Chrome: {e}"))?
        };
        let chrome_pid = child.id();

        let mut ws_debugger_url: Option<String> = None;
        for attempt in 0..60 {
            tokio::time::sleep(std::time::Duration::from_millis(500)).await;
            if let Some((ws, _)) = cdp_probe(port).await {
                ws_debugger_url = Some(ws);
                break;
            }
            if attempt % 10 == 9 {
                println!("  Waiting for headless Chrome ({}/30)...", attempt / 10 + 1);
            }
        }

        let ws_url = ws_debugger_url.ok_or_else(|| {
            let _ = kill_process(chrome_pid);
            format!("Headless Chrome did not expose CDP on port {port} within 30s")
        })?;
        println!("{} Headless Chrome ready on port {} (PID {})", "✓".green(), port, chrome_pid);
        Ok((ws_url, Some(chrome_pid)))
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
                .arg("--headless=new")
                .arg(format!("--remote-debugging-port={port}"))
                .arg(format!("--user-data-dir={user_data_dir}"))
                .arg(format!("--user-agent={stealth_ua}"))
                .arg("--no-first-run")
                .arg("--no-default-browser-check")
                .arg("--disable-blink-features=AutomationControlled")
                .arg("--window-size=1920,1080")
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .pre_exec(|| { libc::setsid(); Ok(()) })
                .spawn()
                .map_err(|e| format!("spawn headless Chrome: {e}"))?
        };
        let chrome_pid = child.id();

        let mut ws_debugger_url: Option<String> = None;
        for attempt in 0..60 {
            tokio::time::sleep(std::time::Duration::from_millis(500)).await;
            if let Some((ws, _)) = cdp_probe(port).await {
                ws_debugger_url = Some(ws);
                break;
            }
            if attempt % 10 == 9 {
                println!("  Waiting for headless Chrome ({}/30)...", attempt / 10 + 1);
            }
        }

        let ws_url = ws_debugger_url.ok_or_else(|| {
            let _ = kill_process(chrome_pid);
            format!("Headless Chrome did not expose CDP on port {port} within 30s")
        })?;

        println!("{} Headless Chrome ready on port {}", "✓".green(), port);
        return Ok((ws_url, Some(chrome_pid)));
    }

    #[cfg(not(any(target_os = "macos", target_os = "linux")))]
    return Err("--headless is only supported on macOS and Linux".to_string());
}
