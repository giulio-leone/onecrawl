//! WebAuthn/FIDO2 virtual authenticator types.

use serde::{Deserialize, Serialize};

// ─── CDP-native passkey credential (persisted to JSON) ────────────────────────

/// A passkey credential exported from Chrome's CDP virtual authenticator.
///
/// Contains the real ECDSA P-256 private key in PKCS#8 format (base64), which
/// Chrome uses to generate valid WebAuthn assertions for server verification.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PasskeyCredential {
    /// Base64-encoded credential ID (assigned by Chrome).
    pub credential_id: String,
    /// Base64-encoded ECDSA P-256 private key in PKCS#8 format.
    pub private_key: String,
    /// Relying party ID (e.g. `"x.com"` or `"twitter.com"`).
    pub rp_id: String,
    /// Base64-encoded user handle (optional).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub user_handle: Option<String>,
    /// Signature counter — incremented on each successful assertion.
    pub sign_count: i64,
    /// Whether this is a resident/discoverable credential.
    pub is_resident_credential: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct VirtualAuthenticator {
    pub id: String,
    /// `"ctap2"` or `"u2f"`
    pub protocol: String,
    /// `"usb"`, `"nfc"`, `"ble"`, or `"internal"`
    pub transport: String,
    pub has_resident_key: bool,
    pub has_user_verification: bool,
    pub is_user_verified: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct VirtualCredential {
    /// base64url-encoded credential ID
    pub credential_id: String,
    /// Relying party ID, e.g. `"example.com"`
    pub rp_id: String,
    /// base64url-encoded user handle
    pub user_handle: String,
    pub sign_count: u32,
}
