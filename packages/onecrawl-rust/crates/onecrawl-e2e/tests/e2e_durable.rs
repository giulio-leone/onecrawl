//! E2E tests for durable sessions.
//! Tests DurableConfig construction, session management, and duration parsing.

use onecrawl_cdp::durable::{DurableConfig, DurableSession, DurableStatus, parse_duration};
use std::time::Duration;
use tempfile::TempDir;

// ────────────────────── DurableConfig + DurableSession construction ──────────────────────

#[test]
fn e2e_durable_config_default() {
    let config = DurableConfig::default();
    assert_eq!(config.name, "default");
    assert!(config.auto_reconnect);
    assert!(config.checkpoint_interval_secs > 0);
    assert!(config.max_reconnect_attempts > 0);
}

#[test]
fn e2e_durable_session_new() {
    let dir = TempDir::new().unwrap();
    let config = DurableConfig {
        name: "test-session".to_string(),
        state_path: dir.path().to_path_buf(),
        ..DurableConfig::default()
    };
    let session = DurableSession::new(config).unwrap();
    assert_eq!(session.config.name, "test-session");
    assert!(matches!(session.state.status, DurableStatus::Stopped));
}

// ────────────────────── list_sessions ──────────────────────

#[test]
fn e2e_durable_list_sessions_empty() {
    let dir = TempDir::new().unwrap();
    let sessions = DurableSession::list_sessions(dir.path()).unwrap();
    assert!(sessions.is_empty(), "expected empty list for fresh dir");
}

// ────────────────────── delete_session / get_status on nonexistent ──────────────────────

#[test]
fn e2e_durable_delete_nonexistent_is_noop() {
    let dir = TempDir::new().unwrap();
    // delete_session is a no-op for nonexistent names (not an error)
    DurableSession::delete_session(dir.path(), "no-such-session").unwrap();
}

#[test]
fn e2e_durable_get_status_nonexistent() {
    let dir = TempDir::new().unwrap();
    let result = DurableSession::get_status(dir.path(), "no-such-session");
    assert!(result.is_err());
}

// ────────────────────── parse_duration ──────────────────────

#[test]
fn e2e_parse_duration_seconds() {
    let d = parse_duration("10s").unwrap();
    assert_eq!(d, Duration::from_secs(10));
}

#[test]
fn e2e_parse_duration_minutes() {
    let d = parse_duration("5m").unwrap();
    assert_eq!(d, Duration::from_secs(300));
}

#[test]
fn e2e_parse_duration_hours() {
    let d = parse_duration("1h").unwrap();
    assert_eq!(d, Duration::from_secs(3600));
}

#[test]
fn e2e_parse_duration_milliseconds() {
    let d = parse_duration("500ms").unwrap();
    assert_eq!(d, Duration::from_millis(500));
}

#[test]
fn e2e_parse_duration_days() {
    let d = parse_duration("7d").unwrap();
    assert_eq!(d, Duration::from_secs(7 * 86400));
}

#[test]
fn e2e_parse_duration_bare_number_defaults_to_seconds() {
    let d = parse_duration("30").unwrap();
    assert_eq!(d, Duration::from_secs(30));
}

// ────────────────────── default_state_dir ──────────────────────

#[test]
fn e2e_default_state_dir_valid() {
    let dir = DurableSession::default_state_dir();
    let s = dir.to_string_lossy();
    assert!(s.contains("onecrawl"), "expected path to contain 'onecrawl': {s}");
}
