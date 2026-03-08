//! Navigation methods for Page.

use futures::SinkExt;

use onecrawl_protocol::cdp::browser_protocol::page::*;

use crate::error::{CdpError, Result};
use crate::ArcHttpRequest;

use super::Page;

impl Page {
    /// This resolves once the navigation finished and the page is loaded.
    ///
    /// This is necessary after an interaction with the page that may trigger a
    /// navigation (`click`, `press_key`) in order to wait until the new browser
    /// page is loaded
    pub async fn wait_for_navigation_response(&self) -> Result<ArcHttpRequest> {
        self.inner.wait_for_navigation()?.await
    }

    /// Same as `wait_for_navigation_response` but returns `Self` instead
    pub async fn wait_for_navigation(&self) -> Result<&Self> {
        self.inner.wait_for_navigation()?.await?;
        Ok(self)
    }

    /// Navigate directly to the given URL.
    ///
    /// This resolves directly after the requested URL is fully loaded.
    pub async fn goto(&self, params: impl Into<NavigateParams>) -> Result<&Self> {
        let res = self.execute(params.into()).await?;
        if let Some(err) = res.result.error_text {
            return Err(CdpError::ChromeMessage(err));
        }

        Ok(self)
    }

    /// Returns the current url of the page
    pub async fn url(&self) -> Result<Option<String>> {
        let (tx, rx) = futures::channel::oneshot::channel();
        self.inner
            .sender()
            .clone()
            .send(crate::handler::target::TargetMessage::Url(
                crate::handler::target::GetUrl::new(tx),
            ))
            .await?;
        Ok(rx.await?)
    }

    /// Activates (focuses) the target.
    pub async fn activate(&self) -> Result<&Self> {
        self.inner.activate().await?;
        Ok(self)
    }

    /// Brings page to front (activates tab)
    pub async fn bring_to_front(&self) -> Result<&Self> {
        self.execute_void(BringToFrontParams::default()).await?;
        Ok(self)
    }

    /// Tries to close page, running its beforeunload hooks, if any.
    /// Calls Page.close with [`CloseParams`]
    pub async fn close(self) -> Result<()> {
        self.execute(CloseParams::default()).await?;
        Ok(())
    }

    /// Reloads given page
    ///
    /// To reload ignoring cache run:
    /// ```no_run
    /// # use onecrawl_browser::page::Page;
    /// # use onecrawl_browser::error::Result;
    /// # use onecrawl_protocol::cdp::browser_protocol::page::ReloadParams;
    /// # async fn demo(page: Page) -> Result<()> {
    ///     page.execute(ReloadParams::builder().ignore_cache(true).build()).await?;
    ///     page.wait_for_navigation().await?;
    ///     # Ok(())
    /// # }
    /// ```
    pub async fn reload(&self) -> Result<&Self> {
        self.execute(ReloadParams::default()).await?;
        self.wait_for_navigation().await
    }
}
