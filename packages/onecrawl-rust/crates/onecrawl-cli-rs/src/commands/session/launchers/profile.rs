

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

pub(crate) fn sync_chrome_profile_essential(src: &str, dst: &str) -> Result<(), String> {
    use std::path::Path;

    std::fs::create_dir_all(dst)
        .map_err(|e| format!("create synced profile dir '{dst}': {e}"))?;

    // Root-level files (profile directory itself, not Default/).
    for file in &["Local State", "First Run"] {
        let s = Path::new(src).join(file);
        if s.exists() {
            std::fs::copy(&s, Path::new(dst).join(file))
                .map_err(|e| format!("copy {file}: {e}"))?;
        }
    }

    // Essential files inside Default/.
    let default_src = Path::new(src).join("Default");
    let default_dst = Path::new(dst).join("Default");
    std::fs::create_dir_all(&default_dst)
        .map_err(|e| format!("create Default/ dir: {e}"))?;

    for file in &[
        "Cookies",
        "Login Data",
        "Login Data For Account",
        "Web Data",
        "Preferences",
        "Secure Preferences",
        "Local State",
        "Visited Links",
    ] {
        let s = default_src.join(file);
        if s.exists() {
            std::fs::copy(&s, default_dst.join(file))
                .map_err(|e| format!("copy Default/{file}: {e}"))?;
        }
    }

    // Network/ sub-directory (newer Chrome may store Cookies here).
    let net_src = default_src.join("Network");
    if net_src.exists() {
        let net_dst = default_dst.join("Network");
        std::fs::create_dir_all(&net_dst)
            .map_err(|e| format!("create Network/ dir: {e}"))?;
        for entry in std::fs::read_dir(&net_src)
            .map_err(|e| format!("read Network/ dir: {e}"))?
        {
            let entry = entry.map_err(|e| format!("read entry: {e}"))?;
            if entry.metadata().map(|m| m.is_file()).unwrap_or(false) {
                std::fs::copy(entry.path(), net_dst.join(entry.file_name()))
                    .map_err(|e| format!("copy Network/{:?}: {e}", entry.file_name()))?;
            }
        }
    }

    Ok(())
}

