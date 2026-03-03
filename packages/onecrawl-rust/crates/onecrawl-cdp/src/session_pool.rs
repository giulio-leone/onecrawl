//! Session pool for managing multiple browser sessions in parallel.

use onecrawl_core::{Error, Result};
use serde::{Deserialize, Serialize};
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

/// Information about a single browser session.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionInfo {
    pub id: String,
    pub name: String,
    pub status: String,
    pub created_at: f64,
    pub last_activity: f64,
    pub request_count: usize,
    pub error_count: usize,
    pub tags: Vec<String>,
}

/// Configuration for the session pool.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PoolConfig {
    pub max_sessions: usize,
    pub idle_timeout_ms: u64,
    pub max_errors_per_session: usize,
    pub rotation_strategy: String,
}

impl Default for PoolConfig {
    fn default() -> Self {
        Self {
            max_sessions: 5,
            idle_timeout_ms: 300_000,
            max_errors_per_session: 3,
            rotation_strategy: "round-robin".to_string(),
        }
    }
}

/// Pool of browser sessions.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionPool {
    pub config: PoolConfig,
    pub sessions: Vec<SessionInfo>,
    pub current_index: usize,
}

/// Aggregate statistics for the pool.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PoolStats {
    pub total: usize,
    pub idle: usize,
    pub busy: usize,
    pub error: usize,
    pub closed: usize,
    pub total_requests: usize,
    pub total_errors: usize,
}

fn now_ms() -> f64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs_f64()
        * 1000.0
}

fn gen_id() -> String {
    let ts = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    format!("sess-{ts:x}")
}

impl SessionPool {
    /// Create a new pool with the given configuration.
    pub fn new(config: PoolConfig) -> Self {
        Self {
            config,
            sessions: Vec::new(),
            current_index: 0,
        }
    }
}

/// Add a session to the pool. Returns the generated session ID.
pub fn add_session(pool: &mut SessionPool, name: &str, tags: Option<Vec<String>>) -> String {
    let now = now_ms();
    let id = gen_id();
    let session = SessionInfo {
        id: id.clone(),
        name: name.to_string(),
        status: "idle".to_string(),
        created_at: now,
        last_activity: now,
        request_count: 0,
        error_count: 0,
        tags: tags.unwrap_or_default(),
    };
    pool.sessions.push(session);
    id
}

/// Get the next available session per the configured rotation strategy.
pub fn get_next(pool: &mut SessionPool) -> Option<&mut SessionInfo> {
    let available: Vec<usize> = pool
        .sessions
        .iter()
        .enumerate()
        .filter(|(_, s)| s.status == "idle")
        .map(|(i, _)| i)
        .collect();

    if available.is_empty() {
        return None;
    }

    let idx = match pool.config.rotation_strategy.as_str() {
        "least-used" => {
            let mut min_idx = available[0];
            let mut min_count = pool.sessions[available[0]].request_count;
            for &i in &available[1..] {
                if pool.sessions[i].request_count < min_count {
                    min_count = pool.sessions[i].request_count;
                    min_idx = i;
                }
            }
            min_idx
        }
        "random" => {
            // Deterministic pseudo-random: use timestamp modulo
            let ts = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_nanos() as usize;
            available[ts % available.len()]
        }
        _ => {
            // round-robin (default)
            let start = pool.current_index;
            let mut chosen = available[0];
            for &i in &available {
                if i >= start {
                    chosen = i;
                    break;
                }
            }
            pool.current_index = chosen + 1;
            chosen
        }
    };

    Some(&mut pool.sessions[idx])
}

/// Mark a session as busy.
pub fn mark_busy(pool: &mut SessionPool, id: &str) {
    if let Some(s) = pool.sessions.iter_mut().find(|s| s.id == id) {
        s.status = "busy".to_string();
        s.last_activity = now_ms();
        s.request_count += 1;
    }
}

/// Mark a session as idle.
pub fn mark_idle(pool: &mut SessionPool, id: &str) {
    if let Some(s) = pool.sessions.iter_mut().find(|s| s.id == id) {
        s.status = "idle".to_string();
        s.last_activity = now_ms();
    }
}

/// Mark a session as having encountered an error. Closes it if error count exceeds the limit.
pub fn mark_error(pool: &mut SessionPool, id: &str, _error: &str) {
    if let Some(s) = pool.sessions.iter_mut().find(|s| s.id == id) {
        s.error_count += 1;
        s.last_activity = now_ms();
        if s.error_count >= pool.config.max_errors_per_session {
            s.status = "closed".to_string();
        } else {
            s.status = "error".to_string();
        }
    }
}

/// Close a session by ID.
pub fn close_session(pool: &mut SessionPool, id: &str) {
    if let Some(s) = pool.sessions.iter_mut().find(|s| s.id == id) {
        s.status = "closed".to_string();
    }
}

/// Get aggregate statistics for the pool.
pub fn get_stats(pool: &SessionPool) -> PoolStats {
    let mut stats = PoolStats {
        total: pool.sessions.len(),
        idle: 0,
        busy: 0,
        error: 0,
        closed: 0,
        total_requests: 0,
        total_errors: 0,
    };
    for s in &pool.sessions {
        match s.status.as_str() {
            "idle" => stats.idle += 1,
            "busy" => stats.busy += 1,
            "error" => stats.error += 1,
            "closed" => stats.closed += 1,
            _ => {}
        }
        stats.total_requests += s.request_count;
        stats.total_errors += s.error_count;
    }
    stats
}

/// Close sessions that have been idle longer than the timeout. Returns the count closed.
pub fn cleanup_idle(pool: &mut SessionPool) -> usize {
    let now = now_ms();
    let timeout = pool.config.idle_timeout_ms as f64;
    let mut count = 0;
    for s in &mut pool.sessions {
        if s.status == "idle" && (now - s.last_activity) > timeout {
            s.status = "closed".to_string();
            count += 1;
        }
    }
    count
}

/// Save the pool to a JSON file.
pub fn save_pool(pool: &SessionPool, path: &Path) -> Result<()> {
    let json = serde_json::to_string_pretty(pool)
        .map_err(|e| Error::Cdp(format!("serialize pool failed: {e}")))?;
    std::fs::write(path, json)?;
    Ok(())
}

/// Load a pool from a JSON file.
pub fn load_pool(path: &Path) -> Result<SessionPool> {
    let data = std::fs::read_to_string(path)?;
    let pool: SessionPool = serde_json::from_str(&data)
        .map_err(|e| Error::Cdp(format!("parse pool failed: {e}")))?;
    Ok(pool)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_pool() {
        let pool = SessionPool::new(PoolConfig::default());
        assert!(pool.sessions.is_empty());
        assert_eq!(pool.current_index, 0);
    }

    #[test]
    fn test_add_session_returns_id() {
        let mut pool = SessionPool::new(PoolConfig::default());
        let id = add_session(&mut pool, "s1", None);
        assert!(id.starts_with("sess-"));
        assert_eq!(pool.sessions.len(), 1);
        assert_eq!(pool.sessions[0].status, "idle");
    }

    #[test]
    fn test_get_next_round_robin() {
        let mut pool = SessionPool::new(PoolConfig::default());
        add_session(&mut pool, "a", None);
        add_session(&mut pool, "b", None);
        let first = get_next(&mut pool).map(|s| s.name.clone());
        assert_eq!(first.as_deref(), Some("a"));
        let id = pool.sessions[0].id.clone();
        mark_busy(&mut pool, &id);
        let second = get_next(&mut pool).map(|s| s.name.clone());
        assert_eq!(second.as_deref(), Some("b"));
    }

    #[test]
    fn test_get_next_none_when_all_busy() {
        let mut pool = SessionPool::new(PoolConfig::default());
        let id = add_session(&mut pool, "only", None);
        mark_busy(&mut pool, &id);
        assert!(get_next(&mut pool).is_none());
    }

    #[test]
    fn test_mark_busy_and_idle() {
        let mut pool = SessionPool::new(PoolConfig::default());
        let id = add_session(&mut pool, "x", None);
        mark_busy(&mut pool, &id);
        assert_eq!(pool.sessions[0].status, "busy");
        assert_eq!(pool.sessions[0].request_count, 1);
        mark_idle(&mut pool, &id);
        assert_eq!(pool.sessions[0].status, "idle");
    }

    #[test]
    fn test_mark_error_closes_after_max() {
        let mut pool = SessionPool::new(PoolConfig {
            max_errors_per_session: 2,
            ..PoolConfig::default()
        });
        let id = add_session(&mut pool, "err", None);
        mark_error(&mut pool, &id, "e1");
        assert_eq!(pool.sessions[0].status, "error");
        mark_error(&mut pool, &id, "e2");
        assert_eq!(pool.sessions[0].status, "closed");
    }

    #[test]
    fn test_close_session() {
        let mut pool = SessionPool::new(PoolConfig::default());
        let id = add_session(&mut pool, "c", None);
        close_session(&mut pool, &id);
        assert_eq!(pool.sessions[0].status, "closed");
    }

    #[test]
    fn test_get_stats() {
        let mut pool = SessionPool::new(PoolConfig::default());
        let id1 = add_session(&mut pool, "s1", None);
        add_session(&mut pool, "s2", None);
        mark_busy(&mut pool, &id1);
        let stats = get_stats(&pool);
        assert_eq!(stats.total, 2);
        assert_eq!(stats.busy, 1);
        assert_eq!(stats.idle, 1);
        assert_eq!(stats.total_requests, 1);
    }

    #[test]
    fn test_cleanup_idle() {
        let mut pool = SessionPool::new(PoolConfig {
            idle_timeout_ms: 0,
            ..PoolConfig::default()
        });
        add_session(&mut pool, "old", None);
        // With timeout=0 and any positive elapsed time, session should be cleaned
        std::thread::sleep(std::time::Duration::from_millis(2));
        let closed = cleanup_idle(&mut pool);
        assert_eq!(closed, 1);
        assert_eq!(pool.sessions[0].status, "closed");
    }

    #[test]
    fn test_save_and_load() {
        let mut pool = SessionPool::new(PoolConfig::default());
        add_session(&mut pool, "persist", Some(vec!["tag1".to_string()]));
        let dir = std::env::temp_dir();
        let path = dir.join("test_session_pool.json");
        save_pool(&pool, &path).unwrap();
        let loaded = load_pool(&path).unwrap();
        assert_eq!(loaded.sessions.len(), 1);
        assert_eq!(loaded.sessions[0].name, "persist");
        assert_eq!(loaded.sessions[0].tags, vec!["tag1"]);
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn test_least_used_strategy() {
        let mut pool = SessionPool::new(PoolConfig {
            rotation_strategy: "least-used".to_string(),
            ..PoolConfig::default()
        });
        let id1 = add_session(&mut pool, "heavy", None);
        add_session(&mut pool, "light", None);
        // Make first session have more requests
        mark_busy(&mut pool, &id1);
        mark_idle(&mut pool, &id1);
        let next = get_next(&mut pool).map(|s| s.name.clone());
        assert_eq!(next.as_deref(), Some("light"));
    }
}
