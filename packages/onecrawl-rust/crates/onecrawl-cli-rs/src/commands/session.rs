use clap::Subcommand;
use colored::Colorize;
use onecrawl_cdp::BrowserSession;
use serde::{Deserialize, Serialize};
use std::path::Path;
use std::process::Stdio;

const SESSION_FILE: &str = "/tmp/onecrawl-session.json";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionInfo {
    pub ws_url: String,
    pub pid: Option<u32>,
    #[serde(default)]
    pub server_port: Option<u16>,
    #[serde(default)]
    pub server_pid: Option<u32>,
    #[serde(default)]
    pub default_tab_id: Option<String>,
    #[serde(default)]
    pub instance_id: Option<String>,
    /// Raw TargetId string of the currently active tab, set by `tab switch`.
    #[serde(default)]
    pub active_tab_id: Option<String>,
    /// Whether this session was started with --headless. When true, stealth patches
    /// are re-registered on every `connect_to_session()` call because
    /// `Page.addScriptToEvaluateOnNewDocument` is per-DevTools-session and is
    /// removed when the session that registered it disconnects.
    #[serde(default)]
    pub headless: bool,
}

#[derive(Subcommand)]
pub enum SessionAction {
    /// Start a new browser session
    Start {
        /// Run headless (default: headed)
        #[arg(short = 'H', long)]
        headless: bool,
        /// Connect to existing browser via CDP URL
        #[arg(short, long)]
        connect: Option<String>,
        /// Fork browser to background
        #[arg(short, long)]
        background: bool,
        /// Launch the system Chrome with the real user profile (no automation flags).
        /// Enables stealth login on sites that detect automation browsers.
        /// Cannot be combined with --headless.
        #[arg(long)]
        normal_chrome: bool,
        /// Chrome user-data-dir to use with --normal-chrome.
        /// Defaults to ~/Library/Application Support/Google/Chrome on macOS.
        #[arg(long, value_name = "DIR")]
        chrome_profile: Option<String>,
        /// Import cookies from a JSON file into the new session.
        /// Use 'cookie export' to generate this file from a headed session.
        #[arg(long, value_name = "FILE")]
        import_cookies: Option<String>,
    },
    /// Show session info
    Info,
    /// Close the current session
    Close,
}

/// Load session info from disk.
pub fn load_session() -> Option<SessionInfo> {
    let data = std::fs::read_to_string(SESSION_FILE).ok()?;
    serde_json::from_str(&data).ok()
}

/// Save session info to disk.
pub fn save_session(info: &SessionInfo) -> std::io::Result<()> {
    let data = serde_json::to_string_pretty(info)?;
    std::fs::write(SESSION_FILE, data)
}

/// Remove session file.
fn remove_session() {
    let _ = std::fs::remove_file(SESSION_FILE);
}

/// Connect to the active session and return the active page.
///
/// If `active_tab_id` is set in the session file (written by `tab switch`),
/// the page with that TargetId is returned. Otherwise falls back to the first
/// available page, creating a blank one if the browser has none.
///
/// Retries target discovery up to 5×50ms because the chromiumoxide handler
/// populates its `targets` map asynchronously after a fresh `connect()`.
pub async fn connect_to_session() -> Result<(BrowserSession, onecrawl_cdp::Page), String> {
    let info = load_session().ok_or_else(|| {
        format!(
            "No active session. Run {} first.",
            "onecrawl session start".yellow()
        )
    })?;
    let session = BrowserSession::connect_with_nav_timeout(&info.ws_url)
        .await
        .map_err(|e| format!("Failed to connect to session: {e}"))?;

    // The chromiumoxide handler discovers targets asynchronously after connect.
    // Retry page lookup with short backoff to avoid a race-condition where
    // `pages()` returns an empty list right after connecting.
    const MAX_ATTEMPTS: u8 = 5;
    const WAIT_MS: u64 = 50;

    for attempt in 0..MAX_ATTEMPTS {
        if attempt > 0 {
            tokio::time::sleep(std::time::Duration::from_millis(WAIT_MS)).await;
        }

        let mut pages = session
            .browser()
            .pages()
            .await
            .map_err(|e| format!("Failed to list pages: {e}"))?;
        // Sort for stable index-consistent ordering across reconnections.
        pages.sort_by(|a, b| a.target_id().inner().cmp(b.target_id().inner()));

        if pages.is_empty() {
            continue; // handler not ready yet, retry
        }

        let page = if let Some(ref tid) = info.active_tab_id {
            // Find the page whose raw TargetId matches the saved one.
            match pages.into_iter().find(|p| p.target_id().inner() == tid) {
                Some(p) => p,
                None => continue, // target not yet visible, retry
            }
        } else {
            // No active tab preference — prefer the first web page (http/https/about:blank).
            // Chrome's internal pages (chrome://, devtools://) must be skipped.
            let mut web_page = None;
            for p in pages {
                let url = p.url().await.ok().flatten().unwrap_or_default();
                if url.is_empty() || url.starts_with("http") || url == "about:blank" {
                    web_page = Some(p);
                    break;
                }
            }
            match web_page {
                Some(p) => p,
                None => continue,
            }
        };

        // For headless sessions: re-register stealth on every connection.
        // Page.addScriptToEvaluateOnNewDocument is per-DevTools-session and is
        // removed when the session that registered it disconnects. Re-registering
        // here ensures stealth scripts run before every future navigation.
        if info.headless {
            let _ = onecrawl_cdp::inject_persistent_stealth(&page).await;
        }

        return Ok((session, page));
    }

    // All retries exhausted — either no pages exist or active_tab_id is stale.
    if let Some(ref tid) = info.active_tab_id {
        // active_tab_id is stale (tab was closed). Try one more time for any page.
        let pages = session
            .browser()
            .pages()
            .await
            .map_err(|e| format!("Failed to list pages: {e}"))?;
        // Prefer web pages over Chrome-internal ones.
        let mut web_first = pages;
        web_first.sort_by(|a, b| a.target_id().inner().cmp(b.target_id().inner()));
        if let Some(p) = web_first.into_iter().next() {
            eprintln!(
                "⚠ Active tab '{}' not found (closed?). Falling back to first available tab.",
                tid
            );
            // Clear the stale active_tab_id from disk.
            let mut stale_info = info.clone();
            stale_info.active_tab_id = None;
            let _ = save_session(&stale_info);
            // Re-register stealth for the fallback page if headless.
            if stale_info.headless {
                let _ = onecrawl_cdp::inject_persistent_stealth(&p).await;
            }
            return Ok((session, p));
        }
        Err(format!(
            "Active tab '{}' not found and no other pages available. \
             Try `onecrawl tab new` to open a fresh tab.",
            tid
        ))
    } else {
        // No pages at all — create a fresh blank page.
        let page = session
            .new_page("about:blank")
            .await
            .map_err(|e| format!("Failed to create page: {e}"))?;
        Ok((session, page))
    }
}

pub async fn handle(action: SessionAction) {
    match action {
        SessionAction::Start {
            headless,
            connect,
            background: _,
            normal_chrome,
            chrome_profile,
            import_cookies,
        } => {
            if Path::new(SESSION_FILE).exists()
                && let Some(info) = load_session()
            {
                // Check if session is still alive
                if BrowserSession::connect(&info.ws_url).await.is_ok() {
                    eprintln!(
                        "{} Session already active at {}",
                        "⚠".yellow(),
                        info.ws_url.cyan()
                    );
                    return;
                }
                // Stale session file, clean up
                remove_session();
            }

            if normal_chrome {
                // Normal Chrome mode — attach to existing Chrome via DevToolsActivePort,
                // or launch Chrome fully detached with real profile and no automation flags.
                // The session-start command exits immediately after saving the session file;
                // Chrome lives independently as a detached process.
                match launch_normal_chrome(chrome_profile.as_deref()).await {
                    Ok((ws_url, maybe_pid)) => {
                        let info = SessionInfo {
                            ws_url: ws_url.clone(),
                            pid: maybe_pid,
                            server_port: None,
                            server_pid: None,
                            default_tab_id: None,
                            instance_id: None,
                            active_tab_id: None,
                            headless: false,
                        };
                        if let Err(e) = save_session(&info) {
                            eprintln!("{} Failed to save session: {e}", "✗".red());
                            std::process::exit(1);
                        }
                        match maybe_pid {
                            Some(pid) => println!(
                                "{} Session started (normal Chrome, PID {})",
                                "✓".green(),
                                pid
                            ),
                            None => println!(
                                "{} Session attached to existing Chrome",
                                "✓".green()
                            ),
                        }
                        println!("  WS: {}", ws_url.cyan());
                        println!("  File: {}", SESSION_FILE.dimmed());
                        println!(
                            "  {}",
                            "Chrome is running. Use 'session close' to end the session."
                                .dimmed()
                        );
                    }
                    Err(e) => {
                        eprintln!("{} Failed to start normal Chrome: {e}", "✗".red());
                        std::process::exit(1);
                    }
                }
            } else if headless && connect.is_none() {
                // Stealth headless mode — launch Chrome with --headless=new + dedicated profile.
                // Chrome runs as a detached process (survives this command's exit).
                // Stealth patches (UA spoof, webdriver=false) are applied on the first page.
                match launch_stealth_headless(chrome_profile.as_deref()).await {
                    Ok((ws_url, maybe_pid)) => {
                        let info = SessionInfo {
                            ws_url: ws_url.clone(),
                            pid: maybe_pid,
                            server_port: None,
                            server_pid: None,
                            default_tab_id: None,
                            instance_id: None,
                            active_tab_id: None,
                            headless: true,
                        };
                        if let Err(e) = save_session(&info) {
                            eprintln!("{} Failed to save session: {e}", "✗".red());
                            std::process::exit(1);
                        }
                        println!("{} Stealth headless session started (--headless=new)", "✓".green());
                        println!("  WS: {}", ws_url.cyan());
                        println!("  File: {}", SESSION_FILE.dimmed());
                        // Auto-inject persistent stealth patches (runs before every page's scripts)
                        if let Err(e) = apply_stealth_persistent(&ws_url).await {
                            eprintln!("{} Stealth injection failed: {e}", "⚠".yellow());
                        }
                        // Apply cookie import if requested
                        if let Some(ref cookie_file) = import_cookies {
                            if let Err(e) = apply_cookie_import(&ws_url, cookie_file).await {
                                eprintln!("{} Cookie import failed: {e}", "⚠".yellow());
                            }
                        }
                    }
                    Err(e) => {
                        eprintln!("{} Failed to start stealth headless Chrome: {e}", "✗".red());
                        std::process::exit(1);
                    }
                }
            } else if connect.is_some() || !headless {
                // Direct CDP mode — connect or headed, no proxy
                let result = if let Some(ref url) = connect {
                    println!("{} Connecting to {}", "→".blue(), url.cyan());
                    BrowserSession::connect(url).await
                } else {
                    println!("{} Launching headed browser...", "→".blue());
                    BrowserSession::launch_headed().await
                };
                match result {
                    Ok(session) => {
                        let ws_url = session.ws_url().to_string();
                        let info = SessionInfo {
                            ws_url: ws_url.clone(),
                            pid: None,
                            server_port: None,
                            server_pid: None,
                            default_tab_id: None,
                            instance_id: None,
                            active_tab_id: None,
                            headless: false,
                        };
                        if let Err(e) = save_session(&info) {
                            eprintln!("{} Failed to save session: {e}", "✗".red());
                            std::process::exit(1);
                        }
                        // Apply cookie import if requested
                        if let Some(ref cookie_file) = import_cookies {
                            if let Err(e) = apply_cookie_import(&ws_url, cookie_file).await {
                                eprintln!("{} Cookie import failed: {e}", "⚠".yellow());
                            }
                        }
                        println!("{} Session started (direct CDP)", "✓".green());
                        println!("  WS: {}", ws_url.cyan());
                        println!("  File: {}", SESSION_FILE.dimmed());
                        println!("  {}", "Press Ctrl+C to stop the browser.".dimmed());
                        tokio::signal::ctrl_c().await.ok();
                        println!("\n{} Shutting down...", "→".blue());
                        remove_session();
                        let _ = session.close().await;
                        println!("{} Session closed", "✓".green());
                    }
                    Err(e) => {
                        eprintln!("{} Launch failed: {e}", "✗".red());
                        std::process::exit(1);
                    }
                }
            } else {
                // Proxy mode — start HTTP server which manages its own browser
                println!("{} Launching headless browser via proxy server...", "→".blue());
                match start_proxy_server(headless).await {
                    Ok((port, server_pid, instance_id, tab_id, ws_url)) => {
                        let info = SessionInfo {
                            ws_url: ws_url.clone(),
                            pid: None,
                            server_port: Some(port),
                            server_pid: Some(server_pid),
                            default_tab_id: Some(tab_id),
                            instance_id: Some(instance_id),
                            active_tab_id: None,
                            headless: false,
                        };
                        if let Err(e) = save_session(&info) {
                            eprintln!("{} Failed to save session: {e}", "✗".red());
                            std::process::exit(1);
                        }
                        println!("{} Session started (proxy mode)", "✓".green());
                        println!(
                            "  Proxy: {}",
                            format!("http://127.0.0.1:{port}").cyan()
                        );
                        println!("  Server PID: {}", server_pid.to_string().yellow());
                        println!("  File: {}", SESSION_FILE.dimmed());
                        println!(
                            "  {}",
                            "Server running as daemon. Use 'onecrawl session close' to stop.".dimmed()
                        );
                        // Exit immediately — server runs as daemon
                    }
                    Err(e) => {
                        // Fallback to direct CDP
                        eprintln!(
                            "{} Proxy failed ({e}), falling back to direct CDP",
                            "⚠".yellow()
                        );
                        match BrowserSession::launch_headless().await {
                            Ok(session) => {
                                let ws_url = session.ws_url().to_string();
                                let info = SessionInfo {
                                    ws_url: ws_url.clone(),
                                    pid: None,
                                    server_port: None,
                                    server_pid: None,
                                    default_tab_id: None,
                                    instance_id: None,
                                    active_tab_id: None,
                                    headless: false,
                                };
                                if let Err(e) = save_session(&info) {
                                    eprintln!("{} Failed to save session: {e}", "✗".red());
                                    std::process::exit(1);
                                }
                                println!("{} Session started (direct CDP fallback)", "✓".green());
                                println!("  WS: {}", ws_url.cyan());
                                println!("  File: {}", SESSION_FILE.dimmed());
                                println!("  {}", "Press Ctrl+C to stop the browser.".dimmed());
                                tokio::signal::ctrl_c().await.ok();
                                println!("\n{} Shutting down...", "→".blue());
                                remove_session();
                                let _ = session.close().await;
                                println!("{} Session closed", "✓".green());
                            }
                            Err(e2) => {
                                eprintln!("{} Launch failed: {e2}", "✗".red());
                                std::process::exit(1);
                            }
                        }
                    }
                }
            }
        }
        SessionAction::Info => match load_session() {
            Some(info) => {
                println!("{} Active session", "●".green());
                println!("  WS:   {}", info.ws_url.cyan());
                if let Some(pid) = info.pid {
                    println!("  PID:  {}", pid.to_string().yellow());
                }
                if let Some(port) = info.server_port {
                    println!(
                        "  Proxy: {}",
                        format!("http://127.0.0.1:{port}").cyan()
                    );
                }
                if let Some(ref tab_id) = info.default_tab_id {
                    println!("  Tab:  {}", tab_id.dimmed());
                }
                println!("  File: {}", SESSION_FILE.dimmed());
                // Verify connectivity
                match BrowserSession::connect(&info.ws_url).await {
                    Ok(_) => println!("  Status: {}", "connected".green()),
                    Err(_) => println!("  Status: {}", "unreachable".red()),
                }
            }
            None => {
                println!("{} No active session", "○".dimmed());
                std::process::exit(1);
            }
        },
        SessionAction::Close => match load_session() {
            Some(info) => {
                // Kill server process if running
                if let Some(server_pid) = info.server_pid {
                    let _ = kill_process(server_pid);
                    println!("{} Proxy server stopped (PID {})", "✓".green(), server_pid);
                }
                match BrowserSession::connect(&info.ws_url).await {
                    Ok(session) => {
                        let _ = session.close().await;
                        println!("{} Browser closed", "✓".green());
                    }
                    Err(_) => {
                        println!("{} Browser already gone, cleaning up", "⚠".yellow());
                    }
                }
                remove_session();
                println!("{} Session file removed", "✓".green());
            }
            None => {
                println!("{} No active session to close", "○".dimmed());
            }
        },
    }
}

/// Find a free TCP port by binding to port 0.
fn find_free_port() -> std::io::Result<u16> {
    let listener = std::net::TcpListener::bind("127.0.0.1:0")?;
    Ok(listener.local_addr()?.port())
}

/// Probe a CDP debugging port. Returns `(ws_url, user_agent_string)` if reachable.
async fn cdp_probe(port: u16) -> Option<(String, String)> {
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

/// Kill a process by PID.
fn kill_process(pid: u32) -> std::io::Result<()> {
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

/// Start the proxy server as a child process and create an instance + default tab.
/// Returns (port, server_pid, instance_id, tab_id, ws_url).
async fn start_proxy_server(headless: bool) -> Result<(u16, u32, String, String, String), String> {
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

/// Launch the system Chrome browser with the user's real profile and no automation flags.
///
/// Strategy (in order):
///   1. Read `DevToolsActivePort` from the onecrawl Chrome profile dir → attach if live.
///   2. Scan Chrome process args for `--remote-debugging-port=N` → try each (non-headless).
///   3. Dedicated profile is in use (no debug port) → wait up to 60s for user to close it.
///   4. Profile not in use → launch Chrome via `open -na` (macOS) or direct spawn (Linux)
///      with `--remote-debugging-port` and no automation flags.
///
/// Default profile: `~/.onecrawl/chrome-profile/` (persists between sessions; avoids
/// macOS Chrome singleton conflicts with the user's own Chrome instance).
async fn launch_normal_chrome(
    chrome_profile: Option<&str>,
) -> Result<(String, Option<u32>), String> {
    let home = std::env::var("HOME").map_err(|_| "HOME not set".to_string())?;
    let user_data_dir = if let Some(dir) = chrome_profile {
        dir.to_string()
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

    // --- Step 1: DevToolsActivePort (written by Chrome itself, read-only for us) ---
    // The file contains "<port>\n<ws-path>" (Chrome 144+) or just "<port>" (older).
    if let Ok(content) = std::fs::read_to_string(&active_port_file) {
        // Only trust the file if it looks like a valid 2-line Chrome file (has ws-path).
        let mut lines = content.lines();
        if let (Some(port_str), Some(_ws_path)) = (lines.next(), lines.next()) {
            if let Ok(port) = port_str.trim().parse::<u16>() {
                if let Some((ws_url, _)) = probe(port).await {
                    println!(
                        "{} Attached to Chrome on port {} (DevToolsActivePort)",
                        "✓".green(), port
                    );
                    return Ok((ws_url, None));
                }
                // Port in file is stale / not reachable — fall through to process scan.
            }
        }
    }

    // --- Step 2: Scan process args for --remote-debugging-port=N ---
    // Extract distinct ports from Chrome/renderer process list.
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

    if let Some(ws_url) = best_ws {
        if best_tier >= 2 {
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
    }

    // scan_ports: re-scan for poll loop (Step 3); only non-headless processes.
    let scan_ports = move || -> Vec<u16> {
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

    // --- Step 3: Real Chrome with user's profile is running but no debug port ---
    // Check specifically for Chrome processes using the user's actual profile directory.
    // Playwright/Puppeteer always use temp profiles (/tmp, /var/folders) — skip those.
    let real_chrome_running = std::process::Command::new("sh")
        .args(["-c", &format!(
            r#"ps -eo args | grep -F '{}' | grep -v grep | grep -q '.'"#,
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
            r#"ps -eo args | grep -F '{}' | grep -v grep | grep -q '.'"#,
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

    // --- Step 4: Launch Chrome with dedicated onecrawl profile + remote debugging ---
    // macOS: use `open -na "Google Chrome" --args …` which always creates a new Chrome
    //        process even if Chrome is already running with a different profile.
    //        We intentionally do NOT capture the PID via 'open'; Chrome will persist
    //        after onecrawl exits so the user can keep browsing.
    // Linux: direct binary spawn (no macOS singleton issue).
    let port = find_free_port().map_err(|e| format!("find port: {e}"))?;
    println!("{} Launching Chrome on port {} (dedicated onecrawl profile)...", "→".blue(), port);
    println!("  Profile: {}", user_data_dir.dimmed());

    #[cfg(target_os = "macos")]
    {
        let chrome_app = if std::path::Path::new("/Applications/Google Chrome.app").exists() {
            "Google Chrome"
        } else if std::path::Path::new("/Applications/Chromium.app").exists() {
            "Chromium"
        } else {
            return Err("Chrome not found in /Applications/. Install Google Chrome first.".to_string());
        };

        std::process::Command::new("open")
            .arg("-na")
            .arg(chrome_app)
            .arg("--args")
            .arg(format!("--remote-debugging-port={port}"))
            .arg(format!("--user-data-dir={user_data_dir}"))
            .arg("--no-first-run")
            .arg("--no-default-browser-check")
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .map_err(|e| format!("open Chrome: {e}"))?;

        // open(1) returns immediately; poll until CDP is alive.
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

        let ws_url = ws_debugger_url
            .ok_or_else(|| format!("Chrome did not expose CDP on port {port} within 30s"))?;
        println!("{} Chrome ready on port {}", "✓".green(), port);
        return Ok((ws_url, None)); // Chrome persists — no PID to track
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

/// Launch Chrome in `--headless=new` mode with the dedicated onecrawl profile.
///
/// Chrome runs as a detached process so it survives after `session start` exits.
/// A stealth init script (webdriver=undefined, UA spoof) is injected on every page.
async fn launch_stealth_headless(
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
        if let Ok(port) = port_str.parse::<u16>() {
            if let Some((ws, ua)) = cdp_probe(port).await {
                if ua.contains("HeadlessChrome") {
                    println!("{} Reusing running headless Chrome on port {}", "✓".green(), port);
                    return Ok((ws, None));
                }
            }
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
        return Ok((ws_url, Some(chrome_pid)));
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

/// Connect to a running browser session and import cookies from a JSON file.
/// The file must be in the CookieJar format produced by `cookie export`.
async fn apply_cookie_import(ws_url: &str, cookie_file: &str) -> Result<(), String> {
    println!("{} Importing cookies from {}...", "→".blue(), cookie_file);
    let session = BrowserSession::connect(ws_url)
        .await
        .map_err(|e| format!("connect for cookie import: {e}"))?;

    let page = session
        .new_page("about:blank")
        .await
        .map_err(|e| format!("open blank page for cookie import: {e}"))?;

    let count = onecrawl_cdp::cookie_jar::load_cookies_from_file(&page, std::path::Path::new(cookie_file))
        .await
        .map_err(|e| format!("load cookies: {e}"))?;

    println!("{} Imported {} cookies", "✓".green(), count);
    Ok(())
}

/// Inject the stealth init script persistently via `Page.addScriptToEvaluateOnNewDocument`.
/// This runs before any page's own scripts on every navigation, ensuring:
///   - navigator.webdriver = undefined
///   - navigator.plugins populated
///   - User-Agent, languages, platform match the fingerprint
///   - WebGL vendor/renderer spoofed
///   - chrome.runtime present (so x.com sees a "normal" Chrome extension API)
async fn apply_stealth_persistent(ws_url: &str) -> Result<(), String> {
    let session = BrowserSession::connect(ws_url)
        .await
        .map_err(|e| format!("connect for stealth inject: {e}"))?;

    let page = session
        .new_page("about:blank")
        .await
        .map_err(|e| format!("open blank page for stealth: {e}"))?;

    // Persist this tab's TargetId so connect_to_session() always returns this
    // specific tab — the one where stealth scripts are registered.
    let tab_id = page.target_id().inner().clone();
    if let Some(mut info) = load_session() {
        info.active_tab_id = Some(tab_id);
        let _ = save_session(&info);
    }

    onecrawl_cdp::inject_persistent_stealth(&page)
        .await
        .map_err(|e| format!("stealth inject: {e}"))?;

    println!("{} Persistent stealth patches registered for all pages", "✓".green());
    Ok(())
}
