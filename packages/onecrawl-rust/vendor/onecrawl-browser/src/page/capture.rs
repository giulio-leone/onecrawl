//! Screenshot and PDF capture methods for Page.

use std::path::Path;

use onecrawl_protocol::cdp::browser_protocol::page::PrintToPdfParams;

use crate::error::Result;
use crate::utils;

use super::{Page, ScreenshotParams};

impl Page {
    /// Take a screenshot of the current page
    pub async fn screenshot(&self, params: impl Into<ScreenshotParams>) -> Result<Vec<u8>> {
        self.inner.screenshot(params).await
    }

    /// Save a screenshot of the page
    ///
    /// # Example save a png file of a website
    ///
    /// ```no_run
    /// # use onecrawl_browser::page::{Page, ScreenshotParams};
    /// # use onecrawl_browser::error::Result;
    /// # use onecrawl_protocol::cdp::browser_protocol::page::CaptureScreenshotFormat;
    /// # async fn demo(page: Page) -> Result<()> {
    ///         page.goto("http://example.com")
    ///             .await?
    ///             .save_screenshot(
    ///             ScreenshotParams::builder()
    ///                 .format(CaptureScreenshotFormat::Png)
    ///                 .full_page(true)
    ///                 .omit_background(true)
    ///                 .build(),
    ///             "example.png",
    ///             )
    ///             .await?;
    ///     # Ok(())
    /// # }
    /// ```
    pub async fn save_screenshot(
        &self,
        params: impl Into<ScreenshotParams>,
        output: impl AsRef<Path>,
    ) -> Result<Vec<u8>> {
        let img = self.screenshot(params).await?;
        utils::write(output.as_ref(), &img).await?;
        Ok(img)
    }

    /// Print the current page as pdf.
    ///
    /// See [`PrintToPdfParams`]
    ///
    /// # Note Generating a pdf is currently only supported in Chrome headless.
    pub async fn pdf(&self, params: PrintToPdfParams) -> Result<Vec<u8>> {
        let res = self.execute(params).await?;
        Ok(utils::base64::decode(&res.data)?)
    }

    /// Save the current page as pdf as file to the `output` path and return the
    /// pdf contents.
    ///
    /// # Note Generating a pdf is currently only supported in Chrome headless.
    pub async fn save_pdf(
        &self,
        opts: PrintToPdfParams,
        output: impl AsRef<Path>,
    ) -> Result<Vec<u8>> {
        let pdf = self.pdf(opts).await?;
        utils::write(output.as_ref(), &pdf).await?;
        Ok(pdf)
    }
}
