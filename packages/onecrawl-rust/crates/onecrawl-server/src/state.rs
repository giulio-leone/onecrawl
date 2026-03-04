use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

use crate::instance::Instance;
use crate::profile::Profile;
use crate::snapshot::SnapshotElement;

/// Shared server state behind an Arc for axum handlers.
pub struct ServerState {
    pub instances: RwLock<HashMap<String, Instance>>,
    pub profiles: RwLock<HashMap<String, Profile>>,
    pub port: u16,
    pub next_instance_port: RwLock<u16>,
    /// Last snapshot per tab_id for element-ref lookups during actions.
    pub snapshots: RwLock<HashMap<String, Arc<Vec<SnapshotElement>>>>,
    /// Reverse index: tab_id -> instance_id for O(1) tab lookup.
    pub tab_index: RwLock<HashMap<String, String>>,
}

impl ServerState {
    pub fn new(port: u16) -> Self {
        Self {
            instances: RwLock::new(HashMap::new()),
            profiles: RwLock::new(HashMap::new()),
            port,
            next_instance_port: RwLock::new(port + 1),
            snapshots: RwLock::new(HashMap::new()),
            tab_index: RwLock::new(HashMap::new()),
        }
    }

    /// Register a tab in the reverse index.
    pub async fn register_tab(&self, tab_id: &str, instance_id: &str) {
        self.tab_index
            .write()
            .await
            .insert(tab_id.to_owned(), instance_id.to_owned());
    }

    /// Remove a tab from the reverse index.
    pub async fn unregister_tab(&self, tab_id: &str) {
        self.tab_index.write().await.remove(tab_id);
    }

    /// O(1) lookup: tab_id -> instance_id.
    pub async fn instance_for_tab(&self, tab_id: &str) -> Option<String> {
        self.tab_index.read().await.get(tab_id).cloned()
    }
}

pub type AppState = Arc<ServerState>;
