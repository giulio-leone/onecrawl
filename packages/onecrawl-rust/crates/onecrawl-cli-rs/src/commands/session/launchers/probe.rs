

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

pub(crate) async fn cdp_probe(port: u16) -> Option<(String, String)> {
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(2))
        .build()
        .ok()?;
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
    let ua = body
        .get("User-Agent")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();
    Some((ws_url, ua))
}

pub(crate) fn kill_process(pid: u32) -> std::io::Result<()> {
    #[cfg(unix)]
    {
        use std::process::Command;
        Command::new("kill").arg(pid.to_string()).output()?;
    }
    #[cfg(not(unix))]
    {
        let _ = pid;
    }
    Ok(())
}

