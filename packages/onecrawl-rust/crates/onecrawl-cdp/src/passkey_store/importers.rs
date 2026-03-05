use serde::Deserialize;
use std::path::Path;
use onecrawl_core::Result;
use crate::webauthn::PasskeyCredential;



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
