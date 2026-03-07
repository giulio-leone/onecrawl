use std::collections::HashMap;
use std::sync::Arc;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::UnixListener;
use tokio::sync::{Mutex, Notify};

use serde::{Deserialize, Serialize};

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

#[derive(Serialize, Deserialize)]
struct PersistedState {
    sessions: Vec<String>,
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
            self.save_state();
        }
        Ok(&self.sessions[name].page)
    }

    fn save_state(&self) {
        let persisted = PersistedState {
            sessions: self.sessions.keys().cloned().collect(),
            headless: self.headless,
        };
        if let Ok(data) = serde_json::to_string_pretty(&persisted) {
            let _ = std::fs::write(STATE_FILE, data);
        }
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
            let mut sigterm = match tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate()) {
                Ok(s) => s,
                Err(e) => {
                    eprintln!("warning: failed to register SIGTERM: {e}");
                    shutdown.notify_one();
                    return;
                }
            };
            let mut sigint = match tokio::signal::unix::signal(tokio::signal::unix::SignalKind::interrupt()) {
                Ok(s) => s,
                Err(e) => {
                    eprintln!("warning: failed to register SIGINT: {e}");
                    shutdown.notify_one();
                    return;
                }
            };
            tokio::select! {
                _ = sigterm.recv() => {},
                _ = sigint.recv() => {},
            }
            eprintln!("daemon received shutdown signal");
            shutdown.notify_one();
        });
    }

    // Health monitoring — check session liveness every 60 seconds.
    {
        let health_state = Arc::clone(&state);
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(std::time::Duration::from_secs(60));
            interval.tick().await; // skip immediate first tick
            loop {
                interval.tick().await;
                let sessions_snapshot: Vec<(String, Page)> = {
                    let ds = health_state.lock().await;
                    ds.sessions
                        .iter()
                        .map(|(n, s)| (n.clone(), s.page.clone()))
                        .collect()
                };
                let mut dead = Vec::new();
                for (name, page) in &sessions_snapshot {
                    if onecrawl_cdp::harness::health_check(page).await.is_err() {
                        eprintln!("[daemon] session '{}' health check failed", name);
                        dead.push(name.clone());
                    }
                }
                if !dead.is_empty() {
                    let mut ds = health_state.lock().await;
                    for name in &dead {
                        ds.sessions.remove(name);
                        eprintln!("[daemon] removed dead session '{}'", name);
                    }
                    ds.save_state();
                }
            }
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

    {
        let ds = state.lock().await;
        ds.save_state();
    }
    cleanup();
    let _ = std::fs::remove_file(STATE_FILE);
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

        "session_list" => {
            let ds = state.lock().await;
            let names: Vec<String> = ds.sessions.keys().cloned().collect();
            DaemonResponse {
                id,
                success: true,
                data: Some(serde_json::json!({"sessions": names, "count": names.len()})),
                error: None,
            }
        }

        "session_close" => {
            let name = match req.args.get("name").and_then(|v| v.as_str()) {
                Some(n) => n.to_string(),
                None => {
                    return DaemonResponse {
                        id,
                        success: false,
                        data: None,
                        error: Some("missing required argument: name".to_string()),
                    };
                }
            };
            let mut ds = state.lock().await;
            if ds.sessions.remove(&name).is_some() {
                ds.save_state();
                DaemonResponse {
                    id,
                    success: true,
                    data: Some(serde_json::json!({"closed": name})),
                    error: None,
                }
            } else {
                DaemonResponse {
                    id,
                    success: false,
                    data: None,
                    error: Some(format!("session not found: {name}")),
                }
            }
        }

        // Browser commands that require a page.
        "goto" | "snapshot" | "click" | "type" | "evaluate" | "screenshot"
        | "fill" | "hover" | "scroll" | "keyboard" | "select" | "wait"
        | "text" | "html" | "back" | "forward" | "reload" | "health" => {
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

/// Extract a required string argument, returning a descriptive error if missing.
fn args_str(key: &str, args: &serde_json::Value) -> Result<String, String> {
    args.get(key)
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())
        .ok_or_else(|| format!("missing required argument: {key}"))
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

            "fill" => {
                let selector = args_str("selector", args)?;
                let text = args_str("text", args)?;
                onecrawl_cdp::keyboard::fill(page, &selector, &text)
                    .await
                    .map_err(|e| e.to_string())?;
                Ok(serde_json::json!({"filled": selector}))
            }

            "hover" => {
                let selector = args_str("selector", args)?;
                onecrawl_cdp::element::hover(page, &selector)
                    .await
                    .map_err(|e| e.to_string())?;
                Ok(serde_json::json!({"hovered": selector}))
            }

            "scroll" => {
                let direction = args.get("direction").and_then(|v| v.as_str()).unwrap_or("down");
                let amount = args.get("amount").and_then(|v| v.as_f64()).unwrap_or(300.0) as i64;
                let (dx, dy) = match direction {
                    "up" => (0, -amount),
                    "down" => (0, amount),
                    "left" => (-amount, 0),
                    "right" => (amount, 0),
                    _ => (0, amount),
                };
                onecrawl_cdp::human::human_scroll(page, dx, dy)
                    .await
                    .map_err(|e| e.to_string())?;
                Ok(serde_json::json!({"scrolled": direction, "amount": amount}))
            }

            "keyboard" => {
                let keys = args_str("keys", args)?;
                if keys.contains('+') {
                    onecrawl_cdp::keyboard::keyboard_shortcut(page, &keys)
                        .await
                        .map_err(|e| e.to_string())?;
                } else {
                    onecrawl_cdp::keyboard::press_key(page, &keys)
                        .await
                        .map_err(|e| e.to_string())?;
                }
                Ok(serde_json::json!({"pressed": keys}))
            }

            "select" => {
                let selector = args_str("selector", args)?;
                let value = args_str("value", args)?;
                onecrawl_cdp::element::select_option(page, &selector, &value)
                    .await
                    .map_err(|e| e.to_string())?;
                Ok(serde_json::json!({"selected": value, "in": selector}))
            }

            "wait" => {
                let selector = args_str("selector", args)?;
                let timeout = args.get("timeout_ms").and_then(|v| v.as_u64()).unwrap_or(5000);
                onecrawl_cdp::navigation::wait_for_selector(page, &selector, timeout)
                    .await
                    .map_err(|e| e.to_string())?;
                Ok(serde_json::json!({"found": selector}))
            }

            "text" => {
                let selector = args.get("selector").and_then(|v| v.as_str());
                let text = if let Some(sel) = selector {
                    onecrawl_cdp::element::get_text(page, sel)
                        .await
                        .map_err(|e| e.to_string())?
                } else {
                    let result = onecrawl_cdp::extract::extract(
                        page,
                        None,
                        onecrawl_cdp::extract::ExtractFormat::Text,
                    )
                    .await
                    .map_err(|e| e.to_string())?;
                    result.content
                };
                Ok(serde_json::json!({"text": text}))
            }

            "html" => {
                let selector = args.get("selector").and_then(|v| v.as_str());
                let result = onecrawl_cdp::extract::extract(
                    page,
                    selector,
                    onecrawl_cdp::extract::ExtractFormat::Html,
                )
                .await
                .map_err(|e| e.to_string())?;
                Ok(serde_json::json!({"html": result.content}))
            }

            "back" => {
                onecrawl_cdp::navigation::go_back(page)
                    .await
                    .map_err(|e| e.to_string())?;
                Ok(serde_json::json!({"navigated": "back"}))
            }

            "forward" => {
                onecrawl_cdp::navigation::go_forward(page)
                    .await
                    .map_err(|e| e.to_string())?;
                Ok(serde_json::json!({"navigated": "forward"}))
            }

            "reload" => {
                onecrawl_cdp::navigation::reload(page)
                    .await
                    .map_err(|e| e.to_string())?;
                Ok(serde_json::json!({"reloaded": true}))
            }

            "health" => {
                let health = onecrawl_cdp::harness::health_check(page)
                    .await
                    .map_err(|e| e.to_string())?;
                Ok(health)
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
