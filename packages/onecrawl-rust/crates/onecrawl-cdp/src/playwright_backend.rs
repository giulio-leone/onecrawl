//! Optional Playwright backend for cross-browser support.
//! Enable with `cargo build -p onecrawl-cdp --features playwright`
//!
//! Uses the `playwright-rs` crate (0.8.x) which wraps Microsoft Playwright
//! via a Node.js driver, providing Chromium, Firefox, and WebKit automation.

use playwright::api::LaunchOptions;
use playwright::{Browser, Page, Playwright};
use std::sync::Arc;
use tokio::sync::Mutex;

/// Which browser engine to launch.
pub enum BrowserEngine {
    Chromium,
    Firefox,
    Webkit,
}

/// A browser session backed by playwright-rs.
pub struct PlaywrightSession {
    _playwright: Playwright,
    browser: Browser,
    page: Arc<Mutex<Page>>,
}

impl PlaywrightSession {
    /// Launch a new Playwright-backed browser session.
    ///
    /// Requires the Playwright Node.js driver and matching browser binaries
    /// to be installed (`npx playwright install`).
    pub async fn launch(
        engine: BrowserEngine,
        headless: bool,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        let playwright = Playwright::launch().await?;

        let options = LaunchOptions::new().headless(headless);

        let browser = match engine {
            BrowserEngine::Chromium => playwright.chromium().launch_with_options(options).await?,
            BrowserEngine::Firefox => playwright.firefox().launch_with_options(options).await?,
            BrowserEngine::Webkit => playwright.webkit().launch_with_options(options).await?,
        };

        let page = browser.new_page().await?;

        Ok(Self {
            _playwright: playwright,
            browser,
            page: Arc::new(Mutex::new(page)),
        })
    }

    /// Navigate the page to a URL.
    pub async fn navigate(&self, url: &str) -> Result<(), Box<dyn std::error::Error>> {
        let page = self.page.lock().await;
        page.goto(url, None).await?;
        Ok(())
    }

    /// Take a full-page screenshot and return the raw bytes.
    pub async fn screenshot(&self) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
        let page = self.page.lock().await;
        let bytes = page.screenshot(None).await?;
        Ok(bytes)
    }

    /// Get the full HTML content of the page.
    pub async fn content(&self) -> Result<String, Box<dyn std::error::Error>> {
        let page = self.page.lock().await;
        let html = page.content().await?;
        Ok(html)
    }

    /// Evaluate a JavaScript expression and return the stringified result.
    pub async fn evaluate(&self, expression: &str) -> Result<String, Box<dyn std::error::Error>> {
        let page = self.page.lock().await;
        let result = page.evaluate_value(expression).await?;
        Ok(result)
    }

    /// Click an element matching the given CSS selector.
    pub async fn click(&self, selector: &str) -> Result<(), Box<dyn std::error::Error>> {
        let page = self.page.lock().await;
        let locator = page.locator(selector).await;
        locator.click(None).await?;
        Ok(())
    }

    /// Close the browser and release resources.
    pub async fn close(self) -> Result<(), Box<dyn std::error::Error>> {
        self.browser.close().await?;
        self._playwright.shutdown().await?;
        Ok(())
    }
}
