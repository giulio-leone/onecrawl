//! CDP protocol enable/disable methods for Page.

use onecrawl_protocol::cdp::{browser_protocol, js_protocol};

use crate::error::Result;

use super::Page;

impl Page {
    /// Enables log domain. Enabled by default.
    ///
    /// Sends the entries collected so far to the client by means of the
    /// entryAdded notification.
    ///
    /// See https://chromedevtools.github.io/devtools-protocol/tot/Log#method-enable
    pub async fn enable_log(&self) -> Result<&Self> {
        self.execute_void(browser_protocol::log::EnableParams::default())
            .await?;
        Ok(self)
    }

    /// Disables log domain
    ///
    /// Prevents further log entries from being reported to the client
    ///
    /// See https://chromedevtools.github.io/devtools-protocol/tot/Log#method-disable
    pub async fn disable_log(&self) -> Result<&Self> {
        self.execute_void(browser_protocol::log::DisableParams::default())
            .await?;
        Ok(self)
    }

    /// Enables runtime domain. Activated by default.
    pub async fn enable_runtime(&self) -> Result<&Self> {
        self.execute_void(js_protocol::runtime::EnableParams::default())
            .await?;
        Ok(self)
    }

    /// Disables runtime domain
    pub async fn disable_runtime(&self) -> Result<&Self> {
        self.execute_void(js_protocol::runtime::DisableParams::default())
            .await?;
        Ok(self)
    }

    /// Enables Debugger. Enabled by default.
    pub async fn enable_debugger(&self) -> Result<&Self> {
        self.execute_void(js_protocol::debugger::EnableParams::default())
            .await?;
        Ok(self)
    }

    /// Disables Debugger.
    pub async fn disable_debugger(&self) -> Result<&Self> {
        self.execute_void(js_protocol::debugger::DisableParams::default())
            .await?;
        Ok(self)
    }

    // Enables DOM agent
    pub async fn enable_dom(&self) -> Result<&Self> {
        self.execute_void(browser_protocol::dom::EnableParams::default())
            .await?;
        Ok(self)
    }

    // Disables DOM agent
    pub async fn disable_dom(&self) -> Result<&Self> {
        self.execute_void(browser_protocol::dom::DisableParams::default())
            .await?;
        Ok(self)
    }

    // Enables the CSS agent
    pub async fn enable_css(&self) -> Result<&Self> {
        self.execute_void(browser_protocol::css::EnableParams::default())
            .await?;
        Ok(self)
    }

    // Disables the CSS agent
    pub async fn disable_css(&self) -> Result<&Self> {
        self.execute_void(browser_protocol::css::DisableParams::default())
            .await?;
        Ok(self)
    }
}
