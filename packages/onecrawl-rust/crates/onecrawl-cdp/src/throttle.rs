//! Network throttling via CDP Network.emulateNetworkConditions.
//!
//! Preset profiles for common network conditions (3G, 4G, WiFi, Offline).

use chromiumoxide::Page;
#[allow(deprecated)]
use chromiumoxide::cdp::browser_protocol::network::{
    ConnectionType, EmulateNetworkConditionsParams,
};
use onecrawl_core::{Error, Result};

/// Preset network profiles for throttling.
#[derive(Debug, Clone, Copy)]
pub enum NetworkProfile {
    Fast3G,
    Slow3G,
    Offline,
    Regular4G,
    WiFi,
    Custom {
        download_kbps: f64,
        upload_kbps: f64,
        latency_ms: f64,
    },
}

impl NetworkProfile {
    /// Human-readable name for the profile.
    pub fn name(&self) -> &'static str {
        match self {
            Self::Fast3G => "Fast 3G",
            Self::Slow3G => "Slow 3G",
            Self::Offline => "Offline",
            Self::Regular4G => "Regular 4G",
            Self::WiFi => "WiFi",
            Self::Custom { .. } => "Custom",
        }
    }
}

/// Convert kbps to bytes/sec for CDP throughput parameters.
fn kbps_to_bytes_per_sec(kbps: f64) -> f64 {
    (kbps * 1000.0) / 8.0
}

/// Apply network throttling with a preset profile.
#[allow(deprecated)]
pub async fn set_network_conditions(page: &Page, profile: NetworkProfile) -> Result<()> {
    let params = match profile {
        NetworkProfile::Fast3G => EmulateNetworkConditionsParams::builder()
            .offline(false)
            .latency(150.0)
            .download_throughput(kbps_to_bytes_per_sec(1600.0))
            .upload_throughput(kbps_to_bytes_per_sec(750.0))
            .connection_type(ConnectionType::Cellular3g)
            .build()
            .map_err(|e| Error::Browser(format!("Fast3G build: {e}")))?,

        NetworkProfile::Slow3G => EmulateNetworkConditionsParams::builder()
            .offline(false)
            .latency(400.0)
            .download_throughput(kbps_to_bytes_per_sec(500.0))
            .upload_throughput(kbps_to_bytes_per_sec(500.0))
            .connection_type(ConnectionType::Cellular3g)
            .build()
            .map_err(|e| Error::Browser(format!("Slow3G build: {e}")))?,

        NetworkProfile::Offline => EmulateNetworkConditionsParams::builder()
            .offline(true)
            .latency(0.0)
            .download_throughput(0.0)
            .upload_throughput(0.0)
            .connection_type(ConnectionType::None)
            .build()
            .map_err(|e| Error::Browser(format!("Offline build: {e}")))?,

        NetworkProfile::Regular4G => EmulateNetworkConditionsParams::builder()
            .offline(false)
            .latency(20.0)
            .download_throughput(kbps_to_bytes_per_sec(4000.0))
            .upload_throughput(kbps_to_bytes_per_sec(3000.0))
            .connection_type(ConnectionType::Cellular4g)
            .build()
            .map_err(|e| Error::Browser(format!("Regular4G build: {e}")))?,

        NetworkProfile::WiFi => EmulateNetworkConditionsParams::builder()
            .offline(false)
            .latency(2.0)
            .download_throughput(kbps_to_bytes_per_sec(30000.0))
            .upload_throughput(kbps_to_bytes_per_sec(15000.0))
            .connection_type(ConnectionType::Wifi)
            .build()
            .map_err(|e| Error::Browser(format!("WiFi build: {e}")))?,

        NetworkProfile::Custom {
            download_kbps,
            upload_kbps,
            latency_ms,
        } => EmulateNetworkConditionsParams::builder()
            .offline(false)
            .latency(latency_ms)
            .download_throughput(kbps_to_bytes_per_sec(download_kbps))
            .upload_throughput(kbps_to_bytes_per_sec(upload_kbps))
            .build()
            .map_err(|e| Error::Browser(format!("Custom profile build: {e}")))?,
    };

    page.execute(params)
        .await
        .map_err(|e| Error::Browser(format!("EmulateNetworkConditions failed: {e}")))?;

    Ok(())
}

/// Clear network throttling (restore full speed).
#[allow(deprecated)]
pub async fn clear_network_conditions(page: &Page) -> Result<()> {
    let params = EmulateNetworkConditionsParams::builder()
        .offline(false)
        .latency(0.0)
        .download_throughput(-1.0)
        .upload_throughput(-1.0)
        .build()
        .map_err(|e| Error::Browser(format!("clear conditions build: {e}")))?;

    page.execute(params)
        .await
        .map_err(|e| Error::Browser(format!("ClearNetworkConditions failed: {e}")))?;

    Ok(())
}

/// Get a description of what a given profile applies.
pub fn describe_profile(profile: &NetworkProfile) -> String {
    match profile {
        NetworkProfile::Fast3G => "Fast 3G: 1.6 Mbps down, 750 Kbps up, 150ms latency".to_string(),
        NetworkProfile::Slow3G => "Slow 3G: 500 Kbps down, 500 Kbps up, 400ms latency".to_string(),
        NetworkProfile::Offline => "Offline: no connectivity".to_string(),
        NetworkProfile::Regular4G => "Regular 4G: 4 Mbps down, 3 Mbps up, 20ms latency".to_string(),
        NetworkProfile::WiFi => "WiFi: 30 Mbps down, 15 Mbps up, 2ms latency".to_string(),
        NetworkProfile::Custom {
            download_kbps,
            upload_kbps,
            latency_ms,
        } => format!(
            "Custom: {download_kbps} Kbps down, {upload_kbps} Kbps up, {latency_ms}ms latency"
        ),
    }
}
