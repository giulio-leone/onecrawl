//! Browser-level cookie and cache operations.

use onecrawl_protocol::cdp::browser_protocol::network::{Cookie, CookieParam};
use onecrawl_protocol::cdp::browser_protocol::storage::{
    ClearCookiesParams, GetCookiesParams, SetCookiesParams,
};

use crate::error::Result;

use super::Browser;

impl Browser {
    /// Clears cookies.
    pub async fn clear_cookies(&self) -> Result<()> {
        self.execute(ClearCookiesParams::default()).await?;
        Ok(())
    }

    /// Returns all browser cookies.
    pub async fn get_cookies(&self) -> Result<Vec<Cookie>> {
        Ok(self
            .execute(GetCookiesParams::default())
            .await?
            .result
            .cookies)
    }

    /// Sets given cookies.
    pub async fn set_cookies(&self, mut cookies: Vec<CookieParam>) -> Result<&Self> {
        for cookie in &mut cookies {
            if let Some(url) = cookie.url.as_ref() {
                crate::page::validate_cookie_url(url)?;
            }
        }

        self.execute(SetCookiesParams::new(cookies)).await?;
        Ok(self)
    }
}
