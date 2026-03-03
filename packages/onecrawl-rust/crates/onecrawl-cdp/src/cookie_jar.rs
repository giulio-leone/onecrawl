//! Persistent cookie storage with import/export.

use chromiumoxide::Page;
use onecrawl_core::{Error, Result};
use serde::{Deserialize, Serialize};
use std::path::Path;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CookieJar {
    pub cookies: Vec<StoredCookie>,
    pub domain: Option<String>,
    pub exported_at: String,
    pub version: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StoredCookie {
    pub name: String,
    pub value: String,
    pub domain: String,
    pub path: String,
    pub expires: f64,
    pub http_only: bool,
    pub secure: bool,
    pub same_site: Option<String>,
}

/// Export all cookies from page to a CookieJar.
pub async fn export_cookies(page: &Page) -> Result<CookieJar> {
    let resp = page
        .execute(chromiumoxide::cdp::browser_protocol::network::GetCookiesParams::default())
        .await
        .map_err(|e| Error::Cdp(format!("export_cookies failed: {e}")))?;

    let cookies: Vec<StoredCookie> = resp
        .result
        .cookies
        .iter()
        .map(|c| StoredCookie {
            name: c.name.clone(),
            value: c.value.clone(),
            domain: c.domain.clone(),
            path: c.path.clone(),
            expires: c.expires,
            http_only: c.http_only,
            secure: c.secure,
            same_site: c.same_site.as_ref().map(|s| format!("{s:?}")),
        })
        .collect();

    let ts = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();

    Ok(CookieJar {
        cookies,
        domain: None,
        exported_at: format!("{ts}"),
        version: "1.0".into(),
    })
}

/// Import cookies from a CookieJar into the page.
pub async fn import_cookies(page: &Page, jar: &CookieJar) -> Result<usize> {
    let mut count = 0;
    for cookie in &jar.cookies {
        let params = chromiumoxide::cdp::browser_protocol::network::SetCookieParams::builder()
            .name(&cookie.name)
            .value(&cookie.value)
            .domain(&cookie.domain)
            .path(&cookie.path)
            .http_only(cookie.http_only)
            .secure(cookie.secure)
            .build()
            .map_err(|e| Error::Cdp(format!("SetCookieParams build failed: {e}")))?;
        page.execute(params)
            .await
            .map_err(|e| Error::Cdp(format!("import cookie '{}' failed: {e}", cookie.name)))?;
        count += 1;
    }
    Ok(count)
}

/// Save cookie jar to a JSON file.
pub async fn save_cookies_to_file(page: &Page, path: &Path) -> Result<usize> {
    let jar = export_cookies(page).await?;
    let json = serde_json::to_string_pretty(&jar)
        .map_err(|e| Error::Cdp(format!("serialize cookies failed: {e}")))?;
    std::fs::write(path, json)
        .map_err(|e| Error::Cdp(format!("write cookies file failed: {e}")))?;
    Ok(jar.cookies.len())
}

/// Load cookie jar from a JSON file and import.
pub async fn load_cookies_from_file(page: &Page, path: &Path) -> Result<usize> {
    let json = std::fs::read_to_string(path)
        .map_err(|e| Error::Cdp(format!("read cookies file failed: {e}")))?;
    let jar: CookieJar = serde_json::from_str(&json)
        .map_err(|e| Error::Cdp(format!("parse cookies file failed: {e}")))?;
    import_cookies(page, &jar).await
}

/// Merge cookies from file into current page (additive, doesn't delete existing).
pub async fn merge_cookies_from_file(page: &Page, path: &Path) -> Result<usize> {
    load_cookies_from_file(page, path).await
}

/// Clear all cookies from the page.
pub async fn clear_all_cookies(page: &Page) -> Result<()> {
    page.execute(
        chromiumoxide::cdp::browser_protocol::network::ClearBrowserCookiesParams::default(),
    )
    .await
    .map_err(|e| Error::Cdp(format!("clear_all_cookies failed: {e}")))?;
    Ok(())
}
