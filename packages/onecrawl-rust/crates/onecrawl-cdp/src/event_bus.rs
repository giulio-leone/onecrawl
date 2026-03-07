//! Event Bus — pub/sub system for integrating OneCrawl with external systems.
//!
//! Provides webhook delivery with HMAC-SHA256 signing, event journaling
//! with replay support, and a broadcast channel for real-time streaming.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Instant;
use tokio::sync::{broadcast, RwLock};

// ────────────────────────────────────────────────────────────────────
//  Core types
// ────────────────────────────────────────────────────────────────────

/// An event that flows through the bus.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BusEvent {
    pub id: String,
    pub event_type: String,
    pub source: String,
    pub timestamp: String,
    pub data: serde_json::Value,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<HashMap<String, String>>,
}

/// A webhook subscription — receives matching events via HTTP POST.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebhookSubscription {
    pub id: String,
    pub event_pattern: String,
    pub url: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub method: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub headers: Option<HashMap<String, String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub secret: Option<String>,
    pub active: bool,
    pub retry_count: u32,
    pub retry_delay_ms: u64,
    pub created_at: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_triggered: Option<String>,
    pub trigger_count: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_error: Option<String>,
}

/// Delivery status for a single subscription.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DeliveryStatus {
    Delivered,
    Failed(String),
    Pending,
    Retrying(u32),
}

/// Journal entry — event + delivery metadata.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JournalEntry {
    pub event: BusEvent,
    pub delivered_to: Vec<String>,
    pub delivery_status: HashMap<String, DeliveryStatus>,
}

/// Event bus runtime statistics.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct BusStats {
    pub total_events: u64,
    pub total_deliveries: u64,
    pub failed_deliveries: u64,
    pub active_webhooks: usize,
    pub journal_size: usize,
    pub uptime_secs: u64,
}

/// Configuration for the event bus.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventBusConfig {
    pub max_journal_size: usize,
    pub max_subscriptions: usize,
    pub webhook_timeout_ms: u64,
    pub enable_journal: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub journal_path: Option<String>,
}

impl Default for EventBusConfig {
    fn default() -> Self {
        Self {
            max_journal_size: 10_000,
            max_subscriptions: 100,
            webhook_timeout_ms: 5_000,
            enable_journal: true,
            journal_path: None,
        }
    }
}

// ────────────────────────────────────────────────────────────────────
//  EventBus
// ────────────────────────────────────────────────────────────────────

pub struct EventBus {
    config: EventBusConfig,
    webhooks: Arc<RwLock<Vec<WebhookSubscription>>>,
    journal: Arc<RwLock<Vec<JournalEntry>>>,
    tx: broadcast::Sender<BusEvent>,
    stats: Arc<RwLock<BusStats>>,
    started_at: Instant,
    http: reqwest::Client,
}

impl EventBus {
    pub fn new(config: EventBusConfig) -> Self {
        let (tx, _) = broadcast::channel(1024);
        let timeout_ms = config.webhook_timeout_ms;
        let http = reqwest::Client::builder()
            .timeout(std::time::Duration::from_millis(timeout_ms))
            .build()
            .unwrap_or_default();

        Self {
            config,
            webhooks: Arc::new(RwLock::new(Vec::new())),
            journal: Arc::new(RwLock::new(Vec::new())),
            tx,
            stats: Arc::new(RwLock::new(BusStats::default())),
            started_at: Instant::now(),
            http,
        }
    }

    /// Emit an event to the bus.
    pub async fn emit(&self, event: BusEvent) -> Result<(), String> {
        // Broadcast (ignore send errors — no subscribers is fine)
        let _ = self.tx.send(event.clone());

        let mut delivered_to = Vec::new();
        let mut delivery_status = HashMap::new();

        // Match against active webhooks and deliver
        let webhooks = self.webhooks.read().await.clone();
        for sub in &webhooks {
            if !sub.active {
                continue;
            }
            if !matches_pattern(&event.event_type, &sub.event_pattern) {
                continue;
            }
            delivered_to.push(sub.id.clone());
            match self.deliver_webhook(sub, &event).await {
                Ok(()) => {
                    delivery_status.insert(sub.id.clone(), DeliveryStatus::Delivered);
                    let mut stats = self.stats.write().await;
                    stats.total_deliveries += 1;
                    // Update subscription stats
                    drop(stats);
                    self.update_sub_success(&sub.id).await;
                }
                Err(e) => {
                    delivery_status
                        .insert(sub.id.clone(), DeliveryStatus::Failed(e.clone()));
                    let mut stats = self.stats.write().await;
                    stats.total_deliveries += 1;
                    stats.failed_deliveries += 1;
                    drop(stats);
                    self.update_sub_error(&sub.id, &e).await;
                }
            }
        }

        // Journal
        if self.config.enable_journal {
            let entry = JournalEntry {
                event: event.clone(),
                delivered_to,
                delivery_status,
            };
            let mut journal = self.journal.write().await;
            if journal.len() >= self.config.max_journal_size {
                let drain = journal.len() - self.config.max_journal_size + 1;
                journal.drain(..drain);
            }
            journal.push(entry);
        }

        let mut stats = self.stats.write().await;
        stats.total_events += 1;
        Ok(())
    }

    /// Subscribe a webhook.
    pub async fn subscribe_webhook(
        &self,
        mut sub: WebhookSubscription,
    ) -> Result<String, String> {
        let mut webhooks = self.webhooks.write().await;
        if webhooks.len() >= self.config.max_subscriptions {
            return Err(format!(
                "max subscriptions ({}) reached",
                self.config.max_subscriptions
            ));
        }
        if sub.id.is_empty() {
            sub.id = generate_id();
        }
        let id = sub.id.clone();
        webhooks.push(sub);
        Ok(id)
    }

    /// Unsubscribe a webhook by ID.
    pub async fn unsubscribe_webhook(&self, id: &str) -> Result<(), String> {
        let mut webhooks = self.webhooks.write().await;
        let len_before = webhooks.len();
        webhooks.retain(|w| w.id != id);
        if webhooks.len() == len_before {
            return Err(format!("subscription '{}' not found", id));
        }
        Ok(())
    }

    /// List all webhook subscriptions.
    pub async fn list_webhooks(&self) -> Vec<WebhookSubscription> {
        self.webhooks.read().await.clone()
    }

    /// Get a broadcast receiver for real-time event streaming.
    pub fn subscribe_stream(&self) -> broadcast::Receiver<BusEvent> {
        self.tx.subscribe()
    }

    /// Replay events from journal matching a pattern and optional timestamp.
    pub async fn replay(
        &self,
        event_pattern: &str,
        since: Option<&str>,
    ) -> Result<Vec<BusEvent>, String> {
        let journal = self.journal.read().await;
        let events: Vec<BusEvent> = journal
            .iter()
            .filter(|entry| {
                if !matches_pattern(&entry.event.event_type, event_pattern) {
                    return false;
                }
                if let Some(since_ts) = since {
                    if entry.event.timestamp.as_str() < since_ts {
                        return false;
                    }
                }
                true
            })
            .map(|entry| entry.event.clone())
            .collect();
        Ok(events)
    }

    /// Get recent events from journal.
    pub async fn recent_events(&self, limit: usize) -> Vec<BusEvent> {
        let journal = self.journal.read().await;
        let start = journal.len().saturating_sub(limit);
        journal[start..]
            .iter()
            .map(|e| e.event.clone())
            .collect()
    }

    /// Clear the journal.
    pub async fn clear_journal(&self) -> Result<(), String> {
        self.journal.write().await.clear();
        Ok(())
    }

    /// Get bus statistics.
    pub async fn stats(&self) -> BusStats {
        let mut s = self.stats.read().await.clone();
        s.uptime_secs = self.started_at.elapsed().as_secs();
        s.active_webhooks = self
            .webhooks
            .read()
            .await
            .iter()
            .filter(|w| w.active)
            .count();
        s.journal_size = self.journal.read().await.len();
        s
    }

    // ── Internal ────────────────────────────────────────────────

    async fn deliver_webhook(
        &self,
        sub: &WebhookSubscription,
        event: &BusEvent,
    ) -> Result<(), String> {
        let body = serde_json::to_string(event).map_err(|e| e.to_string())?;
        let method_str = sub
            .method
            .as_deref()
            .unwrap_or("POST")
            .to_uppercase();

        let mut last_err = String::new();
        let attempts = sub.retry_count.max(1);

        for attempt in 0..attempts {
            if attempt > 0 {
                tokio::time::sleep(std::time::Duration::from_millis(
                    sub.retry_delay_ms * (attempt as u64),
                ))
                .await;
            }

            let mut req = match method_str.as_str() {
                "PUT" => self.http.put(&sub.url),
                "PATCH" => self.http.patch(&sub.url),
                _ => self.http.post(&sub.url),
            };

            req = req
                .header("Content-Type", "application/json")
                .header("X-OneCrawl-Event", &event.event_type)
                .header("X-OneCrawl-Event-Id", &event.id);

            // Custom headers
            if let Some(ref headers) = sub.headers {
                for (k, v) in headers {
                    req = req.header(k.as_str(), v.as_str());
                }
            }

            // HMAC-SHA256 signature
            if let Some(ref secret) = sub.secret {
                let sig = hmac_sha256(secret.as_bytes(), body.as_bytes());
                req = req.header("X-Signature", format!("sha256={}", hex_encode(&sig)));
            }

            req = req.body(body.clone());

            match req.send().await {
                Ok(resp) => {
                    let status = resp.status().as_u16();
                    if (200..300).contains(&status) {
                        return Ok(());
                    }
                    last_err = format!("HTTP {}", status);
                }
                Err(e) => {
                    last_err = e.to_string();
                }
            }
        }

        Err(last_err)
    }

    async fn update_sub_success(&self, id: &str) {
        let mut webhooks = self.webhooks.write().await;
        if let Some(sub) = webhooks.iter_mut().find(|w| w.id == id) {
            sub.trigger_count += 1;
            sub.last_triggered = Some(iso_now());
        }
    }

    async fn update_sub_error(&self, id: &str, err: &str) {
        let mut webhooks = self.webhooks.write().await;
        if let Some(sub) = webhooks.iter_mut().find(|w| w.id == id) {
            sub.trigger_count += 1;
            sub.last_triggered = Some(iso_now());
            sub.last_error = Some(err.to_string());
        }
    }
}

// ────────────────────────────────────────────────────────────────────
//  Glob pattern matching
// ────────────────────────────────────────────────────────────────────

/// Simple glob matching for event type patterns.
///
/// - `**` matches everything
/// - `*` matches any sequence of non-`:` characters within a single segment
/// - Literal characters match exactly
pub fn matches_pattern(event_type: &str, pattern: &str) -> bool {
    if pattern == "**" || pattern == "*" {
        return true;
    }

    let pat_parts: Vec<&str> = pattern.split(':').collect();
    let evt_parts: Vec<&str> = event_type.split(':').collect();

    if pat_parts.len() != evt_parts.len() {
        // Allow trailing ** to match any remaining segments
        if let Some(last) = pat_parts.last() {
            if *last == "**" && evt_parts.len() >= pat_parts.len() - 1 {
                for (p, e) in pat_parts.iter().zip(evt_parts.iter()) {
                    if *p != "**" && *p != "*" && *p != *e {
                        return false;
                    }
                }
                return true;
            }
        }
        return false;
    }

    for (p, e) in pat_parts.iter().zip(evt_parts.iter()) {
        if *p == "*" || *p == "**" {
            continue;
        }
        if *p != *e {
            return false;
        }
    }
    true
}

// ────────────────────────────────────────────────────────────────────
//  Utility functions
// ────────────────────────────────────────────────────────────────────

/// Generate a timestamp-based unique ID.
pub fn generate_id() -> String {
    let ts = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default();
    format!(
        "evt-{}-{}",
        ts.as_millis(),
        ts.subsec_nanos() % 100_000
    )
}

/// ISO 8601 timestamp (UTC).
pub fn iso_now() -> String {
    let d = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default();
    let secs = d.as_secs();
    let millis = d.subsec_millis();
    // Simple ISO format without chrono dependency
    format_unix_timestamp(secs, millis)
}

fn format_unix_timestamp(secs: u64, millis: u32) -> String {
    const SECS_PER_DAY: u64 = 86_400;
    const DAYS_PER_400Y: u64 = 146_097;
    const DAYS_PER_100Y: u64 = 36_524;
    const DAYS_PER_4Y: u64 = 1_461;

    let days = secs / SECS_PER_DAY;
    let time_of_day = secs % SECS_PER_DAY;
    let h = time_of_day / 3600;
    let m = (time_of_day % 3600) / 60;
    let s = time_of_day % 60;

    // Days since 1970-01-01, shift to 2000-03-01 epoch
    let days_from_epoch = days as i64 + 719_468; // shift to 0000-03-01
    let era = if days_from_epoch >= 0 {
        days_from_epoch / DAYS_PER_400Y as i64
    } else {
        (days_from_epoch - (DAYS_PER_400Y as i64 - 1)) / DAYS_PER_400Y as i64
    };
    let doe = (days_from_epoch - era * DAYS_PER_400Y as i64) as u64;
    let yoe = (doe - doe / (DAYS_PER_4Y - 1) + doe / DAYS_PER_100Y
        - doe / (DAYS_PER_400Y - 1))
        / 365;
    let y = yoe as i64 + era * 400;
    let doy = doe
        - (365 * yoe + yoe / 4 - yoe / 100 + yoe / 400);
    let mp = (5 * doy + 2) / 153;
    let d_val = doy - (153 * mp + 2) / 5 + 1;
    let m_val = if mp < 10 { mp + 3 } else { mp - 9 };
    let y_val = if m_val <= 2 { y + 1 } else { y };

    format!(
        "{:04}-{:02}-{:02}T{:02}:{:02}:{:02}.{:03}Z",
        y_val, m_val, d_val, h, m, s, millis
    )
}

/// HMAC-SHA256 using the `ring` crate.
fn hmac_sha256(key: &[u8], data: &[u8]) -> Vec<u8> {
    use ring::hmac;
    let signing_key = hmac::Key::new(hmac::HMAC_SHA256, key);
    let tag = hmac::sign(&signing_key, data);
    tag.as_ref().to_vec()
}

fn hex_encode(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{:02x}", b)).collect()
}

/// Format a BusEvent as an SSE message.
pub fn format_bus_sse(event: &BusEvent) -> String {
    let json = serde_json::to_string(event).unwrap_or_default();
    format!("event: {}\ndata: {}\n\n", event.event_type, json)
}

// ────────────────────────────────────────────────────────────────────
//  Tests
// ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pattern_exact_match() {
        assert!(matches_pattern("page:loaded", "page:loaded"));
        assert!(!matches_pattern("page:loaded", "page:error"));
    }

    #[test]
    fn pattern_wildcard_segment() {
        assert!(matches_pattern("page:loaded", "page:*"));
        assert!(matches_pattern("page:error", "page:*"));
        assert!(matches_pattern("network:error", "*:error"));
        assert!(!matches_pattern("page:loaded", "network:*"));
    }

    #[test]
    fn pattern_double_star() {
        assert!(matches_pattern("page:loaded", "**"));
        assert!(matches_pattern("any:thing", "*"));
        assert!(matches_pattern("deep:nested:event", "deep:**"));
    }

    #[test]
    fn pattern_no_match_different_segments() {
        assert!(!matches_pattern("page:loaded:extra", "page:loaded"));
    }

    #[test]
    fn generate_id_is_nonempty() {
        let id = generate_id();
        assert!(id.starts_with("evt-"));
        assert!(id.len() > 4);
    }

    #[test]
    fn iso_now_format() {
        let ts = iso_now();
        assert!(ts.ends_with('Z'));
        assert!(ts.contains('T'));
    }

    #[test]
    fn hmac_produces_output() {
        let sig = hmac_sha256(b"secret", b"message");
        assert_eq!(sig.len(), 32);
    }

    #[test]
    fn hex_encode_works() {
        assert_eq!(hex_encode(&[0xde, 0xad, 0xbe, 0xef]), "deadbeef");
    }

    #[test]
    fn format_sse_output() {
        let event = BusEvent {
            id: "1".into(),
            event_type: "test:event".into(),
            source: "unit".into(),
            timestamp: "2024-01-01T00:00:00.000Z".into(),
            data: serde_json::json!({"key": "val"}),
            metadata: None,
        };
        let sse = format_bus_sse(&event);
        assert!(sse.starts_with("event: test:event\n"));
        assert!(sse.contains("data: "));
        assert!(sse.ends_with("\n\n"));
    }

    #[tokio::test]
    async fn bus_emit_and_journal() {
        let bus = EventBus::new(EventBusConfig::default());
        let event = BusEvent {
            id: generate_id(),
            event_type: "test:ping".into(),
            source: "test".into(),
            timestamp: iso_now(),
            data: serde_json::json!({}),
            metadata: None,
        };
        bus.emit(event).await.expect("emit should succeed");
        let recent = bus.recent_events(10).await;
        assert_eq!(recent.len(), 1);
        assert_eq!(recent[0].event_type, "test:ping");
    }

    #[tokio::test]
    async fn bus_subscribe_unsubscribe() {
        let bus = EventBus::new(EventBusConfig::default());
        let sub = WebhookSubscription {
            id: String::new(),
            event_pattern: "**".into(),
            url: "http://localhost:9999/hook".into(),
            method: None,
            headers: None,
            secret: None,
            active: true,
            retry_count: 1,
            retry_delay_ms: 100,
            created_at: iso_now(),
            last_triggered: None,
            trigger_count: 0,
            last_error: None,
        };
        let id = bus
            .subscribe_webhook(sub)
            .await
            .expect("subscribe should succeed");
        assert!(!id.is_empty());
        assert_eq!(bus.list_webhooks().await.len(), 1);
        bus.unsubscribe_webhook(&id)
            .await
            .expect("unsubscribe should succeed");
        assert_eq!(bus.list_webhooks().await.len(), 0);
    }

    #[tokio::test]
    async fn bus_replay() {
        let bus = EventBus::new(EventBusConfig::default());
        for i in 0..5 {
            let event = BusEvent {
                id: format!("e{}", i),
                event_type: if i % 2 == 0 {
                    "page:loaded".into()
                } else {
                    "network:error".into()
                },
                source: "test".into(),
                timestamp: iso_now(),
                data: serde_json::json!({"i": i}),
                metadata: None,
            };
            bus.emit(event).await.ok();
        }
        let page_events = bus.replay("page:*", None).await.expect("replay");
        assert_eq!(page_events.len(), 3);
        let net_events = bus.replay("network:*", None).await.expect("replay");
        assert_eq!(net_events.len(), 2);
    }

    #[tokio::test]
    async fn bus_clear_journal() {
        let bus = EventBus::new(EventBusConfig::default());
        let event = BusEvent {
            id: generate_id(),
            event_type: "test:x".into(),
            source: "test".into(),
            timestamp: iso_now(),
            data: serde_json::json!({}),
            metadata: None,
        };
        bus.emit(event).await.ok();
        assert_eq!(bus.recent_events(10).await.len(), 1);
        bus.clear_journal().await.ok();
        assert_eq!(bus.recent_events(10).await.len(), 0);
    }

    #[tokio::test]
    async fn bus_stats() {
        let bus = EventBus::new(EventBusConfig::default());
        let s = bus.stats().await;
        assert_eq!(s.total_events, 0);
        assert_eq!(s.active_webhooks, 0);
    }

    #[tokio::test]
    async fn bus_journal_cap() {
        let cfg = EventBusConfig {
            max_journal_size: 5,
            ..Default::default()
        };
        let bus = EventBus::new(cfg);
        for i in 0..10 {
            let event = BusEvent {
                id: format!("e{}", i),
                event_type: "test:x".into(),
                source: "test".into(),
                timestamp: iso_now(),
                data: serde_json::json!({}),
                metadata: None,
            };
            bus.emit(event).await.ok();
        }
        assert!(bus.recent_events(100).await.len() <= 5);
    }
}
