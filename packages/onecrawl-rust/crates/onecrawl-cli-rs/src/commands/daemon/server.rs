use std::collections::HashMap;
use std::sync::Arc;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::UnixListener;
use tokio::sync::{Mutex, Notify};

use onecrawl_cdp::{BrowserSession, Page};

use super::protocol::*;

/// Per-session state: one browser + one active page.
struct SessionState {
    _session: BrowserSession,
    page: Page,
}

/// Shared daemon state accessible from every connection handler.
struct DaemonState {
    sessions: HashMap<String, SessionState>,
    headless: bool,
}

impl DaemonState {
    /// Get or create a named session. The first session is created at startup;
    /// additional sessions are lazily created on demand.
    async fn get_or_create_session(
        &mut self,
        name: &str,
    ) -> Result<&Page, String> {
        if !self.sessions.contains_key(name) {
            let sess = if self.headless {
                BrowserSession::launch_headless()
                    .await
                    .map_err(|e| format!("launch headless failed: {e}"))?
            } else {
                BrowserSession::launch_headed()
                    .await
                    .map_err(|e| format!("launch headed failed: {e}"))?
            };
            let page = sess
                .new_page("about:blank")
                .await
                .map_err(|e| format!("new page failed: {e}"))?;
            self.sessions.insert(
                name.to_string(),
                SessionState {
                    _session: sess,
                    page,
                },
            );
        }
        Ok(&self.sessions[name].page)
    }
}

/// Start the persistent daemon, binding to a Unix socket.
pub async fn start_daemon(headless: bool) -> Result<(), Box<dyn std::error::Error>> {
    // Clean up stale socket from a previous unclean shutdown.
    let _ = std::fs::remove_file(SOCKET_PATH);

    // Write PID file.
    std::fs::write(PID_FILE, std::process::id().to_string())?;

    // Launch the default browser session eagerly.
    let session = if headless {
        BrowserSession::launch_headless().await?
    } else {
        BrowserSession::launch_headed().await?
    };
    let page = session.new_page("about:blank").await?;

    let mut sessions = HashMap::new();
    sessions.insert(
        "default".to_string(),
        SessionState {
            _session: session,
            page,
        },
    );

    let state = Arc::new(Mutex::new(DaemonState { sessions, headless }));
    let shutdown = Arc::new(Notify::new());
    let idle_reset = Arc::new(Notify::new());

    // Bind the Unix listener.
    let listener = UnixListener::bind(SOCKET_PATH)?;

    eprintln!(
        "onecrawl daemon running  pid={}  socket={}",
        std::process::id(),
        SOCKET_PATH
    );

    // Spawn idle-timeout watcher.
    {
        let shutdown = Arc::clone(&shutdown);
        let idle_reset = Arc::clone(&idle_reset);
        tokio::spawn(async move {
            loop {
                tokio::select! {
                    _ = tokio::time::sleep(std::time::Duration::from_secs(DEFAULT_IDLE_TIMEOUT_SECS)) => {
                        eprintln!("daemon idle timeout reached — shutting down");
                        shutdown.notify_one();
                        return;
                    }
                    _ = idle_reset.notified() => {
                        // Reset the timer by looping.
                    }
                }
            }
        });
    }

    // Spawn signal handler (SIGTERM / SIGINT).
    {
        let shutdown = Arc::clone(&shutdown);
        tokio::spawn(async move {
            let mut sigterm =
                tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
                    .expect("failed to register SIGTERM");
            let mut sigint =
                tokio::signal::unix::signal(tokio::signal::unix::SignalKind::interrupt())
                    .expect("failed to register SIGINT");
            tokio::select! {
                _ = sigterm.recv() => {},
                _ = sigint.recv() => {},
            }
            eprintln!("daemon received shutdown signal");
            shutdown.notify_one();
        });
    }

    // Accept loop — exits on shutdown notification.
    loop {
        tokio::select! {
            accept_result = listener.accept() => {
                let (stream, _addr) = accept_result?;
                let state = Arc::clone(&state);
                let shutdown = Arc::clone(&shutdown);
                let idle_reset = Arc::clone(&idle_reset);
                tokio::spawn(async move {
                    handle_connection(stream, state, shutdown, idle_reset).await;
                });
            }
            _ = shutdown.notified() => {
                break;
            }
        }
    }

    cleanup();
    eprintln!("daemon shut down cleanly");
    Ok(())
}

/// Handle a single client connection (one JSON-line per request).
async fn handle_connection(
    stream: tokio::net::UnixStream,
    state: Arc<Mutex<DaemonState>>,
    shutdown: Arc<Notify>,
    idle_reset: Arc<Notify>,
) {
    let (reader, mut writer) = stream.into_split();
    let mut lines = BufReader::new(reader).lines();

    while let Ok(Some(line)) = lines.next_line().await {
        // Reset idle timer on every command.
        idle_reset.notify_one();

        let req: DaemonRequest = match serde_json::from_str(&line) {
            Ok(r) => r,
            Err(e) => {
                let resp = DaemonResponse {
                    id: String::new(),
                    success: false,
                    data: None,
                    error: Some(format!("invalid request: {e}")),
                };
                let _ = write_response(&mut writer, &resp).await;
                continue;
            }
        };

        let session_name = req
            .session
            .as_deref()
            .unwrap_or("default")
            .to_string();

        let resp = dispatch_command(&req, &session_name, &state, &shutdown).await;
        if write_response(&mut writer, &resp).await.is_err() {
            break;
        }

        // If the command was shutdown, stop processing.
        if req.command == "shutdown" {
            break;
        }
    }
}

async fn write_response(
    writer: &mut tokio::net::unix::OwnedWriteHalf,
    resp: &DaemonResponse,
) -> Result<(), std::io::Error> {
    let mut buf = serde_json::to_vec(resp).unwrap_or_default();
    buf.push(b'\n');
    writer.write_all(&buf).await
}

/// Route a request to the appropriate handler.
async fn dispatch_command(
    req: &DaemonRequest,
    session_name: &str,
    state: &Arc<Mutex<DaemonState>>,
    shutdown: &Arc<Notify>,
) -> DaemonResponse {
    let id = req.id.clone();

    match req.command.as_str() {
        "ping" => DaemonResponse {
            id,
            success: true,
            data: Some(serde_json::json!("pong")),
            error: None,
        },

        "status" => {
            let st = state.lock().await;
            let sessions: Vec<&String> = st.sessions.keys().collect();
            DaemonResponse {
                id,
                success: true,
                data: Some(serde_json::json!({
                    "pid": std::process::id(),
                    "headless": st.headless,
                    "sessions": sessions,
                })),
                error: None,
            }
        }

        "shutdown" => {
            shutdown.notify_one();
            DaemonResponse {
                id,
                success: true,
                data: Some(serde_json::json!("shutting down")),
                error: None,
            }
        }

        // Browser commands that require a page.
        "goto" | "snapshot" | "click" | "type" | "evaluate" | "screenshot" => {
            let mut st = state.lock().await;
            let page = match st.get_or_create_session(session_name).await {
                Ok(p) => p.clone(),
                Err(e) => {
                    return DaemonResponse {
                        id,
                        success: false,
                        data: None,
                        error: Some(e),
                    };
                }
            };
            drop(st);
            exec_browser_command(&req.command, &req.args, &page, id).await
        }

        other => DaemonResponse {
            id,
            success: false,
            data: None,
            error: Some(format!("unknown command: {other}")),
        },
    }
}

/// Execute a browser-level command on the given page.
async fn exec_browser_command(
    command: &str,
    args: &serde_json::Value,
    page: &Page,
    id: String,
) -> DaemonResponse {
    let result: Result<serde_json::Value, String> = (async {
        match command {
            "goto" => {
                let url = args
                    .get("url")
                    .and_then(|v| v.as_str())
                    .unwrap_or("about:blank");
                onecrawl_cdp::navigation::goto(page, url)
                    .await
                    .map_err(|e| e.to_string())?;
                Ok(serde_json::json!({ "navigated": url }))
            }

            "snapshot" => {
                let opts = onecrawl_cdp::accessibility::AgentSnapshotOptions::default();
                let snap = onecrawl_cdp::accessibility::agent_snapshot(page, &opts)
                    .await
                    .map_err(|e| e.to_string())?;
                Ok(serde_json::json!({ "snapshot": snap.snapshot }))
            }

            "click" => {
                let selector = args
                    .get("selector")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| "missing `selector` arg".to_string())?;
                let resolved = onecrawl_cdp::accessibility::resolve_ref(selector);
                onecrawl_cdp::element::click(page, &resolved)
                    .await
                    .map_err(|e| e.to_string())?;
                Ok(serde_json::json!({ "clicked": selector }))
            }

            "type" => {
                let selector = args
                    .get("selector")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| "missing `selector` arg".to_string())?;
                let text = args
                    .get("text")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| "missing `text` arg".to_string())?;
                let resolved = onecrawl_cdp::accessibility::resolve_ref(selector);
                onecrawl_cdp::element::type_text(page, &resolved, text)
                    .await
                    .map_err(|e| e.to_string())?;
                Ok(serde_json::json!({ "typed": text, "into": selector }))
            }

            "evaluate" => {
                let expr = args
                    .get("expression")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| "missing `expression` arg".to_string())?;
                let result = onecrawl_cdp::element::evaluate(page, expr)
                    .await
                    .map_err(|e| e.to_string())?;
                Ok(serde_json::json!({ "result": result }))
            }

            "screenshot" => {
                let data = onecrawl_cdp::screenshot::screenshot_viewport(page)
                    .await
                    .map_err(|e| e.to_string())?;
                use base64::Engine;
                let b64 = base64::engine::general_purpose::STANDARD.encode(&data);
                Ok(serde_json::json!({ "screenshot_base64": b64, "bytes": data.len() }))
            }

            _ => Err(format!("unhandled browser command: {command}")),
        }
    })
    .await;

    match result {
        Ok(data) => DaemonResponse {
            id,
            success: true,
            data: Some(data),
            error: None,
        },
        Err(e) => DaemonResponse {
            id,
            success: false,
            data: None,
            error: Some(e),
        },
    }
}

/// Remove PID and socket files on shutdown.
fn cleanup() {
    let _ = std::fs::remove_file(SOCKET_PATH);
    let _ = std::fs::remove_file(PID_FILE);
}
