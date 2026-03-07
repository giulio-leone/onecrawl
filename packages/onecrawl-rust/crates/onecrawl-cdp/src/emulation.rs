//! Device and viewport emulation via CDP Emulation domain.

use onecrawl_browser::Page;
use onecrawl_core::{Error, Result};
use serde::{Deserialize, Serialize};

/// Viewport configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Viewport {
    pub width: u32,
    pub height: u32,
    pub device_scale_factor: f64,
    pub is_mobile: bool,
    pub has_touch: bool,
}

impl Default for Viewport {
    fn default() -> Self {
        Self {
            width: 1280,
            height: 720,
            device_scale_factor: 1.0,
            is_mobile: false,
            has_touch: false,
        }
    }
}

/// Common device presets.
impl Viewport {
    pub fn desktop() -> Self {
        Self::default()
    }

    pub fn iphone_14() -> Self {
        Self {
            width: 390,
            height: 844,
            device_scale_factor: 3.0,
            is_mobile: true,
            has_touch: true,
        }
    }

    pub fn ipad() -> Self {
        Self {
            width: 810,
            height: 1080,
            device_scale_factor: 2.0,
            is_mobile: true,
            has_touch: true,
        }
    }

    pub fn pixel_7() -> Self {
        Self {
            width: 412,
            height: 915,
            device_scale_factor: 2.625,
            is_mobile: true,
            has_touch: true,
        }
    }
}

/// Set the viewport/device emulation.
pub async fn set_viewport(page: &Page, viewport: &Viewport) -> Result<()> {
    let params =
        onecrawl_browser::cdp::browser_protocol::emulation::SetDeviceMetricsOverrideParams::builder()
            .width(viewport.width as i64)
            .height(viewport.height as i64)
            .device_scale_factor(viewport.device_scale_factor)
            .mobile(viewport.is_mobile)
            .build()
            .map_err(|e| Error::Cdp(format!("SetDeviceMetricsOverride build: {e}")))?;

    page.execute(params)
        .await
        .map_err(|e| Error::Cdp(format!("SetDeviceMetricsOverride failed: {e}")))?;

    if viewport.has_touch {
        let touch_params =
            onecrawl_browser::cdp::browser_protocol::emulation::SetTouchEmulationEnabledParams::new(
                true,
            );
        page.execute(touch_params)
            .await
            .map_err(|e| Error::Cdp(format!("SetTouchEmulationEnabled failed: {e}")))?;
    }

    Ok(())
}

/// Clear viewport override (revert to browser defaults).
pub async fn clear_viewport(page: &Page) -> Result<()> {
    page.execute(
        onecrawl_browser::cdp::browser_protocol::emulation::ClearDeviceMetricsOverrideParams::default(
        ),
    )
    .await
    .map_err(|e| Error::Cdp(format!("ClearDeviceMetricsOverride failed: {e}")))?;
    Ok(())
}

/// Set user agent override with optional Accept-Language header.
pub async fn set_user_agent(page: &Page, user_agent: &str) -> Result<()> {
    set_user_agent_with_lang(page, user_agent, None).await
}

/// Set user agent and Accept-Language header together.
pub async fn set_user_agent_with_lang(
    page: &Page,
    user_agent: &str,
    accept_language: Option<&str>,
) -> Result<()> {
    let mut params =
        onecrawl_browser::cdp::browser_protocol::emulation::SetUserAgentOverrideParams::new(
            user_agent,
        );
    params.accept_language = accept_language.map(|s| s.to_string());
    page.execute(params)
        .await
        .map_err(|e| Error::Cdp(format!("SetUserAgentOverride failed: {e}")))?;
    Ok(())
}

/// Set geolocation override.
pub async fn set_geolocation(
    page: &Page,
    latitude: f64,
    longitude: f64,
    accuracy: f64,
) -> Result<()> {
    let params =
        onecrawl_browser::cdp::browser_protocol::emulation::SetGeolocationOverrideParams::builder()
            .latitude(latitude)
            .longitude(longitude)
            .accuracy(accuracy)
            .build();

    page.execute(params)
        .await
        .map_err(|e| Error::Cdp(format!("SetGeolocationOverride failed: {e}")))?;
    Ok(())
}

/// Emulate a specific timezone.
pub async fn set_timezone(page: &Page, timezone_id: &str) -> Result<()> {
    let js = format!(
        "Intl.DateTimeFormat().resolvedOptions().timeZone = '{}'",
        timezone_id.replace('\\', "\\\\").replace('\'', "\\'")
    );
    page.evaluate(js)
        .await
        .map_err(|e| Error::Cdp(format!("set_timezone failed: {e}")))?;
    Ok(())
}

/// Emulate color scheme (dark/light).
pub async fn set_color_scheme(page: &Page, scheme: &str) -> Result<()> {
    use onecrawl_browser::cdp::browser_protocol::emulation::{MediaFeature, SetEmulatedMediaParams};
    let features = vec![MediaFeature {
        name: "prefers-color-scheme".to_string(),
        value: scheme.to_string(),
    }];
    let params: SetEmulatedMediaParams =
        SetEmulatedMediaParams::builder().features(features).build();
    page.execute(params)
        .await
        .map_err(|e| Error::Cdp(format!("SetEmulatedMedia failed: {e}")))?;
    Ok(())
}
