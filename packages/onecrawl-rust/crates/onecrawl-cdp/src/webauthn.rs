//! WebAuthn/FIDO2 virtual authenticator simulation.
//!
//! Monkey-patches `navigator.credentials` to simulate passkey registration
//! and authentication flows without real hardware tokens.

use chromiumoxide::Page;
use onecrawl_core::Result;
use serde::{Deserialize, Serialize};

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

/// Enable virtual authenticator environment.
///
/// Overrides `navigator.credentials.create()` and `navigator.credentials.get()`
/// to simulate WebAuthn/passkey flows without real hardware.
pub async fn enable_virtual_authenticator(
    page: &Page,
    config: &VirtualAuthenticator,
) -> Result<()> {
    let config_json = serde_json::to_string(config)
        .map_err(|e| onecrawl_core::Error::Cdp(format!("serialize config: {e}")))?;
    let js = format!(
        r#"
        (() => {{
            window.__onecrawl_webauthn = {{
                config: {config_json},
                credentials: [],
                log: []
            }};

            function randomBytes(n) {{
                const arr = new Uint8Array(n);
                for (let i = 0; i < n; i++) arr[i] = Math.floor(Math.random() * 256);
                return arr;
            }}

            function toBase64Url(buffer) {{
                const bytes = new Uint8Array(buffer);
                let str = '';
                for (const b of bytes) str += String.fromCharCode(b);
                return btoa(str).replace(/\+/g, '-').replace(/\//g, '_').replace(/=/g, '');
            }}

            function fromBase64Url(str) {{
                str = str.replace(/-/g, '+').replace(/_/g, '/');
                while (str.length % 4) str += '=';
                const binary = atob(str);
                const bytes = new Uint8Array(binary.length);
                for (let i = 0; i < binary.length; i++) bytes[i] = binary.charCodeAt(i);
                return bytes.buffer;
            }}

            const origCreate = navigator.credentials.create.bind(navigator.credentials);
            const origGet = navigator.credentials.get.bind(navigator.credentials);

            navigator.credentials.create = async function(options) {{
                if (!options.publicKey) return origCreate(options);

                const credId = randomBytes(32);
                const credIdB64 = toBase64Url(credId);

                const credential = {{
                    credential_id: credIdB64,
                    rp_id: options.publicKey.rp?.id || window.location.hostname,
                    user_handle: options.publicKey.user?.id ? toBase64Url(options.publicKey.user.id) : '',
                    sign_count: 0
                }};

                window.__onecrawl_webauthn.credentials.push(credential);
                window.__onecrawl_webauthn.log.push({{
                    type: 'create',
                    rp_id: credential.rp_id,
                    credential_id: credIdB64,
                    timestamp: Date.now()
                }});

                const attestationObject = randomBytes(128);
                const clientDataJSON = new TextEncoder().encode(JSON.stringify({{
                    type: 'webauthn.create',
                    challenge: options.publicKey.challenge ? toBase64Url(options.publicKey.challenge) : '',
                    origin: window.location.origin,
                    crossOrigin: false
                }}));

                return {{
                    id: credIdB64,
                    rawId: credId.buffer,
                    type: 'public-key',
                    response: {{
                        attestationObject: attestationObject.buffer,
                        clientDataJSON: clientDataJSON.buffer,
                        getTransports: () => [window.__onecrawl_webauthn.config.transport],
                        getPublicKey: () => randomBytes(65).buffer,
                        getPublicKeyAlgorithm: () => -7,
                        getAuthenticatorData: () => randomBytes(37).buffer,
                    }},
                    getClientExtensionResults: () => ({{}}),
                    authenticatorAttachment: window.__onecrawl_webauthn.config.transport === 'internal' ? 'platform' : 'cross-platform'
                }};
            }};

            navigator.credentials.get = async function(options) {{
                if (!options.publicKey) return origGet(options);

                const rpId = options.publicKey.rpId || window.location.hostname;
                const matchingCreds = window.__onecrawl_webauthn.credentials.filter(c => c.rp_id === rpId);

                let selectedCred;
                if (matchingCreds.length === 0) {{
                    const firstAllowed = options.publicKey.allowCredentials?.[0];
                    if (firstAllowed) {{
                        const credId = new Uint8Array(firstAllowed.id);
                        selectedCred = {{ credential_id: toBase64Url(credId), rp_id: rpId, sign_count: 1, user_handle: '' }};
                    }} else {{
                        throw new DOMException('No credentials found', 'NotAllowedError');
                    }}
                }} else {{
                    selectedCred = matchingCreds[0];
                    selectedCred.sign_count++;
                }}

                window.__onecrawl_webauthn.log.push({{
                    type: 'get',
                    rp_id: rpId,
                    credential_id: selectedCred.credential_id,
                    timestamp: Date.now()
                }});

                const credIdBytes = fromBase64Url(selectedCred.credential_id);
                const clientDataJSON = new TextEncoder().encode(JSON.stringify({{
                    type: 'webauthn.get',
                    challenge: options.publicKey.challenge ? toBase64Url(options.publicKey.challenge) : '',
                    origin: window.location.origin,
                    crossOrigin: false
                }}));

                return {{
                    id: selectedCred.credential_id,
                    rawId: credIdBytes,
                    type: 'public-key',
                    response: {{
                        authenticatorData: randomBytes(37).buffer,
                        clientDataJSON: clientDataJSON.buffer,
                        signature: randomBytes(64).buffer,
                        userHandle: selectedCred.user_handle ? fromBase64Url(selectedCred.user_handle) : null
                    }},
                    getClientExtensionResults: () => ({{}}),
                    authenticatorAttachment: window.__onecrawl_webauthn.config.transport === 'internal' ? 'platform' : 'cross-platform'
                }};
            }};

            return true;
        }})()
    "#
    );
    page.evaluate(js)
        .await
        .map_err(|e| onecrawl_core::Error::Cdp(format!("enable_virtual_authenticator: {e}")))?;
    Ok(())
}

/// Add a pre-existing credential to the virtual authenticator.
pub async fn add_virtual_credential(page: &Page, credential: &VirtualCredential) -> Result<()> {
    let cred_json = serde_json::to_string(credential)
        .map_err(|e| onecrawl_core::Error::Cdp(format!("serialize credential: {e}")))?;
    let js = format!(
        r#"
        (() => {{
            if (!window.__onecrawl_webauthn) return false;
            window.__onecrawl_webauthn.credentials.push({cred_json});
            return true;
        }})()
    "#
    );
    page.evaluate(js)
        .await
        .map_err(|e| onecrawl_core::Error::Cdp(format!("add_virtual_credential: {e}")))?;
    Ok(())
}

/// Get all stored virtual credentials.
pub async fn get_virtual_credentials(page: &Page) -> Result<Vec<VirtualCredential>> {
    let val = page
        .evaluate("window.__onecrawl_webauthn?.credentials || []")
        .await
        .map_err(|e| onecrawl_core::Error::Cdp(format!("get_virtual_credentials: {e}")))?;
    let creds: Vec<VirtualCredential> =
        serde_json::from_value(val.into_value().unwrap_or(serde_json::json!([])))
            .unwrap_or_default();
    Ok(creds)
}

/// Get WebAuthn operation log.
pub async fn get_webauthn_log(page: &Page) -> Result<Vec<serde_json::Value>> {
    let val = page
        .evaluate("window.__onecrawl_webauthn?.log || []")
        .await
        .map_err(|e| onecrawl_core::Error::Cdp(format!("get_webauthn_log: {e}")))?;
    let log: Vec<serde_json::Value> =
        serde_json::from_value(val.into_value().unwrap_or(serde_json::json!([])))
            .unwrap_or_default();
    Ok(log)
}

/// Clear virtual authenticator and restore original `navigator.credentials`.
pub async fn disable_virtual_authenticator(page: &Page) -> Result<()> {
    page.evaluate(
        r#"
        if (window.__onecrawl_webauthn) {
            delete window.__onecrawl_webauthn;
        }
    "#,
    )
    .await
    .map_err(|e| onecrawl_core::Error::Cdp(format!("disable_virtual_authenticator: {e}")))?;
    Ok(())
}

/// Remove a specific credential by ID. Returns `true` if one was removed.
pub async fn remove_virtual_credential(page: &Page, credential_id: &str) -> Result<bool> {
    let escaped = credential_id.replace('\\', "\\\\").replace('\'', "\\'");
    let js = format!(
        r#"
        (() => {{
            if (!window.__onecrawl_webauthn) return false;
            const before = window.__onecrawl_webauthn.credentials.length;
            window.__onecrawl_webauthn.credentials = window.__onecrawl_webauthn.credentials.filter(c => c.credential_id !== '{escaped}');
            return window.__onecrawl_webauthn.credentials.length < before;
        }})()
    "#
    );
    let val = page
        .evaluate(js)
        .await
        .map_err(|e| onecrawl_core::Error::Cdp(format!("remove_virtual_credential: {e}")))?;
    Ok(val
        .into_value::<serde_json::Value>()
        .ok()
        .and_then(|v| v.as_bool())
        .unwrap_or(false))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn default_authenticator() -> VirtualAuthenticator {
        VirtualAuthenticator {
            id: "auth-1".into(),
            protocol: "ctap2".into(),
            transport: "internal".into(),
            has_resident_key: true,
            has_user_verification: true,
            is_user_verified: true,
        }
    }

    fn default_credential() -> VirtualCredential {
        VirtualCredential {
            credential_id: "Y3JlZC0x".into(),
            rp_id: "example.com".into(),
            user_handle: "dXNlci0x".into(),
            sign_count: 0,
        }
    }

    #[test]
    fn test_virtual_authenticator_default() {
        let auth = default_authenticator();
        assert_eq!(auth.id, "auth-1");
        assert_eq!(auth.protocol, "ctap2");
        assert_eq!(auth.transport, "internal");
        assert!(auth.has_resident_key);
        assert!(auth.has_user_verification);
        assert!(auth.is_user_verified);
    }

    #[test]
    fn test_virtual_credential_serialization() {
        let cred = default_credential();
        let json = serde_json::to_string(&cred).unwrap();
        let parsed: VirtualCredential = serde_json::from_str(&json).unwrap();
        assert_eq!(cred, parsed);
    }

    #[test]
    fn test_authenticator_config_json() {
        let auth = default_authenticator();
        let json_val: serde_json::Value = serde_json::to_value(&auth).unwrap();
        assert_eq!(json_val["id"], "auth-1");
        assert_eq!(json_val["protocol"], "ctap2");
        assert_eq!(json_val["transport"], "internal");
        assert_eq!(json_val["has_resident_key"], true);
        assert_eq!(json_val["has_user_verification"], true);
        assert_eq!(json_val["is_user_verified"], true);
    }

    #[test]
    fn test_credential_roundtrip() {
        let cred = VirtualCredential {
            credential_id: "abc123".into(),
            rp_id: "mysite.org".into(),
            user_handle: "aGFuZGxl".into(),
            sign_count: 42,
        };
        let serialized = serde_json::to_string(&cred).unwrap();
        let deserialized: VirtualCredential = serde_json::from_str(&serialized).unwrap();
        assert_eq!(cred, deserialized);
        assert_eq!(deserialized.sign_count, 42);
    }

    #[test]
    fn test_multiple_credentials() {
        let creds = vec![
            VirtualCredential {
                credential_id: "c1".into(),
                rp_id: "a.com".into(),
                user_handle: "u1".into(),
                sign_count: 0,
            },
            VirtualCredential {
                credential_id: "c2".into(),
                rp_id: "b.com".into(),
                user_handle: "u2".into(),
                sign_count: 5,
            },
            VirtualCredential {
                credential_id: "c3".into(),
                rp_id: "a.com".into(),
                user_handle: "u3".into(),
                sign_count: 10,
            },
        ];
        let json = serde_json::to_string(&creds).unwrap();
        let parsed: Vec<VirtualCredential> = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.len(), 3);
        let a_com: Vec<_> = parsed.iter().filter(|c| c.rp_id == "a.com").collect();
        assert_eq!(a_com.len(), 2);
    }

    #[test]
    fn test_virtual_authenticator_protocols() {
        let ctap2 = VirtualAuthenticator {
            id: "ctap2-auth".into(),
            protocol: "ctap2".into(),
            transport: "usb".into(),
            has_resident_key: true,
            has_user_verification: true,
            is_user_verified: false,
        };
        let u2f = VirtualAuthenticator {
            id: "u2f-auth".into(),
            protocol: "u2f".into(),
            transport: "nfc".into(),
            has_resident_key: false,
            has_user_verification: false,
            is_user_verified: false,
        };
        assert_eq!(ctap2.protocol, "ctap2");
        assert_eq!(u2f.protocol, "u2f");
        assert_ne!(ctap2, u2f);
        // Verify serialization preserves protocol variants
        let ctap2_json: serde_json::Value = serde_json::to_value(&ctap2).unwrap();
        let u2f_json: serde_json::Value = serde_json::to_value(&u2f).unwrap();
        assert_eq!(ctap2_json["protocol"], "ctap2");
        assert_eq!(u2f_json["protocol"], "u2f");
        assert_eq!(ctap2_json["transport"], "usb");
        assert_eq!(u2f_json["transport"], "nfc");
    }
}
