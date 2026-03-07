//! Cookie management via CDP Network domain.

use onecrawl_browser::Page;
use onecrawl_core::{Error, Result};
use serde::{Deserialize, Serialize};

/// A browser cookie.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Cookie {
    pub name: String,
    pub value: String,
    pub domain: String,
    pub path: String,
    pub expires: f64,
    pub http_only: bool,
    pub secure: bool,
    #[serde(default)]
    pub same_site: String,
}

/// Parameters for setting a cookie.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SetCookieParams {
    pub name: String,
    pub value: String,
    #[serde(default)]
    pub domain: Option<String>,
    #[serde(default)]
    pub path: Option<String>,
    #[serde(default)]
    pub expires: Option<f64>,
    #[serde(default)]
    pub http_only: Option<bool>,
    #[serde(default)]
    pub secure: Option<bool>,
    #[serde(default)]
    pub same_site: Option<String>,
    #[serde(default)]
    pub url: Option<String>,
}

/// Get all cookies for the current page.
pub async fn get_cookies(page: &Page) -> Result<Vec<Cookie>> {
    let val = page
        .evaluate(
            r#"
            (async () => {
                const cookies = document.cookie.split(';').map(c => {
                    const [name, ...rest] = c.trim().split('=');
                    return { name: name || '', value: rest.join('=') || '',
                             domain: location.hostname, path: '/',
                             expires: -1, httpOnly: false, secure: location.protocol === 'https:',
                             sameSite: '' };
                }).filter(c => c.name);
                return JSON.stringify(cookies);
            })()
            "#,
        )
        .await
        .map_err(|e| Error::Cdp(format!("get_cookies eval failed: {e}")))?
        .into_value::<serde_json::Value>()
        .map_err(|e| Error::Cdp(format!("get_cookies parse failed: {e}")))?;

    let cookies_str = match val {
        serde_json::Value::String(s) => s,
        other => other.to_string(),
    };

    let cookies: Vec<Cookie> = serde_json::from_str(&cookies_str).unwrap_or_default();
    Ok(cookies)
}

/// Get all cookies via CDP Network.getCookies (includes httpOnly cookies).
pub async fn get_all_cookies(page: &Page) -> Result<Vec<Cookie>> {
    let val = page
        .execute(onecrawl_browser::cdp::browser_protocol::network::GetCookiesParams::default())
        .await
        .map_err(|e| Error::Cdp(format!("Network.getCookies failed: {e}")))?;

    let cookies: Vec<Cookie> = val
        .result
        .cookies
        .into_iter()
        .map(|c| Cookie {
            name: c.name,
            value: c.value,
            domain: c.domain,
            path: c.path,
            expires: c.expires,
            http_only: c.http_only,
            secure: c.secure,
            same_site: c.same_site.map(|s| format!("{s:?}")).unwrap_or_default(),
        })
        .collect();

    Ok(cookies)
}

/// Set a cookie via CDP Network.setCookie.
pub async fn set_cookie(page: &Page, params: &SetCookieParams) -> Result<()> {
    use onecrawl_browser::cdp::browser_protocol::network::TimeSinceEpoch;

    let mut builder = onecrawl_browser::cdp::browser_protocol::network::SetCookieParams::builder()
        .name(&params.name)
        .value(&params.value);

    if let Some(ref domain) = params.domain {
        builder = builder.domain(domain);
    }
    if let Some(ref path) = params.path {
        builder = builder.path(path);
    }
    if let Some(expires) = params.expires {
        builder = builder.expires(TimeSinceEpoch::new(expires));
    }
    if let Some(http_only) = params.http_only {
        builder = builder.http_only(http_only);
    }
    if let Some(secure) = params.secure {
        builder = builder.secure(secure);
    }
    if let Some(ref url) = params.url {
        builder = builder.url(url);
    }

    let cdp_params = builder
        .build()
        .map_err(|e| Error::Cdp(format!("SetCookieParams build failed: {e}")))?;

    page.execute(cdp_params)
        .await
        .map_err(|e| Error::Cdp(format!("Network.setCookie failed: {e}")))?;

    Ok(())
}

/// Delete cookies matching the given name (and optional domain/path).
pub async fn delete_cookies(
    page: &Page,
    name: &str,
    domain: Option<&str>,
    path: Option<&str>,
) -> Result<()> {
    let mut builder =
        onecrawl_browser::cdp::browser_protocol::network::DeleteCookiesParams::builder().name(name);

    if let Some(d) = domain {
        builder = builder.domain(d);
    }
    if let Some(p) = path {
        builder = builder.path(p);
    }

    let cdp_params = builder
        .build()
        .map_err(|e| Error::Cdp(format!("DeleteCookiesParams build failed: {e}")))?;

    page.execute(cdp_params)
        .await
        .map_err(|e| Error::Cdp(format!("Network.deleteCookies failed: {e}")))?;

    Ok(())
}

/// Clear all cookies.
pub async fn clear_cookies(page: &Page) -> Result<()> {
    page.execute(
        onecrawl_browser::cdp::browser_protocol::network::ClearBrowserCookiesParams::default(),
    )
    .await
    .map_err(|e| Error::Cdp(format!("Network.clearBrowserCookies failed: {e}")))?;
    Ok(())
}
