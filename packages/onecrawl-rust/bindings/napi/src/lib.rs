//! NAPI-RS bindings for the OneCrawl Rust workspace.
//!
//! Exposes crypto, parser, and storage functionality to Node.js.

#[macro_use]
extern crate napi_derive;

use base64::{Engine as _, engine::general_purpose::STANDARD as B64};
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
        let store = onecrawl_storage::EncryptedStore::open(std::path::Path::new(&path), &password)
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
    rate_limiter: Arc<TokioMutex<onecrawl_cdp::RateLimitState>>,
    retry_queue: Arc<TokioMutex<onecrawl_cdp::RetryQueue>>,
    scheduler: Arc<TokioMutex<onecrawl_cdp::Scheduler>>,
    session_pool: Arc<TokioMutex<onecrawl_cdp::SessionPool>>,
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
            rate_limiter: Arc::new(TokioMutex::new(onecrawl_cdp::RateLimitState::new(
                onecrawl_cdp::RateLimitConfig::default(),
            ))),
            retry_queue: Arc::new(TokioMutex::new(onecrawl_cdp::RetryQueue::new(
                onecrawl_cdp::RetryConfig::default(),
            ))),
            scheduler: Arc::new(TokioMutex::new(onecrawl_cdp::Scheduler::new())),
            session_pool: Arc::new(TokioMutex::new(onecrawl_cdp::SessionPool::new(
                onecrawl_cdp::PoolConfig::default(),
            ))),
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
            rate_limiter: Arc::new(TokioMutex::new(onecrawl_cdp::RateLimitState::new(
                onecrawl_cdp::RateLimitConfig::default(),
            ))),
            retry_queue: Arc::new(TokioMutex::new(onecrawl_cdp::RetryQueue::new(
                onecrawl_cdp::RetryConfig::default(),
            ))),
            scheduler: Arc::new(TokioMutex::new(onecrawl_cdp::Scheduler::new())),
            session_pool: Arc::new(TokioMutex::new(onecrawl_cdp::SessionPool::new(
                onecrawl_cdp::PoolConfig::default(),
            ))),
        })
    }

    /// Navigate to a URL.
    #[napi]
    pub async fn goto(&self, url: String) -> Result<()> {
        let guard: tokio::sync::MutexGuard<Option<onecrawl_cdp::Page>> = self.page.lock().await;
        let page = guard
            .as_ref()
            .ok_or_else(|| Error::from_reason("browser closed"))?;
        onecrawl_cdp::navigation::goto(page, &url)
            .await
            .map_err(|e| Error::from_reason(e.to_string()))
    }

    /// Get current page URL.
    #[napi]
    pub async fn get_url(&self) -> Result<String> {
        let guard: tokio::sync::MutexGuard<Option<onecrawl_cdp::Page>> = self.page.lock().await;
        let page = guard
            .as_ref()
            .ok_or_else(|| Error::from_reason("browser closed"))?;
        onecrawl_cdp::navigation::get_url(page)
            .await
            .map_err(|e| Error::from_reason(e.to_string()))
    }

    /// Get page title.
    #[napi]
    pub async fn get_title(&self) -> Result<String> {
        let guard: tokio::sync::MutexGuard<Option<onecrawl_cdp::Page>> = self.page.lock().await;
        let page = guard
            .as_ref()
            .ok_or_else(|| Error::from_reason("browser closed"))?;
        onecrawl_cdp::navigation::get_title(page)
            .await
            .map_err(|e| Error::from_reason(e.to_string()))
    }

    /// Get page HTML content.
    #[napi]
    pub async fn content(&self) -> Result<String> {
        let guard: tokio::sync::MutexGuard<Option<onecrawl_cdp::Page>> = self.page.lock().await;
        let page = guard
            .as_ref()
            .ok_or_else(|| Error::from_reason("browser closed"))?;
        onecrawl_cdp::page::get_content(page)
            .await
            .map_err(|e| Error::from_reason(e.to_string()))
    }

    /// Set page HTML content.
    #[napi]
    pub async fn set_content(&self, html: String) -> Result<()> {
        let guard: tokio::sync::MutexGuard<Option<onecrawl_cdp::Page>> = self.page.lock().await;
        let page = guard
            .as_ref()
            .ok_or_else(|| Error::from_reason("browser closed"))?;
        onecrawl_cdp::page::set_content(page, &html)
            .await
            .map_err(|e| Error::from_reason(e.to_string()))
    }

    /// Take a viewport screenshot (PNG). Returns raw bytes.
    #[napi]
    pub async fn screenshot(&self) -> Result<Buffer> {
        let guard: tokio::sync::MutexGuard<Option<onecrawl_cdp::Page>> = self.page.lock().await;
        let page = guard
            .as_ref()
            .ok_or_else(|| Error::from_reason("browser closed"))?;
        let bytes = onecrawl_cdp::screenshot::screenshot_viewport(page)
            .await
            .map_err(|e| Error::from_reason(e.to_string()))?;
        Ok(bytes.into())
    }

    /// Take a full-page screenshot (PNG). Returns raw bytes.
    #[napi]
    pub async fn screenshot_full(&self) -> Result<Buffer> {
        let guard: tokio::sync::MutexGuard<Option<onecrawl_cdp::Page>> = self.page.lock().await;
        let page = guard
            .as_ref()
            .ok_or_else(|| Error::from_reason("browser closed"))?;
        let bytes = onecrawl_cdp::screenshot::screenshot_full(page)
            .await
            .map_err(|e| Error::from_reason(e.to_string()))?;
        Ok(bytes.into())
    }

    /// Screenshot a specific element by CSS selector (PNG).
    #[napi]
    pub async fn screenshot_element(&self, selector: String) -> Result<Buffer> {
        let guard: tokio::sync::MutexGuard<Option<onecrawl_cdp::Page>> = self.page.lock().await;
        let page = guard
            .as_ref()
            .ok_or_else(|| Error::from_reason("browser closed"))?;
        let bytes = onecrawl_cdp::screenshot::screenshot_element(page, &selector)
            .await
            .map_err(|e| Error::from_reason(e.to_string()))?;
        Ok(bytes.into())
    }

    /// Save page as PDF. Returns raw bytes.
    #[napi]
    pub async fn pdf(&self) -> Result<Buffer> {
        let guard: tokio::sync::MutexGuard<Option<onecrawl_cdp::Page>> = self.page.lock().await;
        let page = guard
            .as_ref()
            .ok_or_else(|| Error::from_reason("browser closed"))?;
        let bytes = onecrawl_cdp::screenshot::pdf(page)
            .await
            .map_err(|e| Error::from_reason(e.to_string()))?;
        Ok(bytes.into())
    }

    /// Evaluate JavaScript in the page. Returns JSON string.
    #[napi]
    pub async fn evaluate(&self, expression: String) -> Result<String> {
        let guard: tokio::sync::MutexGuard<Option<onecrawl_cdp::Page>> = self.page.lock().await;
        let page = guard
            .as_ref()
            .ok_or_else(|| Error::from_reason("browser closed"))?;
        let val = onecrawl_cdp::page::evaluate_js(page, &expression)
            .await
            .map_err(|e| Error::from_reason(e.to_string()))?;
        Ok(val.to_string())
    }

    /// Click an element by CSS selector.
    #[napi]
    pub async fn click(&self, selector: String) -> Result<()> {
        let guard: tokio::sync::MutexGuard<Option<onecrawl_cdp::Page>> = self.page.lock().await;
        let page = guard
            .as_ref()
            .ok_or_else(|| Error::from_reason("browser closed"))?;
        onecrawl_cdp::element::click(page, &selector)
            .await
            .map_err(|e| Error::from_reason(e.to_string()))
    }

    /// Double-click an element by CSS selector.
    #[napi]
    pub async fn double_click(&self, selector: String) -> Result<()> {
        let guard: tokio::sync::MutexGuard<Option<onecrawl_cdp::Page>> = self.page.lock().await;
        let page = guard
            .as_ref()
            .ok_or_else(|| Error::from_reason("browser closed"))?;
        onecrawl_cdp::element::double_click(page, &selector)
            .await
            .map_err(|e| Error::from_reason(e.to_string()))
    }

    /// Type text into an element (key-by-key).
    #[napi(js_name = "type")]
    pub async fn type_text(&self, selector: String, text: String) -> Result<()> {
        let guard: tokio::sync::MutexGuard<Option<onecrawl_cdp::Page>> = self.page.lock().await;
        let page = guard
            .as_ref()
            .ok_or_else(|| Error::from_reason("browser closed"))?;
        onecrawl_cdp::element::type_text(page, &selector, &text)
            .await
            .map_err(|e| Error::from_reason(e.to_string()))
    }

    /// Get text content of an element.
    #[napi]
    pub async fn get_text(&self, selector: String) -> Result<String> {
        let guard: tokio::sync::MutexGuard<Option<onecrawl_cdp::Page>> = self.page.lock().await;
        let page = guard
            .as_ref()
            .ok_or_else(|| Error::from_reason("browser closed"))?;
        onecrawl_cdp::element::get_text(page, &selector)
            .await
            .map_err(|e| Error::from_reason(e.to_string()))
    }

    /// Get an attribute value from an element.
    #[napi]
    pub async fn get_attribute(
        &self,
        selector: String,
        attribute: String,
    ) -> Result<Option<String>> {
        let guard: tokio::sync::MutexGuard<Option<onecrawl_cdp::Page>> = self.page.lock().await;
        let page = guard
            .as_ref()
            .ok_or_else(|| Error::from_reason("browser closed"))?;
        onecrawl_cdp::element::get_attribute(page, &selector, &attribute)
            .await
            .map_err(|e| Error::from_reason(e.to_string()))
    }

    /// Hover over an element.
    #[napi]
    pub async fn hover(&self, selector: String) -> Result<()> {
        let guard: tokio::sync::MutexGuard<Option<onecrawl_cdp::Page>> = self.page.lock().await;
        let page = guard
            .as_ref()
            .ok_or_else(|| Error::from_reason("browser closed"))?;
        onecrawl_cdp::element::hover(page, &selector)
            .await
            .map_err(|e| Error::from_reason(e.to_string()))
    }

    /// Scroll an element into view.
    #[napi]
    pub async fn scroll_into_view(&self, selector: String) -> Result<()> {
        let guard: tokio::sync::MutexGuard<Option<onecrawl_cdp::Page>> = self.page.lock().await;
        let page = guard
            .as_ref()
            .ok_or_else(|| Error::from_reason("browser closed"))?;
        onecrawl_cdp::element::scroll_into_view(page, &selector)
            .await
            .map_err(|e| Error::from_reason(e.to_string()))
    }

    /// Check a checkbox.
    #[napi]
    pub async fn check(&self, selector: String) -> Result<()> {
        let guard: tokio::sync::MutexGuard<Option<onecrawl_cdp::Page>> = self.page.lock().await;
        let page = guard
            .as_ref()
            .ok_or_else(|| Error::from_reason("browser closed"))?;
        onecrawl_cdp::element::check(page, &selector)
            .await
            .map_err(|e| Error::from_reason(e.to_string()))
    }

    /// Uncheck a checkbox.
    #[napi]
    pub async fn uncheck(&self, selector: String) -> Result<()> {
        let guard: tokio::sync::MutexGuard<Option<onecrawl_cdp::Page>> = self.page.lock().await;
        let page = guard
            .as_ref()
            .ok_or_else(|| Error::from_reason("browser closed"))?;
        onecrawl_cdp::element::uncheck(page, &selector)
            .await
            .map_err(|e| Error::from_reason(e.to_string()))
    }

    /// Select an option in a `<select>` element by value.
    #[napi]
    pub async fn select_option(&self, selector: String, value: String) -> Result<()> {
        let guard: tokio::sync::MutexGuard<Option<onecrawl_cdp::Page>> = self.page.lock().await;
        let page = guard
            .as_ref()
            .ok_or_else(|| Error::from_reason("browser closed"))?;
        onecrawl_cdp::element::select_option(page, &selector, &value)
            .await
            .map_err(|e| Error::from_reason(e.to_string()))
    }

    /// Wait for a selector to appear (timeout in ms).
    #[napi]
    pub async fn wait_for_selector(&self, selector: String, timeout_ms: Option<u32>) -> Result<()> {
        let guard: tokio::sync::MutexGuard<Option<onecrawl_cdp::Page>> = self.page.lock().await;
        let page = guard
            .as_ref()
            .ok_or_else(|| Error::from_reason("browser closed"))?;
        onecrawl_cdp::navigation::wait_for_selector(
            page,
            &selector,
            timeout_ms.unwrap_or(30000) as u64,
        )
        .await
        .map_err(|e| Error::from_reason(e.to_string()))
    }

    /// Wait for URL to contain a pattern (timeout in ms).
    #[napi]
    pub async fn wait_for_url(&self, pattern: String, timeout_ms: Option<u32>) -> Result<()> {
        let guard: tokio::sync::MutexGuard<Option<onecrawl_cdp::Page>> = self.page.lock().await;
        let page = guard
            .as_ref()
            .ok_or_else(|| Error::from_reason("browser closed"))?;
        onecrawl_cdp::navigation::wait_for_url(page, &pattern, timeout_ms.unwrap_or(30000) as u64)
            .await
            .map_err(|e| Error::from_reason(e.to_string()))
    }

    /// Go back in history.
    #[napi]
    pub async fn go_back(&self) -> Result<()> {
        let guard: tokio::sync::MutexGuard<Option<onecrawl_cdp::Page>> = self.page.lock().await;
        let page = guard
            .as_ref()
            .ok_or_else(|| Error::from_reason("browser closed"))?;
        onecrawl_cdp::navigation::go_back(page)
            .await
            .map_err(|e| Error::from_reason(e.to_string()))
    }

    /// Go forward in history.
    #[napi]
    pub async fn go_forward(&self) -> Result<()> {
        let guard: tokio::sync::MutexGuard<Option<onecrawl_cdp::Page>> = self.page.lock().await;
        let page = guard
            .as_ref()
            .ok_or_else(|| Error::from_reason("browser closed"))?;
        onecrawl_cdp::navigation::go_forward(page)
            .await
            .map_err(|e| Error::from_reason(e.to_string()))
    }

    /// Reload the page.
    #[napi]
    pub async fn reload(&self) -> Result<()> {
        let guard: tokio::sync::MutexGuard<Option<onecrawl_cdp::Page>> = self.page.lock().await;
        let page = guard
            .as_ref()
            .ok_or_else(|| Error::from_reason("browser closed"))?;
        onecrawl_cdp::navigation::reload(page)
            .await
            .map_err(|e| Error::from_reason(e.to_string()))
    }

    /// Inject stealth anti-detection patches. Returns the fingerprint used.
    #[napi]
    pub async fn inject_stealth(&self) -> Result<FingerprintInfo> {
        let guard: tokio::sync::MutexGuard<Option<onecrawl_cdp::Page>> = self.page.lock().await;
        let page = guard
            .as_ref()
            .ok_or_else(|| Error::from_reason("browser closed"))?;
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
        let new_page = self
            .session
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
        let page = guard
            .as_ref()
            .ok_or_else(|| Error::from_reason("browser closed"))?;
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
        let page = guard
            .as_ref()
            .ok_or_else(|| Error::from_reason("browser closed"))?;
        let val = onecrawl_cdp::network::get_intercepted_requests(page)
            .await
            .map_err(|e| Error::from_reason(e.to_string()))?;
        Ok(val.to_string())
    }

    /// Get intercepted responses (after `observeNetwork`). Returns JSON string.
    #[napi]
    pub async fn get_responses(&self) -> Result<String> {
        let guard: tokio::sync::MutexGuard<Option<onecrawl_cdp::Page>> = self.page.lock().await;
        let page = guard
            .as_ref()
            .ok_or_else(|| Error::from_reason("browser closed"))?;
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
        let page = guard
            .as_ref()
            .ok_or_else(|| Error::from_reason("browser closed"))?;
        let cookies = onecrawl_cdp::cookie::get_all_cookies(page)
            .await
            .map_err(|e| Error::from_reason(e.to_string()))?;
        serde_json::to_string(&cookies).map_err(|e| Error::from_reason(e.to_string()))
    }

    /// Set a cookie. Accepts a JSON string of SetCookieParams.
    #[napi]
    pub async fn set_cookie(&self, params_json: String) -> Result<()> {
        let guard: tokio::sync::MutexGuard<Option<onecrawl_cdp::Page>> = self.page.lock().await;
        let page = guard
            .as_ref()
            .ok_or_else(|| Error::from_reason("browser closed"))?;
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
        let page = guard
            .as_ref()
            .ok_or_else(|| Error::from_reason("browser closed"))?;
        onecrawl_cdp::cookie::delete_cookies(page, &name, domain.as_deref(), path.as_deref())
            .await
            .map_err(|e| Error::from_reason(e.to_string()))
    }

    /// Clear all browser cookies.
    #[napi]
    pub async fn clear_cookies(&self) -> Result<()> {
        let guard: tokio::sync::MutexGuard<Option<onecrawl_cdp::Page>> = self.page.lock().await;
        let page = guard
            .as_ref()
            .ok_or_else(|| Error::from_reason("browser closed"))?;
        onecrawl_cdp::cookie::clear_cookies(page)
            .await
            .map_err(|e| Error::from_reason(e.to_string()))
    }

    // ──────────────── Keyboard ────────────────

    /// Press a key (keyDown + keyUp).
    #[napi]
    pub async fn press_key(&self, key: String) -> Result<()> {
        let guard: tokio::sync::MutexGuard<Option<onecrawl_cdp::Page>> = self.page.lock().await;
        let page = guard
            .as_ref()
            .ok_or_else(|| Error::from_reason("browser closed"))?;
        onecrawl_cdp::keyboard::press_key(page, &key)
            .await
            .map_err(|e| Error::from_reason(e.to_string()))
    }

    /// Send a keyboard shortcut (e.g., "Control+a", "Meta+c").
    #[napi]
    pub async fn keyboard_shortcut(&self, shortcut: String) -> Result<()> {
        let guard: tokio::sync::MutexGuard<Option<onecrawl_cdp::Page>> = self.page.lock().await;
        let page = guard
            .as_ref()
            .ok_or_else(|| Error::from_reason("browser closed"))?;
        onecrawl_cdp::keyboard::keyboard_shortcut(page, &shortcut)
            .await
            .map_err(|e| Error::from_reason(e.to_string()))
    }

    /// Hold a key down.
    #[napi]
    pub async fn key_down(&self, key: String) -> Result<()> {
        let guard: tokio::sync::MutexGuard<Option<onecrawl_cdp::Page>> = self.page.lock().await;
        let page = guard
            .as_ref()
            .ok_or_else(|| Error::from_reason("browser closed"))?;
        onecrawl_cdp::keyboard::key_down(page, &key)
            .await
            .map_err(|e| Error::from_reason(e.to_string()))
    }

    /// Release a key.
    #[napi]
    pub async fn key_up(&self, key: String) -> Result<()> {
        let guard: tokio::sync::MutexGuard<Option<onecrawl_cdp::Page>> = self.page.lock().await;
        let page = guard
            .as_ref()
            .ok_or_else(|| Error::from_reason("browser closed"))?;
        onecrawl_cdp::keyboard::key_up(page, &key)
            .await
            .map_err(|e| Error::from_reason(e.to_string()))
    }

    /// Fill an input field (clear + set value + fire events).
    #[napi]
    pub async fn fill(&self, selector: String, value: String) -> Result<()> {
        let guard: tokio::sync::MutexGuard<Option<onecrawl_cdp::Page>> = self.page.lock().await;
        let page = guard
            .as_ref()
            .ok_or_else(|| Error::from_reason("browser closed"))?;
        onecrawl_cdp::keyboard::fill(page, &selector, &value)
            .await
            .map_err(|e| Error::from_reason(e.to_string()))
    }

    // ──────────────── Advanced Input ────────────────

    /// Drag an element and drop onto another (CSS selectors).
    #[napi]
    pub async fn drag_and_drop(&self, source: String, target: String) -> Result<()> {
        let guard: tokio::sync::MutexGuard<Option<onecrawl_cdp::Page>> = self.page.lock().await;
        let page = guard
            .as_ref()
            .ok_or_else(|| Error::from_reason("browser closed"))?;
        onecrawl_cdp::input::drag_and_drop(page, &source, &target)
            .await
            .map_err(|e| Error::from_reason(e.to_string()))
    }

    /// Upload files to a `<input type="file">` element.
    #[napi]
    pub async fn upload_file(&self, selector: String, file_paths: Vec<String>) -> Result<()> {
        let guard: tokio::sync::MutexGuard<Option<onecrawl_cdp::Page>> = self.page.lock().await;
        let page = guard
            .as_ref()
            .ok_or_else(|| Error::from_reason("browser closed"))?;
        onecrawl_cdp::input::set_file_input(page, &selector, &file_paths)
            .await
            .map_err(|e| Error::from_reason(e.to_string()))
    }

    /// Get the bounding box of an element. Returns { x, y, width, height }.
    #[napi]
    pub async fn bounding_box(&self, selector: String) -> Result<String> {
        let guard: tokio::sync::MutexGuard<Option<onecrawl_cdp::Page>> = self.page.lock().await;
        let page = guard
            .as_ref()
            .ok_or_else(|| Error::from_reason("browser closed"))?;
        let (x, y, w, h) = onecrawl_cdp::input::bounding_box(page, &selector)
            .await
            .map_err(|e| Error::from_reason(e.to_string()))?;
        Ok(serde_json::json!({"x": x, "y": y, "width": w, "height": h}).to_string())
    }

    /// Tap an element (touch simulation).
    #[napi]
    pub async fn tap(&self, selector: String) -> Result<()> {
        let guard: tokio::sync::MutexGuard<Option<onecrawl_cdp::Page>> = self.page.lock().await;
        let page = guard
            .as_ref()
            .ok_or_else(|| Error::from_reason("browser closed"))?;
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
        let page = guard
            .as_ref()
            .ok_or_else(|| Error::from_reason("browser closed"))?;
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
        let page = guard
            .as_ref()
            .ok_or_else(|| Error::from_reason("browser closed"))?;
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
        let page = guard
            .as_ref()
            .ok_or_else(|| Error::from_reason("browser closed"))?;
        onecrawl_cdp::emulation::clear_viewport(page)
            .await
            .map_err(|e| Error::from_reason(e.to_string()))
    }

    /// Override the browser user agent string.
    #[napi]
    pub async fn set_user_agent(&self, user_agent: String) -> Result<()> {
        let guard: tokio::sync::MutexGuard<Option<onecrawl_cdp::Page>> = self.page.lock().await;
        let page = guard
            .as_ref()
            .ok_or_else(|| Error::from_reason("browser closed"))?;
        onecrawl_cdp::emulation::set_user_agent(page, &user_agent)
            .await
            .map_err(|e| Error::from_reason(e.to_string()))
    }

    /// Set geolocation override.
    #[napi]
    pub async fn set_geolocation(
        &self,
        latitude: f64,
        longitude: f64,
        accuracy: Option<f64>,
    ) -> Result<()> {
        let guard: tokio::sync::MutexGuard<Option<onecrawl_cdp::Page>> = self.page.lock().await;
        let page = guard
            .as_ref()
            .ok_or_else(|| Error::from_reason("browser closed"))?;
        onecrawl_cdp::emulation::set_geolocation(page, latitude, longitude, accuracy.unwrap_or(1.0))
            .await
            .map_err(|e| Error::from_reason(e.to_string()))
    }

    /// Emulate color scheme preference (dark/light).
    #[napi]
    pub async fn set_color_scheme(&self, scheme: String) -> Result<()> {
        let guard: tokio::sync::MutexGuard<Option<onecrawl_cdp::Page>> = self.page.lock().await;
        let page = guard
            .as_ref()
            .ok_or_else(|| Error::from_reason("browser closed"))?;
        onecrawl_cdp::emulation::set_color_scheme(page, &scheme)
            .await
            .map_err(|e| Error::from_reason(e.to_string()))
    }

    // ──── Network (advanced) ────

    /// Block specific resource types (e.g., ["Image", "Font", "Stylesheet"]).
    #[napi]
    pub async fn block_resources(&self, resource_types: Vec<String>) -> Result<()> {
        let guard: tokio::sync::MutexGuard<Option<onecrawl_cdp::Page>> = self.page.lock().await;
        let page = guard
            .as_ref()
            .ok_or_else(|| Error::from_reason("browser closed"))?;
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
        let page = guard
            .as_ref()
            .ok_or_else(|| Error::from_reason("browser closed"))?;
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
        let page = guard
            .as_ref()
            .ok_or_else(|| Error::from_reason("browser closed"))?;
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
        let page = guard
            .as_ref()
            .ok_or_else(|| Error::from_reason("browser closed"))?;

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
        let page = guard
            .as_ref()
            .ok_or_else(|| Error::from_reason("browser closed"))?;

        let es = self.event_stream.lock().await;
        let stream = es.as_ref().ok_or_else(|| {
            Error::from_reason("event stream not started — call startEventStream() first")
        })?;
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
        })
        .to_string())
    }

    /// Emit a custom event into the stream.
    #[napi]
    pub async fn emit_event(&self, name: String, data: String) -> Result<()> {
        let es = self.event_stream.lock().await;
        let stream = es
            .as_ref()
            .ok_or_else(|| Error::from_reason("event stream not started"))?;
        let tx = stream.sender();
        let json_data: serde_json::Value =
            serde_json::from_str(&data).unwrap_or(serde_json::Value::String(data));
        onecrawl_cdp::events::emit_custom(&tx, &name, json_data)
            .map_err(|e| Error::from_reason(e.to_string()))
    }

    // ── HAR Recording ──────────────────────────────────────────────

    /// Start HAR (HTTP Archive) recording on the current page.
    #[napi]
    pub async fn start_har_recording(&self) -> Result<()> {
        let guard: TokioMutexGuard<Option<onecrawl_cdp::Page>> = self.page.lock().await;
        let page = guard
            .as_ref()
            .ok_or_else(|| Error::from_reason("no page"))?;
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
        let page = guard
            .as_ref()
            .ok_or_else(|| Error::from_reason("no page"))?;
        let hr = self.har_recorder.lock().await;
        let recorder = hr
            .as_ref()
            .ok_or_else(|| Error::from_reason("HAR recording not started"))?;
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
        let recorder = hr
            .as_ref()
            .ok_or_else(|| Error::from_reason("HAR recording not started"))?;
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
        let page = guard
            .as_ref()
            .ok_or_else(|| Error::from_reason("no page"))?;
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
        let page = guard
            .as_ref()
            .ok_or_else(|| Error::from_reason("no page"))?;
        let wr = self.ws_recorder.lock().await;
        let recorder = wr
            .as_ref()
            .ok_or_else(|| Error::from_reason("WS recording not started"))?;
        let count = onecrawl_cdp::websocket::drain_ws_frames(page, recorder)
            .await
            .map_err(|e| Error::from_reason(e.to_string()))?;
        Ok(count as u32)
    }

    /// Export all captured WebSocket frames as JSON string.
    #[napi]
    pub async fn export_ws_frames(&self) -> Result<String> {
        let wr = self.ws_recorder.lock().await;
        let recorder = wr
            .as_ref()
            .ok_or_else(|| Error::from_reason("WS recording not started"))?;
        let frames = onecrawl_cdp::websocket::export_ws_frames(recorder)
            .await
            .map_err(|e| Error::from_reason(e.to_string()))?;
        Ok(frames.to_string())
    }

    /// Get the count of active WebSocket connections.
    #[napi]
    pub async fn active_ws_connections(&self) -> Result<u32> {
        let guard: TokioMutexGuard<Option<onecrawl_cdp::Page>> = self.page.lock().await;
        let page = guard
            .as_ref()
            .ok_or_else(|| Error::from_reason("no page"))?;
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
        let page = guard
            .as_ref()
            .ok_or_else(|| Error::from_reason("no page"))?;
        onecrawl_cdp::console::start_console_capture(page)
            .await
            .map_err(|e| Error::from_reason(e.to_string()))
    }

    /// Drain captured console entries as JSON string.
    #[napi]
    pub async fn drain_console_entries(&self) -> Result<String> {
        let guard: TokioMutexGuard<Option<onecrawl_cdp::Page>> = self.page.lock().await;
        let page = guard
            .as_ref()
            .ok_or_else(|| Error::from_reason("no page"))?;
        let entries = onecrawl_cdp::console::drain_console_entries(page)
            .await
            .map_err(|e| Error::from_reason(e.to_string()))?;
        serde_json::to_string(&entries).map_err(|e| Error::from_reason(e.to_string()))
    }

    /// Clear the console capture buffer.
    #[napi]
    pub async fn clear_console(&self) -> Result<()> {
        let guard: TokioMutexGuard<Option<onecrawl_cdp::Page>> = self.page.lock().await;
        let page = guard
            .as_ref()
            .ok_or_else(|| Error::from_reason("no page"))?;
        onecrawl_cdp::console::clear_console(page)
            .await
            .map_err(|e| Error::from_reason(e.to_string()))
    }

    // ── Dialog Handling ────────────────────────────────────────────

    /// Set dialog auto-handler (alert/confirm/prompt).
    #[napi]
    pub async fn set_dialog_handler(
        &self,
        accept: bool,
        prompt_text: Option<String>,
    ) -> Result<()> {
        let guard: TokioMutexGuard<Option<onecrawl_cdp::Page>> = self.page.lock().await;
        let page = guard
            .as_ref()
            .ok_or_else(|| Error::from_reason("no page"))?;
        onecrawl_cdp::dialog::set_dialog_handler(page, accept, prompt_text.as_deref())
            .await
            .map_err(|e| Error::from_reason(e.to_string()))
    }

    /// Get dialog history as JSON string.
    #[napi]
    pub async fn get_dialog_history(&self) -> Result<String> {
        let guard: TokioMutexGuard<Option<onecrawl_cdp::Page>> = self.page.lock().await;
        let page = guard
            .as_ref()
            .ok_or_else(|| Error::from_reason("no page"))?;
        let events = onecrawl_cdp::dialog::get_dialog_history(page)
            .await
            .map_err(|e| Error::from_reason(e.to_string()))?;
        serde_json::to_string(&events).map_err(|e| Error::from_reason(e.to_string()))
    }

    /// Clear dialog history.
    #[napi]
    pub async fn clear_dialog_history(&self) -> Result<()> {
        let guard: TokioMutexGuard<Option<onecrawl_cdp::Page>> = self.page.lock().await;
        let page = guard
            .as_ref()
            .ok_or_else(|| Error::from_reason("no page"))?;
        onecrawl_cdp::dialog::clear_dialog_history(page)
            .await
            .map_err(|e| Error::from_reason(e.to_string()))
    }

    // ── Service Workers ────────────────────────────────────────────

    /// Get all registered service workers as JSON string.
    #[napi]
    pub async fn get_service_workers(&self) -> Result<String> {
        let guard: TokioMutexGuard<Option<onecrawl_cdp::Page>> = self.page.lock().await;
        let page = guard
            .as_ref()
            .ok_or_else(|| Error::from_reason("no page"))?;
        let workers = onecrawl_cdp::workers::get_service_workers(page)
            .await
            .map_err(|e| Error::from_reason(e.to_string()))?;
        serde_json::to_string(&workers).map_err(|e| Error::from_reason(e.to_string()))
    }

    /// Unregister all service workers. Returns the number unregistered.
    #[napi]
    pub async fn unregister_service_workers(&self) -> Result<u32> {
        let guard: TokioMutexGuard<Option<onecrawl_cdp::Page>> = self.page.lock().await;
        let page = guard
            .as_ref()
            .ok_or_else(|| Error::from_reason("no page"))?;
        let count = onecrawl_cdp::workers::unregister_service_workers(page)
            .await
            .map_err(|e| Error::from_reason(e.to_string()))?;
        Ok(count as u32)
    }

    /// Get worker info as JSON string.
    #[napi]
    pub async fn get_worker_info(&self) -> Result<String> {
        let guard: TokioMutexGuard<Option<onecrawl_cdp::Page>> = self.page.lock().await;
        let page = guard
            .as_ref()
            .ok_or_else(|| Error::from_reason("no page"))?;
        let info = onecrawl_cdp::workers::get_worker_info(page)
            .await
            .map_err(|e| Error::from_reason(e.to_string()))?;
        Ok(info.to_string())
    }

    // ── DOM Observer ───────────────────────────────────────────────

    /// Start observing DOM mutations (optional CSS selector target).
    #[napi]
    pub async fn start_dom_observer(&self, selector: Option<String>) -> Result<()> {
        let guard: TokioMutexGuard<Option<onecrawl_cdp::Page>> = self.page.lock().await;
        let page = guard
            .as_ref()
            .ok_or_else(|| Error::from_reason("no page"))?;
        onecrawl_cdp::dom_observer::start_dom_observer(page, selector.as_deref())
            .await
            .map_err(|e| Error::from_reason(e.to_string()))
    }

    /// Drain accumulated DOM mutations as JSON string.
    #[napi]
    pub async fn drain_dom_mutations(&self) -> Result<String> {
        let guard: TokioMutexGuard<Option<onecrawl_cdp::Page>> = self.page.lock().await;
        let page = guard
            .as_ref()
            .ok_or_else(|| Error::from_reason("no page"))?;
        let mutations = onecrawl_cdp::dom_observer::drain_dom_mutations(page)
            .await
            .map_err(|e| Error::from_reason(e.to_string()))?;
        serde_json::to_string(&mutations).map_err(|e| Error::from_reason(e.to_string()))
    }

    /// Stop the DOM observer.
    #[napi]
    pub async fn stop_dom_observer(&self) -> Result<()> {
        let guard: TokioMutexGuard<Option<onecrawl_cdp::Page>> = self.page.lock().await;
        let page = guard
            .as_ref()
            .ok_or_else(|| Error::from_reason("no page"))?;
        onecrawl_cdp::dom_observer::stop_dom_observer(page)
            .await
            .map_err(|e| Error::from_reason(e.to_string()))
    }

    /// Get a snapshot of the current DOM as HTML string.
    #[napi]
    pub async fn get_dom_snapshot(&self, selector: Option<String>) -> Result<String> {
        let guard: TokioMutexGuard<Option<onecrawl_cdp::Page>> = self.page.lock().await;
        let page = guard
            .as_ref()
            .ok_or_else(|| Error::from_reason("no page"))?;
        onecrawl_cdp::dom_observer::get_dom_snapshot(page, selector.as_deref())
            .await
            .map_err(|e| Error::from_reason(e.to_string()))
    }

    // ── Iframe Management ──────────────────────────────────────────

    /// List all iframes on the page as JSON string.
    #[napi]
    pub async fn list_iframes(&self) -> Result<String> {
        let guard: TokioMutexGuard<Option<onecrawl_cdp::Page>> = self.page.lock().await;
        let page = guard
            .as_ref()
            .ok_or_else(|| Error::from_reason("no page"))?;
        let iframes = onecrawl_cdp::iframe::list_iframes(page)
            .await
            .map_err(|e| Error::from_reason(e.to_string()))?;
        serde_json::to_string(&iframes).map_err(|e| Error::from_reason(e.to_string()))
    }

    /// Execute JavaScript inside a specific iframe by index. Returns JSON string.
    #[napi]
    pub async fn eval_in_iframe(&self, index: u32, expression: String) -> Result<String> {
        let guard: TokioMutexGuard<Option<onecrawl_cdp::Page>> = self.page.lock().await;
        let page = guard
            .as_ref()
            .ok_or_else(|| Error::from_reason("no page"))?;
        let val = onecrawl_cdp::iframe::eval_in_iframe(page, index as usize, &expression)
            .await
            .map_err(|e| Error::from_reason(e.to_string()))?;
        serde_json::to_string(&val).map_err(|e| Error::from_reason(e.to_string()))
    }

    /// Get the inner HTML content of an iframe by index.
    #[napi]
    pub async fn get_iframe_content(&self, index: u32) -> Result<String> {
        let guard: TokioMutexGuard<Option<onecrawl_cdp::Page>> = self.page.lock().await;
        let page = guard
            .as_ref()
            .ok_or_else(|| Error::from_reason("no page"))?;
        onecrawl_cdp::iframe::get_iframe_content(page, index as usize)
            .await
            .map_err(|e| Error::from_reason(e.to_string()))
    }

    // ── Print / PDF (Enhanced) ─────────────────────────────────────

    /// Generate PDF with detailed options (JSON string of DetailedPdfOptions). Returns base64 PDF data.
    #[napi]
    pub async fn print_to_pdf(&self, options: Option<String>) -> Result<Buffer> {
        let guard: TokioMutexGuard<Option<onecrawl_cdp::Page>> = self.page.lock().await;
        let page = guard
            .as_ref()
            .ok_or_else(|| Error::from_reason("no page"))?;
        let opts: onecrawl_cdp::DetailedPdfOptions = match options {
            Some(ref json) => {
                serde_json::from_str(json).map_err(|e| Error::from_reason(e.to_string()))?
            }
            None => Default::default(),
        };
        let bytes = onecrawl_cdp::print::print_to_pdf(page, &opts)
            .await
            .map_err(|e| Error::from_reason(e.to_string()))?;
        Ok(bytes.into())
    }

    /// Get page print preview metrics as JSON string.
    #[napi]
    pub async fn get_print_metrics(&self) -> Result<String> {
        let guard: TokioMutexGuard<Option<onecrawl_cdp::Page>> = self.page.lock().await;
        let page = guard
            .as_ref()
            .ok_or_else(|| Error::from_reason("no page"))?;
        let val = onecrawl_cdp::print::get_print_metrics(page)
            .await
            .map_err(|e| Error::from_reason(e.to_string()))?;
        serde_json::to_string(&val).map_err(|e| Error::from_reason(e.to_string()))
    }

    // ── Web Storage ────────────────────────────────────────────────

    /// Get all localStorage contents as JSON string.
    #[napi]
    pub async fn get_local_storage(&self) -> Result<String> {
        let guard: TokioMutexGuard<Option<onecrawl_cdp::Page>> = self.page.lock().await;
        let page = guard
            .as_ref()
            .ok_or_else(|| Error::from_reason("no page"))?;
        let data = onecrawl_cdp::web_storage::get_local_storage(page)
            .await
            .map_err(|e| Error::from_reason(e.to_string()))?;
        Ok(data.to_string())
    }

    /// Set a localStorage item.
    #[napi]
    pub async fn set_local_storage(&self, key: String, value: String) -> Result<()> {
        let guard: TokioMutexGuard<Option<onecrawl_cdp::Page>> = self.page.lock().await;
        let page = guard
            .as_ref()
            .ok_or_else(|| Error::from_reason("no page"))?;
        onecrawl_cdp::web_storage::set_local_storage(page, &key, &value)
            .await
            .map_err(|e| Error::from_reason(e.to_string()))
    }

    /// Clear all localStorage.
    #[napi]
    pub async fn clear_local_storage(&self) -> Result<()> {
        let guard: TokioMutexGuard<Option<onecrawl_cdp::Page>> = self.page.lock().await;
        let page = guard
            .as_ref()
            .ok_or_else(|| Error::from_reason("no page"))?;
        onecrawl_cdp::web_storage::clear_local_storage(page)
            .await
            .map_err(|e| Error::from_reason(e.to_string()))
    }

    /// Get all sessionStorage contents as JSON string.
    #[napi]
    pub async fn get_session_storage(&self) -> Result<String> {
        let guard: TokioMutexGuard<Option<onecrawl_cdp::Page>> = self.page.lock().await;
        let page = guard
            .as_ref()
            .ok_or_else(|| Error::from_reason("no page"))?;
        let data = onecrawl_cdp::web_storage::get_session_storage(page)
            .await
            .map_err(|e| Error::from_reason(e.to_string()))?;
        Ok(data.to_string())
    }

    /// Set a sessionStorage item.
    #[napi]
    pub async fn set_session_storage(&self, key: String, value: String) -> Result<()> {
        let guard: TokioMutexGuard<Option<onecrawl_cdp::Page>> = self.page.lock().await;
        let page = guard
            .as_ref()
            .ok_or_else(|| Error::from_reason("no page"))?;
        onecrawl_cdp::web_storage::set_session_storage(page, &key, &value)
            .await
            .map_err(|e| Error::from_reason(e.to_string()))
    }

    /// Clear all sessionStorage.
    #[napi]
    pub async fn clear_session_storage(&self) -> Result<()> {
        let guard: TokioMutexGuard<Option<onecrawl_cdp::Page>> = self.page.lock().await;
        let page = guard
            .as_ref()
            .ok_or_else(|| Error::from_reason("no page"))?;
        onecrawl_cdp::web_storage::clear_session_storage(page)
            .await
            .map_err(|e| Error::from_reason(e.to_string()))
    }

    /// Get IndexedDB database names as JSON string.
    #[napi]
    pub async fn get_indexeddb_databases(&self) -> Result<String> {
        let guard: TokioMutexGuard<Option<onecrawl_cdp::Page>> = self.page.lock().await;
        let page = guard
            .as_ref()
            .ok_or_else(|| Error::from_reason("no page"))?;
        let names = onecrawl_cdp::web_storage::get_indexeddb_databases(page)
            .await
            .map_err(|e| Error::from_reason(e.to_string()))?;
        serde_json::to_string(&names).map_err(|e| Error::from_reason(e.to_string()))
    }

    /// Clear all site data (localStorage + sessionStorage + cookies + cache).
    #[napi]
    pub async fn clear_site_data(&self) -> Result<()> {
        let guard: TokioMutexGuard<Option<onecrawl_cdp::Page>> = self.page.lock().await;
        let page = guard
            .as_ref()
            .ok_or_else(|| Error::from_reason("no page"))?;
        onecrawl_cdp::web_storage::clear_site_data(page)
            .await
            .map_err(|e| Error::from_reason(e.to_string()))
    }

    // ── Code Coverage ──────────────────────────────────────────────

    /// Start JavaScript code coverage collection via CDP Profiler.
    #[napi]
    pub async fn start_js_coverage(&self) -> Result<()> {
        let guard: TokioMutexGuard<Option<onecrawl_cdp::Page>> = self.page.lock().await;
        let page = guard
            .as_ref()
            .ok_or_else(|| Error::from_reason("no page"))?;
        onecrawl_cdp::coverage::start_js_coverage(page)
            .await
            .map_err(|e| Error::from_reason(e.to_string()))
    }

    /// Stop JavaScript code coverage and return the report as JSON string.
    #[napi]
    pub async fn stop_js_coverage(&self) -> Result<String> {
        let guard: TokioMutexGuard<Option<onecrawl_cdp::Page>> = self.page.lock().await;
        let page = guard
            .as_ref()
            .ok_or_else(|| Error::from_reason("no page"))?;
        let report = onecrawl_cdp::coverage::stop_js_coverage(page)
            .await
            .map_err(|e| Error::from_reason(e.to_string()))?;
        serde_json::to_string(&report).map_err(|e| Error::from_reason(e.to_string()))
    }

    /// Start CSS coverage collection.
    #[napi]
    pub async fn start_css_coverage(&self) -> Result<()> {
        let guard: TokioMutexGuard<Option<onecrawl_cdp::Page>> = self.page.lock().await;
        let page = guard
            .as_ref()
            .ok_or_else(|| Error::from_reason("no page"))?;
        onecrawl_cdp::coverage::start_css_coverage(page)
            .await
            .map_err(|e| Error::from_reason(e.to_string()))
    }

    /// Get CSS coverage summary as JSON string.
    #[napi]
    pub async fn get_css_coverage(&self) -> Result<String> {
        let guard: TokioMutexGuard<Option<onecrawl_cdp::Page>> = self.page.lock().await;
        let page = guard
            .as_ref()
            .ok_or_else(|| Error::from_reason("no page"))?;
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
        let page = guard
            .as_ref()
            .ok_or_else(|| Error::from_reason("no page"))?;
        let result = onecrawl_cdp::accessibility::get_accessibility_tree(page)
            .await
            .map_err(|e| Error::from_reason(e.to_string()))?;
        Ok(result.to_string())
    }

    /// Get accessibility info for a specific element by CSS selector.
    #[napi]
    pub async fn get_element_accessibility(&self, selector: String) -> Result<String> {
        let guard: TokioMutexGuard<Option<onecrawl_cdp::Page>> = self.page.lock().await;
        let page = guard
            .as_ref()
            .ok_or_else(|| Error::from_reason("no page"))?;
        let result = onecrawl_cdp::accessibility::get_element_accessibility(page, &selector)
            .await
            .map_err(|e| Error::from_reason(e.to_string()))?;
        Ok(result.to_string())
    }

    /// Run an accessibility audit on the current page and return the report as JSON.
    #[napi]
    pub async fn audit_accessibility(&self) -> Result<String> {
        let guard: TokioMutexGuard<Option<onecrawl_cdp::Page>> = self.page.lock().await;
        let page = guard
            .as_ref()
            .ok_or_else(|| Error::from_reason("no page"))?;
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
        let page = guard
            .as_ref()
            .ok_or_else(|| Error::from_reason("no page"))?;
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
        let page = guard
            .as_ref()
            .ok_or_else(|| Error::from_reason("no page"))?;
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
        let page = guard
            .as_ref()
            .ok_or_else(|| Error::from_reason("no page"))?;
        onecrawl_cdp::throttle::clear_network_conditions(page)
            .await
            .map_err(|e| Error::from_reason(e.to_string()))
    }

    // ── Performance Tracing ────────────────────────────────────────

    /// Start performance tracing on the current page.
    #[napi]
    pub async fn start_tracing(&self) -> Result<()> {
        let guard: TokioMutexGuard<Option<onecrawl_cdp::Page>> = self.page.lock().await;
        let page = guard
            .as_ref()
            .ok_or_else(|| Error::from_reason("no page"))?;
        onecrawl_cdp::tracing_cdp::start_tracing(page)
            .await
            .map_err(|e| Error::from_reason(e.to_string()))
    }

    /// Stop tracing and return the trace data as JSON.
    #[napi]
    pub async fn stop_tracing(&self) -> Result<String> {
        let guard: TokioMutexGuard<Option<onecrawl_cdp::Page>> = self.page.lock().await;
        let page = guard
            .as_ref()
            .ok_or_else(|| Error::from_reason("no page"))?;
        let result = onecrawl_cdp::tracing_cdp::stop_tracing(page)
            .await
            .map_err(|e| Error::from_reason(e.to_string()))?;
        Ok(result.to_string())
    }

    /// Get performance metrics from the current page as JSON.
    #[napi]
    pub async fn get_performance_metrics(&self) -> Result<String> {
        let guard: TokioMutexGuard<Option<onecrawl_cdp::Page>> = self.page.lock().await;
        let page = guard
            .as_ref()
            .ok_or_else(|| Error::from_reason("no page"))?;
        let result = onecrawl_cdp::tracing_cdp::get_performance_metrics(page)
            .await
            .map_err(|e| Error::from_reason(e.to_string()))?;
        serde_json::to_string(&result).map_err(|e| Error::from_reason(e.to_string()))
    }

    /// Get navigation timing data as JSON.
    #[napi]
    pub async fn get_navigation_timing(&self) -> Result<String> {
        let guard: TokioMutexGuard<Option<onecrawl_cdp::Page>> = self.page.lock().await;
        let page = guard
            .as_ref()
            .ok_or_else(|| Error::from_reason("no page"))?;
        let result = onecrawl_cdp::tracing_cdp::get_navigation_timing(page)
            .await
            .map_err(|e| Error::from_reason(e.to_string()))?;
        Ok(result.to_string())
    }

    /// Get resource timing entries as JSON.
    #[napi]
    pub async fn get_resource_timing(&self) -> Result<String> {
        let guard: TokioMutexGuard<Option<onecrawl_cdp::Page>> = self.page.lock().await;
        let page = guard
            .as_ref()
            .ok_or_else(|| Error::from_reason("no page"))?;
        let result = onecrawl_cdp::tracing_cdp::get_resource_timing(page)
            .await
            .map_err(|e| Error::from_reason(e.to_string()))?;
        serde_json::to_string(&result).map_err(|e| Error::from_reason(e.to_string()))
    }

    // ── Proxy Pool ─────────────────────────────────────────────────

    /// Create a proxy pool from JSON config. Returns the pool as JSON.
    #[napi]
    pub fn create_proxy_pool(&self, config: String) -> Result<String> {
        let pool: onecrawl_cdp::ProxyPool =
            serde_json::from_str(&config).map_err(|e| Error::from_reason(e.to_string()))?;
        pool.to_json()
            .map_err(|e| Error::from_reason(e.to_string()))
    }

    /// Get Chrome launch args for the first proxy in the pool.
    #[napi]
    pub fn get_proxy_chrome_args(&self, pool: String) -> Result<Vec<String>> {
        let p: onecrawl_cdp::ProxyPool =
            serde_json::from_str(&pool).map_err(|e| Error::from_reason(e.to_string()))?;
        Ok(p.chrome_args())
    }

    /// Rotate to the next proxy in the pool. Returns updated pool JSON.
    #[napi]
    pub fn next_proxy(&self, pool: String) -> Result<String> {
        let mut p: onecrawl_cdp::ProxyPool =
            serde_json::from_str(&pool).map_err(|e| Error::from_reason(e.to_string()))?;
        p.next_proxy();
        p.to_json().map_err(|e| Error::from_reason(e.to_string()))
    }

    // ── Request Interception ───────────────────────────────────────

    /// Set request interception rules (JSON array of InterceptRule).
    #[napi]
    pub async fn set_intercept_rules(&self, rules: String) -> Result<()> {
        let guard: TokioMutexGuard<Option<onecrawl_cdp::Page>> = self.page.lock().await;
        let page = guard
            .as_ref()
            .ok_or_else(|| Error::from_reason("no page"))?;
        let parsed: Vec<onecrawl_cdp::InterceptRule> =
            serde_json::from_str(&rules).map_err(|e| Error::from_reason(e.to_string()))?;
        onecrawl_cdp::intercept::set_intercept_rules(page, parsed)
            .await
            .map_err(|e| Error::from_reason(e.to_string()))
    }

    /// Get intercepted request log as JSON.
    #[napi]
    pub async fn get_intercepted_requests(&self) -> Result<String> {
        let guard: TokioMutexGuard<Option<onecrawl_cdp::Page>> = self.page.lock().await;
        let page = guard
            .as_ref()
            .ok_or_else(|| Error::from_reason("no page"))?;
        let log = onecrawl_cdp::intercept::get_intercepted_requests(page)
            .await
            .map_err(|e| Error::from_reason(e.to_string()))?;
        serde_json::to_string(&log).map_err(|e| Error::from_reason(e.to_string()))
    }

    /// Clear all interception rules and restore originals.
    #[napi]
    pub async fn clear_intercept_rules(&self) -> Result<()> {
        let guard: TokioMutexGuard<Option<onecrawl_cdp::Page>> = self.page.lock().await;
        let page = guard
            .as_ref()
            .ok_or_else(|| Error::from_reason("no page"))?;
        onecrawl_cdp::intercept::clear_intercept_rules(page)
            .await
            .map_err(|e| Error::from_reason(e.to_string()))
    }

    // ── Advanced Emulation ─────────────────────────────────────────

    /// Override device orientation sensor.
    #[napi]
    pub async fn set_device_orientation(&self, alpha: f64, beta: f64, gamma: f64) -> Result<()> {
        let guard: TokioMutexGuard<Option<onecrawl_cdp::Page>> = self.page.lock().await;
        let page = guard
            .as_ref()
            .ok_or_else(|| Error::from_reason("no page"))?;
        let reading = onecrawl_cdp::advanced_emulation::SensorReading { alpha, beta, gamma };
        onecrawl_cdp::advanced_emulation::set_device_orientation(page, reading)
            .await
            .map_err(|e| Error::from_reason(e.to_string()))
    }

    /// Override a permission query result (e.g. "geolocation", "granted").
    #[napi]
    pub async fn override_permission(&self, permission: String, state: String) -> Result<()> {
        let guard: TokioMutexGuard<Option<onecrawl_cdp::Page>> = self.page.lock().await;
        let page = guard
            .as_ref()
            .ok_or_else(|| Error::from_reason("no page"))?;
        onecrawl_cdp::advanced_emulation::override_permission(page, &permission, &state)
            .await
            .map_err(|e| Error::from_reason(e.to_string()))
    }

    /// Override battery status API.
    #[napi]
    pub async fn set_battery_status(&self, level: f64, charging: bool) -> Result<()> {
        let guard: TokioMutexGuard<Option<onecrawl_cdp::Page>> = self.page.lock().await;
        let page = guard
            .as_ref()
            .ok_or_else(|| Error::from_reason("no page"))?;
        onecrawl_cdp::advanced_emulation::set_battery_status(page, level, charging)
            .await
            .map_err(|e| Error::from_reason(e.to_string()))
    }

    /// Override Network Information API.
    #[napi]
    pub async fn set_connection_info(
        &self,
        effective_type: String,
        downlink: f64,
        rtt: u32,
    ) -> Result<()> {
        let guard: TokioMutexGuard<Option<onecrawl_cdp::Page>> = self.page.lock().await;
        let page = guard
            .as_ref()
            .ok_or_else(|| Error::from_reason("no page"))?;
        onecrawl_cdp::advanced_emulation::set_connection_info(page, &effective_type, downlink, rtt)
            .await
            .map_err(|e| Error::from_reason(e.to_string()))
    }

    /// Override hardware concurrency (CPU cores).
    #[napi]
    pub async fn set_hardware_concurrency(&self, cores: u32) -> Result<()> {
        let guard: TokioMutexGuard<Option<onecrawl_cdp::Page>> = self.page.lock().await;
        let page = guard
            .as_ref()
            .ok_or_else(|| Error::from_reason("no page"))?;
        onecrawl_cdp::advanced_emulation::set_hardware_concurrency(page, cores)
            .await
            .map_err(|e| Error::from_reason(e.to_string()))
    }

    /// Override device memory (GB).
    #[napi]
    pub async fn set_device_memory(&self, gb: f64) -> Result<()> {
        let guard: TokioMutexGuard<Option<onecrawl_cdp::Page>> = self.page.lock().await;
        let page = guard
            .as_ref()
            .ok_or_else(|| Error::from_reason("no page"))?;
        onecrawl_cdp::advanced_emulation::set_device_memory(page, gb)
            .await
            .map_err(|e| Error::from_reason(e.to_string()))
    }

    /// Get current navigator properties as JSON.
    #[napi]
    pub async fn get_navigator_info(&self) -> Result<String> {
        let guard: TokioMutexGuard<Option<onecrawl_cdp::Page>> = self.page.lock().await;
        let page = guard
            .as_ref()
            .ok_or_else(|| Error::from_reason("no page"))?;
        let info = onecrawl_cdp::advanced_emulation::get_navigator_info(page)
            .await
            .map_err(|e| Error::from_reason(e.to_string()))?;
        serde_json::to_string(&info).map_err(|e| Error::from_reason(e.to_string()))
    }

    // ──────────────── Benchmark ────────────────

    /// Run the CDP benchmark suite. Returns JSON string of BenchmarkSuite.
    #[napi(js_name = "runBenchmark")]
    pub async fn run_benchmark(&self, iterations: Option<u32>) -> Result<String> {
        let iters = iterations.unwrap_or(100);
        let guard: TokioMutexGuard<Option<onecrawl_cdp::Page>> = self.page.lock().await;
        let page = guard
            .as_ref()
            .ok_or_else(|| Error::from_reason("no page"))?;
        let suite = onecrawl_cdp::benchmark::run_cdp_benchmarks(page, iters).await;
        serde_json::to_string(&suite).map_err(|e| Error::from_reason(e.to_string()))
    }

    // ──────────────── Geofencing ────────────────

    /// Apply a geo profile. Accepts a JSON string of GeoProfile.
    #[napi(js_name = "applyGeoProfile")]
    pub async fn apply_geo_profile(&self, profile: String) -> Result<()> {
        let guard: TokioMutexGuard<Option<onecrawl_cdp::Page>> = self.page.lock().await;
        let page = guard
            .as_ref()
            .ok_or_else(|| Error::from_reason("no page"))?;
        let p: onecrawl_cdp::GeoProfile = serde_json::from_str(&profile)
            .map_err(|e| Error::from_reason(format!("invalid geo profile: {e}")))?;
        onecrawl_cdp::geofencing::apply_geo_profile(page, &p)
            .await
            .map_err(|e| Error::from_reason(e.to_string()))
    }

    /// List available geo preset names.
    #[napi(js_name = "listGeoPresets")]
    pub fn list_geo_presets(&self) -> Vec<String> {
        onecrawl_cdp::geofencing::list_presets()
    }

    /// Get a geo preset by name. Returns JSON string of GeoProfile or null.
    #[napi(js_name = "getGeoPreset")]
    pub fn get_geo_preset(&self, name: String) -> Option<String> {
        onecrawl_cdp::geofencing::get_preset(&name)
            .map(|p| serde_json::to_string(&p).unwrap_or_default())
    }

    /// Get current geolocation as seen by the page. Returns JSON string.
    #[napi(js_name = "getCurrentGeo")]
    pub async fn get_current_geo(&self) -> Result<String> {
        let guard: TokioMutexGuard<Option<onecrawl_cdp::Page>> = self.page.lock().await;
        let page = guard
            .as_ref()
            .ok_or_else(|| Error::from_reason("no page"))?;
        let val = onecrawl_cdp::geofencing::get_current_geo(page)
            .await
            .map_err(|e| Error::from_reason(e.to_string()))?;
        serde_json::to_string(&val).map_err(|e| Error::from_reason(e.to_string()))
    }

    // ──────────────── Cookie Jar ────────────────

    /// Export all cookies as a JSON CookieJar string.
    #[napi(js_name = "exportCookies")]
    pub async fn export_cookies(&self) -> Result<String> {
        let guard: TokioMutexGuard<Option<onecrawl_cdp::Page>> = self.page.lock().await;
        let page = guard
            .as_ref()
            .ok_or_else(|| Error::from_reason("no page"))?;
        let jar = onecrawl_cdp::cookie_jar::export_cookies(page)
            .await
            .map_err(|e| Error::from_reason(e.to_string()))?;
        serde_json::to_string(&jar).map_err(|e| Error::from_reason(e.to_string()))
    }

    /// Import cookies from a JSON CookieJar string. Returns count imported.
    #[napi(js_name = "importCookies")]
    pub async fn import_cookies(&self, jar: String) -> Result<u32> {
        let guard: TokioMutexGuard<Option<onecrawl_cdp::Page>> = self.page.lock().await;
        let page = guard
            .as_ref()
            .ok_or_else(|| Error::from_reason("no page"))?;
        let cookie_jar: onecrawl_cdp::CookieJar = serde_json::from_str(&jar)
            .map_err(|e| Error::from_reason(format!("invalid cookie jar: {e}")))?;
        let count = onecrawl_cdp::cookie_jar::import_cookies(page, &cookie_jar)
            .await
            .map_err(|e| Error::from_reason(e.to_string()))?;
        Ok(count as u32)
    }

    /// Save cookies to a file. Returns count saved.
    #[napi(js_name = "saveCookiesToFile")]
    pub async fn save_cookies_to_file(&self, path: String) -> Result<u32> {
        let guard: TokioMutexGuard<Option<onecrawl_cdp::Page>> = self.page.lock().await;
        let page = guard
            .as_ref()
            .ok_or_else(|| Error::from_reason("no page"))?;
        let count =
            onecrawl_cdp::cookie_jar::save_cookies_to_file(page, std::path::Path::new(&path))
                .await
                .map_err(|e| Error::from_reason(e.to_string()))?;
        Ok(count as u32)
    }

    /// Load cookies from a file. Returns count loaded.
    #[napi(js_name = "loadCookiesFromFile")]
    pub async fn load_cookies_from_file(&self, path: String) -> Result<u32> {
        let guard: TokioMutexGuard<Option<onecrawl_cdp::Page>> = self.page.lock().await;
        let page = guard
            .as_ref()
            .ok_or_else(|| Error::from_reason("no page"))?;
        let count =
            onecrawl_cdp::cookie_jar::load_cookies_from_file(page, std::path::Path::new(&path))
                .await
                .map_err(|e| Error::from_reason(e.to_string()))?;
        Ok(count as u32)
    }

    /// Clear all cookies via cookie_jar module.
    #[napi(js_name = "clearAllCookies")]
    pub async fn clear_all_cookies(&self) -> Result<()> {
        let guard: TokioMutexGuard<Option<onecrawl_cdp::Page>> = self.page.lock().await;
        let page = guard
            .as_ref()
            .ok_or_else(|| Error::from_reason("no page"))?;
        onecrawl_cdp::cookie_jar::clear_all_cookies(page)
            .await
            .map_err(|e| Error::from_reason(e.to_string()))
    }

    // ──────────────── Request Queue ────────────────

    /// Execute a single request with retry. Accepts JSON QueuedRequest. Returns JSON RequestResult.
    #[napi(js_name = "executeRequest")]
    pub async fn execute_request(&self, request: String) -> Result<String> {
        let guard: TokioMutexGuard<Option<onecrawl_cdp::Page>> = self.page.lock().await;
        let page = guard
            .as_ref()
            .ok_or_else(|| Error::from_reason("no page"))?;
        let req: onecrawl_cdp::QueuedRequest = serde_json::from_str(&request)
            .map_err(|e| Error::from_reason(format!("invalid request: {e}")))?;
        let result = onecrawl_cdp::request_queue::execute_request(page, &req)
            .await
            .map_err(|e| Error::from_reason(e.to_string()))?;
        serde_json::to_string(&result).map_err(|e| Error::from_reason(e.to_string()))
    }

    /// Execute a batch of requests. Accepts JSON array of QueuedRequest + optional JSON QueueConfig.
    #[napi(js_name = "executeBatch")]
    pub async fn execute_batch(&self, requests: String, config: Option<String>) -> Result<String> {
        let guard: TokioMutexGuard<Option<onecrawl_cdp::Page>> = self.page.lock().await;
        let page = guard
            .as_ref()
            .ok_or_else(|| Error::from_reason("no page"))?;
        let reqs: Vec<onecrawl_cdp::QueuedRequest> = serde_json::from_str(&requests)
            .map_err(|e| Error::from_reason(format!("invalid requests: {e}")))?;
        let cfg: onecrawl_cdp::QueueConfig = match config {
            Some(c) => serde_json::from_str(&c)
                .map_err(|e| Error::from_reason(format!("invalid config: {e}")))?,
            None => onecrawl_cdp::QueueConfig::default(),
        };
        let results = onecrawl_cdp::request_queue::execute_batch(page, &reqs, &cfg)
            .await
            .map_err(|e| Error::from_reason(e.to_string()))?;
        serde_json::to_string(&results).map_err(|e| Error::from_reason(e.to_string()))
    }

    /// Create a GET request. Returns JSON QueuedRequest.
    #[napi(js_name = "createGetRequest")]
    pub fn create_get_request(&self, id: String, url: String) -> String {
        let req = onecrawl_cdp::request_queue::get_request(&id, &url);
        serde_json::to_string(&req).unwrap_or_default()
    }

    /// Create a POST request. Returns JSON QueuedRequest.
    #[napi(js_name = "createPostRequest")]
    pub fn create_post_request(&self, id: String, url: String, body: String) -> String {
        let req = onecrawl_cdp::request_queue::post_request(&id, &url, &body);
        serde_json::to_string(&req).unwrap_or_default()
    }

    // ──────────────── Smart Selectors ────────────────

    /// CSS selector with pseudo-elements (::text, ::attr(name)). Returns JSON SelectorResult.
    #[napi(js_name = "cssSelect")]
    pub async fn css_select(&self, selector: String) -> Result<String> {
        let guard: TokioMutexGuard<Option<onecrawl_cdp::Page>> = self.page.lock().await;
        let page = guard
            .as_ref()
            .ok_or_else(|| Error::from_reason("no page"))?;
        let result = onecrawl_cdp::selectors::css_select(page, &selector)
            .await
            .map_err(|e| Error::from_reason(e.to_string()))?;
        serde_json::to_string(&result).map_err(|e| Error::from_reason(e.to_string()))
    }

    /// XPath selector. Returns JSON SelectorResult.
    #[napi(js_name = "xpathSelect")]
    pub async fn xpath_select(&self, expression: String) -> Result<String> {
        let guard: TokioMutexGuard<Option<onecrawl_cdp::Page>> = self.page.lock().await;
        let page = guard
            .as_ref()
            .ok_or_else(|| Error::from_reason("no page"))?;
        let result = onecrawl_cdp::selectors::xpath_select(page, &expression)
            .await
            .map_err(|e| Error::from_reason(e.to_string()))?;
        serde_json::to_string(&result).map_err(|e| Error::from_reason(e.to_string()))
    }

    /// Find elements by text content. Returns JSON SelectorResult.
    #[napi(js_name = "findByText")]
    pub async fn find_by_text(&self, text: String, tag: Option<String>) -> Result<String> {
        let guard: TokioMutexGuard<Option<onecrawl_cdp::Page>> = self.page.lock().await;
        let page = guard
            .as_ref()
            .ok_or_else(|| Error::from_reason("no page"))?;
        let result = onecrawl_cdp::selectors::find_by_text(page, &text, tag.as_deref())
            .await
            .map_err(|e| Error::from_reason(e.to_string()))?;
        serde_json::to_string(&result).map_err(|e| Error::from_reason(e.to_string()))
    }

    /// Find elements by regex pattern. Returns JSON SelectorResult.
    #[napi(js_name = "findByRegex")]
    pub async fn find_by_regex(&self, pattern: String, tag: Option<String>) -> Result<String> {
        let guard: TokioMutexGuard<Option<onecrawl_cdp::Page>> = self.page.lock().await;
        let page = guard
            .as_ref()
            .ok_or_else(|| Error::from_reason("no page"))?;
        let result = onecrawl_cdp::selectors::find_by_regex(page, &pattern, tag.as_deref())
            .await
            .map_err(|e| Error::from_reason(e.to_string()))?;
        serde_json::to_string(&result).map_err(|e| Error::from_reason(e.to_string()))
    }

    /// Auto-generate a unique CSS selector for an element. Returns the selector string.
    #[napi(js_name = "autoSelector")]
    pub async fn auto_selector(&self, target_selector: String) -> Result<String> {
        let guard: TokioMutexGuard<Option<onecrawl_cdp::Page>> = self.page.lock().await;
        let page = guard
            .as_ref()
            .ok_or_else(|| Error::from_reason("no page"))?;
        onecrawl_cdp::selectors::auto_selector(page, &target_selector)
            .await
            .map_err(|e| Error::from_reason(e.to_string()))
    }

    // ──────────────── DOM Navigation ────────────────

    /// Get parent element. Returns JSON NavElement or null.
    #[napi(js_name = "getParent")]
    pub async fn get_parent(&self, selector: String) -> Result<Option<String>> {
        let guard: TokioMutexGuard<Option<onecrawl_cdp::Page>> = self.page.lock().await;
        let page = guard
            .as_ref()
            .ok_or_else(|| Error::from_reason("no page"))?;
        let result = onecrawl_cdp::dom_nav::get_parent(page, &selector)
            .await
            .map_err(|e| Error::from_reason(e.to_string()))?;
        match result {
            Some(el) => Ok(Some(
                serde_json::to_string(&el).map_err(|e| Error::from_reason(e.to_string()))?,
            )),
            None => Ok(None),
        }
    }

    /// Get child elements. Returns JSON array of NavElement.
    #[napi(js_name = "getChildren")]
    pub async fn get_children(&self, selector: String) -> Result<String> {
        let guard: TokioMutexGuard<Option<onecrawl_cdp::Page>> = self.page.lock().await;
        let page = guard
            .as_ref()
            .ok_or_else(|| Error::from_reason("no page"))?;
        let result = onecrawl_cdp::dom_nav::get_children(page, &selector)
            .await
            .map_err(|e| Error::from_reason(e.to_string()))?;
        serde_json::to_string(&result).map_err(|e| Error::from_reason(e.to_string()))
    }

    /// Get next sibling element. Returns JSON NavElement or null.
    #[napi(js_name = "getNextSibling")]
    pub async fn get_next_sibling(&self, selector: String) -> Result<Option<String>> {
        let guard: TokioMutexGuard<Option<onecrawl_cdp::Page>> = self.page.lock().await;
        let page = guard
            .as_ref()
            .ok_or_else(|| Error::from_reason("no page"))?;
        let result = onecrawl_cdp::dom_nav::get_next_sibling(page, &selector)
            .await
            .map_err(|e| Error::from_reason(e.to_string()))?;
        match result {
            Some(el) => Ok(Some(
                serde_json::to_string(&el).map_err(|e| Error::from_reason(e.to_string()))?,
            )),
            None => Ok(None),
        }
    }

    /// Get previous sibling element. Returns JSON NavElement or null.
    #[napi(js_name = "getPrevSibling")]
    pub async fn get_prev_sibling(&self, selector: String) -> Result<Option<String>> {
        let guard: TokioMutexGuard<Option<onecrawl_cdp::Page>> = self.page.lock().await;
        let page = guard
            .as_ref()
            .ok_or_else(|| Error::from_reason("no page"))?;
        let result = onecrawl_cdp::dom_nav::get_prev_sibling(page, &selector)
            .await
            .map_err(|e| Error::from_reason(e.to_string()))?;
        match result {
            Some(el) => Ok(Some(
                serde_json::to_string(&el).map_err(|e| Error::from_reason(e.to_string()))?,
            )),
            None => Ok(None),
        }
    }

    /// Get all sibling elements. Returns JSON array of NavElement.
    #[napi(js_name = "getSiblings")]
    pub async fn get_siblings(&self, selector: String) -> Result<String> {
        let guard: TokioMutexGuard<Option<onecrawl_cdp::Page>> = self.page.lock().await;
        let page = guard
            .as_ref()
            .ok_or_else(|| Error::from_reason("no page"))?;
        let result = onecrawl_cdp::dom_nav::get_siblings(page, &selector)
            .await
            .map_err(|e| Error::from_reason(e.to_string()))?;
        serde_json::to_string(&result).map_err(|e| Error::from_reason(e.to_string()))
    }

    /// Find similar elements. Returns JSON array of NavElement.
    #[napi(js_name = "findSimilar")]
    pub async fn find_similar(&self, selector: String) -> Result<String> {
        let guard: TokioMutexGuard<Option<onecrawl_cdp::Page>> = self.page.lock().await;
        let page = guard
            .as_ref()
            .ok_or_else(|| Error::from_reason("no page"))?;
        let result = onecrawl_cdp::dom_nav::find_similar(page, &selector)
            .await
            .map_err(|e| Error::from_reason(e.to_string()))?;
        serde_json::to_string(&result).map_err(|e| Error::from_reason(e.to_string()))
    }

    /// Get elements above the target. Returns JSON array of NavElement.
    #[napi(js_name = "aboveElements")]
    pub async fn above_elements(&self, selector: String, limit: Option<u32>) -> Result<String> {
        let guard: TokioMutexGuard<Option<onecrawl_cdp::Page>> = self.page.lock().await;
        let page = guard
            .as_ref()
            .ok_or_else(|| Error::from_reason("no page"))?;
        let result =
            onecrawl_cdp::dom_nav::above_elements(page, &selector, limit.unwrap_or(10) as usize)
                .await
                .map_err(|e| Error::from_reason(e.to_string()))?;
        serde_json::to_string(&result).map_err(|e| Error::from_reason(e.to_string()))
    }

    /// Get elements below the target. Returns JSON array of NavElement.
    #[napi(js_name = "belowElements")]
    pub async fn below_elements(&self, selector: String, limit: Option<u32>) -> Result<String> {
        let guard: TokioMutexGuard<Option<onecrawl_cdp::Page>> = self.page.lock().await;
        let page = guard
            .as_ref()
            .ok_or_else(|| Error::from_reason("no page"))?;
        let result =
            onecrawl_cdp::dom_nav::below_elements(page, &selector, limit.unwrap_or(10) as usize)
                .await
                .map_err(|e| Error::from_reason(e.to_string()))?;
        serde_json::to_string(&result).map_err(|e| Error::from_reason(e.to_string()))
    }

    // ──────────────── Content Extraction ────────────────

    /// Extract page content. Returns JSON ExtractResult.
    #[napi(js_name = "extract")]
    pub async fn extract_content(
        &self,
        selector: Option<String>,
        format: Option<String>,
    ) -> Result<String> {
        let guard: TokioMutexGuard<Option<onecrawl_cdp::Page>> = self.page.lock().await;
        let page = guard
            .as_ref()
            .ok_or_else(|| Error::from_reason("no page"))?;
        let fmt = onecrawl_cdp::extract::parse_extract_format(format.as_deref().unwrap_or("text"))
            .map_err(|e| Error::from_reason(e.to_string()))?;
        let result = onecrawl_cdp::extract::extract(page, selector.as_deref(), fmt)
            .await
            .map_err(|e| Error::from_reason(e.to_string()))?;
        serde_json::to_string(&result).map_err(|e| Error::from_reason(e.to_string()))
    }

    /// Extract content and save to file. Returns bytes written.
    #[napi(js_name = "extractToFile")]
    pub async fn extract_to_file(
        &self,
        output_path: String,
        selector: Option<String>,
    ) -> Result<u32> {
        let guard: TokioMutexGuard<Option<onecrawl_cdp::Page>> = self.page.lock().await;
        let page = guard
            .as_ref()
            .ok_or_else(|| Error::from_reason("no page"))?;
        let bytes = onecrawl_cdp::extract::extract_to_file(
            page,
            selector.as_deref(),
            std::path::Path::new(&output_path),
        )
        .await
        .map_err(|e| Error::from_reason(e.to_string()))?;
        Ok(bytes as u32)
    }

    /// Get structured page metadata. Returns JSON object.
    #[napi(js_name = "getPageMetadata")]
    pub async fn get_page_metadata(&self) -> Result<String> {
        let guard: TokioMutexGuard<Option<onecrawl_cdp::Page>> = self.page.lock().await;
        let page = guard
            .as_ref()
            .ok_or_else(|| Error::from_reason("no page"))?;
        let meta = onecrawl_cdp::extract::get_page_metadata(page)
            .await
            .map_err(|e| Error::from_reason(e.to_string()))?;
        serde_json::to_string(&meta).map_err(|e| Error::from_reason(e.to_string()))
    }

    // ── Network Request Logger ─────────────────────────────────────

    /// Start network request/response logging.
    #[napi]
    pub async fn start_network_log(&self) -> Result<()> {
        let guard: TokioMutexGuard<Option<onecrawl_cdp::Page>> = self.page.lock().await;
        let page = guard
            .as_ref()
            .ok_or_else(|| Error::from_reason("no page"))?;
        onecrawl_cdp::network_log::start_network_log(page)
            .await
            .map_err(|e| Error::from_reason(e.to_string()))
    }

    /// Drain captured network entries as JSON string.
    #[napi]
    pub async fn drain_network_log(&self) -> Result<String> {
        let guard: TokioMutexGuard<Option<onecrawl_cdp::Page>> = self.page.lock().await;
        let page = guard
            .as_ref()
            .ok_or_else(|| Error::from_reason("no page"))?;
        let entries = onecrawl_cdp::network_log::drain_network_log(page)
            .await
            .map_err(|e| Error::from_reason(e.to_string()))?;
        serde_json::to_string(&entries).map_err(|e| Error::from_reason(e.to_string()))
    }

    /// Get network summary statistics as JSON string.
    #[napi]
    pub async fn get_network_summary(&self) -> Result<String> {
        let guard: TokioMutexGuard<Option<onecrawl_cdp::Page>> = self.page.lock().await;
        let page = guard
            .as_ref()
            .ok_or_else(|| Error::from_reason("no page"))?;
        let summary = onecrawl_cdp::network_log::get_network_summary(page)
            .await
            .map_err(|e| Error::from_reason(e.to_string()))?;
        serde_json::to_string(&summary).map_err(|e| Error::from_reason(e.to_string()))
    }

    /// Stop network logging and restore originals.
    #[napi]
    pub async fn stop_network_log(&self) -> Result<()> {
        let guard: TokioMutexGuard<Option<onecrawl_cdp::Page>> = self.page.lock().await;
        let page = guard
            .as_ref()
            .ok_or_else(|| Error::from_reason("no page"))?;
        onecrawl_cdp::network_log::stop_network_log(page)
            .await
            .map_err(|e| Error::from_reason(e.to_string()))
    }

    /// Export network log to a JSON file.
    #[napi]
    pub async fn export_network_log(&self, path: String) -> Result<()> {
        let guard: TokioMutexGuard<Option<onecrawl_cdp::Page>> = self.page.lock().await;
        let page = guard
            .as_ref()
            .ok_or_else(|| Error::from_reason("no page"))?;
        onecrawl_cdp::network_log::export_network_log(page, &path)
            .await
            .map_err(|e| Error::from_reason(e.to_string()))
    }

    // ── Page Watcher ───────────────────────────────────────────────

    /// Start watching for page state changes.
    #[napi]
    pub async fn start_page_watcher(&self) -> Result<()> {
        let guard: TokioMutexGuard<Option<onecrawl_cdp::Page>> = self.page.lock().await;
        let page = guard
            .as_ref()
            .ok_or_else(|| Error::from_reason("no page"))?;
        onecrawl_cdp::page_watcher::start_page_watcher(page)
            .await
            .map_err(|e| Error::from_reason(e.to_string()))
    }

    /// Drain accumulated page changes as JSON string.
    #[napi]
    pub async fn drain_page_changes(&self) -> Result<String> {
        let guard: TokioMutexGuard<Option<onecrawl_cdp::Page>> = self.page.lock().await;
        let page = guard
            .as_ref()
            .ok_or_else(|| Error::from_reason("no page"))?;
        let changes = onecrawl_cdp::page_watcher::drain_page_changes(page)
            .await
            .map_err(|e| Error::from_reason(e.to_string()))?;
        serde_json::to_string(&changes).map_err(|e| Error::from_reason(e.to_string()))
    }

    /// Stop the page watcher.
    #[napi]
    pub async fn stop_page_watcher(&self) -> Result<()> {
        let guard: TokioMutexGuard<Option<onecrawl_cdp::Page>> = self.page.lock().await;
        let page = guard
            .as_ref()
            .ok_or_else(|| Error::from_reason("no page"))?;
        onecrawl_cdp::page_watcher::stop_page_watcher(page)
            .await
            .map_err(|e| Error::from_reason(e.to_string()))
    }

    /// Get current page state snapshot as JSON string.
    #[napi]
    pub async fn get_page_state(&self) -> Result<String> {
        let guard: TokioMutexGuard<Option<onecrawl_cdp::Page>> = self.page.lock().await;
        let page = guard
            .as_ref()
            .ok_or_else(|| Error::from_reason("no page"))?;
        let state = onecrawl_cdp::page_watcher::get_page_state(page)
            .await
            .map_err(|e| Error::from_reason(e.to_string()))?;
        serde_json::to_string(&state).map_err(|e| Error::from_reason(e.to_string()))
    }

    // ── Spider / Crawl ─────────────────────────────────────────────

    /// Run a crawl. Accepts SpiderConfig as JSON, returns Vec<CrawlResult> as JSON.
    #[napi]
    pub async fn crawl(&self, config_json: String) -> Result<String> {
        let config: onecrawl_cdp::SpiderConfig =
            serde_json::from_str(&config_json).map_err(|e| Error::from_reason(e.to_string()))?;
        let guard: TokioMutexGuard<Option<onecrawl_cdp::Page>> = self.page.lock().await;
        let page = guard
            .as_ref()
            .ok_or_else(|| Error::from_reason("no page"))?;
        let results = onecrawl_cdp::spider::crawl(page, config)
            .await
            .map_err(|e| Error::from_reason(e.to_string()))?;
        serde_json::to_string(&results).map_err(|e| Error::from_reason(e.to_string()))
    }

    /// Compute crawl summary from results JSON.
    #[napi]
    pub fn crawl_summary(&self, results_json: String) -> Result<String> {
        let results: Vec<onecrawl_cdp::CrawlResult> =
            serde_json::from_str(&results_json).map_err(|e| Error::from_reason(e.to_string()))?;
        let summary = onecrawl_cdp::spider::summarize(&results);
        serde_json::to_string(&summary).map_err(|e| Error::from_reason(e.to_string()))
    }

    /// Save crawl state to a JSON file.
    #[napi]
    pub fn save_crawl_state(&self, state_json: String, path: String) -> Result<()> {
        let state: onecrawl_cdp::CrawlState =
            serde_json::from_str(&state_json).map_err(|e| Error::from_reason(e.to_string()))?;
        onecrawl_cdp::spider::save_state(&state, std::path::Path::new(&path))
            .map_err(|e| Error::from_reason(e.to_string()))
    }

    /// Load crawl state from a JSON file.
    #[napi]
    pub fn load_crawl_state(&self, path: String) -> Result<String> {
        let state = onecrawl_cdp::spider::load_state(std::path::Path::new(&path))
            .map_err(|e| Error::from_reason(e.to_string()))?;
        serde_json::to_string(&state).map_err(|e| Error::from_reason(e.to_string()))
    }

    /// Export crawl results to file. Format: "json" (default) or "jsonl".
    #[napi]
    pub fn export_crawl_results(
        &self,
        results_json: String,
        path: String,
        format: Option<String>,
    ) -> Result<u32> {
        let results: Vec<onecrawl_cdp::CrawlResult> =
            serde_json::from_str(&results_json).map_err(|e| Error::from_reason(e.to_string()))?;
        let p = std::path::Path::new(&path);
        let count = match format.as_deref() {
            Some("jsonl") => onecrawl_cdp::spider::export_results_jsonl(&results, p),
            _ => onecrawl_cdp::spider::export_results(&results, p),
        }
        .map_err(|e| Error::from_reason(e.to_string()))?;
        Ok(count as u32)
    }

    // ── Robots.txt ─────────────────────────────────────────────────

    /// Parse robots.txt content. Returns JSON RobotsTxt.
    #[napi(js_name = "robotsParse")]
    pub fn robots_parse(&self, content: String) -> Result<String> {
        let robots = onecrawl_cdp::robots::parse_robots(&content);
        serde_json::to_string(&robots).map_err(|e| Error::from_reason(e.to_string()))
    }

    /// Check if a path is allowed for a user-agent. Accepts JSON RobotsTxt.
    #[napi(js_name = "robotsIsAllowed")]
    pub fn robots_is_allowed(
        &self,
        robots_json: String,
        user_agent: String,
        path: String,
    ) -> Result<bool> {
        let robots: onecrawl_cdp::RobotsTxt =
            serde_json::from_str(&robots_json).map_err(|e| Error::from_reason(e.to_string()))?;
        Ok(onecrawl_cdp::robots::is_allowed(
            &robots,
            &user_agent,
            &path,
        ))
    }

    /// Get crawl delay for a user-agent. Accepts JSON RobotsTxt.
    #[napi(js_name = "robotsCrawlDelay")]
    pub fn robots_crawl_delay(
        &self,
        robots_json: String,
        user_agent: String,
    ) -> Result<Option<f64>> {
        let robots: onecrawl_cdp::RobotsTxt =
            serde_json::from_str(&robots_json).map_err(|e| Error::from_reason(e.to_string()))?;
        Ok(onecrawl_cdp::robots::get_crawl_delay(&robots, &user_agent))
    }

    /// Get sitemaps from parsed robots.txt. Accepts JSON RobotsTxt, returns JSON array.
    #[napi(js_name = "robotsSitemaps")]
    pub fn robots_sitemaps(&self, robots_json: String) -> Result<String> {
        let robots: onecrawl_cdp::RobotsTxt =
            serde_json::from_str(&robots_json).map_err(|e| Error::from_reason(e.to_string()))?;
        let sitemaps = onecrawl_cdp::robots::get_sitemaps(&robots);
        serde_json::to_string(&sitemaps).map_err(|e| Error::from_reason(e.to_string()))
    }

    /// Fetch and parse robots.txt from a URL. Returns JSON RobotsTxt.
    #[napi(js_name = "robotsFetch")]
    pub async fn robots_fetch(&self, base_url: String) -> Result<String> {
        let guard: TokioMutexGuard<Option<onecrawl_cdp::Page>> = self.page.lock().await;
        let page = guard
            .as_ref()
            .ok_or_else(|| Error::from_reason("no page"))?;
        let robots = onecrawl_cdp::robots::fetch_robots(page, &base_url)
            .await
            .map_err(|e| Error::from_reason(e.to_string()))?;
        serde_json::to_string(&robots).map_err(|e| Error::from_reason(e.to_string()))
    }

    // ── Link Graph ─────────────────────────────────────────────────

    /// Extract links from the current page. Returns JSON Vec<LinkEdge>.
    #[napi(js_name = "graphExtractLinks")]
    pub async fn graph_extract_links(&self, base_url: String) -> Result<String> {
        let guard: TokioMutexGuard<Option<onecrawl_cdp::Page>> = self.page.lock().await;
        let page = guard
            .as_ref()
            .ok_or_else(|| Error::from_reason("no page"))?;
        let edges = onecrawl_cdp::link_graph::extract_links(page, &base_url)
            .await
            .map_err(|e| Error::from_reason(e.to_string()))?;
        serde_json::to_string(&edges).map_err(|e| Error::from_reason(e.to_string()))
    }

    /// Build a link graph from edges JSON. Returns JSON LinkGraph.
    #[napi(js_name = "graphBuild")]
    pub fn graph_build(&self, edges_json: String) -> Result<String> {
        let edges: Vec<onecrawl_cdp::LinkEdge> =
            serde_json::from_str(&edges_json).map_err(|e| Error::from_reason(e.to_string()))?;
        let graph = onecrawl_cdp::link_graph::build_graph(&edges);
        serde_json::to_string(&graph).map_err(|e| Error::from_reason(e.to_string()))
    }

    /// Analyze a link graph. Accepts JSON LinkGraph, returns JSON LinkStats.
    #[napi(js_name = "graphAnalyze")]
    pub fn graph_analyze(&self, graph_json: String) -> Result<String> {
        let graph: onecrawl_cdp::LinkGraph =
            serde_json::from_str(&graph_json).map_err(|e| Error::from_reason(e.to_string()))?;
        let stats = onecrawl_cdp::link_graph::analyze_graph(&graph);
        serde_json::to_string(&stats).map_err(|e| Error::from_reason(e.to_string()))
    }

    /// Find orphan pages (no inbound links). Accepts JSON LinkGraph, returns JSON array.
    #[napi(js_name = "graphFindOrphans")]
    pub fn graph_find_orphans(&self, graph_json: String) -> Result<String> {
        let graph: onecrawl_cdp::LinkGraph =
            serde_json::from_str(&graph_json).map_err(|e| Error::from_reason(e.to_string()))?;
        let orphans = onecrawl_cdp::link_graph::find_orphans(&graph);
        serde_json::to_string(&orphans).map_err(|e| Error::from_reason(e.to_string()))
    }

    /// Find hub pages. Accepts JSON LinkGraph and min_outbound threshold.
    #[napi(js_name = "graphFindHubs")]
    pub fn graph_find_hubs(&self, graph_json: String, min_outbound: u32) -> Result<String> {
        let graph: onecrawl_cdp::LinkGraph =
            serde_json::from_str(&graph_json).map_err(|e| Error::from_reason(e.to_string()))?;
        let hubs = onecrawl_cdp::link_graph::find_hubs(&graph, min_outbound as usize);
        serde_json::to_string(&hubs).map_err(|e| Error::from_reason(e.to_string()))
    }

    /// Export link graph to a JSON file.
    #[napi(js_name = "graphExport")]
    pub fn graph_export(&self, graph_json: String, path: String) -> Result<()> {
        let graph: onecrawl_cdp::LinkGraph =
            serde_json::from_str(&graph_json).map_err(|e| Error::from_reason(e.to_string()))?;
        onecrawl_cdp::link_graph::export_graph_json(&graph, std::path::Path::new(&path))
            .map_err(|e| Error::from_reason(e.to_string()))
    }

    /// Build link graph from crawl results JSON. Returns JSON LinkGraph.
    #[napi(js_name = "graphFromCrawlResults")]
    pub fn graph_from_crawl_results(&self, results_json: String) -> Result<String> {
        let results: Vec<onecrawl_cdp::CrawlResult> =
            serde_json::from_str(&results_json).map_err(|e| Error::from_reason(e.to_string()))?;
        let graph = onecrawl_cdp::link_graph::from_crawl_results(&results);
        serde_json::to_string(&graph).map_err(|e| Error::from_reason(e.to_string()))
    }

    // ── Anti-Bot ────────────────────────────────────────────────────

    /// Inject full stealth anti-bot patches. Returns JSON array of applied patch names.
    #[napi]
    pub async fn inject_stealth_full(&self) -> Result<String> {
        let guard: TokioMutexGuard<Option<onecrawl_cdp::Page>> = self.page.lock().await;
        let page = guard
            .as_ref()
            .ok_or_else(|| Error::from_reason("no page"))?;
        let applied = onecrawl_cdp::antibot::inject_stealth_full(page)
            .await
            .map_err(|e| Error::from_reason(e.to_string()))?;
        serde_json::to_string(&applied).map_err(|e| Error::from_reason(e.to_string()))
    }

    /// Run bot detection tests. Returns JSON object with scores.
    #[napi]
    pub async fn bot_detection_test(&self) -> Result<String> {
        let guard: TokioMutexGuard<Option<onecrawl_cdp::Page>> = self.page.lock().await;
        let page = guard
            .as_ref()
            .ok_or_else(|| Error::from_reason("no page"))?;
        let result = onecrawl_cdp::antibot::bot_detection_test(page)
            .await
            .map_err(|e| Error::from_reason(e.to_string()))?;
        serde_json::to_string(&result).map_err(|e| Error::from_reason(e.to_string()))
    }

    /// Get available stealth profiles. Returns JSON array.
    #[napi]
    pub fn stealth_profiles(&self) -> Result<String> {
        let profiles = onecrawl_cdp::antibot::stealth_profiles();
        serde_json::to_string(&profiles).map_err(|e| Error::from_reason(e.to_string()))
    }

    // ── Adaptive Element Tracker ────────────────────────────────────

    /// Fingerprint a DOM element by CSS selector. Returns JSON.
    #[napi]
    pub async fn fingerprint_element(&self, selector: String) -> Result<String> {
        let guard: TokioMutexGuard<Option<onecrawl_cdp::Page>> = self.page.lock().await;
        let page = guard
            .as_ref()
            .ok_or_else(|| Error::from_reason("no page"))?;
        let fp = onecrawl_cdp::adaptive::fingerprint_element(page, &selector)
            .await
            .map_err(|e| Error::from_reason(e.to_string()))?;
        serde_json::to_string(&fp).map_err(|e| Error::from_reason(e.to_string()))
    }

    /// Relocate an element using a previously captured fingerprint (JSON). Returns JSON matches.
    #[napi]
    pub async fn relocate_element(&self, fingerprint: String) -> Result<String> {
        let fp: onecrawl_cdp::ElementFingerprint =
            serde_json::from_str(&fingerprint).map_err(|e| Error::from_reason(e.to_string()))?;
        let guard: TokioMutexGuard<Option<onecrawl_cdp::Page>> = self.page.lock().await;
        let page = guard
            .as_ref()
            .ok_or_else(|| Error::from_reason("no page"))?;
        let matches = onecrawl_cdp::adaptive::relocate_element(page, &fp)
            .await
            .map_err(|e| Error::from_reason(e.to_string()))?;
        serde_json::to_string(&matches).map_err(|e| Error::from_reason(e.to_string()))
    }

    /// Track multiple elements by CSS selectors (JSON array). Optionally save to path.
    #[napi]
    pub async fn track_elements(
        &self,
        selectors: String,
        save_path: Option<String>,
    ) -> Result<String> {
        let sels: Vec<String> =
            serde_json::from_str(&selectors).map_err(|e| Error::from_reason(e.to_string()))?;
        let sel_refs: Vec<&str> = sels.iter().map(|s| s.as_str()).collect();
        let guard: TokioMutexGuard<Option<onecrawl_cdp::Page>> = self.page.lock().await;
        let page = guard
            .as_ref()
            .ok_or_else(|| Error::from_reason("no page"))?;
        let path_buf = save_path.map(std::path::PathBuf::from);
        let fps = onecrawl_cdp::adaptive::track_elements(page, &sel_refs, path_buf.as_deref())
            .await
            .map_err(|e| Error::from_reason(e.to_string()))?;
        serde_json::to_string(&fps).map_err(|e| Error::from_reason(e.to_string()))
    }

    /// Relocate all fingerprints (JSON array). Returns JSON array of (selector, matches).
    #[napi]
    pub async fn relocate_all(&self, fingerprints: String) -> Result<String> {
        let fps: Vec<onecrawl_cdp::ElementFingerprint> =
            serde_json::from_str(&fingerprints).map_err(|e| Error::from_reason(e.to_string()))?;
        let guard: TokioMutexGuard<Option<onecrawl_cdp::Page>> = self.page.lock().await;
        let page = guard
            .as_ref()
            .ok_or_else(|| Error::from_reason("no page"))?;
        let results = onecrawl_cdp::adaptive::relocate_all(page, &fps)
            .await
            .map_err(|e| Error::from_reason(e.to_string()))?;
        serde_json::to_string(&results).map_err(|e| Error::from_reason(e.to_string()))
    }

    /// Save fingerprints JSON to a file path.
    #[napi]
    pub fn save_fingerprints(&self, fingerprints: String, path: String) -> Result<()> {
        let fps: Vec<onecrawl_cdp::ElementFingerprint> =
            serde_json::from_str(&fingerprints).map_err(|e| Error::from_reason(e.to_string()))?;
        onecrawl_cdp::adaptive::save_fingerprints(&fps, std::path::Path::new(&path))
            .map_err(|e| Error::from_reason(e.to_string()))
    }

    /// Load fingerprints from a file path. Returns JSON array.
    #[napi]
    pub fn load_fingerprints(&self, path: String) -> Result<String> {
        let fps = onecrawl_cdp::adaptive::load_fingerprints(std::path::Path::new(&path))
            .map_err(|e| Error::from_reason(e.to_string()))?;
        serde_json::to_string(&fps).map_err(|e| Error::from_reason(e.to_string()))
    }

    // ── Domain Blocker ────────────────────────────────────────────

    /// Block a list of domains (JSON array). Returns total blocked count.
    #[napi]
    pub async fn block_domains(&self, domains: String) -> Result<u32> {
        let list: Vec<String> =
            serde_json::from_str(&domains).map_err(|e| Error::from_reason(e.to_string()))?;
        let guard: TokioMutexGuard<Option<onecrawl_cdp::Page>> = self.page.lock().await;
        let page = guard
            .as_ref()
            .ok_or_else(|| Error::from_reason("no page"))?;
        let count = onecrawl_cdp::domain_blocker::block_domains(page, &list)
            .await
            .map_err(|e| Error::from_reason(e.to_string()))?;
        Ok(count as u32)
    }

    /// Block domains by category (ads, trackers, social, fonts, media). Returns total count.
    #[napi]
    pub async fn block_category(&self, category: String) -> Result<u32> {
        let guard: TokioMutexGuard<Option<onecrawl_cdp::Page>> = self.page.lock().await;
        let page = guard
            .as_ref()
            .ok_or_else(|| Error::from_reason("no page"))?;
        let count = onecrawl_cdp::domain_blocker::block_category(page, &category)
            .await
            .map_err(|e| Error::from_reason(e.to_string()))?;
        Ok(count as u32)
    }

    /// Get blocking statistics as JSON.
    #[napi]
    pub async fn block_stats(&self) -> Result<String> {
        let guard: TokioMutexGuard<Option<onecrawl_cdp::Page>> = self.page.lock().await;
        let page = guard
            .as_ref()
            .ok_or_else(|| Error::from_reason("no page"))?;
        let stats = onecrawl_cdp::domain_blocker::block_stats(page)
            .await
            .map_err(|e| Error::from_reason(e.to_string()))?;
        serde_json::to_string(&stats).map_err(|e| Error::from_reason(e.to_string()))
    }

    /// Clear all blocked domains.
    #[napi]
    pub async fn clear_blocks(&self) -> Result<()> {
        let guard: TokioMutexGuard<Option<onecrawl_cdp::Page>> = self.page.lock().await;
        let page = guard
            .as_ref()
            .ok_or_else(|| Error::from_reason("no page"))?;
        onecrawl_cdp::domain_blocker::clear_blocks(page)
            .await
            .map_err(|e| Error::from_reason(e.to_string()))
    }

    /// List currently blocked domains as JSON array.
    #[napi]
    pub async fn list_blocked(&self) -> Result<String> {
        let guard: TokioMutexGuard<Option<onecrawl_cdp::Page>> = self.page.lock().await;
        let page = guard
            .as_ref()
            .ok_or_else(|| Error::from_reason("no page"))?;
        let domains = onecrawl_cdp::domain_blocker::list_blocked(page)
            .await
            .map_err(|e| Error::from_reason(e.to_string()))?;
        serde_json::to_string(&domains).map_err(|e| Error::from_reason(e.to_string()))
    }

    /// Get available block categories and their domain counts as JSON.
    #[napi]
    pub fn available_block_categories(&self) -> Result<String> {
        let cats = onecrawl_cdp::domain_blocker::available_categories();
        serde_json::to_string(&cats).map_err(|e| Error::from_reason(e.to_string()))
    }

    // ── Shell ─────────────────────────────────────────────────────

    /// Parse a shell command string. Returns JSON.
    #[napi]
    pub fn shell_parse(&self, input: String) -> Result<String> {
        let cmd = onecrawl_cdp::shell::parse_command(&input);
        serde_json::to_string(&cmd).map_err(|e| Error::from_reason(e.to_string()))
    }

    /// Get available shell commands. Returns JSON.
    #[napi]
    pub fn shell_commands(&self) -> Result<String> {
        let cmds = onecrawl_cdp::shell::available_commands();
        serde_json::to_string(&cmds).map_err(|e| Error::from_reason(e.to_string()))
    }

    /// Save shell history (JSON) to file.
    #[napi]
    pub fn shell_save_history(&self, history: String, path: String) -> Result<()> {
        let h: onecrawl_cdp::ShellHistory =
            serde_json::from_str(&history).map_err(|e| Error::from_reason(e.to_string()))?;
        onecrawl_cdp::shell::save_history(&h, std::path::Path::new(&path))
            .map_err(|e| Error::from_reason(e.to_string()))
    }

    /// Load shell history from file. Returns JSON.
    #[napi]
    pub fn shell_load_history(&self, path: String) -> Result<String> {
        let h = onecrawl_cdp::shell::load_history(std::path::Path::new(&path))
            .map_err(|e| Error::from_reason(e.to_string()))?;
        serde_json::to_string(&h).map_err(|e| Error::from_reason(e.to_string()))
    }

    // ── Streaming Extractor ───────────────────────────────────────

    /// Extract structured items from the page using a JSON schema. Returns JSON ExtractionResult.
    #[napi(js_name = "extractItems")]
    pub async fn extract_items(&self, schema_json: String) -> Result<String> {
        let schema: onecrawl_cdp::ExtractionSchema =
            serde_json::from_str(&schema_json).map_err(|e| Error::from_reason(e.to_string()))?;
        let guard: TokioMutexGuard<Option<onecrawl_cdp::Page>> = self.page.lock().await;
        let page = guard
            .as_ref()
            .ok_or_else(|| Error::from_reason("no page"))?;
        let result = onecrawl_cdp::streaming::extract_items(page, &schema)
            .await
            .map_err(|e| Error::from_reason(e.to_string()))?;
        serde_json::to_string(&result).map_err(|e| Error::from_reason(e.to_string()))
    }

    /// Extract items with pagination. Returns JSON ExtractionResult.
    #[napi(js_name = "extractWithPagination")]
    pub async fn extract_with_pagination(&self, schema_json: String) -> Result<String> {
        let schema: onecrawl_cdp::ExtractionSchema =
            serde_json::from_str(&schema_json).map_err(|e| Error::from_reason(e.to_string()))?;
        let guard: TokioMutexGuard<Option<onecrawl_cdp::Page>> = self.page.lock().await;
        let page = guard
            .as_ref()
            .ok_or_else(|| Error::from_reason("no page"))?;
        let result = onecrawl_cdp::streaming::extract_with_pagination(page, &schema)
            .await
            .map_err(|e| Error::from_reason(e.to_string()))?;
        serde_json::to_string(&result).map_err(|e| Error::from_reason(e.to_string()))
    }

    /// Extract a single item from the page (no item_selector). Returns JSON object.
    #[napi(js_name = "extractSingle")]
    pub async fn extract_single(&self, rules_json: String) -> Result<String> {
        let rules: Vec<onecrawl_cdp::ExtractionRule> =
            serde_json::from_str(&rules_json).map_err(|e| Error::from_reason(e.to_string()))?;
        let guard: TokioMutexGuard<Option<onecrawl_cdp::Page>> = self.page.lock().await;
        let page = guard
            .as_ref()
            .ok_or_else(|| Error::from_reason("no page"))?;
        let result = onecrawl_cdp::streaming::extract_single(page, &rules)
            .await
            .map_err(|e| Error::from_reason(e.to_string()))?;
        serde_json::to_string(&result).map_err(|e| Error::from_reason(e.to_string()))
    }

    /// Export extracted items as CSV. Returns number of items written.
    #[napi(js_name = "exportCsv")]
    pub fn export_csv(&self, items_json: String, path: String) -> Result<u32> {
        let items: Vec<onecrawl_cdp::ExtractedItem> =
            serde_json::from_str(&items_json).map_err(|e| Error::from_reason(e.to_string()))?;
        let count = onecrawl_cdp::streaming::export_csv(&items, std::path::Path::new(&path))
            .map_err(|e| Error::from_reason(e.to_string()))?;
        Ok(count as u32)
    }

    /// Export extracted items as JSON file. Returns number of items written.
    #[napi(js_name = "exportJson")]
    pub fn export_json(&self, items_json: String, path: String) -> Result<u32> {
        let items: Vec<onecrawl_cdp::ExtractedItem> =
            serde_json::from_str(&items_json).map_err(|e| Error::from_reason(e.to_string()))?;
        let count = onecrawl_cdp::streaming::export_json(&items, std::path::Path::new(&path))
            .map_err(|e| Error::from_reason(e.to_string()))?;
        Ok(count as u32)
    }

    // ── HTTP Client ───────────────────────────────────────────────

    /// Execute an HTTP request via browser fetch. Returns JSON HttpResponse.
    #[napi(js_name = "httpFetch")]
    pub async fn http_fetch(&self, request_json: String) -> Result<String> {
        let request: onecrawl_cdp::HttpRequest =
            serde_json::from_str(&request_json).map_err(|e| Error::from_reason(e.to_string()))?;
        let guard: TokioMutexGuard<Option<onecrawl_cdp::Page>> = self.page.lock().await;
        let page = guard
            .as_ref()
            .ok_or_else(|| Error::from_reason("no page"))?;
        let resp = onecrawl_cdp::http_client::fetch(page, &request)
            .await
            .map_err(|e| Error::from_reason(e.to_string()))?;
        serde_json::to_string(&resp).map_err(|e| Error::from_reason(e.to_string()))
    }

    /// HTTP GET via browser fetch. Returns JSON HttpResponse.
    #[napi(js_name = "httpGet")]
    pub async fn http_get(&self, url: String, headers_json: Option<String>) -> Result<String> {
        let headers: Option<std::collections::HashMap<String, String>> = match headers_json {
            Some(h) => {
                Some(serde_json::from_str(&h).map_err(|e| Error::from_reason(e.to_string()))?)
            }
            None => None,
        };
        let guard: TokioMutexGuard<Option<onecrawl_cdp::Page>> = self.page.lock().await;
        let page = guard
            .as_ref()
            .ok_or_else(|| Error::from_reason("no page"))?;
        let resp = onecrawl_cdp::http_client::get(page, &url, headers)
            .await
            .map_err(|e| Error::from_reason(e.to_string()))?;
        serde_json::to_string(&resp).map_err(|e| Error::from_reason(e.to_string()))
    }

    /// HTTP POST via browser fetch. Returns JSON HttpResponse.
    #[napi(js_name = "httpPost")]
    pub async fn http_post(
        &self,
        url: String,
        body: String,
        content_type: Option<String>,
        headers_json: Option<String>,
    ) -> Result<String> {
        let headers: Option<std::collections::HashMap<String, String>> = match headers_json {
            Some(h) => {
                Some(serde_json::from_str(&h).map_err(|e| Error::from_reason(e.to_string()))?)
            }
            None => None,
        };
        let ct = content_type.as_deref().unwrap_or("application/json");
        let guard: TokioMutexGuard<Option<onecrawl_cdp::Page>> = self.page.lock().await;
        let page = guard
            .as_ref()
            .ok_or_else(|| Error::from_reason("no page"))?;
        let resp = onecrawl_cdp::http_client::post(page, &url, &body, ct, headers)
            .await
            .map_err(|e| Error::from_reason(e.to_string()))?;
        serde_json::to_string(&resp).map_err(|e| Error::from_reason(e.to_string()))
    }

    /// HTTP HEAD via browser fetch. Returns JSON HttpResponse.
    #[napi(js_name = "httpHead")]
    pub async fn http_head(&self, url: String) -> Result<String> {
        let guard: TokioMutexGuard<Option<onecrawl_cdp::Page>> = self.page.lock().await;
        let page = guard
            .as_ref()
            .ok_or_else(|| Error::from_reason("no page"))?;
        let resp = onecrawl_cdp::http_client::head(page, &url)
            .await
            .map_err(|e| Error::from_reason(e.to_string()))?;
        serde_json::to_string(&resp).map_err(|e| Error::from_reason(e.to_string()))
    }

    /// Fetch a URL and parse as JSON. Returns the parsed JSON value.
    #[napi(js_name = "httpFetchJson")]
    pub async fn http_fetch_json(&self, url: String) -> Result<String> {
        let guard: TokioMutexGuard<Option<onecrawl_cdp::Page>> = self.page.lock().await;
        let page = guard
            .as_ref()
            .ok_or_else(|| Error::from_reason("no page"))?;
        let val = onecrawl_cdp::http_client::fetch_json(page, &url)
            .await
            .map_err(|e| Error::from_reason(e.to_string()))?;
        serde_json::to_string(&val).map_err(|e| Error::from_reason(e.to_string()))
    }

    // ──────────────── TLS Fingerprint ────────────────

    /// List available TLS fingerprint profile names. Returns JSON array.
    #[napi(js_name = "fingerprintProfiles")]
    pub fn fingerprint_profiles(&self) -> Result<String> {
        let profiles = onecrawl_cdp::tls_fingerprint::browser_profiles();
        serde_json::to_string(&profiles).map_err(|e| Error::from_reason(e.to_string()))
    }

    /// Apply a named TLS fingerprint profile. Returns JSON array of overridden properties.
    #[napi(js_name = "applyFingerprint")]
    pub async fn apply_fingerprint(&self, name: String) -> Result<String> {
        let guard: TokioMutexGuard<Option<onecrawl_cdp::Page>> = self.page.lock().await;
        let page = guard
            .as_ref()
            .ok_or_else(|| Error::from_reason("no page"))?;
        let fp = onecrawl_cdp::tls_fingerprint::get_profile(&name)
            .ok_or_else(|| Error::from_reason(format!("unknown fingerprint profile: {name}")))?;
        let overridden = onecrawl_cdp::tls_fingerprint::apply_fingerprint(page, &fp)
            .await
            .map_err(|e| Error::from_reason(e.to_string()))?;
        serde_json::to_string(&overridden).map_err(|e| Error::from_reason(e.to_string()))
    }

    /// Apply a random TLS fingerprint. Returns JSON of the applied fingerprint.
    #[napi(js_name = "applyRandomFingerprint")]
    pub async fn apply_random_fingerprint(&self) -> Result<String> {
        let guard: TokioMutexGuard<Option<onecrawl_cdp::Page>> = self.page.lock().await;
        let page = guard
            .as_ref()
            .ok_or_else(|| Error::from_reason("no page"))?;
        let fp = onecrawl_cdp::tls_fingerprint::random_fingerprint();
        onecrawl_cdp::tls_fingerprint::apply_fingerprint(page, &fp)
            .await
            .map_err(|e| Error::from_reason(e.to_string()))?;
        serde_json::to_string(&fp).map_err(|e| Error::from_reason(e.to_string()))
    }

    /// Detect current browser fingerprint. Returns JSON.
    #[napi(js_name = "detectFingerprint")]
    pub async fn detect_fingerprint(&self) -> Result<String> {
        let guard: TokioMutexGuard<Option<onecrawl_cdp::Page>> = self.page.lock().await;
        let page = guard
            .as_ref()
            .ok_or_else(|| Error::from_reason("no page"))?;
        let fp = onecrawl_cdp::tls_fingerprint::detect_fingerprint(page)
            .await
            .map_err(|e| Error::from_reason(e.to_string()))?;
        serde_json::to_string(&fp).map_err(|e| Error::from_reason(e.to_string()))
    }

    /// Apply a custom fingerprint from JSON string. Returns JSON array of overridden properties.
    #[napi(js_name = "applyCustomFingerprint")]
    pub async fn apply_custom_fingerprint(&self, json: String) -> Result<String> {
        let guard: TokioMutexGuard<Option<onecrawl_cdp::Page>> = self.page.lock().await;
        let page = guard
            .as_ref()
            .ok_or_else(|| Error::from_reason("no page"))?;
        let fp: onecrawl_cdp::BrowserFingerprint = serde_json::from_str(&json)
            .map_err(|e| Error::from_reason(format!("invalid fingerprint JSON: {e}")))?;
        let overridden = onecrawl_cdp::tls_fingerprint::apply_fingerprint(page, &fp)
            .await
            .map_err(|e| Error::from_reason(e.to_string()))?;
        serde_json::to_string(&overridden).map_err(|e| Error::from_reason(e.to_string()))
    }

    // ──────────────── Page Snapshot ────────────────

    /// Take a DOM snapshot of the current page. Returns JSON.
    #[napi(js_name = "takeSnapshot")]
    pub async fn take_snapshot(&self) -> Result<String> {
        let guard: TokioMutexGuard<Option<onecrawl_cdp::Page>> = self.page.lock().await;
        let page = guard
            .as_ref()
            .ok_or_else(|| Error::from_reason("no page"))?;
        let snap = onecrawl_cdp::snapshot::take_snapshot(page)
            .await
            .map_err(|e| Error::from_reason(e.to_string()))?;
        serde_json::to_string(&snap).map_err(|e| Error::from_reason(e.to_string()))
    }

    /// Compare two snapshots (JSON strings). Returns JSON diff.
    #[napi(js_name = "compareSnapshots")]
    pub fn compare_snapshots(&self, before_json: String, after_json: String) -> Result<String> {
        let before: onecrawl_cdp::DomSnapshot = serde_json::from_str(&before_json)
            .map_err(|e| Error::from_reason(format!("invalid before snapshot: {e}")))?;
        let after: onecrawl_cdp::DomSnapshot = serde_json::from_str(&after_json)
            .map_err(|e| Error::from_reason(format!("invalid after snapshot: {e}")))?;
        let diff = onecrawl_cdp::snapshot::compare_snapshots(&before, &after);
        serde_json::to_string(&diff).map_err(|e| Error::from_reason(e.to_string()))
    }

    /// Save a snapshot (JSON string) to a file.
    #[napi(js_name = "saveSnapshot")]
    pub fn save_snapshot(&self, snapshot_json: String, path: String) -> Result<()> {
        let snap: onecrawl_cdp::DomSnapshot = serde_json::from_str(&snapshot_json)
            .map_err(|e| Error::from_reason(format!("invalid snapshot JSON: {e}")))?;
        onecrawl_cdp::snapshot::save_snapshot(&snap, std::path::Path::new(&path))
            .map_err(|e| Error::from_reason(e.to_string()))
    }

    /// Load a snapshot from a file. Returns JSON string.
    #[napi(js_name = "loadSnapshot")]
    pub fn load_snapshot(&self, path: String) -> Result<String> {
        let snap = onecrawl_cdp::snapshot::load_snapshot(std::path::Path::new(&path))
            .map_err(|e| Error::from_reason(e.to_string()))?;
        serde_json::to_string(&snap).map_err(|e| Error::from_reason(e.to_string()))
    }

    /// Watch for DOM changes at an interval. Returns JSON array of diffs.
    #[napi(js_name = "watchForChanges")]
    pub async fn watch_for_changes(
        &self,
        interval_ms: u32,
        selector: Option<String>,
        count: Option<u32>,
    ) -> Result<String> {
        let guard: TokioMutexGuard<Option<onecrawl_cdp::Page>> = self.page.lock().await;
        let page = guard
            .as_ref()
            .ok_or_else(|| Error::from_reason("no page"))?;
        let diffs = onecrawl_cdp::snapshot::watch_for_changes(
            page,
            interval_ms as u64,
            selector.as_deref(),
            count.unwrap_or(3) as usize,
        )
        .await
        .map_err(|e| Error::from_reason(e.to_string()))?;
        serde_json::to_string(&diffs).map_err(|e| Error::from_reason(e.to_string()))
    }

    // ──────────────── Rate Limiter ────────────────

    /// Set rate limiter config. Accepts optional JSON RateLimitConfig or a preset name.
    #[napi(js_name = "rateLimitSet")]
    pub async fn rate_limit_set(&self, config_or_preset: Option<String>) -> Result<String> {
        let mut rl = self.rate_limiter.lock().await;
        let config = match config_or_preset {
            Some(s) => {
                let presets = onecrawl_cdp::rate_limiter::presets();
                if let Some(cfg) = presets.get(&s) {
                    cfg.clone()
                } else {
                    serde_json::from_str(&s)
                        .map_err(|e| Error::from_reason(format!("invalid config: {e}")))?
                }
            }
            None => onecrawl_cdp::RateLimitConfig::default(),
        };
        *rl = onecrawl_cdp::RateLimitState::new(config);
        serde_json::to_string(&onecrawl_cdp::rate_limiter::get_stats(&rl))
            .map_err(|e| Error::from_reason(e.to_string()))
    }

    /// Check if a request can proceed under rate limits.
    #[napi(js_name = "rateLimitCanProceed")]
    pub async fn rate_limit_can_proceed(&self) -> Result<bool> {
        let rl = self.rate_limiter.lock().await;
        Ok(onecrawl_cdp::rate_limiter::can_proceed(&rl))
    }

    /// Record a request. Returns true if allowed, false if throttled.
    #[napi(js_name = "rateLimitRecord")]
    pub async fn rate_limit_record(&self) -> Result<bool> {
        let mut rl = self.rate_limiter.lock().await;
        Ok(onecrawl_cdp::rate_limiter::record_request(&mut rl))
    }

    /// Get ms to wait before next request is allowed.
    #[napi(js_name = "rateLimitWait")]
    pub async fn rate_limit_wait(&self) -> Result<f64> {
        let rl = self.rate_limiter.lock().await;
        Ok(onecrawl_cdp::rate_limiter::wait_duration(&rl) as f64)
    }

    /// Get rate limiter statistics. Returns JSON.
    #[napi(js_name = "rateLimitStats")]
    pub async fn rate_limit_stats(&self) -> Result<String> {
        let rl = self.rate_limiter.lock().await;
        serde_json::to_string(&onecrawl_cdp::rate_limiter::get_stats(&rl))
            .map_err(|e| Error::from_reason(e.to_string()))
    }

    /// Reset rate limiter counters.
    #[napi(js_name = "rateLimitReset")]
    pub async fn rate_limit_reset(&self) -> Result<()> {
        let mut rl = self.rate_limiter.lock().await;
        onecrawl_cdp::rate_limiter::reset(&mut rl);
        Ok(())
    }

    /// List rate limiter presets. Returns JSON map.
    #[napi(js_name = "rateLimitPresets")]
    pub fn rate_limit_presets(&self) -> Result<String> {
        serde_json::to_string(&onecrawl_cdp::rate_limiter::presets())
            .map_err(|e| Error::from_reason(e.to_string()))
    }

    // ──────────────── Retry Queue ────────────────

    /// Enqueue a URL/operation for retry. Returns the item id.
    #[napi(js_name = "retryEnqueue")]
    pub async fn retry_enqueue(
        &self,
        url: String,
        operation: String,
        payload: Option<String>,
    ) -> Result<String> {
        let mut q = self.retry_queue.lock().await;
        Ok(onecrawl_cdp::retry_queue::enqueue(
            &mut q,
            &url,
            &operation,
            payload.as_deref(),
        ))
    }

    /// Get the next item due for retry. Returns JSON RetryItem or null.
    #[napi(js_name = "retryNext")]
    pub async fn retry_next(&self) -> Result<Option<String>> {
        let mut q = self.retry_queue.lock().await;
        match onecrawl_cdp::retry_queue::get_next(&mut q) {
            Some(item) => {
                let json =
                    serde_json::to_string(item).map_err(|e| Error::from_reason(e.to_string()))?;
                Ok(Some(json))
            }
            None => Ok(None),
        }
    }

    /// Mark a retry item as successful.
    #[napi(js_name = "retrySuccess")]
    pub async fn retry_success(&self, id: String) -> Result<()> {
        let mut q = self.retry_queue.lock().await;
        onecrawl_cdp::retry_queue::mark_success(&mut q, &id);
        Ok(())
    }

    /// Mark a retry item as failed. Schedules retry or moves to completed.
    #[napi(js_name = "retryFail")]
    pub async fn retry_fail(&self, id: String, error: String) -> Result<()> {
        let mut q = self.retry_queue.lock().await;
        onecrawl_cdp::retry_queue::mark_failure(&mut q, &id, &error);
        Ok(())
    }

    /// Get retry queue statistics. Returns JSON.
    #[napi(js_name = "retryStats")]
    pub async fn retry_stats(&self) -> Result<String> {
        let q = self.retry_queue.lock().await;
        serde_json::to_string(&onecrawl_cdp::retry_queue::get_stats(&q))
            .map_err(|e| Error::from_reason(e.to_string()))
    }

    /// Clear completed items from the retry queue. Returns count of removed items.
    #[napi(js_name = "retryClear")]
    pub async fn retry_clear(&self) -> Result<u32> {
        let mut q = self.retry_queue.lock().await;
        Ok(onecrawl_cdp::retry_queue::clear_completed(&mut q) as u32)
    }

    /// Save the retry queue to a file.
    #[napi(js_name = "retrySave")]
    pub async fn retry_save(&self, path: String) -> Result<()> {
        let q = self.retry_queue.lock().await;
        onecrawl_cdp::retry_queue::save_queue(&q, std::path::Path::new(&path))
            .map_err(|e| Error::from_reason(e.to_string()))
    }

    /// Load the retry queue from a file.
    #[napi(js_name = "retryLoad")]
    pub async fn retry_load(&self, path: String) -> Result<()> {
        let loaded = onecrawl_cdp::retry_queue::load_queue(std::path::Path::new(&path))
            .map_err(|e| Error::from_reason(e.to_string()))?;
        let mut q = self.retry_queue.lock().await;
        *q = loaded;
        Ok(())
    }

    // ──────────────── Data Pipeline ────────────────

    /// Execute a data pipeline. Accepts pipeline JSON and items JSON array.
    /// Returns PipelineResult JSON.
    #[napi(js_name = "pipelineExecute")]
    pub fn pipeline_execute(&self, pipeline_json: String, items_json: String) -> Result<String> {
        let pipeline: onecrawl_cdp::Pipeline = serde_json::from_str(&pipeline_json)
            .map_err(|e| Error::from_reason(format!("invalid pipeline JSON: {e}")))?;
        let items: Vec<std::collections::HashMap<String, String>> =
            serde_json::from_str(&items_json)
                .map_err(|e| Error::from_reason(format!("invalid items JSON: {e}")))?;
        let result = onecrawl_cdp::data_pipeline::execute_pipeline(&pipeline, items);
        serde_json::to_string(&result).map_err(|e| Error::from_reason(e.to_string()))
    }

    /// Validate a pipeline configuration. Returns JSON array of error strings.
    #[napi(js_name = "pipelineValidate")]
    pub fn pipeline_validate(&self, pipeline_json: String) -> Result<String> {
        let pipeline: onecrawl_cdp::Pipeline = serde_json::from_str(&pipeline_json)
            .map_err(|e| Error::from_reason(format!("invalid pipeline JSON: {e}")))?;
        let errors = onecrawl_cdp::data_pipeline::validate_pipeline(&pipeline);
        serde_json::to_string(&errors).map_err(|e| Error::from_reason(e.to_string()))
    }

    /// Save a pipeline definition to a JSON file.
    #[napi(js_name = "pipelineSave")]
    pub fn pipeline_save(&self, pipeline_json: String, path: String) -> Result<()> {
        let pipeline: onecrawl_cdp::Pipeline = serde_json::from_str(&pipeline_json)
            .map_err(|e| Error::from_reason(format!("invalid pipeline JSON: {e}")))?;
        onecrawl_cdp::data_pipeline::save_pipeline(&pipeline, std::path::Path::new(&path))
            .map_err(|e| Error::from_reason(e.to_string()))
    }

    /// Load a pipeline definition from a JSON file. Returns JSON string.
    #[napi(js_name = "pipelineLoad")]
    pub fn pipeline_load(&self, path: String) -> Result<String> {
        let pipeline = onecrawl_cdp::data_pipeline::load_pipeline(std::path::Path::new(&path))
            .map_err(|e| Error::from_reason(e.to_string()))?;
        serde_json::to_string(&pipeline).map_err(|e| Error::from_reason(e.to_string()))
    }

    /// Export pipeline results to a file. Format: "json", "jsonl", or "csv".
    /// Returns the number of items written.
    #[napi(js_name = "pipelineExport")]
    pub fn pipeline_export(
        &self,
        result_json: String,
        path: String,
        format: Option<String>,
    ) -> Result<u32> {
        let result: onecrawl_cdp::PipelineResult = serde_json::from_str(&result_json)
            .map_err(|e| Error::from_reason(format!("invalid result JSON: {e}")))?;
        let fmt = format.as_deref().unwrap_or("json");
        let count = onecrawl_cdp::data_pipeline::export_processed(
            &result,
            std::path::Path::new(&path),
            fmt,
        )
        .map_err(|e| Error::from_reason(e.to_string()))?;
        Ok(count as u32)
    }

    // ──────────────── Structured Data ────────────────

    /// Extract all structured data (JSON-LD, OG, Twitter, metadata). Returns JSON.
    #[napi(js_name = "structuredExtractAll")]
    pub async fn structured_extract_all(&self) -> Result<String> {
        let guard: TokioMutexGuard<Option<onecrawl_cdp::Page>> = self.page.lock().await;
        let page = guard
            .as_ref()
            .ok_or_else(|| Error::from_reason("no page"))?;
        let data = onecrawl_cdp::structured_data::extract_all(page)
            .await
            .map_err(|e| Error::from_reason(e.to_string()))?;
        serde_json::to_string(&data).map_err(|e| Error::from_reason(e.to_string()))
    }

    /// Extract JSON-LD from the current page. Returns JSON array.
    #[napi(js_name = "structuredJsonLd")]
    pub async fn structured_json_ld(&self) -> Result<String> {
        let guard: TokioMutexGuard<Option<onecrawl_cdp::Page>> = self.page.lock().await;
        let page = guard
            .as_ref()
            .ok_or_else(|| Error::from_reason("no page"))?;
        let data = onecrawl_cdp::structured_data::extract_json_ld(page)
            .await
            .map_err(|e| Error::from_reason(e.to_string()))?;
        serde_json::to_string(&data).map_err(|e| Error::from_reason(e.to_string()))
    }

    /// Extract OpenGraph metadata. Returns JSON.
    #[napi(js_name = "structuredOpenGraph")]
    pub async fn structured_open_graph(&self) -> Result<String> {
        let guard: TokioMutexGuard<Option<onecrawl_cdp::Page>> = self.page.lock().await;
        let page = guard
            .as_ref()
            .ok_or_else(|| Error::from_reason("no page"))?;
        let data = onecrawl_cdp::structured_data::extract_open_graph(page)
            .await
            .map_err(|e| Error::from_reason(e.to_string()))?;
        serde_json::to_string(&data).map_err(|e| Error::from_reason(e.to_string()))
    }

    /// Extract Twitter Card metadata. Returns JSON.
    #[napi(js_name = "structuredTwitterCard")]
    pub async fn structured_twitter_card(&self) -> Result<String> {
        let guard: TokioMutexGuard<Option<onecrawl_cdp::Page>> = self.page.lock().await;
        let page = guard
            .as_ref()
            .ok_or_else(|| Error::from_reason("no page"))?;
        let data = onecrawl_cdp::structured_data::extract_twitter_card(page)
            .await
            .map_err(|e| Error::from_reason(e.to_string()))?;
        serde_json::to_string(&data).map_err(|e| Error::from_reason(e.to_string()))
    }

    /// Extract page metadata (title, description, canonical, etc). Returns JSON.
    #[napi(js_name = "structuredMetadata")]
    pub async fn structured_metadata(&self) -> Result<String> {
        let guard: TokioMutexGuard<Option<onecrawl_cdp::Page>> = self.page.lock().await;
        let page = guard
            .as_ref()
            .ok_or_else(|| Error::from_reason("no page"))?;
        let data = onecrawl_cdp::structured_data::extract_metadata(page)
            .await
            .map_err(|e| Error::from_reason(e.to_string()))?;
        serde_json::to_string(&data).map_err(|e| Error::from_reason(e.to_string()))
    }

    /// Validate structured data completeness. Returns JSON array of warnings.
    #[napi(js_name = "structuredValidate")]
    pub fn structured_validate(&self, data_json: String) -> Result<String> {
        let data: onecrawl_cdp::StructuredDataResult = serde_json::from_str(&data_json)
            .map_err(|e| Error::from_reason(format!("invalid data JSON: {e}")))?;
        let warnings = onecrawl_cdp::structured_data::validate_schema(&data);
        serde_json::to_string(&warnings).map_err(|e| Error::from_reason(e.to_string()))
    }

    // ── Proxy Health ────────────────────────────────────────────────

    /// Check a single proxy health via browser fetch. Returns JSON.
    #[napi(js_name = "proxyHealthCheck")]
    pub async fn proxy_health_check(
        &self,
        proxy_url: String,
        config_json: Option<String>,
    ) -> Result<String> {
        let guard: TokioMutexGuard<Option<onecrawl_cdp::Page>> = self.page.lock().await;
        let page = guard
            .as_ref()
            .ok_or_else(|| Error::from_reason("no page"))?;
        let config: onecrawl_cdp::ProxyHealthConfig = match config_json {
            Some(ref j) => serde_json::from_str(j)
                .map_err(|e| Error::from_reason(format!("invalid config JSON: {e}")))?,
            None => onecrawl_cdp::ProxyHealthConfig::default(),
        };
        let result = onecrawl_cdp::proxy_health::check_proxy(page, &proxy_url, &config)
            .await
            .map_err(|e| Error::from_reason(e.to_string()))?;
        serde_json::to_string(&result).map_err(|e| Error::from_reason(e.to_string()))
    }

    /// Check multiple proxies. Returns JSON array.
    #[napi(js_name = "proxyHealthCheckAll")]
    pub async fn proxy_health_check_all(&self, proxies_json: String) -> Result<String> {
        let guard: TokioMutexGuard<Option<onecrawl_cdp::Page>> = self.page.lock().await;
        let page = guard
            .as_ref()
            .ok_or_else(|| Error::from_reason("no page"))?;
        let proxies: Vec<String> = serde_json::from_str(&proxies_json)
            .map_err(|e| Error::from_reason(format!("invalid proxies JSON: {e}")))?;
        let config = onecrawl_cdp::ProxyHealthConfig::default();
        let results = onecrawl_cdp::proxy_health::check_proxies(page, &proxies, &config)
            .await
            .map_err(|e| Error::from_reason(e.to_string()))?;
        serde_json::to_string(&results).map_err(|e| Error::from_reason(e.to_string()))
    }

    /// Score a single proxy health result. Returns the score (0-100).
    #[napi(js_name = "proxyHealthScore")]
    pub fn proxy_health_score(&self, result_json: String) -> Result<u32> {
        let result: onecrawl_cdp::ProxyHealthResult = serde_json::from_str(&result_json)
            .map_err(|e| Error::from_reason(format!("invalid result JSON: {e}")))?;
        Ok(onecrawl_cdp::proxy_health::score_proxy(&result))
    }

    /// Filter proxy results by minimum score. Returns JSON array.
    #[napi(js_name = "proxyHealthFilter")]
    pub fn proxy_health_filter(&self, results_json: String, min_score: u32) -> Result<String> {
        let results: Vec<onecrawl_cdp::ProxyHealthResult> = serde_json::from_str(&results_json)
            .map_err(|e| Error::from_reason(format!("invalid results JSON: {e}")))?;
        let filtered = onecrawl_cdp::proxy_health::filter_healthy(&results, min_score);
        serde_json::to_string(&filtered).map_err(|e| Error::from_reason(e.to_string()))
    }

    /// Rank proxy results by score descending. Returns JSON array.
    #[napi(js_name = "proxyHealthRank")]
    pub fn proxy_health_rank(&self, results_json: String) -> Result<String> {
        let results: Vec<onecrawl_cdp::ProxyHealthResult> = serde_json::from_str(&results_json)
            .map_err(|e| Error::from_reason(format!("invalid results JSON: {e}")))?;
        let ranked = onecrawl_cdp::proxy_health::rank_proxies(&results);
        serde_json::to_string(&ranked).map_err(|e| Error::from_reason(e.to_string()))
    }

    // ── Captcha ─────────────────────────────────────────────────────

    /// Detect CAPTCHA presence on the current page. Returns JSON.
    #[napi(js_name = "captchaDetect")]
    pub async fn captcha_detect(&self) -> Result<String> {
        let guard: TokioMutexGuard<Option<onecrawl_cdp::Page>> = self.page.lock().await;
        let page = guard
            .as_ref()
            .ok_or_else(|| Error::from_reason("no page"))?;
        let detection = onecrawl_cdp::captcha::detect_captcha(page)
            .await
            .map_err(|e| Error::from_reason(e.to_string()))?;
        serde_json::to_string(&detection).map_err(|e| Error::from_reason(e.to_string()))
    }

    /// Wait for a CAPTCHA to appear. Returns JSON.
    #[napi(js_name = "captchaWait")]
    pub async fn captcha_wait(&self, timeout_ms: Option<f64>) -> Result<String> {
        let guard: TokioMutexGuard<Option<onecrawl_cdp::Page>> = self.page.lock().await;
        let page = guard
            .as_ref()
            .ok_or_else(|| Error::from_reason("no page"))?;
        let timeout = timeout_ms.unwrap_or(30000.0) as u64;
        let detection = onecrawl_cdp::captcha::wait_for_captcha(page, timeout)
            .await
            .map_err(|e| Error::from_reason(e.to_string()))?;
        serde_json::to_string(&detection).map_err(|e| Error::from_reason(e.to_string()))
    }

    /// Screenshot CAPTCHA element. Returns rect JSON or base64.
    #[napi(js_name = "captchaScreenshot")]
    pub async fn captcha_screenshot(&self) -> Result<String> {
        let guard: TokioMutexGuard<Option<onecrawl_cdp::Page>> = self.page.lock().await;
        let page = guard
            .as_ref()
            .ok_or_else(|| Error::from_reason("no page"))?;
        let detection = onecrawl_cdp::captcha::detect_captcha(page)
            .await
            .map_err(|e| Error::from_reason(e.to_string()))?;
        if !detection.detected {
            return Err(Error::from_reason("no captcha detected"));
        }
        let data = onecrawl_cdp::captcha::screenshot_captcha(page, &detection)
            .await
            .map_err(|e| Error::from_reason(e.to_string()))?;
        Ok(data)
    }

    /// Inject a CAPTCHA solution token. Returns true if successful.
    #[napi(js_name = "captchaInject")]
    pub async fn captcha_inject(&self, solution: String) -> Result<bool> {
        let guard: TokioMutexGuard<Option<onecrawl_cdp::Page>> = self.page.lock().await;
        let page = guard
            .as_ref()
            .ok_or_else(|| Error::from_reason("no page"))?;
        let detection = onecrawl_cdp::captcha::detect_captcha(page)
            .await
            .map_err(|e| Error::from_reason(e.to_string()))?;
        if !detection.detected {
            return Err(Error::from_reason("no captcha detected"));
        }
        onecrawl_cdp::captcha::inject_solution(page, &detection, &solution)
            .await
            .map_err(|e| Error::from_reason(e.to_string()))
    }

    /// List supported CAPTCHA types. Returns JSON array of [type, description].
    #[napi(js_name = "captchaTypes")]
    pub fn captcha_types(&self) -> Result<String> {
        let types = onecrawl_cdp::captcha::supported_types();
        serde_json::to_string(&types).map_err(|e| Error::from_reason(e.to_string()))
    }

    // ──────────────── Task Scheduler ────────────────

    /// Add a scheduled task. Returns the task ID.
    #[napi(js_name = "schedulerAddTask")]
    pub async fn scheduler_add_task(
        &self,
        name: String,
        task_type: String,
        config: String,
        schedule_json: String,
    ) -> Result<String> {
        let schedule: onecrawl_cdp::TaskSchedule = serde_json::from_str(&schedule_json)
            .map_err(|e| Error::from_reason(format!("invalid schedule JSON: {e}")))?;
        let mut sched = self.scheduler.lock().await;
        Ok(onecrawl_cdp::scheduler::add_task(
            &mut sched, &name, &task_type, &config, schedule,
        ))
    }

    /// Remove a scheduled task by ID.
    #[napi(js_name = "schedulerRemoveTask")]
    pub async fn scheduler_remove_task(&self, id: String) -> Result<bool> {
        let mut sched = self.scheduler.lock().await;
        Ok(onecrawl_cdp::scheduler::remove_task(&mut sched, &id))
    }

    /// Pause a scheduled task by ID.
    #[napi(js_name = "schedulerPauseTask")]
    pub async fn scheduler_pause_task(&self, id: String) -> Result<bool> {
        let mut sched = self.scheduler.lock().await;
        Ok(onecrawl_cdp::scheduler::pause_task(&mut sched, &id))
    }

    /// Resume a paused task by ID.
    #[napi(js_name = "schedulerResumeTask")]
    pub async fn scheduler_resume_task(&self, id: String) -> Result<bool> {
        let mut sched = self.scheduler.lock().await;
        Ok(onecrawl_cdp::scheduler::resume_task(&mut sched, &id))
    }

    /// Get tasks that are due to execute. Returns JSON array.
    #[napi(js_name = "schedulerGetDueTasks")]
    pub async fn scheduler_get_due_tasks(&self) -> Result<String> {
        let sched = self.scheduler.lock().await;
        let due = onecrawl_cdp::scheduler::get_due_tasks(&sched);
        serde_json::to_string(&due).map_err(|e| Error::from_reason(e.to_string()))
    }

    /// Record a task execution result. Input is JSON of TaskResult.
    #[napi(js_name = "schedulerRecordResult")]
    pub async fn scheduler_record_result(&self, result_json: String) -> Result<()> {
        let result: onecrawl_cdp::TaskResult = serde_json::from_str(&result_json)
            .map_err(|e| Error::from_reason(format!("invalid result JSON: {e}")))?;
        let mut sched = self.scheduler.lock().await;
        onecrawl_cdp::scheduler::record_result(&mut sched, result);
        Ok(())
    }

    /// Get scheduler statistics. Returns JSON map.
    #[napi(js_name = "schedulerGetStats")]
    pub async fn scheduler_get_stats(&self) -> Result<String> {
        let sched = self.scheduler.lock().await;
        let stats = onecrawl_cdp::scheduler::get_stats(&sched);
        serde_json::to_string(&stats).map_err(|e| Error::from_reason(e.to_string()))
    }

    /// List all tasks. Returns JSON array.
    #[napi(js_name = "schedulerListTasks")]
    pub async fn scheduler_list_tasks(&self) -> Result<String> {
        let sched = self.scheduler.lock().await;
        serde_json::to_string(&sched.tasks).map_err(|e| Error::from_reason(e.to_string()))
    }

    /// Save scheduler state to a file.
    #[napi(js_name = "schedulerSave")]
    pub async fn scheduler_save(&self, path: String) -> Result<()> {
        let sched = self.scheduler.lock().await;
        onecrawl_cdp::scheduler::save_scheduler(&sched, std::path::Path::new(&path))
            .map_err(|e| Error::from_reason(e.to_string()))
    }

    /// Load scheduler state from a file.
    #[napi(js_name = "schedulerLoad")]
    pub async fn scheduler_load(&self, path: String) -> Result<()> {
        let loaded = onecrawl_cdp::scheduler::load_scheduler(std::path::Path::new(&path))
            .map_err(|e| Error::from_reason(e.to_string()))?;
        let mut sched = self.scheduler.lock().await;
        *sched = loaded;
        Ok(())
    }

    // ──────────────── Session Pool ────────────────

    /// Add a session to the pool. Returns the session ID.
    #[napi(js_name = "poolAddSession")]
    pub async fn pool_add_session(
        &self,
        name: String,
        tags: Option<Vec<String>>,
    ) -> Result<String> {
        let mut pool = self.session_pool.lock().await;
        Ok(onecrawl_cdp::session_pool::add_session(
            &mut pool, &name, tags,
        ))
    }

    /// Get the next available session. Returns JSON or null.
    #[napi(js_name = "poolGetNext")]
    pub async fn pool_get_next(&self) -> Result<Option<String>> {
        let mut pool = self.session_pool.lock().await;
        match onecrawl_cdp::session_pool::get_next(&mut pool) {
            Some(s) => {
                let json =
                    serde_json::to_string(s).map_err(|e| Error::from_reason(e.to_string()))?;
                Ok(Some(json))
            }
            None => Ok(None),
        }
    }

    /// Mark a pool session as busy.
    #[napi(js_name = "poolMarkBusy")]
    pub async fn pool_mark_busy(&self, id: String) -> Result<()> {
        let mut pool = self.session_pool.lock().await;
        onecrawl_cdp::session_pool::mark_busy(&mut pool, &id);
        Ok(())
    }

    /// Mark a pool session as idle.
    #[napi(js_name = "poolMarkIdle")]
    pub async fn pool_mark_idle(&self, id: String) -> Result<()> {
        let mut pool = self.session_pool.lock().await;
        onecrawl_cdp::session_pool::mark_idle(&mut pool, &id);
        Ok(())
    }

    /// Mark a pool session as errored.
    #[napi(js_name = "poolMarkError")]
    pub async fn pool_mark_error(&self, id: String, error: String) -> Result<()> {
        let mut pool = self.session_pool.lock().await;
        onecrawl_cdp::session_pool::mark_error(&mut pool, &id, &error);
        Ok(())
    }

    /// Close a pool session.
    #[napi(js_name = "poolCloseSession")]
    pub async fn pool_close_session(&self, id: String) -> Result<()> {
        let mut pool = self.session_pool.lock().await;
        onecrawl_cdp::session_pool::close_session(&mut pool, &id);
        Ok(())
    }

    /// Get pool statistics. Returns JSON.
    #[napi(js_name = "poolGetStats")]
    pub async fn pool_get_stats(&self) -> Result<String> {
        let pool = self.session_pool.lock().await;
        let stats = onecrawl_cdp::session_pool::get_stats(&pool);
        serde_json::to_string(&stats).map_err(|e| Error::from_reason(e.to_string()))
    }

    /// Clean up idle sessions past timeout. Returns number closed.
    #[napi(js_name = "poolCleanupIdle")]
    pub async fn pool_cleanup_idle(&self) -> Result<u32> {
        let mut pool = self.session_pool.lock().await;
        Ok(onecrawl_cdp::session_pool::cleanup_idle(&mut pool) as u32)
    }

    /// Save pool state to a file.
    #[napi(js_name = "poolSave")]
    pub async fn pool_save(&self, path: String) -> Result<()> {
        let pool = self.session_pool.lock().await;
        onecrawl_cdp::session_pool::save_pool(&pool, std::path::Path::new(&path))
            .map_err(|e| Error::from_reason(e.to_string()))
    }

    /// Load pool state from a file.
    #[napi(js_name = "poolLoad")]
    pub async fn pool_load(&self, path: String) -> Result<()> {
        let loaded = onecrawl_cdp::session_pool::load_pool(std::path::Path::new(&path))
            .map_err(|e| Error::from_reason(e.to_string()))?;
        let mut pool = self.session_pool.lock().await;
        *pool = loaded;
        Ok(())
    }
}

fn parse_network_profile(name: &str) -> std::result::Result<onecrawl_cdp::NetworkProfile, String> {
    match name.to_lowercase().as_str() {
        "fast3g" | "fast-3g" => Ok(onecrawl_cdp::NetworkProfile::Fast3G),
        "slow3g" | "slow-3g" => Ok(onecrawl_cdp::NetworkProfile::Slow3G),
        "offline" => Ok(onecrawl_cdp::NetworkProfile::Offline),
        "regular4g" | "4g" => Ok(onecrawl_cdp::NetworkProfile::Regular4G),
        "wifi" => Ok(onecrawl_cdp::NetworkProfile::WiFi),
        _ => Err(format!(
            "Unknown profile: {name}. Use: fast3g, slow3g, offline, regular4g, wifi"
        )),
    }
}
