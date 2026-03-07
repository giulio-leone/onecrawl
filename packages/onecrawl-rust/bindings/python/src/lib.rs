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
        let store = onecrawl_storage::EncryptedStore::open(std::path::Path::new(path), password)
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
    rate_limiter: Arc<std::sync::Mutex<onecrawl_cdp::RateLimitState>>,
    retry_queue: Arc<std::sync::Mutex<onecrawl_cdp::RetryQueue>>,
    scheduler: Arc<std::sync::Mutex<onecrawl_cdp::Scheduler>>,
    session_pool: Arc<std::sync::Mutex<onecrawl_cdp::SessionPool>>,
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
        _ => Err(py_err(format!(
            "Unknown profile: {name}. Use: fast3g, slow3g, offline, regular4g, wifi"
        ))),
    }
}

#[pymethods]
impl Browser {
    /// Launch a new browser. `headless` defaults to True.
    #[staticmethod]
    #[pyo3(signature = (headless=true))]
    fn launch(headless: bool) -> PyResult<Self> {
        let rt = tokio::runtime::Runtime::new().map_err(py_err)?;
        let session = rt
            .block_on(async {
                if headless {
                    onecrawl_cdp::BrowserSession::launch_headless().await
                } else {
                    onecrawl_cdp::BrowserSession::launch_headed().await
                }
            })
            .map_err(py_err)?;
        let page = rt
            .block_on(session.new_page("about:blank"))
            .map_err(py_err)?;
        Ok(Self {
            rt: Arc::new(rt),
            session: Arc::new(session),
            page: Arc::new(std::sync::Mutex::new(Some(page))),
            event_stream: Arc::new(std::sync::Mutex::new(None)),
            har_recorder: Arc::new(std::sync::Mutex::new(None)),
            ws_recorder: Arc::new(std::sync::Mutex::new(None)),
            rate_limiter: Arc::new(std::sync::Mutex::new(onecrawl_cdp::RateLimitState::new(
                onecrawl_cdp::RateLimitConfig::default(),
            ))),
            retry_queue: Arc::new(std::sync::Mutex::new(onecrawl_cdp::RetryQueue::new(
                onecrawl_cdp::RetryConfig::default(),
            ))),
            scheduler: Arc::new(std::sync::Mutex::new(onecrawl_cdp::Scheduler::new())),
            session_pool: Arc::new(std::sync::Mutex::new(onecrawl_cdp::SessionPool::new(
                onecrawl_cdp::PoolConfig::default(),
            ))),
        })
    }

    /// Connect to existing browser via CDP WebSocket URL.
    #[staticmethod]
    fn connect(ws_url: &str) -> PyResult<Self> {
        let rt = tokio::runtime::Runtime::new().map_err(py_err)?;
        let session = rt
            .block_on(onecrawl_cdp::BrowserSession::connect(ws_url))
            .map_err(py_err)?;
        let page = rt
            .block_on(session.new_page("about:blank"))
            .map_err(py_err)?;
        Ok(Self {
            rt: Arc::new(rt),
            session: Arc::new(session),
            page: Arc::new(std::sync::Mutex::new(Some(page))),
            event_stream: Arc::new(std::sync::Mutex::new(None)),
            har_recorder: Arc::new(std::sync::Mutex::new(None)),
            ws_recorder: Arc::new(std::sync::Mutex::new(None)),
            rate_limiter: Arc::new(std::sync::Mutex::new(onecrawl_cdp::RateLimitState::new(
                onecrawl_cdp::RateLimitConfig::default(),
            ))),
            retry_queue: Arc::new(std::sync::Mutex::new(onecrawl_cdp::RetryQueue::new(
                onecrawl_cdp::RetryConfig::default(),
            ))),
            scheduler: Arc::new(std::sync::Mutex::new(onecrawl_cdp::Scheduler::new())),
            session_pool: Arc::new(std::sync::Mutex::new(onecrawl_cdp::SessionPool::new(
                onecrawl_cdp::PoolConfig::default(),
            ))),
        })
    }

    /// Navigate to a URL.
    fn goto(&self, url: &str) -> PyResult<()> {
        let guard = self.page.lock().map_err(py_err)?;
        let page = guard.as_ref().ok_or_else(|| py_err("browser closed"))?;
        self.rt
            .block_on(onecrawl_cdp::navigation::goto(page, url))
            .map_err(py_err)
    }

    /// Get current URL.
    fn get_url(&self) -> PyResult<String> {
        let guard = self.page.lock().map_err(py_err)?;
        let page = guard.as_ref().ok_or_else(|| py_err("browser closed"))?;
        self.rt
            .block_on(onecrawl_cdp::navigation::get_url(page))
            .map_err(py_err)
    }

    /// Get page title.
    fn get_title(&self) -> PyResult<String> {
        let guard = self.page.lock().map_err(py_err)?;
        let page = guard.as_ref().ok_or_else(|| py_err("browser closed"))?;
        self.rt
            .block_on(onecrawl_cdp::navigation::get_title(page))
            .map_err(py_err)
    }

    /// Get page HTML content.
    fn content(&self) -> PyResult<String> {
        let guard = self.page.lock().map_err(py_err)?;
        let page = guard.as_ref().ok_or_else(|| py_err("browser closed"))?;
        self.rt
            .block_on(onecrawl_cdp::page::get_content(page))
            .map_err(py_err)
    }

    /// Set page HTML content.
    fn set_content(&self, html: &str) -> PyResult<()> {
        let guard = self.page.lock().map_err(py_err)?;
        let page = guard.as_ref().ok_or_else(|| py_err("browser closed"))?;
        self.rt
            .block_on(onecrawl_cdp::page::set_content(page, html))
            .map_err(py_err)
    }

    /// Take a viewport screenshot (PNG bytes).
    fn screenshot(&self) -> PyResult<Vec<u8>> {
        let guard = self.page.lock().map_err(py_err)?;
        let page = guard.as_ref().ok_or_else(|| py_err("browser closed"))?;
        self.rt
            .block_on(onecrawl_cdp::screenshot::screenshot_viewport(page))
            .map_err(py_err)
    }

    /// Take a full-page screenshot (PNG bytes).
    fn screenshot_full(&self) -> PyResult<Vec<u8>> {
        let guard = self.page.lock().map_err(py_err)?;
        let page = guard.as_ref().ok_or_else(|| py_err("browser closed"))?;
        self.rt
            .block_on(onecrawl_cdp::screenshot::screenshot_full(page))
            .map_err(py_err)
    }

    /// Screenshot a specific element by CSS selector.
    fn screenshot_element(&self, selector: &str) -> PyResult<Vec<u8>> {
        let guard = self.page.lock().map_err(py_err)?;
        let page = guard.as_ref().ok_or_else(|| py_err("browser closed"))?;
        self.rt
            .block_on(onecrawl_cdp::screenshot::screenshot_element(page, selector))
            .map_err(py_err)
    }

    /// Save page as PDF (bytes).
    fn pdf(&self) -> PyResult<Vec<u8>> {
        let guard = self.page.lock().map_err(py_err)?;
        let page = guard.as_ref().ok_or_else(|| py_err("browser closed"))?;
        self.rt
            .block_on(onecrawl_cdp::screenshot::pdf(page))
            .map_err(py_err)
    }

    /// Evaluate JavaScript. Returns JSON string.
    fn evaluate(&self, expression: &str) -> PyResult<String> {
        let guard = self.page.lock().map_err(py_err)?;
        let page = guard.as_ref().ok_or_else(|| py_err("browser closed"))?;
        let val = self
            .rt
            .block_on(onecrawl_cdp::page::evaluate_js(page, expression))
            .map_err(py_err)?;
        Ok(val.to_string())
    }

    /// Click an element by CSS selector.
    fn click(&self, selector: &str) -> PyResult<()> {
        let guard = self.page.lock().map_err(py_err)?;
        let page = guard.as_ref().ok_or_else(|| py_err("browser closed"))?;
        self.rt
            .block_on(onecrawl_cdp::element::click(page, selector))
            .map_err(py_err)
    }

    /// Double-click an element.
    fn double_click(&self, selector: &str) -> PyResult<()> {
        let guard = self.page.lock().map_err(py_err)?;
        let page = guard.as_ref().ok_or_else(|| py_err("browser closed"))?;
        self.rt
            .block_on(onecrawl_cdp::element::double_click(page, selector))
            .map_err(py_err)
    }

    /// Type text into an element (key-by-key).
    fn type_text(&self, selector: &str, text: &str) -> PyResult<()> {
        let guard = self.page.lock().map_err(py_err)?;
        let page = guard.as_ref().ok_or_else(|| py_err("browser closed"))?;
        self.rt
            .block_on(onecrawl_cdp::element::type_text(page, selector, text))
            .map_err(py_err)
    }

    /// Get text content of an element.
    fn get_text(&self, selector: &str) -> PyResult<String> {
        let guard = self.page.lock().map_err(py_err)?;
        let page = guard.as_ref().ok_or_else(|| py_err("browser closed"))?;
        self.rt
            .block_on(onecrawl_cdp::element::get_text(page, selector))
            .map_err(py_err)
    }

    /// Get attribute value from an element.
    fn get_attribute(&self, selector: &str, attribute: &str) -> PyResult<Option<String>> {
        let guard = self.page.lock().map_err(py_err)?;
        let page = guard.as_ref().ok_or_else(|| py_err("browser closed"))?;
        self.rt
            .block_on(onecrawl_cdp::element::get_attribute(
                page, selector, attribute,
            ))
            .map_err(py_err)
    }

    /// Hover over an element.
    fn hover(&self, selector: &str) -> PyResult<()> {
        let guard = self.page.lock().map_err(py_err)?;
        let page = guard.as_ref().ok_or_else(|| py_err("browser closed"))?;
        self.rt
            .block_on(onecrawl_cdp::element::hover(page, selector))
            .map_err(py_err)
    }

    /// Scroll element into view.
    fn scroll_into_view(&self, selector: &str) -> PyResult<()> {
        let guard = self.page.lock().map_err(py_err)?;
        let page = guard.as_ref().ok_or_else(|| py_err("browser closed"))?;
        self.rt
            .block_on(onecrawl_cdp::element::scroll_into_view(page, selector))
            .map_err(py_err)
    }

    /// Check a checkbox.
    fn check(&self, selector: &str) -> PyResult<()> {
        let guard = self.page.lock().map_err(py_err)?;
        let page = guard.as_ref().ok_or_else(|| py_err("browser closed"))?;
        self.rt
            .block_on(onecrawl_cdp::element::check(page, selector))
            .map_err(py_err)
    }

    /// Uncheck a checkbox.
    fn uncheck(&self, selector: &str) -> PyResult<()> {
        let guard = self.page.lock().map_err(py_err)?;
        let page = guard.as_ref().ok_or_else(|| py_err("browser closed"))?;
        self.rt
            .block_on(onecrawl_cdp::element::uncheck(page, selector))
            .map_err(py_err)
    }

    /// Select an option in a `<select>` by value.
    fn select_option(&self, selector: &str, value: &str) -> PyResult<()> {
        let guard = self.page.lock().map_err(py_err)?;
        let page = guard.as_ref().ok_or_else(|| py_err("browser closed"))?;
        self.rt
            .block_on(onecrawl_cdp::element::select_option(page, selector, value))
            .map_err(py_err)
    }

    /// Wait for a selector to appear (timeout in ms, default 30000).
    #[pyo3(signature = (selector, timeout_ms=30000))]
    fn wait_for_selector(&self, selector: &str, timeout_ms: u64) -> PyResult<()> {
        let guard = self.page.lock().map_err(py_err)?;
        let page = guard.as_ref().ok_or_else(|| py_err("browser closed"))?;
        self.rt
            .block_on(onecrawl_cdp::navigation::wait_for_selector(
                page, selector, timeout_ms,
            ))
            .map_err(py_err)
    }

    /// Wait for URL to contain pattern (timeout in ms, default 30000).
    #[pyo3(signature = (pattern, timeout_ms=30000))]
    fn wait_for_url(&self, pattern: &str, timeout_ms: u64) -> PyResult<()> {
        let guard = self.page.lock().map_err(py_err)?;
        let page = guard.as_ref().ok_or_else(|| py_err("browser closed"))?;
        self.rt
            .block_on(onecrawl_cdp::navigation::wait_for_url(
                page, pattern, timeout_ms,
            ))
            .map_err(py_err)
    }

    /// Go back in history.
    fn go_back(&self) -> PyResult<()> {
        let guard = self.page.lock().map_err(py_err)?;
        let page = guard.as_ref().ok_or_else(|| py_err("browser closed"))?;
        self.rt
            .block_on(onecrawl_cdp::navigation::go_back(page))
            .map_err(py_err)
    }

    /// Go forward in history.
    fn go_forward(&self) -> PyResult<()> {
        let guard = self.page.lock().map_err(py_err)?;
        let page = guard.as_ref().ok_or_else(|| py_err("browser closed"))?;
        self.rt
            .block_on(onecrawl_cdp::navigation::go_forward(page))
            .map_err(py_err)
    }

    /// Reload the page.
    fn reload(&self) -> PyResult<()> {
        let guard = self.page.lock().map_err(py_err)?;
        let page = guard.as_ref().ok_or_else(|| py_err("browser closed"))?;
        self.rt
            .block_on(onecrawl_cdp::navigation::reload(page))
            .map_err(py_err)
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
        Ok((
            fp.platform.clone(),
            fp.hardware_concurrency,
            fp.device_memory,
        ))
    }

    /// Open a new page/tab and switch to it.
    #[pyo3(signature = (url=None))]
    fn new_page(&self, url: Option<&str>) -> PyResult<()> {
        let new_page = self
            .rt
            .block_on(self.session.new_page(url.unwrap_or("about:blank")))
            .map_err(py_err)?;
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
        let cookies = self
            .rt
            .block_on(onecrawl_cdp::cookie::get_all_cookies(page))
            .map_err(py_err)?;
        serde_json::to_string(&cookies).map_err(py_err)
    }

    /// Set a cookie. Accepts a JSON string of cookie params.
    fn set_cookie(&self, params_json: &str) -> PyResult<()> {
        let guard = self.page.lock().map_err(py_err)?;
        let page = guard.as_ref().ok_or_else(|| py_err("browser closed"))?;
        let params: onecrawl_cdp::SetCookieParams = serde_json::from_str(params_json)
            .map_err(|e| py_err(format!("invalid cookie params: {e}")))?;
        self.rt
            .block_on(onecrawl_cdp::cookie::set_cookie(page, &params))
            .map_err(py_err)
    }

    /// Delete cookies by name (optional domain/path).
    #[pyo3(signature = (name, domain=None, path=None))]
    fn delete_cookies(&self, name: &str, domain: Option<&str>, path: Option<&str>) -> PyResult<()> {
        let guard = self.page.lock().map_err(py_err)?;
        let page = guard.as_ref().ok_or_else(|| py_err("browser closed"))?;
        self.rt
            .block_on(onecrawl_cdp::cookie::delete_cookies(
                page, name, domain, path,
            ))
            .map_err(py_err)
    }

    /// Clear all browser cookies.
    fn clear_cookies(&self) -> PyResult<()> {
        let guard = self.page.lock().map_err(py_err)?;
        let page = guard.as_ref().ok_or_else(|| py_err("browser closed"))?;
        self.rt
            .block_on(onecrawl_cdp::cookie::clear_cookies(page))
            .map_err(py_err)
    }

    // ──────────────── Keyboard ────────────────

    /// Press a key (keyDown + keyUp).
    fn press_key(&self, key: &str) -> PyResult<()> {
        let guard = self.page.lock().map_err(py_err)?;
        let page = guard.as_ref().ok_or_else(|| py_err("browser closed"))?;
        self.rt
            .block_on(onecrawl_cdp::keyboard::press_key(page, key))
            .map_err(py_err)
    }

    /// Send a keyboard shortcut (e.g., "Control+a", "Meta+c").
    fn keyboard_shortcut(&self, shortcut: &str) -> PyResult<()> {
        let guard = self.page.lock().map_err(py_err)?;
        let page = guard.as_ref().ok_or_else(|| py_err("browser closed"))?;
        self.rt
            .block_on(onecrawl_cdp::keyboard::keyboard_shortcut(page, shortcut))
            .map_err(py_err)
    }

    /// Hold a key down.
    fn key_down(&self, key: &str) -> PyResult<()> {
        let guard = self.page.lock().map_err(py_err)?;
        let page = guard.as_ref().ok_or_else(|| py_err("browser closed"))?;
        self.rt
            .block_on(onecrawl_cdp::keyboard::key_down(page, key))
            .map_err(py_err)
    }

    /// Release a key.
    fn key_up(&self, key: &str) -> PyResult<()> {
        let guard = self.page.lock().map_err(py_err)?;
        let page = guard.as_ref().ok_or_else(|| py_err("browser closed"))?;
        self.rt
            .block_on(onecrawl_cdp::keyboard::key_up(page, key))
            .map_err(py_err)
    }

    /// Fill an input field (clear + set value + fire events).
    fn fill(&self, selector: &str, value: &str) -> PyResult<()> {
        let guard = self.page.lock().map_err(py_err)?;
        let page = guard.as_ref().ok_or_else(|| py_err("browser closed"))?;
        self.rt
            .block_on(onecrawl_cdp::keyboard::fill(page, selector, value))
            .map_err(py_err)
    }

    // ──────────────── Advanced Input ────────────────

    /// Drag an element and drop onto another (CSS selectors).
    fn drag_and_drop(&self, source: &str, target: &str) -> PyResult<()> {
        let guard = self.page.lock().map_err(py_err)?;
        let page = guard.as_ref().ok_or_else(|| py_err("browser closed"))?;
        self.rt
            .block_on(onecrawl_cdp::input::drag_and_drop(page, source, target))
            .map_err(py_err)
    }

    /// Upload files to a `<input type="file">` element.
    fn upload_file(&self, selector: &str, file_paths: Vec<String>) -> PyResult<()> {
        let guard = self.page.lock().map_err(py_err)?;
        let page = guard.as_ref().ok_or_else(|| py_err("browser closed"))?;
        self.rt
            .block_on(onecrawl_cdp::input::set_file_input(
                page,
                selector,
                &file_paths,
            ))
            .map_err(py_err)
    }

    /// Get bounding box of an element. Returns (x, y, width, height).
    fn bounding_box(&self, selector: &str) -> PyResult<(f64, f64, f64, f64)> {
        let guard = self.page.lock().map_err(py_err)?;
        let page = guard.as_ref().ok_or_else(|| py_err("browser closed"))?;
        self.rt
            .block_on(onecrawl_cdp::input::bounding_box(page, selector))
            .map_err(py_err)
    }

    /// Tap an element (touch simulation).
    fn tap(&self, selector: &str) -> PyResult<()> {
        let guard = self.page.lock().map_err(py_err)?;
        let page = guard.as_ref().ok_or_else(|| py_err("browser closed"))?;
        self.rt
            .block_on(onecrawl_cdp::input::tap(page, selector))
            .map_err(py_err)
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
        self.rt
            .block_on(onecrawl_cdp::emulation::set_viewport(page, &vp))
            .map_err(py_err)
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
        self.rt
            .block_on(onecrawl_cdp::emulation::set_viewport(page, &vp))
            .map_err(py_err)
    }

    /// Clear viewport override.
    fn clear_viewport(&self) -> PyResult<()> {
        let guard = self.page.lock().map_err(py_err)?;
        let page = guard.as_ref().ok_or_else(|| py_err("browser closed"))?;
        self.rt
            .block_on(onecrawl_cdp::emulation::clear_viewport(page))
            .map_err(py_err)
    }

    /// Override user agent string.
    fn set_user_agent(&self, user_agent: &str) -> PyResult<()> {
        let guard = self.page.lock().map_err(py_err)?;
        let page = guard.as_ref().ok_or_else(|| py_err("browser closed"))?;
        self.rt
            .block_on(onecrawl_cdp::emulation::set_user_agent(page, user_agent))
            .map_err(py_err)
    }

    /// Set geolocation override.
    #[pyo3(signature = (latitude, longitude, accuracy=None))]
    fn set_geolocation(
        &self,
        latitude: f64,
        longitude: f64,
        accuracy: Option<f64>,
    ) -> PyResult<()> {
        let guard = self.page.lock().map_err(py_err)?;
        let page = guard.as_ref().ok_or_else(|| py_err("browser closed"))?;
        self.rt
            .block_on(onecrawl_cdp::emulation::set_geolocation(
                page,
                latitude,
                longitude,
                accuracy.unwrap_or(1.0),
            ))
            .map_err(py_err)
    }

    /// Emulate color scheme preference (dark/light).
    fn set_color_scheme(&self, scheme: &str) -> PyResult<()> {
        let guard = self.page.lock().map_err(py_err)?;
        let page = guard.as_ref().ok_or_else(|| py_err("browser closed"))?;
        self.rt
            .block_on(onecrawl_cdp::emulation::set_color_scheme(page, scheme))
            .map_err(py_err)
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
        self.rt
            .block_on(onecrawl_cdp::network::block_resources(page, &types))
            .map_err(py_err)
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
        self.rt
            .block_on(onecrawl_cdp::screenshot::screenshot_with_options(
                page, &opts,
            ))
            .map_err(py_err)
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
        self.rt
            .block_on(onecrawl_cdp::screenshot::pdf_with_options(page, &opts))
            .map_err(py_err)
    }

    // ──── Event Streaming ────

    /// Start event observation (console + errors). Call drain_events() to poll.
    fn start_event_stream(&self) -> PyResult<()> {
        let guard = self.page.lock().map_err(py_err)?;
        let page = guard.as_ref().ok_or_else(|| py_err("browser closed"))?;

        let stream = onecrawl_cdp::EventStream::new(256);
        let tx = stream.sender();

        self.rt
            .block_on(onecrawl_cdp::events::observe_console(page, tx.clone()))
            .map_err(py_err)?;
        self.rt
            .block_on(onecrawl_cdp::events::observe_errors(page, tx.clone()))
            .map_err(py_err)?;

        let mut es = self.event_stream.lock().map_err(py_err)?;
        *es = Some(stream);
        Ok(())
    }

    /// Drain buffered events. Returns JSON string with counts.
    fn drain_events(&self) -> PyResult<String> {
        let guard = self.page.lock().map_err(py_err)?;
        let page = guard.as_ref().ok_or_else(|| py_err("browser closed"))?;

        let es = self.event_stream.lock().map_err(py_err)?;
        let stream = es
            .as_ref()
            .ok_or_else(|| py_err("event stream not started — call start_event_stream() first"))?;
        let tx = stream.sender();

        let console_count = self
            .rt
            .block_on(onecrawl_cdp::events::drain_console(page, &tx))
            .map_err(py_err)?;
        let error_count = self
            .rt
            .block_on(onecrawl_cdp::events::drain_errors(page, &tx))
            .map_err(py_err)?;

        Ok(serde_json::json!({
            "console_messages": console_count,
            "page_errors": error_count,
            "total": console_count + error_count,
        })
        .to_string())
    }

    /// Emit a custom event into the stream.
    fn emit_event(&self, name: &str, data: &str) -> PyResult<()> {
        let es = self.event_stream.lock().map_err(py_err)?;
        let stream = es
            .as_ref()
            .ok_or_else(|| py_err("event stream not started"))?;
        let tx = stream.sender();
        let json_data: serde_json::Value =
            serde_json::from_str(data).unwrap_or(serde_json::Value::String(data.to_string()));
        onecrawl_cdp::events::emit_custom(&tx, name, json_data).map_err(py_err)
    }

    // ── HAR Recording ──────────────────────────────────────────────

    /// Start HAR (HTTP Archive) recording on the current page.
    fn start_har_recording(&self) -> PyResult<()> {
        let guard = self.page.lock().map_err(py_err)?;
        let page = guard.as_ref().ok_or_else(|| py_err("browser closed"))?;
        let recorder = onecrawl_cdp::HarRecorder::new();
        self.rt
            .block_on(onecrawl_cdp::har::start_har_recording(page, &recorder))
            .map_err(py_err)?;
        let mut hr = self.har_recorder.lock().map_err(py_err)?;
        *hr = Some(recorder);
        Ok(())
    }

    /// Drain new HAR entries from the page. Returns the number of new entries.
    fn drain_har_entries(&self) -> PyResult<usize> {
        let guard = self.page.lock().map_err(py_err)?;
        let page = guard.as_ref().ok_or_else(|| py_err("browser closed"))?;
        let hr = self.har_recorder.lock().map_err(py_err)?;
        let recorder = hr
            .as_ref()
            .ok_or_else(|| py_err("HAR recording not started"))?;
        self.rt
            .block_on(onecrawl_cdp::har::drain_har_entries(page, recorder))
            .map_err(py_err)
    }

    /// Export all HAR entries as HAR 1.2 JSON string.
    fn export_har(&self) -> PyResult<String> {
        let guard = self.page.lock().map_err(py_err)?;
        let page_url = if let Some(page) = guard.as_ref() {
            self.rt
                .block_on(page.url())
                .unwrap_or(None)
                .unwrap_or_default()
        } else {
            String::new()
        };
        let hr = self.har_recorder.lock().map_err(py_err)?;
        let recorder = hr
            .as_ref()
            .ok_or_else(|| py_err("HAR recording not started"))?;
        let har = self
            .rt
            .block_on(onecrawl_cdp::har::export_har(recorder, &page_url))
            .map_err(py_err)?;
        Ok(har.to_string())
    }

    // ── WebSocket Recording ────────────────────────────────────────

    /// Start WebSocket frame interception on the current page.
    fn start_ws_recording(&self) -> PyResult<()> {
        let guard = self.page.lock().map_err(py_err)?;
        let page = guard.as_ref().ok_or_else(|| py_err("browser closed"))?;
        let recorder = onecrawl_cdp::WsRecorder::new();
        self.rt
            .block_on(onecrawl_cdp::websocket::start_ws_recording(page, &recorder))
            .map_err(py_err)?;
        let mut wr = self.ws_recorder.lock().map_err(py_err)?;
        *wr = Some(recorder);
        Ok(())
    }

    /// Drain new WebSocket frames from the page. Returns the number of new frames.
    fn drain_ws_frames(&self) -> PyResult<usize> {
        let guard = self.page.lock().map_err(py_err)?;
        let page = guard.as_ref().ok_or_else(|| py_err("browser closed"))?;
        let wr = self.ws_recorder.lock().map_err(py_err)?;
        let recorder = wr
            .as_ref()
            .ok_or_else(|| py_err("WS recording not started"))?;
        self.rt
            .block_on(onecrawl_cdp::websocket::drain_ws_frames(page, recorder))
            .map_err(py_err)
    }

    /// Export all captured WebSocket frames as JSON string.
    fn export_ws_frames(&self) -> PyResult<String> {
        let wr = self.ws_recorder.lock().map_err(py_err)?;
        let recorder = wr
            .as_ref()
            .ok_or_else(|| py_err("WS recording not started"))?;
        let frames = self
            .rt
            .block_on(onecrawl_cdp::websocket::export_ws_frames(recorder))
            .map_err(py_err)?;
        Ok(frames.to_string())
    }

    /// Get the count of active WebSocket connections.
    fn active_ws_connections(&self) -> PyResult<usize> {
        let guard = self.page.lock().map_err(py_err)?;
        let page = guard.as_ref().ok_or_else(|| py_err("browser closed"))?;
        self.rt
            .block_on(onecrawl_cdp::websocket::active_ws_connections(page))
            .map_err(py_err)
    }

    // ── Console Interception ──────────────────────────────────────

    /// Start capturing console messages.
    fn start_console_capture(&self) -> PyResult<()> {
        let guard = self.page.lock().map_err(py_err)?;
        let page = guard.as_ref().ok_or_else(|| py_err("browser closed"))?;
        self.rt
            .block_on(onecrawl_cdp::console::start_console_capture(page))
            .map_err(py_err)
    }

    /// Drain captured console entries as JSON string.
    fn drain_console_entries(&self) -> PyResult<String> {
        let guard = self.page.lock().map_err(py_err)?;
        let page = guard.as_ref().ok_or_else(|| py_err("browser closed"))?;
        let entries = self
            .rt
            .block_on(onecrawl_cdp::console::drain_console_entries(page))
            .map_err(py_err)?;
        serde_json::to_string(&entries).map_err(py_err)
    }

    /// Clear the console capture buffer.
    fn clear_console(&self) -> PyResult<()> {
        let guard = self.page.lock().map_err(py_err)?;
        let page = guard.as_ref().ok_or_else(|| py_err("browser closed"))?;
        self.rt
            .block_on(onecrawl_cdp::console::clear_console(page))
            .map_err(py_err)
    }

    // ── Dialog Handling ───────────────────────────────────────────

    /// Set dialog auto-handler.
    #[pyo3(signature = (accept, prompt_text=None))]
    fn set_dialog_handler(&self, accept: bool, prompt_text: Option<&str>) -> PyResult<()> {
        let guard = self.page.lock().map_err(py_err)?;
        let page = guard.as_ref().ok_or_else(|| py_err("browser closed"))?;
        self.rt
            .block_on(onecrawl_cdp::dialog::set_dialog_handler(
                page,
                accept,
                prompt_text,
            ))
            .map_err(py_err)
    }

    /// Get dialog history as JSON string.
    fn get_dialog_history(&self) -> PyResult<String> {
        let guard = self.page.lock().map_err(py_err)?;
        let page = guard.as_ref().ok_or_else(|| py_err("browser closed"))?;
        let events = self
            .rt
            .block_on(onecrawl_cdp::dialog::get_dialog_history(page))
            .map_err(py_err)?;
        serde_json::to_string(&events).map_err(py_err)
    }

    /// Clear dialog history.
    fn clear_dialog_history(&self) -> PyResult<()> {
        let guard = self.page.lock().map_err(py_err)?;
        let page = guard.as_ref().ok_or_else(|| py_err("browser closed"))?;
        self.rt
            .block_on(onecrawl_cdp::dialog::clear_dialog_history(page))
            .map_err(py_err)
    }

    // ── Service Workers ───────────────────────────────────────────

    /// Get all registered service workers as JSON string.
    fn get_service_workers(&self) -> PyResult<String> {
        let guard = self.page.lock().map_err(py_err)?;
        let page = guard.as_ref().ok_or_else(|| py_err("browser closed"))?;
        let workers = self
            .rt
            .block_on(onecrawl_cdp::workers::get_service_workers(page))
            .map_err(py_err)?;
        serde_json::to_string(&workers).map_err(py_err)
    }

    /// Unregister all service workers.
    fn unregister_service_workers(&self) -> PyResult<usize> {
        let guard = self.page.lock().map_err(py_err)?;
        let page = guard.as_ref().ok_or_else(|| py_err("browser closed"))?;
        self.rt
            .block_on(onecrawl_cdp::workers::unregister_service_workers(page))
            .map_err(py_err)
    }

    /// Get worker info as JSON string.
    fn get_worker_info(&self) -> PyResult<String> {
        let guard = self.page.lock().map_err(py_err)?;
        let page = guard.as_ref().ok_or_else(|| py_err("browser closed"))?;
        let info = self
            .rt
            .block_on(onecrawl_cdp::workers::get_worker_info(page))
            .map_err(py_err)?;
        Ok(info.to_string())
    }

    // ── DOM Observer ──────────────────────────────────────────────

    /// Start observing DOM mutations (optional CSS selector target).
    fn start_dom_observer(&self, selector: Option<&str>) -> PyResult<()> {
        let guard = self.page.lock().map_err(py_err)?;
        let page = guard.as_ref().ok_or_else(|| py_err("browser closed"))?;
        self.rt
            .block_on(onecrawl_cdp::dom_observer::start_dom_observer(
                page, selector,
            ))
            .map_err(py_err)
    }

    /// Drain accumulated DOM mutations as JSON string.
    fn drain_dom_mutations(&self) -> PyResult<String> {
        let guard = self.page.lock().map_err(py_err)?;
        let page = guard.as_ref().ok_or_else(|| py_err("browser closed"))?;
        let mutations = self
            .rt
            .block_on(onecrawl_cdp::dom_observer::drain_dom_mutations(page))
            .map_err(py_err)?;
        serde_json::to_string(&mutations).map_err(py_err)
    }

    /// Stop the DOM observer.
    fn stop_dom_observer(&self) -> PyResult<()> {
        let guard = self.page.lock().map_err(py_err)?;
        let page = guard.as_ref().ok_or_else(|| py_err("browser closed"))?;
        self.rt
            .block_on(onecrawl_cdp::dom_observer::stop_dom_observer(page))
            .map_err(py_err)
    }

    /// Get a snapshot of the current DOM as HTML string.
    fn get_dom_snapshot(&self, selector: Option<&str>) -> PyResult<String> {
        let guard = self.page.lock().map_err(py_err)?;
        let page = guard.as_ref().ok_or_else(|| py_err("browser closed"))?;
        self.rt
            .block_on(onecrawl_cdp::dom_observer::get_dom_snapshot(page, selector))
            .map_err(py_err)
    }

    // ── Iframe Management ─────────────────────────────────────────

    /// List all iframes on the page as JSON string.
    fn list_iframes(&self) -> PyResult<String> {
        let guard = self.page.lock().map_err(py_err)?;
        let page = guard.as_ref().ok_or_else(|| py_err("browser closed"))?;
        let iframes = self
            .rt
            .block_on(onecrawl_cdp::iframe::list_iframes(page))
            .map_err(py_err)?;
        serde_json::to_string(&iframes).map_err(py_err)
    }

    /// Execute JavaScript inside a specific iframe by index.
    fn eval_in_iframe(&self, index: usize, expression: &str) -> PyResult<String> {
        let guard = self.page.lock().map_err(py_err)?;
        let page = guard.as_ref().ok_or_else(|| py_err("browser closed"))?;
        let val = self
            .rt
            .block_on(onecrawl_cdp::iframe::eval_in_iframe(
                page, index, expression,
            ))
            .map_err(py_err)?;
        serde_json::to_string(&val).map_err(py_err)
    }

    /// Get the inner HTML content of an iframe by index.
    fn get_iframe_content(&self, index: usize) -> PyResult<String> {
        let guard = self.page.lock().map_err(py_err)?;
        let page = guard.as_ref().ok_or_else(|| py_err("browser closed"))?;
        self.rt
            .block_on(onecrawl_cdp::iframe::get_iframe_content(page, index))
            .map_err(py_err)
    }

    // ── Print / PDF (Enhanced) ────────────────────────────────────

    /// Generate PDF with detailed options (JSON string). Returns PDF bytes.
    fn print_to_pdf(&self, options: Option<&str>) -> PyResult<Vec<u8>> {
        let guard = self.page.lock().map_err(py_err)?;
        let page = guard.as_ref().ok_or_else(|| py_err("browser closed"))?;
        let opts: onecrawl_cdp::DetailedPdfOptions = match options {
            Some(json) => serde_json::from_str(json).map_err(py_err)?,
            None => Default::default(),
        };
        self.rt
            .block_on(onecrawl_cdp::print::print_to_pdf(page, &opts))
            .map_err(py_err)
    }

    /// Get page print preview metrics as JSON string.
    fn get_print_metrics(&self) -> PyResult<String> {
        let guard = self.page.lock().map_err(py_err)?;
        let page = guard.as_ref().ok_or_else(|| py_err("browser closed"))?;
        let val = self
            .rt
            .block_on(onecrawl_cdp::print::get_print_metrics(page))
            .map_err(py_err)?;
        serde_json::to_string(&val).map_err(py_err)
    }

    // ── Web Storage ───────────────────────────────────────────────

    /// Get all localStorage contents as JSON string.
    fn get_local_storage(&self) -> PyResult<String> {
        let guard = self.page.lock().map_err(py_err)?;
        let page = guard.as_ref().ok_or_else(|| py_err("browser closed"))?;
        let data = self
            .rt
            .block_on(onecrawl_cdp::web_storage::get_local_storage(page))
            .map_err(py_err)?;
        Ok(data.to_string())
    }

    /// Set a localStorage item.
    fn set_local_storage(&self, key: &str, value: &str) -> PyResult<()> {
        let guard = self.page.lock().map_err(py_err)?;
        let page = guard.as_ref().ok_or_else(|| py_err("browser closed"))?;
        self.rt
            .block_on(onecrawl_cdp::web_storage::set_local_storage(
                page, key, value,
            ))
            .map_err(py_err)
    }

    /// Clear all localStorage.
    fn clear_local_storage(&self) -> PyResult<()> {
        let guard = self.page.lock().map_err(py_err)?;
        let page = guard.as_ref().ok_or_else(|| py_err("browser closed"))?;
        self.rt
            .block_on(onecrawl_cdp::web_storage::clear_local_storage(page))
            .map_err(py_err)
    }

    /// Get all sessionStorage contents as JSON string.
    fn get_session_storage(&self) -> PyResult<String> {
        let guard = self.page.lock().map_err(py_err)?;
        let page = guard.as_ref().ok_or_else(|| py_err("browser closed"))?;
        let data = self
            .rt
            .block_on(onecrawl_cdp::web_storage::get_session_storage(page))
            .map_err(py_err)?;
        Ok(data.to_string())
    }

    /// Set a sessionStorage item.
    fn set_session_storage(&self, key: &str, value: &str) -> PyResult<()> {
        let guard = self.page.lock().map_err(py_err)?;
        let page = guard.as_ref().ok_or_else(|| py_err("browser closed"))?;
        self.rt
            .block_on(onecrawl_cdp::web_storage::set_session_storage(
                page, key, value,
            ))
            .map_err(py_err)
    }

    /// Clear all sessionStorage.
    fn clear_session_storage(&self) -> PyResult<()> {
        let guard = self.page.lock().map_err(py_err)?;
        let page = guard.as_ref().ok_or_else(|| py_err("browser closed"))?;
        self.rt
            .block_on(onecrawl_cdp::web_storage::clear_session_storage(page))
            .map_err(py_err)
    }

    /// Get IndexedDB database names as JSON string.
    fn get_indexeddb_databases(&self) -> PyResult<String> {
        let guard = self.page.lock().map_err(py_err)?;
        let page = guard.as_ref().ok_or_else(|| py_err("browser closed"))?;
        let names = self
            .rt
            .block_on(onecrawl_cdp::web_storage::get_indexeddb_databases(page))
            .map_err(py_err)?;
        serde_json::to_string(&names).map_err(py_err)
    }

    /// Clear all site data.
    fn clear_site_data(&self) -> PyResult<()> {
        let guard = self.page.lock().map_err(py_err)?;
        let page = guard.as_ref().ok_or_else(|| py_err("browser closed"))?;
        self.rt
            .block_on(onecrawl_cdp::web_storage::clear_site_data(page))
            .map_err(py_err)
    }

    // ── Code Coverage ──────────────────────────────────────────────

    /// Start JavaScript code coverage collection via CDP Profiler.
    fn start_js_coverage(&self) -> PyResult<()> {
        let guard = self.page.lock().map_err(py_err)?;
        let page = guard.as_ref().ok_or_else(|| py_err("browser closed"))?;
        self.rt
            .block_on(onecrawl_cdp::coverage::start_js_coverage(page))
            .map_err(py_err)
    }

    /// Stop JavaScript code coverage and return the report as JSON string.
    fn stop_js_coverage(&self) -> PyResult<String> {
        let guard = self.page.lock().map_err(py_err)?;
        let page = guard.as_ref().ok_or_else(|| py_err("browser closed"))?;
        let report = self
            .rt
            .block_on(onecrawl_cdp::coverage::stop_js_coverage(page))
            .map_err(py_err)?;
        serde_json::to_string(&report).map_err(py_err)
    }

    /// Start CSS coverage collection.
    fn start_css_coverage(&self) -> PyResult<()> {
        let guard = self.page.lock().map_err(py_err)?;
        let page = guard.as_ref().ok_or_else(|| py_err("browser closed"))?;
        self.rt
            .block_on(onecrawl_cdp::coverage::start_css_coverage(page))
            .map_err(py_err)
    }

    /// Get CSS coverage summary as JSON string.
    fn get_css_coverage(&self) -> PyResult<String> {
        let guard = self.page.lock().map_err(py_err)?;
        let page = guard.as_ref().ok_or_else(|| py_err("browser closed"))?;
        let report = self
            .rt
            .block_on(onecrawl_cdp::coverage::get_css_coverage(page))
            .map_err(py_err)?;
        Ok(report.to_string())
    }

    // ── Accessibility ──────────────────────────────────────────────

    /// Get the full accessibility tree as JSON.
    fn get_accessibility_tree(&self) -> PyResult<String> {
        let guard = self.page.lock().map_err(py_err)?;
        let page = guard.as_ref().ok_or_else(|| py_err("browser closed"))?;
        let result = self
            .rt
            .block_on(onecrawl_cdp::accessibility::get_accessibility_tree(page))
            .map_err(py_err)?;
        Ok(result.to_string())
    }

    /// Get accessibility info for a specific element.
    fn get_element_accessibility(&self, selector: &str) -> PyResult<String> {
        let guard = self.page.lock().map_err(py_err)?;
        let page = guard.as_ref().ok_or_else(|| py_err("browser closed"))?;
        let result = self
            .rt
            .block_on(onecrawl_cdp::accessibility::get_element_accessibility(
                page, selector,
            ))
            .map_err(py_err)?;
        Ok(result.to_string())
    }

    /// Run an accessibility audit and return the report as JSON.
    fn audit_accessibility(&self) -> PyResult<String> {
        let guard = self.page.lock().map_err(py_err)?;
        let page = guard.as_ref().ok_or_else(|| py_err("browser closed"))?;
        let result = self
            .rt
            .block_on(onecrawl_cdp::accessibility::audit_accessibility(page))
            .map_err(py_err)?;
        serde_json::to_string(&result).map_err(py_err)
    }

    // ── Network Throttling ─────────────────────────────────────────

    /// Set network throttling to a named profile.
    fn set_network_throttle(&self, profile: &str) -> PyResult<()> {
        let guard = self.page.lock().map_err(py_err)?;
        let page = guard.as_ref().ok_or_else(|| py_err("browser closed"))?;
        let p = py_parse_network_profile(profile)?;
        self.rt
            .block_on(onecrawl_cdp::throttle::set_network_conditions(page, p))
            .map_err(py_err)
    }

    /// Set custom network throttling conditions.
    fn set_network_throttle_custom(
        &self,
        download_kbps: f64,
        upload_kbps: f64,
        latency_ms: f64,
    ) -> PyResult<()> {
        let guard = self.page.lock().map_err(py_err)?;
        let page = guard.as_ref().ok_or_else(|| py_err("browser closed"))?;
        let profile = onecrawl_cdp::NetworkProfile::Custom {
            download_kbps,
            upload_kbps,
            latency_ms,
        };
        self.rt
            .block_on(onecrawl_cdp::throttle::set_network_conditions(
                page, profile,
            ))
            .map_err(py_err)
    }

    /// Clear network throttling.
    fn clear_network_throttle(&self) -> PyResult<()> {
        let guard = self.page.lock().map_err(py_err)?;
        let page = guard.as_ref().ok_or_else(|| py_err("browser closed"))?;
        self.rt
            .block_on(onecrawl_cdp::throttle::clear_network_conditions(page))
            .map_err(py_err)
    }

    // ── Performance Tracing ────────────────────────────────────────

    /// Start performance tracing.
    fn start_tracing(&self) -> PyResult<()> {
        let guard = self.page.lock().map_err(py_err)?;
        let page = guard.as_ref().ok_or_else(|| py_err("browser closed"))?;
        self.rt
            .block_on(onecrawl_cdp::tracing_cdp::start_tracing(page))
            .map_err(py_err)
    }

    /// Stop tracing and return trace data as JSON.
    fn stop_tracing(&self) -> PyResult<String> {
        let guard = self.page.lock().map_err(py_err)?;
        let page = guard.as_ref().ok_or_else(|| py_err("browser closed"))?;
        let result = self
            .rt
            .block_on(onecrawl_cdp::tracing_cdp::stop_tracing(page))
            .map_err(py_err)?;
        Ok(result.to_string())
    }

    /// Get performance metrics as JSON.
    fn get_performance_metrics(&self) -> PyResult<String> {
        let guard = self.page.lock().map_err(py_err)?;
        let page = guard.as_ref().ok_or_else(|| py_err("browser closed"))?;
        let result = self
            .rt
            .block_on(onecrawl_cdp::tracing_cdp::get_performance_metrics(page))
            .map_err(py_err)?;
        serde_json::to_string(&result).map_err(py_err)
    }

    /// Get navigation timing data as JSON.
    fn get_navigation_timing(&self) -> PyResult<String> {
        let guard = self.page.lock().map_err(py_err)?;
        let page = guard.as_ref().ok_or_else(|| py_err("browser closed"))?;
        let result = self
            .rt
            .block_on(onecrawl_cdp::tracing_cdp::get_navigation_timing(page))
            .map_err(py_err)?;
        Ok(result.to_string())
    }

    /// Get resource timing entries as JSON.
    fn get_resource_timing(&self) -> PyResult<String> {
        let guard = self.page.lock().map_err(py_err)?;
        let page = guard.as_ref().ok_or_else(|| py_err("browser closed"))?;
        let result = self
            .rt
            .block_on(onecrawl_cdp::tracing_cdp::get_resource_timing(page))
            .map_err(py_err)?;
        serde_json::to_string(&result).map_err(py_err)
    }

    // ── Proxy Pool ─────────────────────────────────────────────────

    /// Create a proxy pool from JSON config. Returns pool JSON.
    #[staticmethod]
    fn create_proxy_pool(config: &str) -> PyResult<String> {
        let pool: onecrawl_cdp::ProxyPool = serde_json::from_str(config).map_err(py_err)?;
        pool.to_json().map_err(py_err)
    }

    /// Get Chrome launch args for the first proxy in the pool.
    #[staticmethod]
    fn get_proxy_chrome_args(pool: &str) -> PyResult<Vec<String>> {
        let p: onecrawl_cdp::ProxyPool = serde_json::from_str(pool).map_err(py_err)?;
        Ok(p.chrome_args())
    }

    /// Rotate to the next proxy. Returns updated pool JSON.
    #[staticmethod]
    fn next_proxy(pool: &str) -> PyResult<String> {
        let mut p: onecrawl_cdp::ProxyPool = serde_json::from_str(pool).map_err(py_err)?;
        p.next_proxy();
        p.to_json().map_err(py_err)
    }

    // ── Request Interception ───────────────────────────────────────

    /// Set request interception rules (JSON array of InterceptRule).
    fn set_intercept_rules(&self, rules: &str) -> PyResult<()> {
        let guard = self.page.lock().map_err(py_err)?;
        let page = guard.as_ref().ok_or_else(|| py_err("browser closed"))?;
        let parsed: Vec<onecrawl_cdp::InterceptRule> =
            serde_json::from_str(rules).map_err(py_err)?;
        self.rt
            .block_on(onecrawl_cdp::intercept::set_intercept_rules(page, parsed))
            .map_err(py_err)
    }

    /// Get intercepted request log as JSON.
    fn get_intercepted_requests(&self) -> PyResult<String> {
        let guard = self.page.lock().map_err(py_err)?;
        let page = guard.as_ref().ok_or_else(|| py_err("browser closed"))?;
        let log = self
            .rt
            .block_on(onecrawl_cdp::intercept::get_intercepted_requests(page))
            .map_err(py_err)?;
        serde_json::to_string(&log).map_err(py_err)
    }

    /// Clear all interception rules.
    fn clear_intercept_rules(&self) -> PyResult<()> {
        let guard = self.page.lock().map_err(py_err)?;
        let page = guard.as_ref().ok_or_else(|| py_err("browser closed"))?;
        self.rt
            .block_on(onecrawl_cdp::intercept::clear_intercept_rules(page))
            .map_err(py_err)
    }

    // ── Advanced Emulation ─────────────────────────────────────────

    /// Override device orientation sensor.
    fn set_device_orientation(&self, alpha: f64, beta: f64, gamma: f64) -> PyResult<()> {
        let guard = self.page.lock().map_err(py_err)?;
        let page = guard.as_ref().ok_or_else(|| py_err("browser closed"))?;
        let reading = onecrawl_cdp::advanced_emulation::SensorReading { alpha, beta, gamma };
        self.rt
            .block_on(onecrawl_cdp::advanced_emulation::set_device_orientation(
                page, reading,
            ))
            .map_err(py_err)
    }

    /// Override a permission query result.
    fn override_permission(&self, permission: &str, state: &str) -> PyResult<()> {
        let guard = self.page.lock().map_err(py_err)?;
        let page = guard.as_ref().ok_or_else(|| py_err("browser closed"))?;
        self.rt
            .block_on(onecrawl_cdp::advanced_emulation::override_permission(
                page, permission, state,
            ))
            .map_err(py_err)
    }

    /// Override battery status API.
    fn set_battery_status(&self, level: f64, charging: bool) -> PyResult<()> {
        let guard = self.page.lock().map_err(py_err)?;
        let page = guard.as_ref().ok_or_else(|| py_err("browser closed"))?;
        self.rt
            .block_on(onecrawl_cdp::advanced_emulation::set_battery_status(
                page, level, charging,
            ))
            .map_err(py_err)
    }

    /// Override Network Information API.
    fn set_connection_info(&self, effective_type: &str, downlink: f64, rtt: u32) -> PyResult<()> {
        let guard = self.page.lock().map_err(py_err)?;
        let page = guard.as_ref().ok_or_else(|| py_err("browser closed"))?;
        self.rt
            .block_on(onecrawl_cdp::advanced_emulation::set_connection_info(
                page,
                effective_type,
                downlink,
                rtt,
            ))
            .map_err(py_err)
    }

    /// Override hardware concurrency (CPU cores).
    fn set_hardware_concurrency(&self, cores: u32) -> PyResult<()> {
        let guard = self.page.lock().map_err(py_err)?;
        let page = guard.as_ref().ok_or_else(|| py_err("browser closed"))?;
        self.rt
            .block_on(onecrawl_cdp::advanced_emulation::set_hardware_concurrency(
                page, cores,
            ))
            .map_err(py_err)
    }

    /// Override device memory (GB).
    fn set_device_memory(&self, gb: f64) -> PyResult<()> {
        let guard = self.page.lock().map_err(py_err)?;
        let page = guard.as_ref().ok_or_else(|| py_err("browser closed"))?;
        self.rt
            .block_on(onecrawl_cdp::advanced_emulation::set_device_memory(
                page, gb,
            ))
            .map_err(py_err)
    }

    /// Get current navigator properties as JSON.
    fn get_navigator_info(&self) -> PyResult<String> {
        let guard = self.page.lock().map_err(py_err)?;
        let page = guard.as_ref().ok_or_else(|| py_err("browser closed"))?;
        let info = self
            .rt
            .block_on(onecrawl_cdp::advanced_emulation::get_navigator_info(page))
            .map_err(py_err)?;
        serde_json::to_string(&info).map_err(py_err)
    }

    /// Run the CDP benchmark suite. Returns JSON string of BenchmarkSuite.
    #[pyo3(signature = (iterations=100))]
    fn run_benchmark(&self, iterations: u32) -> PyResult<String> {
        let guard = self.page.lock().map_err(py_err)?;
        let page = guard.as_ref().ok_or_else(|| py_err("browser closed"))?;
        let suite = self
            .rt
            .block_on(onecrawl_cdp::benchmark::run_cdp_benchmarks(
                page, iterations,
            ));
        serde_json::to_string(&suite).map_err(py_err)
    }

    // ──────────────── Geofencing ────────────────

    /// Apply a geo profile. Accepts JSON string of GeoProfile.
    fn apply_geo_profile(&self, profile: &str) -> PyResult<()> {
        let guard = self.page.lock().map_err(py_err)?;
        let page = guard.as_ref().ok_or_else(|| py_err("browser closed"))?;
        let p: onecrawl_cdp::GeoProfile = serde_json::from_str(profile)
            .map_err(|e| py_err(format!("invalid geo profile: {e}")))?;
        self.rt
            .block_on(onecrawl_cdp::geofencing::apply_geo_profile(page, &p))
            .map_err(py_err)
    }

    /// List available geo preset names.
    fn list_geo_presets(&self) -> Vec<String> {
        onecrawl_cdp::geofencing::list_presets()
    }

    /// Get a geo preset by name. Returns JSON string or None.
    #[pyo3(signature = (name,))]
    fn get_geo_preset(&self, name: &str) -> Option<String> {
        onecrawl_cdp::geofencing::get_preset(name)
            .map(|p| serde_json::to_string(&p).unwrap_or_default())
    }

    /// Get current geolocation as seen by the page. Returns JSON string.
    fn get_current_geo(&self) -> PyResult<String> {
        let guard = self.page.lock().map_err(py_err)?;
        let page = guard.as_ref().ok_or_else(|| py_err("browser closed"))?;
        let val = self
            .rt
            .block_on(onecrawl_cdp::geofencing::get_current_geo(page))
            .map_err(py_err)?;
        serde_json::to_string(&val).map_err(py_err)
    }

    // ──────────────── Cookie Jar ────────────────

    /// Export all cookies as a JSON CookieJar string.
    fn export_cookies(&self) -> PyResult<String> {
        let guard = self.page.lock().map_err(py_err)?;
        let page = guard.as_ref().ok_or_else(|| py_err("browser closed"))?;
        let jar = self
            .rt
            .block_on(onecrawl_cdp::cookie_jar::export_cookies(page))
            .map_err(py_err)?;
        serde_json::to_string(&jar).map_err(py_err)
    }

    /// Import cookies from a JSON CookieJar string. Returns count imported.
    fn import_cookies(&self, jar: &str) -> PyResult<usize> {
        let guard = self.page.lock().map_err(py_err)?;
        let page = guard.as_ref().ok_or_else(|| py_err("browser closed"))?;
        let cookie_jar: onecrawl_cdp::CookieJar =
            serde_json::from_str(jar).map_err(|e| py_err(format!("invalid cookie jar: {e}")))?;
        self.rt
            .block_on(onecrawl_cdp::cookie_jar::import_cookies(page, &cookie_jar))
            .map_err(py_err)
    }

    /// Save cookies to a file. Returns count saved.
    fn save_cookies_to_file(&self, path: &str) -> PyResult<usize> {
        let guard = self.page.lock().map_err(py_err)?;
        let page = guard.as_ref().ok_or_else(|| py_err("browser closed"))?;
        self.rt
            .block_on(onecrawl_cdp::cookie_jar::save_cookies_to_file(
                page,
                std::path::Path::new(path),
            ))
            .map_err(py_err)
    }

    /// Load cookies from a file. Returns count loaded.
    fn load_cookies_from_file(&self, path: &str) -> PyResult<usize> {
        let guard = self.page.lock().map_err(py_err)?;
        let page = guard.as_ref().ok_or_else(|| py_err("browser closed"))?;
        self.rt
            .block_on(onecrawl_cdp::cookie_jar::load_cookies_from_file(
                page,
                std::path::Path::new(path),
            ))
            .map_err(py_err)
    }

    /// Clear all cookies via cookie_jar module.
    fn clear_all_cookies(&self) -> PyResult<()> {
        let guard = self.page.lock().map_err(py_err)?;
        let page = guard.as_ref().ok_or_else(|| py_err("browser closed"))?;
        self.rt
            .block_on(onecrawl_cdp::cookie_jar::clear_all_cookies(page))
            .map_err(py_err)
    }

    // ──────────────── Request Queue ────────────────

    /// Execute a single request with retry. Accepts JSON QueuedRequest. Returns JSON RequestResult.
    fn execute_request(&self, request: &str) -> PyResult<String> {
        let guard = self.page.lock().map_err(py_err)?;
        let page = guard.as_ref().ok_or_else(|| py_err("browser closed"))?;
        let req: onecrawl_cdp::QueuedRequest =
            serde_json::from_str(request).map_err(|e| py_err(format!("invalid request: {e}")))?;
        let result = self
            .rt
            .block_on(onecrawl_cdp::request_queue::execute_request(page, &req))
            .map_err(py_err)?;
        serde_json::to_string(&result).map_err(py_err)
    }

    /// Execute a batch of requests. Accepts JSON array + optional JSON config. Returns JSON array.
    #[pyo3(signature = (requests, config=None))]
    fn execute_batch(&self, requests: &str, config: Option<&str>) -> PyResult<String> {
        let guard = self.page.lock().map_err(py_err)?;
        let page = guard.as_ref().ok_or_else(|| py_err("browser closed"))?;
        let reqs: Vec<onecrawl_cdp::QueuedRequest> =
            serde_json::from_str(requests).map_err(|e| py_err(format!("invalid requests: {e}")))?;
        let cfg: onecrawl_cdp::QueueConfig = match config {
            Some(c) => {
                serde_json::from_str(c).map_err(|e| py_err(format!("invalid config: {e}")))?
            }
            None => onecrawl_cdp::QueueConfig::default(),
        };
        let results = self
            .rt
            .block_on(onecrawl_cdp::request_queue::execute_batch(
                page, &reqs, &cfg,
            ))
            .map_err(py_err)?;
        serde_json::to_string(&results).map_err(py_err)
    }

    /// Create a GET request. Returns JSON QueuedRequest.
    fn create_get_request(&self, id: &str, url: &str) -> String {
        let req = onecrawl_cdp::request_queue::get_request(id, url);
        serde_json::to_string(&req).unwrap_or_default()
    }

    /// Create a POST request. Returns JSON QueuedRequest.
    fn create_post_request(&self, id: &str, url: &str, body: &str) -> String {
        let req = onecrawl_cdp::request_queue::post_request(id, url, body);
        serde_json::to_string(&req).unwrap_or_default()
    }

    // ──────────────── Smart Selectors ────────────────

    /// CSS selector with pseudo-elements (::text, ::attr(name)). Returns JSON SelectorResult.
    fn css_select(&self, selector: &str) -> PyResult<String> {
        let guard = self.page.lock().map_err(py_err)?;
        let page = guard.as_ref().ok_or_else(|| py_err("browser closed"))?;
        let result = self
            .rt
            .block_on(onecrawl_cdp::selectors::css_select(page, selector))
            .map_err(py_err)?;
        serde_json::to_string(&result).map_err(py_err)
    }

    /// XPath selector. Returns JSON SelectorResult.
    fn xpath_select(&self, expression: &str) -> PyResult<String> {
        let guard = self.page.lock().map_err(py_err)?;
        let page = guard.as_ref().ok_or_else(|| py_err("browser closed"))?;
        let result = self
            .rt
            .block_on(onecrawl_cdp::selectors::xpath_select(page, expression))
            .map_err(py_err)?;
        serde_json::to_string(&result).map_err(py_err)
    }

    /// Find elements by text content. Returns JSON SelectorResult.
    #[pyo3(signature = (text, tag=None))]
    fn find_by_text(&self, text: &str, tag: Option<&str>) -> PyResult<String> {
        let guard = self.page.lock().map_err(py_err)?;
        let page = guard.as_ref().ok_or_else(|| py_err("browser closed"))?;
        let result = self
            .rt
            .block_on(onecrawl_cdp::selectors::find_by_text(page, text, tag))
            .map_err(py_err)?;
        serde_json::to_string(&result).map_err(py_err)
    }

    /// Find elements by regex pattern. Returns JSON SelectorResult.
    #[pyo3(signature = (pattern, tag=None))]
    fn find_by_regex(&self, pattern: &str, tag: Option<&str>) -> PyResult<String> {
        let guard = self.page.lock().map_err(py_err)?;
        let page = guard.as_ref().ok_or_else(|| py_err("browser closed"))?;
        let result = self
            .rt
            .block_on(onecrawl_cdp::selectors::find_by_regex(page, pattern, tag))
            .map_err(py_err)?;
        serde_json::to_string(&result).map_err(py_err)
    }

    /// Auto-generate a unique CSS selector for an element.
    fn auto_selector(&self, target_selector: &str) -> PyResult<String> {
        let guard = self.page.lock().map_err(py_err)?;
        let page = guard.as_ref().ok_or_else(|| py_err("browser closed"))?;
        self.rt
            .block_on(onecrawl_cdp::selectors::auto_selector(
                page,
                target_selector,
            ))
            .map_err(py_err)
    }

    // ──────────────── DOM Navigation ────────────────

    /// Get parent element. Returns JSON NavElement or None.
    fn get_parent(&self, selector: &str) -> PyResult<Option<String>> {
        let guard = self.page.lock().map_err(py_err)?;
        let page = guard.as_ref().ok_or_else(|| py_err("browser closed"))?;
        let result = self
            .rt
            .block_on(onecrawl_cdp::dom_nav::get_parent(page, selector))
            .map_err(py_err)?;
        match result {
            Some(el) => Ok(Some(serde_json::to_string(&el).map_err(py_err)?)),
            None => Ok(None),
        }
    }

    /// Get child elements. Returns JSON array of NavElement.
    fn get_children(&self, selector: &str) -> PyResult<String> {
        let guard = self.page.lock().map_err(py_err)?;
        let page = guard.as_ref().ok_or_else(|| py_err("browser closed"))?;
        let result = self
            .rt
            .block_on(onecrawl_cdp::dom_nav::get_children(page, selector))
            .map_err(py_err)?;
        serde_json::to_string(&result).map_err(py_err)
    }

    /// Get next sibling element. Returns JSON NavElement or None.
    fn get_next_sibling(&self, selector: &str) -> PyResult<Option<String>> {
        let guard = self.page.lock().map_err(py_err)?;
        let page = guard.as_ref().ok_or_else(|| py_err("browser closed"))?;
        let result = self
            .rt
            .block_on(onecrawl_cdp::dom_nav::get_next_sibling(page, selector))
            .map_err(py_err)?;
        match result {
            Some(el) => Ok(Some(serde_json::to_string(&el).map_err(py_err)?)),
            None => Ok(None),
        }
    }

    /// Get previous sibling element. Returns JSON NavElement or None.
    fn get_prev_sibling(&self, selector: &str) -> PyResult<Option<String>> {
        let guard = self.page.lock().map_err(py_err)?;
        let page = guard.as_ref().ok_or_else(|| py_err("browser closed"))?;
        let result = self
            .rt
            .block_on(onecrawl_cdp::dom_nav::get_prev_sibling(page, selector))
            .map_err(py_err)?;
        match result {
            Some(el) => Ok(Some(serde_json::to_string(&el).map_err(py_err)?)),
            None => Ok(None),
        }
    }

    /// Get all sibling elements. Returns JSON array of NavElement.
    fn get_siblings(&self, selector: &str) -> PyResult<String> {
        let guard = self.page.lock().map_err(py_err)?;
        let page = guard.as_ref().ok_or_else(|| py_err("browser closed"))?;
        let result = self
            .rt
            .block_on(onecrawl_cdp::dom_nav::get_siblings(page, selector))
            .map_err(py_err)?;
        serde_json::to_string(&result).map_err(py_err)
    }

    /// Find similar elements. Returns JSON array of NavElement.
    fn find_similar(&self, selector: &str) -> PyResult<String> {
        let guard = self.page.lock().map_err(py_err)?;
        let page = guard.as_ref().ok_or_else(|| py_err("browser closed"))?;
        let result = self
            .rt
            .block_on(onecrawl_cdp::dom_nav::find_similar(page, selector))
            .map_err(py_err)?;
        serde_json::to_string(&result).map_err(py_err)
    }

    /// Get elements above the target. Returns JSON array of NavElement.
    #[pyo3(signature = (selector, limit=10))]
    fn above_elements(&self, selector: &str, limit: usize) -> PyResult<String> {
        let guard = self.page.lock().map_err(py_err)?;
        let page = guard.as_ref().ok_or_else(|| py_err("browser closed"))?;
        let result = self
            .rt
            .block_on(onecrawl_cdp::dom_nav::above_elements(page, selector, limit))
            .map_err(py_err)?;
        serde_json::to_string(&result).map_err(py_err)
    }

    /// Get elements below the target. Returns JSON array of NavElement.
    #[pyo3(signature = (selector, limit=10))]
    fn below_elements(&self, selector: &str, limit: usize) -> PyResult<String> {
        let guard = self.page.lock().map_err(py_err)?;
        let page = guard.as_ref().ok_or_else(|| py_err("browser closed"))?;
        let result = self
            .rt
            .block_on(onecrawl_cdp::dom_nav::below_elements(page, selector, limit))
            .map_err(py_err)?;
        serde_json::to_string(&result).map_err(py_err)
    }

    // ──────────────── Content Extraction ────────────────

    /// Extract page content. Returns JSON ExtractResult.
    #[pyo3(signature = (selector=None, format=None))]
    fn extract_content(&self, selector: Option<&str>, format: Option<&str>) -> PyResult<String> {
        let guard = self.page.lock().map_err(py_err)?;
        let page = guard.as_ref().ok_or_else(|| py_err("browser closed"))?;
        let fmt = onecrawl_cdp::extract::parse_extract_format(format.unwrap_or("text"))
            .map_err(py_err)?;
        let result = self
            .rt
            .block_on(onecrawl_cdp::extract::extract(page, selector, fmt))
            .map_err(py_err)?;
        serde_json::to_string(&result).map_err(py_err)
    }

    /// Extract content and save to file. Returns bytes written.
    #[pyo3(signature = (output_path, selector=None))]
    fn extract_to_file(&self, output_path: &str, selector: Option<&str>) -> PyResult<usize> {
        let guard = self.page.lock().map_err(py_err)?;
        let page = guard.as_ref().ok_or_else(|| py_err("browser closed"))?;
        self.rt
            .block_on(onecrawl_cdp::extract::extract_to_file(
                page,
                selector,
                std::path::Path::new(output_path),
            ))
            .map_err(py_err)
    }

    /// Get structured page metadata. Returns JSON object.
    fn get_page_metadata(&self) -> PyResult<String> {
        let guard = self.page.lock().map_err(py_err)?;
        let page = guard.as_ref().ok_or_else(|| py_err("browser closed"))?;
        let meta = self
            .rt
            .block_on(onecrawl_cdp::extract::get_page_metadata(page))
            .map_err(py_err)?;
        serde_json::to_string(&meta).map_err(py_err)
    }

    // ── Network Request Logger ────────────────────────────────────

    /// Start network request/response logging.
    fn start_network_log(&self) -> PyResult<()> {
        let guard = self.page.lock().map_err(py_err)?;
        let page = guard.as_ref().ok_or_else(|| py_err("browser closed"))?;
        self.rt
            .block_on(onecrawl_cdp::network_log::start_network_log(page))
            .map_err(py_err)
    }

    /// Drain captured network entries as JSON string.
    fn drain_network_log(&self) -> PyResult<String> {
        let guard = self.page.lock().map_err(py_err)?;
        let page = guard.as_ref().ok_or_else(|| py_err("browser closed"))?;
        let entries = self
            .rt
            .block_on(onecrawl_cdp::network_log::drain_network_log(page))
            .map_err(py_err)?;
        serde_json::to_string(&entries).map_err(py_err)
    }

    /// Get network summary statistics as JSON string.
    fn get_network_summary(&self) -> PyResult<String> {
        let guard = self.page.lock().map_err(py_err)?;
        let page = guard.as_ref().ok_or_else(|| py_err("browser closed"))?;
        let summary = self
            .rt
            .block_on(onecrawl_cdp::network_log::get_network_summary(page))
            .map_err(py_err)?;
        serde_json::to_string(&summary).map_err(py_err)
    }

    /// Stop network logging and restore originals.
    fn stop_network_log(&self) -> PyResult<()> {
        let guard = self.page.lock().map_err(py_err)?;
        let page = guard.as_ref().ok_or_else(|| py_err("browser closed"))?;
        self.rt
            .block_on(onecrawl_cdp::network_log::stop_network_log(page))
            .map_err(py_err)
    }

    /// Export network log to a JSON file.
    fn export_network_log(&self, path: &str) -> PyResult<()> {
        let guard = self.page.lock().map_err(py_err)?;
        let page = guard.as_ref().ok_or_else(|| py_err("browser closed"))?;
        self.rt
            .block_on(onecrawl_cdp::network_log::export_network_log(page, path))
            .map_err(py_err)
    }

    // ── Page Watcher ──────────────────────────────────────────────

    /// Start watching for page state changes.
    fn start_page_watcher(&self) -> PyResult<()> {
        let guard = self.page.lock().map_err(py_err)?;
        let page = guard.as_ref().ok_or_else(|| py_err("browser closed"))?;
        self.rt
            .block_on(onecrawl_cdp::page_watcher::start_page_watcher(page))
            .map_err(py_err)
    }

    /// Drain accumulated page changes as JSON string.
    fn drain_page_changes(&self) -> PyResult<String> {
        let guard = self.page.lock().map_err(py_err)?;
        let page = guard.as_ref().ok_or_else(|| py_err("browser closed"))?;
        let changes = self
            .rt
            .block_on(onecrawl_cdp::page_watcher::drain_page_changes(page))
            .map_err(py_err)?;
        serde_json::to_string(&changes).map_err(py_err)
    }

    /// Stop the page watcher.
    fn stop_page_watcher(&self) -> PyResult<()> {
        let guard = self.page.lock().map_err(py_err)?;
        let page = guard.as_ref().ok_or_else(|| py_err("browser closed"))?;
        self.rt
            .block_on(onecrawl_cdp::page_watcher::stop_page_watcher(page))
            .map_err(py_err)
    }

    /// Get current page state snapshot as JSON string.
    fn get_page_state(&self) -> PyResult<String> {
        let guard = self.page.lock().map_err(py_err)?;
        let page = guard.as_ref().ok_or_else(|| py_err("browser closed"))?;
        let state = self
            .rt
            .block_on(onecrawl_cdp::page_watcher::get_page_state(page))
            .map_err(py_err)?;
        serde_json::to_string(&state).map_err(py_err)
    }

    // ── Spider / Crawl ─────────────────────────────────────────────

    /// Run a crawl. Accepts SpiderConfig as JSON, returns Vec<CrawlResult> as JSON.
    fn crawl(&self, config_json: &str) -> PyResult<String> {
        let config: onecrawl_cdp::SpiderConfig =
            serde_json::from_str(config_json).map_err(py_err)?;
        let guard = self.page.lock().map_err(py_err)?;
        let page = guard.as_ref().ok_or_else(|| py_err("browser closed"))?;
        let results = self
            .rt
            .block_on(onecrawl_cdp::spider::crawl(page, config))
            .map_err(py_err)?;
        serde_json::to_string(&results).map_err(py_err)
    }

    /// Compute crawl summary from results JSON.
    fn crawl_summary(&self, results_json: &str) -> PyResult<String> {
        let results: Vec<onecrawl_cdp::CrawlResult> =
            serde_json::from_str(results_json).map_err(py_err)?;
        let summary = onecrawl_cdp::spider::summarize(&results);
        serde_json::to_string(&summary).map_err(py_err)
    }

    /// Save crawl state to a JSON file.
    fn save_crawl_state(&self, state_json: &str, path: &str) -> PyResult<()> {
        let state: onecrawl_cdp::CrawlState = serde_json::from_str(state_json).map_err(py_err)?;
        onecrawl_cdp::spider::save_state(&state, std::path::Path::new(path)).map_err(py_err)
    }

    /// Load crawl state from a JSON file.
    fn load_crawl_state(&self, path: &str) -> PyResult<String> {
        let state = onecrawl_cdp::spider::load_state(std::path::Path::new(path)).map_err(py_err)?;
        serde_json::to_string(&state).map_err(py_err)
    }

    /// Export crawl results to file. Format: "json" (default) or "jsonl".
    fn export_crawl_results(
        &self,
        results_json: &str,
        path: &str,
        format: Option<&str>,
    ) -> PyResult<usize> {
        let results: Vec<onecrawl_cdp::CrawlResult> =
            serde_json::from_str(results_json).map_err(py_err)?;
        let p = std::path::Path::new(path);
        match format {
            Some("jsonl") => {
                onecrawl_cdp::spider::export_results_jsonl(&results, p).map_err(py_err)
            }
            _ => onecrawl_cdp::spider::export_results(&results, p).map_err(py_err),
        }
    }

    // ── Robots.txt ─────────────────────────────────────────────────

    /// Parse robots.txt content. Returns JSON RobotsTxt.
    fn robots_parse(&self, content: &str) -> PyResult<String> {
        let robots = onecrawl_cdp::robots::parse_robots(content);
        serde_json::to_string(&robots).map_err(py_err)
    }

    /// Check if a path is allowed for a user-agent. Accepts JSON RobotsTxt.
    fn robots_is_allowed(&self, robots_json: &str, user_agent: &str, path: &str) -> PyResult<bool> {
        let robots: onecrawl_cdp::RobotsTxt = serde_json::from_str(robots_json).map_err(py_err)?;
        Ok(onecrawl_cdp::robots::is_allowed(&robots, user_agent, path))
    }

    /// Get crawl delay for a user-agent. Accepts JSON RobotsTxt.
    fn robots_crawl_delay(&self, robots_json: &str, user_agent: &str) -> PyResult<Option<f64>> {
        let robots: onecrawl_cdp::RobotsTxt = serde_json::from_str(robots_json).map_err(py_err)?;
        Ok(onecrawl_cdp::robots::get_crawl_delay(&robots, user_agent))
    }

    /// Get sitemaps from parsed robots.txt. Accepts JSON RobotsTxt, returns JSON array.
    fn robots_sitemaps(&self, robots_json: &str) -> PyResult<String> {
        let robots: onecrawl_cdp::RobotsTxt = serde_json::from_str(robots_json).map_err(py_err)?;
        let sitemaps = onecrawl_cdp::robots::get_sitemaps(&robots);
        serde_json::to_string(&sitemaps).map_err(py_err)
    }

    /// Fetch and parse robots.txt from a URL. Returns JSON RobotsTxt.
    fn robots_fetch(&self, base_url: &str) -> PyResult<String> {
        let guard = self.page.lock().map_err(py_err)?;
        let page = guard.as_ref().ok_or_else(|| py_err("browser closed"))?;
        let robots = self
            .rt
            .block_on(onecrawl_cdp::robots::fetch_robots(page, base_url))
            .map_err(py_err)?;
        serde_json::to_string(&robots).map_err(py_err)
    }

    // ── Link Graph ─────────────────────────────────────────────────

    /// Extract links from the current page. Returns JSON Vec<LinkEdge>.
    fn graph_extract_links(&self, base_url: &str) -> PyResult<String> {
        let guard = self.page.lock().map_err(py_err)?;
        let page = guard.as_ref().ok_or_else(|| py_err("browser closed"))?;
        let edges = self
            .rt
            .block_on(onecrawl_cdp::link_graph::extract_links(page, base_url))
            .map_err(py_err)?;
        serde_json::to_string(&edges).map_err(py_err)
    }

    /// Build a link graph from edges JSON. Returns JSON LinkGraph.
    fn graph_build(&self, edges_json: &str) -> PyResult<String> {
        let edges: Vec<onecrawl_cdp::LinkEdge> =
            serde_json::from_str(edges_json).map_err(py_err)?;
        let graph = onecrawl_cdp::link_graph::build_graph(&edges);
        serde_json::to_string(&graph).map_err(py_err)
    }

    /// Analyze a link graph. Accepts JSON LinkGraph, returns JSON LinkStats.
    fn graph_analyze(&self, graph_json: &str) -> PyResult<String> {
        let graph: onecrawl_cdp::LinkGraph = serde_json::from_str(graph_json).map_err(py_err)?;
        let stats = onecrawl_cdp::link_graph::analyze_graph(&graph);
        serde_json::to_string(&stats).map_err(py_err)
    }

    /// Find orphan pages (no inbound links). Accepts JSON LinkGraph, returns JSON array.
    fn graph_find_orphans(&self, graph_json: &str) -> PyResult<String> {
        let graph: onecrawl_cdp::LinkGraph = serde_json::from_str(graph_json).map_err(py_err)?;
        let orphans = onecrawl_cdp::link_graph::find_orphans(&graph);
        serde_json::to_string(&orphans).map_err(py_err)
    }

    /// Find hub pages. Accepts JSON LinkGraph and min_outbound threshold.
    fn graph_find_hubs(&self, graph_json: &str, min_outbound: usize) -> PyResult<String> {
        let graph: onecrawl_cdp::LinkGraph = serde_json::from_str(graph_json).map_err(py_err)?;
        let hubs = onecrawl_cdp::link_graph::find_hubs(&graph, min_outbound);
        serde_json::to_string(&hubs).map_err(py_err)
    }

    /// Export link graph to a JSON file.
    fn graph_export(&self, graph_json: &str, path: &str) -> PyResult<()> {
        let graph: onecrawl_cdp::LinkGraph = serde_json::from_str(graph_json).map_err(py_err)?;
        onecrawl_cdp::link_graph::export_graph_json(&graph, std::path::Path::new(path))
            .map_err(py_err)
    }

    /// Build link graph from crawl results JSON. Returns JSON LinkGraph.
    fn graph_from_crawl_results(&self, results_json: &str) -> PyResult<String> {
        let results: Vec<onecrawl_cdp::CrawlResult> =
            serde_json::from_str(results_json).map_err(py_err)?;
        let graph = onecrawl_cdp::link_graph::from_crawl_results(&results);
        serde_json::to_string(&graph).map_err(py_err)
    }

    // ── Anti-Bot ────────────────────────────────────────────────────

    /// Inject full stealth anti-bot patches. Returns JSON array of applied patch names.
    fn inject_stealth_full(&self) -> PyResult<String> {
        let guard = self.page.lock().map_err(py_err)?;
        let page = guard.as_ref().ok_or_else(|| py_err("browser closed"))?;
        let applied = self
            .rt
            .block_on(onecrawl_cdp::antibot::inject_stealth_full(page))
            .map_err(py_err)?;
        serde_json::to_string(&applied).map_err(py_err)
    }

    /// Run bot detection tests. Returns JSON object with scores.
    fn bot_detection_test(&self) -> PyResult<String> {
        let guard = self.page.lock().map_err(py_err)?;
        let page = guard.as_ref().ok_or_else(|| py_err("browser closed"))?;
        let result = self
            .rt
            .block_on(onecrawl_cdp::antibot::bot_detection_test(page))
            .map_err(py_err)?;
        serde_json::to_string(&result).map_err(py_err)
    }

    /// Get available stealth profiles. Returns JSON array.
    fn stealth_profiles(&self) -> PyResult<String> {
        let profiles = onecrawl_cdp::antibot::stealth_profiles();
        serde_json::to_string(&profiles).map_err(py_err)
    }

    // ── Adaptive Element Tracker ────────────────────────────────────

    /// Fingerprint a DOM element by CSS selector. Returns JSON.
    fn fingerprint_element(&self, selector: &str) -> PyResult<String> {
        let guard = self.page.lock().map_err(py_err)?;
        let page = guard.as_ref().ok_or_else(|| py_err("browser closed"))?;
        let fp = self
            .rt
            .block_on(onecrawl_cdp::adaptive::fingerprint_element(page, selector))
            .map_err(py_err)?;
        serde_json::to_string(&fp).map_err(py_err)
    }

    /// Relocate an element using a previously captured fingerprint (JSON). Returns JSON matches.
    fn relocate_element(&self, fingerprint: &str) -> PyResult<String> {
        let fp: onecrawl_cdp::ElementFingerprint =
            serde_json::from_str(fingerprint).map_err(py_err)?;
        let guard = self.page.lock().map_err(py_err)?;
        let page = guard.as_ref().ok_or_else(|| py_err("browser closed"))?;
        let matches = self
            .rt
            .block_on(onecrawl_cdp::adaptive::relocate_element(page, &fp))
            .map_err(py_err)?;
        serde_json::to_string(&matches).map_err(py_err)
    }

    /// Track multiple elements by CSS selectors (JSON array). Optionally save to path.
    fn track_elements(&self, selectors: &str, save_path: Option<&str>) -> PyResult<String> {
        let sels: Vec<String> = serde_json::from_str(selectors).map_err(py_err)?;
        let sel_refs: Vec<&str> = sels.iter().map(|s| s.as_str()).collect();
        let guard = self.page.lock().map_err(py_err)?;
        let page = guard.as_ref().ok_or_else(|| py_err("browser closed"))?;
        let path_buf = save_path.map(std::path::PathBuf::from);
        let fps = self
            .rt
            .block_on(onecrawl_cdp::adaptive::track_elements(
                page,
                &sel_refs,
                path_buf.as_deref(),
            ))
            .map_err(py_err)?;
        serde_json::to_string(&fps).map_err(py_err)
    }

    /// Relocate all fingerprints (JSON array). Returns JSON array of (selector, matches).
    fn relocate_all(&self, fingerprints: &str) -> PyResult<String> {
        let fps: Vec<onecrawl_cdp::ElementFingerprint> =
            serde_json::from_str(fingerprints).map_err(py_err)?;
        let guard = self.page.lock().map_err(py_err)?;
        let page = guard.as_ref().ok_or_else(|| py_err("browser closed"))?;
        let results = self
            .rt
            .block_on(onecrawl_cdp::adaptive::relocate_all(page, &fps))
            .map_err(py_err)?;
        serde_json::to_string(&results).map_err(py_err)
    }

    /// Save fingerprints JSON to a file path.
    fn save_fingerprints(&self, fingerprints: &str, path: &str) -> PyResult<()> {
        let fps: Vec<onecrawl_cdp::ElementFingerprint> =
            serde_json::from_str(fingerprints).map_err(py_err)?;
        onecrawl_cdp::adaptive::save_fingerprints(&fps, std::path::Path::new(path)).map_err(py_err)
    }

    /// Load fingerprints from a file path. Returns JSON array.
    fn load_fingerprints(&self, path: &str) -> PyResult<String> {
        let fps = onecrawl_cdp::adaptive::load_fingerprints(std::path::Path::new(path))
            .map_err(py_err)?;
        serde_json::to_string(&fps).map_err(py_err)
    }

    // ── Domain Blocker ────────────────────────────────────────────

    /// Block a list of domains (JSON array). Returns total blocked count.
    fn block_domains(&self, domains: &str) -> PyResult<usize> {
        let list: Vec<String> = serde_json::from_str(domains).map_err(py_err)?;
        let guard = self.page.lock().map_err(py_err)?;
        let page = guard.as_ref().ok_or_else(|| py_err("browser closed"))?;
        self.rt
            .block_on(onecrawl_cdp::domain_blocker::block_domains(page, &list))
            .map_err(py_err)
    }

    /// Block domains by category (ads, trackers, social, fonts, media). Returns total count.
    fn block_category(&self, category: &str) -> PyResult<usize> {
        let guard = self.page.lock().map_err(py_err)?;
        let page = guard.as_ref().ok_or_else(|| py_err("browser closed"))?;
        self.rt
            .block_on(onecrawl_cdp::domain_blocker::block_category(page, category))
            .map_err(py_err)
    }

    /// Get blocking statistics as JSON.
    fn block_stats(&self) -> PyResult<String> {
        let guard = self.page.lock().map_err(py_err)?;
        let page = guard.as_ref().ok_or_else(|| py_err("browser closed"))?;
        let stats = self
            .rt
            .block_on(onecrawl_cdp::domain_blocker::block_stats(page))
            .map_err(py_err)?;
        serde_json::to_string(&stats).map_err(py_err)
    }

    /// Clear all blocked domains.
    fn clear_blocks(&self) -> PyResult<()> {
        let guard = self.page.lock().map_err(py_err)?;
        let page = guard.as_ref().ok_or_else(|| py_err("browser closed"))?;
        self.rt
            .block_on(onecrawl_cdp::domain_blocker::clear_blocks(page))
            .map_err(py_err)
    }

    /// List currently blocked domains as JSON array.
    fn list_blocked(&self) -> PyResult<String> {
        let guard = self.page.lock().map_err(py_err)?;
        let page = guard.as_ref().ok_or_else(|| py_err("browser closed"))?;
        let domains = self
            .rt
            .block_on(onecrawl_cdp::domain_blocker::list_blocked(page))
            .map_err(py_err)?;
        serde_json::to_string(&domains).map_err(py_err)
    }

    /// Get available block categories and their domain counts as JSON.
    fn available_block_categories(&self) -> PyResult<String> {
        let cats = onecrawl_cdp::domain_blocker::available_categories();
        serde_json::to_string(&cats).map_err(py_err)
    }

    // ── Shell ─────────────────────────────────────────────────────

    /// Parse a shell command string. Returns JSON.
    fn shell_parse(&self, input: &str) -> PyResult<String> {
        let cmd = onecrawl_cdp::shell::parse_command(input);
        serde_json::to_string(&cmd).map_err(py_err)
    }

    /// Get available shell commands. Returns JSON.
    fn shell_commands(&self) -> PyResult<String> {
        let cmds = onecrawl_cdp::shell::available_commands();
        serde_json::to_string(&cmds).map_err(py_err)
    }

    /// Save shell history (JSON) to file.
    fn shell_save_history(&self, history: &str, path: &str) -> PyResult<()> {
        let h: onecrawl_cdp::ShellHistory = serde_json::from_str(history).map_err(py_err)?;
        onecrawl_cdp::shell::save_history(&h, std::path::Path::new(path)).map_err(py_err)
    }

    /// Load shell history from file. Returns JSON.
    fn shell_load_history(&self, path: &str) -> PyResult<String> {
        let h = onecrawl_cdp::shell::load_history(std::path::Path::new(path)).map_err(py_err)?;
        serde_json::to_string(&h).map_err(py_err)
    }

    // ── Streaming Extractor ───────────────────────────────────────

    /// Extract structured items using a JSON schema. Returns JSON ExtractionResult.
    fn extract_items(&self, schema_json: &str) -> PyResult<String> {
        let schema: onecrawl_cdp::ExtractionSchema =
            serde_json::from_str(schema_json).map_err(py_err)?;
        let guard = self.page.lock().map_err(py_err)?;
        let page = guard.as_ref().ok_or_else(|| py_err("browser closed"))?;
        let result = self
            .rt
            .block_on(onecrawl_cdp::streaming::extract_items(page, &schema))
            .map_err(py_err)?;
        serde_json::to_string(&result).map_err(py_err)
    }

    /// Extract items with pagination. Returns JSON ExtractionResult.
    fn extract_with_pagination(&self, schema_json: &str) -> PyResult<String> {
        let schema: onecrawl_cdp::ExtractionSchema =
            serde_json::from_str(schema_json).map_err(py_err)?;
        let guard = self.page.lock().map_err(py_err)?;
        let page = guard.as_ref().ok_or_else(|| py_err("browser closed"))?;
        let result = self
            .rt
            .block_on(onecrawl_cdp::streaming::extract_with_pagination(
                page, &schema,
            ))
            .map_err(py_err)?;
        serde_json::to_string(&result).map_err(py_err)
    }

    /// Extract a single item (no item_selector). Returns JSON object.
    fn extract_single(&self, rules_json: &str) -> PyResult<String> {
        let rules: Vec<onecrawl_cdp::ExtractionRule> =
            serde_json::from_str(rules_json).map_err(py_err)?;
        let guard = self.page.lock().map_err(py_err)?;
        let page = guard.as_ref().ok_or_else(|| py_err("browser closed"))?;
        let result = self
            .rt
            .block_on(onecrawl_cdp::streaming::extract_single(page, &rules))
            .map_err(py_err)?;
        serde_json::to_string(&result).map_err(py_err)
    }

    /// Export extracted items as CSV. Returns number of items written.
    fn export_csv(&self, items_json: &str, path: &str) -> PyResult<usize> {
        let items: Vec<onecrawl_cdp::ExtractedItem> =
            serde_json::from_str(items_json).map_err(py_err)?;
        onecrawl_cdp::streaming::export_csv(&items, std::path::Path::new(path)).map_err(py_err)
    }

    /// Export extracted items as JSON file. Returns number of items written.
    fn export_json_file(&self, items_json: &str, path: &str) -> PyResult<usize> {
        let items: Vec<onecrawl_cdp::ExtractedItem> =
            serde_json::from_str(items_json).map_err(py_err)?;
        onecrawl_cdp::streaming::export_json(&items, std::path::Path::new(path)).map_err(py_err)
    }

    // ── HTTP Client ───────────────────────────────────────────────

    /// Execute an HTTP request via browser fetch. Returns JSON HttpResponse.
    fn http_fetch(&self, request_json: &str) -> PyResult<String> {
        let request: onecrawl_cdp::HttpRequest =
            serde_json::from_str(request_json).map_err(py_err)?;
        let guard = self.page.lock().map_err(py_err)?;
        let page = guard.as_ref().ok_or_else(|| py_err("browser closed"))?;
        let resp = self
            .rt
            .block_on(onecrawl_cdp::http_client::fetch(page, &request))
            .map_err(py_err)?;
        serde_json::to_string(&resp).map_err(py_err)
    }

    /// HTTP GET via browser fetch. Returns JSON HttpResponse.
    #[pyo3(signature = (url, headers_json=None))]
    fn http_get(&self, url: &str, headers_json: Option<&str>) -> PyResult<String> {
        let headers: Option<std::collections::HashMap<String, String>> = match headers_json {
            Some(h) => Some(serde_json::from_str(h).map_err(py_err)?),
            None => None,
        };
        let guard = self.page.lock().map_err(py_err)?;
        let page = guard.as_ref().ok_or_else(|| py_err("browser closed"))?;
        let resp = self
            .rt
            .block_on(onecrawl_cdp::http_client::get(page, url, headers))
            .map_err(py_err)?;
        serde_json::to_string(&resp).map_err(py_err)
    }

    /// HTTP POST via browser fetch. Returns JSON HttpResponse.
    #[pyo3(signature = (url, body, content_type=None, headers_json=None))]
    fn http_post(
        &self,
        url: &str,
        body: &str,
        content_type: Option<&str>,
        headers_json: Option<&str>,
    ) -> PyResult<String> {
        let headers: Option<std::collections::HashMap<String, String>> = match headers_json {
            Some(h) => Some(serde_json::from_str(h).map_err(py_err)?),
            None => None,
        };
        let ct = content_type.unwrap_or("application/json");
        let guard = self.page.lock().map_err(py_err)?;
        let page = guard.as_ref().ok_or_else(|| py_err("browser closed"))?;
        let resp = self
            .rt
            .block_on(onecrawl_cdp::http_client::post(
                page, url, body, ct, headers,
            ))
            .map_err(py_err)?;
        serde_json::to_string(&resp).map_err(py_err)
    }

    /// HTTP HEAD via browser fetch. Returns JSON HttpResponse.
    fn http_head(&self, url: &str) -> PyResult<String> {
        let guard = self.page.lock().map_err(py_err)?;
        let page = guard.as_ref().ok_or_else(|| py_err("browser closed"))?;
        let resp = self
            .rt
            .block_on(onecrawl_cdp::http_client::head(page, url))
            .map_err(py_err)?;
        serde_json::to_string(&resp).map_err(py_err)
    }

    /// Fetch a URL and parse response as JSON.
    fn http_fetch_json(&self, url: &str) -> PyResult<String> {
        let guard = self.page.lock().map_err(py_err)?;
        let page = guard.as_ref().ok_or_else(|| py_err("browser closed"))?;
        let val = self
            .rt
            .block_on(onecrawl_cdp::http_client::fetch_json(page, url))
            .map_err(py_err)?;
        serde_json::to_string(&val).map_err(py_err)
    }

    // ──────────────── TLS Fingerprint ────────────────

    /// List available TLS fingerprint profile names. Returns JSON array.
    fn fingerprint_profiles(&self) -> PyResult<String> {
        let profiles = onecrawl_cdp::tls_fingerprint::browser_profiles();
        serde_json::to_string(&profiles).map_err(py_err)
    }

    /// Apply a named TLS fingerprint profile. Returns JSON array of overridden properties.
    fn apply_fingerprint(&self, name: &str) -> PyResult<String> {
        let guard = self.page.lock().map_err(py_err)?;
        let page = guard.as_ref().ok_or_else(|| py_err("browser closed"))?;
        let fp = onecrawl_cdp::tls_fingerprint::get_profile(name)
            .ok_or_else(|| py_err(format!("unknown fingerprint profile: {name}")))?;
        let overridden = self
            .rt
            .block_on(onecrawl_cdp::tls_fingerprint::apply_fingerprint(page, &fp))
            .map_err(py_err)?;
        serde_json::to_string(&overridden).map_err(py_err)
    }

    /// Apply a random TLS fingerprint. Returns JSON of the applied fingerprint.
    fn apply_random_fingerprint(&self) -> PyResult<String> {
        let guard = self.page.lock().map_err(py_err)?;
        let page = guard.as_ref().ok_or_else(|| py_err("browser closed"))?;
        let fp = onecrawl_cdp::tls_fingerprint::random_fingerprint();
        self.rt
            .block_on(onecrawl_cdp::tls_fingerprint::apply_fingerprint(page, &fp))
            .map_err(py_err)?;
        serde_json::to_string(&fp).map_err(py_err)
    }

    /// Detect current browser fingerprint. Returns JSON.
    fn detect_fingerprint(&self) -> PyResult<String> {
        let guard = self.page.lock().map_err(py_err)?;
        let page = guard.as_ref().ok_or_else(|| py_err("browser closed"))?;
        let fp = self
            .rt
            .block_on(onecrawl_cdp::tls_fingerprint::detect_fingerprint(page))
            .map_err(py_err)?;
        serde_json::to_string(&fp).map_err(py_err)
    }

    /// Apply a custom fingerprint from JSON string. Returns JSON array of overridden properties.
    fn apply_custom_fingerprint(&self, json: &str) -> PyResult<String> {
        let guard = self.page.lock().map_err(py_err)?;
        let page = guard.as_ref().ok_or_else(|| py_err("browser closed"))?;
        let fp: onecrawl_cdp::BrowserFingerprint = serde_json::from_str(json)
            .map_err(|e| py_err(format!("invalid fingerprint JSON: {e}")))?;
        let overridden = self
            .rt
            .block_on(onecrawl_cdp::tls_fingerprint::apply_fingerprint(page, &fp))
            .map_err(py_err)?;
        serde_json::to_string(&overridden).map_err(py_err)
    }

    // ──────────────── Page Snapshot ────────────────

    /// Take a DOM snapshot of the current page. Returns JSON.
    fn take_snapshot(&self) -> PyResult<String> {
        let guard = self.page.lock().map_err(py_err)?;
        let page = guard.as_ref().ok_or_else(|| py_err("browser closed"))?;
        let snap = self
            .rt
            .block_on(onecrawl_cdp::snapshot::take_snapshot(page))
            .map_err(py_err)?;
        serde_json::to_string(&snap).map_err(py_err)
    }

    /// Compare two snapshots (JSON strings). Returns JSON diff.
    fn compare_snapshots(&self, before_json: &str, after_json: &str) -> PyResult<String> {
        let before: onecrawl_cdp::DomSnapshot = serde_json::from_str(before_json)
            .map_err(|e| py_err(format!("invalid before snapshot: {e}")))?;
        let after: onecrawl_cdp::DomSnapshot = serde_json::from_str(after_json)
            .map_err(|e| py_err(format!("invalid after snapshot: {e}")))?;
        let diff = onecrawl_cdp::snapshot::compare_snapshots(&before, &after);
        serde_json::to_string(&diff).map_err(py_err)
    }

    /// Save a snapshot (JSON string) to a file.
    fn save_snapshot(&self, snapshot_json: &str, path: &str) -> PyResult<()> {
        let snap: onecrawl_cdp::DomSnapshot = serde_json::from_str(snapshot_json)
            .map_err(|e| py_err(format!("invalid snapshot JSON: {e}")))?;
        onecrawl_cdp::snapshot::save_snapshot(&snap, std::path::Path::new(path)).map_err(py_err)
    }

    /// Load a snapshot from a file. Returns JSON string.
    fn load_snapshot(&self, path: &str) -> PyResult<String> {
        let snap =
            onecrawl_cdp::snapshot::load_snapshot(std::path::Path::new(path)).map_err(py_err)?;
        serde_json::to_string(&snap).map_err(py_err)
    }

    /// Watch for DOM changes at an interval. Returns JSON array of diffs.
    #[pyo3(signature = (interval_ms, selector=None, count=None))]
    fn watch_for_changes(
        &self,
        interval_ms: u64,
        selector: Option<&str>,
        count: Option<usize>,
    ) -> PyResult<String> {
        let guard = self.page.lock().map_err(py_err)?;
        let page = guard.as_ref().ok_or_else(|| py_err("browser closed"))?;
        let diffs = self
            .rt
            .block_on(onecrawl_cdp::snapshot::watch_for_changes(
                page,
                interval_ms,
                selector,
                count.unwrap_or(3),
            ))
            .map_err(py_err)?;
        serde_json::to_string(&diffs).map_err(py_err)
    }

    // ──────────────── Rate Limiter ────────────────

    /// Set rate limiter config. Pass a preset name or JSON config string.
    #[pyo3(signature = (config_or_preset=None))]
    fn rate_limit_set(&self, config_or_preset: Option<&str>) -> PyResult<String> {
        let mut rl = self.rate_limiter.lock().map_err(py_err)?;
        let config = match config_or_preset {
            Some(s) => {
                let presets = onecrawl_cdp::rate_limiter::presets();
                if let Some(cfg) = presets.get(s) {
                    cfg.clone()
                } else {
                    serde_json::from_str(s).map_err(|e| py_err(format!("invalid config: {e}")))?
                }
            }
            None => onecrawl_cdp::RateLimitConfig::default(),
        };
        *rl = onecrawl_cdp::RateLimitState::new(config);
        serde_json::to_string(&onecrawl_cdp::rate_limiter::get_stats(&rl)).map_err(py_err)
    }

    /// Check if a request can proceed under rate limits.
    fn rate_limit_can_proceed(&self) -> PyResult<bool> {
        let rl = self.rate_limiter.lock().map_err(py_err)?;
        Ok(onecrawl_cdp::rate_limiter::can_proceed(&rl))
    }

    /// Record a request. Returns True if allowed, False if throttled.
    fn rate_limit_record(&self) -> PyResult<bool> {
        let mut rl = self.rate_limiter.lock().map_err(py_err)?;
        Ok(onecrawl_cdp::rate_limiter::record_request(&mut rl))
    }

    /// Get ms to wait before next allowed request.
    fn rate_limit_wait(&self) -> PyResult<u64> {
        let rl = self.rate_limiter.lock().map_err(py_err)?;
        Ok(onecrawl_cdp::rate_limiter::wait_duration(&rl))
    }

    /// Get rate limiter statistics as JSON.
    fn rate_limit_stats(&self) -> PyResult<String> {
        let rl = self.rate_limiter.lock().map_err(py_err)?;
        serde_json::to_string(&onecrawl_cdp::rate_limiter::get_stats(&rl)).map_err(py_err)
    }

    /// Reset rate limiter counters.
    fn rate_limit_reset(&self) -> PyResult<()> {
        let mut rl = self.rate_limiter.lock().map_err(py_err)?;
        onecrawl_cdp::rate_limiter::reset(&mut rl);
        Ok(())
    }

    /// List rate limiter presets as JSON map.
    fn rate_limit_presets(&self) -> PyResult<String> {
        serde_json::to_string(&onecrawl_cdp::rate_limiter::presets()).map_err(py_err)
    }

    // ──────────────── Retry Queue ────────────────

    /// Enqueue a URL/operation for retry. Returns the item id.
    #[pyo3(signature = (url, operation, payload=None))]
    fn retry_enqueue(&self, url: &str, operation: &str, payload: Option<&str>) -> PyResult<String> {
        let mut q = self.retry_queue.lock().map_err(py_err)?;
        Ok(onecrawl_cdp::retry_queue::enqueue(
            &mut q, url, operation, payload,
        ))
    }

    /// Get the next item due for retry as JSON, or None.
    fn retry_next(&self) -> PyResult<Option<String>> {
        let mut q = self.retry_queue.lock().map_err(py_err)?;
        match onecrawl_cdp::retry_queue::get_next(&mut q) {
            Some(item) => Ok(Some(serde_json::to_string(item).map_err(py_err)?)),
            None => Ok(None),
        }
    }

    /// Mark a retry item as successful.
    fn retry_success(&self, id: &str) -> PyResult<()> {
        let mut q = self.retry_queue.lock().map_err(py_err)?;
        onecrawl_cdp::retry_queue::mark_success(&mut q, id);
        Ok(())
    }

    /// Mark a retry item as failed.
    fn retry_fail(&self, id: &str, error: &str) -> PyResult<()> {
        let mut q = self.retry_queue.lock().map_err(py_err)?;
        onecrawl_cdp::retry_queue::mark_failure(&mut q, id, error);
        Ok(())
    }

    /// Get retry queue statistics as JSON.
    fn retry_stats(&self) -> PyResult<String> {
        let q = self.retry_queue.lock().map_err(py_err)?;
        serde_json::to_string(&onecrawl_cdp::retry_queue::get_stats(&q)).map_err(py_err)
    }

    /// Clear completed items. Returns count removed.
    fn retry_clear(&self) -> PyResult<usize> {
        let mut q = self.retry_queue.lock().map_err(py_err)?;
        Ok(onecrawl_cdp::retry_queue::clear_completed(&mut q))
    }

    /// Save the retry queue to a file.
    fn retry_save(&self, path: &str) -> PyResult<()> {
        let q = self.retry_queue.lock().map_err(py_err)?;
        onecrawl_cdp::retry_queue::save_queue(&q, std::path::Path::new(path)).map_err(py_err)
    }

    /// Load the retry queue from a file.
    fn retry_load(&self, path: &str) -> PyResult<()> {
        let loaded =
            onecrawl_cdp::retry_queue::load_queue(std::path::Path::new(path)).map_err(py_err)?;
        let mut q = self.retry_queue.lock().map_err(py_err)?;
        *q = loaded;
        Ok(())
    }

    // ──────────────── Data Pipeline ────────────────

    /// Execute a data pipeline. Accepts pipeline JSON and items JSON array.
    fn pipeline_execute(&self, pipeline_json: &str, items_json: &str) -> PyResult<String> {
        let pipeline: onecrawl_cdp::Pipeline = serde_json::from_str(pipeline_json)
            .map_err(|e| py_err(format!("invalid pipeline JSON: {e}")))?;
        let items: Vec<std::collections::HashMap<String, String>> =
            serde_json::from_str(items_json)
                .map_err(|e| py_err(format!("invalid items JSON: {e}")))?;
        let result = onecrawl_cdp::data_pipeline::execute_pipeline(&pipeline, items);
        serde_json::to_string(&result).map_err(py_err)
    }

    /// Validate a pipeline configuration. Returns JSON array of error strings.
    fn pipeline_validate(&self, pipeline_json: &str) -> PyResult<String> {
        let pipeline: onecrawl_cdp::Pipeline = serde_json::from_str(pipeline_json)
            .map_err(|e| py_err(format!("invalid pipeline JSON: {e}")))?;
        let errors = onecrawl_cdp::data_pipeline::validate_pipeline(&pipeline);
        serde_json::to_string(&errors).map_err(py_err)
    }

    /// Save a pipeline definition to a JSON file.
    fn pipeline_save(&self, pipeline_json: &str, path: &str) -> PyResult<()> {
        let pipeline: onecrawl_cdp::Pipeline = serde_json::from_str(pipeline_json)
            .map_err(|e| py_err(format!("invalid pipeline JSON: {e}")))?;
        onecrawl_cdp::data_pipeline::save_pipeline(&pipeline, std::path::Path::new(path))
            .map_err(py_err)
    }

    /// Load a pipeline definition from a JSON file. Returns JSON string.
    fn pipeline_load(&self, path: &str) -> PyResult<String> {
        let pipeline = onecrawl_cdp::data_pipeline::load_pipeline(std::path::Path::new(path))
            .map_err(py_err)?;
        serde_json::to_string(&pipeline).map_err(py_err)
    }

    /// Export pipeline results to a file. Format: "json", "jsonl", or "csv".
    #[pyo3(signature = (result_json, path, format=None))]
    fn pipeline_export(
        &self,
        result_json: &str,
        path: &str,
        format: Option<&str>,
    ) -> PyResult<usize> {
        let result: onecrawl_cdp::PipelineResult = serde_json::from_str(result_json)
            .map_err(|e| py_err(format!("invalid result JSON: {e}")))?;
        let fmt = format.unwrap_or("json");
        onecrawl_cdp::data_pipeline::export_processed(&result, std::path::Path::new(path), fmt)
            .map_err(py_err)
    }

    // ──────────────── Structured Data ────────────────

    /// Extract all structured data (JSON-LD, OG, Twitter, metadata). Returns JSON.
    fn structured_extract_all(&self) -> PyResult<String> {
        let guard = self.page.lock().map_err(py_err)?;
        let page = guard.as_ref().ok_or_else(|| py_err("browser closed"))?;
        let data = self
            .rt
            .block_on(onecrawl_cdp::structured_data::extract_all(page))
            .map_err(py_err)?;
        serde_json::to_string(&data).map_err(py_err)
    }

    /// Extract JSON-LD from the current page. Returns JSON array.
    fn structured_json_ld(&self) -> PyResult<String> {
        let guard = self.page.lock().map_err(py_err)?;
        let page = guard.as_ref().ok_or_else(|| py_err("browser closed"))?;
        let data = self
            .rt
            .block_on(onecrawl_cdp::structured_data::extract_json_ld(page))
            .map_err(py_err)?;
        serde_json::to_string(&data).map_err(py_err)
    }

    /// Extract OpenGraph metadata. Returns JSON.
    fn structured_open_graph(&self) -> PyResult<String> {
        let guard = self.page.lock().map_err(py_err)?;
        let page = guard.as_ref().ok_or_else(|| py_err("browser closed"))?;
        let data = self
            .rt
            .block_on(onecrawl_cdp::structured_data::extract_open_graph(page))
            .map_err(py_err)?;
        serde_json::to_string(&data).map_err(py_err)
    }

    /// Extract Twitter Card metadata. Returns JSON.
    fn structured_twitter_card(&self) -> PyResult<String> {
        let guard = self.page.lock().map_err(py_err)?;
        let page = guard.as_ref().ok_or_else(|| py_err("browser closed"))?;
        let data = self
            .rt
            .block_on(onecrawl_cdp::structured_data::extract_twitter_card(page))
            .map_err(py_err)?;
        serde_json::to_string(&data).map_err(py_err)
    }

    /// Extract page metadata. Returns JSON.
    fn structured_metadata(&self) -> PyResult<String> {
        let guard = self.page.lock().map_err(py_err)?;
        let page = guard.as_ref().ok_or_else(|| py_err("browser closed"))?;
        let data = self
            .rt
            .block_on(onecrawl_cdp::structured_data::extract_metadata(page))
            .map_err(py_err)?;
        serde_json::to_string(&data).map_err(py_err)
    }

    /// Validate structured data completeness. Returns JSON array of warnings.
    fn structured_validate(&self, data_json: &str) -> PyResult<String> {
        let data: onecrawl_cdp::StructuredDataResult = serde_json::from_str(data_json)
            .map_err(|e| py_err(format!("invalid data JSON: {e}")))?;
        let warnings = onecrawl_cdp::structured_data::validate_schema(&data);
        serde_json::to_string(&warnings).map_err(py_err)
    }

    // ── Proxy Health ────────────────────────────────────────────────

    /// Check a single proxy health via browser fetch. Returns JSON.
    fn proxy_health_check(&self, proxy_url: &str, config_json: Option<&str>) -> PyResult<String> {
        let guard = self.page.lock().map_err(py_err)?;
        let page = guard.as_ref().ok_or_else(|| py_err("browser closed"))?;
        let config: onecrawl_cdp::ProxyHealthConfig = match config_json {
            Some(j) => {
                serde_json::from_str(j).map_err(|e| py_err(format!("invalid config JSON: {e}")))?
            }
            None => onecrawl_cdp::ProxyHealthConfig::default(),
        };
        let result = self
            .rt
            .block_on(onecrawl_cdp::proxy_health::check_proxy(
                page, proxy_url, &config,
            ))
            .map_err(py_err)?;
        serde_json::to_string(&result).map_err(py_err)
    }

    /// Check multiple proxies. Returns JSON array.
    fn proxy_health_check_all(&self, proxies_json: &str) -> PyResult<String> {
        let guard = self.page.lock().map_err(py_err)?;
        let page = guard.as_ref().ok_or_else(|| py_err("browser closed"))?;
        let proxies: Vec<String> = serde_json::from_str(proxies_json)
            .map_err(|e| py_err(format!("invalid proxies JSON: {e}")))?;
        let config = onecrawl_cdp::ProxyHealthConfig::default();
        let results = self
            .rt
            .block_on(onecrawl_cdp::proxy_health::check_proxies(
                page, &proxies, &config,
            ))
            .map_err(py_err)?;
        serde_json::to_string(&results).map_err(py_err)
    }

    /// Score a single proxy health result. Returns the score (0-100).
    fn proxy_health_score(&self, result_json: &str) -> PyResult<u32> {
        let result: onecrawl_cdp::ProxyHealthResult = serde_json::from_str(result_json)
            .map_err(|e| py_err(format!("invalid result JSON: {e}")))?;
        Ok(onecrawl_cdp::proxy_health::score_proxy(&result))
    }

    /// Filter proxy results by minimum score. Returns JSON array.
    fn proxy_health_filter(&self, results_json: &str, min_score: u32) -> PyResult<String> {
        let results: Vec<onecrawl_cdp::ProxyHealthResult> = serde_json::from_str(results_json)
            .map_err(|e| py_err(format!("invalid results JSON: {e}")))?;
        let filtered = onecrawl_cdp::proxy_health::filter_healthy(&results, min_score);
        serde_json::to_string(&filtered).map_err(py_err)
    }

    /// Rank proxy results by score descending. Returns JSON array.
    fn proxy_health_rank(&self, results_json: &str) -> PyResult<String> {
        let results: Vec<onecrawl_cdp::ProxyHealthResult> = serde_json::from_str(results_json)
            .map_err(|e| py_err(format!("invalid results JSON: {e}")))?;
        let ranked = onecrawl_cdp::proxy_health::rank_proxies(&results);
        serde_json::to_string(&ranked).map_err(py_err)
    }

    // ── Captcha ─────────────────────────────────────────────────────

    /// Detect CAPTCHA presence on the current page. Returns JSON.
    fn captcha_detect(&self) -> PyResult<String> {
        let guard = self.page.lock().map_err(py_err)?;
        let page = guard.as_ref().ok_or_else(|| py_err("browser closed"))?;
        let detection = self
            .rt
            .block_on(onecrawl_cdp::captcha::detect_captcha(page))
            .map_err(py_err)?;
        serde_json::to_string(&detection).map_err(py_err)
    }

    /// Wait for a CAPTCHA to appear. Returns JSON.
    fn captcha_wait(&self, timeout_ms: Option<u64>) -> PyResult<String> {
        let guard = self.page.lock().map_err(py_err)?;
        let page = guard.as_ref().ok_or_else(|| py_err("browser closed"))?;
        let timeout = timeout_ms.unwrap_or(30000);
        let detection = self
            .rt
            .block_on(onecrawl_cdp::captcha::wait_for_captcha(page, timeout))
            .map_err(py_err)?;
        serde_json::to_string(&detection).map_err(py_err)
    }

    /// Screenshot CAPTCHA element. Returns rect JSON or base64.
    fn captcha_screenshot(&self) -> PyResult<String> {
        let guard = self.page.lock().map_err(py_err)?;
        let page = guard.as_ref().ok_or_else(|| py_err("browser closed"))?;
        let detection = self
            .rt
            .block_on(onecrawl_cdp::captcha::detect_captcha(page))
            .map_err(py_err)?;
        if !detection.detected {
            return Err(py_err("no captcha detected"));
        }
        self.rt
            .block_on(onecrawl_cdp::captcha::screenshot_captcha(page, &detection))
            .map_err(py_err)
    }

    /// Inject a CAPTCHA solution token. Returns True if successful.
    fn captcha_inject(&self, solution: &str) -> PyResult<bool> {
        let guard = self.page.lock().map_err(py_err)?;
        let page = guard.as_ref().ok_or_else(|| py_err("browser closed"))?;
        let detection = self
            .rt
            .block_on(onecrawl_cdp::captcha::detect_captcha(page))
            .map_err(py_err)?;
        if !detection.detected {
            return Err(py_err("no captcha detected"));
        }
        self.rt
            .block_on(onecrawl_cdp::captcha::inject_solution(
                page, &detection, solution,
            ))
            .map_err(py_err)
    }

    /// List supported CAPTCHA types. Returns JSON array of [type, description].
    fn captcha_types(&self) -> PyResult<String> {
        let types = onecrawl_cdp::captcha::supported_types();
        serde_json::to_string(&types).map_err(py_err)
    }

    // ──────────────── Task Scheduler ────────────────

    /// Add a scheduled task. Returns the task ID.
    fn scheduler_add_task(
        &self,
        name: &str,
        task_type: &str,
        config: &str,
        schedule_json: &str,
    ) -> PyResult<String> {
        let schedule: onecrawl_cdp::TaskSchedule = serde_json::from_str(schedule_json)
            .map_err(|e| py_err(format!("invalid schedule JSON: {e}")))?;
        let mut sched = self.scheduler.lock().map_err(py_err)?;
        Ok(onecrawl_cdp::scheduler::add_task(
            &mut sched, name, task_type, config, schedule,
        ))
    }

    /// Remove a scheduled task by ID.
    fn scheduler_remove_task(&self, id: &str) -> PyResult<bool> {
        let mut sched = self.scheduler.lock().map_err(py_err)?;
        Ok(onecrawl_cdp::scheduler::remove_task(&mut sched, id))
    }

    /// Pause a scheduled task by ID.
    fn scheduler_pause_task(&self, id: &str) -> PyResult<bool> {
        let mut sched = self.scheduler.lock().map_err(py_err)?;
        Ok(onecrawl_cdp::scheduler::pause_task(&mut sched, id))
    }

    /// Resume a paused task by ID.
    fn scheduler_resume_task(&self, id: &str) -> PyResult<bool> {
        let mut sched = self.scheduler.lock().map_err(py_err)?;
        Ok(onecrawl_cdp::scheduler::resume_task(&mut sched, id))
    }

    /// Get tasks that are due to execute. Returns JSON array.
    fn scheduler_get_due_tasks(&self) -> PyResult<String> {
        let sched = self.scheduler.lock().map_err(py_err)?;
        let due = onecrawl_cdp::scheduler::get_due_tasks(&sched);
        serde_json::to_string(&due).map_err(py_err)
    }

    /// Record a task execution result. Input is JSON of TaskResult.
    fn scheduler_record_result(&self, result_json: &str) -> PyResult<()> {
        let result: onecrawl_cdp::TaskResult = serde_json::from_str(result_json)
            .map_err(|e| py_err(format!("invalid result JSON: {e}")))?;
        let mut sched = self.scheduler.lock().map_err(py_err)?;
        onecrawl_cdp::scheduler::record_result(&mut sched, result);
        Ok(())
    }

    /// Get scheduler statistics. Returns JSON map.
    fn scheduler_get_stats(&self) -> PyResult<String> {
        let sched = self.scheduler.lock().map_err(py_err)?;
        let stats = onecrawl_cdp::scheduler::get_stats(&sched);
        serde_json::to_string(&stats).map_err(py_err)
    }

    /// List all tasks. Returns JSON array.
    fn scheduler_list_tasks(&self) -> PyResult<String> {
        let sched = self.scheduler.lock().map_err(py_err)?;
        serde_json::to_string(&sched.tasks).map_err(py_err)
    }

    /// Save scheduler state to a file.
    fn scheduler_save(&self, path: &str) -> PyResult<()> {
        let sched = self.scheduler.lock().map_err(py_err)?;
        onecrawl_cdp::scheduler::save_scheduler(&sched, std::path::Path::new(path)).map_err(py_err)
    }

    /// Load scheduler state from a file.
    fn scheduler_load(&self, path: &str) -> PyResult<()> {
        let loaded =
            onecrawl_cdp::scheduler::load_scheduler(std::path::Path::new(path)).map_err(py_err)?;
        let mut sched = self.scheduler.lock().map_err(py_err)?;
        *sched = loaded;
        Ok(())
    }

    // ──────────────── Session Pool ────────────────

    /// Add a session to the pool. Returns the session ID.
    #[pyo3(signature = (name, tags=None))]
    fn pool_add_session(&self, name: &str, tags: Option<Vec<String>>) -> PyResult<String> {
        let mut pool = self.session_pool.lock().map_err(py_err)?;
        Ok(onecrawl_cdp::session_pool::add_session(
            &mut pool, name, tags,
        ))
    }

    /// Get the next available session. Returns JSON or None.
    fn pool_get_next(&self) -> PyResult<Option<String>> {
        let mut pool = self.session_pool.lock().map_err(py_err)?;
        match onecrawl_cdp::session_pool::get_next(&mut pool) {
            Some(s) => {
                let json = serde_json::to_string(s).map_err(py_err)?;
                Ok(Some(json))
            }
            None => Ok(None),
        }
    }

    /// Mark a pool session as busy.
    fn pool_mark_busy(&self, id: &str) -> PyResult<()> {
        let mut pool = self.session_pool.lock().map_err(py_err)?;
        onecrawl_cdp::session_pool::mark_busy(&mut pool, id);
        Ok(())
    }

    /// Mark a pool session as idle.
    fn pool_mark_idle(&self, id: &str) -> PyResult<()> {
        let mut pool = self.session_pool.lock().map_err(py_err)?;
        onecrawl_cdp::session_pool::mark_idle(&mut pool, id);
        Ok(())
    }

    /// Mark a pool session as errored.
    fn pool_mark_error(&self, id: &str, error: &str) -> PyResult<()> {
        let mut pool = self.session_pool.lock().map_err(py_err)?;
        onecrawl_cdp::session_pool::mark_error(&mut pool, id, error);
        Ok(())
    }

    /// Close a pool session.
    fn pool_close_session(&self, id: &str) -> PyResult<()> {
        let mut pool = self.session_pool.lock().map_err(py_err)?;
        onecrawl_cdp::session_pool::close_session(&mut pool, id);
        Ok(())
    }

    /// Get pool statistics. Returns JSON.
    fn pool_get_stats(&self) -> PyResult<String> {
        let pool = self.session_pool.lock().map_err(py_err)?;
        let stats = onecrawl_cdp::session_pool::get_stats(&pool);
        serde_json::to_string(&stats).map_err(py_err)
    }

    /// Clean up idle sessions past timeout. Returns number closed.
    fn pool_cleanup_idle(&self) -> PyResult<usize> {
        let mut pool = self.session_pool.lock().map_err(py_err)?;
        Ok(onecrawl_cdp::session_pool::cleanup_idle(&mut pool))
    }

    /// Save pool state to a file.
    fn pool_save(&self, path: &str) -> PyResult<()> {
        let pool = self.session_pool.lock().map_err(py_err)?;
        onecrawl_cdp::session_pool::save_pool(&pool, std::path::Path::new(path)).map_err(py_err)
    }

    /// Load pool state from a file.
    fn pool_load(&self, path: &str) -> PyResult<()> {
        let loaded =
            onecrawl_cdp::session_pool::load_pool(std::path::Path::new(path)).map_err(py_err)?;
        let mut pool = self.session_pool.lock().map_err(py_err)?;
        *pool = loaded;
        Ok(())
    }

    // ──────────────── Passkey / WebAuthn ────────────────

    /// Enable virtual WebAuthn authenticator for passkey simulation.
    #[pyo3(signature = (protocol=None, transport=None))]
    fn enable_passkey(&self, protocol: Option<String>, transport: Option<String>) -> PyResult<()> {
        let guard = self.page.lock().map_err(py_err)?;
        let page = guard.as_ref().ok_or_else(|| py_err("browser closed"))?;
        let config = onecrawl_cdp::webauthn::VirtualAuthenticator {
            id: format!(
                "auth-{}",
                std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_millis()
            ),
            protocol: protocol.unwrap_or_else(|| "ctap2".into()),
            transport: transport.unwrap_or_else(|| "internal".into()),
            has_resident_key: true,
            has_user_verification: true,
            is_user_verified: true,
        };
        self.rt
            .block_on(onecrawl_cdp::webauthn::enable_virtual_authenticator(page, &config))
            .map_err(py_err)
    }

    /// Add a passkey credential to the virtual authenticator.
    #[pyo3(signature = (credential_id, rp_id, user_handle=None))]
    fn add_passkey_credential(
        &self,
        credential_id: String,
        rp_id: String,
        user_handle: Option<String>,
    ) -> PyResult<()> {
        let guard = self.page.lock().map_err(py_err)?;
        let page = guard.as_ref().ok_or_else(|| py_err("browser closed"))?;
        let cred = onecrawl_cdp::webauthn::VirtualCredential {
            credential_id,
            rp_id,
            user_handle: user_handle.unwrap_or_default(),
            sign_count: 0,
        };
        self.rt
            .block_on(onecrawl_cdp::webauthn::add_virtual_credential(page, &cred))
            .map_err(py_err)
    }

    /// Get all stored passkey credentials as JSON.
    fn get_passkey_credentials(&self) -> PyResult<String> {
        let guard = self.page.lock().map_err(py_err)?;
        let page = guard.as_ref().ok_or_else(|| py_err("browser closed"))?;
        let creds = self
            .rt
            .block_on(onecrawl_cdp::webauthn::get_virtual_credentials(page))
            .map_err(py_err)?;
        serde_json::to_string(&creds).map_err(py_err)
    }

    /// Get the WebAuthn operation log as JSON.
    fn get_passkey_log(&self) -> PyResult<String> {
        let guard = self.page.lock().map_err(py_err)?;
        let page = guard.as_ref().ok_or_else(|| py_err("browser closed"))?;
        let log = self
            .rt
            .block_on(onecrawl_cdp::webauthn::get_webauthn_log(page))
            .map_err(py_err)?;
        serde_json::to_string(&log).map_err(py_err)
    }

    /// Disable the virtual WebAuthn authenticator.
    fn disable_passkey(&self) -> PyResult<()> {
        let guard = self.page.lock().map_err(py_err)?;
        let page = guard.as_ref().ok_or_else(|| py_err("browser closed"))?;
        self.rt
            .block_on(onecrawl_cdp::webauthn::disable_virtual_authenticator(page))
            .map_err(py_err)
    }

    /// Remove a passkey credential by ID. Returns true if removed.
    fn remove_passkey_credential(&self, credential_id: String) -> PyResult<bool> {
        let guard = self.page.lock().map_err(py_err)?;
        let page = guard.as_ref().ok_or_else(|| py_err("browser closed"))?;
        self.rt
            .block_on(onecrawl_cdp::webauthn::remove_virtual_credential(page, &credential_id))
            .map_err(py_err)
    }

    // ── CDP-native passkey (real ECDSA, server-verifiable) ─────────────────

    /// Enable Chrome's CDP WebAuthn domain. Must be called before `cdp_create_authenticator`.
    fn cdp_passkey_enable(&self) -> PyResult<()> {
        let guard = self.page.lock().map_err(py_err)?;
        let page = guard.as_ref().ok_or_else(|| py_err("browser closed"))?;
        self.rt
            .block_on(onecrawl_cdp::cdp_enable(page))
            .map_err(py_err)
    }

    /// Create a CTAP2.1 virtual authenticator. Returns authenticator_id string.
    fn cdp_create_authenticator(&self) -> PyResult<String> {
        let guard = self.page.lock().map_err(py_err)?;
        let page = guard.as_ref().ok_or_else(|| py_err("browser closed"))?;
        self.rt
            .block_on(onecrawl_cdp::cdp_create_authenticator(page))
            .map_err(py_err)
    }

    /// Get all credentials from a CDP virtual authenticator as JSON array string.
    fn cdp_get_credentials(&self, authenticator_id: String) -> PyResult<String> {
        let guard = self.page.lock().map_err(py_err)?;
        let page = guard.as_ref().ok_or_else(|| py_err("browser closed"))?;
        let creds = self.rt
            .block_on(onecrawl_cdp::cdp_get_credentials(page, &authenticator_id))
            .map_err(py_err)?;
        serde_json::to_string(&creds).map_err(py_err)
    }

    /// Inject a passkey credential JSON into the CDP virtual authenticator.
    fn cdp_add_credential(
        &self,
        authenticator_id: String,
        credential_json: String,
    ) -> PyResult<()> {
        let cred: onecrawl_cdp::PasskeyCredential =
            serde_json::from_str(&credential_json)
                .map_err(|e| py_err(format!("invalid credential JSON: {e}")))?;
        let guard = self.page.lock().map_err(py_err)?;
        let page = guard.as_ref().ok_or_else(|| py_err("browser closed"))?;
        self.rt
            .block_on(onecrawl_cdp::cdp_add_credential(page, &authenticator_id, &cred))
            .map_err(py_err)
    }

    // ── Passkey vault ───────────────────────────────────────────────────────

    /// List vault rp_ids and counts. Returns JSON: `[{"rpId": str, "count": int}]`.
    fn passkey_vault_list(&self) -> PyResult<String> {
        let vault = onecrawl_cdp::load_vault().map_err(py_err)?;
        let list: Vec<serde_json::Value> = onecrawl_cdp::vault_list(&vault)
            .into_iter()
            .map(|(rp_id, count)| serde_json::json!({ "rpId": rp_id, "count": count }))
            .collect();
        serde_json::to_string(&list).map_err(py_err)
    }

    /// Get credentials for an rp_id from vault as JSON array string.
    fn passkey_vault_get(&self, rp_id: String) -> PyResult<String> {
        let vault = onecrawl_cdp::load_vault().map_err(py_err)?;
        let creds = onecrawl_cdp::vault_get(&vault, &rp_id);
        serde_json::to_string(&creds).map_err(py_err)
    }

    /// Add credentials (JSON array) to the vault. Deduplicates by credential_id.
    fn passkey_vault_add(&self, credentials_json: String) -> PyResult<()> {
        let creds: Vec<onecrawl_cdp::PasskeyCredential> =
            serde_json::from_str(&credentials_json)
                .map_err(|e| py_err(format!("invalid credentials JSON: {e}")))?;
        let mut vault = onecrawl_cdp::load_vault().map_err(py_err)?;
        onecrawl_cdp::vault_add(&mut vault, creds);
        onecrawl_cdp::save_vault(&vault).map_err(py_err)
    }

    /// Remove a credential from the vault by credential_id. Returns True if removed.
    fn passkey_vault_remove(&self, credential_id: String) -> PyResult<bool> {
        let mut vault = onecrawl_cdp::load_vault().map_err(py_err)?;
        let removed = onecrawl_cdp::vault_remove(&mut vault, &credential_id);
        if removed {
            onecrawl_cdp::save_vault(&vault).map_err(py_err)?;
        }
        Ok(removed)
    }

    /// Import passkeys from a Bitwarden JSON export. Returns JSON array of credentials.
    ///
    /// If `save_to_vault=True` (default), saves to `~/.onecrawl/passkeys/vault.json`.
    #[pyo3(signature = (file_path, save_to_vault=true))]
    fn passkey_import_bitwarden(
        &self,
        file_path: String,
        save_to_vault: bool,
    ) -> PyResult<String> {
        let creds = onecrawl_cdp::import_bitwarden(std::path::Path::new(&file_path))
            .map_err(py_err)?;
        if save_to_vault && !creds.is_empty() {
            let mut vault = onecrawl_cdp::load_vault().map_err(py_err)?;
            onecrawl_cdp::vault_add(&mut vault, creds.clone());
            onecrawl_cdp::save_vault(&vault).map_err(py_err)?;
        }
        serde_json::to_string(&creds).map_err(py_err)
    }

    /// Import passkeys from a 1Password export.data JSON file (extracted from .1pux).
    ///
    /// If `save_to_vault=True` (default), saves to vault.
    #[pyo3(signature = (file_path, save_to_vault=true))]
    fn passkey_import_1password(
        &self,
        file_path: String,
        save_to_vault: bool,
    ) -> PyResult<String> {
        let creds = onecrawl_cdp::import_1password_json(std::path::Path::new(&file_path))
            .map_err(py_err)?;
        if save_to_vault && !creds.is_empty() {
            let mut vault = onecrawl_cdp::load_vault().map_err(py_err)?;
            onecrawl_cdp::vault_add(&mut vault, creds.clone());
            onecrawl_cdp::save_vault(&vault).map_err(py_err)?;
        }
        serde_json::to_string(&creds).map_err(py_err)
    }

    /// Import passkeys from a FIDO Alliance CXF v1.0 JSON file.
    ///
    /// If `save_to_vault=True` (default), saves to vault.
    #[pyo3(signature = (file_path, save_to_vault=true))]
    fn passkey_import_cxf(
        &self,
        file_path: String,
        save_to_vault: bool,
    ) -> PyResult<String> {
        let creds = onecrawl_cdp::import_cxf(std::path::Path::new(&file_path))
            .map_err(py_err)?;
        if save_to_vault && !creds.is_empty() {
            let mut vault = onecrawl_cdp::load_vault().map_err(py_err)?;
            onecrawl_cdp::vault_add(&mut vault, creds.clone());
            onecrawl_cdp::save_vault(&vault).map_err(py_err)?;
        }
        serde_json::to_string(&creds).map_err(py_err)
    }

    // ── Tabs ──────────────────────────────────────────────────────────

    /// List all open tabs.
    fn list_tabs(&self) -> PyResult<String> {
        let tabs = self
            .rt
            .block_on(onecrawl_cdp::tabs::list_tabs(self.session.browser()))
            .map_err(py_err)?;
        serde_json::to_string(&tabs).map_err(py_err)
    }

    /// Close a tab by index.
    fn close_tab(&self, index: usize) -> PyResult<()> {
        self.rt
            .block_on(onecrawl_cdp::tabs::close_tab(
                self.session.browser(),
                index,
            ))
            .map_err(py_err)
    }

    /// Get number of open tabs.
    fn tab_count(&self) -> PyResult<usize> {
        self.rt
            .block_on(onecrawl_cdp::tabs::tab_count(self.session.browser()))
            .map_err(py_err)
    }

    // ── Downloads ─────────────────────────────────────────────────────

    /// Set the download path.
    fn set_download_path(&self, path: &str) -> PyResult<()> {
        let guard = self.page.lock().map_err(py_err)?;
        let page = guard.as_ref().ok_or_else(|| py_err("browser closed"))?;
        self.rt
            .block_on(onecrawl_cdp::downloads::set_download_path(
                page,
                std::path::Path::new(path),
            ))
            .map_err(py_err)
    }

    /// Get list of downloads as JSON.
    fn get_downloads(&self) -> PyResult<String> {
        let guard = self.page.lock().map_err(py_err)?;
        let page = guard.as_ref().ok_or_else(|| py_err("browser closed"))?;
        let downloads = self
            .rt
            .block_on(onecrawl_cdp::downloads::get_downloads(page))
            .map_err(py_err)?;
        serde_json::to_string(&downloads).map_err(py_err)
    }

    /// Download a file from a URL.
    fn download_file(&self, url: &str) -> PyResult<String> {
        let guard = self.page.lock().map_err(py_err)?;
        let page = guard.as_ref().ok_or_else(|| py_err("browser closed"))?;
        let result = self
            .rt
            .block_on(onecrawl_cdp::downloads::download_file(page, url))
            .map_err(py_err)?;
        serde_json::to_string(&result).map_err(py_err)
    }

    /// Wait for a download to complete.
    #[pyo3(signature = (timeout_ms=30000))]
    fn wait_for_download(&self, timeout_ms: u64) -> PyResult<Option<String>> {
        let guard = self.page.lock().map_err(py_err)?;
        let page = guard.as_ref().ok_or_else(|| py_err("browser closed"))?;
        let result = self
            .rt
            .block_on(onecrawl_cdp::downloads::wait_for_download(
                page, timeout_ms,
            ))
            .map_err(py_err)?;
        match result {
            Some(v) => Ok(Some(serde_json::to_string(&v).map_err(py_err)?)),
            None => Ok(None),
        }
    }

    /// Clear all downloads.
    fn clear_downloads(&self) -> PyResult<()> {
        let guard = self.page.lock().map_err(py_err)?;
        let page = guard.as_ref().ok_or_else(|| py_err("browser closed"))?;
        self.rt
            .block_on(onecrawl_cdp::downloads::clear_downloads(page))
            .map_err(py_err)
    }

    // ── Form Filler ───────────────────────────────────────────────────

    /// Detect forms on the page.
    fn detect_forms(&self) -> PyResult<String> {
        let guard = self.page.lock().map_err(py_err)?;
        let page = guard.as_ref().ok_or_else(|| py_err("browser closed"))?;
        let result = self
            .rt
            .block_on(onecrawl_cdp::form_filler::detect_forms(page))
            .map_err(py_err)?;
        serde_json::to_string(&result).map_err(py_err)
    }

    /// Fill a form with provided values.
    fn fill_form(&self, form_selector: &str, values_json: &str) -> PyResult<String> {
        let values: std::collections::HashMap<String, String> =
            serde_json::from_str(values_json).map_err(py_err)?;
        let guard = self.page.lock().map_err(py_err)?;
        let page = guard.as_ref().ok_or_else(|| py_err("browser closed"))?;
        let result = self
            .rt
            .block_on(onecrawl_cdp::form_filler::fill_form(
                page,
                form_selector,
                &values,
            ))
            .map_err(py_err)?;
        serde_json::to_string(&result).map_err(py_err)
    }

    /// Auto-fill a form using a profile.
    fn auto_fill_form(&self, form_selector: &str, profile_json: &str) -> PyResult<String> {
        let profile: std::collections::HashMap<String, String> =
            serde_json::from_str(profile_json).map_err(py_err)?;
        let guard = self.page.lock().map_err(py_err)?;
        let page = guard.as_ref().ok_or_else(|| py_err("browser closed"))?;
        let result = self
            .rt
            .block_on(onecrawl_cdp::form_filler::auto_fill(
                page,
                form_selector,
                &profile,
            ))
            .map_err(py_err)?;
        serde_json::to_string(&result).map_err(py_err)
    }

    /// Submit a form.
    fn submit_form(&self, form_selector: &str) -> PyResult<()> {
        let guard = self.page.lock().map_err(py_err)?;
        let page = guard.as_ref().ok_or_else(|| py_err("browser closed"))?;
        self.rt
            .block_on(onecrawl_cdp::form_filler::submit_form(page, form_selector))
            .map_err(py_err)
    }

    // ── Human-like Behavior ───────────────────────────────────────────

    /// Perform a human-like click on an element.
    fn human_click(&self, selector: &str) -> PyResult<()> {
        let guard = self.page.lock().map_err(py_err)?;
        let page = guard.as_ref().ok_or_else(|| py_err("browser closed"))?;
        self.rt
            .block_on(onecrawl_cdp::human::human_click(page, selector))
            .map_err(py_err)
    }

    /// Move mouse along a Bézier curve.
    fn mouse_move_bezier(&self, x0: f64, y0: f64, x1: f64, y1: f64) -> PyResult<()> {
        let guard = self.page.lock().map_err(py_err)?;
        let page = guard.as_ref().ok_or_else(|| py_err("browser closed"))?;
        self.rt
            .block_on(onecrawl_cdp::human::mouse_move_bezier(page, x0, y0, x1, y1))
            .map_err(py_err)
    }

    /// Perform a human-like scroll.
    fn human_scroll(&self, dx: i64, dy: i64) -> PyResult<()> {
        let guard = self.page.lock().map_err(py_err)?;
        let page = guard.as_ref().ok_or_else(|| py_err("browser closed"))?;
        self.rt
            .block_on(onecrawl_cdp::human::human_scroll(page, dx, dy))
            .map_err(py_err)
    }

    /// Apply a pre-action delay for human-like behavior.
    fn human_pre_delay(&self) -> PyResult<()> {
        self.rt.block_on(onecrawl_cdp::human::pre_action_delay());
        Ok(())
    }

    /// Apply a post-action delay for human-like behavior.
    fn human_post_delay(&self) -> PyResult<()> {
        self.rt.block_on(onecrawl_cdp::human::post_action_delay());
        Ok(())
    }

    /// Check if a Cloudflare challenge is present.
    fn is_cloudflare_challenge(&self) -> PyResult<bool> {
        let guard = self.page.lock().map_err(py_err)?;
        let page = guard.as_ref().ok_or_else(|| py_err("browser closed"))?;
        Ok(self
            .rt
            .block_on(onecrawl_cdp::human::is_cloudflare_challenge(page)))
    }

    /// Wait for Cloudflare clearance.
    #[pyo3(signature = (timeout_ms=30000))]
    fn wait_for_cf_clearance(&self, timeout_ms: u64) -> PyResult<bool> {
        let guard = self.page.lock().map_err(py_err)?;
        let page = guard.as_ref().ok_or_else(|| py_err("browser closed"))?;
        Ok(self
            .rt
            .block_on(onecrawl_cdp::human::wait_for_cf_clearance(
                page, timeout_ms,
            )))
    }

    // ── Smart Actions ─────────────────────────────────────────────────

    /// Find elements using natural language query.
    fn smart_find(&self, query: &str) -> PyResult<String> {
        let guard = self.page.lock().map_err(py_err)?;
        let page = guard.as_ref().ok_or_else(|| py_err("browser closed"))?;
        let result = self
            .rt
            .block_on(onecrawl_cdp::smart_actions::smart_find(page, query))
            .map_err(py_err)?;
        serde_json::to_string(&result).map_err(py_err)
    }

    /// Click an element using natural language query.
    fn smart_click(&self, query: &str) -> PyResult<String> {
        let guard = self.page.lock().map_err(py_err)?;
        let page = guard.as_ref().ok_or_else(|| py_err("browser closed"))?;
        let result = self
            .rt
            .block_on(onecrawl_cdp::smart_actions::smart_click(page, query))
            .map_err(py_err)?;
        serde_json::to_string(&result).map_err(py_err)
    }

    /// Fill an element using natural language query.
    fn smart_fill(&self, query: &str, value: &str) -> PyResult<String> {
        let guard = self.page.lock().map_err(py_err)?;
        let page = guard.as_ref().ok_or_else(|| py_err("browser closed"))?;
        let result = self
            .rt
            .block_on(onecrawl_cdp::smart_actions::smart_fill(page, query, value))
            .map_err(py_err)?;
        serde_json::to_string(&result).map_err(py_err)
    }

    // ── SPA Helpers ───────────────────────────────────────────────────

    /// Detect virtual scroll containers.
    fn detect_virtual_scroll(&self) -> PyResult<String> {
        let guard = self.page.lock().map_err(py_err)?;
        let page = guard.as_ref().ok_or_else(|| py_err("browser closed"))?;
        let result = self
            .rt
            .block_on(onecrawl_cdp::spa::detect_virtual_scroll(page))
            .map_err(py_err)?;
        serde_json::to_string(&result).map_err(py_err)
    }

    /// Extract items from a virtual scroll container.
    fn extract_virtual_scroll(
        &self,
        container_selector: &str,
        item_selector: &str,
        max_items: usize,
    ) -> PyResult<String> {
        let guard = self.page.lock().map_err(py_err)?;
        let page = guard.as_ref().ok_or_else(|| py_err("browser closed"))?;
        let result = self
            .rt
            .block_on(onecrawl_cdp::spa::extract_virtual_scroll(
                page,
                container_selector,
                item_selector,
                max_items,
            ))
            .map_err(py_err)?;
        serde_json::to_string(&result).map_err(py_err)
    }

    /// Wait for SPA hydration to complete.
    #[pyo3(signature = (timeout_ms=10000))]
    fn wait_hydration(&self, timeout_ms: u64) -> PyResult<String> {
        let guard = self.page.lock().map_err(py_err)?;
        let page = guard.as_ref().ok_or_else(|| py_err("browser closed"))?;
        let result = self
            .rt
            .block_on(onecrawl_cdp::spa::wait_hydration(page, timeout_ms))
            .map_err(py_err)?;
        serde_json::to_string(&result).map_err(py_err)
    }

    /// Wait for CSS animations/transitions to finish.
    #[pyo3(signature = (selector, timeout_ms=5000))]
    fn wait_animations(&self, selector: &str, timeout_ms: u64) -> PyResult<bool> {
        let guard = self.page.lock().map_err(py_err)?;
        let page = guard.as_ref().ok_or_else(|| py_err("browser closed"))?;
        self.rt
            .block_on(onecrawl_cdp::spa::wait_animations(
                page, selector, timeout_ms,
            ))
            .map_err(py_err)
    }

    /// Wait for network to become idle.
    #[pyo3(signature = (idle_ms=500, timeout_ms=30000))]
    fn wait_network_idle(&self, idle_ms: u64, timeout_ms: u64) -> PyResult<bool> {
        let guard = self.page.lock().map_err(py_err)?;
        let page = guard.as_ref().ok_or_else(|| py_err("browser closed"))?;
        self.rt
            .block_on(onecrawl_cdp::spa::wait_network_idle(
                page, idle_ms, timeout_ms,
            ))
            .map_err(py_err)
    }

    /// Trigger lazy loading for elements matching a selector.
    fn trigger_lazy_load(&self, selector: &str) -> PyResult<usize> {
        let guard = self.page.lock().map_err(py_err)?;
        let page = guard.as_ref().ok_or_else(|| py_err("browser closed"))?;
        self.rt
            .block_on(onecrawl_cdp::spa::trigger_lazy_load(page, selector))
            .map_err(py_err)
    }

    /// Inspect SPA state store.
    fn state_inspect(&self, store_path: Option<&str>) -> PyResult<String> {
        let guard = self.page.lock().map_err(py_err)?;
        let page = guard.as_ref().ok_or_else(|| py_err("browser closed"))?;
        let result = self
            .rt
            .block_on(onecrawl_cdp::spa::state_inspect(page, store_path))
            .map_err(py_err)?;
        serde_json::to_string(&result).map_err(py_err)
    }

    /// Track multi-step form wizard state.
    fn form_wizard_track(&self) -> PyResult<String> {
        let guard = self.page.lock().map_err(py_err)?;
        let page = guard.as_ref().ok_or_else(|| py_err("browser closed"))?;
        let result = self
            .rt
            .block_on(onecrawl_cdp::spa::form_wizard_track(page))
            .map_err(py_err)?;
        serde_json::to_string(&result).map_err(py_err)
    }

    /// Wait for a dynamic import to load.
    #[pyo3(signature = (module_pattern, timeout_ms=10000))]
    fn dynamic_import_wait(&self, module_pattern: &str, timeout_ms: u64) -> PyResult<String> {
        let guard = self.page.lock().map_err(py_err)?;
        let page = guard.as_ref().ok_or_else(|| py_err("browser closed"))?;
        let result = self
            .rt
            .block_on(onecrawl_cdp::spa::dynamic_import_wait(
                page,
                module_pattern,
                timeout_ms,
            ))
            .map_err(py_err)?;
        serde_json::to_string(&result).map_err(py_err)
    }

    /// Execute multiple actions in parallel.
    fn parallel_exec(&self, actions_json: &str) -> PyResult<String> {
        let actions: Vec<String> = serde_json::from_str(actions_json).map_err(py_err)?;
        let guard = self.page.lock().map_err(py_err)?;
        let page = guard.as_ref().ok_or_else(|| py_err("browser closed"))?;
        let result = self
            .rt
            .block_on(onecrawl_cdp::spa::parallel_exec(page, &actions))
            .map_err(py_err)?;
        serde_json::to_string(&result).map_err(py_err)
    }

    // ── Harness ───────────────────────────────────────────────────────

    /// Run a health check on the browser connection.
    fn health_check(&self) -> PyResult<String> {
        let guard = self.page.lock().map_err(py_err)?;
        let page = guard.as_ref().ok_or_else(|| py_err("browser closed"))?;
        let result = self
            .rt
            .block_on(onecrawl_cdp::harness::health_check(page))
            .map_err(py_err)?;
        serde_json::to_string(&result).map_err(py_err)
    }

    /// Reconnect CDP session.
    #[pyo3(signature = (max_retries=3))]
    fn reconnect_cdp(&self, max_retries: usize) -> PyResult<String> {
        let guard = self.page.lock().map_err(py_err)?;
        let page = guard.as_ref().ok_or_else(|| py_err("browser closed"))?;
        let result = self
            .rt
            .block_on(onecrawl_cdp::harness::reconnect_cdp(page, max_retries))
            .map_err(py_err)?;
        serde_json::to_string(&result).map_err(py_err)
    }

    /// Save a checkpoint.
    fn checkpoint_save(&self, path: &str, name: &str) -> PyResult<String> {
        let guard = self.page.lock().map_err(py_err)?;
        let page = guard.as_ref().ok_or_else(|| py_err("browser closed"))?;
        let result = self
            .rt
            .block_on(onecrawl_cdp::harness::checkpoint_save(page, path, name))
            .map_err(py_err)?;
        serde_json::to_string(&result).map_err(py_err)
    }

    /// Restore a checkpoint.
    fn checkpoint_restore(&self, path: &str, name: &str) -> PyResult<String> {
        let guard = self.page.lock().map_err(py_err)?;
        let page = guard.as_ref().ok_or_else(|| py_err("browser closed"))?;
        let result = self
            .rt
            .block_on(onecrawl_cdp::harness::checkpoint_restore(page, path, name))
            .map_err(py_err)?;
        serde_json::to_string(&result).map_err(py_err)
    }

    /// Get tab GC info.
    fn gc_tabs_info(&self) -> PyResult<String> {
        let guard = self.page.lock().map_err(py_err)?;
        let page = guard.as_ref().ok_or_else(|| py_err("browser closed"))?;
        let result = self
            .rt
            .block_on(onecrawl_cdp::harness::gc_tabs_info(page))
            .map_err(py_err)?;
        serde_json::to_string(&result).map_err(py_err)
    }

    /// Get watchdog status.
    fn watchdog_status(&self) -> PyResult<String> {
        let guard = self.page.lock().map_err(py_err)?;
        let page = guard.as_ref().ok_or_else(|| py_err("browser closed"))?;
        let result = self
            .rt
            .block_on(onecrawl_cdp::harness::watchdog_status(page))
            .map_err(py_err)?;
        serde_json::to_string(&result).map_err(py_err)
    }

    // ── Screencast ────────────────────────────────────────────────────

    /// Start a screencast session.
    #[pyo3(signature = (format=None, quality=None, max_width=None, max_height=None, every_nth_frame=None))]
    fn start_screencast(
        &self,
        format: Option<&str>,
        quality: Option<u32>,
        max_width: Option<u32>,
        max_height: Option<u32>,
        every_nth_frame: Option<u32>,
    ) -> PyResult<()> {
        let opts = onecrawl_cdp::screencast::ScreencastOptions {
            format: format.unwrap_or("png").to_string(),
            quality,
            max_width,
            max_height,
            every_nth_frame,
        };
        let guard = self.page.lock().map_err(py_err)?;
        let page = guard.as_ref().ok_or_else(|| py_err("browser closed"))?;
        self.rt
            .block_on(onecrawl_cdp::screencast::start_screencast(page, &opts))
            .map_err(py_err)
    }

    /// Stop the screencast session.
    fn stop_screencast(&self) -> PyResult<()> {
        let guard = self.page.lock().map_err(py_err)?;
        let page = guard.as_ref().ok_or_else(|| py_err("browser closed"))?;
        self.rt
            .block_on(onecrawl_cdp::screencast::stop_screencast(page))
            .map_err(py_err)
    }

    /// Capture a single frame.
    #[pyo3(signature = (format=None, quality=None))]
    fn capture_frame(&self, format: Option<&str>, quality: Option<u32>) -> PyResult<Vec<u8>> {
        let opts = onecrawl_cdp::screencast::ScreencastOptions {
            format: format.unwrap_or("png").to_string(),
            quality,
            max_width: None,
            max_height: None,
            every_nth_frame: Some(1),
        };
        let guard = self.page.lock().map_err(py_err)?;
        let page = guard.as_ref().ok_or_else(|| py_err("browser closed"))?;
        self.rt
            .block_on(onecrawl_cdp::screencast::capture_frame(page, &opts))
            .map_err(py_err)
    }

    /// Capture multiple frames in burst mode.
    #[pyo3(signature = (count=5, interval_ms=200))]
    fn capture_frames_burst(
        &self,
        count: usize,
        interval_ms: u64,
    ) -> PyResult<Vec<Vec<u8>>> {
        let opts = onecrawl_cdp::screencast::ScreencastOptions {
            format: "png".to_string(),
            quality: Some(80),
            max_width: None,
            max_height: None,
            every_nth_frame: Some(1),
        };
        let guard = self.page.lock().map_err(py_err)?;
        let page = guard.as_ref().ok_or_else(|| py_err("browser closed"))?;
        self.rt
            .block_on(onecrawl_cdp::screencast::capture_frames_burst(
                page,
                &opts,
                count,
                interval_ms,
            ))
            .map_err(py_err)
    }

    /// Stream captured frames to disk.
    #[pyo3(signature = (output_dir, count=10, interval_ms=200, format=None))]
    fn stream_to_disk(
        &self,
        output_dir: &str,
        count: usize,
        interval_ms: u64,
        format: Option<&str>,
    ) -> PyResult<String> {
        let opts = onecrawl_cdp::screencast::ScreencastOptions {
            format: format.unwrap_or("png").to_string(),
            quality: Some(80),
            max_width: None,
            max_height: None,
            every_nth_frame: Some(1),
        };
        let guard = self.page.lock().map_err(py_err)?;
        let page = guard.as_ref().ok_or_else(|| py_err("browser closed"))?;
        let result = self
            .rt
            .block_on(onecrawl_cdp::screencast::stream_to_disk(
                page,
                &opts,
                output_dir,
                count,
                interval_ms,
            ))
            .map_err(py_err)?;
        serde_json::to_string(&result).map_err(py_err)
    }

    // ── Recording ─────────────────────────────────────────────────────

    /// Encode frames into a video file.
    #[pyo3(signature = (frames_dir, output, fps=30, format="mp4"))]
    fn recording_encode(
        &self,
        frames_dir: &str,
        output: &str,
        fps: u32,
        format: &str,
    ) -> PyResult<String> {
        let result = onecrawl_cdp::recording::encode_video(frames_dir, output, fps, format)
            .map_err(py_err)?;
        serde_json::to_string(&result).map_err(py_err)
    }

    /// Save frames and encode into a video.
    #[pyo3(signature = (frames_json, output_dir, output, fps=30, format="mp4"))]
    fn recording_save_and_encode(
        &self,
        frames_json: &str,
        output_dir: &str,
        output: &str,
        fps: u32,
        format: &str,
    ) -> PyResult<String> {
        let frames: Vec<Vec<u8>> = serde_json::from_str(frames_json).map_err(py_err)?;
        let result = onecrawl_cdp::recording::save_and_encode(
            &frames, output_dir, output, fps, format,
        )
        .map_err(py_err)?;
        serde_json::to_string(&result).map_err(py_err)
    }

    // ── Agent ─────────────────────────────────────────────────────────

    /// Run an agent loop toward a goal.
    #[pyo3(signature = (goal, max_steps=10, verify_js=None))]
    fn agent_loop(
        &self,
        goal: &str,
        max_steps: usize,
        verify_js: Option<&str>,
    ) -> PyResult<String> {
        let guard = self.page.lock().map_err(py_err)?;
        let page = guard.as_ref().ok_or_else(|| py_err("browser closed"))?;
        let result = self
            .rt
            .block_on(onecrawl_cdp::agent::agent_loop(
                page, goal, max_steps, verify_js,
            ))
            .map_err(py_err)?;
        serde_json::to_string(&result).map_err(py_err)
    }

    /// Assert goals against the page.
    fn goal_assert(&self, assertions_json: &str) -> PyResult<String> {
        let pairs: Vec<(String, String)> =
            serde_json::from_str(assertions_json).map_err(py_err)?;
        let refs: Vec<(&str, &str)> = pairs.iter().map(|(a, b)| (a.as_str(), b.as_str())).collect();
        let guard = self.page.lock().map_err(py_err)?;
        let page = guard.as_ref().ok_or_else(|| py_err("browser closed"))?;
        let result = self
            .rt
            .block_on(onecrawl_cdp::agent::goal_assert(page, &refs))
            .map_err(py_err)?;
        serde_json::to_string(&result).map_err(py_err)
    }

    /// Get an annotated observation of the page.
    fn annotated_observe(&self) -> PyResult<String> {
        let guard = self.page.lock().map_err(py_err)?;
        let page = guard.as_ref().ok_or_else(|| py_err("browser closed"))?;
        let result = self
            .rt
            .block_on(onecrawl_cdp::agent::annotated_observe(page))
            .map_err(py_err)?;
        serde_json::to_string(&result).map_err(py_err)
    }

    /// Manage agent session context.
    #[pyo3(signature = (command, key=None, value=None))]
    fn session_context(
        &self,
        command: &str,
        key: Option<&str>,
        value: Option<&str>,
    ) -> PyResult<String> {
        let guard = self.page.lock().map_err(py_err)?;
        let page = guard.as_ref().ok_or_else(|| py_err("browser closed"))?;
        let result = self
            .rt
            .block_on(onecrawl_cdp::agent::session_context(
                page, command, key, value,
            ))
            .map_err(py_err)?;
        serde_json::to_string(&result).map_err(py_err)
    }

    /// Chain actions with error recovery.
    #[pyo3(signature = (actions_json, on_error="retry", max_retries=3))]
    fn auto_chain(
        &self,
        actions_json: &str,
        on_error: &str,
        max_retries: usize,
    ) -> PyResult<String> {
        let actions: Vec<String> = serde_json::from_str(actions_json).map_err(py_err)?;
        let guard = self.page.lock().map_err(py_err)?;
        let page = guard.as_ref().ok_or_else(|| py_err("browser closed"))?;
        let result = self
            .rt
            .block_on(onecrawl_cdp::agent::auto_chain(
                page,
                &actions,
                on_error,
                max_retries,
            ))
            .map_err(py_err)?;
        serde_json::to_string(&result).map_err(py_err)
    }

    /// Agent think step — observe and plan.
    fn think(&self) -> PyResult<String> {
        let guard = self.page.lock().map_err(py_err)?;
        let page = guard.as_ref().ok_or_else(|| py_err("browser closed"))?;
        let result = self
            .rt
            .block_on(onecrawl_cdp::agent::think(page))
            .map_err(py_err)?;
        serde_json::to_string(&result).map_err(py_err)
    }

    /// Click at specific coordinates.
    fn click_at_coords(&self, x: f64, y: f64) -> PyResult<String> {
        let guard = self.page.lock().map_err(py_err)?;
        let page = guard.as_ref().ok_or_else(|| py_err("browser closed"))?;
        let result = self
            .rt
            .block_on(onecrawl_cdp::agent::click_at_coords(page, x, y))
            .map_err(py_err)?;
        serde_json::to_string(&result).map_err(py_err)
    }

    /// Replay input events.
    fn input_replay(&self, events_json: &str) -> PyResult<String> {
        let events: Vec<serde_json::Value> =
            serde_json::from_str(events_json).map_err(py_err)?;
        let guard = self.page.lock().map_err(py_err)?;
        let page = guard.as_ref().ok_or_else(|| py_err("browser closed"))?;
        let result = self
            .rt
            .block_on(onecrawl_cdp::agent::input_replay(page, &events))
            .map_err(py_err)?;
        serde_json::to_string(&result).map_err(py_err)
    }

    // ── Annotated Screenshots ─────────────────────────────────────────

    /// Take an annotated screenshot of the page.
    fn annotated_screenshot(&self) -> PyResult<String> {
        let guard = self.page.lock().map_err(py_err)?;
        let page = guard.as_ref().ok_or_else(|| py_err("browser closed"))?;
        let result = self
            .rt
            .block_on(onecrawl_cdp::annotated::annotated_screenshot(page))
            .map_err(py_err)?;
        serde_json::to_string(&result).map_err(py_err)
    }

    /// Retry an action with adaptive strategies.
    #[pyo3(signature = (action_js, max_retries=3, strategies_json=None))]
    fn adaptive_retry(
        &self,
        action_js: &str,
        max_retries: usize,
        strategies_json: Option<&str>,
    ) -> PyResult<String> {
        let strategies: Vec<String> = match strategies_json {
            Some(s) => serde_json::from_str(s).map_err(py_err)?,
            None => vec!["wait".into(), "scroll".into(), "reload".into()],
        };
        let guard = self.page.lock().map_err(py_err)?;
        let page = guard.as_ref().ok_or_else(|| py_err("browser closed"))?;
        let result = self
            .rt
            .block_on(onecrawl_cdp::annotated::adaptive_retry(
                page,
                action_js,
                max_retries,
                &strategies,
            ))
            .map_err(py_err)?;
        serde_json::to_string(&result).map_err(py_err)
    }

    // ── Computer Use ──────────────────────────────────────────────────

    /// Observe page for computer-use style interaction.
    #[pyo3(signature = (last_error=None, include_screenshot=true))]
    fn computer_use_observe(
        &self,
        last_error: Option<&str>,
        include_screenshot: bool,
    ) -> PyResult<String> {
        let guard = self.page.lock().map_err(py_err)?;
        let page = guard.as_ref().ok_or_else(|| py_err("browser closed"))?;
        let result = self
            .rt
            .block_on(onecrawl_cdp::computer_use::observe(
                page,
                last_error.map(String::from),
                include_screenshot,
            ))
            .map_err(py_err)?;
        serde_json::to_string(&result).map_err(py_err)
    }

    // ── Pixel Diff ────────────────────────────────────────────────────

    /// Compare two base64-encoded images pixel by pixel.
    #[pyo3(signature = (image_a_b64, image_b_b64, threshold=0.01, generate_diff_image=false))]
    fn pixel_diff(
        &self,
        image_a_b64: &str,
        image_b_b64: &str,
        threshold: f64,
        generate_diff_image: bool,
    ) -> PyResult<String> {
        let guard = self.page.lock().map_err(py_err)?;
        let page = guard.as_ref().ok_or_else(|| py_err("browser closed"))?;
        let result = self
            .rt
            .block_on(onecrawl_cdp::pixel_diff::pixel_diff(
                page,
                image_a_b64,
                image_b_b64,
                threshold,
                generate_diff_image,
            ))
            .map_err(py_err)?;
        serde_json::to_string(&result).map_err(py_err)
    }

    /// Compare two image files pixel by pixel.
    #[pyo3(signature = (path_a, path_b, threshold=0.01))]
    fn pixel_diff_files(
        &self,
        path_a: &str,
        path_b: &str,
        threshold: f64,
    ) -> PyResult<String> {
        let guard = self.page.lock().map_err(py_err)?;
        let page = guard.as_ref().ok_or_else(|| py_err("browser closed"))?;
        let result = self
            .rt
            .block_on(onecrawl_cdp::pixel_diff::pixel_diff_files(
                page, path_a, path_b, threshold,
            ))
            .map_err(py_err)?;
        serde_json::to_string(&result).map_err(py_err)
    }

    // ── Screenshot Diff ───────────────────────────────────────────────

    /// Compare two screenshot byte arrays.
    fn compare_screenshots_bytes(&self, baseline: Vec<u8>, current: Vec<u8>) -> PyResult<String> {
        let result =
            onecrawl_cdp::screenshot_diff::compare_screenshots(&baseline, &current)
                .map_err(py_err)?;
        serde_json::to_string(&result).map_err(py_err)
    }

    /// Run visual regression against a baseline file.
    fn visual_regression(&self, baseline_path: &str) -> PyResult<String> {
        let guard = self.page.lock().map_err(py_err)?;
        let page = guard.as_ref().ok_or_else(|| py_err("browser closed"))?;
        let result = self
            .rt
            .block_on(onecrawl_cdp::screenshot_diff::visual_regression(
                page,
                std::path::Path::new(baseline_path),
            ))
            .map_err(py_err)?;
        serde_json::to_string(&result).map_err(py_err)
    }

    // ── Snapshot Diff ─────────────────────────────────────────────────

    /// Diff two text snapshots.
    fn diff_snapshots(&self, before: &str, after: &str) -> PyResult<String> {
        let result =
            onecrawl_cdp::snapshot_diff::diff_snapshots(before, after);
        serde_json::to_string(&result).map_err(py_err)
    }

    // ── Workflow Engine ───────────────────────────────────────────────

    /// Validate a workflow definition (JSON or YAML).
    fn workflow_validate(&self, workflow: &str) -> PyResult<String> {
        let wf = if workflow.trim_start().starts_with('{') {
            onecrawl_cdp::workflow::parse_json(workflow).map_err(py_err)?
        } else {
            onecrawl_cdp::workflow::parse_json_compat(workflow).map_err(py_err)?
        };
        let errors = onecrawl_cdp::workflow::validate(&wf);
        serde_json::to_string(&errors).map_err(py_err)
    }

    /// Execute a workflow definition (JSON or YAML).
    fn workflow_execute(&self, workflow: &str) -> PyResult<String> {
        let wf = if workflow.trim_start().starts_with('{') {
            onecrawl_cdp::workflow::parse_json(workflow).map_err(py_err)?
        } else {
            onecrawl_cdp::workflow::parse_json_compat(workflow).map_err(py_err)?
        };
        let guard = self.page.lock().map_err(py_err)?;
        let page = guard.as_ref().ok_or_else(|| py_err("browser closed"))?;
        let result = self
            .rt
            .block_on(onecrawl_cdp::workflow::execute_workflow(page, &wf))
            .map_err(py_err)?;
        serde_json::to_string(&result).map_err(py_err)
    }

    /// Execute a workflow from a file.
    fn workflow_execute_file(&self, path: &str) -> PyResult<String> {
        let wf = onecrawl_cdp::workflow::load_from_file(path).map_err(py_err)?;
        let guard = self.page.lock().map_err(py_err)?;
        let page = guard.as_ref().ok_or_else(|| py_err("browser closed"))?;
        let result = self
            .rt
            .block_on(onecrawl_cdp::workflow::execute_workflow(page, &wf))
            .map_err(py_err)?;
        serde_json::to_string(&result).map_err(py_err)
    }

    // ── Task Planner ──────────────────────────────────────────────────

    /// Plan tasks from a goal description.
    #[pyo3(signature = (goal, context_json=None))]
    fn task_plan(&self, goal: &str, context_json: Option<&str>) -> PyResult<String> {
        let context: std::collections::HashMap<String, String> = match context_json {
            Some(c) => serde_json::from_str(c).map_err(py_err)?,
            None => std::collections::HashMap::new(),
        };
        let result =
            onecrawl_cdp::task_planner::plan_from_goal(goal, &context);
        serde_json::to_string(&result).map_err(py_err)
    }

    /// List built-in task patterns.
    fn task_patterns(&self) -> PyResult<String> {
        let result = onecrawl_cdp::task_planner::builtin_patterns();
        serde_json::to_string(&result).map_err(py_err)
    }

    /// Match a goal to a task category.
    fn task_match_goal(&self, goal: &str) -> PyResult<String> {
        let (cat, steps) = onecrawl_cdp::task_planner::match_goal(goal);
        let result = serde_json::json!({
            "category": format!("{:?}", cat),
            "steps": steps,
        });
        serde_json::to_string(&result).map_err(py_err)
    }

    // ── Network Intelligence ──────────────────────────────────────────

    /// Classify a network request.
    fn net_classify_request(
        &self,
        url: &str,
        content_type: Option<&str>,
        method: &str,
    ) -> PyResult<String> {
        let cat = onecrawl_cdp::network_intel::classify_request(url, content_type, method);
        Ok(format!("{:?}", cat))
    }

    /// Infer a JSON schema from a JSON value.
    fn net_infer_schema(&self, json_value: &str) -> PyResult<String> {
        let value: serde_json::Value = serde_json::from_str(json_value).map_err(py_err)?;
        let schema = onecrawl_cdp::network_intel::infer_json_schema(&value);
        serde_json::to_string(&schema).map_err(py_err)
    }

    // ── Performance Monitor ───────────────────────────────────────────

    /// Get the performance metrics collection JavaScript.
    fn perf_metrics_js(&self) -> String {
        onecrawl_cdp::perf_monitor::metrics_collection_js().to_string()
    }

    // ── VRT ───────────────────────────────────────────────────────────

    /// Compare two images and return similarity score.
    fn vrt_compare_images(&self, baseline: Vec<u8>, current: Vec<u8>) -> f64 {
        onecrawl_cdp::vrt::compare_images(&baseline, &current)
    }

    /// Load a VRT test suite from a file.
    fn vrt_load_suite(&self, path: &str) -> PyResult<String> {
        let suite = onecrawl_cdp::vrt::load_suite(path).map_err(py_err)?;
        serde_json::to_string(&suite).map_err(py_err)
    }

    /// Save a baseline image.
    fn vrt_save_baseline(&self, dir: &str, test_name: &str, data: Vec<u8>) -> PyResult<String> {
        let path =
            onecrawl_cdp::vrt::save_baseline(dir, test_name, &data).map_err(py_err)?;
        Ok(path.to_string_lossy().to_string())
    }

    /// Load a baseline image.
    fn vrt_load_baseline(&self, dir: &str, test_name: &str) -> Option<Vec<u8>> {
        onecrawl_cdp::vrt::load_baseline(dir, test_name)
    }

    /// Validate a VRT suite definition.
    fn vrt_validate_suite(&self, suite_json: &str) -> PyResult<String> {
        let suite: onecrawl_cdp::vrt::VrtSuite =
            serde_json::from_str(suite_json).map_err(py_err)?;
        let errors = onecrawl_cdp::vrt::validate_suite(&suite);
        serde_json::to_string(&errors).map_err(py_err)
    }

    // ── Sitemap ───────────────────────────────────────────────────────

    /// Parse a sitemap XML string.
    fn sitemap_parse(&self, xml: &str) -> PyResult<String> {
        let result = onecrawl_cdp::sitemap::parse_sitemap(xml).map_err(py_err)?;
        serde_json::to_string(&result).map_err(py_err)
    }

    /// Generate a sitemap XML string.
    fn sitemap_generate(&self, entries_json: &str, config_json: &str) -> PyResult<String> {
        let entries: Vec<onecrawl_cdp::sitemap::SitemapEntry> =
            serde_json::from_str(entries_json).map_err(py_err)?;
        let config: onecrawl_cdp::sitemap::SitemapConfig =
            serde_json::from_str(config_json).map_err(py_err)?;
        Ok(onecrawl_cdp::sitemap::generate_sitemap(&entries, &config))
    }

    /// Save a sitemap to a file.
    fn sitemap_save(
        &self,
        entries_json: &str,
        config_json: &str,
        path: &str,
    ) -> PyResult<usize> {
        let entries: Vec<onecrawl_cdp::sitemap::SitemapEntry> =
            serde_json::from_str(entries_json).map_err(py_err)?;
        let config: onecrawl_cdp::sitemap::SitemapConfig =
            serde_json::from_str(config_json).map_err(py_err)?;
        onecrawl_cdp::sitemap::save_sitemap(&entries, &config, std::path::Path::new(path))
            .map_err(py_err)
    }

    /// Generate a sitemap from crawl results.
    fn sitemap_from_crawl(
        &self,
        results_json: &str,
        config_json: &str,
    ) -> PyResult<String> {
        let results: Vec<onecrawl_cdp::spider::CrawlResult> =
            serde_json::from_str(results_json).map_err(py_err)?;
        let config: onecrawl_cdp::sitemap::SitemapConfig =
            serde_json::from_str(config_json).map_err(py_err)?;
        let entries = onecrawl_cdp::sitemap::from_crawl_results(&results, &config);
        serde_json::to_string(&entries).map_err(py_err)
    }

    /// Generate a sitemap index from URLs.
    fn sitemap_index(&self, urls_json: &str) -> PyResult<String> {
        let urls: Vec<String> = serde_json::from_str(urls_json).map_err(py_err)?;
        Ok(onecrawl_cdp::sitemap::generate_sitemap_index(&urls))
    }

    // ── Skills ────────────────────────────────────────────────────────

    /// List built-in skills.
    fn skills_list_builtins(&self) -> PyResult<String> {
        let result = onecrawl_cdp::skills::SkillRegistry::builtins();
        serde_json::to_string(&result).map_err(py_err)
    }

    // ── Stealth Extras ────────────────────────────────────────────────

    /// Inject persistent stealth patches.
    fn inject_persistent_stealth(&self, real_ua: Option<&str>) -> PyResult<()> {
        let guard = self.page.lock().map_err(py_err)?;
        let page = guard.as_ref().ok_or_else(|| py_err("browser closed"))?;
        self.rt
            .block_on(onecrawl_cdp::stealth::inject_persistent_stealth(
                page, real_ua,
            ))
            .map_err(py_err)
    }
}

// ──────────────────────────── IosClient ────────────────────────────

#[pyclass]
struct IosClient {
    rt: Arc<tokio::runtime::Runtime>,
    inner: Arc<std::sync::Mutex<onecrawl_cdp::ios::IosClient>>,
}

#[pymethods]
impl IosClient {
    /// Connect to an iOS device via WebDriverAgent.
    #[staticmethod]
    fn connect(wda_url: &str, bundle_id: &str) -> PyResult<Self> {
        let rt = tokio::runtime::Runtime::new().map_err(py_err)?;
        let config = onecrawl_cdp::ios::IosSessionConfig {
            wda_url: wda_url.to_string(),
            device_udid: None,
            bundle_id: bundle_id.to_string(),
        };
        let mut client = onecrawl_cdp::ios::IosClient::new(config);
        rt.block_on(client.create_session()).map_err(py_err)?;
        Ok(Self {
            rt: Arc::new(rt),
            inner: Arc::new(std::sync::Mutex::new(client)),
        })
    }

    /// List available iOS devices.
    #[staticmethod]
    fn list_devices() -> PyResult<String> {
        let devices = onecrawl_cdp::ios::IosClient::list_devices()
            .map_err(py_err)?;
        serde_json::to_string(&devices).map_err(py_err)
    }

    /// Navigate to a URL.
    fn navigate(&self, url: &str) -> PyResult<()> {
        let client = self.inner.lock().map_err(py_err)?;
        self.rt
            .block_on(client.navigate(url))
            .map_err(py_err)
    }

    /// Tap at coordinates.
    fn tap(&self, x: f64, y: f64) -> PyResult<()> {
        let client = self.inner.lock().map_err(py_err)?;
        self.rt.block_on(client.tap(x, y)).map_err(py_err)
    }

    /// Take a screenshot.
    fn screenshot(&self) -> PyResult<Vec<u8>> {
        let client = self.inner.lock().map_err(py_err)?;
        self.rt
            .block_on(client.screenshot())
            .map_err(py_err)
    }

    /// Swipe from one point to another.
    fn swipe(&self, x0: f64, y0: f64, x1: f64, y1: f64, duration: f64) -> PyResult<()> {
        let client = self.inner.lock().map_err(py_err)?;
        self.rt
            .block_on(client.swipe(x0, y0, x1, y1, duration))
            .map_err(py_err)
    }

    /// Pinch gesture.
    fn pinch(&self, x: f64, y: f64, scale: f64, velocity: f64) -> PyResult<()> {
        let client = self.inner.lock().map_err(py_err)?;
        self.rt
            .block_on(client.pinch(x, y, scale, velocity))
            .map_err(py_err)
    }

    /// Long press at coordinates.
    fn long_press(&self, x: f64, y: f64, duration_ms: u64) -> PyResult<()> {
        let client = self.inner.lock().map_err(py_err)?;
        self.rt
            .block_on(client.long_press(x, y, duration_ms))
            .map_err(py_err)
    }

    /// Double tap at coordinates.
    fn double_tap(&self, x: f64, y: f64) -> PyResult<()> {
        let client = self.inner.lock().map_err(py_err)?;
        self.rt
            .block_on(client.double_tap(x, y))
            .map_err(py_err)
    }

    /// Set device orientation.
    fn set_orientation(&self, orientation: &str) -> PyResult<()> {
        let client = self.inner.lock().map_err(py_err)?;
        self.rt
            .block_on(client.set_orientation(orientation))
            .map_err(py_err)
    }

    /// Get device orientation.
    fn get_orientation(&self) -> PyResult<String> {
        let client = self.inner.lock().map_err(py_err)?;
        self.rt
            .block_on(client.get_orientation())
            .map_err(py_err)
    }

    /// Scroll to an element.
    fn scroll_to_element(&self, using: &str, value: &str) -> PyResult<()> {
        let client = self.inner.lock().map_err(py_err)?;
        self.rt
            .block_on(client.scroll_to_element(using, value))
            .map_err(py_err)
    }

    /// Execute a script.
    fn execute_script(&self, script: &str, args_json: Option<&str>) -> PyResult<String> {
        let client = self.inner.lock().map_err(py_err)?;
        let args: Vec<serde_json::Value> = match args_json {
            Some(a) => serde_json::from_str(a).map_err(py_err)?,
            None => Vec::new(),
        };
        let result = self
            .rt
            .block_on(client.execute_script(script, &args))
            .map_err(py_err)?;
        serde_json::to_string(&result).map_err(py_err)
    }

    /// Get cookies.
    fn get_cookies(&self) -> PyResult<String> {
        let client = self.inner.lock().map_err(py_err)?;
        let result = self
            .rt
            .block_on(client.get_cookies())
            .map_err(py_err)?;
        serde_json::to_string(&result).map_err(py_err)
    }

    /// Launch an app by bundle ID.
    fn launch_app(&self, bundle_id: &str) -> PyResult<()> {
        let client = self.inner.lock().map_err(py_err)?;
        self.rt
            .block_on(client.launch_app(bundle_id))
            .map_err(py_err)
    }

    /// Kill a running app.
    fn kill_app(&self, bundle_id: &str) -> PyResult<()> {
        let client = self.inner.lock().map_err(py_err)?;
        self.rt
            .block_on(client.terminate_app(bundle_id))
            .map_err(py_err)
    }

    /// Get app state.
    fn app_state(&self, bundle_id: &str) -> PyResult<String> {
        let client = self.inner.lock().map_err(py_err)?;
        let result = self
            .rt
            .block_on(client.app_state(bundle_id))
            .map_err(py_err)?;
        serde_json::to_string(&result).map_err(py_err)
    }

    /// Lock the device.
    fn lock_device(&self) -> PyResult<()> {
        let client = self.inner.lock().map_err(py_err)?;
        self.rt
            .block_on(client.lock_device())
            .map_err(py_err)
    }

    /// Unlock the device.
    fn unlock_device(&self) -> PyResult<()> {
        let client = self.inner.lock().map_err(py_err)?;
        self.rt
            .block_on(client.unlock_device())
            .map_err(py_err)
    }

    /// Press the home button.
    fn home_button(&self) -> PyResult<()> {
        let client = self.inner.lock().map_err(py_err)?;
        self.rt
            .block_on(client.home_button())
            .map_err(py_err)
    }

    /// Press a named button.
    fn press_button(&self, button: &str) -> PyResult<()> {
        let client = self.inner.lock().map_err(py_err)?;
        self.rt
            .block_on(client.press_button(button))
            .map_err(py_err)
    }

    /// Get battery info.
    fn battery_info(&self) -> PyResult<String> {
        let client = self.inner.lock().map_err(py_err)?;
        let result = self
            .rt
            .block_on(client.battery_info())
            .map_err(py_err)?;
        serde_json::to_string(&result).map_err(py_err)
    }

    /// Get device info.
    fn device_info(&self) -> PyResult<String> {
        let client = self.inner.lock().map_err(py_err)?;
        let result = self
            .rt
            .block_on(client.device_info())
            .map_err(py_err)?;
        serde_json::to_string(&result).map_err(py_err)
    }

    /// Get the current URL.
    fn get_url(&self) -> PyResult<String> {
        let client = self.inner.lock().map_err(py_err)?;
        self.rt
            .block_on(client.get_url())
            .map_err(py_err)
    }

    /// Get the page title.
    fn get_title(&self) -> PyResult<String> {
        let client = self.inner.lock().map_err(py_err)?;
        self.rt
            .block_on(client.get_title())
            .map_err(py_err)
    }

    /// Get screen size.
    fn get_screen_size(&self) -> PyResult<String> {
        let client = self.inner.lock().map_err(py_err)?;
        let result = self
            .rt
            .block_on(client.get_screen_size())
            .map_err(py_err)?;
        serde_json::to_string(&result).map_err(py_err)
    }

    /// Find an element by selector.
    fn find_element(&self, using: &str, value: &str) -> PyResult<String> {
        let client = self.inner.lock().map_err(py_err)?;
        let result = self
            .rt
            .block_on(client.find_element(using, value))
            .map_err(py_err)?;
        serde_json::to_string(&result).map_err(py_err)
    }

    /// Click an element by ID.
    fn click_element(&self, element_id: &str) -> PyResult<()> {
        let client = self.inner.lock().map_err(py_err)?;
        self.rt
            .block_on(client.click_element(element_id))
            .map_err(py_err)
    }

    /// Type text into an element.
    fn type_text(&self, element_id: &str, text: &str) -> PyResult<()> {
        let client = self.inner.lock().map_err(py_err)?;
        self.rt
            .block_on(client.type_text(element_id, text))
            .map_err(py_err)
    }

    /// Get the page source.
    fn page_source(&self) -> PyResult<String> {
        let client = self.inner.lock().map_err(py_err)?;
        self.rt
            .block_on(client.page_source())
            .map_err(py_err)
    }

    /// Perform a simulator action.
    #[staticmethod]
    fn simulator_action(
        action: &str,
        udid: Option<&str>,
        device_type: Option<&str>,
        runtime: Option<&str>,
    ) -> PyResult<String> {
        let rt = tokio::runtime::Runtime::new().map_err(py_err)?;
        let result = rt
            .block_on(onecrawl_cdp::ios::IosClient::simulator_action(
                action,
                udid,
                device_type,
                runtime,
            ))
            .map_err(py_err)?;
        serde_json::to_string(&result).map_err(py_err)
    }

    /// Close the client session.
    fn close(&self) -> PyResult<()> {
        let mut client = self.inner.lock().map_err(py_err)?;
        self.rt.block_on(client.close_session()).map_err(py_err)
    }
}

// ──────────────────────────── AndroidClient ────────────────────────────

#[pyclass]
struct AndroidClient {
    rt: Arc<tokio::runtime::Runtime>,
    inner: Arc<std::sync::Mutex<onecrawl_cdp::android::AndroidClient>>,
}

#[pymethods]
impl AndroidClient {
    /// Connect to an Android device.
    #[staticmethod]
    #[pyo3(signature = (server_url, package, device_serial=None, activity=None))]
    fn connect(
        server_url: &str,
        package: &str,
        device_serial: Option<&str>,
        activity: Option<&str>,
    ) -> PyResult<Self> {
        let rt = tokio::runtime::Runtime::new().map_err(py_err)?;
        let config = onecrawl_cdp::android::AndroidSessionConfig {
            server_url: server_url.to_string(),
            device_serial: device_serial.map(String::from),
            package: package.to_string(),
            activity: activity.map(String::from),
        };
        let mut client = onecrawl_cdp::android::AndroidClient::new(config);
        rt.block_on(client.create_session(None, activity))
            .map_err(py_err)?;
        Ok(Self {
            rt: Arc::new(rt),
            inner: Arc::new(std::sync::Mutex::new(client)),
        })
    }

    /// List available Android devices.
    #[staticmethod]
    fn list_devices() -> PyResult<String> {
        let rt = tokio::runtime::Runtime::new().map_err(py_err)?;
        let devices = rt
            .block_on(onecrawl_cdp::android::AndroidClient::list_devices())
            .map_err(py_err)?;
        serde_json::to_string(&devices).map_err(py_err)
    }

    /// Navigate to a URL.
    fn navigate(&self, url: &str) -> PyResult<()> {
        let client = self.inner.lock().map_err(py_err)?;
        self.rt
            .block_on(client.navigate(url))
            .map_err(py_err)
    }

    /// Get the current URL.
    fn get_url(&self) -> PyResult<String> {
        let client = self.inner.lock().map_err(py_err)?;
        self.rt
            .block_on(client.get_url())
            .map_err(py_err)
    }

    /// Get the page title.
    fn get_title(&self) -> PyResult<String> {
        let client = self.inner.lock().map_err(py_err)?;
        self.rt
            .block_on(client.get_title())
            .map_err(py_err)
    }

    /// Go back.
    fn back(&self) -> PyResult<()> {
        let client = self.inner.lock().map_err(py_err)?;
        self.rt.block_on(client.back()).map_err(py_err)
    }

    /// Tap at coordinates.
    fn tap(&self, x: f64, y: f64) -> PyResult<()> {
        let client = self.inner.lock().map_err(py_err)?;
        self.rt.block_on(client.tap(x, y)).map_err(py_err)
    }

    /// Swipe from one point to another.
    fn swipe(&self, x0: f64, y0: f64, x1: f64, y1: f64, duration_ms: u64) -> PyResult<()> {
        let client = self.inner.lock().map_err(py_err)?;
        self.rt
            .block_on(client.swipe(x0, y0, x1, y1, duration_ms))
            .map_err(py_err)
    }

    /// Long press at coordinates.
    fn long_press(&self, x: f64, y: f64, duration_ms: u64) -> PyResult<()> {
        let client = self.inner.lock().map_err(py_err)?;
        self.rt
            .block_on(client.long_press(x, y, duration_ms))
            .map_err(py_err)
    }

    /// Double tap at coordinates.
    fn double_tap(&self, x: f64, y: f64) -> PyResult<()> {
        let client = self.inner.lock().map_err(py_err)?;
        self.rt
            .block_on(client.double_tap(x, y))
            .map_err(py_err)
    }

    /// Pinch gesture.
    fn pinch(&self, x: f64, y: f64, scale: f64) -> PyResult<()> {
        let client = self.inner.lock().map_err(py_err)?;
        self.rt
            .block_on(client.pinch(x, y, scale))
            .map_err(py_err)
    }

    /// Type text.
    fn type_text(&self, text: &str) -> PyResult<()> {
        let client = self.inner.lock().map_err(py_err)?;
        self.rt
            .block_on(client.type_text(text))
            .map_err(py_err)
    }

    /// Find an element by strategy and value.
    fn find_element(&self, strategy: &str, value: &str) -> PyResult<String> {
        let client = self.inner.lock().map_err(py_err)?;
        let result = self
            .rt
            .block_on(client.find_element(strategy, value))
            .map_err(py_err)?;
        serde_json::to_string(&result).map_err(py_err)
    }

    /// Click an element by ID.
    fn click_element(&self, element_id: &str) -> PyResult<()> {
        let client = self.inner.lock().map_err(py_err)?;
        self.rt
            .block_on(client.click_element(element_id))
            .map_err(py_err)
    }

    /// Take a screenshot (returns base64 PNG).
    fn screenshot(&self) -> PyResult<String> {
        let client = self.inner.lock().map_err(py_err)?;
        self.rt
            .block_on(client.screenshot())
            .map_err(py_err)
    }

    /// Set device orientation.
    fn set_orientation(&self, orientation: &str) -> PyResult<()> {
        let client = self.inner.lock().map_err(py_err)?;
        self.rt
            .block_on(client.set_orientation(orientation))
            .map_err(py_err)
    }

    /// Get device orientation.
    fn get_orientation(&self) -> PyResult<String> {
        let client = self.inner.lock().map_err(py_err)?;
        self.rt
            .block_on(client.get_orientation())
            .map_err(py_err)
    }

    /// Press a hardware key.
    fn press_key(&self, keycode: i32) -> PyResult<()> {
        let client = self.inner.lock().map_err(py_err)?;
        self.rt
            .block_on(client.press_key(keycode))
            .map_err(py_err)
    }

    /// Launch an app.
    fn launch_app(&self, package: &str, activity: Option<&str>) -> PyResult<()> {
        let client = self.inner.lock().map_err(py_err)?;
        self.rt
            .block_on(client.launch_app(package, activity))
            .map_err(py_err)
    }

    /// Kill a running app.
    fn kill_app(&self, package: &str) -> PyResult<()> {
        let client = self.inner.lock().map_err(py_err)?;
        self.rt
            .block_on(client.terminate_app(package))
            .map_err(py_err)
    }

    /// Get app state.
    fn app_state(&self, package: &str) -> PyResult<String> {
        let client = self.inner.lock().map_err(py_err)?;
        let result = self
            .rt
            .block_on(client.app_state(package))
            .map_err(py_err)?;
        serde_json::to_string(&result).map_err(py_err)
    }

    /// Install an APK.
    fn install_app(&self, apk_path: &str) -> PyResult<()> {
        let client = self.inner.lock().map_err(py_err)?;
        self.rt
            .block_on(client.install_app(apk_path))
            .map_err(py_err)
    }

    /// Execute a script.
    fn execute_script(&self, script: &str, args_json: Option<&str>) -> PyResult<String> {
        let client = self.inner.lock().map_err(py_err)?;
        let args: Vec<serde_json::Value> = match args_json {
            Some(a) => serde_json::from_str(a).map_err(py_err)?,
            None => Vec::new(),
        };
        let result = self
            .rt
            .block_on(client.execute_script(script, &args))
            .map_err(py_err)?;
        serde_json::to_string(&result).map_err(py_err)
    }

    /// Run an ADB shell command.
    #[staticmethod]
    fn shell(serial: &str, command: &str) -> PyResult<String> {
        let rt = tokio::runtime::Runtime::new().map_err(py_err)?;
        rt.block_on(onecrawl_cdp::android::AndroidClient::shell(serial, command))
            .map_err(py_err)
    }

    /// Push a file to the device.
    #[staticmethod]
    fn push_file(serial: &str, local: &str, remote: &str) -> PyResult<()> {
        let rt = tokio::runtime::Runtime::new().map_err(py_err)?;
        rt.block_on(onecrawl_cdp::android::AndroidClient::push_file(serial, local, remote))
            .map_err(py_err)
    }

    /// Pull a file from the device.
    #[staticmethod]
    fn pull_file(serial: &str, remote: &str, local: &str) -> PyResult<()> {
        let rt = tokio::runtime::Runtime::new().map_err(py_err)?;
        rt.block_on(onecrawl_cdp::android::AndroidClient::pull_file(serial, remote, local))
            .map_err(py_err)
    }

    /// Get device info.
    #[staticmethod]
    fn device_info(serial: &str) -> PyResult<String> {
        let rt = tokio::runtime::Runtime::new().map_err(py_err)?;
        let result = rt
            .block_on(onecrawl_cdp::android::AndroidClient::device_info(serial))
            .map_err(py_err)?;
        serde_json::to_string(&result).map_err(py_err)
    }

    /// Get battery info.
    #[staticmethod]
    fn battery_info(serial: &str) -> PyResult<String> {
        let rt = tokio::runtime::Runtime::new().map_err(py_err)?;
        let result = rt
            .block_on(onecrawl_cdp::android::AndroidClient::battery_info(serial))
            .map_err(py_err)?;
        serde_json::to_string(&result).map_err(py_err)
    }

    /// Close the client session.
    fn close(&self) -> PyResult<()> {
        let mut client = self.inner.lock().map_err(py_err)?;
        self.rt.block_on(client.close_session()).map_err(py_err)
    }
}

// ──────────────────────────── AgentMemory ────────────────────────────

#[pyclass]
struct AgentMemory {
    inner: std::sync::Mutex<onecrawl_cdp::agent_memory::AgentMemory>,
}

#[pymethods]
impl AgentMemory {
    /// Create a new empty agent memory.
    #[new]
    fn new(path: &str) -> PyResult<Self> {
        let mem = onecrawl_cdp::agent_memory::AgentMemory::new(path);
        Ok(Self {
            inner: std::sync::Mutex::new(mem),
        })
    }

    /// Load agent memory from a file.
    #[staticmethod]
    fn load(path: &str) -> PyResult<Self> {
        let mem =
            onecrawl_cdp::agent_memory::AgentMemory::load(path).map_err(py_err)?;
        Ok(Self {
            inner: std::sync::Mutex::new(mem),
        })
    }

    /// Store a key-value pair with category and optional domain.
    #[pyo3(signature = (key, value_json, category, domain=None))]
    fn store(
        &self,
        key: &str,
        value_json: &str,
        category: &str,
        domain: Option<&str>,
    ) -> PyResult<()> {
        let mut mem = self.inner.lock().map_err(py_err)?;
        let value: serde_json::Value = serde_json::from_str(value_json).map_err(py_err)?;
        let cat: onecrawl_cdp::agent_memory::MemoryCategory =
            serde_json::from_str(&format!("\"{}\"", category)).map_err(py_err)?;
        mem.store(key, value, cat, domain.map(String::from))
            .map_err(py_err)
    }

    /// Recall a value by key.
    fn recall(&self, key: &str) -> PyResult<Option<String>> {
        let mut mem = self.inner.lock().map_err(py_err)?;
        match mem.recall(key) {
            Some(entry) => Ok(Some(serde_json::to_string(entry).map_err(py_err)?)),
            None => Ok(None),
        }
    }

    /// Search memory by query with optional category and domain filters.
    #[pyo3(signature = (query, category=None, domain=None))]
    fn search(
        &self,
        query: &str,
        category: Option<&str>,
        domain: Option<&str>,
    ) -> PyResult<String> {
        let mem = self.inner.lock().map_err(py_err)?;
        let cat: Option<onecrawl_cdp::agent_memory::MemoryCategory> = match category {
            Some(c) => Some(serde_json::from_str(&format!("\"{}\"", c)).map_err(py_err)?),
            None => None,
        };
        let results = mem.search(query, cat, domain);
        serde_json::to_string(&results).map_err(py_err)
    }

    /// Forget a specific key.
    fn forget(&self, key: &str) -> PyResult<bool> {
        let mut mem = self.inner.lock().map_err(py_err)?;
        Ok(mem.forget(key))
    }

    /// Clear all entries in a domain.
    fn clear_domain(&self, domain: &str) -> PyResult<usize> {
        let mut mem = self.inner.lock().map_err(py_err)?;
        Ok(mem.clear_domain(domain))
    }

    /// Clear all memory.
    fn clear_all(&self) -> PyResult<usize> {
        let mut mem = self.inner.lock().map_err(py_err)?;
        Ok(mem.clear_all())
    }

    /// Get memory statistics.
    fn stats(&self) -> PyResult<String> {
        let mem = self.inner.lock().map_err(py_err)?;
        let result = mem.stats();
        serde_json::to_string(&result).map_err(py_err)
    }

    /// Save memory to disk.
    fn save(&self) -> PyResult<()> {
        let mem = self.inner.lock().map_err(py_err)?;
        mem.save().map_err(py_err)
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

// ──────────────────────────── Server ────────────────────────────

/// Start the OneCrawl HTTP server.
#[pyfunction]
#[pyo3(signature = (port=9867))]
fn start_server(port: u16) -> PyResult<()> {
    let rt = tokio::runtime::Runtime::new()
        .map_err(|e| pyo3::exceptions::PyRuntimeError::new_err(e.to_string()))?;
    rt.block_on(async {
        onecrawl_server::serve::start_server(port)
            .await
            .map_err(|e| pyo3::exceptions::PyRuntimeError::new_err(e.to_string()))
    })
}

// ──────────────────────────── Module ────────────────────────────────

#[pymodule]
fn onecrawl(m: &Bound<'_, PyModule>) -> PyResult<()> {
    register_crypto(m)?;
    register_parser(m)?;
    m.add_class::<Store>()?;
    m.add_class::<Browser>()?;
    m.add_class::<IosClient>()?;
    m.add_class::<AndroidClient>()?;
    m.add_class::<AgentMemory>()?;
    m.add_function(wrap_pyfunction!(start_server, m)?)?;
    Ok(())
}
