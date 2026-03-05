use colored::Colorize;
use super::super::helpers::{with_page};

// Rate Limiter (standalone — no Page required)
// Retry Queue (standalone — no Page required)
// Task Scheduler (standalone — no Page required)
// Session Pool (standalone — no Page required)

pub async fn geo_apply(profile: &str) {
    let profile = profile.to_string();
    with_page(|page| async move {
        let geo: onecrawl_cdp::GeoProfile =
            if let Some(p) = onecrawl_cdp::geofencing::get_preset(&profile) {
                p
            } else {
                serde_json::from_str(&profile)
                    .map_err(|e| format!("Invalid profile name or JSON: {e}"))?
            };
        onecrawl_cdp::geofencing::apply_geo_profile(&page, &geo)
            .await
            .map_err(|e| e.to_string())?;
        println!(
            "{} Geo profile '{}' applied (lat={}, lng={})",
            "✓".green(),
            geo.name,
            geo.latitude,
            geo.longitude
        );
        Ok(())
    })
    .await;
}

pub async fn geo_presets() {
    let presets = onecrawl_cdp::geofencing::list_presets();
    for name in &presets {
        if let Some(p) = onecrawl_cdp::geofencing::get_preset(name) {
            println!(
                "  {} — lat={:.4}, lng={:.4}, tz={}",
                name.green(),
                p.latitude,
                p.longitude,
                p.timezone
            );
        }
    }
}

pub async fn geo_current() {
    with_page(|page| async move {
        let val = onecrawl_cdp::geofencing::get_current_geo(&page)
            .await
            .map_err(|e| e.to_string())?;
        println!("{}", serde_json::to_string_pretty(&val).unwrap_or_default());
        Ok(())
    })
    .await;
}

