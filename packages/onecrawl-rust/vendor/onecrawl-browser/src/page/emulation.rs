//! Emulation, stealth, and user-agent methods for Page.

use onecrawl_protocol::cdp::browser_protocol::emulation::{
    MediaFeature, SetEmulatedMediaParams, SetGeolocationOverrideParams, SetLocaleOverrideParams,
    SetTimezoneOverrideParams,
};
use onecrawl_protocol::cdp::browser_protocol::network::SetUserAgentOverrideParams;
use onecrawl_protocol::cdp::browser_protocol::page::AddScriptToEvaluateOnNewDocumentParams;

use futures::SinkExt;

use crate::auth::Credentials;
use crate::error::{CdpError, Result};
use crate::handler::target::TargetMessage;

use super::{MediaTypeParams, Page};

impl Page {
    /// Removes the `navigator.webdriver` property
    /// changes permissions, pluggins rendering contexts and the `window.chrome`
    /// property to make it harder to detect the scraper as a bot
    async fn _enable_stealth_mode(&self) -> Result<()> {
        self.hide_webdriver().await?;
        self.hide_permissions().await?;
        self.hide_plugins().await?;
        self.hide_webgl_vendor().await?;
        self.hide_chrome().await?;

        Ok(())
    }

    /// Changes your user_agent, removes the `navigator.webdriver` property
    /// changes permissions, pluggins rendering contexts and the `window.chrome`
    /// property to make it harder to detect the scraper as a bot
    pub async fn enable_stealth_mode(&self) -> Result<()> {
        self._enable_stealth_mode().await?;
        self.set_user_agent("Mozilla/5.0 (Windows NT 11.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/107.0.5296.0 Safari/537.36").await?;

        Ok(())
    }

    /// Changes your user_agent with a custom agent, removes the `navigator.webdriver` property
    /// changes permissions, pluggins rendering contexts and the `window.chrome`
    /// property to make it harder to detect the scraper as a bot
    pub async fn enable_stealth_mode_with_agent(&self, ua: &str) -> Result<()> {
        self._enable_stealth_mode().await?;
        if !ua.is_empty() {
            self.set_user_agent(ua).await?;
        }
        Ok(())
    }

    /// Sets `window.chrome` on frame creation
    async fn hide_chrome(&self) -> Result<(), CdpError> {
        self.execute(AddScriptToEvaluateOnNewDocumentParams {
            source: "window.chrome = { runtime: {} };".to_string(),
            world_name: None,
            include_command_line_api: None,
            run_immediately: None,
        })
        .await?;
        Ok(())
    }

    /// Obfuscates WebGL vendor on frame creation
    async fn hide_webgl_vendor(&self) -> Result<(), CdpError> {
        self
            .execute(AddScriptToEvaluateOnNewDocumentParams {
                source: "
                    const getParameter = WebGLRenderingContext.getParameter;
                    WebGLRenderingContext.prototype.getParameter = function (parameter) {
                        if (parameter === 37445) {
                            return 'Google Inc. (NVIDIA)';
                        }

                        if (parameter === 37446) {
                            return 'ANGLE (NVIDIA, NVIDIA GeForce GTX 1050 Direct3D11 vs_5_0 ps_5_0, D3D11-27.21.14.5671)';
                        }

                        return getParameter(parameter);
                    };
                "
                .to_string(),
                world_name: None,
                include_command_line_api: None,
                run_immediately: None,
            })
            .await?;
        Ok(())
    }

    /// Obfuscates browser plugins on frame creation
    async fn hide_plugins(&self) -> Result<(), CdpError> {
        self.execute(AddScriptToEvaluateOnNewDocumentParams {
            source: "
                    Object.defineProperty(
                        navigator,
                        'plugins',
                        {
                            get: () => [
                                { filename: 'internal-pdf-viewer' },
                                { filename: 'adsfkjlkjhalkh' },
                                { filename: 'internal-nacl-plugin '}
                            ],
                        }
                    );
                "
            .to_string(),
            world_name: None,
            include_command_line_api: None,
            run_immediately: None,
        })
        .await?;
        Ok(())
    }

    /// Obfuscates browser permissions on frame creation
    async fn hide_permissions(&self) -> Result<(), CdpError> {
        self.execute(AddScriptToEvaluateOnNewDocumentParams {
            source: "
                    const originalQuery = window.navigator.permissions.query;
                    window.navigator.permissions.__proto__.query = parameters => {
                        return parameters.name === 'notifications'
                            ? Promise.resolve({ state: Notification.permission })
                            : originalQuery(parameters);
                    }
                "
            .to_string(),
            world_name: None,
            include_command_line_api: None,
            run_immediately: None,
        })
        .await?;
        Ok(())
    }

    /// Removes the `navigator.webdriver` property on frame creation
    async fn hide_webdriver(&self) -> Result<(), CdpError> {
        self.execute(AddScriptToEvaluateOnNewDocumentParams {
            source: "
                    Object.defineProperty(
                        navigator,
                        'webdriver',
                        { get: () => undefined }
                    );
                "
            .to_string(),
            world_name: None,
            include_command_line_api: None,
            run_immediately: None,
        })
        .await?;
        Ok(())
    }

    /// Allows overriding user agent with the given string.
    pub async fn set_user_agent(
        &self,
        params: impl Into<SetUserAgentOverrideParams>,
    ) -> Result<&Self> {
        self.execute_void(params.into()).await?;
        Ok(self)
    }

    /// Returns the user agent of the browser
    pub async fn user_agent(&self) -> Result<String> {
        Ok(self.inner.version().await?.user_agent)
    }

    pub async fn authenticate(&self, credentials: Credentials) -> Result<()> {
        self.inner
            .sender()
            .clone()
            .send(TargetMessage::Authenticate(credentials))
            .await?;

        Ok(())
    }

    /// Emulates the given media type or media feature for CSS media queries
    pub async fn emulate_media_features(&self, features: Vec<MediaFeature>) -> Result<&Self> {
        self.execute_void(SetEmulatedMediaParams::builder().features(features).build())
            .await?;
        Ok(self)
    }

    /// Changes the CSS media type of the page
    // Based on https://pptr.dev/api/puppeteer.page.emulatemediatype
    pub async fn emulate_media_type(
        &self,
        media_type: impl Into<MediaTypeParams>,
    ) -> Result<&Self> {
        self.execute_void(
            SetEmulatedMediaParams::builder()
                .media(media_type.into())
                .build(),
        )
        .await?;
        Ok(self)
    }

    /// Overrides default host system timezone
    pub async fn emulate_timezone(
        &self,
        timezoune_id: impl Into<SetTimezoneOverrideParams>,
    ) -> Result<&Self> {
        self.execute_void(timezoune_id.into()).await?;
        Ok(self)
    }

    /// Overrides default host system locale with the specified one
    pub async fn emulate_locale(
        &self,
        locale: impl Into<SetLocaleOverrideParams>,
    ) -> Result<&Self> {
        self.execute_void(locale.into()).await?;
        Ok(self)
    }

    /// Overrides the Geolocation Position or Error. Omitting any of the parameters emulates position unavailable.
    pub async fn emulate_geolocation(
        &self,
        geolocation: impl Into<SetGeolocationOverrideParams>,
    ) -> Result<&Self> {
        self.execute_void(geolocation.into()).await?;
        Ok(self)
    }
}
