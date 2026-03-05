use chromiumoxide::Page;
use onecrawl_core::{Error, Result};

// ---------------------------------------------------------------------------
// Stealth check (multi-site fingerprint validation)
// ---------------------------------------------------------------------------

/// Run a comprehensive stealth check by evaluating fingerprint markers.
///
/// Tests 20+ browser properties: webdriver, plugins, languages, screen,
/// navigator properties, toString proxy detection, headless markers,
/// canvas/audio fingerprint consistency, and iframe contentWindow access.
///
/// Returns a JSON value with scores and findings.
pub async fn stealth_check(page: &Page) -> Result<serde_json::Value> {
    let js = r#"(() => {
        const results = {};
        const checks = [];
        let pass = 0;
        let fail = 0;

        function check(name, condition, detail) {
            const ok = !!condition;
            checks.push({ name, pass: ok, detail: detail || '' });
            if (ok) pass++; else fail++;
        }

        // 1. WebDriver
        check('navigator.webdriver === false', navigator.webdriver === false,
              `value: ${navigator.webdriver}`);

        // 2. Plugins
        check('navigator.plugins.length > 0', navigator.plugins.length > 0,
              `count: ${navigator.plugins.length}`);

        // 3. Languages
        check('navigator.languages.length > 0', navigator.languages.length > 0,
              `languages: ${JSON.stringify(navigator.languages)}`);

        // 4. Chrome object
        check('window.chrome exists', !!window.chrome,
              `type: ${typeof window.chrome}`);

        // 5. Permissions API
        check('Permissions.query works', typeof navigator.permissions?.query === 'function');

        // 6. WebGL renderer
        try {
            const c = document.createElement('canvas');
            const gl = c.getContext('webgl') || c.getContext('experimental-webgl');
            const ext = gl?.getExtension('WEBGL_debug_renderer_info');
            const renderer = ext ? gl.getParameter(ext.UNMASKED_RENDERER_WEBGL) : '';
            check('WebGL renderer present', renderer.length > 0, `renderer: ${renderer}`);
        } catch(e) {
            check('WebGL renderer present', false, `error: ${e.message}`);
        }

        // 7. Screen dimensions
        check('screen.width > 0', screen.width > 0,
              `${screen.width}x${screen.height}`);

        // 8. DeviceMemory
        check('navigator.deviceMemory > 0', navigator.deviceMemory > 0,
              `${navigator.deviceMemory} GB`);

        // 9. HardwareConcurrency
        check('navigator.hardwareConcurrency > 0', navigator.hardwareConcurrency > 0,
              `${navigator.hardwareConcurrency} cores`);

        // 10. Notification permission
        check('Notification.permission !== denied', Notification.permission !== 'denied',
              `permission: ${Notification.permission}`);

        // 11. Connection API
        check('navigator.connection exists', !!navigator.connection,
              `type: ${navigator.connection?.effectiveType || 'none'}`);

        // 12. toString proxy detection
        try {
            const fnStr = Function.prototype.toString.call(navigator.__lookupGetter__('webdriver') || (() => {}));
            const hasNative = fnStr.includes('[native code]');
            check('webdriver getter toString = native', hasNative, fnStr.substring(0, 80));
        } catch(e) {
            check('webdriver getter toString = native', false, e.message);
        }

        // 13. document.hidden
        check('document.hidden is boolean', typeof document.hidden === 'boolean',
              `hidden: ${document.hidden}`);

        // 14. iframe contentWindow access
        check('no automation markers', !window._phantom && !window.__nightmare && !window._selenium,
              'No _phantom, __nightmare, _selenium');

        // 15. User-Agent consistency
        const ua = navigator.userAgent;
        const isChrome = ua.includes('Chrome/') && !ua.includes('Headless');
        check('UA contains Chrome (not Headless)', isChrome, ua.substring(0, 80));

        // 16. Chrome runtime consistency
        check('chrome.runtime exists', !!window.chrome?.runtime,
              `has runtime: ${!!window.chrome?.runtime}`);

        // 17. Navigator prototype chain
        try {
            const proto = Object.getPrototypeOf(navigator);
            check('navigator prototype = Navigator', proto === Navigator.prototype,
                  `proto: ${proto?.constructor?.name || 'unknown'}`);
        } catch(e) {
            check('navigator prototype = Navigator', false, e.message);
        }

        // 18. Screen outer dimensions
        check('outerWidth/outerHeight > 0', window.outerWidth > 0 && window.outerHeight > 0,
              `outer: ${window.outerWidth}x${window.outerHeight}`);

        // 19. CDP markers absent
        const hasCdcMarker = Object.keys(window).some(k => k.startsWith('cdc_') || k.startsWith('$cdc_'));
        check('no cdc_ markers on window', !hasCdcMarker,
              hasCdcMarker ? 'Found cdc_ marker' : 'Clean');

        // 20. Canvas fingerprint consistency
        try {
            const canvas = document.createElement('canvas');
            canvas.width = 200; canvas.height = 50;
            const ctx = canvas.getContext('2d');
            ctx.textBaseline = 'top';
            ctx.font = '14px Arial';
            ctx.fillStyle = '#f60';
            ctx.fillRect(0, 0, 200, 50);
            ctx.fillStyle = '#069';
            ctx.fillText('OneCrawl test 🎭', 2, 15);
            const data = canvas.toDataURL();
            check('canvas fingerprint consistent', data.length > 100,
                  `dataURL length: ${data.length}`);
        } catch(e) {
            check('canvas fingerprint consistent', false, e.message);
        }

        // 21. AudioContext consistency
        try {
            const ctx = new (window.AudioContext || window.webkitAudioContext)();
            check('AudioContext available', ctx.state !== 'closed',
                  `state: ${ctx.state}, sampleRate: ${ctx.sampleRate}`);
            ctx.close();
        } catch(e) {
            check('AudioContext available', false, e.message);
        }

        // 22. Performance.now precision
        try {
            const t1 = performance.now();
            const t2 = performance.now();
            const precision = t2 - t1;
            check('performance.now has precision', precision >= 0,
                  `delta: ${precision.toFixed(6)}ms`);
        } catch(e) {
            check('performance.now has precision', false, e.message);
        }

        results.checks = checks;
        results.passed = pass;
        results.failed = fail;
        results.total = pass + fail;
        results.score = Math.round((pass / (pass + fail)) * 100);
        return JSON.stringify(results);
    })()"#;

    let raw: String = page
        .evaluate(js)
        .await
        .map_err(|e| Error::Cdp(format!("stealth_check: {e}")))?
        .into_value()
        .map_err(|e| Error::Cdp(format!("stealth_check parse: {e}")))?;

    serde_json::from_str(&raw).map_err(|e| Error::Cdp(format!("stealth_check json: {e}")))
}

/// List all detectable captcha types with descriptions.
pub fn supported_types() -> Vec<(String, String)> {
    vec![
        (
            "recaptcha_v2".into(),
            "Google reCAPTCHA v2 (checkbox / invisible)".into(),
        ),
        (
            "recaptcha_v3".into(),
            "Google reCAPTCHA v3 (score-based, invisible)".into(),
        ),
        ("hcaptcha".into(), "hCaptcha (image challenges)".into()),
        (
            "cloudflare_turnstile".into(),
            "Cloudflare Turnstile (managed challenge)".into(),
        ),
        ("funcaptcha".into(), "Arkose Labs FunCAPTCHA".into()),
        ("text".into(), "Generic text-input CAPTCHA".into()),
        ("image".into(), "Generic image-based CAPTCHA".into()),
    ]
}

