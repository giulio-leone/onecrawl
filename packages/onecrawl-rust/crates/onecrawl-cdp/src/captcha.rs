//! CAPTCHA detection and solution injection framework.
//!
//! Detects common CAPTCHA providers on the current page and provides helpers
//! for injecting externally-obtained solutions.  Actual solving requires
//! third-party API services which users configure separately.

use chromiumoxide::Page;
use onecrawl_core::{Error, Result};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CaptchaDetection {
    pub detected: bool,
    /// `"recaptcha_v2"`, `"recaptcha_v3"`, `"hcaptcha"`,
    /// `"cloudflare_turnstile"`, `"funcaptcha"`, `"text"`, `"image"`,
    /// `"unknown"`, `"none"`
    pub captcha_type: String,
    pub provider: String,
    pub selector: Option<String>,
    pub sitekey: Option<String>,
    pub confidence: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CaptchaConfig {
    pub auto_detect: bool,
    pub wait_timeout_ms: u64,
    pub solver_api_key: Option<String>,
    /// `"2captcha"`, `"anticaptcha"`, `"capsolver"`
    pub solver_service: Option<String>,
}

impl Default for CaptchaConfig {
    fn default() -> Self {
        Self {
            auto_detect: true,
            wait_timeout_ms: 30000,
            solver_api_key: None,
            solver_service: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CaptchaResult {
    pub captcha_type: String,
    pub solved: bool,
    pub solution: Option<String>,
    pub duration_ms: f64,
    /// `"manual"`, `"api"`, `"bypass"`, `"none"`
    pub method: String,
}

// ── JS snippet for detection ──────────────────────────────────────

const DETECT_CAPTCHA_JS: &str = r#"
(() => {
    const result = {
        detected: false,
        captcha_type: 'none',
        provider: '',
        selector: null,
        sitekey: null,
        confidence: 0.0
    };

    // reCAPTCHA v2
    const rc2 = document.querySelector('.g-recaptcha, [data-sitekey], iframe[src*="recaptcha"]');
    if (rc2) {
        result.detected = true;
        result.captcha_type = 'recaptcha_v2';
        result.provider = 'google';
        result.confidence = 0.95;
        if (rc2.dataset && rc2.dataset.sitekey) {
            result.sitekey = rc2.dataset.sitekey;
        } else {
            const sk = document.querySelector('[data-sitekey]');
            if (sk) result.sitekey = sk.dataset.sitekey;
        }
        if (rc2.id) result.selector = '#' + rc2.id;
        else if (rc2.classList.contains('g-recaptcha')) result.selector = '.g-recaptcha';
        else result.selector = 'iframe[src*="recaptcha"]';
        return JSON.stringify(result);
    }

    // reCAPTCHA v3 (invisible — present as script)
    const rc3Script = document.querySelector('script[src*="recaptcha/api.js?render="]');
    if (rc3Script) {
        result.detected = true;
        result.captcha_type = 'recaptcha_v3';
        result.provider = 'google';
        result.confidence = 0.85;
        const m = rc3Script.src.match(/render=([^&]+)/);
        if (m) result.sitekey = m[1];
        return JSON.stringify(result);
    }

    // hCaptcha
    const hc = document.querySelector('.h-captcha, iframe[src*="hcaptcha"]');
    if (hc) {
        result.detected = true;
        result.captcha_type = 'hcaptcha';
        result.provider = 'hcaptcha';
        result.confidence = 0.95;
        if (hc.dataset && hc.dataset.sitekey) result.sitekey = hc.dataset.sitekey;
        result.selector = hc.classList.contains('h-captcha') ? '.h-captcha' : 'iframe[src*="hcaptcha"]';
        return JSON.stringify(result);
    }

    // Cloudflare Turnstile
    const cf = document.querySelector('.cf-turnstile, iframe[src*="challenges.cloudflare"]');
    if (cf) {
        result.detected = true;
        result.captcha_type = 'cloudflare_turnstile';
        result.provider = 'cloudflare';
        result.confidence = 0.90;
        if (cf.dataset && cf.dataset.sitekey) result.sitekey = cf.dataset.sitekey;
        result.selector = cf.classList.contains('cf-turnstile') ? '.cf-turnstile' : 'iframe[src*="challenges.cloudflare"]';
        return JSON.stringify(result);
    }

    // FunCAPTCHA
    const fc = document.querySelector('#FunCaptcha, iframe[src*="funcaptcha"]');
    if (fc) {
        result.detected = true;
        result.captcha_type = 'funcaptcha';
        result.provider = 'arkose';
        result.confidence = 0.90;
        result.selector = fc.id === 'FunCaptcha' ? '#FunCaptcha' : 'iframe[src*="funcaptcha"]';
        return JSON.stringify(result);
    }

    // Generic image / text captcha
    const generic = document.querySelector('img[src*="captcha"], input[name*="captcha"]');
    if (generic) {
        result.detected = true;
        const tag = generic.tagName.toLowerCase();
        result.captcha_type = tag === 'img' ? 'image' : 'text';
        result.provider = 'custom';
        result.confidence = 0.60;
        if (generic.id) result.selector = '#' + generic.id;
        else if (tag === 'img') result.selector = 'img[src*="captcha"]';
        else result.selector = 'input[name*="captcha"]';
        return JSON.stringify(result);
    }

    return JSON.stringify(result);
})()
"#;

/// Detect CAPTCHA presence and type on the current page.
pub async fn detect_captcha(page: &Page) -> Result<CaptchaDetection> {
    let raw: String = page
        .evaluate(DETECT_CAPTCHA_JS)
        .await
        .map_err(|e| Error::Cdp(format!("captcha detect eval failed: {e}")))?
        .into_value()
        .map_err(|e| Error::Cdp(format!("captcha detect parse failed: {e}")))?;

    serde_json::from_str(&raw).map_err(|e| Error::Cdp(format!("captcha json parse: {e}")))
}

/// Wait up to `timeout_ms` for a CAPTCHA to appear, polling every 500 ms.
pub async fn wait_for_captcha(page: &Page, timeout_ms: u64) -> Result<CaptchaDetection> {
    let deadline = std::time::Instant::now() + std::time::Duration::from_millis(timeout_ms);
    loop {
        let det = detect_captcha(page).await?;
        if det.detected {
            return Ok(det);
        }
        if std::time::Instant::now() >= deadline {
            return Ok(det); // returns the "none" detection
        }
        tokio::time::sleep(std::time::Duration::from_millis(500)).await;
    }
}

/// Take a base64-encoded screenshot of the captcha element.
pub async fn screenshot_captcha(page: &Page, detection: &CaptchaDetection) -> Result<String> {
    let selector = detection
        .selector
        .as_deref()
        .ok_or_else(|| Error::Cdp("no captcha selector available".into()))?;

    let js = format!(
        r#"
        (async () => {{
            const el = document.querySelector({sel});
            if (!el) return '';
            if (typeof el.scrollIntoView === 'function') el.scrollIntoView();
            // Use html2canvas-style: convert element rect to a data URL via canvas
            const rect = el.getBoundingClientRect();
            if (rect.width === 0 || rect.height === 0) return '';
            // Fallback: return bounding rect as JSON so the caller can use CDP screenshot
            return JSON.stringify({{x: rect.x, y: rect.y, w: rect.width, h: rect.height}});
        }})()
        "#,
        sel = serde_json::to_string(selector)
            .map_err(|e| Error::Cdp(format!("selector serialize: {e}")))?
    );

    let raw: String = page
        .evaluate(js.as_str())
        .await
        .map_err(|e| Error::Cdp(format!("screenshot eval failed: {e}")))?
        .into_value()
        .map_err(|e| Error::Cdp(format!("screenshot parse failed: {e}")))?;

    if raw.is_empty() {
        return Err(Error::Cdp(
            "captcha element not found or zero-size".into(),
        ));
    }

    Ok(raw)
}

/// Inject a captcha solution token into the page.
pub async fn inject_solution(
    page: &Page,
    detection: &CaptchaDetection,
    solution: &str,
) -> Result<bool> {
    let escaped_solution = serde_json::to_string(solution)
        .map_err(|e| Error::Cdp(format!("solution serialize: {e}")))?;

    let js = match detection.captcha_type.as_str() {
        "recaptcha_v2" | "recaptcha_v3" => {
            format!(
                r#"
                (() => {{
                    const token = {tok};
                    const ta = document.querySelector('#g-recaptcha-response, textarea[name="g-recaptcha-response"]');
                    if (ta) {{
                        ta.style.display = 'block';
                        ta.value = token;
                        ta.style.display = 'none';
                    }}
                    if (typeof window.___grecaptcha_cfg !== 'undefined') {{
                        const clients = window.___grecaptcha_cfg.clients || {{}};
                        for (const cid of Object.keys(clients)) {{
                            try {{
                                const c = clients[cid];
                                for (const k of Object.keys(c)) {{
                                    const v = c[k];
                                    if (v && typeof v === 'object') {{
                                        for (const kk of Object.keys(v)) {{
                                            if (v[kk] && v[kk].callback) {{
                                                v[kk].callback(token);
                                                return 'true';
                                            }}
                                        }}
                                    }}
                                }}
                            }} catch(_) {{}}
                        }}
                    }}
                    return ta ? 'true' : 'false';
                }})()
                "#,
                tok = escaped_solution
            )
        }
        "hcaptcha" => {
            format!(
                r#"
                (() => {{
                    const token = {tok};
                    const ta = document.querySelector('[name="h-captcha-response"], textarea[name="h-captcha-response"]');
                    if (ta) ta.value = token;
                    const inp = document.querySelector('[name="g-recaptcha-response"]');
                    if (inp) inp.value = token;
                    return ta ? 'true' : 'false';
                }})()
                "#,
                tok = escaped_solution
            )
        }
        "cloudflare_turnstile" => {
            format!(
                r#"
                (() => {{
                    const token = {tok};
                    const inp = document.querySelector('[name="cf-turnstile-response"], input[name="cf-turnstile-response"]');
                    if (inp) {{ inp.value = token; return 'true'; }}
                    return 'false';
                }})()
                "#,
                tok = escaped_solution
            )
        }
        "text" => {
            let sel = detection
                .selector
                .as_deref()
                .unwrap_or("input[name*=\"captcha\"]");
            let escaped_sel = serde_json::to_string(sel)
                .map_err(|e| Error::Cdp(format!("selector serialize: {e}")))?;
            format!(
                r#"
                (() => {{
                    const el = document.querySelector({sel});
                    if (el) {{ el.value = {tok}; return 'true'; }}
                    return 'false';
                }})()
                "#,
                sel = escaped_sel,
                tok = escaped_solution
            )
        }
        other => {
            return Err(Error::Cdp(format!(
                "injection not supported for captcha type: {other}"
            )));
        }
    };

    let raw: String = page
        .evaluate(js.as_str())
        .await
        .map_err(|e| Error::Cdp(format!("inject eval: {e}")))?
        .into_value()
        .map_err(|e| Error::Cdp(format!("inject parse: {e}")))?;
    Ok(raw == "true")
}

// ---------------------------------------------------------------------------
// Browser-native Turnstile solver (free — no external API)
// ---------------------------------------------------------------------------

/// Solve a Cloudflare Turnstile challenge using browser-native interaction.
///
/// Strategy:
/// 1. Find the Turnstile iframe
/// 2. Click the checkbox inside it using human-like behavior
/// 3. Wait for the challenge to auto-clear (stealth Chrome passes verification)
///
/// Returns `true` if the challenge was solved within `timeout_ms`.
pub async fn solve_turnstile_native(page: &Page, timeout_ms: u64) -> Result<bool> {
    use crate::human;

    // Step 1: Detect Turnstile iframe
    let iframe_sel: String = page
        .evaluate(
            r#"(() => {
                const cf = document.querySelector('.cf-turnstile iframe, iframe[src*="challenges.cloudflare"]');
                if (!cf) return '';
                // Tag the iframe for reliable selector
                cf.setAttribute('data-onecrawl-turnstile', '1');
                return '[data-onecrawl-turnstile="1"]';
            })()"#,
        )
        .await
        .map_err(|e| Error::Cdp(format!("turnstile detect: {e}")))?
        .into_value()
        .map_err(|e| Error::Cdp(format!("turnstile parse: {e}")))?;

    if iframe_sel.is_empty() {
        return Err(Error::Cdp(
            "No Turnstile iframe found on page".into(),
        ));
    }

    // Step 2: Click the Turnstile checkbox with human-like behavior
    human::pre_action_delay().await;
    let _ = human::human_click(page, &iframe_sel).await;
    human::post_action_delay().await;

    // Step 3: Wait for CF clearance
    Ok(human::wait_for_cf_clearance(page, timeout_ms).await)
}

// ---------------------------------------------------------------------------
// reCAPTCHA audio solver (free — uses local Whisper for transcription)
// ---------------------------------------------------------------------------

/// Solve a reCAPTCHA v2 challenge using the audio fallback + local Whisper STT.
///
/// Strategy:
/// 1. Click "I'm not a robot" checkbox
/// 2. Switch to audio challenge
/// 3. Download the audio file URL
/// 4. Transcribe using local `whisper` CLI (must be installed: `pip install openai-whisper`)
/// 5. Submit the transcription
///
/// Returns the transcription text if successful.
pub async fn solve_recaptcha_audio(page: &Page) -> Result<String> {
    use crate::human;

    // Step 1: Click the reCAPTCHA checkbox
    let checkbox_sel = r#"iframe[src*="recaptcha/api2/anchor"], iframe[title*="reCAPTCHA"]"#;
    human::human_click(page, checkbox_sel).await.map_err(|e| {
        Error::Cdp(format!("recaptcha checkbox click: {e}"))
    })?;

    // Brief wait for challenge popup to appear
    tokio::time::sleep(std::time::Duration::from_millis(2000)).await;

    // Step 2: Switch to audio challenge
    // The challenge opens in a new iframe
    let audio_btn_js = r#"(() => {
        // Find the challenge iframe
        const frames = document.querySelectorAll('iframe[src*="recaptcha/api2/bframe"]');
        for (const f of frames) {
            try {
                const doc = f.contentDocument || f.contentWindow?.document;
                if (!doc) continue;
                const btn = doc.querySelector('#recaptcha-audio-button, .rc-button-audio');
                if (btn) { btn.click(); return 'clicked'; }
            } catch(_) {}
        }
        return 'not_found';
    })()"#;

    let result: String = page
        .evaluate(audio_btn_js)
        .await
        .map_err(|e| Error::Cdp(format!("audio button: {e}")))?
        .into_value()
        .unwrap_or_default();

    if result != "clicked" {
        // Try cross-origin approach: click via selector in main frame
        let _ = page
            .evaluate(
                r#"document.querySelector('#recaptcha-audio-button, .rc-button-audio')?.click()"#,
            )
            .await;
    }

    tokio::time::sleep(std::time::Duration::from_millis(2000)).await;

    // Step 3: Get the audio URL
    let audio_url: String = page
        .evaluate(
            r#"(() => {
                const links = document.querySelectorAll('a.rc-audiochallenge-tdownload-link, audio source, #audio-source');
                for (const el of links) {
                    const href = el.href || el.src || el.getAttribute('src');
                    if (href) return href;
                }
                // Try inside iframes
                for (const f of document.querySelectorAll('iframe[src*="recaptcha"]')) {
                    try {
                        const doc = f.contentDocument || f.contentWindow?.document;
                        if (!doc) continue;
                        const link = doc.querySelector('.rc-audiochallenge-tdownload-link, audio source');
                        if (link) return link.href || link.src || '';
                    } catch(_) {}
                }
                return '';
            })()"#,
        )
        .await
        .map_err(|e| Error::Cdp(format!("audio url: {e}")))?
        .into_value()
        .unwrap_or_default();

    if audio_url.is_empty() {
        return Err(Error::Cdp(
            "Could not find reCAPTCHA audio URL. Challenge may be in a cross-origin iframe.".into(),
        ));
    }

    // Step 4: Download audio via page fetch and transcribe with local Whisper
    let audio_b64: String = page
        .evaluate(format!(
            r#"(async () => {{
                const resp = await fetch({url});
                const blob = await resp.blob();
                return new Promise(resolve => {{
                    const reader = new FileReader();
                    reader.onload = () => resolve(reader.result.split(',')[1]);
                    reader.readAsDataURL(blob);
                }});
            }})()"#,
            url = serde_json::to_string(&audio_url).unwrap_or_default()
        ))
        .await
        .map_err(|e| Error::Cdp(format!("audio download: {e}")))?
        .into_value()
        .unwrap_or_default();

    if audio_b64.is_empty() {
        return Err(Error::Cdp("Failed to download audio file".into()));
    }

    // Save audio to temp file and run Whisper
    let tmp_dir = std::env::temp_dir();
    let audio_path = tmp_dir.join("onecrawl_recaptcha_audio.mp3");
    let text_path = tmp_dir.join("onecrawl_recaptcha_audio.txt");

    // Decode base64 and save
    use std::io::Write;
    let audio_bytes = base64_decode(&audio_b64)?;
    std::fs::File::create(&audio_path)
        .and_then(|mut f| f.write_all(&audio_bytes))
        .map_err(|e| Error::Cdp(format!("save audio: {e}")))?;

    // Run Whisper CLI (must be installed: pip install openai-whisper)
    let output = std::process::Command::new("whisper")
        .args([
            audio_path.to_str().unwrap_or(""),
            "--model",
            "base",
            "--language",
            "en",
            "--output_format",
            "txt",
            "--output_dir",
            tmp_dir.to_str().unwrap_or("/tmp"),
        ])
        .output()
        .map_err(|e| Error::Cdp(format!(
            "whisper command failed (is it installed? `pip install openai-whisper`): {e}"
        )))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(Error::Cdp(format!("whisper failed: {stderr}")));
    }

    let transcription = std::fs::read_to_string(&text_path)
        .map_err(|e| Error::Cdp(format!("read whisper output: {e}")))?
        .trim()
        .to_string();

    // Cleanup temp files
    let _ = std::fs::remove_file(&audio_path);
    let _ = std::fs::remove_file(&text_path);

    if transcription.is_empty() {
        return Err(Error::Cdp("Whisper produced empty transcription".into()));
    }

    // Step 5: Submit the transcription
    let submit_js = format!(
        r#"(() => {{
            const input = document.querySelector('#audio-response, input[id="audio-response"]');
            if (!input) {{
                // Try inside iframe
                for (const f of document.querySelectorAll('iframe[src*="recaptcha"]')) {{
                    try {{
                        const doc = f.contentDocument || f.contentWindow?.document;
                        if (!doc) continue;
                        const inp = doc.querySelector('#audio-response');
                        if (inp) {{ inp.value = {text}; return 'filled'; }}
                    }} catch(_) {{}}
                }}
                return 'not_found';
            }}
            input.value = {text};
            return 'filled';
        }})()"#,
        text = serde_json::to_string(&transcription).unwrap_or_default()
    );

    let fill_result: String = page
        .evaluate(submit_js)
        .await
        .map_err(|e| Error::Cdp(format!("fill audio response: {e}")))?
        .into_value()
        .unwrap_or_default();

    if fill_result == "filled" {
        // Click verify button
        let _ = page
            .evaluate(
                r#"(() => {
                    const btn = document.querySelector('#recaptcha-verify-button, .rc-button-default');
                    if (btn) { btn.click(); return 'clicked'; }
                    for (const f of document.querySelectorAll('iframe[src*="recaptcha"]')) {
                        try {
                            const doc = f.contentDocument || f.contentWindow?.document;
                            if (!doc) continue;
                            const b = doc.querySelector('#recaptcha-verify-button, .rc-button-default');
                            if (b) { b.click(); return 'clicked'; }
                        } catch(_) {}
                    }
                    return 'not_found';
                })()"#,
            )
            .await;
    }

    Ok(transcription)
}

/// Simple base64 decoder (standard alphabet, no padding required).
fn base64_decode(input: &str) -> Result<Vec<u8>> {
    const TABLE: [u8; 128] = {
        let mut t = [255u8; 128];
        let mut i = 0u8;
        while i < 26 { t[(b'A' + i) as usize] = i; i += 1; }
        i = 0;
        while i < 26 { t[(b'a' + i) as usize] = 26 + i; i += 1; }
        i = 0;
        while i < 10 { t[(b'0' + i) as usize] = 52 + i; i += 1; }
        t[b'+' as usize] = 62;
        t[b'/' as usize] = 63;
        t
    };

    let bytes: Vec<u8> = input.bytes().filter(|&b| b != b'=' && b != b'\n' && b != b'\r').collect();
    let mut out = Vec::with_capacity(bytes.len() * 3 / 4);

    for chunk in bytes.chunks(4) {
        let mut buf = 0u32;
        let len = chunk.len();
        for (i, &b) in chunk.iter().enumerate() {
            let val = if (b as usize) < 128 { TABLE[b as usize] } else { 255 };
            if val == 255 {
                return Err(Error::Cdp(format!("invalid base64 char: {b}")));
            }
            buf |= (val as u32) << (18 - 6 * i);
        }
        if len > 1 { out.push((buf >> 16) as u8); }
        if len > 2 { out.push((buf >> 8) as u8); }
        if len > 3 { out.push(buf as u8); }
    }

    Ok(out)
}

// ---------------------------------------------------------------------------
// Stealth check (multi-site fingerprint validation)
// ---------------------------------------------------------------------------

/// Run a comprehensive stealth check by evaluating fingerprint markers.
///
/// Tests: webdriver, plugins, languages, screen, navigator properties,
/// toString proxy detection, and common headless markers.
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let cfg = CaptchaConfig::default();
        assert!(cfg.auto_detect);
        assert_eq!(cfg.wait_timeout_ms, 30000);
        assert!(cfg.solver_api_key.is_none());
        assert!(cfg.solver_service.is_none());
    }

    #[test]
    fn test_supported_types_count() {
        let types = supported_types();
        assert_eq!(types.len(), 7);
    }

    #[test]
    fn test_supported_types_names() {
        let types = supported_types();
        let names: Vec<&str> = types.iter().map(|(n, _)| n.as_str()).collect();
        assert!(names.contains(&"recaptcha_v2"));
        assert!(names.contains(&"hcaptcha"));
        assert!(names.contains(&"cloudflare_turnstile"));
        assert!(names.contains(&"funcaptcha"));
        assert!(names.contains(&"image"));
    }

    #[test]
    fn test_detection_serialize_none() {
        let det = CaptchaDetection {
            detected: false,
            captcha_type: "none".into(),
            provider: "".into(),
            selector: None,
            sitekey: None,
            confidence: 0.0,
        };
        let json = serde_json::to_string(&det).unwrap();
        assert!(json.contains("\"detected\":false"));
        let parsed: CaptchaDetection = serde_json::from_str(&json).unwrap();
        assert!(!parsed.detected);
        assert_eq!(parsed.captcha_type, "none");
    }

    #[test]
    fn test_detection_serialize_recaptcha() {
        let det = CaptchaDetection {
            detected: true,
            captcha_type: "recaptcha_v2".into(),
            provider: "google".into(),
            selector: Some(".g-recaptcha".into()),
            sitekey: Some("6Le-test".into()),
            confidence: 0.95,
        };
        let json = serde_json::to_string(&det).unwrap();
        let parsed: CaptchaDetection = serde_json::from_str(&json).unwrap();
        assert!(parsed.detected);
        assert_eq!(parsed.sitekey.as_deref(), Some("6Le-test"));
    }

    #[test]
    fn test_result_serialize() {
        let res = CaptchaResult {
            captcha_type: "hcaptcha".into(),
            solved: true,
            solution: Some("token123".into()),
            duration_ms: 1500.0,
            method: "api".into(),
        };
        let json = serde_json::to_string(&res).unwrap();
        let parsed: CaptchaResult = serde_json::from_str(&json).unwrap();
        assert!(parsed.solved);
        assert_eq!(parsed.method, "api");
    }

    #[test]
    fn test_config_serialize() {
        let cfg = CaptchaConfig {
            auto_detect: false,
            wait_timeout_ms: 5000,
            solver_api_key: Some("key123".into()),
            solver_service: Some("2captcha".into()),
        };
        let json = serde_json::to_string(&cfg).unwrap();
        let parsed: CaptchaConfig = serde_json::from_str(&json).unwrap();
        assert!(!parsed.auto_detect);
        assert_eq!(parsed.solver_service.as_deref(), Some("2captcha"));
    }

    #[test]
    fn test_detection_all_types_deserialize() {
        for captcha_type in &[
            "recaptcha_v2",
            "recaptcha_v3",
            "hcaptcha",
            "cloudflare_turnstile",
            "funcaptcha",
            "text",
            "image",
            "unknown",
            "none",
        ] {
            let json = format!(
                r#"{{"detected":true,"captcha_type":"{}","provider":"test","selector":null,"sitekey":null,"confidence":0.5}}"#,
                captcha_type
            );
            let parsed: CaptchaDetection = serde_json::from_str(&json).unwrap();
            assert_eq!(parsed.captcha_type, *captcha_type);
        }
    }

    #[test]
    fn test_base64_decode_simple() {
        // "hello" in base64 is "aGVsbG8="
        let decoded = base64_decode("aGVsbG8=").unwrap();
        assert_eq!(decoded, b"hello");
    }

    #[test]
    fn test_base64_decode_no_padding() {
        // "hello" without padding
        let decoded = base64_decode("aGVsbG8").unwrap();
        assert_eq!(decoded, b"hello");
    }

    #[test]
    fn test_base64_decode_empty() {
        let decoded = base64_decode("").unwrap();
        assert!(decoded.is_empty());
    }

    #[test]
    fn test_base64_decode_binary() {
        // Known binary: [0xFF, 0x00, 0xAB] = "/wCr"
        let decoded = base64_decode("/wCr").unwrap();
        assert_eq!(decoded, vec![0xFF, 0x00, 0xAB]);
    }

    #[test]
    fn test_base64_decode_with_newlines() {
        // Decoder should skip \n and \r
        let decoded = base64_decode("aGVs\nbG8=\r").unwrap();
        assert_eq!(decoded, b"hello");
    }

    #[test]
    fn test_base64_decode_invalid_char() {
        let result = base64_decode("aGVs!G8=");
        assert!(result.is_err());
    }
}
