//! Multi-device orchestrator — coordinate browser + Android + iOS devices
//! from a single unified workflow.
//!
//! Supports parallel action execution across devices within a step,
//! variable interpolation, conditional steps, error policies, and retry logic.

use base64::Engine;
use onecrawl_core::{Error, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use super::android::{AndroidClient, AndroidSessionConfig};
use super::browser::BrowserSession;
use super::ios::{IosClient, IosSessionConfig};

// ══════════════════════════════════════════════════════════════════════
//  Types
// ══════════════════════════════════════════════════════════════════════

/// Device type in the orchestration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DeviceType {
    /// Desktop browser via Chrome DevTools Protocol.
    Browser,
    /// Android device via ADB / UIAutomator2.
    Android,
    /// iOS device via WebDriverAgent.
    Ios,
}

/// Configuration for a device participating in an orchestration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeviceConfig {
    /// User-defined device ID (e.g., "desktop", "phone", "tablet").
    pub id: String,
    pub device_type: DeviceType,

    // Browser-specific
    pub headless: Option<bool>,
    pub user_data_dir: Option<String>,
    pub viewport: Option<(u32, u32)>,

    // Android-specific
    pub adb_serial: Option<String>,
    pub appium_url: Option<String>,
    pub package_name: Option<String>,
    pub activity_name: Option<String>,

    // iOS-specific
    pub udid: Option<String>,
    pub wda_url: Option<String>,
    pub bundle_id: Option<String>,
}

/// An action targeted at a specific device.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeviceAction {
    /// Device ID from config.
    pub device: String,
    /// What to do on this device.
    pub action: OrchAction,
}

/// Actions that can be performed on any device.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum OrchAction {
    Navigate { url: String },
    Click {
        selector: Option<String>,
        x: Option<f64>,
        y: Option<f64>,
        text: Option<String>,
    },
    Type {
        selector: Option<String>,
        text: String,
        x: Option<f64>,
        y: Option<f64>,
    },
    SmartClick { query: String },
    SmartFill { query: String, value: String },
    Screenshot { path: Option<String> },
    Wait {
        selector: Option<String>,
        timeout_ms: Option<u64>,
    },
    Swipe {
        start_x: f64,
        start_y: f64,
        end_x: f64,
        end_y: f64,
        duration_ms: Option<u64>,
    },
    Back,
    Evaluate { script: String },
    Extract {
        selector: String,
        attribute: Option<String>,
    },
    Assert { condition: String, value: String },
    Sleep { ms: u64 },
    Log { message: String },
    SetVariable { name: String, value: String },
    LaunchApp {
        package: Option<String>,
        bundle_id: Option<String>,
    },
}

/// An orchestration step — can target one or multiple devices.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrchStep {
    pub name: Option<String>,
    /// Executed in parallel if targeting different devices.
    pub actions: Vec<DeviceAction>,
    /// Skip condition (variable-based, skip when "false" or empty).
    pub condition: Option<String>,
    pub on_error: Option<ErrorPolicy>,
    /// Variable capture: map of variable name → device ID whose result to capture.
    pub save_as: Option<HashMap<String, String>>,
    pub retry: Option<u32>,
}

/// How to handle errors in a step or globally.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ErrorPolicy {
    Stop,
    Continue,
    Retry,
    Skip,
}

/// Full orchestration workflow.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Orchestration {
    pub name: String,
    pub description: Option<String>,
    pub devices: HashMap<String, DeviceConfig>,
    pub variables: Option<HashMap<String, String>>,
    pub steps: Vec<OrchStep>,
    pub on_error: Option<ErrorPolicy>,
    pub timeout_secs: Option<u64>,
}

// ══════════════════════════════════════════════════════════════════════
//  Results
// ══════════════════════════════════════════════════════════════════════

/// Result of a single step.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StepResult {
    pub step_index: usize,
    pub step_name: Option<String>,
    pub device_results: Vec<DeviceActionResult>,
    pub duration_ms: u64,
    pub status: StepResultStatus,
}

/// Result of executing a single action on a single device.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeviceActionResult {
    pub device: String,
    pub action_type: String,
    pub success: bool,
    pub result: Option<serde_json::Value>,
    pub error: Option<String>,
    pub screenshot_path: Option<String>,
    pub duration_ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum StepResultStatus {
    Completed,
    Failed,
    Skipped,
    PartialSuccess,
}

/// Full orchestration result.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrchestrationResult {
    pub name: String,
    pub success: bool,
    pub steps_completed: usize,
    pub steps_total: usize,
    pub step_results: Vec<StepResult>,
    pub variables: HashMap<String, String>,
    pub duration_secs: f64,
    pub errors: Vec<String>,
}

// ══════════════════════════════════════════════════════════════════════
//  Device handle & Orchestrator
// ══════════════════════════════════════════════════════════════════════

/// Holds connected device handles.
pub struct DeviceHandle {
    pub config: DeviceConfig,
    pub browser_session: Option<BrowserSession>,
    pub browser_page: Option<chromiumoxide::Page>,
    pub android: Option<AndroidClient>,
    pub ios: Option<IosClient>,
}

/// The multi-device orchestrator engine.
pub struct Orchestrator {
    orchestration: Orchestration,
    devices: HashMap<String, DeviceHandle>,
    variables: HashMap<String, String>,
}

impl Orchestrator {
    /// Create a new orchestrator from an orchestration definition.
    pub fn new(orchestration: Orchestration) -> Self {
        let variables = orchestration.variables.clone().unwrap_or_default();
        Self {
            orchestration,
            devices: HashMap::new(),
            variables,
        }
    }

    /// Connect to all devices specified in the orchestration.
    pub async fn connect_devices(&mut self) -> Result<()> {
        let configs: Vec<(String, DeviceConfig)> = self
            .orchestration
            .devices
            .iter()
            .map(|(id, cfg)| (id.clone(), cfg.clone()))
            .collect();

        for (id, config) in configs {
            let handle = match config.device_type {
                DeviceType::Browser => {
                    let headless = config.headless.unwrap_or(true);
                    let session = if headless {
                        BrowserSession::launch_headless().await?
                    } else {
                        BrowserSession::launch_headed().await?
                    };
                    let page = session.new_page("about:blank").await?;
                    DeviceHandle {
                        config,
                        browser_session: Some(session),
                        browser_page: Some(page),
                        android: None,
                        ios: None,
                    }
                }
                DeviceType::Android => {
                    let android_config = AndroidSessionConfig {
                        server_url: config
                            .appium_url
                            .clone()
                            .unwrap_or_else(|| "http://localhost:4723".to_string()),
                        device_serial: config.adb_serial.clone(),
                        package: config
                            .package_name
                            .clone()
                            .unwrap_or_else(|| "com.android.chrome".to_string()),
                        activity: config.activity_name.clone(),
                    };
                    let mut client = AndroidClient::new(android_config);
                    client
                        .create_session(
                            config.package_name.as_deref(),
                            config.activity_name.as_deref(),
                        )
                        .await?;
                    DeviceHandle {
                        config,
                        browser_session: None,
                        browser_page: None,
                        android: Some(client),
                        ios: None,
                    }
                }
                DeviceType::Ios => {
                    let ios_config = IosSessionConfig {
                        wda_url: config
                            .wda_url
                            .clone()
                            .unwrap_or_else(|| "http://localhost:8100".to_string()),
                        device_udid: config.udid.clone(),
                        bundle_id: config
                            .bundle_id
                            .clone()
                            .unwrap_or_else(|| "com.apple.mobilesafari".to_string()),
                    };
                    let mut client = IosClient::new(ios_config);
                    client.create_session().await?;
                    DeviceHandle {
                        config,
                        browser_session: None,
                        browser_page: None,
                        android: None,
                        ios: Some(client),
                    }
                }
            };
            self.devices.insert(id, handle);
        }
        Ok(())
    }

    /// Execute the full orchestration workflow.
    pub async fn execute(&mut self) -> Result<OrchestrationResult> {
        let start = std::time::Instant::now();
        let steps_total = self.orchestration.steps.len();
        let mut step_results = Vec::new();
        let mut all_errors: Vec<String> = Vec::new();
        let global_policy = self
            .orchestration
            .on_error
            .clone()
            .unwrap_or(ErrorPolicy::Stop);

        for step_idx in 0..steps_total {
            let step = self.orchestration.steps[step_idx].clone();
            let step_start = std::time::Instant::now();

            // ── Check condition ──
            if let Some(ref cond) = step.condition {
                let interpolated = self.interpolate(cond);
                if interpolated == "false" || interpolated.is_empty() {
                    step_results.push(StepResult {
                        step_index: step_idx,
                        step_name: step.name.clone(),
                        device_results: Vec::new(),
                        duration_ms: 0,
                        status: StepResultStatus::Skipped,
                    });
                    continue;
                }
            }

            // ── Execute actions (parallel across devices) ──
            let device_results = self.execute_step_actions(&step).await;

            // ── Determine step status ──
            let any_failed = device_results.iter().any(|r| !r.success);
            let all_failed = !device_results.is_empty() && device_results.iter().all(|r| !r.success);
            let status = if all_failed {
                StepResultStatus::Failed
            } else if any_failed {
                StepResultStatus::PartialSuccess
            } else {
                StepResultStatus::Completed
            };

            // ── Collect errors ──
            for r in &device_results {
                if let Some(ref err) = r.error {
                    all_errors.push(format!("Step {} [{}]: {}", step_idx, r.device, err));
                }
            }

            // ── Capture variables (save_as) ──
            if let Some(ref save_map) = step.save_as {
                for (var_name, device_id) in save_map {
                    if let Some(dr) = device_results.iter().find(|r| r.device == *device_id) {
                        if let Some(ref val) = dr.result {
                            let val_str = match val {
                                serde_json::Value::String(s) => s.clone(),
                                other => other.to_string(),
                            };
                            self.variables.insert(var_name.clone(), val_str);
                        }
                    }
                }
            }

            // ── Handle SetVariable actions directly ──
            for da in &step.actions {
                if let OrchAction::SetVariable { name, value } = &da.action {
                    let val = self.interpolate(value);
                    self.variables.insert(name.clone(), val);
                }
            }

            let step_result = StepResult {
                step_index: step_idx,
                step_name: step.name.clone(),
                device_results,
                duration_ms: step_start.elapsed().as_millis() as u64,
                status: status.clone(),
            };
            step_results.push(step_result);

            // ── Handle error policy ──
            if status == StepResultStatus::Failed {
                let policy = step.on_error.as_ref().unwrap_or(&global_policy);
                match policy {
                    ErrorPolicy::Stop => break,
                    ErrorPolicy::Continue | ErrorPolicy::Skip => continue,
                    ErrorPolicy::Retry => {
                        let max_retries = step.retry.unwrap_or(1);
                        for _ in 0..max_retries {
                            let retry_results = self.execute_step_actions(&step).await;
                            let retry_ok = retry_results.iter().any(|r| r.success);
                            if retry_ok {
                                break;
                            }
                        }
                    }
                }
            }
        }

        let success = all_errors.is_empty();
        let steps_completed = step_results
            .iter()
            .filter(|r| r.status == StepResultStatus::Completed)
            .count();

        Ok(OrchestrationResult {
            name: self.orchestration.name.clone(),
            success,
            steps_completed,
            steps_total,
            step_results,
            variables: self.variables.clone(),
            duration_secs: start.elapsed().as_secs_f64(),
            errors: all_errors,
        })
    }

    /// Execute all actions in a step. Actions targeting different devices run in parallel.
    async fn execute_step_actions(&self, step: &OrchStep) -> Vec<DeviceActionResult> {
        let futs = step
            .actions
            .iter()
            .map(|da| self.execute_device_action(da));
        futures::future::join_all(futs).await
    }

    /// Execute a single DeviceAction, resolving the device and dispatching.
    async fn execute_device_action(&self, da: &DeviceAction) -> DeviceActionResult {
        let device_id = self.interpolate(&da.device);
        let start = std::time::Instant::now();
        let action_type = action_type_name(&da.action);

        match self.devices.get(&device_id) {
            Some(handle) => match self.run_action(handle, &da.action).await {
                Ok(result) => DeviceActionResult {
                    device: device_id,
                    action_type,
                    success: true,
                    result,
                    error: None,
                    screenshot_path: None,
                    duration_ms: start.elapsed().as_millis() as u64,
                },
                Err(err) => DeviceActionResult {
                    device: device_id,
                    action_type,
                    success: false,
                    result: None,
                    error: Some(err),
                    screenshot_path: None,
                    duration_ms: start.elapsed().as_millis() as u64,
                },
            },
            None => DeviceActionResult {
                device: device_id,
                action_type,
                success: false,
                result: None,
                error: Some("device not found".to_string()),
                screenshot_path: None,
                duration_ms: 0,
            },
        }
    }

    /// Execute a single action on a device handle.
    #[allow(clippy::too_many_lines)]
    async fn run_action(
        &self,
        handle: &DeviceHandle,
        action: &OrchAction,
    ) -> std::result::Result<Option<serde_json::Value>, String> {
        match (&handle.config.device_type, action) {
            // ════════════════════════ Browser ════════════════════════

            (DeviceType::Browser, OrchAction::Navigate { url }) => {
                let page = browser_page(handle)?;
                let url = self.interpolate(url);
                page.goto(&url)
                    .await
                    .map_err(|e| format!("navigate: {e}"))?;
                Ok(None)
            }

            (DeviceType::Browser, OrchAction::Click { selector, x, y, text }) => {
                let page = browser_page(handle)?;
                if let Some(sel) = selector {
                    let sel = self.interpolate(sel);
                    page.find_element(&sel)
                        .await
                        .map_err(|e| format!("find element: {e}"))?
                        .click()
                        .await
                        .map_err(|e| format!("click: {e}"))?;
                } else if let (Some(cx), Some(cy)) = (x, y) {
                    let js = format!(
                        "document.elementFromPoint({}, {})?.click()",
                        cx, cy
                    );
                    page.evaluate(js)
                        .await
                        .map_err(|e| format!("click at coords: {e}"))?;
                } else if let Some(txt) = text {
                    let txt = self.interpolate(txt);
                    let escaped = serde_json::to_string(&txt).unwrap_or_default();
                    let js = format!(
                        r#"(() => {{ const el = [...document.querySelectorAll('*')].find(e => (e.textContent||'').trim() === JSON.parse('{escaped}')); if(el) el.click(); else throw new Error('not found'); }})()"#
                    );
                    page.evaluate(js)
                        .await
                        .map_err(|e| format!("click by text: {e}"))?;
                } else {
                    return Err("click requires selector, coordinates, or text".into());
                }
                Ok(None)
            }

            (DeviceType::Browser, OrchAction::Type { selector, text, .. }) => {
                let page = browser_page(handle)?;
                let text = self.interpolate(text);
                if let Some(sel) = selector {
                    let sel = self.interpolate(sel);
                    page.find_element(&sel)
                        .await
                        .map_err(|e| format!("find element: {e}"))?
                        .type_str(&text)
                        .await
                        .map_err(|e| format!("type: {e}"))?;
                } else {
                    return Err("selector required for typing in browser".into());
                }
                Ok(None)
            }

            (DeviceType::Browser, OrchAction::SmartClick { query }) => {
                let page = browser_page(handle)?;
                let query = self.interpolate(query);
                let escaped = serde_json::to_string(&query).unwrap_or_default();
                let js = format!(
                    r#"(() => {{
                        const q = {escaped}.toLowerCase();
                        const all = [...document.querySelectorAll('a,button,input,[role="button"],[onclick]')];
                        const el = all.find(e => {{
                            const t = (e.textContent||'').trim().toLowerCase();
                            const a = (e.getAttribute('aria-label')||'').toLowerCase();
                            const p = (e.getAttribute('placeholder')||'').toLowerCase();
                            return t.includes(q) || a.includes(q) || p.includes(q);
                        }});
                        if(el) {{ el.click(); return 'clicked'; }}
                        throw new Error('element not found: '+{escaped});
                    }})()"#
                );
                page.evaluate(js)
                    .await
                    .map_err(|e| format!("smart_click: {e}"))?;
                Ok(None)
            }

            (DeviceType::Browser, OrchAction::SmartFill { query, value }) => {
                let page = browser_page(handle)?;
                let query = self.interpolate(query);
                let value = self.interpolate(value);
                let escaped_q = serde_json::to_string(&query).unwrap_or_default();
                let escaped_v = serde_json::to_string(&value).unwrap_or_default();
                let js = format!(
                    r#"(() => {{
                        const q = {escaped_q}.toLowerCase();
                        const inputs = [...document.querySelectorAll('input,textarea,select')];
                        const el = inputs.find(e => {{
                            const p = (e.getAttribute('placeholder')||'').toLowerCase();
                            const n = (e.getAttribute('name')||'').toLowerCase();
                            const a = (e.getAttribute('aria-label')||'').toLowerCase();
                            const l = e.labels && e.labels[0] ? e.labels[0].textContent.toLowerCase() : '';
                            return p.includes(q) || n.includes(q) || a.includes(q) || l.includes(q);
                        }});
                        if(el) {{
                            el.focus(); el.value = {escaped_v};
                            el.dispatchEvent(new Event('input', {{bubbles:true}}));
                            el.dispatchEvent(new Event('change', {{bubbles:true}}));
                            return 'filled';
                        }}
                        throw new Error('input not found: '+{escaped_q});
                    }})()"#
                );
                page.evaluate(js)
                    .await
                    .map_err(|e| format!("smart_fill: {e}"))?;
                Ok(None)
            }

            (DeviceType::Browser, OrchAction::Evaluate { script }) => {
                let page = browser_page(handle)?;
                let script = self.interpolate(script);
                let result = page
                    .evaluate(script.as_str())
                    .await
                    .map_err(|e| format!("evaluate: {e}"))?;
                Ok(result.value().cloned())
            }

            (DeviceType::Browser, OrchAction::Screenshot { path }) => {
                let page = browser_page(handle)?;
                let params = chromiumoxide::page::ScreenshotParams::builder().build();
                let bytes = page
                    .screenshot(params)
                    .await
                    .map_err(|e| format!("screenshot: {e}"))?;
                if let Some(p) = path {
                    let p = self.interpolate(p);
                    std::fs::write(&p, &bytes)
                        .map_err(|e| format!("save screenshot: {e}"))?;
                    Ok(Some(serde_json::json!({ "path": p, "bytes": bytes.len() })))
                } else {
                    Ok(Some(serde_json::json!({ "bytes": bytes.len() })))
                }
            }

            (DeviceType::Browser, OrchAction::Wait { selector, timeout_ms }) => {
                if let Some(sel) = selector {
                    let page = browser_page(handle)?;
                    let sel = self.interpolate(sel);
                    let timeout = timeout_ms.unwrap_or(30_000);
                    let deadline =
                        std::time::Instant::now() + std::time::Duration::from_millis(timeout);
                    loop {
                        if page.find_element(&sel).await.is_ok() {
                            break;
                        }
                        if std::time::Instant::now() > deadline {
                            return Err(format!("timeout waiting for selector: {sel}"));
                        }
                        tokio::time::sleep(std::time::Duration::from_millis(200)).await;
                    }
                } else {
                    let ms = timeout_ms.unwrap_or(1000);
                    tokio::time::sleep(std::time::Duration::from_millis(ms)).await;
                }
                Ok(None)
            }

            (DeviceType::Browser, OrchAction::Back) => {
                let page = browser_page(handle)?;
                page.evaluate("window.history.back()")
                    .await
                    .map_err(|e| format!("back: {e}"))?;
                Ok(None)
            }

            (DeviceType::Browser, OrchAction::Extract { selector, attribute }) => {
                let page = browser_page(handle)?;
                let sel = self.interpolate(selector);
                let element = page
                    .find_element(&sel)
                    .await
                    .map_err(|e| format!("find element: {e}"))?;
                let value = if let Some(attr) = attribute {
                    let attr = self.interpolate(attr);
                    element
                        .attribute(&attr)
                        .await
                        .map_err(|e| format!("get attribute: {e}"))?
                        .unwrap_or_default()
                } else {
                    element
                        .inner_text()
                        .await
                        .map_err(|e| format!("inner text: {e}"))?
                        .unwrap_or_default()
                };
                Ok(Some(serde_json::Value::String(value)))
            }

            // ════════════════════════ Android ════════════════════════

            (DeviceType::Android, OrchAction::Navigate { url }) => {
                let client = android_client(handle)?;
                let url = self.interpolate(url);
                client
                    .navigate(&url)
                    .await
                    .map_err(|e| format!("android navigate: {e}"))?;
                Ok(None)
            }

            (DeviceType::Android, OrchAction::Click { x, y, text, .. }) => {
                let client = android_client(handle)?;
                if let (Some(cx), Some(cy)) = (x, y) {
                    client
                        .tap(*cx, *cy)
                        .await
                        .map_err(|e| format!("android tap: {e}"))?;
                } else if let Some(txt) = text {
                    let txt = self.interpolate(txt);
                    let eid = client
                        .find_element("text", &txt)
                        .await
                        .map_err(|e| format!("android find: {e}"))?;
                    client
                        .click_element(&eid)
                        .await
                        .map_err(|e| format!("android click: {e}"))?;
                } else {
                    return Err("android click requires coordinates or text".into());
                }
                Ok(None)
            }

            (DeviceType::Android, OrchAction::Type { text, .. }) => {
                let client = android_client(handle)?;
                let text = self.interpolate(text);
                client
                    .type_text(&text)
                    .await
                    .map_err(|e| format!("android type: {e}"))?;
                Ok(None)
            }

            (DeviceType::Android, OrchAction::Screenshot { path }) => {
                let client = android_client(handle)?;
                let base64_data = client
                    .screenshot()
                    .await
                    .map_err(|e| format!("android screenshot: {e}"))?;
                if let Some(p) = path {
                    let p = self.interpolate(p);
                    let bytes = base64::engine::general_purpose::STANDARD
                        .decode(&base64_data)
                        .map_err(|e| format!("decode screenshot: {e}"))?;
                    std::fs::write(&p, &bytes)
                        .map_err(|e| format!("save screenshot: {e}"))?;
                    Ok(Some(serde_json::json!({ "path": p })))
                } else {
                    Ok(Some(
                        serde_json::json!({ "bytes": base64_data.len() }),
                    ))
                }
            }

            (DeviceType::Android, OrchAction::Swipe {
                start_x,
                start_y,
                end_x,
                end_y,
                duration_ms,
            }) => {
                let client = android_client(handle)?;
                let dur = duration_ms.unwrap_or(300);
                client
                    .swipe(*start_x, *start_y, *end_x, *end_y, dur)
                    .await
                    .map_err(|e| format!("android swipe: {e}"))?;
                Ok(None)
            }

            (DeviceType::Android, OrchAction::Back) => {
                let client = android_client(handle)?;
                client
                    .back()
                    .await
                    .map_err(|e| format!("android back: {e}"))?;
                Ok(None)
            }

            (DeviceType::Android, OrchAction::LaunchApp { package, .. }) => {
                let client = android_client(handle)?;
                let pkg = package
                    .as_ref()
                    .ok_or("package required for android launch_app")?;
                let pkg = self.interpolate(pkg);
                client
                    .launch_app(&pkg, None)
                    .await
                    .map_err(|e| format!("android launch: {e}"))?;
                Ok(None)
            }

            (DeviceType::Android, OrchAction::Wait { timeout_ms, .. }) => {
                let ms = timeout_ms.unwrap_or(1000);
                tokio::time::sleep(std::time::Duration::from_millis(ms)).await;
                Ok(None)
            }

            // ════════════════════════ iOS ════════════════════════

            (DeviceType::Ios, OrchAction::Navigate { url }) => {
                let client = ios_client(handle)?;
                let url = self.interpolate(url);
                client
                    .navigate(&url)
                    .await
                    .map_err(|e| format!("ios navigate: {e}"))?;
                Ok(None)
            }

            (DeviceType::Ios, OrchAction::Click { x, y, text, .. }) => {
                let client = ios_client(handle)?;
                if let (Some(cx), Some(cy)) = (x, y) {
                    client
                        .tap(*cx, *cy)
                        .await
                        .map_err(|e| format!("ios tap: {e}"))?;
                } else if let Some(txt) = text {
                    let txt = self.interpolate(txt);
                    let eid = client
                        .find_element("link text", &txt)
                        .await
                        .map_err(|e| format!("ios find: {e}"))?;
                    client
                        .click_element(&eid)
                        .await
                        .map_err(|e| format!("ios click: {e}"))?;
                } else {
                    return Err("ios click requires coordinates or text".into());
                }
                Ok(None)
            }

            (DeviceType::Ios, OrchAction::Type { selector, text, .. }) => {
                let client = ios_client(handle)?;
                let text = self.interpolate(text);
                if let Some(sel) = selector {
                    let sel = self.interpolate(sel);
                    let eid = client
                        .find_element("css selector", &sel)
                        .await
                        .map_err(|e| format!("ios find: {e}"))?;
                    client
                        .type_text(&eid, &text)
                        .await
                        .map_err(|e| format!("ios type: {e}"))?;
                } else {
                    return Err("selector required for typing on iOS".into());
                }
                Ok(None)
            }

            (DeviceType::Ios, OrchAction::Screenshot { path }) => {
                let client = ios_client(handle)?;
                let bytes = client
                    .screenshot()
                    .await
                    .map_err(|e| format!("ios screenshot: {e}"))?;
                if let Some(p) = path {
                    let p = self.interpolate(p);
                    std::fs::write(&p, &bytes)
                        .map_err(|e| format!("save screenshot: {e}"))?;
                    Ok(Some(
                        serde_json::json!({ "path": p, "bytes": bytes.len() }),
                    ))
                } else {
                    Ok(Some(serde_json::json!({ "bytes": bytes.len() })))
                }
            }

            (DeviceType::Ios, OrchAction::Swipe {
                start_x,
                start_y,
                end_x,
                end_y,
                duration_ms,
            }) => {
                let client = ios_client(handle)?;
                let dur_secs = duration_ms.unwrap_or(300) as f64 / 1000.0;
                client
                    .swipe(*start_x, *start_y, *end_x, *end_y, dur_secs)
                    .await
                    .map_err(|e| format!("ios swipe: {e}"))?;
                Ok(None)
            }

            (DeviceType::Ios, OrchAction::Back) => {
                let client = ios_client(handle)?;
                client
                    .execute_script(
                        "mobile: pressButton",
                        &[serde_json::json!({"name": "back"})],
                    )
                    .await
                    .map_err(|e| format!("ios back: {e}"))?;
                Ok(None)
            }

            (DeviceType::Ios, OrchAction::LaunchApp { bundle_id, .. }) => {
                let client = ios_client(handle)?;
                let bid = bundle_id
                    .as_ref()
                    .ok_or("bundle_id required for iOS launch_app")?;
                let bid = self.interpolate(bid);
                client
                    .launch_app(&bid)
                    .await
                    .map_err(|e| format!("ios launch: {e}"))?;
                Ok(None)
            }

            (DeviceType::Ios, OrchAction::Wait { timeout_ms, .. }) => {
                let ms = timeout_ms.unwrap_or(1000);
                tokio::time::sleep(std::time::Duration::from_millis(ms)).await;
                Ok(None)
            }

            // ════════════════════════ Common (all device types) ════════════════════════

            (_, OrchAction::Sleep { ms }) => {
                tokio::time::sleep(std::time::Duration::from_millis(*ms)).await;
                Ok(None)
            }

            (_, OrchAction::Log { message }) => {
                let msg = self.interpolate(message);
                tracing::info!("[{}] {}", handle.config.id, msg);
                Ok(None)
            }

            (_, OrchAction::SetVariable { name: _, value }) => {
                let val = self.interpolate(value);
                Ok(Some(serde_json::Value::String(val)))
            }

            (_, OrchAction::Assert { condition, value }) => {
                let cond = self.interpolate(condition);
                let val = self.interpolate(value);
                if cond == val {
                    Ok(None)
                } else {
                    Err(format!("assertion failed: {cond} != {val}"))
                }
            }

            // ── Unsupported combinations ──
            (device_type, _) => Err(format!(
                "{} not supported on {:?}",
                action_type_name(action),
                device_type,
            )),
        }
    }

    /// Disconnect all devices gracefully.
    pub async fn disconnect(&mut self) -> Result<()> {
        let device_ids: Vec<String> = self.devices.keys().cloned().collect();
        for id in device_ids {
            if let Some(mut handle) = self.devices.remove(&id) {
                if let Some(mut android) = handle.android.take() {
                    let _ = android.close_session().await;
                }
                if let Some(mut ios) = handle.ios.take() {
                    let _ = ios.close_session().await;
                }
                if let Some(session) = handle.browser_session.take() {
                    let _ = session.close().await;
                }
            }
        }
        Ok(())
    }

    /// Load an orchestration definition from a JSON file.
    pub fn from_file(path: &str) -> Result<Orchestration> {
        let content = std::fs::read_to_string(path)
            .map_err(|e| Error::Cdp(format!("read orchestration file: {e}")))?;
        serde_json::from_str(&content)
            .map_err(|e| Error::Cdp(format!("parse orchestration: {e}")))
    }

    /// Interpolate `${var}` references in a string using current variables.
    fn interpolate(&self, text: &str) -> String {
        let mut result = text.to_string();
        for (key, value) in &self.variables {
            result = result.replace(&format!("${{{}}}", key), value);
        }
        result
    }

    /// Validate an orchestration definition without executing it.
    pub fn validate(orchestration: &Orchestration) -> std::result::Result<(), Vec<String>> {
        let mut errors = Vec::new();

        if orchestration.name.is_empty() {
            errors.push("orchestration name is required".to_string());
        }
        if orchestration.devices.is_empty() {
            errors.push("at least one device is required".to_string());
        }
        if orchestration.steps.is_empty() {
            errors.push("at least one step is required".to_string());
        }
        for (idx, step) in orchestration.steps.iter().enumerate() {
            for action in &step.actions {
                if !orchestration.devices.contains_key(&action.device) {
                    errors.push(format!(
                        "step {}: action references unknown device '{}'",
                        idx, action.device
                    ));
                }
            }
        }

        if errors.is_empty() {
            Ok(())
        } else {
            Err(errors)
        }
    }

    /// Get the current variable state.
    pub fn variables(&self) -> &HashMap<String, String> {
        &self.variables
    }

    /// Get the device configurations.
    pub fn device_configs(&self) -> &HashMap<String, DeviceConfig> {
        &self.orchestration.devices
    }
}

// ══════════════════════════════════════════════════════════════════════
//  Helpers
// ══════════════════════════════════════════════════════════════════════

fn browser_page(handle: &DeviceHandle) -> std::result::Result<&chromiumoxide::Page, String> {
    handle
        .browser_page
        .as_ref()
        .ok_or_else(|| "browser page not initialized".to_string())
}

fn android_client(handle: &DeviceHandle) -> std::result::Result<&AndroidClient, String> {
    handle
        .android
        .as_ref()
        .ok_or_else(|| "android client not initialized".to_string())
}

fn ios_client(handle: &DeviceHandle) -> std::result::Result<&IosClient, String> {
    handle
        .ios
        .as_ref()
        .ok_or_else(|| "ios client not initialized".to_string())
}

fn action_type_name(action: &OrchAction) -> String {
    match action {
        OrchAction::Navigate { .. } => "navigate",
        OrchAction::Click { .. } => "click",
        OrchAction::Type { .. } => "type",
        OrchAction::SmartClick { .. } => "smart_click",
        OrchAction::SmartFill { .. } => "smart_fill",
        OrchAction::Screenshot { .. } => "screenshot",
        OrchAction::Wait { .. } => "wait",
        OrchAction::Swipe { .. } => "swipe",
        OrchAction::Back => "back",
        OrchAction::Evaluate { .. } => "evaluate",
        OrchAction::Extract { .. } => "extract",
        OrchAction::Assert { .. } => "assert",
        OrchAction::Sleep { .. } => "sleep",
        OrchAction::Log { .. } => "log",
        OrchAction::SetVariable { .. } => "set_variable",
        OrchAction::LaunchApp { .. } => "launch_app",
    }
    .to_string()
}
