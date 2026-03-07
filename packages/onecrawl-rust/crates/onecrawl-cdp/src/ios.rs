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
        if session_id.is_empty() {
            return Err(onecrawl_core::Error::Cdp("WDA session creation returned empty session ID".into()));
        }
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

    /// Pinch gesture (zoom in/out).
    pub async fn pinch(&self, x: f64, y: f64, scale: f64, velocity: f64) -> Result<()> {
        let base = self.session_url()?;
        self.client
            .post(format!("{base}/wda/pinch"))
            .json(&serde_json::json!({
                "x": x, "y": y, "scale": scale, "velocity": velocity
            }))
            .send()
            .await
            .map_err(|e| onecrawl_core::Error::Cdp(format!("iOS pinch failed: {e}")))?;
        Ok(())
    }

    /// Long press at coordinates.
    pub async fn long_press(&self, x: f64, y: f64, duration_ms: u64) -> Result<()> {
        let base = self.session_url()?;
        self.client
            .post(format!("{base}/actions"))
            .json(&serde_json::json!({
                "actions": [{
                    "type": "pointer",
                    "id": "finger1",
                    "parameters": {"pointerType": "touch"},
                    "actions": [
                        {"type": "pointerMove", "duration": 0, "x": x as i64, "y": y as i64},
                        {"type": "pointerDown", "button": 0},
                        {"type": "pause", "duration": duration_ms},
                        {"type": "pointerUp", "button": 0}
                    ]
                }]
            }))
            .send()
            .await
            .map_err(|e| onecrawl_core::Error::Cdp(format!("iOS long press failed: {e}")))?;
        Ok(())
    }

    /// Double tap at coordinates.
    pub async fn double_tap(&self, x: f64, y: f64) -> Result<()> {
        let base = self.session_url()?;
        self.client
            .post(format!("{base}/wda/doubleTap"))
            .json(&serde_json::json!({"x": x, "y": y}))
            .send()
            .await
            .map_err(|e| onecrawl_core::Error::Cdp(format!("iOS double tap failed: {e}")))?;
        Ok(())
    }

    /// Set device orientation.
    pub async fn set_orientation(&self, orientation: &str) -> Result<()> {
        let base = self.session_url()?;
        self.client
            .post(format!("{base}/orientation"))
            .json(&serde_json::json!({"orientation": orientation.to_uppercase()}))
            .send()
            .await
            .map_err(|e| onecrawl_core::Error::Cdp(format!("iOS set orientation failed: {e}")))?;
        Ok(())
    }

    /// Get device orientation.
    pub async fn get_orientation(&self) -> Result<String> {
        let base = self.session_url()?;
        let resp = self
            .client
            .get(format!("{base}/orientation"))
            .send()
            .await
            .map_err(|e| onecrawl_core::Error::Cdp(format!("iOS get orientation failed: {e}")))?;
        let json: serde_json::Value = resp
            .json()
            .await
            .map_err(|e| {
                onecrawl_core::Error::Cdp(format!("iOS orientation parse failed: {e}"))
            })?;
        Ok(json["value"].as_str().unwrap_or("PORTRAIT").to_string())
    }

    /// Get device screen size.
    pub async fn get_screen_size(&self) -> Result<serde_json::Value> {
        let base = self.session_url()?;
        let resp = self
            .client
            .get(format!("{base}/window/size"))
            .send()
            .await
            .map_err(|e| onecrawl_core::Error::Cdp(format!("iOS screen size failed: {e}")))?;
        let json: serde_json::Value = resp
            .json()
            .await
            .map_err(|e| {
                onecrawl_core::Error::Cdp(format!("iOS screen size parse failed: {e}"))
            })?;
        Ok(json["value"].clone())
    }

    /// Scroll to element by locator strategy.
    pub async fn scroll_to_element(&self, using: &str, value: &str) -> Result<()> {
        let el_id = self.find_element(using, value).await?;
        let base = self.session_url()?;
        self.client
            .post(format!("{base}/wda/element/{el_id}/scroll"))
            .json(&serde_json::json!({"toVisible": true}))
            .send()
            .await
            .map_err(|e| onecrawl_core::Error::Cdp(format!("iOS scroll failed: {e}")))?;
        Ok(())
    }

    /// Get current page URL (Safari).
    pub async fn get_url(&self) -> Result<String> {
        let base = self.session_url()?;
        let resp = self
            .client
            .get(format!("{base}/url"))
            .send()
            .await
            .map_err(|e| onecrawl_core::Error::Cdp(format!("iOS get URL failed: {e}")))?;
        let json: serde_json::Value = resp
            .json()
            .await
            .map_err(|e| onecrawl_core::Error::Cdp(format!("iOS URL parse failed: {e}")))?;
        Ok(json["value"].as_str().unwrap_or("").to_string())
    }

    /// Get page title (Safari).
    pub async fn get_title(&self) -> Result<String> {
        let base = self.session_url()?;
        let resp = self
            .client
            .get(format!("{base}/title"))
            .send()
            .await
            .map_err(|e| onecrawl_core::Error::Cdp(format!("iOS get title failed: {e}")))?;
        let json: serde_json::Value = resp
            .json()
            .await
            .map_err(|e| onecrawl_core::Error::Cdp(format!("iOS title parse failed: {e}")))?;
        Ok(json["value"].as_str().unwrap_or("").to_string())
    }

    /// Execute JavaScript in Safari.
    pub async fn execute_script(
        &self,
        script: &str,
        args: &[serde_json::Value],
    ) -> Result<serde_json::Value> {
        let base = self.session_url()?;
        let resp = self
            .client
            .post(format!("{base}/execute/sync"))
            .json(&serde_json::json!({"script": script, "args": args}))
            .send()
            .await
            .map_err(|e| onecrawl_core::Error::Cdp(format!("iOS execute script failed: {e}")))?;
        let json: serde_json::Value = resp
            .json()
            .await
            .map_err(|e| {
                onecrawl_core::Error::Cdp(format!("iOS script parse failed: {e}"))
            })?;
        Ok(json["value"].clone())
    }

    /// Get all cookies (Safari).
    pub async fn get_cookies(&self) -> Result<serde_json::Value> {
        let base = self.session_url()?;
        let resp = self
            .client
            .get(format!("{base}/cookie"))
            .send()
            .await
            .map_err(|e| onecrawl_core::Error::Cdp(format!("iOS get cookies failed: {e}")))?;
        let json: serde_json::Value = resp
            .json()
            .await
            .map_err(|e| {
                onecrawl_core::Error::Cdp(format!("iOS cookies parse failed: {e}"))
            })?;
        Ok(json["value"].clone())
    }

    /// Launch app by bundle ID.
    pub async fn launch_app(&self, bundle_id: &str) -> Result<()> {
        let base = self.session_url()?;
        self.client
            .post(format!("{base}/wda/apps/launch"))
            .json(&serde_json::json!({"bundleId": bundle_id}))
            .send()
            .await
            .map_err(|e| onecrawl_core::Error::Cdp(format!("iOS launch app failed: {e}")))?;
        Ok(())
    }

    /// Terminate (kill) app by bundle ID.
    pub async fn terminate_app(&self, bundle_id: &str) -> Result<()> {
        let base = self.session_url()?;
        self.client
            .post(format!("{base}/wda/apps/terminate"))
            .json(&serde_json::json!({"bundleId": bundle_id}))
            .send()
            .await
            .map_err(|e| onecrawl_core::Error::Cdp(format!("iOS terminate app failed: {e}")))?;
        Ok(())
    }

    /// Get app state (1=not running, 2=bg suspended, 3=bg, 4=foreground).
    pub async fn app_state(&self, bundle_id: &str) -> Result<u8> {
        let base = self.session_url()?;
        let resp = self
            .client
            .post(format!("{base}/wda/apps/state"))
            .json(&serde_json::json!({"bundleId": bundle_id}))
            .send()
            .await
            .map_err(|e| onecrawl_core::Error::Cdp(format!("iOS app state failed: {e}")))?;
        let json: serde_json::Value = resp
            .json()
            .await
            .map_err(|e| {
                onecrawl_core::Error::Cdp(format!("iOS app state parse failed: {e}"))
            })?;
        Ok(json["value"].as_u64().unwrap_or(1) as u8)
    }

    /// Lock device.
    pub async fn lock_device(&self) -> Result<()> {
        let base = self.session_url()?;
        self.client
            .post(format!("{base}/wda/lock"))
            .json(&serde_json::json!({}))
            .send()
            .await
            .map_err(|e| onecrawl_core::Error::Cdp(format!("iOS lock failed: {e}")))?;
        Ok(())
    }

    /// Unlock device.
    pub async fn unlock_device(&self) -> Result<()> {
        let base = self.session_url()?;
        self.client
            .post(format!("{base}/wda/unlock"))
            .json(&serde_json::json!({}))
            .send()
            .await
            .map_err(|e| onecrawl_core::Error::Cdp(format!("iOS unlock failed: {e}")))?;
        Ok(())
    }

    /// Press home button.
    pub async fn home_button(&self) -> Result<()> {
        self.client
            .post(format!("{}/wda/homescreen", self.config.wda_url))
            .json(&serde_json::json!({}))
            .send()
            .await
            .map_err(|e| onecrawl_core::Error::Cdp(format!("iOS home button failed: {e}")))?;
        Ok(())
    }

    /// Press a hardware button (e.g. `"volumeUp"`, `"volumeDown"`).
    pub async fn press_button(&self, name: &str) -> Result<()> {
        let base = self.session_url()?;
        self.client
            .post(format!("{base}/wda/pressButton"))
            .json(&serde_json::json!({"name": name}))
            .send()
            .await
            .map_err(|e| onecrawl_core::Error::Cdp(format!("iOS press button failed: {e}")))?;
        Ok(())
    }

    /// Get battery info.
    pub async fn battery_info(&self) -> Result<serde_json::Value> {
        let base = self.session_url()?;
        let resp = self
            .client
            .get(format!("{base}/wda/batteryInfo"))
            .send()
            .await
            .map_err(|e| onecrawl_core::Error::Cdp(format!("iOS battery info failed: {e}")))?;
        let json: serde_json::Value = resp
            .json()
            .await
            .map_err(|e| {
                onecrawl_core::Error::Cdp(format!("iOS battery info parse failed: {e}"))
            })?;
        Ok(json["value"].clone())
    }

    /// Get device info (model, name, OS version).
    pub async fn device_info(&self) -> Result<serde_json::Value> {
        let base = self.session_url()?;
        let resp = self
            .client
            .get(format!("{base}/wda/device/info"))
            .send()
            .await
            .map_err(|e| onecrawl_core::Error::Cdp(format!("iOS device info failed: {e}")))?;
        let json: serde_json::Value = resp
            .json()
            .await
            .map_err(|e| {
                onecrawl_core::Error::Cdp(format!("iOS device info parse failed: {e}"))
            })?;
        Ok(json["value"].clone())
    }

    /// Manage iOS simulators: list, boot, shutdown, create, delete.
    pub async fn simulator_action(
        action: &str,
        udid: Option<&str>,
        device_type: Option<&str>,
        runtime: Option<&str>,
    ) -> Result<serde_json::Value> {
        use tokio::process::Command;
        match action {
            "list" => {
                let out = Command::new("xcrun")
                    .args(["simctl", "list", "devices", "--json"])
                    .output()
                    .await
                    .map_err(|e| onecrawl_core::Error::Cdp(format!("xcrun simctl: {e}")))?;
                let json: serde_json::Value = serde_json::from_slice(&out.stdout)
                    .unwrap_or(serde_json::json!({"error": "parse failed"}));
                Ok(json)
            }
            "boot" => {
                let udid =
                    udid.ok_or(onecrawl_core::Error::Cdp("udid required for boot".into()))?;
                Command::new("xcrun")
                    .args(["simctl", "boot", udid])
                    .output()
                    .await
                    .map_err(|e| onecrawl_core::Error::Cdp(e.to_string()))?;
                Ok(serde_json::json!({"status": "booted", "udid": udid}))
            }
            "shutdown" => {
                let udid = udid
                    .ok_or(onecrawl_core::Error::Cdp("udid required for shutdown".into()))?;
                Command::new("xcrun")
                    .args(["simctl", "shutdown", udid])
                    .output()
                    .await
                    .map_err(|e| onecrawl_core::Error::Cdp(e.to_string()))?;
                Ok(serde_json::json!({"status": "shutdown", "udid": udid}))
            }
            "create" => {
                let dt = device_type
                    .ok_or(onecrawl_core::Error::Cdp("device_type required".into()))?;
                let rt =
                    runtime.ok_or(onecrawl_core::Error::Cdp("runtime required".into()))?;
                let out = Command::new("xcrun")
                    .args(["simctl", "create", "OneCrawl", dt, rt])
                    .output()
                    .await
                    .map_err(|e| onecrawl_core::Error::Cdp(e.to_string()))?;
                let new_udid = String::from_utf8_lossy(&out.stdout).trim().to_string();
                Ok(serde_json::json!({"status": "created", "udid": new_udid}))
            }
            "delete" => {
                let udid = udid
                    .ok_or(onecrawl_core::Error::Cdp("udid required for delete".into()))?;
                Command::new("xcrun")
                    .args(["simctl", "delete", udid])
                    .output()
                    .await
                    .map_err(|e| onecrawl_core::Error::Cdp(e.to_string()))?;
                Ok(serde_json::json!({"status": "deleted", "udid": udid}))
            }
            _ => Err(onecrawl_core::Error::Cdp(format!(
                "unknown simulator action: {action}"
            ))),
        }
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
