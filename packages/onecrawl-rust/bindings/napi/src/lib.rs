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

// ──────────────────────────── Browser (CDP) ────────────────────────────

use std::sync::Arc;
use tokio::sync::{Mutex as TokioMutex, MutexGuard as TokioMutexGuard};

/// Stealth fingerprint configuration.
#[napi(object)]
pub struct FingerprintInfo {
    pub platform: String,
    pub hardware_concurrency: u32,
    pub device_memory: u32,
}

/// High-level browser automation powered by chromiumoxide (native CDP).
///
/// ```js
/// const browser = await NativeBrowser.launch(true);
/// await browser.goto("https://example.com");
/// const title = await browser.getTitle();
/// const screenshot = await browser.screenshot();
/// await browser.close();
/// ```
#[napi(js_name = "NativeBrowser")]
pub struct NativeBrowser {
    session: Arc<onecrawl_cdp::BrowserSession>,
    page: Arc<TokioMutex<Option<onecrawl_cdp::Page>>>,
    event_stream: Arc<TokioMutex<Option<onecrawl_cdp::EventStream>>>,
    har_recorder: Arc<TokioMutex<Option<onecrawl_cdp::HarRecorder>>>,
    ws_recorder: Arc<TokioMutex<Option<onecrawl_cdp::WsRecorder>>>,
}

#[napi]
impl NativeBrowser {
    /// Launch a new browser instance. Returns a Promise.
    #[napi(factory)]
    pub async fn launch(headless: Option<bool>) -> Result<Self> {
        let is_headless = headless.unwrap_or(true);
        let session = if is_headless {
            onecrawl_cdp::BrowserSession::launch_headless().await
        } else {
            onecrawl_cdp::BrowserSession::launch_headed().await
        }
        .map_err(|e| Error::from_reason(e.to_string()))?;

        let page = session
            .new_page("about:blank")
            .await
            .map_err(|e| Error::from_reason(e.to_string()))?;

        Ok(Self {
            session: Arc::new(session),
            page: Arc::new(TokioMutex::new(Some(page))),
            event_stream: Arc::new(TokioMutex::new(None)),
            har_recorder: Arc::new(TokioMutex::new(None)),
            ws_recorder: Arc::new(TokioMutex::new(None)),
        })
    }

    /// Connect to an existing browser via CDP WebSocket URL.
    #[napi(factory)]
    pub async fn connect(ws_url: String) -> Result<Self> {
        let session = onecrawl_cdp::BrowserSession::connect(&ws_url)
            .await
            .map_err(|e| Error::from_reason(e.to_string()))?;

        let page = session
            .new_page("about:blank")
            .await
            .map_err(|e| Error::from_reason(e.to_string()))?;

        Ok(Self {
            session: Arc::new(session),
            page: Arc::new(TokioMutex::new(Some(page))),
            event_stream: Arc::new(TokioMutex::new(None)),
            har_recorder: Arc::new(TokioMutex::new(None)),
            ws_recorder: Arc::new(TokioMutex::new(None)),
        })
    }

    /// Navigate to a URL.
    #[napi]
    pub async fn goto(&self, url: String) -> Result<()> {
        let guard: tokio::sync::MutexGuard<Option<onecrawl_cdp::Page>> = self.page.lock().await;
        let page = guard.as_ref().ok_or_else(|| Error::from_reason("browser closed"))?;
        onecrawl_cdp::navigation::goto(page, &url)
            .await
            .map_err(|e| Error::from_reason(e.to_string()))
    }

    /// Get current page URL.
    #[napi]
    pub async fn get_url(&self) -> Result<String> {
        let guard: tokio::sync::MutexGuard<Option<onecrawl_cdp::Page>> = self.page.lock().await;
        let page = guard.as_ref().ok_or_else(|| Error::from_reason("browser closed"))?;
        onecrawl_cdp::navigation::get_url(page)
            .await
            .map_err(|e| Error::from_reason(e.to_string()))
    }

    /// Get page title.
    #[napi]
    pub async fn get_title(&self) -> Result<String> {
        let guard: tokio::sync::MutexGuard<Option<onecrawl_cdp::Page>> = self.page.lock().await;
        let page = guard.as_ref().ok_or_else(|| Error::from_reason("browser closed"))?;
        onecrawl_cdp::navigation::get_title(page)
            .await
            .map_err(|e| Error::from_reason(e.to_string()))
    }

    /// Get page HTML content.
    #[napi]
    pub async fn content(&self) -> Result<String> {
        let guard: tokio::sync::MutexGuard<Option<onecrawl_cdp::Page>> = self.page.lock().await;
        let page = guard.as_ref().ok_or_else(|| Error::from_reason("browser closed"))?;
        onecrawl_cdp::page::get_content(page)
            .await
            .map_err(|e| Error::from_reason(e.to_string()))
    }

    /// Set page HTML content.
    #[napi]
    pub async fn set_content(&self, html: String) -> Result<()> {
        let guard: tokio::sync::MutexGuard<Option<onecrawl_cdp::Page>> = self.page.lock().await;
        let page = guard.as_ref().ok_or_else(|| Error::from_reason("browser closed"))?;
        onecrawl_cdp::page::set_content(page, &html)
            .await
            .map_err(|e| Error::from_reason(e.to_string()))
    }

    /// Take a viewport screenshot (PNG). Returns raw bytes.
    #[napi]
    pub async fn screenshot(&self) -> Result<Buffer> {
        let guard: tokio::sync::MutexGuard<Option<onecrawl_cdp::Page>> = self.page.lock().await;
        let page = guard.as_ref().ok_or_else(|| Error::from_reason("browser closed"))?;
        let bytes = onecrawl_cdp::screenshot::screenshot_viewport(page)
            .await
            .map_err(|e| Error::from_reason(e.to_string()))?;
        Ok(bytes.into())
    }

    /// Take a full-page screenshot (PNG). Returns raw bytes.
    #[napi]
    pub async fn screenshot_full(&self) -> Result<Buffer> {
        let guard: tokio::sync::MutexGuard<Option<onecrawl_cdp::Page>> = self.page.lock().await;
        let page = guard.as_ref().ok_or_else(|| Error::from_reason("browser closed"))?;
        let bytes = onecrawl_cdp::screenshot::screenshot_full(page)
            .await
            .map_err(|e| Error::from_reason(e.to_string()))?;
        Ok(bytes.into())
    }

    /// Screenshot a specific element by CSS selector (PNG).
    #[napi]
    pub async fn screenshot_element(&self, selector: String) -> Result<Buffer> {
        let guard: tokio::sync::MutexGuard<Option<onecrawl_cdp::Page>> = self.page.lock().await;
        let page = guard.as_ref().ok_or_else(|| Error::from_reason("browser closed"))?;
        let bytes = onecrawl_cdp::screenshot::screenshot_element(page, &selector)
            .await
            .map_err(|e| Error::from_reason(e.to_string()))?;
        Ok(bytes.into())
    }

    /// Save page as PDF. Returns raw bytes.
    #[napi]
    pub async fn pdf(&self) -> Result<Buffer> {
        let guard: tokio::sync::MutexGuard<Option<onecrawl_cdp::Page>> = self.page.lock().await;
        let page = guard.as_ref().ok_or_else(|| Error::from_reason("browser closed"))?;
        let bytes = onecrawl_cdp::screenshot::pdf(page)
            .await
            .map_err(|e| Error::from_reason(e.to_string()))?;
        Ok(bytes.into())
    }

    /// Evaluate JavaScript in the page. Returns JSON string.
    #[napi]
    pub async fn evaluate(&self, expression: String) -> Result<String> {
        let guard: tokio::sync::MutexGuard<Option<onecrawl_cdp::Page>> = self.page.lock().await;
        let page = guard.as_ref().ok_or_else(|| Error::from_reason("browser closed"))?;
        let val = onecrawl_cdp::page::evaluate_js(page, &expression)
            .await
            .map_err(|e| Error::from_reason(e.to_string()))?;
        Ok(val.to_string())
    }

    /// Click an element by CSS selector.
    #[napi]
    pub async fn click(&self, selector: String) -> Result<()> {
        let guard: tokio::sync::MutexGuard<Option<onecrawl_cdp::Page>> = self.page.lock().await;
        let page = guard.as_ref().ok_or_else(|| Error::from_reason("browser closed"))?;
        onecrawl_cdp::element::click(page, &selector)
            .await
            .map_err(|e| Error::from_reason(e.to_string()))
    }

    /// Double-click an element by CSS selector.
    #[napi]
    pub async fn double_click(&self, selector: String) -> Result<()> {
        let guard: tokio::sync::MutexGuard<Option<onecrawl_cdp::Page>> = self.page.lock().await;
        let page = guard.as_ref().ok_or_else(|| Error::from_reason("browser closed"))?;
        onecrawl_cdp::element::double_click(page, &selector)
            .await
            .map_err(|e| Error::from_reason(e.to_string()))
    }

    /// Type text into an element (key-by-key).
    #[napi(js_name = "type")]
    pub async fn type_text(&self, selector: String, text: String) -> Result<()> {
        let guard: tokio::sync::MutexGuard<Option<onecrawl_cdp::Page>> = self.page.lock().await;
        let page = guard.as_ref().ok_or_else(|| Error::from_reason("browser closed"))?;
        onecrawl_cdp::element::type_text(page, &selector, &text)
            .await
            .map_err(|e| Error::from_reason(e.to_string()))
    }

    /// Get text content of an element.
    #[napi]
    pub async fn get_text(&self, selector: String) -> Result<String> {
        let guard: tokio::sync::MutexGuard<Option<onecrawl_cdp::Page>> = self.page.lock().await;
        let page = guard.as_ref().ok_or_else(|| Error::from_reason("browser closed"))?;
        onecrawl_cdp::element::get_text(page, &selector)
            .await
            .map_err(|e| Error::from_reason(e.to_string()))
    }

    /// Get an attribute value from an element.
    #[napi]
    pub async fn get_attribute(&self, selector: String, attribute: String) -> Result<Option<String>> {
        let guard: tokio::sync::MutexGuard<Option<onecrawl_cdp::Page>> = self.page.lock().await;
        let page = guard.as_ref().ok_or_else(|| Error::from_reason("browser closed"))?;
        onecrawl_cdp::element::get_attribute(page, &selector, &attribute)
            .await
            .map_err(|e| Error::from_reason(e.to_string()))
    }

    /// Hover over an element.
    #[napi]
    pub async fn hover(&self, selector: String) -> Result<()> {
        let guard: tokio::sync::MutexGuard<Option<onecrawl_cdp::Page>> = self.page.lock().await;
        let page = guard.as_ref().ok_or_else(|| Error::from_reason("browser closed"))?;
        onecrawl_cdp::element::hover(page, &selector)
            .await
            .map_err(|e| Error::from_reason(e.to_string()))
    }

    /// Scroll an element into view.
    #[napi]
    pub async fn scroll_into_view(&self, selector: String) -> Result<()> {
        let guard: tokio::sync::MutexGuard<Option<onecrawl_cdp::Page>> = self.page.lock().await;
        let page = guard.as_ref().ok_or_else(|| Error::from_reason("browser closed"))?;
        onecrawl_cdp::element::scroll_into_view(page, &selector)
            .await
            .map_err(|e| Error::from_reason(e.to_string()))
    }

    /// Check a checkbox.
    #[napi]
    pub async fn check(&self, selector: String) -> Result<()> {
        let guard: tokio::sync::MutexGuard<Option<onecrawl_cdp::Page>> = self.page.lock().await;
        let page = guard.as_ref().ok_or_else(|| Error::from_reason("browser closed"))?;
        onecrawl_cdp::element::check(page, &selector)
            .await
            .map_err(|e| Error::from_reason(e.to_string()))
    }

    /// Uncheck a checkbox.
    #[napi]
    pub async fn uncheck(&self, selector: String) -> Result<()> {
        let guard: tokio::sync::MutexGuard<Option<onecrawl_cdp::Page>> = self.page.lock().await;
        let page = guard.as_ref().ok_or_else(|| Error::from_reason("browser closed"))?;
        onecrawl_cdp::element::uncheck(page, &selector)
            .await
            .map_err(|e| Error::from_reason(e.to_string()))
    }

    /// Select an option in a `<select>` element by value.
    #[napi]
    pub async fn select_option(&self, selector: String, value: String) -> Result<()> {
        let guard: tokio::sync::MutexGuard<Option<onecrawl_cdp::Page>> = self.page.lock().await;
        let page = guard.as_ref().ok_or_else(|| Error::from_reason("browser closed"))?;
        onecrawl_cdp::element::select_option(page, &selector, &value)
            .await
            .map_err(|e| Error::from_reason(e.to_string()))
    }

    /// Wait for a selector to appear (timeout in ms).
    #[napi]
    pub async fn wait_for_selector(&self, selector: String, timeout_ms: Option<u32>) -> Result<()> {
        let guard: tokio::sync::MutexGuard<Option<onecrawl_cdp::Page>> = self.page.lock().await;
        let page = guard.as_ref().ok_or_else(|| Error::from_reason("browser closed"))?;
        onecrawl_cdp::navigation::wait_for_selector(page, &selector, timeout_ms.unwrap_or(30000) as u64)
            .await
            .map_err(|e| Error::from_reason(e.to_string()))
    }

    /// Wait for URL to contain a pattern (timeout in ms).
    #[napi]
    pub async fn wait_for_url(&self, pattern: String, timeout_ms: Option<u32>) -> Result<()> {
        let guard: tokio::sync::MutexGuard<Option<onecrawl_cdp::Page>> = self.page.lock().await;
        let page = guard.as_ref().ok_or_else(|| Error::from_reason("browser closed"))?;
        onecrawl_cdp::navigation::wait_for_url(page, &pattern, timeout_ms.unwrap_or(30000) as u64)
            .await
            .map_err(|e| Error::from_reason(e.to_string()))
    }

    /// Go back in history.
    #[napi]
    pub async fn go_back(&self) -> Result<()> {
        let guard: tokio::sync::MutexGuard<Option<onecrawl_cdp::Page>> = self.page.lock().await;
        let page = guard.as_ref().ok_or_else(|| Error::from_reason("browser closed"))?;
        onecrawl_cdp::navigation::go_back(page)
            .await
            .map_err(|e| Error::from_reason(e.to_string()))
    }

    /// Go forward in history.
    #[napi]
    pub async fn go_forward(&self) -> Result<()> {
        let guard: tokio::sync::MutexGuard<Option<onecrawl_cdp::Page>> = self.page.lock().await;
        let page = guard.as_ref().ok_or_else(|| Error::from_reason("browser closed"))?;
        onecrawl_cdp::navigation::go_forward(page)
            .await
            .map_err(|e| Error::from_reason(e.to_string()))
    }

    /// Reload the page.
    #[napi]
    pub async fn reload(&self) -> Result<()> {
        let guard: tokio::sync::MutexGuard<Option<onecrawl_cdp::Page>> = self.page.lock().await;
        let page = guard.as_ref().ok_or_else(|| Error::from_reason("browser closed"))?;
        onecrawl_cdp::navigation::reload(page)
            .await
            .map_err(|e| Error::from_reason(e.to_string()))
    }

    /// Inject stealth anti-detection patches. Returns the fingerprint used.
    #[napi]
    pub async fn inject_stealth(&self) -> Result<FingerprintInfo> {
        let guard: tokio::sync::MutexGuard<Option<onecrawl_cdp::Page>> = self.page.lock().await;
        let page = guard.as_ref().ok_or_else(|| Error::from_reason("browser closed"))?;
        let fp = onecrawl_cdp::generate_fingerprint();
        let script = onecrawl_cdp::get_stealth_init_script(&fp);
        page.evaluate(script)
            .await
            .map_err(|e| Error::from_reason(format!("stealth injection failed: {e}")))?;
        Ok(FingerprintInfo {
            platform: fp.platform.clone(),
            hardware_concurrency: fp.hardware_concurrency,
            device_memory: fp.device_memory,
        })
    }

    /// Open a new page/tab and switch to it.
    #[napi]
    pub async fn new_page(&self, url: Option<String>) -> Result<()> {
        let new_page = self.session
            .new_page(url.as_deref().unwrap_or("about:blank"))
            .await
            .map_err(|e| Error::from_reason(e.to_string()))?;
        let mut guard = self.page.lock().await;
        *guard = Some(new_page);
        Ok(())
    }

    /// Enable network observation (intercept requests/responses).
    #[napi]
    pub async fn observe_network(&self) -> Result<()> {
        let guard: tokio::sync::MutexGuard<Option<onecrawl_cdp::Page>> = self.page.lock().await;
        let page = guard.as_ref().ok_or_else(|| Error::from_reason("browser closed"))?;
        onecrawl_cdp::network::observe_requests(page)
            .await
            .map_err(|e| Error::from_reason(e.to_string()))?;
        onecrawl_cdp::network::observe_responses(page)
            .await
            .map_err(|e| Error::from_reason(e.to_string()))
    }

    /// Get intercepted requests (after `observeNetwork`). Returns JSON string.
    #[napi]
    pub async fn get_requests(&self) -> Result<String> {
        let guard: tokio::sync::MutexGuard<Option<onecrawl_cdp::Page>> = self.page.lock().await;
        let page = guard.as_ref().ok_or_else(|| Error::from_reason("browser closed"))?;
        let val = onecrawl_cdp::network::get_intercepted_requests(page)
            .await
            .map_err(|e| Error::from_reason(e.to_string()))?;
        Ok(val.to_string())
    }

    /// Get intercepted responses (after `observeNetwork`). Returns JSON string.
    #[napi]
    pub async fn get_responses(&self) -> Result<String> {
        let guard: tokio::sync::MutexGuard<Option<onecrawl_cdp::Page>> = self.page.lock().await;
        let page = guard.as_ref().ok_or_else(|| Error::from_reason("browser closed"))?;
        let val = onecrawl_cdp::network::get_intercepted_responses(page)
            .await
            .map_err(|e| Error::from_reason(e.to_string()))?;
        Ok(val.to_string())
    }

    /// Wait for a specified number of milliseconds.
    #[napi]
    pub async fn wait(&self, ms: u32) -> Result<()> {
        onecrawl_cdp::navigation::wait_ms(ms as u64).await;
        Ok(())
    }

    /// Close the browser.
    #[napi]
    pub async fn close(&self) -> Result<()> {
        let mut guard = self.page.lock().await;
        *guard = None;
        Ok(())
    }

    // ──────────────── Cookie Management ────────────────

    /// Get all cookies (including httpOnly) via CDP.
    #[napi]
    pub async fn get_cookies(&self) -> Result<String> {
        let guard: tokio::sync::MutexGuard<Option<onecrawl_cdp::Page>> = self.page.lock().await;
        let page = guard.as_ref().ok_or_else(|| Error::from_reason("browser closed"))?;
        let cookies = onecrawl_cdp::cookie::get_all_cookies(page)
            .await
            .map_err(|e| Error::from_reason(e.to_string()))?;
        serde_json::to_string(&cookies).map_err(|e| Error::from_reason(e.to_string()))
    }

    /// Set a cookie. Accepts a JSON string of SetCookieParams.
    #[napi]
    pub async fn set_cookie(&self, params_json: String) -> Result<()> {
        let guard: tokio::sync::MutexGuard<Option<onecrawl_cdp::Page>> = self.page.lock().await;
        let page = guard.as_ref().ok_or_else(|| Error::from_reason("browser closed"))?;
        let params: onecrawl_cdp::SetCookieParams = serde_json::from_str(&params_json)
            .map_err(|e| Error::from_reason(format!("invalid cookie params: {e}")))?;
        onecrawl_cdp::cookie::set_cookie(page, &params)
            .await
            .map_err(|e| Error::from_reason(e.to_string()))
    }

    /// Delete cookies by name (optional domain/path).
    #[napi]
    pub async fn delete_cookies(
        &self,
        name: String,
        domain: Option<String>,
        path: Option<String>,
    ) -> Result<()> {
        let guard: tokio::sync::MutexGuard<Option<onecrawl_cdp::Page>> = self.page.lock().await;
        let page = guard.as_ref().ok_or_else(|| Error::from_reason("browser closed"))?;
        onecrawl_cdp::cookie::delete_cookies(
            page,
            &name,
            domain.as_deref(),
            path.as_deref(),
        )
        .await
        .map_err(|e| Error::from_reason(e.to_string()))
    }

    /// Clear all browser cookies.
    #[napi]
    pub async fn clear_cookies(&self) -> Result<()> {
        let guard: tokio::sync::MutexGuard<Option<onecrawl_cdp::Page>> = self.page.lock().await;
        let page = guard.as_ref().ok_or_else(|| Error::from_reason("browser closed"))?;
        onecrawl_cdp::cookie::clear_cookies(page)
            .await
            .map_err(|e| Error::from_reason(e.to_string()))
    }

    // ──────────────── Keyboard ────────────────

    /// Press a key (keyDown + keyUp).
    #[napi]
    pub async fn press_key(&self, key: String) -> Result<()> {
        let guard: tokio::sync::MutexGuard<Option<onecrawl_cdp::Page>> = self.page.lock().await;
        let page = guard.as_ref().ok_or_else(|| Error::from_reason("browser closed"))?;
        onecrawl_cdp::keyboard::press_key(page, &key)
            .await
            .map_err(|e| Error::from_reason(e.to_string()))
    }

    /// Send a keyboard shortcut (e.g., "Control+a", "Meta+c").
    #[napi]
    pub async fn keyboard_shortcut(&self, shortcut: String) -> Result<()> {
        let guard: tokio::sync::MutexGuard<Option<onecrawl_cdp::Page>> = self.page.lock().await;
        let page = guard.as_ref().ok_or_else(|| Error::from_reason("browser closed"))?;
        onecrawl_cdp::keyboard::keyboard_shortcut(page, &shortcut)
            .await
            .map_err(|e| Error::from_reason(e.to_string()))
    }

    /// Hold a key down.
    #[napi]
    pub async fn key_down(&self, key: String) -> Result<()> {
        let guard: tokio::sync::MutexGuard<Option<onecrawl_cdp::Page>> = self.page.lock().await;
        let page = guard.as_ref().ok_or_else(|| Error::from_reason("browser closed"))?;
        onecrawl_cdp::keyboard::key_down(page, &key)
            .await
            .map_err(|e| Error::from_reason(e.to_string()))
    }

    /// Release a key.
    #[napi]
    pub async fn key_up(&self, key: String) -> Result<()> {
        let guard: tokio::sync::MutexGuard<Option<onecrawl_cdp::Page>> = self.page.lock().await;
        let page = guard.as_ref().ok_or_else(|| Error::from_reason("browser closed"))?;
        onecrawl_cdp::keyboard::key_up(page, &key)
            .await
            .map_err(|e| Error::from_reason(e.to_string()))
    }

    /// Fill an input field (clear + set value + fire events).
    #[napi]
    pub async fn fill(&self, selector: String, value: String) -> Result<()> {
        let guard: tokio::sync::MutexGuard<Option<onecrawl_cdp::Page>> = self.page.lock().await;
        let page = guard.as_ref().ok_or_else(|| Error::from_reason("browser closed"))?;
        onecrawl_cdp::keyboard::fill(page, &selector, &value)
            .await
            .map_err(|e| Error::from_reason(e.to_string()))
    }

    // ──────────────── Advanced Input ────────────────

    /// Drag an element and drop onto another (CSS selectors).
    #[napi]
    pub async fn drag_and_drop(&self, source: String, target: String) -> Result<()> {
        let guard: tokio::sync::MutexGuard<Option<onecrawl_cdp::Page>> = self.page.lock().await;
        let page = guard.as_ref().ok_or_else(|| Error::from_reason("browser closed"))?;
        onecrawl_cdp::input::drag_and_drop(page, &source, &target)
            .await
            .map_err(|e| Error::from_reason(e.to_string()))
    }

    /// Upload files to a `<input type="file">` element.
    #[napi]
    pub async fn upload_file(&self, selector: String, file_paths: Vec<String>) -> Result<()> {
        let guard: tokio::sync::MutexGuard<Option<onecrawl_cdp::Page>> = self.page.lock().await;
        let page = guard.as_ref().ok_or_else(|| Error::from_reason("browser closed"))?;
        onecrawl_cdp::input::set_file_input(page, &selector, &file_paths)
            .await
            .map_err(|e| Error::from_reason(e.to_string()))
    }

    /// Get the bounding box of an element. Returns { x, y, width, height }.
    #[napi]
    pub async fn bounding_box(&self, selector: String) -> Result<String> {
        let guard: tokio::sync::MutexGuard<Option<onecrawl_cdp::Page>> = self.page.lock().await;
        let page = guard.as_ref().ok_or_else(|| Error::from_reason("browser closed"))?;
        let (x, y, w, h) = onecrawl_cdp::input::bounding_box(page, &selector)
            .await
            .map_err(|e| Error::from_reason(e.to_string()))?;
        Ok(serde_json::json!({"x": x, "y": y, "width": w, "height": h}).to_string())
    }

    /// Tap an element (touch simulation).
    #[napi]
    pub async fn tap(&self, selector: String) -> Result<()> {
        let guard: tokio::sync::MutexGuard<Option<onecrawl_cdp::Page>> = self.page.lock().await;
        let page = guard.as_ref().ok_or_else(|| Error::from_reason("browser closed"))?;
        onecrawl_cdp::input::tap(page, &selector)
            .await
            .map_err(|e| Error::from_reason(e.to_string()))
    }

    // ──── Emulation ────

    /// Set viewport dimensions and device emulation.
    #[napi]
    pub async fn set_viewport(
        &self,
        width: u32,
        height: u32,
        device_scale_factor: Option<f64>,
        is_mobile: Option<bool>,
        has_touch: Option<bool>,
    ) -> Result<()> {
        let guard: tokio::sync::MutexGuard<Option<onecrawl_cdp::Page>> = self.page.lock().await;
        let page = guard.as_ref().ok_or_else(|| Error::from_reason("browser closed"))?;
        let vp = onecrawl_cdp::emulation::Viewport {
            width,
            height,
            device_scale_factor: device_scale_factor.unwrap_or(1.0),
            is_mobile: is_mobile.unwrap_or(false),
            has_touch: has_touch.unwrap_or(false),
        };
        onecrawl_cdp::emulation::set_viewport(page, &vp)
            .await
            .map_err(|e| Error::from_reason(e.to_string()))
    }

    /// Set viewport from a device preset name (desktop, iphone14, ipad, pixel7).
    #[napi]
    pub async fn set_device(&self, device: String) -> Result<()> {
        let guard: tokio::sync::MutexGuard<Option<onecrawl_cdp::Page>> = self.page.lock().await;
        let page = guard.as_ref().ok_or_else(|| Error::from_reason("browser closed"))?;
        let vp = match device.to_lowercase().as_str() {
            "desktop" => onecrawl_cdp::emulation::Viewport::desktop(),
            "iphone14" | "iphone_14" | "iphone" => onecrawl_cdp::emulation::Viewport::iphone_14(),
            "ipad" => onecrawl_cdp::emulation::Viewport::ipad(),
            "pixel7" | "pixel_7" | "pixel" => onecrawl_cdp::emulation::Viewport::pixel_7(),
            _ => return Err(Error::from_reason(format!("Unknown device: {device}"))),
        };
        onecrawl_cdp::emulation::set_viewport(page, &vp)
            .await
            .map_err(|e| Error::from_reason(e.to_string()))
    }

    /// Clear viewport override (revert to browser defaults).
    #[napi]
    pub async fn clear_viewport(&self) -> Result<()> {
        let guard: tokio::sync::MutexGuard<Option<onecrawl_cdp::Page>> = self.page.lock().await;
        let page = guard.as_ref().ok_or_else(|| Error::from_reason("browser closed"))?;
        onecrawl_cdp::emulation::clear_viewport(page)
            .await
            .map_err(|e| Error::from_reason(e.to_string()))
    }

    /// Override the browser user agent string.
    #[napi]
    pub async fn set_user_agent(&self, user_agent: String) -> Result<()> {
        let guard: tokio::sync::MutexGuard<Option<onecrawl_cdp::Page>> = self.page.lock().await;
        let page = guard.as_ref().ok_or_else(|| Error::from_reason("browser closed"))?;
        onecrawl_cdp::emulation::set_user_agent(page, &user_agent)
            .await
            .map_err(|e| Error::from_reason(e.to_string()))
    }

    /// Set geolocation override.
    #[napi]
    pub async fn set_geolocation(&self, latitude: f64, longitude: f64, accuracy: Option<f64>) -> Result<()> {
        let guard: tokio::sync::MutexGuard<Option<onecrawl_cdp::Page>> = self.page.lock().await;
        let page = guard.as_ref().ok_or_else(|| Error::from_reason("browser closed"))?;
        onecrawl_cdp::emulation::set_geolocation(page, latitude, longitude, accuracy.unwrap_or(1.0))
            .await
            .map_err(|e| Error::from_reason(e.to_string()))
    }

    /// Emulate color scheme preference (dark/light).
    #[napi]
    pub async fn set_color_scheme(&self, scheme: String) -> Result<()> {
        let guard: tokio::sync::MutexGuard<Option<onecrawl_cdp::Page>> = self.page.lock().await;
        let page = guard.as_ref().ok_or_else(|| Error::from_reason("browser closed"))?;
        onecrawl_cdp::emulation::set_color_scheme(page, &scheme)
            .await
            .map_err(|e| Error::from_reason(e.to_string()))
    }

    // ──── Network (advanced) ────

    /// Block specific resource types (e.g., ["Image", "Font", "Stylesheet"]).
    #[napi]
    pub async fn block_resources(&self, resource_types: Vec<String>) -> Result<()> {
        let guard: tokio::sync::MutexGuard<Option<onecrawl_cdp::Page>> = self.page.lock().await;
        let page = guard.as_ref().ok_or_else(|| Error::from_reason("browser closed"))?;
        let types: Vec<onecrawl_cdp::ResourceType> = resource_types
            .iter()
            .map(|s| serde_json::from_str(&format!("\"{}\"", s)))
            .collect::<std::result::Result<_, _>>()
            .map_err(|e| Error::from_reason(format!("Invalid resource type: {e}")))?;
        onecrawl_cdp::network::block_resources(page, &types)
            .await
            .map_err(|e| Error::from_reason(e.to_string()))
    }

    // ──── Screenshot & PDF (with options) ────

    /// Take a screenshot with custom options.
    /// format: "png" | "jpeg" | "webp", quality: 0-100 (jpeg/webp only), fullPage: boolean
    #[napi]
    pub async fn screenshot_with_options(
        &self,
        format: Option<String>,
        quality: Option<u32>,
        full_page: Option<bool>,
    ) -> Result<Buffer> {
        let guard: tokio::sync::MutexGuard<Option<onecrawl_cdp::Page>> = self.page.lock().await;
        let page = guard.as_ref().ok_or_else(|| Error::from_reason("browser closed"))?;
        let fmt = match format.as_deref() {
            Some("jpeg") | Some("jpg") => onecrawl_cdp::ImageFormat::Jpeg,
            Some("webp") => onecrawl_cdp::ImageFormat::Webp,
            _ => onecrawl_cdp::ImageFormat::Png,
        };
        let opts = onecrawl_cdp::ScreenshotOptions {
            format: fmt,
            quality,
            full_page: full_page.unwrap_or(false),
        };
        let bytes = onecrawl_cdp::screenshot::screenshot_with_options(page, &opts)
            .await
            .map_err(|e| Error::from_reason(e.to_string()))?;
        Ok(bytes.into())
    }

    /// Generate PDF with custom options (landscape, scale, paper size).
    #[napi]
    pub async fn pdf_with_options(
        &self,
        landscape: Option<bool>,
        scale: Option<f64>,
        paper_width: Option<f64>,
        paper_height: Option<f64>,
    ) -> Result<Buffer> {
        let guard: tokio::sync::MutexGuard<Option<onecrawl_cdp::Page>> = self.page.lock().await;
        let page = guard.as_ref().ok_or_else(|| Error::from_reason("browser closed"))?;
        let opts = onecrawl_cdp::PdfOptions {
            landscape: landscape.unwrap_or(false),
            scale: scale.unwrap_or(1.0),
            paper_width: paper_width.unwrap_or(8.5),
            paper_height: paper_height.unwrap_or(11.0),
        };
        let bytes = onecrawl_cdp::screenshot::pdf_with_options(page, &opts)
            .await
            .map_err(|e| Error::from_reason(e.to_string()))?;
        Ok(bytes.into())
    }

    // ──── Event Streaming ────

    /// Start event observation (console + errors). Call drainEvents() to poll.
    #[napi]
    pub async fn start_event_stream(&self) -> Result<()> {
        let guard: tokio::sync::MutexGuard<Option<onecrawl_cdp::Page>> = self.page.lock().await;
        let page = guard.as_ref().ok_or_else(|| Error::from_reason("browser closed"))?;

        let stream = onecrawl_cdp::EventStream::new(256);
        let tx = stream.sender();

        onecrawl_cdp::events::observe_console(page, tx.clone())
            .await
            .map_err(|e| Error::from_reason(e.to_string()))?;
        onecrawl_cdp::events::observe_errors(page, tx.clone())
            .await
            .map_err(|e| Error::from_reason(e.to_string()))?;

        let mut es = self.event_stream.lock().await;
        *es = Some(stream);
        Ok(())
    }

    /// Drain buffered events (console messages + page errors). Returns JSON array.
    #[napi]
    pub async fn drain_events(&self) -> Result<String> {
        let guard: tokio::sync::MutexGuard<Option<onecrawl_cdp::Page>> = self.page.lock().await;
        let page = guard.as_ref().ok_or_else(|| Error::from_reason("browser closed"))?;

        let es = self.event_stream.lock().await;
        let stream = es.as_ref().ok_or_else(|| Error::from_reason("event stream not started — call startEventStream() first"))?;
        let tx = stream.sender();

        let console_count = onecrawl_cdp::events::drain_console(page, &tx)
            .await
            .map_err(|e| Error::from_reason(e.to_string()))?;
        let error_count = onecrawl_cdp::events::drain_errors(page, &tx)
            .await
            .map_err(|e| Error::from_reason(e.to_string()))?;

        Ok(serde_json::json!({
            "console_messages": console_count,
            "page_errors": error_count,
            "total": console_count + error_count,
        }).to_string())
    }

    /// Emit a custom event into the stream.
    #[napi]
    pub async fn emit_event(&self, name: String, data: String) -> Result<()> {
        let es = self.event_stream.lock().await;
        let stream = es.as_ref().ok_or_else(|| Error::from_reason("event stream not started"))?;
        let tx = stream.sender();
        let json_data: serde_json::Value = serde_json::from_str(&data)
            .unwrap_or(serde_json::Value::String(data));
        onecrawl_cdp::events::emit_custom(&tx, &name, json_data)
            .map_err(|e| Error::from_reason(e.to_string()))
    }

    // ── HAR Recording ──────────────────────────────────────────────

    /// Start HAR (HTTP Archive) recording on the current page.
    #[napi]
    pub async fn start_har_recording(&self) -> Result<()> {
        let guard: TokioMutexGuard<Option<onecrawl_cdp::Page>> = self.page.lock().await;
        let page = guard.as_ref().ok_or_else(|| Error::from_reason("no page"))?;
        let recorder = onecrawl_cdp::HarRecorder::new();
        onecrawl_cdp::har::start_har_recording(page, &recorder)
            .await
            .map_err(|e| Error::from_reason(e.to_string()))?;
        let mut hr = self.har_recorder.lock().await;
        *hr = Some(recorder);
        Ok(())
    }

    /// Drain new HAR entries from the page. Returns the number of new entries.
    #[napi]
    pub async fn drain_har_entries(&self) -> Result<u32> {
        let guard: TokioMutexGuard<Option<onecrawl_cdp::Page>> = self.page.lock().await;
        let page = guard.as_ref().ok_or_else(|| Error::from_reason("no page"))?;
        let hr = self.har_recorder.lock().await;
        let recorder = hr.as_ref().ok_or_else(|| Error::from_reason("HAR recording not started"))?;
        let count = onecrawl_cdp::har::drain_har_entries(page, recorder)
            .await
            .map_err(|e| Error::from_reason(e.to_string()))?;
        Ok(count as u32)
    }

    /// Export all HAR entries as HAR 1.2 JSON string.
    #[napi]
    pub async fn export_har(&self) -> Result<String> {
        let guard: TokioMutexGuard<Option<onecrawl_cdp::Page>> = self.page.lock().await;
        let page_url = if let Some(page) = guard.as_ref() {
            page.url().await.unwrap_or(None).unwrap_or_default()
        } else {
            String::new()
        };
        let hr = self.har_recorder.lock().await;
        let recorder = hr.as_ref().ok_or_else(|| Error::from_reason("HAR recording not started"))?;
        let har = onecrawl_cdp::har::export_har(recorder, &page_url)
            .await
            .map_err(|e| Error::from_reason(e.to_string()))?;
        Ok(har.to_string())
    }

    // ── WebSocket Recording ────────────────────────────────────────

    /// Start WebSocket frame interception on the current page.
    #[napi]
    pub async fn start_ws_recording(&self) -> Result<()> {
        let guard: TokioMutexGuard<Option<onecrawl_cdp::Page>> = self.page.lock().await;
        let page = guard.as_ref().ok_or_else(|| Error::from_reason("no page"))?;
        let recorder = onecrawl_cdp::WsRecorder::new();
        onecrawl_cdp::websocket::start_ws_recording(page, &recorder)
            .await
            .map_err(|e| Error::from_reason(e.to_string()))?;
        let mut wr = self.ws_recorder.lock().await;
        *wr = Some(recorder);
        Ok(())
    }

    /// Drain new WebSocket frames from the page. Returns the number of new frames.
    #[napi]
    pub async fn drain_ws_frames(&self) -> Result<u32> {
        let guard: TokioMutexGuard<Option<onecrawl_cdp::Page>> = self.page.lock().await;
        let page = guard.as_ref().ok_or_else(|| Error::from_reason("no page"))?;
        let wr = self.ws_recorder.lock().await;
        let recorder = wr.as_ref().ok_or_else(|| Error::from_reason("WS recording not started"))?;
        let count = onecrawl_cdp::websocket::drain_ws_frames(page, recorder)
            .await
            .map_err(|e| Error::from_reason(e.to_string()))?;
        Ok(count as u32)
    }

    /// Export all captured WebSocket frames as JSON string.
    #[napi]
    pub async fn export_ws_frames(&self) -> Result<String> {
        let wr = self.ws_recorder.lock().await;
        let recorder = wr.as_ref().ok_or_else(|| Error::from_reason("WS recording not started"))?;
        let frames = onecrawl_cdp::websocket::export_ws_frames(recorder)
            .await
            .map_err(|e| Error::from_reason(e.to_string()))?;
        Ok(frames.to_string())
    }

    /// Get the count of active WebSocket connections.
    #[napi]
    pub async fn active_ws_connections(&self) -> Result<u32> {
        let guard: TokioMutexGuard<Option<onecrawl_cdp::Page>> = self.page.lock().await;
        let page = guard.as_ref().ok_or_else(|| Error::from_reason("no page"))?;
        let count = onecrawl_cdp::websocket::active_ws_connections(page)
            .await
            .map_err(|e| Error::from_reason(e.to_string()))?;
        Ok(count as u32)
    }

    // ── Console Interception ───────────────────────────────────────

    /// Start capturing console messages (log/warn/error/info/debug).
    #[napi]
    pub async fn start_console_capture(&self) -> Result<()> {
        let guard: TokioMutexGuard<Option<onecrawl_cdp::Page>> = self.page.lock().await;
        let page = guard.as_ref().ok_or_else(|| Error::from_reason("no page"))?;
        onecrawl_cdp::console::start_console_capture(page)
            .await
            .map_err(|e| Error::from_reason(e.to_string()))
    }

    /// Drain captured console entries as JSON string.
    #[napi]
    pub async fn drain_console_entries(&self) -> Result<String> {
        let guard: TokioMutexGuard<Option<onecrawl_cdp::Page>> = self.page.lock().await;
        let page = guard.as_ref().ok_or_else(|| Error::from_reason("no page"))?;
        let entries = onecrawl_cdp::console::drain_console_entries(page)
            .await
            .map_err(|e| Error::from_reason(e.to_string()))?;
        serde_json::to_string(&entries).map_err(|e| Error::from_reason(e.to_string()))
    }

    /// Clear the console capture buffer.
    #[napi]
    pub async fn clear_console(&self) -> Result<()> {
        let guard: TokioMutexGuard<Option<onecrawl_cdp::Page>> = self.page.lock().await;
        let page = guard.as_ref().ok_or_else(|| Error::from_reason("no page"))?;
        onecrawl_cdp::console::clear_console(page)
            .await
            .map_err(|e| Error::from_reason(e.to_string()))
    }

    // ── Dialog Handling ────────────────────────────────────────────

    /// Set dialog auto-handler (alert/confirm/prompt).
    #[napi]
    pub async fn set_dialog_handler(&self, accept: bool, prompt_text: Option<String>) -> Result<()> {
        let guard: TokioMutexGuard<Option<onecrawl_cdp::Page>> = self.page.lock().await;
        let page = guard.as_ref().ok_or_else(|| Error::from_reason("no page"))?;
        onecrawl_cdp::dialog::set_dialog_handler(page, accept, prompt_text.as_deref())
            .await
            .map_err(|e| Error::from_reason(e.to_string()))
    }

    /// Get dialog history as JSON string.
    #[napi]
    pub async fn get_dialog_history(&self) -> Result<String> {
        let guard: TokioMutexGuard<Option<onecrawl_cdp::Page>> = self.page.lock().await;
        let page = guard.as_ref().ok_or_else(|| Error::from_reason("no page"))?;
        let events = onecrawl_cdp::dialog::get_dialog_history(page)
            .await
            .map_err(|e| Error::from_reason(e.to_string()))?;
        serde_json::to_string(&events).map_err(|e| Error::from_reason(e.to_string()))
    }

    /// Clear dialog history.
    #[napi]
    pub async fn clear_dialog_history(&self) -> Result<()> {
        let guard: TokioMutexGuard<Option<onecrawl_cdp::Page>> = self.page.lock().await;
        let page = guard.as_ref().ok_or_else(|| Error::from_reason("no page"))?;
        onecrawl_cdp::dialog::clear_dialog_history(page)
            .await
            .map_err(|e| Error::from_reason(e.to_string()))
    }

    // ── Service Workers ────────────────────────────────────────────

    /// Get all registered service workers as JSON string.
    #[napi]
    pub async fn get_service_workers(&self) -> Result<String> {
        let guard: TokioMutexGuard<Option<onecrawl_cdp::Page>> = self.page.lock().await;
        let page = guard.as_ref().ok_or_else(|| Error::from_reason("no page"))?;
        let workers = onecrawl_cdp::workers::get_service_workers(page)
            .await
            .map_err(|e| Error::from_reason(e.to_string()))?;
        serde_json::to_string(&workers).map_err(|e| Error::from_reason(e.to_string()))
    }

    /// Unregister all service workers. Returns the number unregistered.
    #[napi]
    pub async fn unregister_service_workers(&self) -> Result<u32> {
        let guard: TokioMutexGuard<Option<onecrawl_cdp::Page>> = self.page.lock().await;
        let page = guard.as_ref().ok_or_else(|| Error::from_reason("no page"))?;
        let count = onecrawl_cdp::workers::unregister_service_workers(page)
            .await
            .map_err(|e| Error::from_reason(e.to_string()))?;
        Ok(count as u32)
    }

    /// Get worker info as JSON string.
    #[napi]
    pub async fn get_worker_info(&self) -> Result<String> {
        let guard: TokioMutexGuard<Option<onecrawl_cdp::Page>> = self.page.lock().await;
        let page = guard.as_ref().ok_or_else(|| Error::from_reason("no page"))?;
        let info = onecrawl_cdp::workers::get_worker_info(page)
            .await
            .map_err(|e| Error::from_reason(e.to_string()))?;
        Ok(info.to_string())
    }

    // ── Web Storage ────────────────────────────────────────────────

    /// Get all localStorage contents as JSON string.
    #[napi]
    pub async fn get_local_storage(&self) -> Result<String> {
        let guard: TokioMutexGuard<Option<onecrawl_cdp::Page>> = self.page.lock().await;
        let page = guard.as_ref().ok_or_else(|| Error::from_reason("no page"))?;
        let data = onecrawl_cdp::web_storage::get_local_storage(page)
            .await
            .map_err(|e| Error::from_reason(e.to_string()))?;
        Ok(data.to_string())
    }

    /// Set a localStorage item.
    #[napi]
    pub async fn set_local_storage(&self, key: String, value: String) -> Result<()> {
        let guard: TokioMutexGuard<Option<onecrawl_cdp::Page>> = self.page.lock().await;
        let page = guard.as_ref().ok_or_else(|| Error::from_reason("no page"))?;
        onecrawl_cdp::web_storage::set_local_storage(page, &key, &value)
            .await
            .map_err(|e| Error::from_reason(e.to_string()))
    }

    /// Clear all localStorage.
    #[napi]
    pub async fn clear_local_storage(&self) -> Result<()> {
        let guard: TokioMutexGuard<Option<onecrawl_cdp::Page>> = self.page.lock().await;
        let page = guard.as_ref().ok_or_else(|| Error::from_reason("no page"))?;
        onecrawl_cdp::web_storage::clear_local_storage(page)
            .await
            .map_err(|e| Error::from_reason(e.to_string()))
    }

    /// Get all sessionStorage contents as JSON string.
    #[napi]
    pub async fn get_session_storage(&self) -> Result<String> {
        let guard: TokioMutexGuard<Option<onecrawl_cdp::Page>> = self.page.lock().await;
        let page = guard.as_ref().ok_or_else(|| Error::from_reason("no page"))?;
        let data = onecrawl_cdp::web_storage::get_session_storage(page)
            .await
            .map_err(|e| Error::from_reason(e.to_string()))?;
        Ok(data.to_string())
    }

    /// Set a sessionStorage item.
    #[napi]
    pub async fn set_session_storage(&self, key: String, value: String) -> Result<()> {
        let guard: TokioMutexGuard<Option<onecrawl_cdp::Page>> = self.page.lock().await;
        let page = guard.as_ref().ok_or_else(|| Error::from_reason("no page"))?;
        onecrawl_cdp::web_storage::set_session_storage(page, &key, &value)
            .await
            .map_err(|e| Error::from_reason(e.to_string()))
    }

    /// Clear all sessionStorage.
    #[napi]
    pub async fn clear_session_storage(&self) -> Result<()> {
        let guard: TokioMutexGuard<Option<onecrawl_cdp::Page>> = self.page.lock().await;
        let page = guard.as_ref().ok_or_else(|| Error::from_reason("no page"))?;
        onecrawl_cdp::web_storage::clear_session_storage(page)
            .await
            .map_err(|e| Error::from_reason(e.to_string()))
    }

    /// Get IndexedDB database names as JSON string.
    #[napi]
    pub async fn get_indexeddb_databases(&self) -> Result<String> {
        let guard: TokioMutexGuard<Option<onecrawl_cdp::Page>> = self.page.lock().await;
        let page = guard.as_ref().ok_or_else(|| Error::from_reason("no page"))?;
        let names = onecrawl_cdp::web_storage::get_indexeddb_databases(page)
            .await
            .map_err(|e| Error::from_reason(e.to_string()))?;
        serde_json::to_string(&names).map_err(|e| Error::from_reason(e.to_string()))
    }

    /// Clear all site data (localStorage + sessionStorage + cookies + cache).
    #[napi]
    pub async fn clear_site_data(&self) -> Result<()> {
        let guard: TokioMutexGuard<Option<onecrawl_cdp::Page>> = self.page.lock().await;
        let page = guard.as_ref().ok_or_else(|| Error::from_reason("no page"))?;
        onecrawl_cdp::web_storage::clear_site_data(page)
            .await
            .map_err(|e| Error::from_reason(e.to_string()))
    }

    // ── Code Coverage ──────────────────────────────────────────────

    /// Start JavaScript code coverage collection via CDP Profiler.
    #[napi]
    pub async fn start_js_coverage(&self) -> Result<()> {
        let guard: TokioMutexGuard<Option<onecrawl_cdp::Page>> = self.page.lock().await;
        let page = guard.as_ref().ok_or_else(|| Error::from_reason("no page"))?;
        onecrawl_cdp::coverage::start_js_coverage(page)
            .await
            .map_err(|e| Error::from_reason(e.to_string()))
    }

    /// Stop JavaScript code coverage and return the report as JSON string.
    #[napi]
    pub async fn stop_js_coverage(&self) -> Result<String> {
        let guard: TokioMutexGuard<Option<onecrawl_cdp::Page>> = self.page.lock().await;
        let page = guard.as_ref().ok_or_else(|| Error::from_reason("no page"))?;
        let report = onecrawl_cdp::coverage::stop_js_coverage(page)
            .await
            .map_err(|e| Error::from_reason(e.to_string()))?;
        serde_json::to_string(&report)
            .map_err(|e| Error::from_reason(e.to_string()))
    }

    /// Start CSS coverage collection.
    #[napi]
    pub async fn start_css_coverage(&self) -> Result<()> {
        let guard: TokioMutexGuard<Option<onecrawl_cdp::Page>> = self.page.lock().await;
        let page = guard.as_ref().ok_or_else(|| Error::from_reason("no page"))?;
        onecrawl_cdp::coverage::start_css_coverage(page)
            .await
            .map_err(|e| Error::from_reason(e.to_string()))
    }

    /// Get CSS coverage summary as JSON string.
    #[napi]
    pub async fn get_css_coverage(&self) -> Result<String> {
        let guard: TokioMutexGuard<Option<onecrawl_cdp::Page>> = self.page.lock().await;
        let page = guard.as_ref().ok_or_else(|| Error::from_reason("no page"))?;
        let report = onecrawl_cdp::coverage::get_css_coverage(page)
            .await
            .map_err(|e| Error::from_reason(e.to_string()))?;
        Ok(report.to_string())
    }

    // ── Accessibility ──────────────────────────────────────────────

    /// Get the full accessibility tree of the current page as JSON.
    #[napi]
    pub async fn get_accessibility_tree(&self) -> Result<String> {
        let guard: TokioMutexGuard<Option<onecrawl_cdp::Page>> = self.page.lock().await;
        let page = guard.as_ref().ok_or_else(|| Error::from_reason("no page"))?;
        let result = onecrawl_cdp::accessibility::get_accessibility_tree(page)
            .await
            .map_err(|e| Error::from_reason(e.to_string()))?;
        Ok(result.to_string())
    }

    /// Get accessibility info for a specific element by CSS selector.
    #[napi]
    pub async fn get_element_accessibility(&self, selector: String) -> Result<String> {
        let guard: TokioMutexGuard<Option<onecrawl_cdp::Page>> = self.page.lock().await;
        let page = guard.as_ref().ok_or_else(|| Error::from_reason("no page"))?;
        let result = onecrawl_cdp::accessibility::get_element_accessibility(page, &selector)
            .await
            .map_err(|e| Error::from_reason(e.to_string()))?;
        Ok(result.to_string())
    }

    /// Run an accessibility audit on the current page and return the report as JSON.
    #[napi]
    pub async fn audit_accessibility(&self) -> Result<String> {
        let guard: TokioMutexGuard<Option<onecrawl_cdp::Page>> = self.page.lock().await;
        let page = guard.as_ref().ok_or_else(|| Error::from_reason("no page"))?;
        let result = onecrawl_cdp::accessibility::audit_accessibility(page)
            .await
            .map_err(|e| Error::from_reason(e.to_string()))?;
        serde_json::to_string(&result).map_err(|e| Error::from_reason(e.to_string()))
    }

    // ── Network Throttling ─────────────────────────────────────────

    /// Set network throttling to a named profile (fast3g, slow3g, offline, regular4g, wifi).
    #[napi]
    pub async fn set_network_throttle(&self, profile: String) -> Result<()> {
        let guard: TokioMutexGuard<Option<onecrawl_cdp::Page>> = self.page.lock().await;
        let page = guard.as_ref().ok_or_else(|| Error::from_reason("no page"))?;
        let p = parse_network_profile(&profile).map_err(Error::from_reason)?;
        onecrawl_cdp::throttle::set_network_conditions(page, p)
            .await
            .map_err(|e| Error::from_reason(e.to_string()))
    }

    /// Set custom network throttling conditions.
    #[napi]
    pub async fn set_network_throttle_custom(
        &self,
        download_kbps: f64,
        upload_kbps: f64,
        latency_ms: f64,
    ) -> Result<()> {
        let guard: TokioMutexGuard<Option<onecrawl_cdp::Page>> = self.page.lock().await;
        let page = guard.as_ref().ok_or_else(|| Error::from_reason("no page"))?;
        let profile = onecrawl_cdp::NetworkProfile::Custom {
            download_kbps,
            upload_kbps,
            latency_ms,
        };
        onecrawl_cdp::throttle::set_network_conditions(page, profile)
            .await
            .map_err(|e| Error::from_reason(e.to_string()))
    }

    /// Clear network throttling.
    #[napi]
    pub async fn clear_network_throttle(&self) -> Result<()> {
        let guard: TokioMutexGuard<Option<onecrawl_cdp::Page>> = self.page.lock().await;
        let page = guard.as_ref().ok_or_else(|| Error::from_reason("no page"))?;
        onecrawl_cdp::throttle::clear_network_conditions(page)
            .await
            .map_err(|e| Error::from_reason(e.to_string()))
    }

    // ── Performance Tracing ────────────────────────────────────────

    /// Start performance tracing on the current page.
    #[napi]
    pub async fn start_tracing(&self) -> Result<()> {
        let guard: TokioMutexGuard<Option<onecrawl_cdp::Page>> = self.page.lock().await;
        let page = guard.as_ref().ok_or_else(|| Error::from_reason("no page"))?;
        onecrawl_cdp::tracing_cdp::start_tracing(page)
            .await
            .map_err(|e| Error::from_reason(e.to_string()))
    }

    /// Stop tracing and return the trace data as JSON.
    #[napi]
    pub async fn stop_tracing(&self) -> Result<String> {
        let guard: TokioMutexGuard<Option<onecrawl_cdp::Page>> = self.page.lock().await;
        let page = guard.as_ref().ok_or_else(|| Error::from_reason("no page"))?;
        let result = onecrawl_cdp::tracing_cdp::stop_tracing(page)
            .await
            .map_err(|e| Error::from_reason(e.to_string()))?;
        Ok(result.to_string())
    }

    /// Get performance metrics from the current page as JSON.
    #[napi]
    pub async fn get_performance_metrics(&self) -> Result<String> {
        let guard: TokioMutexGuard<Option<onecrawl_cdp::Page>> = self.page.lock().await;
        let page = guard.as_ref().ok_or_else(|| Error::from_reason("no page"))?;
        let result = onecrawl_cdp::tracing_cdp::get_performance_metrics(page)
            .await
            .map_err(|e| Error::from_reason(e.to_string()))?;
        serde_json::to_string(&result).map_err(|e| Error::from_reason(e.to_string()))
    }

    /// Get navigation timing data as JSON.
    #[napi]
    pub async fn get_navigation_timing(&self) -> Result<String> {
        let guard: TokioMutexGuard<Option<onecrawl_cdp::Page>> = self.page.lock().await;
        let page = guard.as_ref().ok_or_else(|| Error::from_reason("no page"))?;
        let result = onecrawl_cdp::tracing_cdp::get_navigation_timing(page)
            .await
            .map_err(|e| Error::from_reason(e.to_string()))?;
        Ok(result.to_string())
    }

    /// Get resource timing entries as JSON.
    #[napi]
    pub async fn get_resource_timing(&self) -> Result<String> {
        let guard: TokioMutexGuard<Option<onecrawl_cdp::Page>> = self.page.lock().await;
        let page = guard.as_ref().ok_or_else(|| Error::from_reason("no page"))?;
        let result = onecrawl_cdp::tracing_cdp::get_resource_timing(page)
            .await
            .map_err(|e| Error::from_reason(e.to_string()))?;
        serde_json::to_string(&result).map_err(|e| Error::from_reason(e.to_string()))
    }
}

fn parse_network_profile(name: &str) -> std::result::Result<onecrawl_cdp::NetworkProfile, String> {
    match name.to_lowercase().as_str() {
        "fast3g" | "fast-3g" => Ok(onecrawl_cdp::NetworkProfile::Fast3G),
        "slow3g" | "slow-3g" => Ok(onecrawl_cdp::NetworkProfile::Slow3G),
        "offline" => Ok(onecrawl_cdp::NetworkProfile::Offline),
        "regular4g" | "4g" => Ok(onecrawl_cdp::NetworkProfile::Regular4G),
        "wifi" => Ok(onecrawl_cdp::NetworkProfile::WiFi),
        _ => Err(format!("Unknown profile: {name}. Use: fast3g, slow3g, offline, regular4g, wifi")),
    }
}
