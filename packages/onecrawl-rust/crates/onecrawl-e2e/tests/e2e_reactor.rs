//! E2E tests for the event reactor.
//! Tests ReactorConfig, rule management, status, and event history.

use onecrawl_cdp::reactor::{
    EventFilter, Reactor, ReactorConfig, ReactorEventType, ReactorHandler, ReactorRule,
};

fn test_reactor() -> Reactor {
    Reactor::new(ReactorConfig {
        name: "test-reactor".to_string(),
        rules: vec![],
        max_events_per_minute: Some(100),
        buffer_size: Some(1000),
        persist_events: false,
        event_log_path: None,
    })
}

fn test_rule(id: &str) -> ReactorRule {
    ReactorRule {
        id: id.to_string(),
        event_type: ReactorEventType::Console,
        filter: Some(EventFilter {
            selector: None,
            url_pattern: None,
            message_pattern: Some("error".to_string()),
            event_subtype: None,
        }),
        handler: ReactorHandler::Log {
            format: None,
            output: None,
        },
        enabled: true,
        max_triggers: None,
        cooldown_ms: None,
        trigger_count: 0,
    }
}

// ────────────────────── Construction ──────────────────────

#[test]
fn e2e_reactor_config_construction() {
    let config = ReactorConfig {
        name: "my-reactor".to_string(),
        rules: vec![test_rule("r1")],
        max_events_per_minute: Some(60),
        buffer_size: Some(500),
        persist_events: false,
        event_log_path: None,
    };
    assert_eq!(config.name, "my-reactor");
    assert_eq!(config.rules.len(), 1);
}

#[test]
fn e2e_reactor_new() {
    let reactor = test_reactor();
    // Just verify construction succeeds (Reactor fields are private)
    let _ = reactor;
}

// ────────────────────── add_rule, remove_rule, toggle_rule ──────────────────────

#[tokio::test]
async fn e2e_reactor_add_rule() {
    let reactor = test_reactor();
    reactor.add_rule(test_rule("rule-1")).await.unwrap();

    let status = reactor.status().await;
    assert_eq!(status.rules_count, 1);
}

#[tokio::test]
async fn e2e_reactor_add_duplicate_rule_fails() {
    let reactor = test_reactor();
    reactor.add_rule(test_rule("dup")).await.unwrap();
    let result = reactor.add_rule(test_rule("dup")).await;
    assert!(result.is_err());
}

#[tokio::test]
async fn e2e_reactor_remove_rule() {
    let reactor = test_reactor();
    reactor.add_rule(test_rule("rm-me")).await.unwrap();
    reactor.remove_rule("rm-me").await.unwrap();

    let status = reactor.status().await;
    assert_eq!(status.rules_count, 0);
}

#[tokio::test]
async fn e2e_reactor_remove_nonexistent_fails() {
    let reactor = test_reactor();
    let result = reactor.remove_rule("ghost").await;
    assert!(result.is_err());
}

#[tokio::test]
async fn e2e_reactor_toggle_rule() {
    let reactor = test_reactor();
    reactor.add_rule(test_rule("tog")).await.unwrap();

    reactor.toggle_rule("tog", false).await.unwrap();
    let status = reactor.status().await;
    assert!(!status.rules[0].enabled);

    reactor.toggle_rule("tog", true).await.unwrap();
    let status = reactor.status().await;
    assert!(status.rules[0].enabled);
}

#[tokio::test]
async fn e2e_reactor_toggle_nonexistent_fails() {
    let reactor = test_reactor();
    let result = reactor.toggle_rule("nope", true).await;
    assert!(result.is_err());
}

// ────────────────────── status ──────────────────────

#[tokio::test]
async fn e2e_reactor_status() {
    let reactor = test_reactor();
    let status = reactor.status().await;
    assert_eq!(status.name, "test-reactor");
    assert!(!status.running);
    assert_eq!(status.rules_count, 0);
    assert_eq!(status.total_events, 0);
}

// ────────────────────── recent_events / clear_events ──────────────────────

#[tokio::test]
async fn e2e_reactor_recent_events_empty() {
    let reactor = test_reactor();
    let events = reactor.recent_events(10).await;
    assert!(events.is_empty());
}

#[tokio::test]
async fn e2e_reactor_clear_events() {
    let reactor = test_reactor();
    reactor.clear_events().await;
    let events = reactor.recent_events(10).await;
    assert!(events.is_empty());
}
