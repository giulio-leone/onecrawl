use clap::Subcommand;
use colored::Colorize;
use onecrawl_cdp::BrowserSession;
use serde::{Deserialize, Serialize};
use std::path::Path;

const SESSION_FILE: &str = "/tmp/onecrawl-session.json";

#[derive(Debug, Serialize, Deserialize)]
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
fn save_session(info: &SessionInfo) -> std::io::Result<()> {
    let data = serde_json::to_string_pretty(info)?;
    std::fs::write(SESSION_FILE, data)
}

/// Remove session file.
fn remove_session() {
    let _ = std::fs::remove_file(SESSION_FILE);
}

/// Connect to the active session and return the first page.
pub async fn connect_to_session() -> Result<(BrowserSession, onecrawl_cdp::Page), String> {
    let info = load_session().ok_or_else(|| {
        format!(
            "No active session. Run {} first.",
            "onecrawl session start".yellow()
        )
    })?;
    let session = BrowserSession::connect(&info.ws_url)
        .await
        .map_err(|e| format!("Failed to connect to session: {e}"))?;

    let pages = session
        .browser()
        .pages()
        .await
        .map_err(|e| format!("Failed to list pages: {e}"))?;

    let page = if let Some(p) = pages.into_iter().next() {
        p
    } else {
        session
            .new_page("about:blank")
            .await
            .map_err(|e| format!("Failed to create page: {e}"))?
    };

    Ok((session, page))
}

pub async fn handle(action: SessionAction) {
    match action {
        SessionAction::Start {
            headless,
            connect,
            background: _,
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

            if connect.is_some() || !headless {
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
                        };
                        if let Err(e) = save_session(&info) {
                            eprintln!("{} Failed to save session: {e}", "✗".red());
                            std::process::exit(1);
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
