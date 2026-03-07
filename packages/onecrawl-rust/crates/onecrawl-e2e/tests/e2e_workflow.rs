//! E2E tests for the workflow engine.
//! Tests validation, JSON parsing, and file loading.

use onecrawl_cdp::workflow::{self, Action, Step, Workflow};
use tempfile::TempDir;

fn valid_workflow() -> Workflow {
    Workflow {
        name: "test-workflow".to_string(),
        description: "A test workflow".to_string(),
        version: "1.0.0".to_string(),
        variables: Default::default(),
        steps: vec![Step {
            id: "step-1".to_string(),
            name: "Navigate".to_string(),
            action: Action::Navigate {
                url: "https://example.com".to_string(),
            },
            condition: None,
            retries: 0,
            retry_delay_ms: 0,
            timeout_ms: None,
            on_error: None,
            save_as: None,
        }],
        on_error: Default::default(),
    }
}

fn invalid_workflow() -> Workflow {
    Workflow {
        name: String::new(), // empty name
        description: String::new(),
        version: String::new(),
        variables: Default::default(),
        steps: vec![], // no steps
        on_error: Default::default(),
    }
}

// ────────────────────── validate: valid workflow ──────────────────────

#[test]
fn e2e_workflow_validate_valid() {
    let wf = valid_workflow();
    let errors = workflow::validate(&wf);
    assert!(errors.is_empty(), "expected no errors, got: {errors:?}");
}

// ────────────────────── validate: invalid workflow ──────────────────────

#[test]
fn e2e_workflow_validate_empty_name() {
    let wf = invalid_workflow();
    let errors = workflow::validate(&wf);
    assert!(!errors.is_empty(), "expected validation errors");
    assert!(
        errors.iter().any(|e| e.contains("name")),
        "should report name error: {errors:?}"
    );
}

#[test]
fn e2e_workflow_validate_no_steps() {
    let wf = Workflow {
        name: "has-name".to_string(),
        steps: vec![],
        ..invalid_workflow()
    };
    let errors = workflow::validate(&wf);
    assert!(
        errors.iter().any(|e| e.contains("step")),
        "should report missing steps: {errors:?}"
    );
}

#[test]
fn e2e_workflow_validate_empty_url() {
    let wf = Workflow {
        name: "bad-url".to_string(),
        steps: vec![Step {
            id: "s1".to_string(),
            name: "bad nav".to_string(),
            action: Action::Navigate {
                url: String::new(),
            },
            condition: None,
            retries: 0,
            retry_delay_ms: 0,
            timeout_ms: None,
            on_error: None,
            save_as: None,
        }],
        ..invalid_workflow()
    };
    let errors = workflow::validate(&wf);
    assert!(
        errors.iter().any(|e| e.contains("url")),
        "should report empty url: {errors:?}"
    );
}

// ────────────────────── load_from_file ──────────────────────

#[test]
fn e2e_workflow_load_from_file_json() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("workflow.json");
    let json = serde_json::json!({
        "name": "file-workflow",
        "steps": [
            { "action": { "type": "navigate", "url": "https://example.com" } }
        ]
    });
    std::fs::write(&path, serde_json::to_string_pretty(&json).unwrap()).unwrap();

    let wf = workflow::load_from_file(path.to_str().unwrap()).unwrap();
    assert_eq!(wf.name, "file-workflow");
    assert_eq!(wf.steps.len(), 1);
}

#[test]
fn e2e_workflow_load_from_file_nonexistent() {
    let result = workflow::load_from_file("/tmp/nonexistent-workflow-12345.json");
    assert!(result.is_err());
}

// ────────────────────── parse_json roundtrip ──────────────────────

#[test]
fn e2e_workflow_parse_json_roundtrip() {
    let json = r##"{
        "name": "parsed",
        "steps": [
            { "action": { "type": "click", "selector": "#btn" } },
            { "action": { "type": "screenshot" } }
        ]
    }"##;

    let wf = workflow::parse_json(json).unwrap();
    assert_eq!(wf.name, "parsed");
    assert_eq!(wf.steps.len(), 2);
}

#[test]
fn e2e_workflow_parse_json_invalid() {
    let result = workflow::parse_json("{ not valid json }}}");
    assert!(result.is_err());
}

// ────────────────────── interpolate ──────────────────────

#[test]
fn e2e_workflow_interpolate() {
    let mut vars = std::collections::HashMap::new();
    vars.insert("name".to_string(), serde_json::json!("Alice"));
    vars.insert("count".to_string(), serde_json::json!(42));

    let result = workflow::interpolate("Hello {{name}}, count={{count}}", &vars);
    assert!(result.contains("Alice"));
    assert!(result.contains("42"));
}

// ────────────────────── evaluate_condition ──────────────────────

#[test]
fn e2e_workflow_evaluate_condition() {
    let mut vars = std::collections::HashMap::new();
    vars.insert("status".to_string(), serde_json::json!("ok"));
    vars.insert("retries".to_string(), serde_json::json!(3));

    assert!(workflow::evaluate_condition(r#"status == "ok""#, &vars));
    assert!(!workflow::evaluate_condition(r#"status == "fail""#, &vars));
    assert!(workflow::evaluate_condition("retries > 2", &vars));
    assert!(!workflow::evaluate_condition("retries > 5", &vars));
}
