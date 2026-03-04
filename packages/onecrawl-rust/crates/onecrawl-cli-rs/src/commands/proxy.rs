//! HTTP proxy client for routing CLI commands through the persistent server.
//!
//! When `session start` launches a browser it also spawns an HTTP server.
//! Subsequent CLI invocations use this proxy (single HTTP call) instead of
//! re-connecting to Chrome via CDP WebSocket (~40-80 ms savings per command).

use crate::commands::session::load_session;
use std::time::Duration;

/// Lightweight client that talks to the co-located HTTP server.
pub struct ServerProxy {
    client: reqwest::Client,
    /// Pre-computed base URL for tab operations: `http://127.0.0.1:{port}/tabs/{tab_id}`
    tab_url: String,
    pub _tab_id: String,
}

impl ServerProxy {
    /// Try to build a proxy from the saved session file.
    /// Returns `None` if the session has no server_port / tab_id or the server
    /// is unreachable.
    pub async fn from_session() -> Option<Self> {
        let session = load_session()?;
        let port = session.server_port?;
        let tab_id = session.default_tab_id?;
        let client = reqwest::Client::builder()
            .connect_timeout(Duration::from_millis(500))
            .timeout(Duration::from_secs(30))
            .pool_max_idle_per_host(1)
            .build()
            .ok()?;
        let tab_url = format!("http://127.0.0.1:{port}/tabs/{tab_id}");
        let proxy = Self { client, tab_url, _tab_id: tab_id };
        // Quick health-check with tight timeout
        proxy.client
            .get(format!("http://127.0.0.1:{port}/health"))
            .timeout(Duration::from_millis(500))
            .send()
            .await
            .ok()?;
        Some(proxy)
    }

    // ── Navigation ──────────────────────────────────────────────

    pub async fn navigate(&self, url: &str) -> Result<serde_json::Value, String> {
        let resp = self
            .client
            .post(format!("{}/navigate", self.tab_url))
            .json(&serde_json::json!({ "url": url }))
            .send()
            .await
            .map_err(|e| e.to_string())?;
        resp.json().await.map_err(|e| e.to_string())
    }

    // ── Content ─────────────────────────────────────────────────

    pub async fn get_text(&self) -> Result<String, String> {
        let resp = self
            .client
            .get(format!("{}/text", self.tab_url))
            .send()
            .await
            .map_err(|e| e.to_string())?;
        let body: serde_json::Value = resp.json().await.map_err(|e| e.to_string())?;
        Ok(body["text"].as_str().unwrap_or("").to_owned())
    }

    pub async fn evaluate(&self, expr: &str) -> Result<serde_json::Value, String> {
        let resp = self
            .client
            .post(format!("{}/evaluate", self.tab_url))
            .json(&serde_json::json!({ "expression": expr }))
            .send()
            .await
            .map_err(|e| e.to_string())?;
        resp.json().await.map_err(|e| e.to_string())
    }

    // ── Screenshot ──────────────────────────────────────────────

    pub async fn screenshot(&self) -> Result<Vec<u8>, String> {
        let resp = self
            .client
            .get(format!("{}/screenshot", self.tab_url))
            .send()
            .await
            .map_err(|e| e.to_string())?;
        let body: serde_json::Value = resp.json().await.map_err(|e| e.to_string())?;
        let b64 = body["data"].as_str().unwrap_or("");
        use base64::Engine as _;
        base64::engine::general_purpose::STANDARD
            .decode(b64)
            .map_err(|e| e.to_string())
    }
}
