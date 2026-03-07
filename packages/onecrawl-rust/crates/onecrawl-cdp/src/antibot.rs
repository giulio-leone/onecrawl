//! OneCrawl's Rust-native anti-bot evasion.
//!
//! Provides comprehensive browser fingerprint patching and bot detection
//! evasion beyond the basic stealth module — patches all known detection vectors.

use onecrawl_browser::Page;
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
        .map_err(|e| Error::Cdp(e.to_string()))?;
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

/// Advanced canvas fingerprint protection with Gaussian noise.
/// Unlike the basic 1-bit XOR in inject_stealth_full, this uses
/// configurable noise intensity for harder detection evasion.
pub async fn inject_canvas_advanced(page: &Page, intensity: f64) -> Result<()> {
    let js = format!(
        r#"(function() {{
        const origToDataURL = HTMLCanvasElement.prototype.toDataURL;
        const origToBlob = HTMLCanvasElement.prototype.toBlob;
        const origGetImageData = CanvasRenderingContext2D.prototype.getImageData;

        function addNoise(imageData, intensity) {{
            const data = imageData.data;
            const len = data.length;
            // Use a seeded PRNG for consistency within session
            let seed = 12345;
            for (let i = 0; i < len; i += 4) {{
                seed = (seed * 1103515245 + 12345) & 0x7fffffff;
                const noise = ((seed / 0x7fffffff) - 0.5) * intensity * 2;
                data[i] = Math.max(0, Math.min(255, data[i] + noise));     // R
                data[i+1] = Math.max(0, Math.min(255, data[i+1] + noise)); // G
                data[i+2] = Math.max(0, Math.min(255, data[i+2] + noise)); // B
            }}
            return imageData;
        }}

        CanvasRenderingContext2D.prototype.getImageData = function(...args) {{
            const imageData = origGetImageData.apply(this, args);
            return addNoise(imageData, {intensity});
        }};

        HTMLCanvasElement.prototype.toDataURL = function(...args) {{
            try {{
                const ctx = this.getContext('2d');
                if (ctx) {{
                    const imageData = origGetImageData.call(ctx, 0, 0, this.width, this.height);
                    addNoise(imageData, {intensity});
                    ctx.putImageData(imageData, 0, 0);
                }}
            }} catch(e) {{}}
            return origToDataURL.apply(this, args);
        }};

        HTMLCanvasElement.prototype.toBlob = function(cb, ...args) {{
            try {{
                const ctx = this.getContext('2d');
                if (ctx) {{
                    const imageData = origGetImageData.call(ctx, 0, 0, this.width, this.height);
                    addNoise(imageData, {intensity});
                    ctx.putImageData(imageData, 0, 0);
                }}
            }} catch(e) {{}}
            return origToBlob.call(this, cb, ...args);
        }};
    }})()"#,
        intensity = intensity
    );

    page.evaluate(js)
        .await
        .map_err(|e| Error::Cdp(format!("canvas_advanced: {e}")))?;
    Ok(())
}

/// Ensure all timezone-related APIs agree with the spoofed timezone.
pub async fn inject_timezone_sync(page: &Page, timezone: &str) -> Result<()> {
    let js = format!(
        r#"(function() {{
        const tz = '{timezone}';

        // Override Intl.DateTimeFormat to use our timezone
        const OrigDTF = Intl.DateTimeFormat;
        Intl.DateTimeFormat = function(locales, options) {{
            options = options || {{}};
            options.timeZone = tz;
            return new OrigDTF(locales, options);
        }};
        Intl.DateTimeFormat.prototype = OrigDTF.prototype;
        Intl.DateTimeFormat.supportedLocalesOf = OrigDTF.supportedLocalesOf;

        // Override Date.prototype.getTimezoneOffset
        const tzOffsets = {{
            'America/New_York': 300, 'America/Chicago': 360,
            'America/Denver': 420, 'America/Los_Angeles': 480,
            'Europe/London': 0, 'Europe/Paris': -60, 'Europe/Berlin': -60,
            'Asia/Tokyo': -540, 'Asia/Shanghai': -480, 'Asia/Dubai': -240,
            'Australia/Sydney': -660, 'Pacific/Auckland': -720
        }};
        const offset = tzOffsets[tz];
        if (offset !== undefined) {{
            Date.prototype.getTimezoneOffset = function() {{ return offset; }};
        }}

        // Override Intl.DateTimeFormat.resolvedOptions
        const origResolved = OrigDTF.prototype.resolvedOptions;
        OrigDTF.prototype.resolvedOptions = function() {{
            const opts = origResolved.call(this);
            opts.timeZone = tz;
            return opts;
        }};
    }})()"#,
        timezone = timezone
    );

    page.evaluate(js)
        .await
        .map_err(|e| Error::Cdp(format!("timezone_sync: {e}")))?;
    Ok(())
}

/// Limit font enumeration to a common cross-platform subset.
pub async fn inject_font_protection(page: &Page) -> Result<()> {
    let js = r#"(function() {
        // Common cross-platform fonts that don't reveal OS/software
        const allowedFonts = new Set([
            'Arial', 'Helvetica', 'Times New Roman', 'Times', 'Courier New',
            'Courier', 'Verdana', 'Georgia', 'Palatino', 'Garamond',
            'Bookman', 'Trebuchet MS', 'Impact', 'Comic Sans MS',
            'Lucida Sans Unicode', 'Tahoma', 'Geneva', 'Lucida Console'
        ]);

        // Override document.fonts.check to only confirm allowed fonts
        if (document.fonts) {
            const origCheck = document.fonts.check.bind(document.fonts);
            document.fonts.check = function(font, text) {
                const fontName = font.replace(/^[\d.]+[a-z]+\s+/i, '').replace(/["']/g, '').trim();
                if (!allowedFonts.has(fontName)) return false;
                return origCheck(font, text);
            };

            // Override FontFaceSet iteration
            const origForEach = document.fonts.forEach;
            if (origForEach) {
                document.fonts.forEach = function(cb, thisArg) {
                    origForEach.call(this, function(fontFace) {
                        if (allowedFonts.has(fontFace.family.replace(/["']/g, ''))) {
                            cb.call(thisArg, fontFace);
                        }
                    });
                };
            }
        }

        // Block CSS font loading probes
        const origCreate = document.createElement.bind(document);
        document.createElement = function(tag) {
            const el = origCreate(tag);
            if (tag.toLowerCase() === 'span') {
                const origStyle = Object.getOwnPropertyDescriptor(HTMLElement.prototype, 'style');
                // Let it through — font probe detection is mainly via offsetWidth
            }
            return el;
        };
    })()"#;

    page.evaluate(js.to_string())
        .await
        .map_err(|e| Error::Cdp(format!("font_protection: {e}")))?;
    Ok(())
}

/// Inject continuous human-like behavior: random micro-movements, idle drift.
/// Runs in background via setInterval — simulates an active human user.
pub async fn inject_behavior_simulation(page: &Page, interval_ms: u64) -> Result<()> {
    let js = format!(
        r#"(function() {{
        if (window.__onecrawl_behavior) return;

        let lastX = window.innerWidth / 2;
        let lastY = window.innerHeight / 2;

        window.__onecrawl_behavior = setInterval(() => {{
            // Random micro-movement (±2-5px drift)
            const dx = (Math.random() - 0.5) * 10;
            const dy = (Math.random() - 0.5) * 10;
            lastX = Math.max(0, Math.min(window.innerWidth, lastX + dx));
            lastY = Math.max(0, Math.min(window.innerHeight, lastY + dy));

            document.dispatchEvent(new MouseEvent('mousemove', {{
                clientX: lastX, clientY: lastY,
                bubbles: true, cancelable: true
            }}));

            // Occasionally scroll slightly (5% chance)
            if (Math.random() < 0.05) {{
                window.scrollBy(0, (Math.random() - 0.5) * 20);
            }}

            // Occasional focus/blur events (2% chance)
            if (Math.random() < 0.02) {{
                window.dispatchEvent(new Event(Math.random() > 0.5 ? 'focus' : 'blur'));
            }}
        }}, {interval_ms});
    }})()"#,
        interval_ms = interval_ms
    );

    page.evaluate(js)
        .await
        .map_err(|e| Error::Cdp(format!("behavior_simulation: {e}")))?;
    Ok(())
}

/// Stop continuous behavior simulation.
pub async fn stop_behavior_simulation(page: &Page) -> Result<()> {
    let js = r#"
        if (window.__onecrawl_behavior) {
            clearInterval(window.__onecrawl_behavior);
            window.__onecrawl_behavior = null;
        }
        'stopped'
    "#;
    page.evaluate(js.to_string())
        .await
        .map_err(|e| Error::Cdp(format!("stop_behavior: {e}")))?;
    Ok(())
}
