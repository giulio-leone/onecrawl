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

pub(super) const DETECT_CAPTCHA_JS: &str = r#"
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
