//! Android device automation via ADB + UIAutomator2 HTTP server.
//!
//! Communicates with the UIAutomator2 server (same protocol as Appium's
//! UIAutomator2 driver) over HTTP to automate Android devices and emulators.
//! ADB commands are used for device management, file transfer, and shell access.

use onecrawl_core::Result;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

/// Android session configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AndroidSessionConfig {
    /// UIAutomator2 server URL (default: `http://localhost:4723`).
    pub server_url: String,
    /// Device serial (optional, auto-detect).
    pub device_serial: Option<String>,
    /// Android package to automate (default: `com.android.chrome`).
    pub package: String,
    /// Activity to launch (optional).
    pub activity: Option<String>,
}

impl Default for AndroidSessionConfig {
    fn default() -> Self {
        Self {
            server_url: "http://localhost:4723".to_string(),
            device_serial: None,
            package: "com.android.chrome".to_string(),
            activity: None,
        }
    }
}

/// Android automation client — communicates with UIAutomator2 via HTTP and ADB.
pub struct AndroidClient {
    config: AndroidSessionConfig,
    session_id: Option<String>,
    client: reqwest::Client,
}

impl AndroidClient {
    pub fn new(config: AndroidSessionConfig) -> Self {
        Self {
            config,
            session_id: None,
            client: reqwest::Client::new(),
        }
    }

    // ─── HTTP helpers ───────────────────────────────────────────

    fn url(&self, path: &str) -> String {
        format!("{}{}", self.config.server_url, path)
    }

    async fn get(&self, path: &str) -> Result<Value> {
        let resp = self
            .client
            .get(self.url(path))
            .send()
            .await
            .map_err(|e| onecrawl_core::Error::Cdp(format!("Android GET {path} failed: {e}")))?;
        resp.json()
            .await
            .map_err(|e| onecrawl_core::Error::Cdp(format!("Android GET {path} parse: {e}")))
    }

    async fn post(&self, path: &str, body: &Value) -> Result<Value> {
        let resp = self
            .client
            .post(self.url(path))
            .json(body)
            .send()
            .await
            .map_err(|e| onecrawl_core::Error::Cdp(format!("Android POST {path} failed: {e}")))?;
        resp.json()
            .await
            .map_err(|e| onecrawl_core::Error::Cdp(format!("Android POST {path} parse: {e}")))
    }

    async fn delete(&self, path: &str) -> Result<Value> {
        let resp = self
            .client
            .delete(self.url(path))
            .send()
            .await
            .map_err(|e| onecrawl_core::Error::Cdp(format!("Android DELETE {path} failed: {e}")))?;
        resp.json()
            .await
            .map_err(|e| onecrawl_core::Error::Cdp(format!("Android DELETE {path} parse: {e}")))
    }

    fn session_path(&self) -> Result<String> {
        let sid = self
            .session_id
            .as_ref()
            .ok_or_else(|| onecrawl_core::Error::Cdp("No active Android session".to_string()))?;
        Ok(format!("/session/{sid}"))
    }

    // ─── Session management ─────────────────────────────────────

    /// Create a new UIAutomator2 session.
    pub async fn create_session(
        &mut self,
        package: Option<&str>,
        activity: Option<&str>,
    ) -> Result<String> {
        let pkg = package.unwrap_or(&self.config.package);
        let mut caps = json!({
            "platformName": "Android",
            "automationName": "UiAutomator2",
            "appPackage": pkg,
            "noReset": true,
        });
        if let Some(act) = activity.or(self.config.activity.as_deref()) {
            caps["appActivity"] = json!(act);
        }
        if let Some(serial) = &self.config.device_serial {
            caps["udid"] = json!(serial);
        }
        let body = json!({
            "capabilities": {
                "alwaysMatch": caps
            }
        });
        let resp = self.post("/session", &body).await?;
        let session_id = resp["value"]["sessionId"]
            .as_str()
            .unwrap_or("")
            .to_string();
        self.session_id = Some(session_id.clone());
        Ok(session_id)
    }

    /// Close the current session.
    pub async fn close_session(&mut self) -> Result<()> {
        if let Ok(path) = self.session_path() {
            let _ = self.delete(&path).await;
        }
        self.session_id = None;
        Ok(())
    }

    // ─── Navigation (Chrome on Android) ─────────────────────────

    /// Navigate Chrome to a URL.
    pub async fn navigate(&self, url: &str) -> Result<()> {
        let path = self.session_path()?;
        self.post(&format!("{path}/url"), &json!({"url": url}))
            .await?;
        Ok(())
    }

    /// Get current page URL.
    pub async fn get_url(&self) -> Result<String> {
        let path = self.session_path()?;
        let resp = self.get(&format!("{path}/url")).await?;
        Ok(resp["value"].as_str().unwrap_or("").to_string())
    }

    /// Get current page title.
    pub async fn get_title(&self) -> Result<String> {
        let path = self.session_path()?;
        let resp = self.get(&format!("{path}/title")).await?;
        Ok(resp["value"].as_str().unwrap_or("").to_string())
    }

    /// Press the back button.
    pub async fn back(&self) -> Result<()> {
        let path = self.session_path()?;
        self.post(&format!("{path}/back"), &json!({})).await?;
        Ok(())
    }

    // ─── Touch gestures ─────────────────────────────────────────

    /// Tap at coordinates.
    pub async fn tap(&self, x: f64, y: f64) -> Result<()> {
        let path = self.session_path()?;
        self.post(
            &format!("{path}/actions"),
            &json!({
                "actions": [{
                    "type": "pointer",
                    "id": "finger1",
                    "parameters": {"pointerType": "touch"},
                    "actions": [
                        {"type": "pointerMove", "duration": 0, "x": x as i64, "y": y as i64},
                        {"type": "pointerDown", "button": 0},
                        {"type": "pointerUp", "button": 0}
                    ]
                }]
            }),
        )
        .await?;
        Ok(())
    }

    /// Swipe from one point to another.
    pub async fn swipe(
        &self,
        from_x: f64,
        from_y: f64,
        to_x: f64,
        to_y: f64,
        duration_ms: u64,
    ) -> Result<()> {
        let path = self.session_path()?;
        self.post(
            &format!("{path}/actions"),
            &json!({
                "actions": [{
                    "type": "pointer",
                    "id": "finger1",
                    "parameters": {"pointerType": "touch"},
                    "actions": [
                        {"type": "pointerMove", "duration": 0, "x": from_x as i64, "y": from_y as i64},
                        {"type": "pointerDown", "button": 0},
                        {"type": "pointerMove", "duration": duration_ms, "x": to_x as i64, "y": to_y as i64},
                        {"type": "pointerUp", "button": 0}
                    ]
                }]
            }),
        )
        .await?;
        Ok(())
    }

    /// Long press at coordinates.
    pub async fn long_press(&self, x: f64, y: f64, duration_ms: u64) -> Result<()> {
        let path = self.session_path()?;
        self.post(
            &format!("{path}/actions"),
            &json!({
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
            }),
        )
        .await?;
        Ok(())
    }

    /// Double tap at coordinates.
    pub async fn double_tap(&self, x: f64, y: f64) -> Result<()> {
        let path = self.session_path()?;
        self.post(
            &format!("{path}/actions"),
            &json!({
                "actions": [{
                    "type": "pointer",
                    "id": "finger1",
                    "parameters": {"pointerType": "touch"},
                    "actions": [
                        {"type": "pointerMove", "duration": 0, "x": x as i64, "y": y as i64},
                        {"type": "pointerDown", "button": 0},
                        {"type": "pointerUp", "button": 0},
                        {"type": "pause", "duration": 100},
                        {"type": "pointerDown", "button": 0},
                        {"type": "pointerUp", "button": 0}
                    ]
                }]
            }),
        )
        .await?;
        Ok(())
    }

    /// Pinch gesture at coordinates (two-finger zoom).
    pub async fn pinch(&self, x: f64, y: f64, scale: f64) -> Result<()> {
        let path = self.session_path()?;
        let offset = 100.0;
        let end_offset = offset * scale;
        self.post(
            &format!("{path}/actions"),
            &json!({
                "actions": [
                    {
                        "type": "pointer",
                        "id": "finger1",
                        "parameters": {"pointerType": "touch"},
                        "actions": [
                            {"type": "pointerMove", "duration": 0, "x": (x - offset) as i64, "y": y as i64},
                            {"type": "pointerDown", "button": 0},
                            {"type": "pointerMove", "duration": 500, "x": (x - end_offset) as i64, "y": y as i64},
                            {"type": "pointerUp", "button": 0}
                        ]
                    },
                    {
                        "type": "pointer",
                        "id": "finger2",
                        "parameters": {"pointerType": "touch"},
                        "actions": [
                            {"type": "pointerMove", "duration": 0, "x": (x + offset) as i64, "y": y as i64},
                            {"type": "pointerDown", "button": 0},
                            {"type": "pointerMove", "duration": 500, "x": (x + end_offset) as i64, "y": y as i64},
                            {"type": "pointerUp", "button": 0}
                        ]
                    }
                ]
            }),
        )
        .await?;
        Ok(())
    }

    /// Type text (sends keys to the focused element).
    pub async fn type_text(&self, text: &str) -> Result<()> {
        let path = self.session_path()?;
        // Find the currently active element and send keys to it
        let active = self.get(&format!("{path}/element/active")).await?;
        let element_id = active["value"]["ELEMENT"]
            .as_str()
            .or_else(|| {
                active["value"]
                    .as_object()
                    .and_then(|obj| obj.values().next())
                    .and_then(|v| v.as_str())
            })
            .unwrap_or("")
            .to_string();
        if element_id.is_empty() {
            return Err(onecrawl_core::Error::Cdp(
                "No active element to type into".to_string(),
            ));
        }
        self.post(
            &format!("{path}/element/{element_id}/value"),
            &json!({
                "text": text,
                "value": text.chars().map(|c| c.to_string()).collect::<Vec<_>>()
            }),
        )
        .await?;
        Ok(())
    }

    // ─── Element interaction ────────────────────────────────────

    /// Find element by locator strategy (e.g. `"accessibility id"`, `"id"`, `"xpath"`).
    pub async fn find_element(&self, strategy: &str, value: &str) -> Result<String> {
        let path = self.session_path()?;
        let resp = self
            .post(
                &format!("{path}/element"),
                &json!({"using": strategy, "value": value}),
            )
            .await?;
        let element_id = resp["value"]["ELEMENT"]
            .as_str()
            .or_else(|| {
                resp["value"]
                    .as_object()
                    .and_then(|obj| obj.values().next())
                    .and_then(|v| v.as_str())
            })
            .unwrap_or("")
            .to_string();
        Ok(element_id)
    }

    /// Click/tap an element by its element ID.
    pub async fn click_element(&self, element_id: &str) -> Result<()> {
        let path = self.session_path()?;
        self.post(&format!("{path}/element/{element_id}/click"), &json!({}))
            .await?;
        Ok(())
    }

    /// Get text content of an element.
    pub async fn element_text(&self, element_id: &str) -> Result<String> {
        let path = self.session_path()?;
        let resp = self
            .get(&format!("{path}/element/{element_id}/text"))
            .await?;
        Ok(resp["value"].as_str().unwrap_or("").to_string())
    }

    // ─── Screenshots ────────────────────────────────────────────

    /// Take a screenshot (returns base64-encoded PNG).
    pub async fn screenshot(&self) -> Result<String> {
        let path = self.session_path()?;
        let resp = self.get(&format!("{path}/screenshot")).await?;
        Ok(resp["value"].as_str().unwrap_or("").to_string())
    }

    // ─── Device control ─────────────────────────────────────────

    /// Set device orientation.
    pub async fn set_orientation(&self, orientation: &str) -> Result<()> {
        let path = self.session_path()?;
        self.post(
            &format!("{path}/orientation"),
            &json!({"orientation": orientation.to_uppercase()}),
        )
        .await?;
        Ok(())
    }

    /// Get device orientation.
    pub async fn get_orientation(&self) -> Result<String> {
        let path = self.session_path()?;
        let resp = self.get(&format!("{path}/orientation")).await?;
        Ok(resp["value"].as_str().unwrap_or("PORTRAIT").to_string())
    }

    /// Press a hardware key by Android keycode.
    pub async fn press_key(&self, keycode: i32) -> Result<()> {
        let path = self.session_path()?;
        self.post(
            &format!("{path}/appium/device/press_keycode"),
            &json!({"keycode": keycode}),
        )
        .await?;
        Ok(())
    }

    /// Get screen size.
    pub async fn get_screen_size(&self) -> Result<Value> {
        let path = self.session_path()?;
        let resp = self.get(&format!("{path}/window/current/size")).await?;
        Ok(resp["value"].clone())
    }

    // ─── App management ─────────────────────────────────────────

    /// Launch an app by package name.
    pub async fn launch_app(&self, package: &str, activity: Option<&str>) -> Result<()> {
        let path = self.session_path()?;
        let mut body = json!({"appPackage": package});
        if let Some(act) = activity {
            body["appActivity"] = json!(act);
        }
        self.post(&format!("{path}/appium/device/activate_app"), &body)
            .await?;
        Ok(())
    }

    /// Terminate (kill) an app by package name.
    pub async fn terminate_app(&self, package: &str) -> Result<()> {
        let path = self.session_path()?;
        self.post(
            &format!("{path}/appium/device/terminate_app"),
            &json!({"appId": package}),
        )
        .await?;
        Ok(())
    }

    /// Get app state (1=not running, 2=bg, 3=bg suspended, 4=foreground).
    pub async fn app_state(&self, package: &str) -> Result<u8> {
        let path = self.session_path()?;
        let resp = self
            .post(
                &format!("{path}/appium/device/app_state"),
                &json!({"appId": package}),
            )
            .await?;
        Ok(resp["value"].as_u64().unwrap_or(1) as u8)
    }

    /// Install an APK from a local path.
    pub async fn install_app(&self, apk_path: &str) -> Result<()> {
        let path = self.session_path()?;
        self.post(
            &format!("{path}/appium/device/install_app"),
            &json!({"appPath": apk_path}),
        )
        .await?;
        Ok(())
    }

    // ─── JavaScript (Chrome context) ────────────────────────────

    /// Execute JavaScript in Chrome context.
    pub async fn execute_script(&self, script: &str, args: &[Value]) -> Result<Value> {
        let path = self.session_path()?;
        let resp = self
            .post(
                &format!("{path}/execute/sync"),
                &json!({"script": script, "args": args}),
            )
            .await?;
        Ok(resp["value"].clone())
    }

    // ─── ADB commands (static) ──────────────────────────────────

    /// List connected Android devices via `adb devices`.
    pub async fn list_devices() -> Result<Value> {
        let out = tokio::process::Command::new("adb")
            .args(["devices", "-l"])
            .output()
            .await
            .map_err(|e| onecrawl_core::Error::Cdp(format!("adb devices: {e}")))?;
        let text = String::from_utf8_lossy(&out.stdout);
        let devices: Vec<Value> = text
            .lines()
            .skip(1)
            .filter(|l| !l.trim().is_empty())
            .map(|l| {
                let parts: Vec<&str> = l.split_whitespace().collect();
                json!({
                    "serial": parts.first().unwrap_or(&""),
                    "state": parts.get(1).unwrap_or(&""),
                    "info": parts.get(2..).map(|p| p.join(" ")).unwrap_or_default()
                })
            })
            .collect();
        let count = devices.len();
        Ok(json!({"devices": devices, "count": count}))
    }

    /// Run a shell command on a device via `adb -s <serial> shell`.
    pub async fn shell(serial: &str, command: &str) -> Result<String> {
        let out = tokio::process::Command::new("adb")
            .args(["-s", serial, "shell", command])
            .output()
            .await
            .map_err(|e| onecrawl_core::Error::Cdp(format!("adb shell: {e}")))?;
        Ok(String::from_utf8_lossy(&out.stdout).trim().to_string())
    }

    /// Push a file to a device.
    pub async fn push_file(serial: &str, local: &str, remote: &str) -> Result<()> {
        let out = tokio::process::Command::new("adb")
            .args(["-s", serial, "push", local, remote])
            .output()
            .await
            .map_err(|e| onecrawl_core::Error::Cdp(format!("adb push: {e}")))?;
        if !out.status.success() {
            return Err(onecrawl_core::Error::Cdp(format!(
                "adb push failed: {}",
                String::from_utf8_lossy(&out.stderr)
            )));
        }
        Ok(())
    }

    /// Pull a file from a device.
    pub async fn pull_file(serial: &str, remote: &str, local: &str) -> Result<()> {
        let out = tokio::process::Command::new("adb")
            .args(["-s", serial, "pull", remote, local])
            .output()
            .await
            .map_err(|e| onecrawl_core::Error::Cdp(format!("adb pull: {e}")))?;
        if !out.status.success() {
            return Err(onecrawl_core::Error::Cdp(format!(
                "adb pull failed: {}",
                String::from_utf8_lossy(&out.stderr)
            )));
        }
        Ok(())
    }

    /// Get device info via ADB (model, manufacturer, Android version, SDK).
    pub async fn device_info(serial: &str) -> Result<Value> {
        let model = Self::shell(serial, "getprop ro.product.model").await?;
        let manufacturer = Self::shell(serial, "getprop ro.product.manufacturer").await?;
        let version = Self::shell(serial, "getprop ro.build.version.release").await?;
        let sdk = Self::shell(serial, "getprop ro.build.version.sdk").await?;
        let device = Self::shell(serial, "getprop ro.product.device").await?;
        Ok(json!({
            "serial": serial,
            "model": model,
            "manufacturer": manufacturer,
            "android_version": version,
            "sdk_version": sdk,
            "device": device
        }))
    }

    /// Get battery info via ADB.
    pub async fn battery_info(serial: &str) -> Result<Value> {
        let output = Self::shell(serial, "dumpsys battery").await?;
        let mut info = serde_json::Map::new();
        for line in output.lines() {
            let line = line.trim();
            if let Some((key, val)) = line.split_once(':') {
                let key = key.trim().to_lowercase().replace(' ', "_");
                let val = val.trim();
                info.insert(key, json!(val));
            }
        }
        Ok(Value::Object(info))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_config() {
        let cfg = AndroidSessionConfig::default();
        assert_eq!(cfg.server_url, "http://localhost:4723");
        assert_eq!(cfg.package, "com.android.chrome");
        assert!(cfg.device_serial.is_none());
        assert!(cfg.activity.is_none());
    }

    #[test]
    fn client_no_session_errors() {
        let client = AndroidClient::new(AndroidSessionConfig::default());
        assert!(client.session_path().is_err());
    }
}
