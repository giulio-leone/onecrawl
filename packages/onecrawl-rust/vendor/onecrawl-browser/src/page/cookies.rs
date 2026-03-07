//! Cookie operations for Page.

use futures::{stream, StreamExt};

use onecrawl_protocol::cdp::browser_protocol::network::{
    Cookie, CookieParam, DeleteCookiesParams, GetCookiesParams, SetCookiesParams,
};

use crate::error::{CdpError, Result};

use super::{validate_cookie_url, Page};

impl Page {
    /// Returns all cookies that match the tab's current URL.
    pub async fn get_cookies(&self) -> Result<Vec<Cookie>> {
        Ok(self
            .execute(GetCookiesParams::default())
            .await?
            .result
            .cookies)
    }

    /// Set a single cookie
    ///
    /// # Example
    /// ```no_run
    /// # use onecrawl_browser::page::Page;
    /// # use onecrawl_browser::error::Result;
    /// # use onecrawl_protocol::cdp::browser_protocol::network::CookieParam;
    /// # async fn demo(page: Page) -> Result<()> {
    ///     page.set_cookie(CookieParam::new("foo", "bar")).await?;
    ///     # Ok(())
    /// # }
    /// ```
    pub async fn set_cookie(&self, cookie: impl Into<CookieParam>) -> Result<&Self> {
        let mut cookie = cookie.into();
        if cookie.url.is_none() {
            let url = self
                .url()
                .await?
                .ok_or_else(|| CdpError::msg("Page url not found"))?;
            validate_cookie_url(&url)?;
            if url.starts_with("http") {
                cookie.url = Some(url);
            }
        }
        self.execute_void(DeleteCookiesParams::from_cookie(&cookie))
            .await?;
        self.execute_void(SetCookiesParams::new(vec![cookie])).await?;
        Ok(self)
    }

    /// Set all the cookies
    pub async fn set_cookies(&self, mut cookies: Vec<CookieParam>) -> Result<&Self> {
        let url = self
            .url()
            .await?
            .ok_or_else(|| CdpError::msg("Page url not found"))?;
        let is_http = url.starts_with("http");
        if !is_http {
            validate_cookie_url(&url)?;
        }

        for cookie in &mut cookies {
            if let Some(url) = cookie.url.as_ref() {
                validate_cookie_url(url)?;
            } else if is_http {
                cookie.url = Some(url.clone());
            }
        }
        self.delete_cookies_unchecked(cookies.iter().map(DeleteCookiesParams::from_cookie))
            .await?;

        self.execute_void(SetCookiesParams::new(cookies)).await?;
        Ok(self)
    }

    /// Delete a single cookie
    pub async fn delete_cookie(&self, cookie: impl Into<DeleteCookiesParams>) -> Result<&Self> {
        let mut cookie = cookie.into();
        if cookie.url.is_none() {
            let url = self
                .url()
                .await?
                .ok_or_else(|| CdpError::msg("Page url not found"))?;
            if url.starts_with("http") {
                cookie.url = Some(url);
            }
        }
        self.execute_void(cookie).await?;
        Ok(self)
    }

    /// Delete all the cookies
    pub async fn delete_cookies(&self, mut cookies: Vec<DeleteCookiesParams>) -> Result<&Self> {
        let mut url: Option<(String, bool)> = None;
        for cookie in &mut cookies {
            if cookie.url.is_none() {
                if let Some((url, is_http)) = url.as_ref() {
                    if *is_http {
                        cookie.url = Some(url.clone())
                    }
                } else {
                    let page_url = self
                        .url()
                        .await?
                        .ok_or_else(|| CdpError::msg("Page url not found"))?;
                    let is_http = page_url.starts_with("http");
                    if is_http {
                        cookie.url = Some(page_url.clone())
                    }
                    url = Some((page_url, is_http));
                }
            }
        }
        self.delete_cookies_unchecked(cookies.into_iter()).await?;
        Ok(self)
    }

    /// Convenience method that prevents another channel roundtrip to get the
    /// url and validate it
    async fn delete_cookies_unchecked(
        &self,
        cookies: impl Iterator<Item = DeleteCookiesParams>,
    ) -> Result<&Self> {
        // NOTE: the buffer size is arbitrary
        let mut cmds = stream::iter(cookies.into_iter().map(|cookie| self.execute(cookie)))
            .buffer_unordered(5);
        while let Some(resp) = cmds.next().await {
            resp?;
        }
        Ok(self)
    }
}
