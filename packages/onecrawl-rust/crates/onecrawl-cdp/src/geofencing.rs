//! Virtual geolocation profiles with timezone/locale coordination.

use onecrawl_browser::Page;
use onecrawl_core::{Error, Result};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GeoProfile {
    pub name: String,
    pub latitude: f64,
    pub longitude: f64,
    pub accuracy: f64,
    pub timezone: String,
    pub locale: String,
    pub language: String,
}

/// Predefined geo profiles for common locations.
pub fn preset_profiles() -> Vec<GeoProfile> {
    vec![
        GeoProfile {
            name: "New York".into(),
            latitude: 40.7128,
            longitude: -74.0060,
            accuracy: 10.0,
            timezone: "America/New_York".into(),
            locale: "en-US".into(),
            language: "en-US".into(),
        },
        GeoProfile {
            name: "London".into(),
            latitude: 51.5074,
            longitude: -0.1278,
            accuracy: 10.0,
            timezone: "Europe/London".into(),
            locale: "en-GB".into(),
            language: "en-GB".into(),
        },
        GeoProfile {
            name: "Tokyo".into(),
            latitude: 35.6762,
            longitude: 139.6503,
            accuracy: 10.0,
            timezone: "Asia/Tokyo".into(),
            locale: "ja-JP".into(),
            language: "ja".into(),
        },
        GeoProfile {
            name: "Berlin".into(),
            latitude: 52.5200,
            longitude: 13.4050,
            accuracy: 10.0,
            timezone: "Europe/Berlin".into(),
            locale: "de-DE".into(),
            language: "de".into(),
        },
        GeoProfile {
            name: "Sydney".into(),
            latitude: -33.8688,
            longitude: 151.2093,
            accuracy: 10.0,
            timezone: "Australia/Sydney".into(),
            locale: "en-AU".into(),
            language: "en-AU".into(),
        },
        GeoProfile {
            name: "São Paulo".into(),
            latitude: -23.5505,
            longitude: -46.6333,
            accuracy: 10.0,
            timezone: "America/Sao_Paulo".into(),
            locale: "pt-BR".into(),
            language: "pt-BR".into(),
        },
        GeoProfile {
            name: "Mumbai".into(),
            latitude: 19.0760,
            longitude: 72.8777,
            accuracy: 10.0,
            timezone: "Asia/Kolkata".into(),
            locale: "hi-IN".into(),
            language: "hi".into(),
        },
        GeoProfile {
            name: "Dubai".into(),
            latitude: 25.2048,
            longitude: 55.2708,
            accuracy: 10.0,
            timezone: "Asia/Dubai".into(),
            locale: "ar-AE".into(),
            language: "ar".into(),
        },
    ]
}

/// Apply a geo profile (geolocation + timezone + locale override).
pub async fn apply_geo_profile(page: &Page, profile: &GeoProfile) -> Result<()> {
    let js = format!(
        r#"
        (() => {{
            // Override geolocation
            navigator.geolocation.getCurrentPosition = function(success, _error, _options) {{
                success({{
                    coords: {{
                        latitude: {lat},
                        longitude: {lng},
                        accuracy: {acc},
                        altitude: null,
                        altitudeAccuracy: null,
                        heading: null,
                        speed: null
                    }},
                    timestamp: Date.now()
                }});
            }};

            navigator.geolocation.watchPosition = function(success, _error, _options) {{
                const id = setInterval(() => {{
                    success({{
                        coords: {{
                            latitude: {lat},
                            longitude: {lng},
                            accuracy: {acc},
                            altitude: null,
                            altitudeAccuracy: null,
                            heading: null,
                            speed: null
                        }},
                        timestamp: Date.now()
                    }});
                }}, 1000);
                return id;
            }};

            // Override Intl.DateTimeFormat for timezone
            const origDTF = Intl.DateTimeFormat;
            const tz = '{tz}';
            Intl.DateTimeFormat = function(locales, options) {{
                options = options || {{}};
                options.timeZone = options.timeZone || tz;
                return new origDTF(locales || '{locale}', options);
            }};
            Intl.DateTimeFormat.prototype = origDTF.prototype;
            Object.keys(origDTF).forEach(k => {{ Intl.DateTimeFormat[k] = origDTF[k]; }});

            // Override navigator.language
            Object.defineProperty(navigator, 'language', {{ value: '{lang}', configurable: true }});
            Object.defineProperty(navigator, 'languages', {{ value: ['{lang}', '{locale}'], configurable: true }});

            return true;
        }})()
    "#,
        lat = profile.latitude,
        lng = profile.longitude,
        acc = profile.accuracy,
        tz = profile.timezone,
        locale = profile.locale,
        lang = profile.language
    );
    page.evaluate(js)
        .await
        .map_err(|e| Error::Cdp(format!("apply_geo_profile failed: {e}")))?;
    Ok(())
}

/// Get the list of available preset profiles.
pub fn list_presets() -> Vec<String> {
    preset_profiles().iter().map(|p| p.name.clone()).collect()
}

/// Get a preset profile by name.
pub fn get_preset(name: &str) -> Option<GeoProfile> {
    preset_profiles()
        .into_iter()
        .find(|p| p.name.to_lowercase() == name.to_lowercase())
}

/// Get current geolocation as seen by the page.
pub async fn get_current_geo(page: &Page) -> Result<serde_json::Value> {
    let js = r#"
        new Promise((resolve) => {
            navigator.geolocation.getCurrentPosition(
                (pos) => resolve({
                    latitude: pos.coords.latitude,
                    longitude: pos.coords.longitude,
                    accuracy: pos.coords.accuracy,
                    timezone: Intl.DateTimeFormat().resolvedOptions().timeZone,
                    language: navigator.language,
                    languages: navigator.languages
                }),
                (err) => resolve({ error: err.message }),
                { timeout: 5000 }
            );
        })
    "#;
    let val = page
        .evaluate(js)
        .await
        .map_err(|e| Error::Cdp(format!("get_current_geo failed: {e}")))?;
    Ok(val.into_value().unwrap_or(serde_json::json!({})))
}
