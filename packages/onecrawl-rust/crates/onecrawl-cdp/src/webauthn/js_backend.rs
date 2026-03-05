use chromiumoxide::Page;
use onecrawl_core::Result;
use super::types::*;

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
