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
            if Path::new(SESSION_FILE).exists() {
                if let Some(info) = load_session() {
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
            }

            let result = if let Some(ref url) = connect {
                println!("{} Connecting to {}", "→".blue(), url.cyan());
                BrowserSession::connect(url).await
            } else if headless {
                println!("{} Launching headless browser...", "→".blue());
                BrowserSession::launch_headless().await
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
                    };
                    if let Err(e) = save_session(&info) {
                        eprintln!("{} Failed to save session: {e}", "✗".red());
                        std::process::exit(1);
                    }
                    println!("{} Session started", "✓".green());
                    println!("  WS: {}", ws_url.cyan());
                    println!("  File: {}", SESSION_FILE.dimmed());

                    // Keep the session alive — wait for Ctrl+C
                    println!(
                        "  {}",
                        "Press Ctrl+C to stop the browser.".dimmed()
                    );
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
        }
        SessionAction::Info => match load_session() {
            Some(info) => {
                println!("{} Active session", "●".green());
                println!("  WS:   {}", info.ws_url.cyan());
                if let Some(pid) = info.pid {
                    println!("  PID:  {}", pid.to_string().yellow());
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
