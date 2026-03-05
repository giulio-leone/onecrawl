//! Passkey vault — multi-site credential storage and import utilities.
//!
//! Supports importing passkeys from:
//! - **Bitwarden** unencrypted JSON export (`bw export --format json`)
//! - **1Password** `.1pux` format (ZIP containing `export.data`)
//! - **FIDO Alliance CXF** credential-exchange-format JSON (v1.0 draft)
//! - **Onecrawl native** JSON (produced by `auth passkey-register`)
//!
//! The vault is stored at `~/.onecrawl/passkeys/vault.json` and is keyed by
//! `rp_id` (relying-party domain, e.g. `"x.com"`).

use std::{
    collections::HashMap,
    path::PathBuf,
};

use serde::{Deserialize, Serialize};

use crate::webauthn::PasskeyCredential;
use onecrawl_core::Result;

// ─── Vault ────────────────────────────────────────────────────────────────────

/// Multi-site passkey vault keyed by `rp_id`.
///
/// Stored at `~/.onecrawl/passkeys/vault.json`.
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct PasskeyVault {
    /// `rp_id → credentials` mapping.
    pub credentials: HashMap<String, Vec<PasskeyCredential>>,
}

/// Returns `~/.onecrawl/passkeys/vault.json`.
pub fn vault_path() -> PathBuf {
    onecrawl_dir().join("passkeys").join("vault.json")
}

fn onecrawl_dir() -> PathBuf {
    let home = std::env::var("HOME").unwrap_or_else(|_| "/tmp".into());
    PathBuf::from(home).join(".onecrawl")
}

/// Load vault from disk. Returns an empty vault if the file doesn't exist.
pub fn load_vault() -> Result<PasskeyVault> {
    let path = vault_path();
    if !path.exists() {
        return Ok(PasskeyVault::default());
    }
    let json = std::fs::read_to_string(&path)
        .map_err(|e| onecrawl_core::Error::Cdp(format!("read vault: {e}")))?;
    serde_json::from_str(&json)
        .map_err(|e| onecrawl_core::Error::Cdp(format!("parse vault: {e}")))
}

/// Persist vault to disk (creates parent directories automatically).
pub fn save_vault(vault: &PasskeyVault) -> Result<()> {
    let path = vault_path();
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|e| onecrawl_core::Error::Cdp(format!("create vault dir: {e}")))?;
    }
    let json = serde_json::to_string_pretty(vault)
        .map_err(|e| onecrawl_core::Error::Cdp(format!("serialize vault: {e}")))?;
    std::fs::write(&path, json)
        .map_err(|e| onecrawl_core::Error::Cdp(format!("write vault: {e}")))
}

/// Add credentials to the vault, deduplicating by `credential_id`.
pub fn vault_add(vault: &mut PasskeyVault, credentials: Vec<PasskeyCredential>) {
    for cred in credentials {
        let entry = vault.credentials.entry(cred.rp_id.clone()).or_default();
        if !entry.iter().any(|c| c.credential_id == cred.credential_id) {
            entry.push(cred);
        }
    }
}

/// Get all credentials for a specific `rp_id`.
pub fn vault_get(vault: &PasskeyVault, rp_id: &str) -> Vec<PasskeyCredential> {
    vault.credentials.get(rp_id).cloned().unwrap_or_default()
}

/// Remove a credential by `credential_id`. Returns `true` if a credential was removed.
pub fn vault_remove(vault: &mut PasskeyVault, credential_id: &str) -> bool {
    let mut removed = false;
    for creds in vault.credentials.values_mut() {
        let before = creds.len();
        creds.retain(|c| c.credential_id != credential_id);
        if creds.len() < before {
            removed = true;
        }
    }
    vault.credentials.retain(|_, v| !v.is_empty());
    removed
}

/// Remove all credentials for a specific `rp_id`. Returns number removed.
pub fn vault_clear_site(vault: &mut PasskeyVault, rp_id: &str) -> usize {
    vault.credentials.remove(rp_id).map(|v| v.len()).unwrap_or(0)
}

/// List all `rp_id`s in the vault with credential counts, sorted alphabetically.
pub fn vault_list(vault: &PasskeyVault) -> Vec<(String, usize)> {
    let mut list: Vec<(String, usize)> = vault
        .credentials
        .iter()
        .map(|(k, v)| (k.clone(), v.len()))
        .collect();
    list.sort_by(|a, b| a.0.cmp(&b.0));
    list
}

/// Total number of credentials across all sites.
pub fn vault_total(vault: &PasskeyVault) -> usize {
    vault.credentials.values().map(|v| v.len()).sum()
}
