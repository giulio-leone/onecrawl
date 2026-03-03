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

// ──────────────────────────── Browser (CDP) ────────────────────────────

use std::sync::Arc;

/// Browser automation class powered by chromiumoxide (native CDP).
///
/// ```python
/// browser = Browser.launch(headless=True)
/// browser.goto("https://example.com")
/// title = browser.get_title()
/// png = browser.screenshot()
/// browser.close()
/// ```
#[pyclass]
struct Browser {
    rt: Arc<tokio::runtime::Runtime>,
    session: Arc<onecrawl_cdp::BrowserSession>,
    page: Arc<std::sync::Mutex<Option<onecrawl_cdp::Page>>>,
    event_stream: Arc<std::sync::Mutex<Option<onecrawl_cdp::EventStream>>>,
    har_recorder: Arc<std::sync::Mutex<Option<onecrawl_cdp::HarRecorder>>>,
    ws_recorder: Arc<std::sync::Mutex<Option<onecrawl_cdp::WsRecorder>>>,
}

fn py_err(e: impl std::fmt::Display) -> PyErr {
    pyo3::exceptions::PyRuntimeError::new_err(e.to_string())
}

fn py_parse_network_profile(name: &str) -> PyResult<onecrawl_cdp::NetworkProfile> {
    match name.to_lowercase().as_str() {
        "fast3g" | "fast-3g" => Ok(onecrawl_cdp::NetworkProfile::Fast3G),
        "slow3g" | "slow-3g" => Ok(onecrawl_cdp::NetworkProfile::Slow3G),
        "offline" => Ok(onecrawl_cdp::NetworkProfile::Offline),
        "regular4g" | "4g" => Ok(onecrawl_cdp::NetworkProfile::Regular4G),
        "wifi" => Ok(onecrawl_cdp::NetworkProfile::WiFi),
        _ => Err(py_err(format!("Unknown profile: {name}. Use: fast3g, slow3g, offline, regular4g, wifi"))),
    }
}

#[pymethods]
impl Browser {
    /// Launch a new browser. `headless` defaults to True.
    #[staticmethod]
    #[pyo3(signature = (headless=true))]
    fn launch(headless: bool) -> PyResult<Self> {
        let rt = tokio::runtime::Runtime::new().map_err(py_err)?;
        let session = rt.block_on(async {
            if headless {
                onecrawl_cdp::BrowserSession::launch_headless().await
            } else {
                onecrawl_cdp::BrowserSession::launch_headed().await
            }
        }).map_err(py_err)?;
        let page = rt.block_on(session.new_page("about:blank")).map_err(py_err)?;
        Ok(Self {
            rt: Arc::new(rt),
            session: Arc::new(session),
            page: Arc::new(std::sync::Mutex::new(Some(page))),
            event_stream: Arc::new(std::sync::Mutex::new(None)),
            har_recorder: Arc::new(std::sync::Mutex::new(None)),
            ws_recorder: Arc::new(std::sync::Mutex::new(None)),
        })
    }

    /// Connect to existing browser via CDP WebSocket URL.
    #[staticmethod]
    fn connect(ws_url: &str) -> PyResult<Self> {
        let rt = tokio::runtime::Runtime::new().map_err(py_err)?;
        let session = rt.block_on(onecrawl_cdp::BrowserSession::connect(ws_url)).map_err(py_err)?;
        let page = rt.block_on(session.new_page("about:blank")).map_err(py_err)?;
        Ok(Self {
            rt: Arc::new(rt),
            session: Arc::new(session),
            page: Arc::new(std::sync::Mutex::new(Some(page))),
            event_stream: Arc::new(std::sync::Mutex::new(None)),
            har_recorder: Arc::new(std::sync::Mutex::new(None)),
            ws_recorder: Arc::new(std::sync::Mutex::new(None)),
        })
    }

    /// Navigate to a URL.
    fn goto(&self, url: &str) -> PyResult<()> {
        let guard = self.page.lock().map_err(py_err)?;
        let page = guard.as_ref().ok_or_else(|| py_err("browser closed"))?;
        self.rt.block_on(onecrawl_cdp::navigation::goto(page, url)).map_err(py_err)
    }

    /// Get current URL.
    fn get_url(&self) -> PyResult<String> {
        let guard = self.page.lock().map_err(py_err)?;
        let page = guard.as_ref().ok_or_else(|| py_err("browser closed"))?;
        self.rt.block_on(onecrawl_cdp::navigation::get_url(page)).map_err(py_err)
    }

    /// Get page title.
    fn get_title(&self) -> PyResult<String> {
        let guard = self.page.lock().map_err(py_err)?;
        let page = guard.as_ref().ok_or_else(|| py_err("browser closed"))?;
        self.rt.block_on(onecrawl_cdp::navigation::get_title(page)).map_err(py_err)
    }

    /// Get page HTML content.
    fn content(&self) -> PyResult<String> {
        let guard = self.page.lock().map_err(py_err)?;
        let page = guard.as_ref().ok_or_else(|| py_err("browser closed"))?;
        self.rt.block_on(onecrawl_cdp::page::get_content(page)).map_err(py_err)
    }

    /// Set page HTML content.
    fn set_content(&self, html: &str) -> PyResult<()> {
        let guard = self.page.lock().map_err(py_err)?;
        let page = guard.as_ref().ok_or_else(|| py_err("browser closed"))?;
        self.rt.block_on(onecrawl_cdp::page::set_content(page, html)).map_err(py_err)
    }

    /// Take a viewport screenshot (PNG bytes).
    fn screenshot(&self) -> PyResult<Vec<u8>> {
        let guard = self.page.lock().map_err(py_err)?;
        let page = guard.as_ref().ok_or_else(|| py_err("browser closed"))?;
        self.rt.block_on(onecrawl_cdp::screenshot::screenshot_viewport(page)).map_err(py_err)
    }

    /// Take a full-page screenshot (PNG bytes).
    fn screenshot_full(&self) -> PyResult<Vec<u8>> {
        let guard = self.page.lock().map_err(py_err)?;
        let page = guard.as_ref().ok_or_else(|| py_err("browser closed"))?;
        self.rt.block_on(onecrawl_cdp::screenshot::screenshot_full(page)).map_err(py_err)
    }

    /// Screenshot a specific element by CSS selector.
    fn screenshot_element(&self, selector: &str) -> PyResult<Vec<u8>> {
        let guard = self.page.lock().map_err(py_err)?;
        let page = guard.as_ref().ok_or_else(|| py_err("browser closed"))?;
        self.rt.block_on(onecrawl_cdp::screenshot::screenshot_element(page, selector)).map_err(py_err)
    }

    /// Save page as PDF (bytes).
    fn pdf(&self) -> PyResult<Vec<u8>> {
        let guard = self.page.lock().map_err(py_err)?;
        let page = guard.as_ref().ok_or_else(|| py_err("browser closed"))?;
        self.rt.block_on(onecrawl_cdp::screenshot::pdf(page)).map_err(py_err)
    }

    /// Evaluate JavaScript. Returns JSON string.
    fn evaluate(&self, expression: &str) -> PyResult<String> {
        let guard = self.page.lock().map_err(py_err)?;
        let page = guard.as_ref().ok_or_else(|| py_err("browser closed"))?;
        let val = self.rt.block_on(onecrawl_cdp::page::evaluate_js(page, expression)).map_err(py_err)?;
        Ok(val.to_string())
    }

    /// Click an element by CSS selector.
    fn click(&self, selector: &str) -> PyResult<()> {
        let guard = self.page.lock().map_err(py_err)?;
        let page = guard.as_ref().ok_or_else(|| py_err("browser closed"))?;
        self.rt.block_on(onecrawl_cdp::element::click(page, selector)).map_err(py_err)
    }

    /// Double-click an element.
    fn double_click(&self, selector: &str) -> PyResult<()> {
        let guard = self.page.lock().map_err(py_err)?;
        let page = guard.as_ref().ok_or_else(|| py_err("browser closed"))?;
        self.rt.block_on(onecrawl_cdp::element::double_click(page, selector)).map_err(py_err)
    }

    /// Type text into an element (key-by-key).
    fn type_text(&self, selector: &str, text: &str) -> PyResult<()> {
        let guard = self.page.lock().map_err(py_err)?;
        let page = guard.as_ref().ok_or_else(|| py_err("browser closed"))?;
        self.rt.block_on(onecrawl_cdp::element::type_text(page, selector, text)).map_err(py_err)
    }

    /// Get text content of an element.
    fn get_text(&self, selector: &str) -> PyResult<String> {
        let guard = self.page.lock().map_err(py_err)?;
        let page = guard.as_ref().ok_or_else(|| py_err("browser closed"))?;
        self.rt.block_on(onecrawl_cdp::element::get_text(page, selector)).map_err(py_err)
    }

    /// Get attribute value from an element.
    fn get_attribute(&self, selector: &str, attribute: &str) -> PyResult<Option<String>> {
        let guard = self.page.lock().map_err(py_err)?;
        let page = guard.as_ref().ok_or_else(|| py_err("browser closed"))?;
        self.rt.block_on(onecrawl_cdp::element::get_attribute(page, selector, attribute)).map_err(py_err)
    }

    /// Hover over an element.
    fn hover(&self, selector: &str) -> PyResult<()> {
        let guard = self.page.lock().map_err(py_err)?;
        let page = guard.as_ref().ok_or_else(|| py_err("browser closed"))?;
        self.rt.block_on(onecrawl_cdp::element::hover(page, selector)).map_err(py_err)
    }

    /// Scroll element into view.
    fn scroll_into_view(&self, selector: &str) -> PyResult<()> {
        let guard = self.page.lock().map_err(py_err)?;
        let page = guard.as_ref().ok_or_else(|| py_err("browser closed"))?;
        self.rt.block_on(onecrawl_cdp::element::scroll_into_view(page, selector)).map_err(py_err)
    }

    /// Check a checkbox.
    fn check(&self, selector: &str) -> PyResult<()> {
        let guard = self.page.lock().map_err(py_err)?;
        let page = guard.as_ref().ok_or_else(|| py_err("browser closed"))?;
        self.rt.block_on(onecrawl_cdp::element::check(page, selector)).map_err(py_err)
    }

    /// Uncheck a checkbox.
    fn uncheck(&self, selector: &str) -> PyResult<()> {
        let guard = self.page.lock().map_err(py_err)?;
        let page = guard.as_ref().ok_or_else(|| py_err("browser closed"))?;
        self.rt.block_on(onecrawl_cdp::element::uncheck(page, selector)).map_err(py_err)
    }

    /// Select an option in a `<select>` by value.
    fn select_option(&self, selector: &str, value: &str) -> PyResult<()> {
        let guard = self.page.lock().map_err(py_err)?;
        let page = guard.as_ref().ok_or_else(|| py_err("browser closed"))?;
        self.rt.block_on(onecrawl_cdp::element::select_option(page, selector, value)).map_err(py_err)
    }

    /// Wait for a selector to appear (timeout in ms, default 30000).
    #[pyo3(signature = (selector, timeout_ms=30000))]
    fn wait_for_selector(&self, selector: &str, timeout_ms: u64) -> PyResult<()> {
        let guard = self.page.lock().map_err(py_err)?;
        let page = guard.as_ref().ok_or_else(|| py_err("browser closed"))?;
        self.rt.block_on(onecrawl_cdp::navigation::wait_for_selector(page, selector, timeout_ms)).map_err(py_err)
    }

    /// Wait for URL to contain pattern (timeout in ms, default 30000).
    #[pyo3(signature = (pattern, timeout_ms=30000))]
    fn wait_for_url(&self, pattern: &str, timeout_ms: u64) -> PyResult<()> {
        let guard = self.page.lock().map_err(py_err)?;
        let page = guard.as_ref().ok_or_else(|| py_err("browser closed"))?;
        self.rt.block_on(onecrawl_cdp::navigation::wait_for_url(page, pattern, timeout_ms)).map_err(py_err)
    }

    /// Go back in history.
    fn go_back(&self) -> PyResult<()> {
        let guard = self.page.lock().map_err(py_err)?;
        let page = guard.as_ref().ok_or_else(|| py_err("browser closed"))?;
        self.rt.block_on(onecrawl_cdp::navigation::go_back(page)).map_err(py_err)
    }

    /// Go forward in history.
    fn go_forward(&self) -> PyResult<()> {
        let guard = self.page.lock().map_err(py_err)?;
        let page = guard.as_ref().ok_or_else(|| py_err("browser closed"))?;
        self.rt.block_on(onecrawl_cdp::navigation::go_forward(page)).map_err(py_err)
    }

    /// Reload the page.
    fn reload(&self) -> PyResult<()> {
        let guard = self.page.lock().map_err(py_err)?;
        let page = guard.as_ref().ok_or_else(|| py_err("browser closed"))?;
        self.rt.block_on(onecrawl_cdp::navigation::reload(page)).map_err(py_err)
    }

    /// Inject stealth anti-detection patches. Returns (platform, hw_concurrency, device_memory).
    fn inject_stealth(&self) -> PyResult<(String, u32, u32)> {
        let guard = self.page.lock().map_err(py_err)?;
        let page = guard.as_ref().ok_or_else(|| py_err("browser closed"))?;
        let fp = onecrawl_cdp::generate_fingerprint();
        let script = onecrawl_cdp::get_stealth_init_script(&fp);
        self.rt.block_on(async {
            page.evaluate(script)
                .await
                .map_err(|e| py_err(format!("stealth injection failed: {e}")))?;
            Ok::<_, PyErr>(())
        })?;
        Ok((fp.platform.clone(), fp.hardware_concurrency, fp.device_memory))
    }

    /// Open a new page/tab and switch to it.
    #[pyo3(signature = (url=None))]
    fn new_page(&self, url: Option<&str>) -> PyResult<()> {
        let new_page = self.rt.block_on(
            self.session.new_page(url.unwrap_or("about:blank"))
        ).map_err(py_err)?;
        let mut guard = self.page.lock().map_err(py_err)?;
        *guard = Some(new_page);
        Ok(())
    }

    /// Wait for a specified number of milliseconds.
    fn wait(&self, ms: u64) -> PyResult<()> {
        self.rt.block_on(onecrawl_cdp::navigation::wait_ms(ms));
        Ok(())
    }

    /// Close the browser.
    fn close(&self) -> PyResult<()> {
        let mut guard = self.page.lock().map_err(py_err)?;
        *guard = None;
        Ok(())
    }

    // ──────────────── Cookie Management ────────────────

    /// Get all cookies (including httpOnly) via CDP. Returns JSON string.
    fn get_cookies(&self) -> PyResult<String> {
        let guard = self.page.lock().map_err(py_err)?;
        let page = guard.as_ref().ok_or_else(|| py_err("browser closed"))?;
        let cookies = self.rt.block_on(onecrawl_cdp::cookie::get_all_cookies(page)).map_err(py_err)?;
        serde_json::to_string(&cookies).map_err(py_err)
    }

    /// Set a cookie. Accepts a JSON string of cookie params.
    fn set_cookie(&self, params_json: &str) -> PyResult<()> {
        let guard = self.page.lock().map_err(py_err)?;
        let page = guard.as_ref().ok_or_else(|| py_err("browser closed"))?;
        let params: onecrawl_cdp::SetCookieParams = serde_json::from_str(params_json)
            .map_err(|e| py_err(format!("invalid cookie params: {e}")))?;
        self.rt.block_on(onecrawl_cdp::cookie::set_cookie(page, &params)).map_err(py_err)
    }

    /// Delete cookies by name (optional domain/path).
    #[pyo3(signature = (name, domain=None, path=None))]
    fn delete_cookies(&self, name: &str, domain: Option<&str>, path: Option<&str>) -> PyResult<()> {
        let guard = self.page.lock().map_err(py_err)?;
        let page = guard.as_ref().ok_or_else(|| py_err("browser closed"))?;
        self.rt.block_on(onecrawl_cdp::cookie::delete_cookies(page, name, domain, path)).map_err(py_err)
    }

    /// Clear all browser cookies.
    fn clear_cookies(&self) -> PyResult<()> {
        let guard = self.page.lock().map_err(py_err)?;
        let page = guard.as_ref().ok_or_else(|| py_err("browser closed"))?;
        self.rt.block_on(onecrawl_cdp::cookie::clear_cookies(page)).map_err(py_err)
    }

    // ──────────────── Keyboard ────────────────

    /// Press a key (keyDown + keyUp).
    fn press_key(&self, key: &str) -> PyResult<()> {
        let guard = self.page.lock().map_err(py_err)?;
        let page = guard.as_ref().ok_or_else(|| py_err("browser closed"))?;
        self.rt.block_on(onecrawl_cdp::keyboard::press_key(page, key)).map_err(py_err)
    }

    /// Send a keyboard shortcut (e.g., "Control+a", "Meta+c").
    fn keyboard_shortcut(&self, shortcut: &str) -> PyResult<()> {
        let guard = self.page.lock().map_err(py_err)?;
        let page = guard.as_ref().ok_or_else(|| py_err("browser closed"))?;
        self.rt.block_on(onecrawl_cdp::keyboard::keyboard_shortcut(page, shortcut)).map_err(py_err)
    }

    /// Hold a key down.
    fn key_down(&self, key: &str) -> PyResult<()> {
        let guard = self.page.lock().map_err(py_err)?;
        let page = guard.as_ref().ok_or_else(|| py_err("browser closed"))?;
        self.rt.block_on(onecrawl_cdp::keyboard::key_down(page, key)).map_err(py_err)
    }

    /// Release a key.
    fn key_up(&self, key: &str) -> PyResult<()> {
        let guard = self.page.lock().map_err(py_err)?;
        let page = guard.as_ref().ok_or_else(|| py_err("browser closed"))?;
        self.rt.block_on(onecrawl_cdp::keyboard::key_up(page, key)).map_err(py_err)
    }

    /// Fill an input field (clear + set value + fire events).
    fn fill(&self, selector: &str, value: &str) -> PyResult<()> {
        let guard = self.page.lock().map_err(py_err)?;
        let page = guard.as_ref().ok_or_else(|| py_err("browser closed"))?;
        self.rt.block_on(onecrawl_cdp::keyboard::fill(page, selector, value)).map_err(py_err)
    }

    // ──────────────── Advanced Input ────────────────

    /// Drag an element and drop onto another (CSS selectors).
    fn drag_and_drop(&self, source: &str, target: &str) -> PyResult<()> {
        let guard = self.page.lock().map_err(py_err)?;
        let page = guard.as_ref().ok_or_else(|| py_err("browser closed"))?;
        self.rt.block_on(onecrawl_cdp::input::drag_and_drop(page, source, target)).map_err(py_err)
    }

    /// Upload files to a `<input type="file">` element.
    fn upload_file(&self, selector: &str, file_paths: Vec<String>) -> PyResult<()> {
        let guard = self.page.lock().map_err(py_err)?;
        let page = guard.as_ref().ok_or_else(|| py_err("browser closed"))?;
        self.rt.block_on(onecrawl_cdp::input::set_file_input(page, selector, &file_paths)).map_err(py_err)
    }

    /// Get bounding box of an element. Returns (x, y, width, height).
    fn bounding_box(&self, selector: &str) -> PyResult<(f64, f64, f64, f64)> {
        let guard = self.page.lock().map_err(py_err)?;
        let page = guard.as_ref().ok_or_else(|| py_err("browser closed"))?;
        self.rt.block_on(onecrawl_cdp::input::bounding_box(page, selector)).map_err(py_err)
    }

    /// Tap an element (touch simulation).
    fn tap(&self, selector: &str) -> PyResult<()> {
        let guard = self.page.lock().map_err(py_err)?;
        let page = guard.as_ref().ok_or_else(|| py_err("browser closed"))?;
        self.rt.block_on(onecrawl_cdp::input::tap(page, selector)).map_err(py_err)
    }

    // ──── Emulation ────

    /// Set viewport dimensions and device emulation.
    #[pyo3(signature = (width, height, device_scale_factor=None, is_mobile=None, has_touch=None))]
    fn set_viewport(
        &self,
        width: u32,
        height: u32,
        device_scale_factor: Option<f64>,
        is_mobile: Option<bool>,
        has_touch: Option<bool>,
    ) -> PyResult<()> {
        let guard = self.page.lock().map_err(py_err)?;
        let page = guard.as_ref().ok_or_else(|| py_err("browser closed"))?;
        let vp = onecrawl_cdp::emulation::Viewport {
            width,
            height,
            device_scale_factor: device_scale_factor.unwrap_or(1.0),
            is_mobile: is_mobile.unwrap_or(false),
            has_touch: has_touch.unwrap_or(false),
        };
        self.rt.block_on(onecrawl_cdp::emulation::set_viewport(page, &vp)).map_err(py_err)
    }

    /// Set viewport from a device preset name.
    fn set_device(&self, device: &str) -> PyResult<()> {
        let guard = self.page.lock().map_err(py_err)?;
        let page = guard.as_ref().ok_or_else(|| py_err("browser closed"))?;
        let vp = match device.to_lowercase().as_str() {
            "desktop" => onecrawl_cdp::emulation::Viewport::desktop(),
            "iphone14" | "iphone_14" | "iphone" => onecrawl_cdp::emulation::Viewport::iphone_14(),
            "ipad" => onecrawl_cdp::emulation::Viewport::ipad(),
            "pixel7" | "pixel_7" | "pixel" => onecrawl_cdp::emulation::Viewport::pixel_7(),
            _ => return Err(py_err(format!("Unknown device: {device}"))),
        };
        self.rt.block_on(onecrawl_cdp::emulation::set_viewport(page, &vp)).map_err(py_err)
    }

    /// Clear viewport override.
    fn clear_viewport(&self) -> PyResult<()> {
        let guard = self.page.lock().map_err(py_err)?;
        let page = guard.as_ref().ok_or_else(|| py_err("browser closed"))?;
        self.rt.block_on(onecrawl_cdp::emulation::clear_viewport(page)).map_err(py_err)
    }

    /// Override user agent string.
    fn set_user_agent(&self, user_agent: &str) -> PyResult<()> {
        let guard = self.page.lock().map_err(py_err)?;
        let page = guard.as_ref().ok_or_else(|| py_err("browser closed"))?;
        self.rt.block_on(onecrawl_cdp::emulation::set_user_agent(page, user_agent)).map_err(py_err)
    }

    /// Set geolocation override.
    #[pyo3(signature = (latitude, longitude, accuracy=None))]
    fn set_geolocation(&self, latitude: f64, longitude: f64, accuracy: Option<f64>) -> PyResult<()> {
        let guard = self.page.lock().map_err(py_err)?;
        let page = guard.as_ref().ok_or_else(|| py_err("browser closed"))?;
        self.rt.block_on(onecrawl_cdp::emulation::set_geolocation(page, latitude, longitude, accuracy.unwrap_or(1.0))).map_err(py_err)
    }

    /// Emulate color scheme preference (dark/light).
    fn set_color_scheme(&self, scheme: &str) -> PyResult<()> {
        let guard = self.page.lock().map_err(py_err)?;
        let page = guard.as_ref().ok_or_else(|| py_err("browser closed"))?;
        self.rt.block_on(onecrawl_cdp::emulation::set_color_scheme(page, scheme)).map_err(py_err)
    }

    // ──── Network (advanced) ────

    /// Block specific resource types (e.g., ["Image", "Font"]).
    fn block_resources(&self, resource_types: Vec<String>) -> PyResult<()> {
        let guard = self.page.lock().map_err(py_err)?;
        let page = guard.as_ref().ok_or_else(|| py_err("browser closed"))?;
        let types: Vec<onecrawl_cdp::ResourceType> = resource_types
            .iter()
            .map(|s| serde_json::from_str(&format!("\"{}\"", s)))
            .collect::<std::result::Result<_, _>>()
            .map_err(|e| py_err(format!("Invalid resource type: {e}")))?;
        self.rt.block_on(onecrawl_cdp::network::block_resources(page, &types)).map_err(py_err)
    }

    // ──── Screenshot & PDF (with options) ────

    /// Take a screenshot with custom options.
    #[pyo3(signature = (format=None, quality=None, full_page=None))]
    fn screenshot_with_options(
        &self,
        format: Option<&str>,
        quality: Option<u32>,
        full_page: Option<bool>,
    ) -> PyResult<Vec<u8>> {
        let guard = self.page.lock().map_err(py_err)?;
        let page = guard.as_ref().ok_or_else(|| py_err("browser closed"))?;
        let fmt = match format {
            Some("jpeg") | Some("jpg") => onecrawl_cdp::ImageFormat::Jpeg,
            Some("webp") => onecrawl_cdp::ImageFormat::Webp,
            _ => onecrawl_cdp::ImageFormat::Png,
        };
        let opts = onecrawl_cdp::ScreenshotOptions {
            format: fmt,
            quality,
            full_page: full_page.unwrap_or(false),
        };
        self.rt.block_on(onecrawl_cdp::screenshot::screenshot_with_options(page, &opts)).map_err(py_err)
    }

    /// Generate PDF with custom options.
    #[pyo3(signature = (landscape=None, scale=None, paper_width=None, paper_height=None))]
    fn pdf_with_options(
        &self,
        landscape: Option<bool>,
        scale: Option<f64>,
        paper_width: Option<f64>,
        paper_height: Option<f64>,
    ) -> PyResult<Vec<u8>> {
        let guard = self.page.lock().map_err(py_err)?;
        let page = guard.as_ref().ok_or_else(|| py_err("browser closed"))?;
        let opts = onecrawl_cdp::PdfOptions {
            landscape: landscape.unwrap_or(false),
            scale: scale.unwrap_or(1.0),
            paper_width: paper_width.unwrap_or(8.5),
            paper_height: paper_height.unwrap_or(11.0),
        };
        self.rt.block_on(onecrawl_cdp::screenshot::pdf_with_options(page, &opts)).map_err(py_err)
    }

    // ──── Event Streaming ────

    /// Start event observation (console + errors). Call drain_events() to poll.
    fn start_event_stream(&self) -> PyResult<()> {
        let guard = self.page.lock().map_err(py_err)?;
        let page = guard.as_ref().ok_or_else(|| py_err("browser closed"))?;

        let stream = onecrawl_cdp::EventStream::new(256);
        let tx = stream.sender();

        self.rt.block_on(onecrawl_cdp::events::observe_console(page, tx.clone())).map_err(py_err)?;
        self.rt.block_on(onecrawl_cdp::events::observe_errors(page, tx.clone())).map_err(py_err)?;

        let mut es = self.event_stream.lock().map_err(py_err)?;
        *es = Some(stream);
        Ok(())
    }

    /// Drain buffered events. Returns JSON string with counts.
    fn drain_events(&self) -> PyResult<String> {
        let guard = self.page.lock().map_err(py_err)?;
        let page = guard.as_ref().ok_or_else(|| py_err("browser closed"))?;

        let es = self.event_stream.lock().map_err(py_err)?;
        let stream = es.as_ref().ok_or_else(|| py_err("event stream not started — call start_event_stream() first"))?;
        let tx = stream.sender();

        let console_count = self.rt.block_on(onecrawl_cdp::events::drain_console(page, &tx)).map_err(py_err)?;
        let error_count = self.rt.block_on(onecrawl_cdp::events::drain_errors(page, &tx)).map_err(py_err)?;

        Ok(serde_json::json!({
            "console_messages": console_count,
            "page_errors": error_count,
            "total": console_count + error_count,
        }).to_string())
    }

    /// Emit a custom event into the stream.
    fn emit_event(&self, name: &str, data: &str) -> PyResult<()> {
        let es = self.event_stream.lock().map_err(py_err)?;
        let stream = es.as_ref().ok_or_else(|| py_err("event stream not started"))?;
        let tx = stream.sender();
        let json_data: serde_json::Value = serde_json::from_str(data)
            .unwrap_or(serde_json::Value::String(data.to_string()));
        onecrawl_cdp::events::emit_custom(&tx, name, json_data).map_err(py_err)
    }

    // ── HAR Recording ──────────────────────────────────────────────

    /// Start HAR (HTTP Archive) recording on the current page.
    fn start_har_recording(&self) -> PyResult<()> {
        let guard = self.page.lock().map_err(py_err)?;
        let page = guard.as_ref().ok_or_else(|| py_err("browser closed"))?;
        let recorder = onecrawl_cdp::HarRecorder::new();
        self.rt.block_on(onecrawl_cdp::har::start_har_recording(page, &recorder)).map_err(py_err)?;
        let mut hr = self.har_recorder.lock().map_err(py_err)?;
        *hr = Some(recorder);
        Ok(())
    }

    /// Drain new HAR entries from the page. Returns the number of new entries.
    fn drain_har_entries(&self) -> PyResult<usize> {
        let guard = self.page.lock().map_err(py_err)?;
        let page = guard.as_ref().ok_or_else(|| py_err("browser closed"))?;
        let hr = self.har_recorder.lock().map_err(py_err)?;
        let recorder = hr.as_ref().ok_or_else(|| py_err("HAR recording not started"))?;
        self.rt.block_on(onecrawl_cdp::har::drain_har_entries(page, recorder)).map_err(py_err)
    }

    /// Export all HAR entries as HAR 1.2 JSON string.
    fn export_har(&self) -> PyResult<String> {
        let guard = self.page.lock().map_err(py_err)?;
        let page_url = if let Some(page) = guard.as_ref() {
            self.rt.block_on(page.url()).unwrap_or(None).unwrap_or_default()
        } else {
            String::new()
        };
        let hr = self.har_recorder.lock().map_err(py_err)?;
        let recorder = hr.as_ref().ok_or_else(|| py_err("HAR recording not started"))?;
        let har = self.rt.block_on(onecrawl_cdp::har::export_har(recorder, &page_url)).map_err(py_err)?;
        Ok(har.to_string())
    }

    // ── WebSocket Recording ────────────────────────────────────────

    /// Start WebSocket frame interception on the current page.
    fn start_ws_recording(&self) -> PyResult<()> {
        let guard = self.page.lock().map_err(py_err)?;
        let page = guard.as_ref().ok_or_else(|| py_err("browser closed"))?;
        let recorder = onecrawl_cdp::WsRecorder::new();
        self.rt.block_on(onecrawl_cdp::websocket::start_ws_recording(page, &recorder)).map_err(py_err)?;
        let mut wr = self.ws_recorder.lock().map_err(py_err)?;
        *wr = Some(recorder);
        Ok(())
    }

    /// Drain new WebSocket frames from the page. Returns the number of new frames.
    fn drain_ws_frames(&self) -> PyResult<usize> {
        let guard = self.page.lock().map_err(py_err)?;
        let page = guard.as_ref().ok_or_else(|| py_err("browser closed"))?;
        let wr = self.ws_recorder.lock().map_err(py_err)?;
        let recorder = wr.as_ref().ok_or_else(|| py_err("WS recording not started"))?;
        self.rt.block_on(onecrawl_cdp::websocket::drain_ws_frames(page, recorder)).map_err(py_err)
    }

    /// Export all captured WebSocket frames as JSON string.
    fn export_ws_frames(&self) -> PyResult<String> {
        let wr = self.ws_recorder.lock().map_err(py_err)?;
        let recorder = wr.as_ref().ok_or_else(|| py_err("WS recording not started"))?;
        let frames = self.rt.block_on(onecrawl_cdp::websocket::export_ws_frames(recorder)).map_err(py_err)?;
        Ok(frames.to_string())
    }

    /// Get the count of active WebSocket connections.
    fn active_ws_connections(&self) -> PyResult<usize> {
        let guard = self.page.lock().map_err(py_err)?;
        let page = guard.as_ref().ok_or_else(|| py_err("browser closed"))?;
        self.rt.block_on(onecrawl_cdp::websocket::active_ws_connections(page)).map_err(py_err)
    }

    // ── Code Coverage ──────────────────────────────────────────────

    /// Start JavaScript code coverage collection via CDP Profiler.
    fn start_js_coverage(&self) -> PyResult<()> {
        let guard = self.page.lock().map_err(py_err)?;
        let page = guard.as_ref().ok_or_else(|| py_err("browser closed"))?;
        self.rt.block_on(onecrawl_cdp::coverage::start_js_coverage(page)).map_err(py_err)
    }

    /// Stop JavaScript code coverage and return the report as JSON string.
    fn stop_js_coverage(&self) -> PyResult<String> {
        let guard = self.page.lock().map_err(py_err)?;
        let page = guard.as_ref().ok_or_else(|| py_err("browser closed"))?;
        let report = self.rt.block_on(onecrawl_cdp::coverage::stop_js_coverage(page)).map_err(py_err)?;
        serde_json::to_string(&report).map_err(py_err)
    }

    /// Start CSS coverage collection.
    fn start_css_coverage(&self) -> PyResult<()> {
        let guard = self.page.lock().map_err(py_err)?;
        let page = guard.as_ref().ok_or_else(|| py_err("browser closed"))?;
        self.rt.block_on(onecrawl_cdp::coverage::start_css_coverage(page)).map_err(py_err)
    }

    /// Get CSS coverage summary as JSON string.
    fn get_css_coverage(&self) -> PyResult<String> {
        let guard = self.page.lock().map_err(py_err)?;
        let page = guard.as_ref().ok_or_else(|| py_err("browser closed"))?;
        let report = self.rt.block_on(onecrawl_cdp::coverage::get_css_coverage(page)).map_err(py_err)?;
        Ok(report.to_string())
    }

    // ── Accessibility ──────────────────────────────────────────────

    /// Get the full accessibility tree as JSON.
    fn get_accessibility_tree(&self) -> PyResult<String> {
        let guard = self.page.lock().map_err(py_err)?;
        let page = guard.as_ref().ok_or_else(|| py_err("browser closed"))?;
        let result = self.rt.block_on(onecrawl_cdp::accessibility::get_accessibility_tree(page)).map_err(py_err)?;
        Ok(result.to_string())
    }

    /// Get accessibility info for a specific element.
    fn get_element_accessibility(&self, selector: &str) -> PyResult<String> {
        let guard = self.page.lock().map_err(py_err)?;
        let page = guard.as_ref().ok_or_else(|| py_err("browser closed"))?;
        let result = self.rt.block_on(onecrawl_cdp::accessibility::get_element_accessibility(page, selector)).map_err(py_err)?;
        Ok(result.to_string())
    }

    /// Run an accessibility audit and return the report as JSON.
    fn audit_accessibility(&self) -> PyResult<String> {
        let guard = self.page.lock().map_err(py_err)?;
        let page = guard.as_ref().ok_or_else(|| py_err("browser closed"))?;
        let result = self.rt.block_on(onecrawl_cdp::accessibility::audit_accessibility(page)).map_err(py_err)?;
        serde_json::to_string(&result).map_err(py_err)
    }

    // ── Network Throttling ─────────────────────────────────────────

    /// Set network throttling to a named profile.
    fn set_network_throttle(&self, profile: &str) -> PyResult<()> {
        let guard = self.page.lock().map_err(py_err)?;
        let page = guard.as_ref().ok_or_else(|| py_err("browser closed"))?;
        let p = py_parse_network_profile(profile)?;
        self.rt.block_on(onecrawl_cdp::throttle::set_network_conditions(page, p)).map_err(py_err)
    }

    /// Set custom network throttling conditions.
    fn set_network_throttle_custom(&self, download_kbps: f64, upload_kbps: f64, latency_ms: f64) -> PyResult<()> {
        let guard = self.page.lock().map_err(py_err)?;
        let page = guard.as_ref().ok_or_else(|| py_err("browser closed"))?;
        let profile = onecrawl_cdp::NetworkProfile::Custom { download_kbps, upload_kbps, latency_ms };
        self.rt.block_on(onecrawl_cdp::throttle::set_network_conditions(page, profile)).map_err(py_err)
    }

    /// Clear network throttling.
    fn clear_network_throttle(&self) -> PyResult<()> {
        let guard = self.page.lock().map_err(py_err)?;
        let page = guard.as_ref().ok_or_else(|| py_err("browser closed"))?;
        self.rt.block_on(onecrawl_cdp::throttle::clear_network_conditions(page)).map_err(py_err)
    }

    // ── Performance Tracing ────────────────────────────────────────

    /// Start performance tracing.
    fn start_tracing(&self) -> PyResult<()> {
        let guard = self.page.lock().map_err(py_err)?;
        let page = guard.as_ref().ok_or_else(|| py_err("browser closed"))?;
        self.rt.block_on(onecrawl_cdp::tracing_cdp::start_tracing(page)).map_err(py_err)
    }

    /// Stop tracing and return trace data as JSON.
    fn stop_tracing(&self) -> PyResult<String> {
        let guard = self.page.lock().map_err(py_err)?;
        let page = guard.as_ref().ok_or_else(|| py_err("browser closed"))?;
        let result = self.rt.block_on(onecrawl_cdp::tracing_cdp::stop_tracing(page)).map_err(py_err)?;
        Ok(result.to_string())
    }

    /// Get performance metrics as JSON.
    fn get_performance_metrics(&self) -> PyResult<String> {
        let guard = self.page.lock().map_err(py_err)?;
        let page = guard.as_ref().ok_or_else(|| py_err("browser closed"))?;
        let result = self.rt.block_on(onecrawl_cdp::tracing_cdp::get_performance_metrics(page)).map_err(py_err)?;
        serde_json::to_string(&result).map_err(py_err)
    }

    /// Get navigation timing data as JSON.
    fn get_navigation_timing(&self) -> PyResult<String> {
        let guard = self.page.lock().map_err(py_err)?;
        let page = guard.as_ref().ok_or_else(|| py_err("browser closed"))?;
        let result = self.rt.block_on(onecrawl_cdp::tracing_cdp::get_navigation_timing(page)).map_err(py_err)?;
        Ok(result.to_string())
    }

    /// Get resource timing entries as JSON.
    fn get_resource_timing(&self) -> PyResult<String> {
        let guard = self.page.lock().map_err(py_err)?;
        let page = guard.as_ref().ok_or_else(|| py_err("browser closed"))?;
        let result = self.rt.block_on(onecrawl_cdp::tracing_cdp::get_resource_timing(page)).map_err(py_err)?;
        serde_json::to_string(&result).map_err(py_err)
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
    m.add_class::<Browser>()?;
    Ok(())
}
