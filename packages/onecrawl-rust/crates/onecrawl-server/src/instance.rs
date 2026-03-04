use chromiumoxide::browser::{Browser, BrowserConfig};
use chromiumoxide::Page;
use futures::StreamExt;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tokio::sync::RwLock;

/// Summary info returned by list/get endpoints.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InstanceInfo {
    pub id: String,
    pub profile: Option<String>,
    pub headless: bool,
    pub status: &'static str,
    pub port: u16,
    pub start_time: String,
    pub tabs: Vec<TabSummary>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TabSummary {
    pub id: String,
    pub url: String,
    pub title: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateInstanceRequest {
    pub profile: Option<String>,
    pub headless: Option<bool>,
}

/// A running Chrome instance.
pub struct Instance {
    pub id: String,
    pub browser: Browser,
    pub _handler_task: tokio::task::JoinHandle<()>,
    pub profile: Option<String>,
    pub headless: bool,
    pub port: u16,
    pub start_time: String,
    /// tab_id -> Page handle
    pub tabs: RwLock<HashMap<String, Page>>,
    pub tab_counter: RwLock<u32>,
}

impl Instance {
    /// Launch a new Chrome instance.
    pub async fn launch(
        id: String,
        headless: bool,
        port: u16,
        profile: Option<String>,
        user_data_dir: Option<std::path::PathBuf>,
    ) -> Result<Self, String> {
        let mut builder = BrowserConfig::builder();
        if !headless {
            builder = builder.with_head();
        }
        if let Some(dir) = user_data_dir {
            builder = builder.user_data_dir(dir);
        }
        let config = builder
            .build()
            .map_err(|e| format!("config error: {e}"))?;

        let (browser, mut handler) = Browser::launch(config)
            .await
            .map_err(|e| format!("launch failed: {e}"))?;

        let handler_task = tokio::spawn(async move {
            while let Some(h) = handler.next().await {
                if h.is_err() {
                    break;
                }
            }
        });

        let now = chrono_now();

        Ok(Self {
            id,
            browser,
            _handler_task: handler_task,
            profile,
            headless,
            port,
            start_time: now,
            tabs: RwLock::new(HashMap::new()),
            tab_counter: RwLock::new(0),
        })
    }

    /// Build an InstanceInfo snapshot (requires async for tab URLs).
    /// Clones Page handles (cheap channel handles), drops the lock, then
    /// fetches URL/title concurrently via `join_all` — O(rtt) not O(n*rtt).
    pub async fn info(&self) -> InstanceInfo {
        let pages: Vec<(String, Page)> = {
            let tabs_guard = self.tabs.read().await;
            tabs_guard
                .iter()
                .map(|(tid, page)| (tid.clone(), page.clone()))
                .collect()
        }; // lock dropped here

        let futs = pages.into_iter().map(|(tid, page)| async move {
            let url = page.url().await.ok().flatten().unwrap_or_default();
            let title: String = page
                .evaluate("document.title")
                .await
                .ok()
                .and_then(|v| v.into_value().ok())
                .unwrap_or_default();
            TabSummary { id: tid, url, title }
        });
        let tab_summaries = futures::future::join_all(futs).await;

        InstanceInfo {
            id: self.id.clone(),
            profile: self.profile.clone(),
            headless: self.headless,
            status: "running",
            port: self.port,
            start_time: self.start_time.clone(),
            tabs: tab_summaries,
        }
    }
}

fn chrono_now() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    // Simple ISO-ish timestamp without chrono dep
    format!("{secs}")
}
