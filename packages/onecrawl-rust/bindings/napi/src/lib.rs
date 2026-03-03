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
use tokio::sync::Mutex as TokioMutex;

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
}
