//! Dependency-injection factory for browser automation.
//!
//! Returns trait objects (`Box<dyn BrowserPort>`) so consumers depend on
//! the port abstraction, not the concrete `Browser` type.

use crate::browser::{BrowserConfig};
use crate::error::Result;
use crate::handler::Handler;
use crate::ports::BrowserPort;
use crate::Browser;

/// Launch a new browser with the given configuration and return a
/// `BrowserPort` trait object plus the event-loop `Handler`.
pub async fn create_browser(
    config: BrowserConfig,
) -> Result<(Box<dyn BrowserPort>, Handler)> {
    let (browser, handler) = Browser::launch(config).await?;
    Ok((Box::new(browser), handler))
}

/// Connect to an already-running browser at the given WebSocket (or HTTP) URL
/// and return a `BrowserPort` trait object plus the event-loop `Handler`.
pub async fn connect_browser(
    url: impl Into<String>,
) -> Result<(Box<dyn BrowserPort>, Handler)> {
    let (browser, handler) = Browser::connect(url).await?;
    Ok((Box::new(browser), handler))
}
