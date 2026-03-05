//! HTTP proxy client for routing CLI commands through the persistent server.
//!
//! When `session start` launches a browser it also spawns an HTTP server.
//! Subsequent CLI invocations use this proxy (single HTTP call) instead of
//! re-connecting to Chrome via CDP WebSocket (~40-80 ms savings per command).

use crate::commands::session::load_session;
use serde::Serialize;
use std::time::Duration;

/// Typed request bodies (avoid json!() macro overhead per call).
#[derive(Serialize)]
struct NavRequest<'a> {
    url: &'a str,
}

#[derive(Serialize)]
struct EvalRequest<'a> {
    expression: &'a str,
}

/// Lightweight client that talks to the co-located HTTP server.
pub struct ServerProxy {
    client: reqwest::Client,
    /// Pre-computed URLs to avoid format!() on every call
    navigate_url: String,
    text_url: String,
    evaluate_url: String,
    screenshot_url: String,
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
            .gzip(true)
            .build()
            .ok()?;
        let tab_base = format!("http://127.0.0.1:{port}/tabs/{tab_id}");
        let proxy = Self {
            navigate_url: format!("{tab_base}/navigate"),
            text_url: format!("{tab_base}/text"),
            evaluate_url: format!("{tab_base}/evaluate"),
            screenshot_url: format!("{tab_base}/screenshot"),
            client,
        };
        // Quick health-check with tight timeout
        proxy.client
            .get(format!("http://127.0.0.1:{port}/health"))
            .timeout(Duration::from_millis(500))
            .send()
            .await
            .ok()?;
        Some(proxy)
    }

    pub async fn navigate(&self, url: &str) -> Result<serde_json::Value, String> {
        let resp = self
            .client
            .post(&self.navigate_url)
            .json(&NavRequest { url })
            .send()
            .await
            .map_err(|e| e.to_string())?;
        resp.json().await.map_err(|e| e.to_string())
    }

    pub async fn get_text(&self) -> Result<String, String> {
        let resp = self
            .client
            .get(&self.text_url)
            .send()
            .await
            .map_err(|e| e.to_string())?;
        let body: serde_json::Value = resp.json().await.map_err(|e| e.to_string())?;
        Ok(body["text"].as_str().unwrap_or("").to_owned())
    }

    pub async fn evaluate(&self, expr: &str) -> Result<serde_json::Value, String> {
        let resp = self
            .client
            .post(&self.evaluate_url)
            .json(&EvalRequest { expression: expr })
            .send()
            .await
            .map_err(|e| e.to_string())?;
        resp.json().await.map_err(|e| e.to_string())
    }

    pub async fn screenshot(&self) -> Result<Vec<u8>, String> {
        let resp = self
            .client
            .get(&self.screenshot_url)
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
