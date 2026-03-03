//! Retry queue for failed browser automation operations with exponential backoff.

use onecrawl_core::{Error, Result};
use serde::{Deserialize, Serialize};
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

/// Configuration for retry behaviour.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RetryConfig {
    pub max_retries: usize,
    pub initial_delay_ms: u64,
    pub max_delay_ms: u64,
    pub backoff_factor: f64,
    pub jitter: bool,
}

impl Default for RetryConfig {
    fn default() -> Self {
        Self {
            max_retries: 3,
            initial_delay_ms: 1000,
            max_delay_ms: 30000,
            backoff_factor: 2.0,
            jitter: true,
        }
    }
}

/// A single item in the retry queue.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RetryItem {
    pub id: String,
    pub url: String,
    pub operation: String,
    pub payload: Option<String>,
    pub retries: usize,
    pub last_error: Option<String>,
    pub next_retry_ms: f64,
    pub status: String,
    pub created_at: f64,
}

/// The retry queue holding pending and completed items.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RetryQueue {
    pub config: RetryConfig,
    pub items: Vec<RetryItem>,
    pub completed: Vec<RetryItem>,
}

/// Aggregate statistics for the queue.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueueStats {
    pub pending: usize,
    pub retrying: usize,
    pub completed_success: usize,
    pub completed_failed: usize,
    pub total_retries: usize,
    pub avg_retries: f64,
}

fn now_ms() -> f64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs_f64()
        * 1000.0
}

/// Generate a simple unique id from timestamp.
fn gen_id() -> String {
    let ts = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    format!("retry-{ts:x}")
}

impl RetryQueue {
    /// Create a new empty retry queue.
    pub fn new(config: RetryConfig) -> Self {
        Self {
            config,
            items: Vec::new(),
            completed: Vec::new(),
        }
    }
}

/// Compute delay in ms for a given retry count using exponential backoff + optional jitter.
pub fn calculate_delay(config: &RetryConfig, retry_count: usize) -> u64 {
    let base = config.initial_delay_ms as f64 * config.backoff_factor.powi(retry_count as i32);
    let capped = base.min(config.max_delay_ms as f64);

    if config.jitter {
        // Deterministic jitter: use timestamp nanos modulo to add 0-30%
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .subsec_nanos() as f64;
        let jitter_frac = (nanos % 1000.0) / 1000.0 * 0.3; // 0..0.3
        (capped * (1.0 + jitter_frac)).min(config.max_delay_ms as f64) as u64
    } else {
        capped as u64
    }
}

/// Add an item to the queue. Returns the generated item id.
pub fn enqueue(
    queue: &mut RetryQueue,
    url: &str,
    operation: &str,
    payload: Option<&str>,
) -> String {
    let id = gen_id();
    let now = now_ms();
    let item = RetryItem {
        id: id.clone(),
        url: url.to_string(),
        operation: operation.to_string(),
        payload: payload.map(|s| s.to_string()),
        retries: 0,
        last_error: None,
        next_retry_ms: now, // immediately available
        status: "pending".to_string(),
        created_at: now,
    };
    queue.items.push(item);
    id
}

/// Get the next item that is due for retry (earliest `next_retry_ms` ≤ now).
/// Returns `None` if no item is due.
pub fn get_next(queue: &mut RetryQueue) -> Option<&RetryItem> {
    let now = now_ms();
    // Find the item with smallest next_retry_ms that is <= now
    let idx = queue
        .items
        .iter()
        .enumerate()
        .filter(|(_, item)| item.next_retry_ms <= now && item.status != "success")
        .min_by(|(_, a), (_, b)| {
            a.next_retry_ms
                .partial_cmp(&b.next_retry_ms)
                .unwrap_or(std::cmp::Ordering::Equal)
        })
        .map(|(i, _)| i);

    if let Some(i) = idx {
        queue.items[i].status = "retrying".to_string();
        Some(&queue.items[i])
    } else {
        None
    }
}

/// Mark an item as successfully completed.
pub fn mark_success(queue: &mut RetryQueue, id: &str) {
    if let Some(pos) = queue.items.iter().position(|item| item.id == id) {
        let mut item = queue.items.remove(pos);
        item.status = "success".to_string();
        queue.completed.push(item);
    }
}

/// Mark an item as failed. Schedules a retry or moves to completed if max retries reached.
pub fn mark_failure(queue: &mut RetryQueue, id: &str, error: &str) {
    if let Some(item) = queue.items.iter_mut().find(|item| item.id == id) {
        item.retries += 1;
        item.last_error = Some(error.to_string());

        if item.retries >= queue.config.max_retries {
            item.status = "failed".to_string();
            // Move to completed
            let pos = queue.items.iter().position(|i| i.id == id).unwrap();
            let done = queue.items.remove(pos);
            queue.completed.push(done);
        } else {
            let delay = calculate_delay(&queue.config, item.retries);
            item.next_retry_ms = now_ms() + delay as f64;
            item.status = "pending".to_string();
        }
    }
}

/// Get aggregate statistics for the queue.
pub fn get_stats(queue: &RetryQueue) -> QueueStats {
    let pending = queue.items.iter().filter(|i| i.status == "pending").count();
    let retrying = queue
        .items
        .iter()
        .filter(|i| i.status == "retrying")
        .count();
    let completed_success = queue
        .completed
        .iter()
        .filter(|i| i.status == "success")
        .count();
    let completed_failed = queue
        .completed
        .iter()
        .filter(|i| i.status == "failed")
        .count();
    let total_retries: usize = queue.items.iter().map(|i| i.retries).sum::<usize>()
        + queue.completed.iter().map(|i| i.retries).sum::<usize>();
    let total_items = queue.items.len() + queue.completed.len();
    let avg_retries = if total_items > 0 {
        total_retries as f64 / total_items as f64
    } else {
        0.0
    };

    QueueStats {
        pending,
        retrying,
        completed_success,
        completed_failed,
        total_retries,
        avg_retries,
    }
}

/// Remove all completed items and return how many were removed.
pub fn clear_completed(queue: &mut RetryQueue) -> usize {
    let count = queue.completed.len();
    queue.completed.clear();
    count
}

/// Save the queue to a JSON file.
pub fn save_queue(queue: &RetryQueue, path: &Path) -> Result<()> {
    let json = serde_json::to_string_pretty(queue)
        .map_err(|e| Error::Browser(format!("serialize queue failed: {e}")))?;
    std::fs::write(path, json)?;
    Ok(())
}

/// Load a queue from a JSON file.
pub fn load_queue(path: &Path) -> Result<RetryQueue> {
    let data = std::fs::read_to_string(path)?;
    let queue: RetryQueue = serde_json::from_str(&data)
        .map_err(|e| Error::Browser(format!("parse queue failed: {e}")))?;
    Ok(queue)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_queue() {
        let q = RetryQueue::new(RetryConfig::default());
        assert!(q.items.is_empty());
        assert!(q.completed.is_empty());
    }

    #[test]
    fn test_enqueue_and_get_next() {
        let mut q = RetryQueue::new(RetryConfig::default());
        let id = enqueue(&mut q, "https://example.com", "navigate", None);
        assert!(!id.is_empty());
        assert_eq!(q.items.len(), 1);

        let next = get_next(&mut q);
        assert!(next.is_some());
        assert_eq!(next.unwrap().url, "https://example.com");
    }

    #[test]
    fn test_mark_success() {
        let mut q = RetryQueue::new(RetryConfig::default());
        let id = enqueue(&mut q, "https://example.com", "click", None);
        mark_success(&mut q, &id);
        assert!(q.items.is_empty());
        assert_eq!(q.completed.len(), 1);
        assert_eq!(q.completed[0].status, "success");
    }

    #[test]
    fn test_mark_failure_retries() {
        let mut q = RetryQueue::new(RetryConfig {
            max_retries: 3,
            ..RetryConfig::default()
        });
        let id = enqueue(&mut q, "https://example.com", "extract", None);
        mark_failure(&mut q, &id, "timeout");
        // Still in items (retry 1 of 3)
        assert_eq!(q.items.len(), 1);
        assert_eq!(q.items[0].retries, 1);
        assert_eq!(q.items[0].last_error.as_deref(), Some("timeout"));
    }

    #[test]
    fn test_mark_failure_exhausted() {
        let mut q = RetryQueue::new(RetryConfig {
            max_retries: 2,
            ..RetryConfig::default()
        });
        let id = enqueue(&mut q, "https://example.com", "submit", None);
        mark_failure(&mut q, &id, "err1");
        mark_failure(&mut q, &id, "err2");
        // After 2 failures with max_retries=2, should be completed
        assert!(q.items.is_empty());
        assert_eq!(q.completed.len(), 1);
        assert_eq!(q.completed[0].status, "failed");
    }

    #[test]
    fn test_get_stats() {
        let mut q = RetryQueue::new(RetryConfig::default());
        enqueue(&mut q, "https://a.com", "navigate", None);
        let id2 = enqueue(&mut q, "https://b.com", "click", None);
        mark_success(&mut q, &id2);
        let stats = get_stats(&q);
        assert_eq!(stats.pending, 1);
        assert_eq!(stats.completed_success, 1);
    }

    #[test]
    fn test_clear_completed() {
        let mut q = RetryQueue::new(RetryConfig::default());
        let id = enqueue(&mut q, "https://a.com", "navigate", None);
        mark_success(&mut q, &id);
        assert_eq!(q.completed.len(), 1);
        let cleared = clear_completed(&mut q);
        assert_eq!(cleared, 1);
        assert!(q.completed.is_empty());
    }

    #[test]
    fn test_calculate_delay_exponential() {
        let config = RetryConfig {
            initial_delay_ms: 1000,
            max_delay_ms: 30000,
            backoff_factor: 2.0,
            jitter: false,
            ..RetryConfig::default()
        };
        assert_eq!(calculate_delay(&config, 0), 1000);
        assert_eq!(calculate_delay(&config, 1), 2000);
        assert_eq!(calculate_delay(&config, 2), 4000);
        assert_eq!(calculate_delay(&config, 3), 8000);
    }

    #[test]
    fn test_calculate_delay_capped() {
        let config = RetryConfig {
            initial_delay_ms: 1000,
            max_delay_ms: 5000,
            backoff_factor: 2.0,
            jitter: false,
            ..RetryConfig::default()
        };
        assert_eq!(calculate_delay(&config, 10), 5000);
    }

    #[test]
    fn test_save_and_load_queue() {
        let mut q = RetryQueue::new(RetryConfig::default());
        enqueue(
            &mut q,
            "https://example.com",
            "navigate",
            Some("test-payload"),
        );

        let dir = std::env::temp_dir();
        let path = dir.join("onecrawl_retry_queue_test.json");
        save_queue(&q, &path).unwrap();

        let loaded = load_queue(&path).unwrap();
        assert_eq!(loaded.items.len(), 1);
        assert_eq!(loaded.items[0].url, "https://example.com");
        assert_eq!(loaded.items[0].payload.as_deref(), Some("test-payload"));

        let _ = std::fs::remove_file(&path);
    }
}
