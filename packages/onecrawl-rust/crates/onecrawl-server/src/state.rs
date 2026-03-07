use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;

use crate::instance::Instance;
use crate::profile::Profile;
use crate::snapshot::SnapshotElement;

/// Maximum number of cached snapshots before LRU eviction kicks in.
const MAX_SNAPSHOTS: usize = 64;

/// Default lock TTL: 60 seconds.
const DEFAULT_LOCK_TTL_SECS: u64 = 60;

/// A per-tab lock for multi-agent safety.
#[derive(Debug, Clone, serde::Serialize)]
pub struct TabLock {
    pub owner: String,
    #[serde(skip)]
    pub acquired_at: Instant,
    pub ttl_secs: u64,
}

impl TabLock {
    pub fn is_expired(&self) -> bool {
        self.acquired_at.elapsed() > Duration::from_secs(self.ttl_secs)
    }
}

/// Shared server state behind an Arc for axum handlers.
pub struct ServerState {
    pub instances: RwLock<HashMap<String, Instance>>,
    pub profiles: RwLock<HashMap<String, Profile>>,
    pub port: u16,
    pub next_instance_port: RwLock<u16>,
    /// Last snapshot per tab_id for element-ref lookups during actions.
    /// Bounded to MAX_SNAPSHOTS entries with simple eviction.
    pub snapshots: RwLock<HashMap<String, Arc<Vec<SnapshotElement>>>>,
    /// Reverse index: tab_id -> instance_id for O(1) tab lookup.
    pub tab_index: RwLock<HashMap<String, String>>,
    /// Per-tab locks for multi-agent safety.
    pub tab_locks: RwLock<HashMap<String, TabLock>>,
    /// Global event bus for pub/sub webhook integration.
    pub event_bus: onecrawl_cdp::EventBus,
}

impl ServerState {
    pub fn new(port: u16) -> Self {
        Self {
            instances: RwLock::new(HashMap::new()),
            profiles: RwLock::new(HashMap::new()),
            port,
            next_instance_port: RwLock::new(port + 1),
            snapshots: RwLock::new(HashMap::with_capacity(MAX_SNAPSHOTS)),
            tab_index: RwLock::new(HashMap::new()),
            tab_locks: RwLock::new(HashMap::new()),
            event_bus: onecrawl_cdp::EventBus::new(onecrawl_cdp::EventBusConfig::default()),
        }
    }

    /// Register a tab in the reverse index.
    pub async fn register_tab(&self, tab_id: &str, instance_id: &str) {
        self.tab_index
            .write()
            .await
            .insert(tab_id.to_owned(), instance_id.to_owned());
    }

    /// Remove a tab from the reverse index (and its cached snapshot).
    pub async fn unregister_tab(&self, tab_id: &str) {
        self.tab_index.write().await.remove(tab_id);
        self.snapshots.write().await.remove(tab_id);
    }

    /// O(1) lookup: tab_id -> instance_id.
    pub async fn instance_for_tab(&self, tab_id: &str) -> Option<String> {
        self.tab_index.read().await.get(tab_id).cloned()
    }

    /// Insert a snapshot, evicting oldest entries if over capacity.
    pub async fn cache_snapshot(&self, tab_id: String, elements: Arc<Vec<SnapshotElement>>) {
        let mut snapshots = self.snapshots.write().await;
        if snapshots.len() >= MAX_SNAPSHOTS && !snapshots.contains_key(&tab_id) {
            // Simple eviction: remove first key (arbitrary but O(1))
            if let Some(key) = snapshots.keys().next().cloned() {
                snapshots.remove(&key);
            }
        }
        snapshots.insert(tab_id, elements);
    }

    /// Acquire a lock on a tab. Returns Ok(()) if acquired, Err with current owner if locked.
    pub async fn lock_tab(
        &self,
        tab_id: &str,
        owner: &str,
        ttl_secs: Option<u64>,
    ) -> Result<(), String> {
        let mut locks = self.tab_locks.write().await;
        if let Some(existing) = locks.get(tab_id)
            && !existing.is_expired() && existing.owner != owner {
                return Err(existing.owner.clone());
            }
        locks.insert(
            tab_id.to_owned(),
            TabLock {
                owner: owner.to_owned(),
                acquired_at: Instant::now(),
                ttl_secs: ttl_secs.unwrap_or(DEFAULT_LOCK_TTL_SECS),
            },
        );
        Ok(())
    }

    /// Release a lock. Only the owner can release it.
    pub async fn unlock_tab(&self, tab_id: &str, owner: &str) -> bool {
        let mut locks = self.tab_locks.write().await;
        if let Some(existing) = locks.get(tab_id) {
            if existing.owner == owner || existing.is_expired() {
                locks.remove(tab_id);
                return true;
            }
            return false;
        }
        true // no lock = already unlocked
    }

    /// Check if a tab is locked and by whom.
    pub async fn get_tab_lock(&self, tab_id: &str) -> Option<TabLock> {
        let locks = self.tab_locks.read().await;
        locks.get(tab_id).and_then(|l| {
            if l.is_expired() { None } else { Some(l.clone()) }
        })
    }
}

pub type AppState = Arc<ServerState>;

#[cfg(test)]
mod tests {
    use super::*;

    fn make_state() -> ServerState {
        ServerState::new(9867)
    }

    #[tokio::test]
    async fn test_lock_acquire_and_release() {
        let state = make_state();
        assert!(state.lock_tab("t1", "agent-a", None).await.is_ok());
        assert!(state.get_tab_lock("t1").await.is_some());
        assert!(state.unlock_tab("t1", "agent-a").await);
        assert!(state.get_tab_lock("t1").await.is_none());
    }

    #[tokio::test]
    async fn test_lock_conflict() {
        let state = make_state();
        assert!(state.lock_tab("t1", "agent-a", None).await.is_ok());
        let err = state.lock_tab("t1", "agent-b", None).await;
        assert_eq!(err, Err("agent-a".into()));
    }

    #[tokio::test]
    async fn test_lock_same_owner_reacquire() {
        let state = make_state();
        assert!(state.lock_tab("t1", "agent-a", None).await.is_ok());
        assert!(state.lock_tab("t1", "agent-a", Some(120)).await.is_ok());
        let lock = state.get_tab_lock("t1").await.unwrap();
        assert_eq!(lock.ttl_secs, 120);
    }

    #[tokio::test]
    async fn test_lock_expired_allows_takeover() {
        let state = make_state();
        // Lock with 0-second TTL (instantly expired)
        assert!(state.lock_tab("t1", "agent-a", Some(0)).await.is_ok());
        // Another agent can take over
        assert!(state.lock_tab("t1", "agent-b", None).await.is_ok());
        let lock = state.get_tab_lock("t1").await.unwrap();
        assert_eq!(lock.owner, "agent-b");
    }

    #[tokio::test]
    async fn test_unlock_wrong_owner() {
        let state = make_state();
        assert!(state.lock_tab("t1", "agent-a", None).await.is_ok());
        assert!(!state.unlock_tab("t1", "agent-b").await);
    }

    #[tokio::test]
    async fn test_unlock_nonexistent() {
        let state = make_state();
        assert!(state.unlock_tab("t1", "anyone").await);
    }

    #[tokio::test]
    async fn test_expired_lock_invisible() {
        let state = make_state();
        assert!(state.lock_tab("t1", "agent-a", Some(0)).await.is_ok());
        // Expired lock should be invisible
        assert!(state.get_tab_lock("t1").await.is_none());
    }

    #[tokio::test]
    async fn test_multiple_tab_locks() {
        let state = make_state();
        assert!(state.lock_tab("t1", "agent-a", None).await.is_ok());
        assert!(state.lock_tab("t2", "agent-b", None).await.is_ok());
        assert!(state.lock_tab("t3", "agent-a", None).await.is_ok());
        assert_eq!(state.get_tab_lock("t1").await.unwrap().owner, "agent-a");
        assert_eq!(state.get_tab_lock("t2").await.unwrap().owner, "agent-b");
        assert_eq!(state.get_tab_lock("t3").await.unwrap().owner, "agent-a");
    }

    #[test]
    fn test_tab_lock_is_expired() {
        let lock = TabLock {
            owner: "test".into(),
            acquired_at: Instant::now() - Duration::from_secs(100),
            ttl_secs: 10,
        };
        assert!(lock.is_expired());
    }

    #[test]
    fn test_tab_lock_not_expired() {
        let lock = TabLock {
            owner: "test".into(),
            acquired_at: Instant::now(),
            ttl_secs: 60,
        };
        assert!(!lock.is_expired());
    }
}
