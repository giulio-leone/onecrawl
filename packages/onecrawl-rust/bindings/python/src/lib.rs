//! PyO3 bindings for the OneCrawl Rust workspace.
//!
//! Exposes crypto, parser, and storage functionality to Python.

use base64::{Engine as _, engine::general_purpose::STANDARD as B64};
use pyo3::prelude::*;

// ──────────────────────────── Crypto ────────────────────────────

/// AES-256-GCM encrypt. Returns `salt(16) + nonce(12) + ciphertext`.
#[pyfunction]
fn encrypt(plaintext: &[u8], password: &str) -> PyResult<Vec<u8>> {
    let payload = onecrawl_crypto::encrypt(plaintext, password)
        .map_err(|e| pyo3::exceptions::PyRuntimeError::new_err(e.to_string()))?;

    let salt = B64
        .decode(&payload.salt)
        .map_err(|e| pyo3::exceptions::PyRuntimeError::new_err(e.to_string()))?;
    let nonce = B64
        .decode(&payload.nonce)
        .map_err(|e| pyo3::exceptions::PyRuntimeError::new_err(e.to_string()))?;
    let ct = B64
        .decode(&payload.ciphertext)
        .map_err(|e| pyo3::exceptions::PyRuntimeError::new_err(e.to_string()))?;

    let mut out = Vec::with_capacity(salt.len() + nonce.len() + ct.len());
    out.extend_from_slice(&salt);
    out.extend_from_slice(&nonce);
    out.extend_from_slice(&ct);

    Ok(out)
}

/// AES-256-GCM decrypt. Input format: `salt(16) + nonce(12) + ciphertext`.
#[pyfunction]
fn decrypt(ciphertext: &[u8], password: &str) -> PyResult<Vec<u8>> {
    if ciphertext.len() < 28 {
        return Err(pyo3::exceptions::PyValueError::new_err(
            "ciphertext too short: need at least 28 bytes (16 salt + 12 nonce)",
        ));
    }

    let payload = onecrawl_core::EncryptedPayload {
        salt: B64.encode(&ciphertext[..16]),
        nonce: B64.encode(&ciphertext[16..28]),
        ciphertext: B64.encode(&ciphertext[28..]),
    };

    let plaintext = onecrawl_crypto::decrypt(&payload, password)
        .map_err(|e| pyo3::exceptions::PyRuntimeError::new_err(e.to_string()))?;

    Ok(plaintext)
}

/// PBKDF2-HMAC-SHA256 key derivation (returns 32-byte key).
#[pyfunction]
fn derive_key(password: &str, salt: &[u8]) -> PyResult<Vec<u8>> {
    let key = onecrawl_crypto::derive_key(password, salt)
        .map_err(|e| pyo3::exceptions::PyRuntimeError::new_err(e.to_string()))?;
    Ok(key.to_vec())
}

/// Generate a PKCE S256 challenge pair. Returns (verifier, challenge).
#[pyfunction]
fn generate_pkce() -> PyResult<(String, String)> {
    let c = onecrawl_crypto::generate_pkce_challenge()
        .map_err(|e| pyo3::exceptions::PyRuntimeError::new_err(e.to_string()))?;
    Ok((c.code_verifier, c.code_challenge))
}

/// Generate a 6-digit TOTP code (SHA-1, 30s period).
#[pyfunction]
fn generate_totp(secret: &str) -> PyResult<String> {
    let config = onecrawl_core::TotpConfig {
        secret: secret.to_string(),
        ..Default::default()
    };
    onecrawl_crypto::totp::generate_totp(&config)
        .map_err(|e| pyo3::exceptions::PyRuntimeError::new_err(e.to_string()))
}

/// Verify a TOTP code with ±1 step window.
#[pyfunction]
fn verify_totp(secret: &str, code: &str) -> PyResult<bool> {
    let config = onecrawl_core::TotpConfig {
        secret: secret.to_string(),
        ..Default::default()
    };
    onecrawl_crypto::totp::verify_totp(&config, code)
        .map_err(|e| pyo3::exceptions::PyRuntimeError::new_err(e.to_string()))
}

// ──────────────────────────── Parser ────────────────────────────

/// Build and render an accessibility tree from HTML.
#[pyfunction]
fn parse_accessibility_tree(html: &str) -> PyResult<String> {
    let tree = onecrawl_parser::get_accessibility_tree(html)
        .map_err(|e| pyo3::exceptions::PyRuntimeError::new_err(e.to_string()))?;
    Ok(onecrawl_parser::accessibility::render_tree(&tree, 0, false))
}

/// Query HTML with a CSS selector, returns JSON array of matching elements.
#[pyfunction]
fn query_selector(html: &str, selector: &str) -> PyResult<String> {
    let elements = onecrawl_parser::query_selector(html, selector)
        .map_err(|e| pyo3::exceptions::PyRuntimeError::new_err(e.to_string()))?;
    serde_json::to_string(&elements)
        .map_err(|e| pyo3::exceptions::PyRuntimeError::new_err(e.to_string()))
}

/// Extract all visible text from HTML.
#[pyfunction]
fn extract_text(html: &str) -> PyResult<String> {
    let texts = onecrawl_parser::extract_text(html, "body")
        .map_err(|e| pyo3::exceptions::PyRuntimeError::new_err(e.to_string()))?;
    Ok(texts.join("\n"))
}

/// Extract all links from HTML with external detection.
/// Returns list of (href, text, is_external) tuples.
#[pyfunction]
fn extract_links(html: &str) -> PyResult<Vec<(String, String, bool)>> {
    let links = onecrawl_parser::extract::extract_links(html)
        .map_err(|e| pyo3::exceptions::PyRuntimeError::new_err(e.to_string()))?;
    Ok(links
        .into_iter()
        .map(|(href, text)| {
            let is_external = href.starts_with("http://") || href.starts_with("https://");
            (href, text, is_external)
        })
        .collect())
}

// ──────────────────────────── Storage ────────────────────────────

/// Encrypted key-value store backed by sled + AES-256-GCM.
#[pyclass]
struct Store {
    inner: onecrawl_storage::EncryptedStore,
}

#[pymethods]
impl Store {
    /// Open (or create) an encrypted store at the given path.
    #[new]
    fn new(path: &str, password: &str) -> PyResult<Self> {
        let store =
            onecrawl_storage::EncryptedStore::open(std::path::Path::new(path), password)
                .map_err(|e| pyo3::exceptions::PyRuntimeError::new_err(e.to_string()))?;
        Ok(Self { inner: store })
    }

    /// Retrieve a value by key.
    fn get(&self, key: &str) -> PyResult<Option<String>> {
        let value = self
            .inner
            .get(key)
            .map_err(|e| pyo3::exceptions::PyRuntimeError::new_err(e.to_string()))?;
        Ok(value.map(|v| String::from_utf8_lossy(&v).into_owned()))
    }

    /// Store a value.
    fn set(&self, key: &str, value: &str) -> PyResult<()> {
        self.inner
            .set(key, value.as_bytes())
            .map_err(|e| pyo3::exceptions::PyRuntimeError::new_err(e.to_string()))
    }

    /// Delete a key, returns true if it existed.
    fn delete(&self, key: &str) -> PyResult<bool> {
        self.inner
            .delete(key)
            .map_err(|e| pyo3::exceptions::PyRuntimeError::new_err(e.to_string()))
    }

    /// List all keys in the store.
    fn keys(&self) -> PyResult<Vec<String>> {
        self.inner
            .list("")
            .map_err(|e| pyo3::exceptions::PyRuntimeError::new_err(e.to_string()))
    }

    /// Check if a key exists.
    fn contains(&self, key: &str) -> PyResult<bool> {
        self.inner
            .contains(key)
            .map_err(|e| pyo3::exceptions::PyRuntimeError::new_err(e.to_string()))
    }

    /// Flush pending writes to disk.
    fn flush(&self) -> PyResult<()> {
        self.inner
            .flush()
            .map_err(|e| pyo3::exceptions::PyRuntimeError::new_err(e.to_string()))
    }
}

// ──────────────────────────── Module ────────────────────────────

fn register_crypto(parent: &Bound<'_, PyModule>) -> PyResult<()> {
    let m = PyModule::new(parent.py(), "crypto")?;
    m.add_function(wrap_pyfunction!(encrypt, &m)?)?;
    m.add_function(wrap_pyfunction!(decrypt, &m)?)?;
    m.add_function(wrap_pyfunction!(derive_key, &m)?)?;
    m.add_function(wrap_pyfunction!(generate_pkce, &m)?)?;
    m.add_function(wrap_pyfunction!(generate_totp, &m)?)?;
    m.add_function(wrap_pyfunction!(verify_totp, &m)?)?;
    parent.add_submodule(&m)?;
    // Register in sys.modules so `from onecrawl.crypto import X` works
    parent
        .py()
        .import("sys")?
        .getattr("modules")?
        .set_item("onecrawl.crypto", &m)?;
    Ok(())
}

fn register_parser(parent: &Bound<'_, PyModule>) -> PyResult<()> {
    let m = PyModule::new(parent.py(), "parser")?;
    m.add_function(wrap_pyfunction!(parse_accessibility_tree, &m)?)?;
    m.add_function(wrap_pyfunction!(query_selector, &m)?)?;
    m.add_function(wrap_pyfunction!(extract_text, &m)?)?;
    m.add_function(wrap_pyfunction!(extract_links, &m)?)?;
    parent.add_submodule(&m)?;
    parent
        .py()
        .import("sys")?
        .getattr("modules")?
        .set_item("onecrawl.parser", &m)?;
    Ok(())
}

#[pymodule]
fn onecrawl(m: &Bound<'_, PyModule>) -> PyResult<()> {
    register_crypto(m)?;
    register_parser(m)?;
    m.add_class::<Store>()?;
    Ok(())
}
