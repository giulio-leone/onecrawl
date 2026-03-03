use chromiumoxide::browser::Browser as CrBrowser;
use chromiumoxide::Page;
use onecrawl_core::{Error, Result};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TabInfo {
    pub index: usize,
    pub url: String,
    pub title: String,
    pub target_id: String,
}

/// List all open tabs.
pub async fn list_tabs(browser: &CrBrowser) -> Result<Vec<TabInfo>> {
    let pages = browser
        .pages()
        .await
        .map_err(|e| Error::Browser(format!("list_tabs failed: {e}")))?;
    let mut tabs = Vec::new();
    for (i, page) in pages.iter().enumerate() {
        let url = page
            .url()
            .await
            .ok()
            .flatten()
            .unwrap_or_default();
        let title: String = page
            .evaluate("document.title")
            .await
            .ok()
            .and_then(|v| v.into_value().ok())
            .unwrap_or_default();
        let target_id = format!("{:?}", page.target_id());
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
        .map_err(|e| Error::Browser(format!("new_tab failed: {e}")))
}

/// Close a tab by index (0-based). The page is consumed.
pub async fn close_tab(browser: &CrBrowser, index: usize) -> Result<()> {
    let pages = browser
        .pages()
        .await
        .map_err(|e| Error::Browser(format!("close_tab list failed: {e}")))?;
    if index >= pages.len() {
        return Err(Error::Browser(format!(
            "Tab index {} out of bounds ({})",
            index,
            pages.len()
        )));
    }
    // Page::close takes ownership — clone first then close
    let page = pages.into_iter().nth(index).unwrap();
    page.close()
        .await
        .map_err(|e| Error::Browser(format!("close_tab failed: {e}")))?;
    Ok(())
}

/// Get a page reference by index.
pub async fn get_tab(browser: &CrBrowser, index: usize) -> Result<Page> {
    let pages = browser
        .pages()
        .await
        .map_err(|e| Error::Browser(format!("get_tab list failed: {e}")))?;
    if index >= pages.len() {
        return Err(Error::Browser(format!(
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
        .map_err(|e| Error::Browser(format!("tab_count failed: {e}")))?;
    Ok(pages.len())
}
