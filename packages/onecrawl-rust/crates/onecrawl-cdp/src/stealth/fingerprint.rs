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
    "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/134.0.0.0 Safari/537.36",
    "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/134.0.0.0 Safari/537.36",
    "Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/133.0.0.0 Safari/537.36",
    "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/133.0.0.0 Safari/537.36",
    "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/132.0.0.0 Safari/537.36",
    "Mozilla/5.0 (Macintosh; Intel Mac OS X 10.15; rv:134.0) Gecko/20100101 Firefox/134.0",
    "Mozilla/5.0 (Windows NT 10.0; Win64; x64; rv:133.0) Gecko/20100101 Firefox/133.0",
    "Mozilla/5.0 (Macintosh; Intel Mac OS X 14_5) AppleWebKit/605.1.15 (KHTML, like Gecko) Version/18.1 Safari/605.1.15",
];

#[allow(dead_code)]
const PLATFORMS: &[&str] = &[
    "Win32",
    "MacIntel",
    "Linux x86_64",
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

const LANGUAGES: &[(&str, &[&str])] = &[
    ("en-US", &["en-US", "en"]),
    ("en-GB", &["en-GB", "en"]),
    ("it-IT", &["it-IT", "it", "en-US", "en"]),
    ("de-DE", &["de-DE", "de", "en-US", "en"]),
    ("fr-FR", &["fr-FR", "fr", "en-US", "en"]),
    ("es-ES", &["es-ES", "es", "en-US", "en"]),
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
    ("Intel Inc.", "ANGLE (Intel, Intel(R) UHD Graphics 630, OpenGL 4.1)"),
    ("Google Inc. (NVIDIA)", "ANGLE (NVIDIA, NVIDIA GeForce GTX 1650, OpenGL 4.5)"),
    ("Google Inc. (NVIDIA)", "ANGLE (NVIDIA, NVIDIA GeForce RTX 3060, OpenGL 4.5)"),
    ("Google Inc. (AMD)", "ANGLE (AMD, AMD Radeon Pro 5500M, OpenGL 4.1)"),
    ("Google Inc. (Intel)", "ANGLE (Intel, Intel(R) UHD Graphics, OpenGL 4.1)"),
];

const HARDWARE_CONCURRENCY: &[u32] = &[4, 6, 8, 12, 16];
const DEVICE_MEMORY: &[u32] = &[4, 8, 16, 32];
const DEVICE_SCALE_FACTORS: &[f64] = &[1.0, 1.25, 1.5, 2.0];

/// Generate a randomized, internally-consistent fingerprint.
pub fn generate_fingerprint() -> Fingerprint {
    let mut rng = rand::rng();

    let ua = USER_AGENTS[rng.random_range(0..USER_AGENTS.len())];
    let platform = if ua.contains("Macintosh") || ua.contains("Mac OS X") {
        "MacIntel"
    } else if ua.contains("Windows") {
        "Win32"
    } else {
        "Linux x86_64"
    };

    let (vw, vh) = VIEWPORTS[rng.random_range(0..VIEWPORTS.len())];
    let (lang, langs) = LANGUAGES[rng.random_range(0..LANGUAGES.len())];
    let (gl_vendor, gl_renderer) = WEBGL_CONFIGS[rng.random_range(0..WEBGL_CONFIGS.len())];

    Fingerprint {
        user_agent: ua.to_string(),
        platform: platform.to_string(),
        viewport_width: vw,
        viewport_height: vh,
        device_scale_factor: DEVICE_SCALE_FACTORS[rng.random_range(0..DEVICE_SCALE_FACTORS.len())],
        language: lang.to_string(),
        languages: langs.iter().map(|s| s.to_string()).collect(),
        timezone: TIMEZONES[rng.random_range(0..TIMEZONES.len())].to_string(),
        webgl_vendor: gl_vendor.to_string(),
        webgl_renderer: gl_renderer.to_string(),
        hardware_concurrency: HARDWARE_CONCURRENCY[rng.random_range(0..HARDWARE_CONCURRENCY.len())],
        device_memory: DEVICE_MEMORY[rng.random_range(0..DEVICE_MEMORY.len())],
    }
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
