//! E2E tests for the studio / workflow builder.
//! Tests project CRUD, template listing, workflow export/import/validate.

use onecrawl_cdp::studio::{StudioProject, StudioWorkspace};
use tempfile::TempDir;

fn test_workspace() -> (TempDir, StudioWorkspace) {
    let dir = TempDir::new().unwrap();
    let ws = StudioWorkspace::new(dir.path().to_str().unwrap()).unwrap();
    (dir, ws)
}

fn test_project(id: &str, name: &str) -> StudioProject {
    StudioProject {
        id: id.to_string(),
        name: name.to_string(),
        description: Some("test project".to_string()),
        workflow: serde_json::json!({
            "steps": [
                { "action": { "type": "navigate", "url": "https://example.com" } }
            ]
        }),
        created_at: "2025-01-01T00:00:00Z".to_string(),
        updated_at: "2025-01-01T00:00:00Z".to_string(),
        last_run: None,
        run_count: 0,
    }
}

// ────────────────────── Construction ──────────────────────

#[test]
fn e2e_studio_workspace_new() {
    let (_dir, _ws) = test_workspace();
}

// ────────────────────── templates ──────────────────────

#[test]
fn e2e_studio_templates_non_empty() {
    let templates = StudioWorkspace::templates();
    assert!(!templates.is_empty());
    for t in &templates {
        assert!(!t.id.is_empty());
        assert!(!t.name.is_empty());
    }
}

// ────────────────────── save + load roundtrip ──────────────────────

#[test]
fn e2e_studio_save_load_roundtrip() {
    let (_dir, ws) = test_workspace();
    let project = test_project("roundtrip-1", "Roundtrip Test");

    ws.save_project(&project).unwrap();
    let loaded = ws.load_project("roundtrip-1").unwrap();

    assert_eq!(loaded.id, "roundtrip-1");
    assert_eq!(loaded.name, "Roundtrip Test");
    assert_eq!(loaded.workflow, project.workflow);
}

// ────────────────────── list_projects ──────────────────────

#[test]
fn e2e_studio_list_projects_after_save() {
    let (_dir, ws) = test_workspace();
    ws.save_project(&test_project("list-a", "Project A"))
        .unwrap();
    ws.save_project(&test_project("list-b", "Project B"))
        .unwrap();

    let projects = ws.list_projects().unwrap();
    assert_eq!(projects.len(), 2);
}

// ────────────────────── delete_project ──────────────────────

#[test]
fn e2e_studio_delete_project() {
    let (_dir, ws) = test_workspace();
    ws.save_project(&test_project("del-me", "Delete Me"))
        .unwrap();

    ws.delete_project("del-me").unwrap();

    let result = ws.load_project("del-me");
    assert!(result.is_err());
}

#[test]
fn e2e_studio_delete_nonexistent_fails() {
    let (_dir, ws) = test_workspace();
    let result = ws.delete_project("nope");
    assert!(result.is_err());
}

// ────────────────────── export_workflow ──────────────────────

#[test]
fn e2e_studio_export_workflow_valid_json() {
    let (_dir, ws) = test_workspace();
    ws.save_project(&test_project("export-1", "Export Test"))
        .unwrap();

    let json_str = ws.export_workflow("export-1").unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&json_str).unwrap();
    assert!(parsed.get("steps").is_some());
}

// ────────────────────── import_workflow ──────────────────────

#[test]
fn e2e_studio_import_workflow_creates_project() {
    let (_dir, ws) = test_workspace();
    let wf_json = r#"{"steps": [{"action": {"type": "navigate", "url": "https://example.com"}}]}"#;

    let project = ws.import_workflow("Imported Flow", wf_json).unwrap();
    assert!(!project.id.is_empty());

    let loaded = ws.load_project(&project.id).unwrap();
    assert_eq!(loaded.name, "Imported Flow");
}

// ────────────────────── validate_workflow ──────────────────────

#[test]
fn e2e_studio_validate_workflow_valid() {
    let wf = serde_json::json!({
        "steps": [
            { "action": { "type": "navigate", "url": "https://example.com" } }
        ]
    });
    let warnings = StudioWorkspace::validate_workflow(&wf).unwrap();
    assert!(warnings.is_empty(), "valid workflow should have no warnings");
}

#[test]
fn e2e_studio_validate_workflow_no_steps_key() {
    let wf = serde_json::json!({ "name": "bad" });
    let result = StudioWorkspace::validate_workflow(&wf);
    assert!(result.is_err(), "missing 'steps' key should error");
}

#[test]
fn e2e_studio_validate_workflow_empty_steps() {
    let wf = serde_json::json!({ "steps": [] });
    let warnings = StudioWorkspace::validate_workflow(&wf).unwrap();
    assert!(
        !warnings.is_empty(),
        "empty steps array should produce a warning"
    );
}

#[test]
fn e2e_studio_validate_workflow_step_no_action() {
    let wf = serde_json::json!({
        "steps": [{ "name": "bad step" }]
    });
    let warnings = StudioWorkspace::validate_workflow(&wf).unwrap();
    assert!(
        warnings.iter().any(|w| w.contains("action")),
        "step without action should warn"
    );
}

// ────────────────────── is_safe_id (tested indirectly) ──────────────────────

#[test]
fn e2e_studio_safe_id_rejects_path_traversal() {
    let (_dir, ws) = test_workspace();
    let project = test_project("../etc/passwd", "Evil");
    let result = ws.save_project(&project);
    assert!(result.is_err(), "path traversal ID should be rejected");
}

#[test]
fn e2e_studio_safe_id_rejects_dots() {
    let (_dir, ws) = test_workspace();
    let project = test_project("foo.bar", "Dotted");
    let result = ws.save_project(&project);
    assert!(result.is_err(), "dotted ID should be rejected");
}

#[test]
fn e2e_studio_safe_id_rejects_empty() {
    let (_dir, ws) = test_workspace();
    let project = test_project("", "Empty");
    let result = ws.save_project(&project);
    assert!(result.is_err(), "empty ID should be rejected");
}

#[test]
fn e2e_studio_safe_id_allows_valid() {
    let (_dir, ws) = test_workspace();
    let project = test_project("valid-id_123", "Valid");
    ws.save_project(&project).unwrap();
}
