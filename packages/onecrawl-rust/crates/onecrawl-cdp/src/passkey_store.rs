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
    path::{Path, PathBuf},
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

// ─── Bitwarden import ─────────────────────────────────────────────────────────

/// Import passkeys from a **Bitwarden** unencrypted JSON export.
///
/// Generate the export via: `bw export --format json --output export.json`
///
/// Parses: `items[].login.fido2Credentials[]` entries where
/// `keyValue` contains the PKCS#8 private key (base64).
///
/// # Note on Apple/Windows platform passkeys
/// Passkeys registered with Apple Touch ID or Windows Hello store the private
/// key in hardware (Secure Enclave / TPM) and are marked as non-exportable.
/// Bitwarden cannot include the private key for those credentials. Only
/// passkeys natively stored in Bitwarden (software-backed) are importable.
pub fn import_bitwarden(path: &Path) -> Result<Vec<PasskeyCredential>> {
    let json = std::fs::read_to_string(path)
        .map_err(|e| onecrawl_core::Error::Cdp(format!("read bitwarden export: {e}")))?;
    let root: BitwardenExport = serde_json::from_str(&json)
        .map_err(|e| onecrawl_core::Error::Cdp(format!("parse bitwarden JSON: {e}")))?;

    let mut out = Vec::new();
    for item in root.items {
        if let Some(login) = item.login {
            for cred in login.fido2_credentials.unwrap_or_default() {
                // Skip credentials without a private key (hardware-bound)
                let Some(key_value) = cred.key_value else {
                    continue;
                };
                let sign_count = cred
                    .counter
                    .as_deref()
                    .and_then(|s| s.parse::<i64>().ok())
                    .unwrap_or(0);
                let is_resident = cred
                    .discoverable
                    .as_deref()
                    .map(|s| s == "true")
                    .unwrap_or(true);
                out.push(PasskeyCredential {
                    credential_id: cred.credential_id,
                    private_key: key_value,
                    rp_id: cred.rp_id,
                    user_handle: cred.user_handle,
                    sign_count,
                    is_resident_credential: is_resident,
                });
            }
        }
    }
    Ok(out)
}

// ─── Bitwarden JSON serde models ──────────────────────────────────────────────

#[derive(Deserialize)]
struct BitwardenExport {
    items: Vec<BitwardenItem>,
}

#[derive(Deserialize)]
struct BitwardenItem {
    login: Option<BitwardenLogin>,
}

#[derive(Deserialize)]
struct BitwardenLogin {
    #[serde(rename = "fido2Credentials")]
    fido2_credentials: Option<Vec<BitwardenFido2Credential>>,
}

#[derive(Deserialize)]
struct BitwardenFido2Credential {
    #[serde(rename = "credentialId")]
    credential_id: String,
    /// PKCS#8 private key, base64-encoded. Absent for hardware-bound credentials.
    #[serde(rename = "keyValue")]
    key_value: Option<String>,
    #[serde(rename = "rpId")]
    rp_id: String,
    #[serde(rename = "userHandle")]
    user_handle: Option<String>,
    /// Signature counter as string (Bitwarden stores it as string).
    counter: Option<String>,
    /// Whether the credential is a resident/discoverable credential.
    discoverable: Option<String>,
}

// ─── 1Password .1pux import ───────────────────────────────────────────────────

/// Import passkeys from a **1Password** `.1pux` export (a ZIP archive).
///
/// Generate via: `1Password > File > Export > 1PUX format`
/// Or via CLI: `op export --output export.1pux`
///
/// The ZIP contains `export.data` (JSON). Passkeys are items with
/// `categoryUuid = "119"` and have `loginFields` containing `credentialId`,
/// `keyValue`, `rpId`, `userHandle`, and `counter`.
///
/// # Dependency note
/// Requires the `zip` crate. Without it, extract the ZIP manually and call
/// `import_1password_json()` with the extracted `export.data` path instead.
#[cfg(feature = "zip")]
pub fn import_1password(path: &Path) -> Result<Vec<PasskeyCredential>> {
    use std::io::Read;
    let file = std::fs::File::open(path)
        .map_err(|e| onecrawl_core::Error::Cdp(format!("open 1pux: {e}")))?;
    let mut archive = zip::ZipArchive::new(file)
        .map_err(|e| onecrawl_core::Error::Cdp(format!("open 1pux zip: {e}")))?;
    let mut entry = archive
        .by_name("export.data")
        .map_err(|e| onecrawl_core::Error::Cdp(format!("export.data in 1pux: {e}")))?;
    let mut json = String::new();
    entry
        .read_to_string(&mut json)
        .map_err(|e| onecrawl_core::Error::Cdp(format!("read export.data: {e}")))?;
    import_1password_json_str(&json)
}

/// Import passkeys from a **1Password** `export.data` JSON string (extracted from `.1pux`).
pub fn import_1password_json(path: &Path) -> Result<Vec<PasskeyCredential>> {
    let json = std::fs::read_to_string(path)
        .map_err(|e| onecrawl_core::Error::Cdp(format!("read 1password json: {e}")))?;
    import_1password_json_str(&json)
}

fn import_1password_json_str(json: &str) -> Result<Vec<PasskeyCredential>> {
    let root: OnePuxExport = serde_json::from_str(json)
        .map_err(|e| onecrawl_core::Error::Cdp(format!("parse 1password JSON: {e}")))?;

    let mut out = Vec::new();
    for account in root.accounts {
        for vault in account.vaults {
            for item in vault.items {
                // 1Password passkey category UUID is "119"
                if item.overview.category_uuid.as_deref() != Some("119") {
                    // Also check by login fields if category is missing
                    if !has_passkey_fields(&item) {
                        continue;
                    }
                }
                if let Some(cred) = extract_1password_passkey(&item) {
                    out.push(cred);
                }
            }
        }
    }
    Ok(out)
}

fn has_passkey_fields(item: &OnePuxItem) -> bool {
    item.details
        .login_fields
        .iter()
        .any(|f| f.name.as_deref() == Some("credentialId"))
}

fn extract_1password_passkey(item: &OnePuxItem) -> Option<PasskeyCredential> {
    let fields = &item.details.login_fields;
    let get = |name: &str| -> Option<String> {
        fields
            .iter()
            .find(|f| f.name.as_deref() == Some(name))
            .and_then(|f| f.value.clone())
    };
    let credential_id = get("credentialId")?;
    let private_key = get("keyValue").or_else(|| get("privateKey"))?;
    let rp_id = get("rpId")?;
    let user_handle = get("userHandle");
    let sign_count = get("counter")
        .and_then(|s| s.parse::<i64>().ok())
        .unwrap_or(0);
    Some(PasskeyCredential {
        credential_id,
        private_key,
        rp_id,
        user_handle,
        sign_count,
        is_resident_credential: true,
    })
}

// ─── 1Password .1pux serde models ─────────────────────────────────────────────

#[derive(Deserialize)]
struct OnePuxExport {
    accounts: Vec<OnePuxAccount>,
}

#[derive(Deserialize)]
struct OnePuxAccount {
    vaults: Vec<OnePuxVault>,
}

#[derive(Deserialize)]
struct OnePuxVault {
    items: Vec<OnePuxItem>,
}

#[derive(Deserialize)]
struct OnePuxItem {
    overview: OnePuxOverview,
    details: OnePuxDetails,
}

#[derive(Deserialize, Default)]
struct OnePuxOverview {
    #[serde(rename = "categoryUuid")]
    category_uuid: Option<String>,
}

#[derive(Deserialize, Default)]
struct OnePuxDetails {
    #[serde(rename = "loginFields", default)]
    login_fields: Vec<OnePuxField>,
}

#[derive(Deserialize)]
struct OnePuxField {
    name: Option<String>,
    value: Option<String>,
}

// ─── FIDO CXF import ──────────────────────────────────────────────────────────

/// Import passkeys from a **FIDO Alliance CXF** (Credential Exchange Format) JSON file.
///
/// CXF v1.0 (working draft, Oct 2024): unencrypted JSON with the shape:
/// ```json
/// {
///   "version": "1.0",
///   "credentials": [
///     {
///       "type": "passkey",
///       "data": {
///         "id": "base64url",
///         "relyingPartyId": "x.com",
///         "userId": "base64url",
///         "userName": "user@example.com",
///         "privateKey": "base64-pkcs8",
///         "signatureCounter": 0
///       }
///     }
///   ]
/// }
/// ```
///
/// If the file is a CXF ZIP (encrypted or bundled), extract `cxf.json` first.
/// Encrypted CXF (HPKE) is not yet supported — use the unencrypted export.
pub fn import_cxf(path: &Path) -> Result<Vec<PasskeyCredential>> {
    let json = std::fs::read_to_string(path)
        .map_err(|e| onecrawl_core::Error::Cdp(format!("read CXF file: {e}")))?;
    import_cxf_json_str(&json)
}

fn import_cxf_json_str(json: &str) -> Result<Vec<PasskeyCredential>> {
    let root: CxfExport = serde_json::from_str(json)
        .map_err(|e| onecrawl_core::Error::Cdp(format!("parse CXF JSON: {e}")))?;

    let mut out = Vec::new();
    for entry in root.credentials {
        if entry.r#type.as_deref() != Some("passkey") {
            continue;
        }
        let d = entry.data;
        // `privateKey` is required; skip if absent (hardware-bound)
        let Some(private_key) = d.private_key else {
            continue;
        };
        let rp_id = d.relying_party_id.unwrap_or_default();
        if rp_id.is_empty() {
            continue;
        }
        out.push(PasskeyCredential {
            credential_id: d.id,
            private_key,
            rp_id,
            user_handle: d.user_id,
            sign_count: d.signature_counter.unwrap_or(0),
            is_resident_credential: true,
        });
    }
    Ok(out)
}

// ─── FIDO CXF serde models ─────────────────────────────────────────────────────

#[derive(Deserialize)]
struct CxfExport {
    #[serde(default)]
    credentials: Vec<CxfEntry>,
}

#[derive(Deserialize)]
struct CxfEntry {
    r#type: Option<String>,
    #[serde(default)]
    data: CxfCredentialData,
}

#[derive(Deserialize, Default)]
struct CxfCredentialData {
    /// Base64url-encoded credential ID.
    #[serde(default)]
    id: String,
    /// Relying party ID (e.g. `"x.com"`).
    #[serde(rename = "relyingPartyId")]
    relying_party_id: Option<String>,
    /// Base64url-encoded user handle.
    #[serde(rename = "userId")]
    user_id: Option<String>,
    /// PKCS#8 private key, base64-encoded. Absent for hardware-bound credentials.
    #[serde(rename = "privateKey")]
    private_key: Option<String>,
    /// Signature counter.
    #[serde(rename = "signatureCounter")]
    signature_counter: Option<i64>,
}

// ─── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_cred(id: &str, rp_id: &str) -> PasskeyCredential {
        PasskeyCredential {
            credential_id: id.to_string(),
            private_key: "dGVzdA==".to_string(),
            rp_id: rp_id.to_string(),
            user_handle: None,
            sign_count: 0,
            is_resident_credential: true,
        }
    }

    // ── Vault operations ──────────────────────────────────────────────────────

    #[test]
    fn vault_add_and_get() {
        let mut v = PasskeyVault::default();
        vault_add(&mut v, vec![sample_cred("aaa", "x.com")]);
        let got = vault_get(&v, "x.com");
        assert_eq!(got.len(), 1);
        assert_eq!(got[0].credential_id, "aaa");
    }

    #[test]
    fn vault_add_deduplicates_by_credential_id() {
        let mut v = PasskeyVault::default();
        vault_add(&mut v, vec![sample_cred("aaa", "x.com")]);
        vault_add(&mut v, vec![sample_cred("aaa", "x.com")]); // duplicate
        vault_add(&mut v, vec![sample_cred("bbb", "x.com")]);
        assert_eq!(vault_get(&v, "x.com").len(), 2);
    }

    #[test]
    fn vault_remove_by_credential_id() {
        let mut v = PasskeyVault::default();
        vault_add(&mut v, vec![sample_cred("aaa", "x.com"), sample_cred("bbb", "x.com")]);
        assert!(vault_remove(&mut v, "aaa"));
        assert_eq!(vault_get(&v, "x.com").len(), 1);
    }

    #[test]
    fn vault_remove_cleans_empty_entries() {
        let mut v = PasskeyVault::default();
        vault_add(&mut v, vec![sample_cred("aaa", "x.com")]);
        vault_remove(&mut v, "aaa");
        assert!(v.credentials.is_empty());
    }

    #[test]
    fn vault_clear_site() {
        let mut v = PasskeyVault::default();
        vault_add(&mut v, vec![sample_cred("aaa", "x.com"), sample_cred("bbb", "github.com")]);
        let removed = super::vault_clear_site(&mut v, "x.com");
        assert_eq!(removed, 1);
        assert!(vault_get(&v, "x.com").is_empty());
        assert_eq!(vault_get(&v, "github.com").len(), 1);
    }

    #[test]
    fn vault_list_sorted() {
        let mut v = PasskeyVault::default();
        vault_add(&mut v, vec![sample_cred("a", "z.com"), sample_cred("b", "a.com")]);
        let list = vault_list(&v);
        assert_eq!(list[0].0, "a.com");
        assert_eq!(list[1].0, "z.com");
    }

    #[test]
    fn vault_total() {
        let mut v = PasskeyVault::default();
        vault_add(&mut v, vec![sample_cred("a", "x.com"), sample_cred("b", "y.com"), sample_cred("c", "y.com")]);
        assert_eq!(super::vault_total(&v), 3);
    }

    #[test]
    fn vault_roundtrip_json() {
        let mut v = PasskeyVault::default();
        vault_add(&mut v, vec![sample_cred("cred1", "example.com")]);
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("vault.json");
        let json = serde_json::to_string_pretty(&v).unwrap();
        std::fs::write(&path, &json).unwrap();
        let loaded: PasskeyVault = serde_json::from_str(&json).unwrap();
        assert_eq!(vault_get(&loaded, "example.com").len(), 1);
    }

    // ── Bitwarden import ──────────────────────────────────────────────────────

    #[test]
    fn import_bitwarden_parses_fido2_credentials() {
        let json = r#"{
            "items": [{
                "name": "X.com",
                "type": 1,
                "login": {
                    "fido2Credentials": [{
                        "credentialId": "Y3JlZElk",
                        "keyValue": "cHJpdktleQ==",
                        "rpId": "x.com",
                        "userHandle": "dXNlckhhbmRsZQ==",
                        "counter": "5",
                        "discoverable": "true"
                    }]
                }
            }]
        }"#;
        let dir = tempfile::tempdir().unwrap();
        let p = dir.path().join("bw.json");
        std::fs::write(&p, json).unwrap();
        let creds = import_bitwarden(&p).unwrap();
        assert_eq!(creds.len(), 1);
        assert_eq!(creds[0].credential_id, "Y3JlZElk");
        assert_eq!(creds[0].private_key, "cHJpdktleQ==");
        assert_eq!(creds[0].rp_id, "x.com");
        assert_eq!(creds[0].sign_count, 5);
        assert!(creds[0].is_resident_credential);
    }

    #[test]
    fn import_bitwarden_skips_no_private_key() {
        let json = r#"{"items": [{"login": {"fido2Credentials": [{"credentialId": "abc", "rpId": "x.com", "counter": "0", "discoverable": "true"}]}}]}"#;
        let dir = tempfile::tempdir().unwrap();
        let p = dir.path().join("bw.json");
        std::fs::write(&p, json).unwrap();
        let creds = import_bitwarden(&p).unwrap();
        assert!(creds.is_empty(), "hardware-bound cred should be skipped");
    }

    #[test]
    fn import_bitwarden_empty_items() {
        let json = r#"{"items": []}"#;
        let dir = tempfile::tempdir().unwrap();
        let p = dir.path().join("bw.json");
        std::fs::write(&p, json).unwrap();
        assert!(import_bitwarden(&p).unwrap().is_empty());
    }

    // ── 1Password import ──────────────────────────────────────────────────────

    #[test]
    fn import_1password_json_parses_passkey_category() {
        let json = r#"{
            "accounts": [{
                "vaults": [{
                    "items": [{
                        "overview": {"categoryUuid": "119"},
                        "details": {
                            "loginFields": [
                                {"name": "credentialId", "value": "Y3JlZElk"},
                                {"name": "keyValue",     "value": "cHJpdktleQ=="},
                                {"name": "rpId",         "value": "x.com"},
                                {"name": "userHandle",   "value": "dXNlcg=="},
                                {"name": "counter",      "value": "3"}
                            ]
                        }
                    }]
                }]
            }]
        }"#;
        let dir = tempfile::tempdir().unwrap();
        let p = dir.path().join("export.data");
        std::fs::write(&p, json).unwrap();
        let creds = import_1password_json(&p).unwrap();
        assert_eq!(creds.len(), 1);
        assert_eq!(creds[0].rp_id, "x.com");
        assert_eq!(creds[0].sign_count, 3);
    }

    #[test]
    fn import_1password_json_skips_non_passkey_items() {
        let json = r#"{"accounts":[{"vaults":[{"items":[{"overview":{"categoryUuid":"001"},"details":{"loginFields":[]}}]}]}]}"#;
        let dir = tempfile::tempdir().unwrap();
        let p = dir.path().join("export.data");
        std::fs::write(&p, json).unwrap();
        assert!(import_1password_json(&p).unwrap().is_empty());
    }

    // ── FIDO CXF import ───────────────────────────────────────────────────────

    #[test]
    fn import_cxf_parses_passkey_entries() {
        let json = r#"{
            "version": "1.0",
            "credentials": [{
                "type": "passkey",
                "data": {
                    "id": "Y3JlZElk",
                    "relyingPartyId": "x.com",
                    "userId": "dXNlcg==",
                    "userName": "user@x.com",
                    "privateKey": "cHJpdktleQ==",
                    "signatureCounter": 7
                }
            }]
        }"#;
        let dir = tempfile::tempdir().unwrap();
        let p = dir.path().join("cxf.json");
        std::fs::write(&p, json).unwrap();
        let creds = import_cxf(&p).unwrap();
        assert_eq!(creds.len(), 1);
        assert_eq!(creds[0].rp_id, "x.com");
        assert_eq!(creds[0].sign_count, 7);
    }

    #[test]
    fn import_cxf_skips_non_passkey_type() {
        let json = r#"{"version":"1.0","credentials":[{"type":"totp","data":{"id":"abc","relyingPartyId":"x.com","privateKey":"key"}}]}"#;
        let dir = tempfile::tempdir().unwrap();
        let p = dir.path().join("cxf.json");
        std::fs::write(&p, json).unwrap();
        assert!(import_cxf(&p).unwrap().is_empty());
    }

    #[test]
    fn import_cxf_skips_no_private_key() {
        let json = r#"{"version":"1.0","credentials":[{"type":"passkey","data":{"id":"abc","relyingPartyId":"x.com"}}]}"#;
        let dir = tempfile::tempdir().unwrap();
        let p = dir.path().join("cxf.json");
        std::fs::write(&p, json).unwrap();
        assert!(import_cxf(&p).unwrap().is_empty());
    }
}
