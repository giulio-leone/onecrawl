//! Advanced browser emulation: sensors, permissions, battery, network info, hardware.

use chromiumoxide::Page;
use onecrawl_core::Result;
use serde::{Deserialize, Serialize};

/// Device orientation sensor reading.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SensorReading {
    pub alpha: f64,
    pub beta: f64,
    pub gamma: f64,
}

/// Override device orientation sensor.
pub async fn set_device_orientation(page: &Page, reading: SensorReading) -> Result<()> {
    let js = format!(
        r#"(() => {{
        window.addEventListener('deviceorientation', function() {{}});
        Object.defineProperty(window, '__onecrawl_orientation', {{
            value: {{ alpha: {}, beta: {}, gamma: {} }},
            writable: true,
            configurable: true
        }});
        window.dispatchEvent(new DeviceOrientationEvent('deviceorientation', {{
            alpha: {}, beta: {}, gamma: {}
        }}));
    }})()"#,
        reading.alpha, reading.beta, reading.gamma, reading.alpha, reading.beta, reading.gamma
    );
    page.evaluate(js)
        .await
        .map_err(|e| onecrawl_core::Error::Cdp(format!("set_device_orientation: {e}")))?;
    Ok(())
}

/// Override a permission query result.
pub async fn override_permission(page: &Page, permission: &str, state: &str) -> Result<()> {
    let js = format!(
        r#"(() => {{
        const origQuery = navigator.permissions.query.bind(navigator.permissions);
        navigator.permissions.query = function(desc) {{
            if (desc.name === '{}') {{
                return Promise.resolve({{ state: '{}', onchange: null }});
            }}
            return origQuery(desc);
        }};
    }})()"#,
        permission.replace('\\', "\\\\").replace('\'', "\\'"),
        state.replace('\\', "\\\\").replace('\'', "\\'")
    );
    page.evaluate(js)
        .await
        .map_err(|e| onecrawl_core::Error::Cdp(format!("override_permission: {e}")))?;
    Ok(())
}

/// Override battery status API.
pub async fn set_battery_status(page: &Page, level: f64, charging: bool) -> Result<()> {
    let charging_time = if charging { "0" } else { "Infinity" };
    let js = format!(
        r#"Object.defineProperty(navigator, 'getBattery', {{
            value: () => Promise.resolve({{
                charging: {},
                chargingTime: {},
                dischargingTime: Infinity,
                level: {},
                addEventListener: function() {{}},
                removeEventListener: function() {{}}
            }}),
            configurable: true
        }})"#,
        charging, charging_time, level
    );
    page.evaluate(js)
        .await
        .map_err(|e| onecrawl_core::Error::Cdp(format!("set_battery_status: {e}")))?;
    Ok(())
}

/// Override Network Information API (navigator.connection).
pub async fn set_connection_info(
    page: &Page,
    effective_type: &str,
    downlink: f64,
    rtt: u32,
) -> Result<()> {
    let js = format!(
        r#"Object.defineProperty(navigator, 'connection', {{
            value: {{
                effectiveType: '{}',
                downlink: {},
                rtt: {},
                saveData: false,
                addEventListener: function() {{}},
                removeEventListener: function() {{}}
            }},
            configurable: true
        }})"#,
        effective_type.replace('\\', "\\\\").replace('\'', "\\'"),
        downlink,
        rtt
    );
    page.evaluate(js)
        .await
        .map_err(|e| onecrawl_core::Error::Cdp(format!("set_connection_info: {e}")))?;
    Ok(())
}

/// Override hardware concurrency (CPU cores).
pub async fn set_hardware_concurrency(page: &Page, cores: u32) -> Result<()> {
    let js = format!(
        "Object.defineProperty(navigator, 'hardwareConcurrency', {{ value: {}, configurable: true }})",
        cores
    );
    page.evaluate(js)
        .await
        .map_err(|e| onecrawl_core::Error::Cdp(format!("set_hardware_concurrency: {e}")))?;
    Ok(())
}

/// Override device memory.
pub async fn set_device_memory(page: &Page, memory_gb: f64) -> Result<()> {
    let js = format!(
        "Object.defineProperty(navigator, 'deviceMemory', {{ value: {}, configurable: true }})",
        memory_gb
    );
    page.evaluate(js)
        .await
        .map_err(|e| onecrawl_core::Error::Cdp(format!("set_device_memory: {e}")))?;
    Ok(())
}

/// Get current navigator properties as JSON.
pub async fn get_navigator_info(page: &Page) -> Result<serde_json::Value> {
    let js = r#"JSON.stringify({
        userAgent: navigator.userAgent,
        platform: navigator.platform,
        language: navigator.language,
        languages: Array.from(navigator.languages),
        hardwareConcurrency: navigator.hardwareConcurrency,
        deviceMemory: navigator.deviceMemory,
        maxTouchPoints: navigator.maxTouchPoints,
        cookieEnabled: navigator.cookieEnabled,
        webdriver: navigator.webdriver,
        vendor: navigator.vendor,
        doNotTrack: navigator.doNotTrack
    })"#;
    let val = page
        .evaluate(js)
        .await
        .map_err(|e| onecrawl_core::Error::Cdp(format!("get_navigator_info: {e}")))?;
    let raw = val
        .into_value::<String>()
        .unwrap_or_else(|_| "{}".to_string());
    let parsed: serde_json::Value = serde_json::from_str(&raw).unwrap_or(serde_json::json!({}));
    Ok(parsed)
}
