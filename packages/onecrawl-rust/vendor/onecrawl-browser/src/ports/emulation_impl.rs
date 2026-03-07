use async_trait::async_trait;
use onecrawl_protocol::cdp::browser_protocol::emulation::{
    SetDeviceMetricsOverrideParams, SetEmulatedMediaParams, SetGeolocationOverrideParams,
    SetLocaleOverrideParams, SetTimezoneOverrideParams,
};
use onecrawl_protocol::cdp::browser_protocol::network::SetUserAgentOverrideParams;

use crate::error::Result;
use crate::page::Page;
use super::EmulationPort;

#[async_trait]
impl EmulationPort for Page {
    async fn set_viewport_size(
        &self,
        width: u32,
        height: u32,
        device_scale_factor: f64,
    ) -> Result<()> {
        let params = SetDeviceMetricsOverrideParams::new(
            width as i64,
            height as i64,
            device_scale_factor,
            false,
        );
        self.execute(params).await?;
        Ok(())
    }

    async fn set_user_agent_override(&self, ua: &str) -> Result<()> {
        Page::set_user_agent(self, SetUserAgentOverrideParams::new(ua)).await?;
        Ok(())
    }

    async fn set_geolocation(
        &self,
        latitude: f64,
        longitude: f64,
        accuracy: f64,
    ) -> Result<()> {
        self.emulate_geolocation(
            SetGeolocationOverrideParams::builder()
                .latitude(latitude)
                .longitude(longitude)
                .accuracy(accuracy)
                .build(),
        )
        .await?;
        Ok(())
    }

    async fn set_timezone_override(&self, timezone_id: &str) -> Result<()> {
        self.emulate_timezone(SetTimezoneOverrideParams::new(timezone_id)).await?;
        Ok(())
    }

    async fn set_locale_override(&self, locale: &str) -> Result<()> {
        self.emulate_locale(
            SetLocaleOverrideParams::builder()
                .locale(locale.to_string())
                .build(),
        )
        .await?;
        Ok(())
    }

    async fn set_media_type(&self, media_type: &str) -> Result<()> {
        self.execute(
            SetEmulatedMediaParams::builder()
                .media(media_type.to_string())
                .build(),
        )
        .await?;
        Ok(())
    }
}
