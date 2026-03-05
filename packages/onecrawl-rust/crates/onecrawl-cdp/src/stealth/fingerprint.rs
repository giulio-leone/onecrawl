use rand::prelude::*;
use serde::{Deserialize, Serialize};

/// A randomized browser fingerprint for stealth sessions.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Fingerprint {
    pub user_agent: String,
    pub platform: String,
    pub viewport_width: u32,
    pub viewport_height: u32,
    pub device_scale_factor: f64,
    pub language: String,
    pub languages: Vec<String>,
    pub timezone: String,
    pub webgl_vendor: String,
    pub webgl_renderer: String,
    pub hardware_concurrency: u32,
    pub device_memory: u32,
}

const USER_AGENTS: &[&str] = &[
    "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/136.0.0.0 Safari/537.36",
    "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/134.0.0.0 Safari/537.36",
    "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/133.0.0.0 Safari/537.36",
    "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/131.0.0.0 Safari/537.36",
];

const VIEWPORTS: &[(u32, u32)] = &[
    (1920, 1080),
    (1366, 768),
    (1536, 864),
    (1440, 900),
    (1280, 720),
    (2560, 1440),
    (1680, 1050),
    (1600, 900),
];

const TIMEZONES: &[&str] = &[
    "America/New_York",
    "America/Chicago",
    "America/Los_Angeles",
    "Europe/London",
    "Europe/Rome",
    "Europe/Berlin",
    "Asia/Tokyo",
    "Asia/Shanghai",
];

const WEBGL_CONFIGS: &[(&str, &str)] = &[
    ("Intel Inc.", "Intel Iris OpenGL Engine"),
    (
        "Intel Inc.",
        "ANGLE (Intel, Intel(R) UHD Graphics 630, OpenGL 4.1)",
    ),
    (
        "Google Inc. (NVIDIA)",
        "ANGLE (NVIDIA, NVIDIA GeForce GTX 1650, OpenGL 4.5)",
    ),
    (
        "Google Inc. (NVIDIA)",
        "ANGLE (NVIDIA, NVIDIA GeForce RTX 3060, OpenGL 4.5)",
    ),
    (
        "Google Inc. (AMD)",
        "ANGLE (AMD, AMD Radeon Pro 5500M, OpenGL 4.1)",
    ),
    (
        "Google Inc. (Intel)",
        "ANGLE (Intel, Intel(R) UHD Graphics, OpenGL 4.1)",
    ),
];

const HARDWARE_CONCURRENCY: &[u32] = &[4, 6, 8, 12, 16];
const DEVICE_MEMORY: &[u32] = &[4, 8, 16, 32];
const DEVICE_SCALE_FACTORS: &[f64] = &[1.0, 1.25, 1.5, 2.0];

/// Detect system locale — uses macOS UI locale (what Chrome's Intl API uses) on macOS,
/// falls back to LANG env var on other platforms.
fn get_system_locale() -> String {
    // On macOS, Chrome derives Intl.DateTimeFormat locale from the UI language set in
    // System Preferences, not from the UNIX LANG env var.
    #[cfg(target_os = "macos")]
    if let Ok(out) = std::process::Command::new("defaults")
        .args(["read", "-g", "AppleLocale"])
        .output()
    {
        let locale = String::from_utf8_lossy(&out.stdout)
            .trim()
            .replace('_', "-");
        if locale.len() >= 2 {
            return locale;
        }
    }
    // Fallback: UNIX LANG / LC_ALL env vars.
    std::env::var("LC_ALL")
        .ok()
        .filter(|s| s.len() >= 2)
        .or_else(|| std::env::var("LANG").ok().filter(|s| s.len() >= 2))
        .map(|s| s.split('.').next().unwrap_or("en_US").replace('_', "-"))
        .unwrap_or_else(|| "en-US".to_string())
}

/// Build a navigator.languages list consistent with the given locale.
fn languages_for_locale(locale: &str) -> Vec<String> {
    let primary = locale.split('-').next().unwrap_or("en").to_string();
    let mut langs: Vec<String> = vec![locale.to_string()];
    if primary != locale {
        langs.push(primary.clone());
    }
    if !locale.starts_with("en") {
        langs.push("en-US".to_string());
        langs.push("en".to_string());
    }
    langs
}

/// Generate a randomized, internally-consistent fingerprint.
/// If `real_ua` is provided, it is used as the User-Agent string (with "HeadlessChrome"
/// replaced by "Chrome" so headless mode is not disclosed). This ensures the main-thread
/// UA matches the unpatched Worker context UA, eliminating version mismatch detection.
pub fn generate_fingerprint_with_real_ua(real_ua: Option<&str>) -> Fingerprint {
    let mut rng = rand::rng();

    let ua_owned;
    let ua: &str = if let Some(rua) = real_ua {
        // Use the real browser UA, only sanitizing the headless marker.
        ua_owned = rua.replace("HeadlessChrome", "Chrome");
        &ua_owned
    } else {
        USER_AGENTS[rng.random_range(0..USER_AGENTS.len())]
    };

    let platform = if ua.contains("Macintosh") || ua.contains("Mac OS X") {
        "MacIntel"
    } else if ua.contains("Windows") {
        "Win32"
    } else {
        "Linux x86_64"
    };

    let (vw, vh) = VIEWPORTS[rng.random_range(0..VIEWPORTS.len())];
    // Use system locale so navigator.languages, Intl API, and IP geolocation all agree.
    let locale = get_system_locale();
    let languages = languages_for_locale(&locale);
    let (gl_vendor, gl_renderer) = WEBGL_CONFIGS[rng.random_range(0..WEBGL_CONFIGS.len())];

    Fingerprint {
        user_agent: ua.to_string(),
        platform: platform.to_string(),
        viewport_width: vw,
        viewport_height: vh,
        device_scale_factor: DEVICE_SCALE_FACTORS[rng.random_range(0..DEVICE_SCALE_FACTORS.len())],
        language: locale.clone(),
        languages,
        timezone: TIMEZONES[rng.random_range(0..TIMEZONES.len())].to_string(),
        webgl_vendor: gl_vendor.to_string(),
        webgl_renderer: gl_renderer.to_string(),
        hardware_concurrency: HARDWARE_CONCURRENCY[rng.random_range(0..HARDWARE_CONCURRENCY.len())],
        device_memory: DEVICE_MEMORY[rng.random_range(0..DEVICE_MEMORY.len())],
    }
}

/// Generate a randomized fingerprint (uses a random UA from the built-in list).
/// Callers that can supply the real browser UA should use `generate_fingerprint_with_real_ua`
/// instead to avoid version-mismatch detection in Worker contexts.
pub fn generate_fingerprint() -> Fingerprint {
    generate_fingerprint_with_real_ua(None)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fingerprint_consistent_platform() {
        for _ in 0..100 {
            let fp = generate_fingerprint();
            if fp.user_agent.contains("Macintosh") {
                assert_eq!(fp.platform, "MacIntel");
            } else if fp.user_agent.contains("Windows") {
                assert_eq!(fp.platform, "Win32");
            } else {
                assert_eq!(fp.platform, "Linux x86_64");
            }
        }
    }

    #[test]
    fn fingerprint_valid_viewport() {
        let fp = generate_fingerprint();
        assert!(fp.viewport_width >= 1280);
        assert!(fp.viewport_height >= 720);
    }

    #[test]
    fn fingerprint_has_languages() {
        let fp = generate_fingerprint();
        assert!(!fp.languages.is_empty());
        assert_eq!(fp.languages[0], fp.language);
    }

    #[test]
    fn fingerprints_vary() {
        let fps: Vec<Fingerprint> = (0..20).map(|_| generate_fingerprint()).collect();
        let uas: std::collections::HashSet<_> = fps.iter().map(|f| &f.user_agent).collect();
        assert!(uas.len() > 1, "all 20 fingerprints had same UA");
    }
}
