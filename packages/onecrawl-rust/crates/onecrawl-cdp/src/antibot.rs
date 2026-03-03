//! OneCrawl's Rust-native anti-bot evasion.
//!
//! Provides comprehensive browser fingerprint patching and bot detection
//! evasion beyond the basic stealth module — patches all known detection vectors.

use chromiumoxide::Page;
use onecrawl_core::{Error, Result};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AntibotProfile {
    pub name: String,
    pub patches: Vec<String>,
    /// `"basic"`, `"standard"`, or `"aggressive"`
    pub level: String,
}

/// Inject comprehensive stealth patches into the page.
pub async fn inject_stealth_full(page: &Page) -> Result<Vec<String>> {
    let patches: Vec<(&str, &str)> = vec![
        // 1. WebDriver detection
        (
            "webdriver",
            r#"
            Object.defineProperty(navigator, 'webdriver', { get: () => undefined });
            delete navigator.__proto__.webdriver;
        "#,
        ),
        // 2. Chrome runtime
        (
            "chrome_runtime",
            r#"
            window.chrome = window.chrome || {};
            window.chrome.runtime = window.chrome.runtime || {
                connect: function() {},
                sendMessage: function() {},
                id: 'aapnijgdinlhnhlmodcfapnahmbfebeb'
            };
            window.chrome.loadTimes = function() {
                return {
                    requestTime: Date.now() / 1000,
                    startLoadTime: Date.now() / 1000,
                    commitLoadTime: Date.now() / 1000,
                    finishDocumentLoadTime: Date.now() / 1000,
                    finishLoadTime: Date.now() / 1000,
                    firstPaintTime: Date.now() / 1000,
                    firstPaintAfterLoadTime: 0,
                    navigationType: 'Other',
                    wasFetchedViaSpdy: false,
                    wasNpnNegotiated: true,
                    npnNegotiatedProtocol: 'h2',
                    wasAlternateProtocolAvailable: false,
                    connectionInfo: 'h2'
                };
            };
            window.chrome.csi = function() {
                return { onloadT: Date.now(), startE: Date.now(), pageT: 0 };
            };
        "#,
        ),
        // 3. Plugins (headless Chrome has 0 plugins)
        (
            "plugins",
            r#"
            Object.defineProperty(navigator, 'plugins', {
                get: () => {
                    const plugins = [
                        { name: 'Chrome PDF Plugin', filename: 'internal-pdf-viewer', description: 'Portable Document Format' },
                        { name: 'Chrome PDF Viewer', filename: 'mhjfbmdgcfjbbpaeojofohoefgiehjai', description: '' },
                        { name: 'Native Client', filename: 'internal-nacl-plugin', description: '' }
                    ];
                    plugins.length = 3;
                    return plugins;
                }
            });
        "#,
        ),
        // 4. Languages
        (
            "languages",
            r#"
            Object.defineProperty(navigator, 'languages', {
                get: () => ['en-US', 'en'],
                configurable: true
            });
        "#,
        ),
        // 5. Permissions
        (
            "permissions",
            r#"
            const origQuery = navigator.permissions.query;
            navigator.permissions.query = (params) => {
                if (params.name === 'notifications') {
                    return Promise.resolve({ state: Notification.permission });
                }
                return origQuery.call(navigator.permissions, params);
            };
        "#,
        ),
        // 6. WebGL vendor/renderer
        (
            "webgl",
            r#"
            const getParameter = WebGLRenderingContext.prototype.getParameter;
            WebGLRenderingContext.prototype.getParameter = function(parameter) {
                if (parameter === 37445) return 'Intel Inc.';
                if (parameter === 37446) return 'Intel Iris OpenGL Engine';
                return getParameter.call(this, parameter);
            };
            if (typeof WebGL2RenderingContext !== 'undefined') {
                const getParam2 = WebGL2RenderingContext.prototype.getParameter;
                WebGL2RenderingContext.prototype.getParameter = function(parameter) {
                    if (parameter === 37445) return 'Intel Inc.';
                    if (parameter === 37446) return 'Intel Iris OpenGL Engine';
                    return getParam2.call(this, parameter);
                };
            }
        "#,
        ),
        // 7. Canvas fingerprint randomization
        (
            "canvas",
            r#"
            const origToDataURL = HTMLCanvasElement.prototype.toDataURL;
            HTMLCanvasElement.prototype.toDataURL = function(type) {
                if (type === 'image/png' || !type) {
                    const ctx = this.getContext('2d');
                    if (ctx) {
                        const imageData = ctx.getImageData(0, 0, this.width, this.height);
                        for (let i = 0; i < imageData.data.length; i += 4) {
                            imageData.data[i] ^= 1;
                        }
                        ctx.putImageData(imageData, 0, 0);
                    }
                }
                return origToDataURL.apply(this, arguments);
            };
        "#,
        ),
        // 8. AudioContext fingerprint
        (
            "audio",
            r#"
            const origCreateAnalyser = AudioContext.prototype.createAnalyser;
            AudioContext.prototype.createAnalyser = function() {
                const analyser = origCreateAnalyser.call(this);
                const origGetFloat = analyser.getFloatFrequencyData.bind(analyser);
                analyser.getFloatFrequencyData = function(array) {
                    origGetFloat(array);
                    for (let i = 0; i < array.length; i++) {
                        array[i] += (Math.random() - 0.5) * 0.0001;
                    }
                };
                return analyser;
            };
        "#,
        ),
        // 9. Iframe contentWindow
        (
            "iframe_contentwindow",
            r#"
            Object.defineProperty(HTMLIFrameElement.prototype, 'contentWindow', {
                get: function() {
                    return new Proxy(this.contentWindow || window, {
                        get: function(target, prop) {
                            if (prop === 'chrome') return window.chrome;
                            return Reflect.get(target, prop);
                        }
                    });
                }
            });
        "#,
        ),
        // 10. Screen dimensions (avoid headless defaults)
        (
            "screen",
            r#"
            Object.defineProperty(screen, 'width', { get: () => 1920 });
            Object.defineProperty(screen, 'height', { get: () => 1080 });
            Object.defineProperty(screen, 'availWidth', { get: () => 1920 });
            Object.defineProperty(screen, 'availHeight', { get: () => 1040 });
            Object.defineProperty(screen, 'colorDepth', { get: () => 24 });
            Object.defineProperty(screen, 'pixelDepth', { get: () => 24 });
        "#,
        ),
        // 11. Headless detection
        (
            "headless_detect",
            r#"
            Object.defineProperty(document, 'hidden', { get: () => false });
            Object.defineProperty(document, 'visibilityState', { get: () => 'visible' });
            window.outerWidth = window.innerWidth;
            window.outerHeight = window.innerHeight + 85;
        "#,
        ),
        // 12. Console.debug leak
        (
            "console_debug",
            r#"
            const origDebug = console.debug;
            console.debug = function() {};
        "#,
        ),
    ];

    let mut applied = Vec::new();
    for (name, js) in &patches {
        match page.evaluate(*js).await {
            Ok(_) => applied.push(name.to_string()),
            Err(e) => eprintln!("Patch {} failed: {}", name, e),
        }
    }

    Ok(applied)
}

/// Check if the page passes common bot detection tests.
pub async fn bot_detection_test(page: &Page) -> Result<serde_json::Value> {
    let js = r#"({
        webdriver: navigator.webdriver,
        chrome: !!window.chrome,
        chrome_runtime: !!window.chrome?.runtime,
        plugins_length: navigator.plugins.length,
        languages: navigator.languages,
        permissions_api: typeof navigator.permissions?.query === 'function',
        webgl_vendor: (() => {
            try {
                const canvas = document.createElement('canvas');
                const gl = canvas.getContext('webgl');
                const ext = gl?.getExtension('WEBGL_debug_renderer_info');
                return ext ? gl.getParameter(ext.UNMASKED_VENDOR_WEBGL) : 'unavailable';
            } catch(e) { return 'error'; }
        })(),
        webgl_renderer: (() => {
            try {
                const canvas = document.createElement('canvas');
                const gl = canvas.getContext('webgl');
                const ext = gl?.getExtension('WEBGL_debug_renderer_info');
                return ext ? gl.getParameter(ext.UNMASKED_RENDERER_WEBGL) : 'unavailable';
            } catch(e) { return 'error'; }
        })(),
        screen: { width: screen.width, height: screen.height, colorDepth: screen.colorDepth },
        document_hidden: document.hidden,
        visibility_state: document.visibilityState,
        has_notification: typeof Notification !== 'undefined',
        window_size_match: window.outerWidth > 0 && window.outerHeight > 0,
        connection_rtt: navigator.connection?.rtt,
        device_memory: navigator.deviceMemory,
        hardware_concurrency: navigator.hardwareConcurrency,
        score: 0
    })"#;

    let val = page
        .evaluate(js)
        .await
        .map_err(|e| Error::Browser(e.to_string()))?;
    let mut result = val.into_value().unwrap_or(serde_json::json!({}));

    // Calculate bot detection score (0 = definitely bot, 100 = appears human)
    let mut score = 0u64;
    if result.get("webdriver").and_then(|v| v.as_bool()) != Some(true) {
        score += 15;
    }
    if result.get("chrome").and_then(|v| v.as_bool()) == Some(true) {
        score += 10;
    }
    if result.get("chrome_runtime").and_then(|v| v.as_bool()) == Some(true) {
        score += 10;
    }
    if result
        .get("plugins_length")
        .and_then(|v| v.as_u64())
        .unwrap_or(0)
        > 0
    {
        score += 10;
    }
    if result
        .get("screen")
        .and_then(|v| v.get("width"))
        .and_then(|v| v.as_u64())
        .unwrap_or(0)
        > 0
    {
        score += 10;
    }
    if result.get("document_hidden").and_then(|v| v.as_bool()) != Some(true) {
        score += 10;
    }
    if result.get("visibility_state").and_then(|v| v.as_str()) == Some("visible") {
        score += 10;
    }
    if result.get("window_size_match").and_then(|v| v.as_bool()) == Some(true) {
        score += 10;
    }
    if result
        .get("hardware_concurrency")
        .and_then(|v| v.as_u64())
        .unwrap_or(0)
        > 0
    {
        score += 10;
    }
    if result
        .get("device_memory")
        .and_then(|v| v.as_f64())
        .unwrap_or(0.0)
        > 0.0
    {
        score += 5;
    }

    result["score"] = serde_json::json!(score);
    Ok(result)
}

/// Get available stealth profiles.
pub fn stealth_profiles() -> Vec<AntibotProfile> {
    vec![
        AntibotProfile {
            name: "basic".to_string(),
            patches: vec![
                "webdriver".to_string(),
                "chrome_runtime".to_string(),
                "headless_detect".to_string(),
            ],
            level: "basic".to_string(),
        },
        AntibotProfile {
            name: "standard".to_string(),
            patches: vec![
                "webdriver".to_string(),
                "chrome_runtime".to_string(),
                "plugins".to_string(),
                "languages".to_string(),
                "permissions".to_string(),
                "screen".to_string(),
                "headless_detect".to_string(),
                "console_debug".to_string(),
            ],
            level: "standard".to_string(),
        },
        AntibotProfile {
            name: "aggressive".to_string(),
            patches: vec![
                "webdriver".to_string(),
                "chrome_runtime".to_string(),
                "plugins".to_string(),
                "languages".to_string(),
                "permissions".to_string(),
                "webgl".to_string(),
                "canvas".to_string(),
                "audio".to_string(),
                "iframe_contentwindow".to_string(),
                "screen".to_string(),
                "headless_detect".to_string(),
                "console_debug".to_string(),
            ],
            level: "aggressive".to_string(),
        },
    ]
}
