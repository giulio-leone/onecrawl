//! Encrypted credential vault with AES-256-GCM encryption.
//!
//! Stores secrets at rest using PBKDF2 key derivation + AES-256-GCM.
//! Designed for browser automation credential management.

use onecrawl_core::{EncryptedPayload, Error, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;

// ──────────────────────────── Types ────────────────────────────

/// A single secret entry stored in the vault.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VaultEntry {
    pub key: String,
    pub value: String,
    pub category: Option<String>,
    pub created_at: String,
    pub updated_at: String,
    pub expires_at: Option<String>,
    pub metadata: Option<HashMap<String, String>>,
}

/// Service credential template describing required fields.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServiceTemplate {
    pub service: String,
    pub fields: Vec<String>,
}

/// Summary of a vault entry (no secret values exposed).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VaultEntrySummary {
    pub key: String,
    pub category: Option<String>,
    pub created_at: String,
    pub updated_at: String,
    pub has_expiry: bool,
    pub expired: bool,
}

/// Encrypted vault file format persisted to disk.
#[derive(Debug, Clone, Serialize, Deserialize)]
struct VaultFile {
    version: u32,
    payload: EncryptedPayload,
}

// ──────────────────────────── Vault ────────────────────────────

/// Encrypted credential vault backed by AES-256-GCM.
pub struct Vault {
    path: PathBuf,
    entries: HashMap<String, VaultEntry>,
    passphrase: String,
    dirty: bool,
}

impl Vault {
    /// Create a new vault with a master password.
    pub fn create(path: &str, password: &str) -> Result<Self> {
        if password.is_empty() {
            return Err(Error::Config("vault password cannot be empty".into()));
        }

        let vault_path = PathBuf::from(path);

        let vault = Self {
            path: vault_path,
            entries: HashMap::new(),
            passphrase: password.to_string(),
            dirty: true,
        };

        vault.save()?;
        Ok(vault)
    }

    /// Open an existing vault with password.
    pub fn open(path: &str, password: &str) -> Result<Self> {
        if password.is_empty() {
            return Err(Error::Config("vault password cannot be empty".into()));
        }

        let vault_path = PathBuf::from(path);
        let data = std::fs::read_to_string(&vault_path)
            .map_err(|e| Error::Crypto(format!("failed to read vault: {e}")))?;

        let vault_file: VaultFile = serde_json::from_str(&data)
            .map_err(|e| Error::Crypto(format!("invalid vault file: {e}")))?;

        if vault_file.version != 1 {
            return Err(Error::Crypto(format!(
                "unsupported vault version: {}",
                vault_file.version
            )));
        }

        let plaintext = crate::aes_gcm::decrypt(&vault_file.payload, password)?;
        let json_str = String::from_utf8(plaintext)
            .map_err(|e| Error::Crypto(format!("invalid vault data: {e}")))?;
        let entries: HashMap<String, VaultEntry> = serde_json::from_str(&json_str)
            .map_err(|e| Error::Crypto(format!("invalid vault entries: {e}")))?;

        Ok(Self {
            path: vault_path,
            entries,
            passphrase: password.to_string(),
            dirty: false,
        })
    }

    /// Set a secret value, creating or updating the entry.
    pub fn set(&mut self, key: &str, value: &str, category: Option<&str>) -> Result<()> {
        if key.is_empty() {
            return Err(Error::Config("vault key cannot be empty".into()));
        }

        let now = now_iso8601();

        if let Some(existing) = self.entries.get_mut(key) {
            existing.value = value.to_string();
            existing.updated_at = now;
            if let Some(cat) = category {
                existing.category = Some(cat.to_string());
            }
        } else {
            let entry = VaultEntry {
                key: key.to_string(),
                value: value.to_string(),
                category: category.map(|s| s.to_string()),
                created_at: now.clone(),
                updated_at: now,
                expires_at: None,
                metadata: None,
            };
            self.entries.insert(key.to_string(), entry);
        }

        self.dirty = true;
        self.save()
    }

    /// Get a secret entry by key.
    pub fn get(&self, key: &str) -> Option<&VaultEntry> {
        self.entries.get(key)
    }

    /// Delete a secret by key.
    pub fn delete(&mut self, key: &str) -> Result<()> {
        if self.entries.remove(key).is_none() {
            return Err(Error::Crypto(format!("key '{key}' not found in vault")));
        }
        self.dirty = true;
        self.save()
    }

    /// List all entries as summaries (no secret values).
    pub fn list(&self) -> Vec<VaultEntrySummary> {
        let now = now_iso8601();
        self.entries
            .values()
            .map(|e| VaultEntrySummary {
                key: e.key.clone(),
                category: e.category.clone(),
                created_at: e.created_at.clone(),
                updated_at: e.updated_at.clone(),
                has_expiry: e.expires_at.is_some(),
                expired: e
                    .expires_at
                    .as_ref()
                    .map(|exp| exp.as_str() < now.as_str())
                    .unwrap_or(false),
            })
            .collect()
    }

    /// List entries filtered by category/service.
    pub fn list_by_category(&self, category: &str) -> Vec<VaultEntrySummary> {
        self.list()
            .into_iter()
            .filter(|s| s.category.as_deref() == Some(category))
            .collect()
    }

    /// Export entries for a service as a key→value map for workflow variables.
    pub fn export_for_workflow(&self, service: &str) -> HashMap<String, String> {
        self.entries
            .iter()
            .filter(|(_, e)| e.category.as_deref() == Some(service))
            .map(|(k, e)| (k.clone(), e.value.clone()))
            .collect()
    }

    /// Change the master password. Re-encrypts all data.
    pub fn change_password(&mut self, new_password: &str) -> Result<()> {
        if new_password.is_empty() {
            return Err(Error::Config("new password cannot be empty".into()));
        }
        self.passphrase = new_password.to_string();
        self.dirty = true;
        self.save()
    }

    /// Save vault to disk (encrypt and write).
    pub fn save(&self) -> Result<()> {
        let json = serde_json::to_string(&self.entries)
            .map_err(|e| Error::Crypto(format!("serialize failed: {e}")))?;

        let payload = crate::aes_gcm::encrypt(json.as_bytes(), &self.passphrase)?;

        let vault_file = VaultFile {
            version: 1,
            payload,
        };

        let output = serde_json::to_string_pretty(&vault_file)
            .map_err(|e| Error::Crypto(format!("serialize vault file: {e}")))?;

        if let Some(parent) = self.path.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|e| Error::Crypto(format!("create vault dir: {e}")))?;
        }

        std::fs::write(&self.path, output)
            .map_err(|e| Error::Crypto(format!("write vault: {e}")))?;

        Ok(())
    }

    /// Return keys of expired entries.
    pub fn check_expired(&self) -> Vec<String> {
        let now = now_iso8601();
        self.entries
            .values()
            .filter(|e| {
                e.expires_at
                    .as_ref()
                    .map(|exp| exp.as_str() < now.as_str())
                    .unwrap_or(false)
            })
            .map(|e| e.key.clone())
            .collect()
    }

    /// Import secrets from environment variables matching a prefix.
    ///
    /// E.g., prefix `"ONECRAWL_VAULT_"` imports `ONECRAWL_VAULT_GITHUB_TOKEN` as
    /// key `"github_token"` (lowercased, prefix stripped).
    pub fn import_env(&mut self, prefix: &str) -> Result<usize> {
        let mut count = 0usize;
        for (k, v) in std::env::vars() {
            if let Some(suffix) = k.strip_prefix(prefix) {
                let key = suffix.to_lowercase();
                if !key.is_empty() {
                    self.set(&key, &v, None)?;
                    count += 1;
                }
            }
        }
        Ok(count)
    }

    /// Return the vault file path.
    pub fn path(&self) -> &std::path::Path {
        &self.path
    }

    /// Return the number of entries.
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Check if the vault is empty.
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
}

// ──────────────────────── Service Templates ────────────────────

/// Pre-defined service templates for common credential shapes.
pub fn service_templates() -> Vec<ServiceTemplate> {
    vec![
        ServiceTemplate {
            service: "linkedin".into(),
            fields: vec!["email".into(), "password".into()],
        },
        ServiceTemplate {
            service: "github".into(),
            fields: vec!["username".into(), "token".into()],
        },
        ServiceTemplate {
            service: "google".into(),
            fields: vec![
                "email".into(),
                "password".into(),
                "2fa_secret".into(),
            ],
        },
        ServiceTemplate {
            service: "twitter".into(),
            fields: vec!["username".into(), "password".into()],
        },
        ServiceTemplate {
            service: "aws".into(),
            fields: vec![
                "access_key_id".into(),
                "secret_access_key".into(),
                "region".into(),
            ],
        },
    ]
}

/// Return the default vault file path (`~/.onecrawl/vault.enc`).
pub fn default_vault_path() -> PathBuf {
    std::env::var("HOME")
        .map(|h| PathBuf::from(h).join(".onecrawl/vault.enc"))
        .unwrap_or_else(|_| PathBuf::from("vault.enc"))
}

// ──────────────────────────── Helpers ───────────────────────────

fn now_iso8601() -> String {
    // Simple UTC timestamp without chrono dependency.
    let d = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default();
    let secs = d.as_secs();
    // Approximate ISO-8601: good enough for ordering/display.
    let days = secs / 86400;
    let time_secs = secs % 86400;
    let hours = time_secs / 3600;
    let mins = (time_secs % 3600) / 60;
    let s = time_secs % 60;

    // Days since epoch → year/month/day (simplified leap-year-aware)
    let (year, month, day) = days_to_ymd(days);
    format!("{year:04}-{month:02}-{day:02}T{hours:02}:{mins:02}:{s:02}Z")
}

fn days_to_ymd(mut days: u64) -> (u64, u64, u64) {
    let mut year = 1970u64;
    loop {
        let days_in_year = if is_leap(year) { 366 } else { 365 };
        if days < days_in_year {
            break;
        }
        days -= days_in_year;
        year += 1;
    }
    let leap = is_leap(year);
    let months: [u64; 12] = [
        31,
        if leap { 29 } else { 28 },
        31, 30, 31, 30, 31, 31, 30, 31, 30, 31,
    ];
    let mut month = 0u64;
    for m_days in &months {
        if days < *m_days {
            break;
        }
        days -= *m_days;
        month += 1;
    }
    (year, month + 1, days + 1)
}

fn is_leap(y: u64) -> bool {
    (y % 4 == 0 && y % 100 != 0) || y % 400 == 0
}

// ──────────────────────────── Tests ────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn tmp_vault_path() -> String {
        let id: u64 = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos() as u64;
        format!("/tmp/onecrawl_test_vault_{id}.enc")
    }

    fn cleanup(path: &str) {
        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn create_and_open_roundtrip() {
        let path = tmp_vault_path();
        let password = "test-pass-2024";

        let mut vault = Vault::create(&path, password).expect("create");
        vault.set("github.token", "ghp_abc123", Some("github")).expect("set");
        vault.set("linkedin.email", "user@test.com", Some("linkedin")).expect("set");

        drop(vault);

        let vault2 = Vault::open(&path, password).expect("open");
        let entry = vault2.get("github.token").expect("get");
        assert_eq!(entry.value, "ghp_abc123");
        assert_eq!(entry.category.as_deref(), Some("github"));

        let entry2 = vault2.get("linkedin.email").expect("get");
        assert_eq!(entry2.value, "user@test.com");

        cleanup(&path);
    }

    #[test]
    fn wrong_password_fails() {
        let path = tmp_vault_path();
        Vault::create(&path, "correct").expect("create");
        let result = Vault::open(&path, "wrong");
        assert!(result.is_err());
        cleanup(&path);
    }

    #[test]
    fn delete_entry() {
        let path = tmp_vault_path();
        let mut vault = Vault::create(&path, "pass").expect("create");
        vault.set("key1", "val1", None).expect("set");
        vault.delete("key1").expect("delete");
        assert!(vault.get("key1").is_none());
        assert!(vault.delete("nonexistent").is_err());
        cleanup(&path);
    }

    #[test]
    fn list_without_values() {
        let path = tmp_vault_path();
        let mut vault = Vault::create(&path, "pass").expect("create");
        vault.set("a", "secret_a", Some("svc")).expect("set");
        vault.set("b", "secret_b", None).expect("set");

        let list = vault.list();
        assert_eq!(list.len(), 2);
        // Summaries must not contain the actual secret values
        for summary in &list {
            assert!(summary.key == "a" || summary.key == "b");
        }

        let by_cat = vault.list_by_category("svc");
        assert_eq!(by_cat.len(), 1);
        assert_eq!(by_cat[0].key, "a");

        cleanup(&path);
    }

    #[test]
    fn export_for_workflow() {
        let path = tmp_vault_path();
        let mut vault = Vault::create(&path, "pass").expect("create");
        vault.set("github.token", "tok", Some("github")).expect("set");
        vault.set("github.user", "usr", Some("github")).expect("set");
        vault.set("other.key", "val", Some("other")).expect("set");

        let exported = vault.export_for_workflow("github");
        assert_eq!(exported.len(), 2);
        assert_eq!(exported.get("github.token").map(|s| s.as_str()), Some("tok"));

        cleanup(&path);
    }

    #[test]
    fn change_password() {
        let path = tmp_vault_path();
        let mut vault = Vault::create(&path, "old").expect("create");
        vault.set("k", "v", None).expect("set");
        vault.change_password("new").expect("change");

        drop(vault);

        assert!(Vault::open(&path, "old").is_err());

        let vault2 = Vault::open(&path, "new").expect("open with new pass");
        assert_eq!(vault2.get("k").expect("get").value, "v");

        cleanup(&path);
    }

    #[test]
    fn empty_password_rejected() {
        let path = tmp_vault_path();
        assert!(Vault::create(&path, "").is_err());
        cleanup(&path);
    }

    #[test]
    fn service_templates_non_empty() {
        let templates = service_templates();
        assert!(templates.len() >= 3);
        assert!(templates.iter().any(|t| t.service == "linkedin"));
        assert!(templates.iter().any(|t| t.service == "github"));
    }

    #[test]
    fn update_existing_entry() {
        let path = tmp_vault_path();
        let mut vault = Vault::create(&path, "pass").expect("create");
        vault.set("k", "v1", None).expect("set");
        vault.set("k", "v2", Some("cat")).expect("update");

        let entry = vault.get("k").expect("get");
        assert_eq!(entry.value, "v2");
        assert_eq!(entry.category.as_deref(), Some("cat"));
        assert_eq!(vault.len(), 1);

        cleanup(&path);
    }
}
