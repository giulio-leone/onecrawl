//! NAPI-RS bindings for the OneCrawl Rust workspace.
//!
//! Exposes crypto, parser, and storage functionality to Node.js.

#[macro_use]
extern crate napi_derive;

use base64::{engine::general_purpose::STANDARD as B64, Engine as _};
use napi::bindgen_prelude::*;

// ──────────────────────────── Crypto ────────────────────────────

/// PKCE challenge pair (code_verifier + code_challenge).
#[napi(object)]
pub struct PkceChallenge {
    pub verifier: String,
    pub challenge: String,
}

/// AES-256-GCM encrypt. Returns `salt(16) + nonce(12) + ciphertext`.
#[napi]
pub fn encrypt(plaintext: Buffer, password: String) -> Result<Buffer> {
    let payload = onecrawl_crypto::encrypt(&plaintext, &password)
        .map_err(|e| Error::from_reason(e.to_string()))?;

    let salt = B64
        .decode(&payload.salt)
        .map_err(|e| Error::from_reason(e.to_string()))?;
    let nonce = B64
        .decode(&payload.nonce)
        .map_err(|e| Error::from_reason(e.to_string()))?;
    let ct = B64
        .decode(&payload.ciphertext)
        .map_err(|e| Error::from_reason(e.to_string()))?;

    let mut out = Vec::with_capacity(salt.len() + nonce.len() + ct.len());
    out.extend_from_slice(&salt);
    out.extend_from_slice(&nonce);
    out.extend_from_slice(&ct);

    Ok(out.into())
}

/// AES-256-GCM decrypt. Input format: `salt(16) + nonce(12) + ciphertext`.
#[napi]
pub fn decrypt(ciphertext: Buffer, password: String) -> Result<Buffer> {
    if ciphertext.len() < 28 {
        return Err(Error::from_reason(
            "ciphertext too short: need at least 28 bytes (16 salt + 12 nonce)",
        ));
    }

    let payload = onecrawl_core::EncryptedPayload {
        salt: B64.encode(&ciphertext[..16]),
        nonce: B64.encode(&ciphertext[16..28]),
        ciphertext: B64.encode(&ciphertext[28..]),
    };

    let plaintext = onecrawl_crypto::decrypt(&payload, &password)
        .map_err(|e| Error::from_reason(e.to_string()))?;

    Ok(plaintext.into())
}

/// PBKDF2-HMAC-SHA256 key derivation (returns 32-byte key).
#[napi]
pub fn derive_key(password: String, salt: Buffer) -> Result<Buffer> {
    let key = onecrawl_crypto::derive_key(&password, &salt)
        .map_err(|e| Error::from_reason(e.to_string()))?;
    Ok(key.to_vec().into())
}

/// Generate a PKCE S256 challenge pair.
#[napi]
pub fn generate_pkce() -> Result<PkceChallenge> {
    let c = onecrawl_crypto::generate_pkce_challenge()
        .map_err(|e| Error::from_reason(e.to_string()))?;
    Ok(PkceChallenge {
        verifier: c.code_verifier,
        challenge: c.code_challenge,
    })
}

/// Generate a 6-digit TOTP code (SHA-1, 30s period).
#[napi]
pub fn generate_totp(secret: String) -> Result<String> {
    let config = onecrawl_core::TotpConfig {
        secret,
        ..Default::default()
    };
    onecrawl_crypto::totp::generate_totp(&config).map_err(|e| Error::from_reason(e.to_string()))
}

/// Verify a TOTP code with ±1 step window.
#[napi]
pub fn verify_totp(secret: String, code: String) -> Result<bool> {
    let config = onecrawl_core::TotpConfig {
        secret,
        ..Default::default()
    };
    onecrawl_crypto::totp::verify_totp(&config, &code)
        .map_err(|e| Error::from_reason(e.to_string()))
}

// ──────────────────────────── Parser ────────────────────────────

/// Link extracted from HTML.
#[napi(object)]
pub struct LinkInfo {
    pub href: String,
    pub text: String,
    pub is_external: bool,
}

/// Build and render an accessibility tree from HTML.
#[napi]
pub fn parse_accessibility_tree(html: String) -> Result<String> {
    let tree = onecrawl_parser::get_accessibility_tree(&html)
        .map_err(|e| Error::from_reason(e.to_string()))?;
    Ok(onecrawl_parser::accessibility::render_tree(&tree, 0, false))
}

/// Query HTML with a CSS selector, returns JSON array of matching elements.
#[napi]
pub fn query_selector(html: String, selector: String) -> Result<String> {
    let elements = onecrawl_parser::query_selector(&html, &selector)
        .map_err(|e| Error::from_reason(e.to_string()))?;
    serde_json::to_string(&elements).map_err(|e| Error::from_reason(e.to_string()))
}

/// Extract all visible text from HTML.
#[napi]
pub fn extract_text(html: String) -> Result<String> {
    let texts = onecrawl_parser::extract_text(&html, "body")
        .map_err(|e| Error::from_reason(e.to_string()))?;
    Ok(texts.join("\n"))
}

/// Extract all links from HTML with external detection.
#[napi]
pub fn extract_links(html: String) -> Result<Vec<LinkInfo>> {
    let links = onecrawl_parser::extract::extract_links(&html)
        .map_err(|e| Error::from_reason(e.to_string()))?;
    Ok(links
        .into_iter()
        .map(|(href, text)| {
            let is_external = href.starts_with("http://") || href.starts_with("https://");
            LinkInfo {
                href,
                text,
                is_external,
            }
        })
        .collect())
}

// ──────────────────────────── Storage ────────────────────────────

/// Encrypted key-value store backed by sled + AES-256-GCM.
#[napi(js_name = "NativeStore")]
pub struct NativeStore {
    inner: onecrawl_storage::EncryptedStore,
}

#[napi]
impl NativeStore {
    /// Open (or create) an encrypted store at the given path.
    #[napi(constructor)]
    pub fn new(path: String, password: String) -> Result<Self> {
        let store =
            onecrawl_storage::EncryptedStore::open(std::path::Path::new(&path), &password)
                .map_err(|e| Error::from_reason(e.to_string()))?;
        Ok(Self { inner: store })
    }

    /// Retrieve a value by key.
    #[napi]
    pub fn get(&self, key: String) -> Result<Option<String>> {
        let value = self
            .inner
            .get(&key)
            .map_err(|e| Error::from_reason(e.to_string()))?;
        Ok(value.map(|v| String::from_utf8_lossy(&v).into_owned()))
    }

    /// Store a value.
    #[napi]
    pub fn set(&self, key: String, value: String) -> Result<()> {
        self.inner
            .set(&key, value.as_bytes())
            .map_err(|e| Error::from_reason(e.to_string()))
    }

    /// Delete a key, returns true if it existed.
    #[napi]
    pub fn delete(&self, key: String) -> Result<bool> {
        self.inner
            .delete(&key)
            .map_err(|e| Error::from_reason(e.to_string()))
    }

    /// List all keys in the store.
    #[napi]
    pub fn list(&self) -> Result<Vec<String>> {
        self.inner
            .list("")
            .map_err(|e| Error::from_reason(e.to_string()))
    }

    /// Check if a key exists.
    #[napi]
    pub fn contains(&self, key: String) -> Result<bool> {
        self.inner
            .contains(&key)
            .map_err(|e| Error::from_reason(e.to_string()))
    }

    /// Flush pending writes to disk.
    #[napi]
    pub fn flush(&self) -> Result<()> {
        self.inner
            .flush()
            .map_err(|e| Error::from_reason(e.to_string()))
    }
}
