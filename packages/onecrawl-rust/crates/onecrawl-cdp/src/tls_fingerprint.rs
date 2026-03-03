//! Browser TLS fingerprint impersonation via CDP.
//!
//! Overrides JavaScript-visible navigator, screen, and WebGL properties
//! to match specific browser profiles.

use chromiumoxide::Page;
use onecrawl_core::{Error, Result};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BrowserFingerprint {
    pub name: String,
    pub user_agent: String,
    pub platform: String,
    pub vendor: String,
    pub app_version: String,
    pub oscpu: String,
    pub languages: Vec<String>,
    pub hardware_concurrency: u32,
    pub device_memory: f64,
    pub max_touch_points: u32,
    pub screen_width: u32,
    pub screen_height: u32,
    pub color_depth: u32,
    pub pixel_ratio: f64,
    pub webgl_vendor: String,
    pub webgl_renderer: String,
}

/// Return predefined browser fingerprint profiles.
pub fn browser_profiles() -> Vec<BrowserFingerprint> {
    vec![
        BrowserFingerprint {
            name: "chrome-win".into(),
            user_agent: "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/124.0.0.0 Safari/537.36".into(),
            platform: "Win32".into(),
            vendor: "Google Inc.".into(),
            app_version: "5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/124.0.0.0 Safari/537.36".into(),
            oscpu: String::new(),
            languages: vec!["en-US".into(), "en".into()],
            hardware_concurrency: 8,
            device_memory: 8.0,
            max_touch_points: 0,
            screen_width: 1920,
            screen_height: 1080,
            color_depth: 24,
            pixel_ratio: 1.0,
            webgl_vendor: "Google Inc. (NVIDIA)".into(),
            webgl_renderer: "ANGLE (NVIDIA, NVIDIA GeForce GTX 1660 SUPER Direct3D11 vs_5_0 ps_5_0, D3D11)".into(),
        },
        BrowserFingerprint {
            name: "chrome-mac".into(),
            user_agent: "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/124.0.0.0 Safari/537.36".into(),
            platform: "MacIntel".into(),
            vendor: "Google Inc.".into(),
            app_version: "5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/124.0.0.0 Safari/537.36".into(),
            oscpu: String::new(),
            languages: vec!["en-US".into(), "en".into()],
            hardware_concurrency: 10,
            device_memory: 8.0,
            max_touch_points: 0,
            screen_width: 2560,
            screen_height: 1440,
            color_depth: 30,
            pixel_ratio: 2.0,
            webgl_vendor: "Google Inc. (Apple)".into(),
            webgl_renderer: "ANGLE (Apple, Apple M1 Pro, OpenGL 4.1)".into(),
        },
        BrowserFingerprint {
            name: "firefox-win".into(),
            user_agent: "Mozilla/5.0 (Windows NT 10.0; Win64; x64; rv:125.0) Gecko/20100101 Firefox/125.0".into(),
            platform: "Win32".into(),
            vendor: String::new(),
            app_version: "5.0 (Windows)".into(),
            oscpu: "Windows NT 10.0; Win64; x64".into(),
            languages: vec!["en-US".into(), "en".into()],
            hardware_concurrency: 8,
            device_memory: 8.0,
            max_touch_points: 0,
            screen_width: 1920,
            screen_height: 1080,
            color_depth: 24,
            pixel_ratio: 1.0,
            webgl_vendor: "Mozilla".into(),
            webgl_renderer: "Mozilla".into(),
        },
        BrowserFingerprint {
            name: "firefox-mac".into(),
            user_agent: "Mozilla/5.0 (Macintosh; Intel Mac OS X 10.15; rv:125.0) Gecko/20100101 Firefox/125.0".into(),
            platform: "MacIntel".into(),
            vendor: String::new(),
            app_version: "5.0 (Macintosh)".into(),
            oscpu: "Intel Mac OS X 10.15".into(),
            languages: vec!["en-US".into(), "en".into()],
            hardware_concurrency: 10,
            device_memory: 8.0,
            max_touch_points: 0,
            screen_width: 2560,
            screen_height: 1440,
            color_depth: 30,
            pixel_ratio: 2.0,
            webgl_vendor: "Mozilla".into(),
            webgl_renderer: "Mozilla".into(),
        },
        BrowserFingerprint {
            name: "safari-mac".into(),
            user_agent: "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/605.1.15 (KHTML, like Gecko) Version/17.4 Safari/605.1.15".into(),
            platform: "MacIntel".into(),
            vendor: "Apple Computer, Inc.".into(),
            app_version: "5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/605.1.15 (KHTML, like Gecko) Version/17.4 Safari/605.1.15".into(),
            oscpu: String::new(),
            languages: vec!["en-US".into()],
            hardware_concurrency: 10,
            device_memory: 8.0,
            max_touch_points: 0,
            screen_width: 2560,
            screen_height: 1440,
            color_depth: 30,
            pixel_ratio: 2.0,
            webgl_vendor: "Apple Inc.".into(),
            webgl_renderer: "Apple GPU".into(),
        },
        BrowserFingerprint {
            name: "edge-win".into(),
            user_agent: "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/124.0.0.0 Safari/537.36 Edg/124.0.0.0".into(),
            platform: "Win32".into(),
            vendor: "Google Inc.".into(),
            app_version: "5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/124.0.0.0 Safari/537.36 Edg/124.0.0.0".into(),
            oscpu: String::new(),
            languages: vec!["en-US".into(), "en".into()],
            hardware_concurrency: 8,
            device_memory: 8.0,
            max_touch_points: 0,
            screen_width: 1920,
            screen_height: 1080,
            color_depth: 24,
            pixel_ratio: 1.0,
            webgl_vendor: "Google Inc. (NVIDIA)".into(),
            webgl_renderer: "ANGLE (NVIDIA, NVIDIA GeForce RTX 3060 Direct3D11 vs_5_0 ps_5_0, D3D11)".into(),
        },
    ]
}

/// Apply a browser fingerprint profile to the page via `Object.defineProperty` overrides.
pub async fn apply_fingerprint(page: &Page, fp: &BrowserFingerprint) -> Result<Vec<String>> {
    let langs_json = serde_json::to_string(&fp.languages)
        .map_err(|e| Error::Browser(format!("serialize languages: {e}")))?;

    let js = format!(
        r#"(() => {{
const def = (obj, prop, val) => Object.defineProperty(obj, prop, {{get: () => val, configurable: true}});
def(navigator, 'userAgent', {ua});
def(navigator, 'platform', {platform});
def(navigator, 'vendor', {vendor});
def(navigator, 'appVersion', {app_version});
def(navigator, 'oscpu', {oscpu});
def(navigator, 'languages', {languages});
def(navigator, 'language', {language});
def(navigator, 'hardwareConcurrency', {hw});
def(navigator, 'deviceMemory', {dm});
def(navigator, 'maxTouchPoints', {mtp});
def(screen, 'width', {sw});
def(screen, 'height', {sh});
def(screen, 'colorDepth', {cd});
def(window, 'devicePixelRatio', {pr});
const getParam = WebGLRenderingContext.prototype.getParameter;
WebGLRenderingContext.prototype.getParameter = function(p) {{
    if (p === 0x9245) return {wv};
    if (p === 0x9246) return {wr};
    return getParam.call(this, p);
}};
if (typeof WebGL2RenderingContext !== 'undefined') {{
    const getParam2 = WebGL2RenderingContext.prototype.getParameter;
    WebGL2RenderingContext.prototype.getParameter = function(p) {{
        if (p === 0x9245) return {wv};
        if (p === 0x9246) return {wr};
        return getParam2.call(this, p);
    }};
}}
return ['userAgent','platform','vendor','appVersion','oscpu','languages','hardwareConcurrency','deviceMemory','maxTouchPoints','screen.width','screen.height','colorDepth','devicePixelRatio','webgl_vendor','webgl_renderer'];
}})()"#,
        ua = serde_json::to_string(&fp.user_agent).unwrap_or_default(),
        platform = serde_json::to_string(&fp.platform).unwrap_or_default(),
        vendor = serde_json::to_string(&fp.vendor).unwrap_or_default(),
        app_version = serde_json::to_string(&fp.app_version).unwrap_or_default(),
        oscpu = serde_json::to_string(&fp.oscpu).unwrap_or_default(),
        languages = langs_json,
        language = serde_json::to_string(fp.languages.first().map_or("en-US", |s| s.as_str()))
            .unwrap_or_default(),
        hw = fp.hardware_concurrency,
        dm = fp.device_memory,
        mtp = fp.max_touch_points,
        sw = fp.screen_width,
        sh = fp.screen_height,
        cd = fp.color_depth,
        pr = fp.pixel_ratio,
        wv = serde_json::to_string(&fp.webgl_vendor).unwrap_or_default(),
        wr = serde_json::to_string(&fp.webgl_renderer).unwrap_or_default(),
    );

    let val = page
        .evaluate(js)
        .await
        .map_err(|e| Error::Browser(format!("apply_fingerprint failed: {e}")))?;

    let overridden: Vec<String> =
        serde_json::from_value(val.into_value().unwrap_or(serde_json::json!([])))
            .unwrap_or_default();

    // Also set the CDP user-agent override so network requests match
    let ua_params =
        chromiumoxide::cdp::browser_protocol::emulation::SetUserAgentOverrideParams::new(
            &fp.user_agent,
        );
    page.execute(ua_params)
        .await
        .map_err(|e| Error::Browser(format!("SetUserAgentOverride failed: {e}")))?;

    Ok(overridden)
}

/// Generate a pseudo-random fingerprint by mixing realistic values from the profile set.
/// Uses system time modulo to pick values without requiring the `rand` crate.
pub fn random_fingerprint() -> BrowserFingerprint {
    let profiles = browser_profiles();
    let ts = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();

    let pick = |i: usize| &profiles[((ts as usize).wrapping_add(i * 7)) % profiles.len()];

    let base = pick(0);
    let hw_src = pick(1);
    let screen_src = pick(2);
    let webgl_src = pick(3);

    BrowserFingerprint {
        name: "random".into(),
        user_agent: base.user_agent.clone(),
        platform: base.platform.clone(),
        vendor: base.vendor.clone(),
        app_version: base.app_version.clone(),
        oscpu: base.oscpu.clone(),
        languages: base.languages.clone(),
        hardware_concurrency: hw_src.hardware_concurrency,
        device_memory: hw_src.device_memory,
        max_touch_points: base.max_touch_points,
        screen_width: screen_src.screen_width,
        screen_height: screen_src.screen_height,
        color_depth: screen_src.color_depth,
        pixel_ratio: screen_src.pixel_ratio,
        webgl_vendor: webgl_src.webgl_vendor.clone(),
        webgl_renderer: webgl_src.webgl_renderer.clone(),
    }
}

/// Detect the current browser fingerprint by reading JS-visible properties.
pub async fn detect_fingerprint(page: &Page) -> Result<BrowserFingerprint> {
    let js = r#"(() => {
const gl = document.createElement('canvas').getContext('webgl');
let wv = '', wr = '';
if (gl) {
    const di = gl.getExtension('WEBGL_debug_renderer_info');
    if (di) { wv = gl.getParameter(di.UNMASKED_VENDOR_WEBGL); wr = gl.getParameter(di.UNMASKED_RENDERER_WEBGL); }
}
return JSON.stringify({
    user_agent: navigator.userAgent,
    platform: navigator.platform,
    vendor: navigator.vendor || '',
    app_version: navigator.appVersion,
    oscpu: navigator.oscpu || '',
    languages: Array.from(navigator.languages || [navigator.language || 'en-US']),
    hardware_concurrency: navigator.hardwareConcurrency || 0,
    device_memory: navigator.deviceMemory || 0,
    max_touch_points: navigator.maxTouchPoints || 0,
    screen_width: screen.width,
    screen_height: screen.height,
    color_depth: screen.colorDepth,
    pixel_ratio: window.devicePixelRatio || 1,
    webgl_vendor: wv,
    webgl_renderer: wr,
});
})()"#;

    let val = page
        .evaluate(js)
        .await
        .map_err(|e| Error::Browser(format!("detect_fingerprint failed: {e}")))?;

    let json_str: String =
        serde_json::from_value(val.into_value().unwrap_or(serde_json::json!("")))
            .unwrap_or_default();

    let mut fp: BrowserFingerprint = serde_json::from_str(&json_str)
        .map_err(|e| Error::Browser(format!("parse fingerprint: {e}")))?;
    fp.name = "detected".into();
    Ok(fp)
}

/// Look up a profile by name. Returns `None` if not found.
pub fn get_profile(name: &str) -> Option<BrowserFingerprint> {
    let lower = name.to_lowercase();
    browser_profiles().into_iter().find(|p| p.name == lower)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_browser_profiles_count() {
        let profiles = browser_profiles();
        assert!(profiles.len() >= 6);
    }

    #[test]
    fn test_browser_profiles_have_valid_data() {
        for p in browser_profiles() {
            assert!(!p.name.is_empty());
            assert!(!p.user_agent.is_empty());
            assert!(!p.platform.is_empty());
            assert!(!p.languages.is_empty());
            assert!(p.hardware_concurrency > 0);
            assert!(p.device_memory > 0.0);
            assert!(p.screen_width > 0);
            assert!(p.screen_height > 0);
            assert!(p.color_depth > 0);
            assert!(p.pixel_ratio > 0.0);
        }
    }

    #[test]
    fn test_random_fingerprint() {
        let fp = random_fingerprint();
        assert_eq!(fp.name, "random");
        assert!(!fp.user_agent.is_empty());
        assert!(!fp.platform.is_empty());
        assert!(fp.hardware_concurrency > 0);
        assert!(fp.screen_width > 0);
    }

    #[test]
    fn test_get_profile_by_name() {
        assert!(get_profile("chrome-win").is_some());
        assert!(get_profile("chrome-mac").is_some());
        assert!(get_profile("firefox-win").is_some());
        assert!(get_profile("safari-mac").is_some());
        assert!(get_profile("edge-win").is_some());
    }

    #[test]
    fn test_get_profile_case_insensitive() {
        assert!(get_profile("Chrome-Win").is_some());
        assert!(get_profile("CHROME-MAC").is_some());
    }

    #[test]
    fn test_get_profile_unknown() {
        assert!(get_profile("nonexistent-browser").is_none());
    }
}
