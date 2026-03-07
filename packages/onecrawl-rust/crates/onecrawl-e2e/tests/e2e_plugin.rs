//! E2E tests for the plugin registry.
//! Tests plugin lifecycle: create, install, enable, disable, uninstall.

use onecrawl_cdp::plugin::{builtin_templates, default_plugins_dir, PluginRegistry, PluginStatus};
use tempfile::TempDir;

fn test_registry() -> (TempDir, PluginRegistry) {
    let dir = TempDir::new().unwrap();
    let registry = PluginRegistry::new(dir.path().to_str().unwrap()).unwrap();
    (dir, registry)
}

// ────────────────────── Construction ──────────────────────

#[test]
fn e2e_plugin_registry_new() {
    let (_dir, registry) = test_registry();
    assert!(registry.list().is_empty());
}

// ────────────────────── list on empty ──────────────────────

#[test]
fn e2e_plugin_list_empty() {
    let (_dir, registry) = test_registry();
    let plugins = registry.list();
    assert!(plugins.is_empty());
}

// ────────────────────── create_scaffold ──────────────────────

#[test]
fn e2e_plugin_create_scaffold() {
    let (_dir, registry) = test_registry();
    let scaffold_dir = TempDir::new().unwrap();
    let scaffold_path = scaffold_dir.path().join("my-plugin");
    registry
        .create_scaffold("my-plugin", scaffold_path.to_str().unwrap())
        .unwrap();

    assert!(scaffold_path.join("plugin.json").exists());
}

// ────────────────────── install_local from scaffold ──────────────────────

#[test]
fn e2e_plugin_install_local_from_scaffold() {
    let (_dir, mut registry) = test_registry();
    let scaffold_dir = TempDir::new().unwrap();
    let scaffold_path = scaffold_dir.path().join("test-plugin");
    registry
        .create_scaffold("test-plugin", scaffold_path.to_str().unwrap())
        .unwrap();

    let installed = registry
        .install_local(scaffold_path.to_str().unwrap())
        .unwrap();
    assert_eq!(installed.manifest.name, "test-plugin");

    let plugins = registry.list();
    assert_eq!(plugins.len(), 1);
}

// ────────────────────── enable / disable ──────────────────────

#[test]
fn e2e_plugin_enable_disable() {
    let (_dir, mut registry) = test_registry();
    let scaffold_dir = TempDir::new().unwrap();
    let scaffold_path = scaffold_dir.path().join("toggle-plugin");
    registry
        .create_scaffold("toggle-plugin", scaffold_path.to_str().unwrap())
        .unwrap();
    registry
        .install_local(scaffold_path.to_str().unwrap())
        .unwrap();

    // Enable
    registry.enable("toggle-plugin").unwrap();
    let p = registry.get("toggle-plugin").unwrap();
    assert_eq!(p.status, PluginStatus::Active);

    // Disable
    registry.disable("toggle-plugin").unwrap();
    let p = registry.get("toggle-plugin").unwrap();
    assert_eq!(p.status, PluginStatus::Disabled);
}

// ────────────────────── uninstall ──────────────────────

#[test]
fn e2e_plugin_uninstall() {
    let (_dir, mut registry) = test_registry();
    let scaffold_dir = TempDir::new().unwrap();
    let scaffold_path = scaffold_dir.path().join("rm-plugin");
    registry
        .create_scaffold("rm-plugin", scaffold_path.to_str().unwrap())
        .unwrap();
    registry
        .install_local(scaffold_path.to_str().unwrap())
        .unwrap();
    assert_eq!(registry.list().len(), 1);

    registry.uninstall("rm-plugin").unwrap();
    assert!(registry.list().is_empty());
}

#[test]
fn e2e_plugin_uninstall_nonexistent_fails() {
    let (_dir, mut registry) = test_registry();
    let result = registry.uninstall("ghost-plugin");
    assert!(result.is_err());
}

// ────────────────────── builtin_templates ──────────────────────

#[test]
fn e2e_builtin_templates_non_empty() {
    let templates = builtin_templates();
    assert!(!templates.is_empty(), "should have built-in templates");
    for (name, desc) in &templates {
        assert!(!name.is_empty());
        assert!(!desc.is_empty());
    }
}

// ────────────────────── default_plugins_dir ──────────────────────

#[test]
fn e2e_default_plugins_dir_valid() {
    let dir = default_plugins_dir();
    let s = dir.to_string_lossy();
    assert!(
        s.contains("onecrawl"),
        "expected path to contain 'onecrawl': {s}"
    );
}
