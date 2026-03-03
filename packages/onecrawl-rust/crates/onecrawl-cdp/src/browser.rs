use chromiumoxide::browser::{Browser, BrowserConfig};
use futures::StreamExt;
use onecrawl_core::{Error, Result};

/// A managed browser session with CDP connection.
pub struct BrowserSession {
    browser: Browser,
    _handler_task: tokio::task::JoinHandle<()>,
}

impl BrowserSession {
    /// Launch a new headless browser.
    pub async fn launch_headless() -> Result<Self> {
        Self::launch_with_config(
            BrowserConfig::builder()
                .build()
                .map_err(|e| Error::Browser(format!("config error: {e}")))?,
            false,
        )
        .await
    }

    /// Launch a new headed (visible) browser.
    pub async fn launch_headed() -> Result<Self> {
        Self::launch_with_config(
            BrowserConfig::builder()
                .with_head()
                .build()
                .map_err(|e| Error::Browser(format!("config error: {e}")))?,
            true,
        )
        .await
    }

    /// Launch with custom config.
    async fn launch_with_config(config: BrowserConfig, _headed: bool) -> Result<Self> {
        let (browser, mut handler) = Browser::launch(config)
            .await
            .map_err(|e| Error::Browser(format!("launch failed: {e}")))?;

        let handler_task = tokio::spawn(async move {
            while let Some(h) = handler.next().await {
                if h.is_err() {
                    break;
                }
            }
        });

        Ok(Self {
            browser,
            _handler_task: handler_task,
        })
    }

    /// Connect to an existing browser via CDP WebSocket URL.
    pub async fn connect(ws_url: &str) -> Result<Self> {
        let (browser, mut handler) = Browser::connect(ws_url)
            .await
            .map_err(|e| Error::Browser(format!("connect failed: {e}")))?;

        let handler_task = tokio::spawn(async move {
            while let Some(h) = handler.next().await {
                if h.is_err() {
                    break;
                }
            }
        });

        Ok(Self {
            browser,
            _handler_task: handler_task,
        })
    }

    /// Get the inner browser handle.
    pub fn browser(&self) -> &Browser {
        &self.browser
    }

    /// Get the CDP WebSocket URL for this browser session.
    pub fn ws_url(&self) -> &str {
        self.browser.websocket_address()
    }

    /// Create a new page/tab.
    pub async fn new_page(&self, url: &str) -> Result<chromiumoxide::Page> {
        self.browser
            .new_page(url)
            .await
            .map_err(|e| Error::Browser(format!("new page failed: {e}")))
    }

    /// Close the browser.
    pub async fn close(mut self) -> Result<()> {
        self.browser
            .close()
            .await
            .map_err(|e| Error::Browser(format!("close failed: {e}")))?;
        Ok(())
    }
}
