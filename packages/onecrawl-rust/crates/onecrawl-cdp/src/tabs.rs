use chromiumoxide::Page;
use chromiumoxide::browser::Browser as CrBrowser;
use onecrawl_core::{Error, Result};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TabInfo {
    pub index: usize,
    pub url: String,
    pub title: String,
    pub target_id: String,
}

/// Return pages sorted by TargetId so that index assignments are stable
/// across process restarts and reconnections (HashMap iteration order is not
/// deterministic, so we must sort before exposing an index-based API).
async fn sorted_pages(browser: &CrBrowser) -> Result<Vec<Page>> {
    let mut pages = browser
        .pages()
        .await
        .map_err(|e| Error::Cdp(format!("pages() failed: {e}")))?;
    pages.sort_by(|a, b| a.target_id().inner().cmp(b.target_id().inner()));
    Ok(pages)
}

/// List all open tabs.
pub async fn list_tabs(browser: &CrBrowser) -> Result<Vec<TabInfo>> {
    let pages = sorted_pages(browser).await?;
    let mut tabs = Vec::new();
    for (i, page) in pages.iter().enumerate() {
        let url = page.url().await.ok().flatten().unwrap_or_default();
        let title: String = page
            .evaluate("document.title")
            .await
            .ok()
            .and_then(|v| v.into_value().ok())
            .unwrap_or_default();
        let target_id = page.target_id().inner().clone();
        tabs.push(TabInfo {
            index: i,
            url,
            title,
            target_id,
        });
    }
    Ok(tabs)
}

/// Open a new tab and navigate to URL.
pub async fn new_tab(browser: &CrBrowser, url: &str) -> Result<Page> {
    browser
        .new_page(url)
        .await
        .map_err(|e| Error::Cdp(format!("new_tab failed: {e}")))
}

/// Close a tab by index (0-based). The page is consumed.
pub async fn close_tab(browser: &CrBrowser, index: usize) -> Result<()> {
    let pages = sorted_pages(browser).await?;
    if index >= pages.len() {
        return Err(Error::Cdp(format!(
            "Tab index {} out of bounds ({})",
            index,
            pages.len()
        )));
    }
    let page = pages.into_iter().nth(index).unwrap();
    page.close()
        .await
        .map_err(|e| Error::Cdp(format!("close_tab failed: {e}")))?;
    Ok(())
}

/// Get a page reference by index.
pub async fn get_tab(browser: &CrBrowser, index: usize) -> Result<Page> {
    let pages = sorted_pages(browser).await?;
    if index >= pages.len() {
        return Err(Error::Cdp(format!(
            "Tab index {} out of bounds ({})",
            index,
            pages.len()
        )));
    }
    Ok(pages.into_iter().nth(index).unwrap())
}

/// Get the count of open tabs.
pub async fn tab_count(browser: &CrBrowser) -> Result<usize> {
    let pages = browser
        .pages()
        .await
        .map_err(|e| Error::Cdp(format!("tab_count failed: {e}")))?;
    Ok(pages.len())
}
