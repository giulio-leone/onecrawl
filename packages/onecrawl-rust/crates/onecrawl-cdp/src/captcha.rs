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
}
