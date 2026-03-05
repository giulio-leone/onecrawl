use super::probe::kill_process;

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
pub(crate) async fn start_proxy_server(headless: bool) -> Result<(u16, u32, String, String, String), String> {
    let port = find_free_port().map_err(|e| format!("find port: {e}"))?;

    let exe = std::env::current_exe().map_err(|e| format!("current_exe: {e}"))?;

    #[cfg(unix)]
    let child = {
        use std::os::unix::process::CommandExt;
        unsafe {
            std::process::Command::new(exe)
                .args(["serve", "--port", &port.to_string()])
                .stdout(std::process::Stdio::null())
                .stderr(std::process::Stdio::null())
                .pre_exec(|| {
                    libc::setsid();
                    Ok(())
                })
                .spawn()
                .map_err(|e| format!("spawn server: {e}"))?
        }
    };
    #[cfg(not(unix))]
    let child = std::process::Command::new(exe)
        .args(["serve", "--port", &port.to_string()])
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .spawn()
        .map_err(|e| format!("spawn server: {e}"))?;
    let server_pid = child.id();

    // Wait for the server to be ready (poll /health)
    let client = reqwest::Client::new();
    let base = format!("http://127.0.0.1:{port}");
    let mut ready = false;
    for _ in 0..30 {
        tokio::time::sleep(std::time::Duration::from_millis(200)).await;
        if client.get(format!("{base}/health")).send().await.is_ok() {
            ready = true;
            break;
        }
    }
    if !ready {
        let _ = kill_process(server_pid);
        return Err("server did not start in time".into());
    }

    // Create a browser instance via the server
    let resp = client
        .post(format!("{base}/instances"))
        .json(&serde_json::json!({ "headless": headless }))
        .send()
        .await
        .map_err(|e| format!("create instance: {e}"))?;

    if !resp.status().is_success() {
        let body = resp.text().await.unwrap_or_default();
        let _ = kill_process(server_pid);
        return Err(format!("create instance failed: {body}"));
    }

    let body: serde_json::Value = resp.json().await.map_err(|e| format!("parse instance: {e}"))?;
    let instance_id = body["instance"]["id"]
        .as_str()
        .ok_or_else(|| {
            let _ = kill_process(server_pid);
            format!("missing instance id in response: {body}")
        })?
        .to_string();

    // Get WS URL from instance info (if available)
    let ws_url = body["instance"]["ws_url"]
        .as_str()
        .unwrap_or("unknown")
        .to_string();

    // Open a default tab
    let resp = client
        .post(format!("{base}/instances/{instance_id}/tabs/open"))
        .json(&serde_json::json!({ "url": "about:blank" }))
        .send()
        .await
        .map_err(|e| format!("open tab: {e}"))?;

    if !resp.status().is_success() {
        let body = resp.text().await.unwrap_or_default();
        let _ = kill_process(server_pid);
        return Err(format!("open tab failed: {body}"));
    }

    let tab_body: serde_json::Value =
        resp.json().await.map_err(|e| format!("parse tab: {e}"))?;
    let tab_id = tab_body["tab"]["id"]
        .as_str()
        .ok_or_else(|| {
            let _ = kill_process(server_pid);
            format!("missing tab id in response: {tab_body}")
        })?
        .to_string();

    Ok((port, server_pid, instance_id, tab_id, ws_url))
}

