//! E2E tests for the event bus.
//! Tests event emission, journal, stats, pattern matching, and webhook validation.

use onecrawl_cdp::event_bus::{
    self, BusEvent, EventBus, EventBusConfig, WebhookSubscription,
};


fn test_bus() -> EventBus {
    EventBus::new(EventBusConfig::default())
}

fn test_event(event_type: &str) -> BusEvent {
    BusEvent {
        id: event_bus::generate_id(),
        event_type: event_type.to_string(),
        source: "test".to_string(),
        timestamp: event_bus::iso_now(),
        data: serde_json::json!({"key": "value"}),
        metadata: None,
    }
}

// ────────────────────── Construction ──────────────────────

#[test]
fn e2e_event_bus_config_default() {
    let config = EventBusConfig::default();
    assert!(config.max_journal_size > 0);
    assert!(config.max_subscriptions > 0);
    assert!(config.enable_journal);
}

#[test]
fn e2e_event_bus_new() {
    let _bus = test_bus();
}

// ────────────────────── emit + recent_events ──────────────────────

#[tokio::test]
async fn e2e_event_bus_emit_and_recent() {
    let bus = test_bus();
    bus.emit(test_event("page:load")).await.unwrap();

    let events = bus.recent_events(10).await;
    assert_eq!(events.len(), 1);
    assert_eq!(events[0].event_type, "page:load");
}

#[tokio::test]
async fn e2e_event_bus_emit_multiple() {
    let bus = test_bus();
    bus.emit(test_event("page:load")).await.unwrap();
    bus.emit(test_event("page:click")).await.unwrap();
    bus.emit(test_event("network:request")).await.unwrap();

    let events = bus.recent_events(10).await;
    assert_eq!(events.len(), 3);
}

// ────────────────────── stats ──────────────────────

#[tokio::test]
async fn e2e_event_bus_stats() {
    let bus = test_bus();
    bus.emit(test_event("page:load")).await.unwrap();
    bus.emit(test_event("page:click")).await.unwrap();

    let stats = bus.stats().await;
    assert_eq!(stats.total_events, 2);
    assert_eq!(stats.journal_size, 2);
}

// ────────────────────── clear_journal ──────────────────────

#[tokio::test]
async fn e2e_event_bus_clear_journal() {
    let bus = test_bus();
    bus.emit(test_event("page:load")).await.unwrap();

    bus.clear_journal().await.unwrap();

    let events = bus.recent_events(10).await;
    assert!(events.is_empty());
}

// ────────────────────── matches_pattern ──────────────────────

#[test]
fn e2e_matches_pattern_exact() {
    assert!(event_bus::matches_pattern("page:load", "page:load"));
    assert!(!event_bus::matches_pattern("page:load", "page:click"));
}

#[test]
fn e2e_matches_pattern_wildcard_all() {
    assert!(event_bus::matches_pattern("page:load", "**"));
    assert!(event_bus::matches_pattern("network:request", "**"));
}

#[test]
fn e2e_matches_pattern_glob() {
    assert!(event_bus::matches_pattern("page:load", "page:*"));
    assert!(event_bus::matches_pattern("page:click", "page:*"));
    assert!(!event_bus::matches_pattern("network:request", "page:*"));
}

// ────────────────────── generate_id ──────────────────────

#[test]
fn e2e_generate_id_unique() {
    let id1 = event_bus::generate_id();
    let id2 = event_bus::generate_id();
    assert_ne!(id1, id2, "generated IDs must be unique");
    assert!(!id1.is_empty());
}

// ────────────────────── subscribe_webhook validation ──────────────────────

#[tokio::test]
async fn e2e_subscribe_webhook_invalid_url_fails() {
    let bus = test_bus();
    let sub = WebhookSubscription {
        id: String::new(),
        event_pattern: "**".to_string(),
        url: "http://localhost:1234/hook".to_string(),
        method: None,
        headers: None,
        secret: None,
        active: true,
        retry_count: 0,
        retry_delay_ms: 0,
        created_at: event_bus::iso_now(),
        last_triggered: None,
        trigger_count: 0,
        last_error: None,
    };
    // localhost/127.0.0.1 should be rejected as SSRF
    let result = bus.subscribe_webhook(sub).await;
    assert!(result.is_err(), "localhost webhook should be rejected");
}

#[tokio::test]
async fn e2e_subscribe_webhook_internal_ip_fails() {
    let bus = test_bus();
    let sub = WebhookSubscription {
        id: String::new(),
        event_pattern: "**".to_string(),
        url: "http://169.254.169.254/metadata".to_string(),
        method: None,
        headers: None,
        secret: None,
        active: true,
        retry_count: 0,
        retry_delay_ms: 0,
        created_at: event_bus::iso_now(),
        last_triggered: None,
        trigger_count: 0,
        last_error: None,
    };
    let result = bus.subscribe_webhook(sub).await;
    assert!(result.is_err(), "internal IP webhook should be rejected");
}
