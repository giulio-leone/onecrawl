use clap::Subcommand;
use colored::Colorize;
use onecrawl_cdp::BrowserSession;
use serde::{Deserialize, Serialize};
use std::path::Path;
use super::launchers::{launch_normal_chrome, launch_stealth_headless, start_proxy_server, kill_process};
use super::injection::{apply_cookie_import, apply_stealth_persistent};

pub const SESSION_FILE: &str = "/tmp/onecrawl-session.json";

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
    /// Path to a passkey JSON file (produced by `auth passkey-register`).
    /// When set, CDP WebAuthn is enabled and the credentials are injected on
    /// every `connect_to_session()` call — same lifecycle as stealth scripts.
    #[serde(default)]
    pub passkey_file: Option<String>,
    /// Relying-party ID to auto-load from the vault on every connection.
    /// When set and `passkey_file` is absent, credentials for this `rp_id`
    /// are loaded from `~/.onecrawl/passkeys/vault.json`.
    #[serde(default)]
    pub passkey_rp_id: Option<String>,
    /// Raw User-Agent string returned by `Browser.getVersion` at session start.
    /// Stored so every `connect_to_session()` call uses the same UA, avoiding
    /// random re-generation that would create version mismatches.
    #[serde(default)]
    pub fingerprint_ua: Option<String>,
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
        /// Import passkey credentials from a JSON file produced by 'auth passkey-register'.
        /// The CDP virtual authenticator is re-enabled on every connect.
        #[arg(long, value_name = "FILE")]
        import_passkey: Option<String>,
        /// Auto-load passkey credentials for this rp_id from the vault
        /// (~/.onecrawl/passkeys/vault.json) on every connect.
        /// Use this instead of --import-passkey when credentials are in the vault.
        #[arg(long, value_name = "RP_ID")]
        passkey_rp_id: Option<String>,
    },
    /// Show session info
    Info,
    /// Close the current session
    Close,
}

/// Load session info from disk.

/// Save session info to disk.

/// Remove session file.

/// Connect to the active session and return the active page.
///
/// If `active_tab_id` is set in the session file (written by `tab switch`),
/// the page with that TargetId is returned. Otherwise falls back to the first
/// Resolve passkey credentials from session info.
/// Priority: explicit file → vault rp_id → empty.

/// available page, creating a blank one if the browser has none.
///
/// Retries target discovery up to 5×50ms because the chromiumoxide handler
/// populates its `targets` map asynchronously after a fresh `connect()`.

/// Find a free TCP port by binding to port 0.

pub fn load_session() -> Option<SessionInfo> {
    let data = std::fs::read_to_string(SESSION_FILE).ok()?;
    serde_json::from_str(&data).ok()
}

pub fn save_session(info: &SessionInfo) -> std::io::Result<()> {
    let data = serde_json::to_string_pretty(info)?;
    std::fs::write(SESSION_FILE, data)
}

pub(super) fn remove_session() {
    let _ = std::fs::remove_file(SESSION_FILE);
}

pub(super) fn resolve_passkey_creds(info: &SessionInfo) -> Vec<onecrawl_cdp::PasskeyCredential> {
    if let Some(ref pfile) = info.passkey_file {
        return onecrawl_cdp::load_passkeys(std::path::Path::new(pfile))
            .unwrap_or_default();
    }
    if let Some(ref rp_id) = info.passkey_rp_id {
        return onecrawl_cdp::load_vault()
            .map(|v| onecrawl_cdp::vault_get(&v, rp_id))
            .unwrap_or_default();
    }
    Vec::new()
}

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
        let _ = onecrawl_cdp::inject_persistent_stealth(&page, info.fingerprint_ua.as_deref()).await;

        // Re-enable CDP WebAuthn virtual authenticator for passkey sessions.
        // Virtual authenticators are also per-DevTools-session and must be
        // re-created on every connect_to_session() call.
        let passkey_creds = resolve_passkey_creds(&info);
        if !passkey_creds.is_empty() {
            if let Ok(()) = onecrawl_cdp::cdp_enable(&page).await {
                if let Ok(auth_id) = onecrawl_cdp::cdp_create_authenticator(&page).await {
                    for cred in &passkey_creds {
                        let _ = onecrawl_cdp::cdp_add_credential(&page, &auth_id, cred).await;
                    }
                }
            }
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
        // Re-register stealth for the fallback page.
        let _ = onecrawl_cdp::inject_persistent_stealth(&p, stale_info.fingerprint_ua.as_deref()).await;
            // Re-inject passkeys for the fallback page if configured.
            let passkey_creds = resolve_passkey_creds(&stale_info);
            if !passkey_creds.is_empty() {
                if let Ok(()) = onecrawl_cdp::cdp_enable(&p).await {
                    if let Ok(auth_id) = onecrawl_cdp::cdp_create_authenticator(&p).await {
                        for cred in &passkey_creds {
                            let _ = onecrawl_cdp::cdp_add_credential(&p, &auth_id, cred).await;
                        }
                    }
                }
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
            import_passkey,
            passkey_rp_id,
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
                            passkey_file: import_passkey.clone(),
                            passkey_rp_id: passkey_rp_id.clone(),
                            fingerprint_ua: None,
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
                        // Auto-inject persistent stealth patches (runs before every page's scripts)
                        if let Err(e) = apply_stealth_persistent(&ws_url).await {
                            eprintln!("{} Stealth injection failed: {e}", "⚠".yellow());
                        }
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
                            passkey_file: import_passkey.clone(),
                            passkey_rp_id: passkey_rp_id.clone(),
                            fingerprint_ua: None,
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
                            passkey_file: import_passkey.clone(),
                            passkey_rp_id: passkey_rp_id.clone(),
                            fingerprint_ua: None,
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
                            passkey_file: import_passkey.clone(),
                            passkey_rp_id: passkey_rp_id.clone(),
                            fingerprint_ua: None,
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
                                    passkey_file: import_passkey.clone(),
                            passkey_rp_id: passkey_rp_id.clone(),
                            fingerprint_ua: None,
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

pub(super) fn find_free_port() -> std::io::Result<u16> {
    let listener = std::net::TcpListener::bind("127.0.0.1:0")?;
    Ok(listener.local_addr()?.port())
}
