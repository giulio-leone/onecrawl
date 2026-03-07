//! Multi-browser pool — manages a pool of browser page handles for parallel automation.

use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;

use onecrawl_browser::Page;
use onecrawl_core::{Error, Result};
use serde::{Deserialize, Serialize};

/// Public snapshot of a browser instance (no `Page` handle).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BrowserInstance {
    pub id: String,
    pub status: BrowserStatus,
    pub url: Option<String>,
    pub created_at: u64,
}

/// Lifecycle status of a pooled browser page.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum BrowserStatus {
    Idle,
    Busy,
    Error,
    Closed,
}

/// Internal bookkeeping for a pooled page.
struct PoolEntry {
    page: Page,
    status: BrowserStatus,
    url: Option<String>,
    created_at: u64,
}

/// Manages a pool of browser instances for parallel automation.
pub struct BrowserPool {
    instances: HashMap<String, PoolEntry>,
    max_size: usize,
}

impl BrowserPool {
    pub fn new(max_size: usize) -> Self {
        Self {
            instances: HashMap::new(),
            max_size,
        }
    }

    /// Add a browser page to the pool with a given ID.
    pub fn add(&mut self, id: String, page: Page) -> Result<()> {
        if self.instances.len() >= self.max_size {
            return Err(Error::Cdp(format!(
                "Pool full: max {} instances",
                self.max_size
            )));
        }
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        self.instances.insert(
            id,
            PoolEntry {
                page,
                status: BrowserStatus::Idle,
                url: None,
                created_at: now,
            },
        );
        Ok(())
    }

    /// Get a page from the pool by ID.
    pub fn get(&self, id: &str) -> Option<&Page> {
        self.instances.get(id).map(|e| &e.page)
    }

    /// Remove an instance from the pool, returning its page handle.
    pub fn remove(&mut self, id: &str) -> Option<Page> {
        self.instances.remove(id).map(|e| e.page)
    }

    /// List all instances with their status.
    pub fn list(&self) -> Vec<BrowserInstance> {
        self.instances
            .iter()
            .map(|(id, entry)| BrowserInstance {
                id: id.clone(),
                status: entry.status.clone(),
                url: entry.url.clone(),
                created_at: entry.created_at,
            })
            .collect()
    }

    /// Get the first idle instance.
    pub fn get_idle(&self) -> Option<(&str, &Page)> {
        self.instances
            .iter()
            .find(|(_, e)| e.status == BrowserStatus::Idle)
            .map(|(id, e)| (id.as_str(), &e.page))
    }

    /// Mark instance as busy/idle/etc.
    pub fn set_status(&mut self, id: &str, status: BrowserStatus) {
        if let Some(entry) = self.instances.get_mut(id) {
            entry.status = status;
        }
    }

    /// Set URL for tracking.
    pub fn set_url(&mut self, id: &str, url: String) {
        if let Some(entry) = self.instances.get_mut(id) {
            entry.url = Some(url);
        }
    }

    /// Number of instances in the pool.
    pub fn len(&self) -> usize {
        self.instances.len()
    }

    /// Whether the pool has no instances.
    pub fn is_empty(&self) -> bool {
        self.instances.is_empty()
    }

    /// Configured maximum pool size.
    pub fn max_size(&self) -> usize {
        self.max_size
    }

    /// Number of idle instances.
    pub fn idle_count(&self) -> usize {
        self.instances
            .values()
            .filter(|e| e.status == BrowserStatus::Idle)
            .count()
    }
}

impl Default for BrowserPool {
    fn default() -> Self {
        Self::new(10)
    }
}

/// Thread-safe pool wrapper.
pub type SharedPool = Arc<Mutex<BrowserPool>>;

/// Create a new thread-safe browser pool.
pub fn new_shared_pool(max_size: usize) -> SharedPool {
    Arc::new(Mutex::new(BrowserPool::new(max_size)))
}
