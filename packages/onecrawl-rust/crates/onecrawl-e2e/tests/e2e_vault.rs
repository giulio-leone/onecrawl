//! E2E tests for the vault (encrypted secrets store).
//! Tests CRUD, password management, and persistence.

use onecrawl_crypto::vault::Vault;
use tempfile::TempDir;

fn vault_path(dir: &TempDir) -> String {
    dir.path().join("test.vault").to_str().unwrap().to_string()
}

// ────────────────────── Create + Set + Get roundtrip ──────────────────────

#[test]
fn e2e_vault_set_get_roundtrip() {
    let dir = TempDir::new().unwrap();
    let mut vault = Vault::create(&vault_path(&dir), "master-pass").unwrap();
    vault.set("api_key", "sk-12345", None).unwrap();

    let entry = vault.get("api_key").unwrap();
    assert_eq!(entry.value, "sk-12345");
}

#[test]
fn e2e_vault_set_with_category() {
    let dir = TempDir::new().unwrap();
    let mut vault = Vault::create(&vault_path(&dir), "pass").unwrap();
    vault.set("token", "abc", Some("github")).unwrap();

    let entry = vault.get("token").unwrap();
    assert_eq!(entry.category.as_deref(), Some("github"));
}

// ────────────────────── list ──────────────────────

#[test]
fn e2e_vault_list_shows_keys() {
    let dir = TempDir::new().unwrap();
    let mut vault = Vault::create(&vault_path(&dir), "pass").unwrap();
    vault.set("key-a", "val-a", None).unwrap();
    vault.set("key-b", "val-b", None).unwrap();

    let summaries = vault.list();
    assert_eq!(summaries.len(), 2);
    let keys: Vec<_> = summaries.iter().map(|s| s.key.as_str()).collect();
    assert!(keys.contains(&"key-a"));
    assert!(keys.contains(&"key-b"));
}

// ────────────────────── delete ──────────────────────

#[test]
fn e2e_vault_delete_removes_key() {
    let dir = TempDir::new().unwrap();
    let mut vault = Vault::create(&vault_path(&dir), "pass").unwrap();
    vault.set("gone", "bye", None).unwrap();
    vault.delete("gone").unwrap();

    assert!(vault.get("gone").is_none());
    assert!(vault.list().is_empty());
}

#[test]
fn e2e_vault_delete_nonexistent_fails() {
    let dir = TempDir::new().unwrap();
    let mut vault = Vault::create(&vault_path(&dir), "pass").unwrap();
    let result = vault.delete("nope");
    assert!(result.is_err());
}

// ────────────────────── contains check (via get) ──────────────────────

#[test]
fn e2e_vault_contains_check() {
    let dir = TempDir::new().unwrap();
    let mut vault = Vault::create(&vault_path(&dir), "pass").unwrap();
    vault.set("present", "yes", None).unwrap();

    assert!(vault.get("present").is_some());
    assert!(vault.get("absent").is_none());
}

// ────────────────────── wrong password ──────────────────────

#[test]
fn e2e_vault_wrong_password_fails() {
    let dir = TempDir::new().unwrap();
    let path = vault_path(&dir);
    let mut vault = Vault::create(&path, "correct-pass").unwrap();
    vault.set("secret", "data", None).unwrap();
    vault.save().unwrap();
    drop(vault);

    let result = Vault::open(&path, "wrong-pass");
    assert!(result.is_err(), "wrong password should fail to open vault");
}

// ────────────────────── change_password ──────────────────────

#[test]
fn e2e_vault_change_password() {
    let dir = TempDir::new().unwrap();
    let path = vault_path(&dir);

    let mut vault = Vault::create(&path, "old-pass").unwrap();
    vault.set("secret", "data", None).unwrap();
    vault.change_password("new-pass").unwrap();
    vault.save().unwrap();
    drop(vault);

    // Old password should fail
    assert!(Vault::open(&path, "old-pass").is_err());

    // New password should work
    let vault2 = Vault::open(&path, "new-pass").unwrap();
    let entry = vault2.get("secret").unwrap();
    assert_eq!(entry.value, "data");
}

// ────────────────────── save persists data ──────────────────────

#[test]
fn e2e_vault_save_persists() {
    let dir = TempDir::new().unwrap();
    let path = vault_path(&dir);

    let mut vault = Vault::create(&path, "pass").unwrap();
    vault.set("persisted", "value", None).unwrap();
    vault.save().unwrap();
    drop(vault);

    // Re-open and verify
    let vault2 = Vault::open(&path, "pass").unwrap();
    let entry = vault2.get("persisted").unwrap();
    assert_eq!(entry.value, "value");
}

// ────────────────────── empty vault ──────────────────────

#[test]
fn e2e_vault_empty_on_creation() {
    let dir = TempDir::new().unwrap();
    let vault = Vault::create(&vault_path(&dir), "pass").unwrap();
    assert!(vault.is_empty());
    assert_eq!(vault.len(), 0);
}
