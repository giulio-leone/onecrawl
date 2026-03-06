//! iOS/Mobile Safari automation via WebDriverAgent (WDA) protocol.
//!
//! Communicates with WebDriverAgent over HTTP to automate Mobile Safari
//! on real iOS devices and simulators.

use base64::Engine;
use onecrawl_core::Result;
use serde::{Deserialize, Serialize};

/// iOS device info returned by `xcrun simctl list`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IosDevice {
    pub udid: String,
    pub name: String,
    /// `"iOS"` or `"tvOS"`
    pub platform: String,
    pub version: String,
    pub is_simulator: bool,
}

/// iOS session configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IosSessionConfig {
    /// WebDriverAgent URL (default: `http://localhost:8100`).
    pub wda_url: String,
    /// Device UDID (optional, auto-detect).
    pub device_udid: Option<String>,
    /// Bundle ID to automate (default: `com.apple.mobilesafari`).
    pub bundle_id: String,
}

impl Default for IosSessionConfig {
    fn default() -> Self {
        Self {
            wda_url: "http://localhost:8100".to_string(),
            device_udid: None,
            bundle_id: "com.apple.mobilesafari".to_string(),
        }
    }
}

/// iOS automation client — communicates with WebDriverAgent via HTTP.
pub struct IosClient {
    config: IosSessionConfig,
    session_id: Option<String>,
    client: reqwest::Client,
}

impl IosClient {
    pub fn new(config: IosSessionConfig) -> Self {
        Self {
            config,
            session_id: None,
            client: reqwest::Client::new(),
        }
    }

    /// Create a new WDA session.
    pub async fn create_session(&mut self) -> Result<String> {
        let body = serde_json::json!({
            "capabilities": {
                "alwaysMatch": {
                    "bundleId": self.config.bundle_id,
                }
            }
        });
        let resp = self
            .client
            .post(format!("{}/session", self.config.wda_url))
            .json(&body)
            .send()
            .await
            .map_err(|e| onecrawl_core::Error::Cdp(format!("WDA session failed: {e}")))?;
        let json: serde_json::Value = resp
            .json()
            .await
            .map_err(|e| onecrawl_core::Error::Cdp(format!("WDA response parse failed: {e}")))?;
        let session_id = json["value"]["sessionId"]
            .as_str()
            .unwrap_or("")
            .to_string();
        self.session_id = Some(session_id.clone());
        Ok(session_id)
    }

    fn session_url(&self) -> Result<String> {
        let sid = self
            .session_id
            .as_ref()
            .ok_or_else(|| onecrawl_core::Error::Cdp("No active iOS session".to_string()))?;
        Ok(format!("{}/session/{}", self.config.wda_url, sid))
    }

    /// Navigate Safari to a URL.
    pub async fn navigate(&self, url: &str) -> Result<()> {
        let base = self.session_url()?;
        self.client
            .post(format!("{base}/url"))
            .json(&serde_json::json!({"url": url}))
            .send()
            .await
            .map_err(|e| onecrawl_core::Error::Cdp(format!("iOS navigate failed: {e}")))?;
        Ok(())
    }

    /// Tap at coordinates.
    pub async fn tap(&self, x: f64, y: f64) -> Result<()> {
        let base = self.session_url()?;
        self.client
            .post(format!("{base}/wda/tap/0"))
            .json(&serde_json::json!({"x": x, "y": y}))
            .send()
            .await
            .map_err(|e| onecrawl_core::Error::Cdp(format!("iOS tap failed: {e}")))?;
        Ok(())
    }

    /// Swipe gesture.
    pub async fn swipe(
        &self,
        from_x: f64,
        from_y: f64,
        to_x: f64,
        to_y: f64,
        duration: f64,
    ) -> Result<()> {
        let base = self.session_url()?;
        self.client
            .post(format!("{base}/wda/dragfromtoforduration"))
            .json(&serde_json::json!({
                "fromX": from_x, "fromY": from_y,
                "toX": to_x, "toY": to_y,
                "duration": duration
            }))
            .send()
            .await
            .map_err(|e| onecrawl_core::Error::Cdp(format!("iOS swipe failed: {e}")))?;
        Ok(())
    }

    /// Take a screenshot (returns raw PNG/JPEG bytes).
    pub async fn screenshot(&self) -> Result<Vec<u8>> {
        let base = self.session_url()?;
        let resp = self
            .client
            .get(format!("{base}/screenshot"))
            .send()
            .await
            .map_err(|e| onecrawl_core::Error::Cdp(format!("iOS screenshot failed: {e}")))?;
        let json: serde_json::Value = resp
            .json()
            .await
            .map_err(|e| {
                onecrawl_core::Error::Cdp(format!("iOS screenshot parse failed: {e}"))
            })?;
        let b64 = json["value"].as_str().unwrap_or("");
        let bytes = base64::engine::general_purpose::STANDARD
            .decode(b64)
            .map_err(|e| onecrawl_core::Error::Cdp(format!("base64 decode failed: {e}")))?;
        Ok(bytes)
    }

    /// Get page source (accessibility tree XML).
    pub async fn page_source(&self) -> Result<String> {
        let base = self.session_url()?;
        let resp = self
            .client
            .get(format!("{base}/source"))
            .send()
            .await
            .map_err(|e| onecrawl_core::Error::Cdp(format!("iOS source failed: {e}")))?;
        let json: serde_json::Value = resp
            .json()
            .await
            .map_err(|e| onecrawl_core::Error::Cdp(format!("iOS source parse failed: {e}")))?;
        Ok(json["value"].as_str().unwrap_or("").to_string())
    }

    /// Find element by locator strategy (e.g. `"accessibility id"`, `"class name"`).
    pub async fn find_element(&self, using: &str, value: &str) -> Result<String> {
        let base = self.session_url()?;
        let resp = self
            .client
            .post(format!("{base}/element"))
            .json(&serde_json::json!({"using": using, "value": value}))
            .send()
            .await
            .map_err(|e| {
                onecrawl_core::Error::Cdp(format!("iOS find element failed: {e}"))
            })?;
        let json: serde_json::Value = resp
            .json()
            .await
            .map_err(|e| {
                onecrawl_core::Error::Cdp(format!("iOS element parse failed: {e}"))
            })?;
        // WDA returns {"value": {"ELEMENT": "..."}} or similar keyed object
        let element_id = json["value"]["ELEMENT"]
            .as_str()
            .or_else(|| {
                json["value"]
                    .as_object()
                    .and_then(|obj| obj.values().next())
                    .and_then(|v| v.as_str())
            })
            .unwrap_or("")
            .to_string();
        Ok(element_id)
    }

    /// Click/tap an element by its WDA element ID.
    pub async fn click_element(&self, element_id: &str) -> Result<()> {
        let base = self.session_url()?;
        self.client
            .post(format!("{base}/element/{element_id}/click"))
            .send()
            .await
            .map_err(|e| onecrawl_core::Error::Cdp(format!("iOS click failed: {e}")))?;
        Ok(())
    }

    /// Type text into an element.
    pub async fn type_text(&self, element_id: &str, text: &str) -> Result<()> {
        let base = self.session_url()?;
        self.client
            .post(format!("{base}/element/{element_id}/value"))
            .json(&serde_json::json!({
                "value": text.chars().map(|c| c.to_string()).collect::<Vec<_>>()
            }))
            .send()
            .await
            .map_err(|e| onecrawl_core::Error::Cdp(format!("iOS type failed: {e}")))?;
        Ok(())
    }

    /// Close the current WDA session.
    pub async fn close_session(&mut self) -> Result<()> {
        if let Ok(base) = self.session_url() {
            let _ = self.client.delete(&base).send().await;
        }
        self.session_id = None;
        Ok(())
    }

    /// List available iOS simulator devices via `xcrun simctl`.
    pub fn list_devices() -> Result<Vec<IosDevice>> {
        let output = std::process::Command::new("xcrun")
            .args(["simctl", "list", "devices", "--json"])
            .output()
            .map_err(|e| onecrawl_core::Error::Cdp(format!("xcrun simctl failed: {e}")))?;
        let json: serde_json::Value = serde_json::from_slice(&output.stdout)
            .map_err(|e| onecrawl_core::Error::Cdp(format!("parse simctl output: {e}")))?;
        let mut devices = Vec::new();
        if let Some(runtimes) = json["devices"].as_object() {
            for (runtime, devs) in runtimes {
                if let Some(arr) = devs.as_array() {
                    for dev in arr {
                        if dev["isAvailable"].as_bool() == Some(true) {
                            devices.push(IosDevice {
                                udid: dev["udid"].as_str().unwrap_or("").to_string(),
                                name: dev["name"].as_str().unwrap_or("").to_string(),
                                platform: "iOS".to_string(),
                                version: runtime.split('.').last().unwrap_or("").to_string(),
                                is_simulator: true,
                            });
                        }
                    }
                }
            }
        }
        Ok(devices)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_config() {
        let cfg = IosSessionConfig::default();
        assert_eq!(cfg.wda_url, "http://localhost:8100");
        assert_eq!(cfg.bundle_id, "com.apple.mobilesafari");
        assert!(cfg.device_udid.is_none());
    }

    #[test]
    fn client_no_session_errors() {
        let client = IosClient::new(IosSessionConfig::default());
        assert!(client.session_url().is_err());
    }
}
