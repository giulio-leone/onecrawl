use async_trait::async_trait;
use std::collections::HashMap;

use onecrawl_protocol::cdp::browser_protocol::fetch::{
    EnableParams as FetchEnableParams, RequestPattern,
};
use onecrawl_protocol::cdp::browser_protocol::network::{
    DeleteCookiesParams, Headers, SetExtraHttpHeadersParams,
};

use crate::auth::Credentials;
use crate::error::Result;
use crate::page::Page;
use super::{CookieInfo, NetworkPort};

#[async_trait]
impl NetworkPort for Page {
    async fn set_extra_headers(&self, headers: HashMap<String, String>) -> Result<()> {
        let json_headers = serde_json::to_value(&headers)?;
        let params = SetExtraHttpHeadersParams::new(Headers::new(json_headers));
        self.execute(params).await?;
        Ok(())
    }

    async fn set_request_interception(&self, patterns: &[String]) -> Result<()> {
        let mut builder = FetchEnableParams::builder();
        for pattern in patterns {
            builder = builder.pattern(
                RequestPattern::builder()
                    .url_pattern(pattern.clone())
                    .build(),
            );
        }
        self.execute(builder.build()).await?;
        Ok(())
    }

    async fn set_user_agent(&self, ua: &str) -> Result<()> {
        Page::set_user_agent(self, ua).await?;
        Ok(())
    }

    async fn authenticate(&self, username: &str, password: &str) -> Result<()> {
        Page::authenticate(
            self,
            Credentials {
                username: username.to_string(),
                password: password.to_string(),
            },
        )
        .await
    }

    async fn get_cookies(&self) -> Result<Vec<CookieInfo>> {
        let cookies = Page::get_cookies(self).await?;
        Ok(cookies
            .into_iter()
            .map(|c| CookieInfo {
                name: c.name,
                value: c.value,
                domain: c.domain,
                path: c.path,
                expires: Some(c.expires),
                http_only: c.http_only,
                secure: c.secure,
                same_site: c.same_site.map(|s| s.as_ref().to_string()),
            })
            .collect())
    }

    async fn set_cookie(&self, name: &str, value: &str, domain: &str, path: &str) -> Result<()> {
        use onecrawl_protocol::cdp::browser_protocol::network::CookieParam;
        let mut cookie = CookieParam::new(name, value);
        cookie.domain = Some(domain.to_string());
        cookie.path = Some(path.to_string());
        Page::set_cookie(self, cookie).await?;
        Ok(())
    }

    async fn delete_cookies_by_name(&self, name: &str) -> Result<()> {
        self.delete_cookie(DeleteCookiesParams::new(name)).await?;
        Ok(())
    }

    async fn clear_cookies(&self) -> Result<()> {
        let cookies = Page::get_cookies(self).await?;
        if !cookies.is_empty() {
            let delete_params: Vec<DeleteCookiesParams> = cookies
                .into_iter()
                .map(|c| {
                    let mut params = DeleteCookiesParams::new(&c.name);
                    params.domain = Some(c.domain);
                    params
                })
                .collect();
            self.delete_cookies(delete_params).await?;
        }
        Ok(())
    }

    async fn enable_stealth(&self) -> Result<()> {
        self.enable_stealth_mode().await
    }
}
