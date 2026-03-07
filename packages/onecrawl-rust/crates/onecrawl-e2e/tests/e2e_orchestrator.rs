//! E2E tests for the orchestrator (validation only, no real devices).
//! Tests config validation and file loading.

use onecrawl_cdp::orchestrator::{
    DeviceAction, DeviceConfig, DeviceType, ErrorPolicy, OrchAction, OrchStep, Orchestration,
    Orchestrator,
};
use std::collections::HashMap;
use tempfile::TempDir;

fn valid_orchestration() -> Orchestration {
    let mut devices = HashMap::new();
    devices.insert(
        "desktop".to_string(),
        DeviceConfig {
            id: "desktop".to_string(),
            device_type: DeviceType::Browser,
            headless: Some(true),
            user_data_dir: None,
            viewport: Some((1920, 1080)),
            adb_serial: None,
            appium_url: None,
            package_name: None,
            activity_name: None,
            udid: None,
            wda_url: None,
            bundle_id: None,
        },
    );

    Orchestration {
        name: "test-orchestration".to_string(),
        description: Some("E2E test".to_string()),
        devices,
        variables: Some(HashMap::new()),
        steps: vec![OrchStep {
            name: Some("Navigate".to_string()),
            actions: vec![DeviceAction {
                device: "desktop".to_string(),
                action: OrchAction::Navigate {
                    url: "https://example.com".to_string(),
                },
            }],
            condition: None,
            on_error: None,
            save_as: None,
            retry: None,
        }],
        on_error: Some(ErrorPolicy::Stop),
        timeout_secs: Some(60),
    }
}

// ────────────────────── validate: valid config ──────────────────────

#[test]
fn e2e_orchestrator_validate_valid() {
    let orch = valid_orchestration();
    Orchestrator::validate(&orch).unwrap();
}

// ────────────────────── validate: invalid configs ──────────────────────

#[test]
fn e2e_orchestrator_validate_empty_name() {
    let mut orch = valid_orchestration();
    orch.name = String::new();

    let result = Orchestrator::validate(&orch);
    assert!(result.is_err());
    let errors = result.unwrap_err();
    assert!(
        errors.iter().any(|e| e.contains("name")),
        "should report name error: {errors:?}"
    );
}

#[test]
fn e2e_orchestrator_validate_no_devices() {
    let mut orch = valid_orchestration();
    orch.devices.clear();

    let result = Orchestrator::validate(&orch);
    assert!(result.is_err());
    let errors = result.unwrap_err();
    assert!(
        errors.iter().any(|e| e.contains("device")),
        "should report device error: {errors:?}"
    );
}

#[test]
fn e2e_orchestrator_validate_no_steps() {
    let mut orch = valid_orchestration();
    orch.steps.clear();

    let result = Orchestrator::validate(&orch);
    assert!(result.is_err());
    let errors = result.unwrap_err();
    assert!(
        errors.iter().any(|e| e.contains("step")),
        "should report step error: {errors:?}"
    );
}

#[test]
fn e2e_orchestrator_validate_unknown_device_ref() {
    let mut orch = valid_orchestration();
    orch.steps = vec![OrchStep {
        name: Some("bad ref".to_string()),
        actions: vec![DeviceAction {
            device: "nonexistent-device".to_string(),
            action: OrchAction::Navigate {
                url: "https://example.com".to_string(),
            },
        }],
        condition: None,
        on_error: None,
        save_as: None,
        retry: None,
    }];

    let result = Orchestrator::validate(&orch);
    assert!(result.is_err());
    let errors = result.unwrap_err();
    assert!(
        errors.iter().any(|e| e.contains("nonexistent-device")),
        "should report unknown device: {errors:?}"
    );
}

// ────────────────────── from_file: JSON ──────────────────────

#[test]
fn e2e_orchestrator_from_file_json() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("orch.json");

    let orch = valid_orchestration();
    let json = serde_json::to_string_pretty(&orch).unwrap();
    std::fs::write(&path, json).unwrap();

    let loaded = Orchestrator::from_file(path.to_str().unwrap()).unwrap();
    assert_eq!(loaded.name, "test-orchestration");
    assert_eq!(loaded.devices.len(), 1);
    assert_eq!(loaded.steps.len(), 1);
}

#[test]
fn e2e_orchestrator_from_file_nonexistent() {
    let result = Orchestrator::from_file("/tmp/nonexistent-orch-12345.json");
    assert!(result.is_err());
}

// ────────────────────── Orchestrator::new ──────────────────────

#[test]
fn e2e_orchestrator_new() {
    let orch = valid_orchestration();
    let orchestrator = Orchestrator::new(orch);
    assert!(orchestrator.variables().is_empty());
}

// ────────────────────── multiple validation errors returned ──────────────────────

#[test]
fn e2e_orchestrator_validate_returns_all_errors() {
    let orch = Orchestration {
        name: String::new(),
        description: None,
        devices: HashMap::new(),
        variables: None,
        steps: vec![],
        on_error: None,
        timeout_secs: None,
    };

    let result = Orchestrator::validate(&orch);
    assert!(result.is_err());
    let errors = result.unwrap_err();
    // Should have at least 3 errors: name, devices, steps
    assert!(errors.len() >= 3, "expected >= 3 errors, got: {errors:?}");
}
