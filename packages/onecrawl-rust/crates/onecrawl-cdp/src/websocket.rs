//! WebSocket message interception via CDP Network domain.
//!
//! Captures WebSocket frames (sent and received) during a page session.

use onecrawl_browser::Page;
use onecrawl_core::{Error, Result};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::Mutex;

/// Direction of a WebSocket frame.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum WsDirection {
    Sent,
    Received,
}

/// A captured WebSocket frame.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WsFrame {
    pub url: String,
    pub direction: WsDirection,
    pub opcode: u32,
    pub payload: String,
    pub timestamp: f64,
}

/// Captures WebSocket frames during a page session.
#[derive(Clone)]
pub struct WsRecorder {
    frames: Arc<Mutex<Vec<WsFrame>>>,
}

impl Default for WsRecorder {
    fn default() -> Self {
        Self::new()
    }
}

impl WsRecorder {
    pub fn new() -> Self {
        Self {
            frames: Arc::new(Mutex::new(Vec::new())),
        }
    }

    /// Get all captured frames.
    pub async fn frames(&self) -> Vec<WsFrame> {
        self.frames.lock().await.clone()
    }

    /// Clear all frames.
    pub async fn clear(&self) {
        self.frames.lock().await.clear();
    }

    /// Number of captured frames.
    pub async fn len(&self) -> usize {
        self.frames.lock().await.len()
    }

    /// Returns true if no frames have been captured.
    pub async fn is_empty(&self) -> bool {
        self.frames.lock().await.is_empty()
    }
}

/// Start WebSocket message interception.
pub async fn start_ws_recording(page: &Page, _recorder: &WsRecorder) -> Result<()> {
    // Monkey-patch WebSocket to capture messages
    let js = r#"
        (() => {
            if (window.__onecrawl_ws_active) return 'already';
            window.__onecrawl_ws_active = true;
            window.__onecrawl_ws_frames = [];
            window.__onecrawl_ws_connections = new Map();

            const OrigWS = window.WebSocket;
            window.WebSocket = function(url, protocols) {
                const ws = protocols ? new OrigWS(url, protocols) : new OrigWS(url);
                const id = Math.random().toString(36).substr(2, 9);
                window.__onecrawl_ws_connections.set(id, url);

                ws.addEventListener('message', (evt) => {
                    window.__onecrawl_ws_frames.push({
                        url: url,
                        direction: 'received',
                        opcode: typeof evt.data === 'string' ? 1 : 2,
                        payload: typeof evt.data === 'string' ? evt.data : '[binary]',
                        timestamp: Date.now()
                    });
                });

                const origSend = ws.send.bind(ws);
                ws.send = function(data) {
                    window.__onecrawl_ws_frames.push({
                        url: url,
                        direction: 'sent',
                        opcode: typeof data === 'string' ? 1 : 2,
                        payload: typeof data === 'string' ? data : '[binary]',
                        timestamp: Date.now()
                    });
                    return origSend(data);
                };

                ws.addEventListener('close', () => {
                    window.__onecrawl_ws_connections.delete(id);
                });

                return ws;
            };
            window.WebSocket.prototype = OrigWS.prototype;
            window.WebSocket.CONNECTING = OrigWS.CONNECTING;
            window.WebSocket.OPEN = OrigWS.OPEN;
            window.WebSocket.CLOSING = OrigWS.CLOSING;
            window.WebSocket.CLOSED = OrigWS.CLOSED;

            return 'installed';
        })()
    "#;

    page.evaluate(js)
        .await
        .map_err(|e| Error::Cdp(format!("start_ws_recording failed: {e}")))?;

    Ok(())
}

/// Drain new WebSocket frames from the page.
pub async fn drain_ws_frames(page: &Page, recorder: &WsRecorder) -> Result<usize> {
    let result = page
        .evaluate(
            r#"
            (() => {
                const frames = window.__onecrawl_ws_frames || [];
                window.__onecrawl_ws_frames = [];
                return frames;
            })()
            "#,
        )
        .await
        .map_err(|e| Error::Cdp(format!("drain_ws_frames failed: {e}")))?;

    let frames: Vec<WsFrame> = match result.into_value() {
        Ok(v) => v,
        Err(_) => return Ok(0),
    };

    let count = frames.len();
    let mut stored = recorder.frames.lock().await;
    stored.extend(frames);

    Ok(count)
}

/// Get count of active WebSocket connections.
pub async fn active_ws_connections(page: &Page) -> Result<usize> {
    let result = page
        .evaluate("(window.__onecrawl_ws_connections || new Map()).size")
        .await
        .map_err(|e| Error::Cdp(format!("active_ws_connections: {e}")))?;

    let count: usize = result.into_value().unwrap_or(0);
    Ok(count)
}

/// Export all captured frames as JSON.
pub async fn export_ws_frames(recorder: &WsRecorder) -> Result<serde_json::Value> {
    let frames = recorder.frames.lock().await;
    serde_json::to_value(&*frames).map_err(|e| Error::Cdp(format!("export_ws_frames: {e}")))
}
