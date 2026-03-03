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
    pub snapshots: RwLock<HashMap<String, Vec<SnapshotElement>>>,
}

impl ServerState {
    pub fn new(port: u16) -> Self {
        Self {
            instances: RwLock::new(HashMap::new()),
            profiles: RwLock::new(HashMap::new()),
            port,
            next_instance_port: RwLock::new(port + 1),
            snapshots: RwLock::new(HashMap::new()),
        }
    }
}

pub type AppState = Arc<ServerState>;
