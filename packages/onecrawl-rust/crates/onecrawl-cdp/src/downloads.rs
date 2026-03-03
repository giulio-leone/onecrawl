use chromiumoxide::Page;
use onecrawl_core::{Error, Result};
use serde::{Deserialize, Serialize};
use std::path::Path;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DownloadInfo {
    pub url: String,
    pub filename: String,
    pub size_bytes: Option<u64>,
    pub mime_type: Option<String>,
    pub status: String,
    pub timestamp: f64,
}

/// Set the download directory and enable download interception via JS.
pub async fn set_download_path(page: &Page, download_path: &Path) -> Result<()> {
    let path_str = download_path.to_string_lossy().replace('\'', "\\'");
    let js = format!(
        r#"
        window.__onecrawl_downloads = window.__onecrawl_downloads || {{
            path: '{path}',
            entries: [],
            interceptEnabled: false
        }};
        window.__onecrawl_downloads.path = '{path}';

        if (!window.__onecrawl_downloads.interceptEnabled) {{
            window.__onecrawl_downloads.interceptEnabled = true;
            document.addEventListener('click', function(e) {{
                const link = e.target.closest(
                    'a[download], a[href$=".pdf"], a[href$=".zip"], a[href$=".csv"], a[href$=".xlsx"]'
                );
                if (link) {{
                    window.__onecrawl_downloads.entries.push({{
                        url: link.href,
                        filename: link.download || link.href.split('/').pop() || 'unknown',
                        size_bytes: null,
                        mime_type: null,
                        status: 'pending',
                        timestamp: Date.now()
                    }});
                }}
            }}, true);
        }}
        true
    "#,
        path = path_str
    );
    page.evaluate(js)
        .await
        .map_err(|e| Error::Browser(format!("set_download_path failed: {e}")))?;
    Ok(())
}

/// Get download history tracked by JS interception.
pub async fn get_downloads(page: &Page) -> Result<Vec<DownloadInfo>> {
    let val = page
        .evaluate("JSON.stringify(window.__onecrawl_downloads?.entries || [])")
        .await
        .map_err(|e| Error::Browser(format!("get_downloads failed: {e}")))?;
    let json_str: String = val
        .into_value()
        .unwrap_or_else(|_| "[]".to_string());
    let downloads: Vec<DownloadInfo> =
        serde_json::from_str(&json_str).unwrap_or_default();
    Ok(downloads)
}

/// Download a file by URL using fetch and return as base64.
pub async fn download_file(page: &Page, url: &str) -> Result<String> {
    let escaped = url.replace('\'', "\\'");
    let js = format!(
        r#"
        (async () => {{
            const resp = await fetch('{url}');
            const blob = await resp.blob();
            return new Promise((resolve) => {{
                const reader = new FileReader();
                reader.onload = () => resolve(reader.result.split(',')[1]);
                reader.readAsDataURL(blob);
            }});
        }})()
    "#,
        url = escaped
    );
    let val = page
        .evaluate(js)
        .await
        .map_err(|e| Error::Browser(format!("download_file failed: {e}")))?;
    let b64: String = val.into_value().unwrap_or_default();
    Ok(b64)
}

/// Wait for a download to appear (polls download entries).
pub async fn wait_for_download(page: &Page, timeout_ms: u64) -> Result<Option<DownloadInfo>> {
    let start = std::time::Instant::now();
    loop {
        let downloads = get_downloads(page).await?;
        if let Some(last) = downloads.last() {
            return Ok(Some(last.clone()));
        }
        if start.elapsed().as_millis() as u64 > timeout_ms {
            return Ok(None);
        }
        tokio::time::sleep(std::time::Duration::from_millis(250)).await;
    }
}

/// Clear download history.
pub async fn clear_downloads(page: &Page) -> Result<()> {
    page.evaluate("if (window.__onecrawl_downloads) { window.__onecrawl_downloads.entries = []; }")
        .await
        .map_err(|e| Error::Browser(format!("clear_downloads failed: {e}")))?;
    Ok(())
}
