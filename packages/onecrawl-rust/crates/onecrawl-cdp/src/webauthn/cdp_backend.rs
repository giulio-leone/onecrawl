use chromiumoxide::cdp::browser_protocol::web_authn::{
    AddCredentialParams, AddVirtualAuthenticatorParams, AuthenticatorId,
    AuthenticatorProtocol, AuthenticatorTransport, Ctap2Version, EnableParams,
    GetCredentialsParams, VirtualAuthenticatorOptions,
};
use chromiumoxide::Page;
use onecrawl_core::Result;
use super::types::PasskeyCredential;

pub async fn cdp_enable(page: &Page) -> Result<()> {
    page.execute(EnableParams::default())
        .await
        .map_err(|e| onecrawl_core::Error::Cdp(format!("WebAuthn.enable: {e}")))?;
    Ok(())
}

/// Create a CTAP2.1 platform virtual authenticator with user-verification.
///
/// Returns the opaque `authenticator_id` string that must be passed to all
/// subsequent `cdp_get_credentials` / `cdp_add_credential` calls.
pub async fn cdp_create_authenticator(page: &Page) -> Result<String> {
    let mut options = VirtualAuthenticatorOptions::new(
        AuthenticatorProtocol::Ctap2,
        AuthenticatorTransport::Internal,
    );
    options.ctap2_version = Some(Ctap2Version::Ctap21);
    options.has_resident_key = Some(true);
    options.has_user_verification = Some(true);
    options.is_user_verified = Some(true);
    options.automatic_presence_simulation = Some(true);

    let result = page
        .execute(AddVirtualAuthenticatorParams::new(options))
        .await
        .map_err(|e| onecrawl_core::Error::Cdp(format!("addVirtualAuthenticator: {e}")))?;
    Ok(result.authenticator_id.inner().clone())
}

/// Retrieve all credentials stored in the virtual authenticator.
pub async fn cdp_get_credentials(
    page: &Page,
    authenticator_id: &str,
) -> Result<Vec<PasskeyCredential>> {
    let result = page
        .execute(GetCredentialsParams::new(AuthenticatorId::new(
            authenticator_id,
        )))
        .await
        .map_err(|e| onecrawl_core::Error::Cdp(format!("getCredentials: {e}")))?;
    Ok(result
        .credentials
        .clone()
        .into_iter()
        .map(|c| PasskeyCredential {
            credential_id: String::from(c.credential_id),
            private_key: String::from(c.private_key),
            rp_id: c.rp_id.unwrap_or_default(),
            user_handle: c.user_handle.map(String::from),
            sign_count: c.sign_count,
            is_resident_credential: c.is_resident_credential,
        })
        .collect())
}

/// Inject a saved passkey credential into the virtual authenticator.
///
/// The credential's `private_key` field is used by Chrome for real ECDSA
/// signing — assertions produced are cryptographically valid.
pub async fn cdp_add_credential(
    page: &Page,
    authenticator_id: &str,
    credential: &PasskeyCredential,
) -> Result<()> {
    use chromiumoxide::cdp::browser_protocol::web_authn::Credential;
    let mut cdp_cred = Credential::new(
        credential.credential_id.clone(),
        credential.is_resident_credential,
        credential.private_key.clone(),
        credential.sign_count,
    );
    cdp_cred.rp_id = Some(credential.rp_id.clone());
    cdp_cred.user_handle = credential.user_handle.clone().map(Into::into);
    page.execute(AddCredentialParams::new(
        AuthenticatorId::new(authenticator_id),
        cdp_cred,
    ))
    .await
    .map_err(|e| onecrawl_core::Error::Cdp(format!("addCredential: {e}")))?;
    Ok(())
}

/// Serialize passkey credentials to a pretty-printed JSON file.
pub fn save_passkeys(path: &std::path::Path, credentials: &[PasskeyCredential]) -> Result<()> {
    let json = serde_json::to_string_pretty(credentials)
        .map_err(|e| onecrawl_core::Error::Cdp(format!("serialize passkeys: {e}")))?;
    std::fs::write(path, json)
        .map_err(|e| onecrawl_core::Error::Cdp(format!("write passkeys: {e}")))?;
    Ok(())
}

/// Deserialize passkey credentials from a JSON file produced by `save_passkeys`.
pub fn load_passkeys(path: &std::path::Path) -> Result<Vec<PasskeyCredential>> {
    let json = std::fs::read_to_string(path)
        .map_err(|e| onecrawl_core::Error::Cdp(format!("read passkeys: {e}")))?;
    serde_json::from_str(&json)
        .map_err(|e| onecrawl_core::Error::Cdp(format!("parse passkeys: {e}")))
}
