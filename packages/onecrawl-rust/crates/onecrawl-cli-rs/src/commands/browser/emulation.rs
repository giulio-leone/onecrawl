use colored::Colorize;
use super::helpers::{with_page};

// ---------------------------------------------------------------------------
// TLS Fingerprint
// ---------------------------------------------------------------------------

pub async fn emulate_viewport(width: u32, height: u32, scale: f64) {
    with_page(|page| async move {
        let vp = onecrawl_cdp::Viewport {
            width,
            height,
            device_scale_factor: scale,
            is_mobile: false,
            has_touch: false,
        };
        onecrawl_cdp::emulation::set_viewport(&page, &vp)
            .await
            .map_err(|e| e.to_string())?;
        println!(
            "{} Viewport set to {}×{} @{:.1}x",
            "✓".green(),
            width,
            height,
            scale
        );
        Ok(())
    })
    .await;
}

pub async fn emulate_device(name: &str) {
    let n = name.to_string();
    with_page(|page| async move {
        let vp = match n.as_str() {
            "iphone_14" | "iphone14" | "iphone" => onecrawl_cdp::Viewport::iphone_14(),
            "ipad" => onecrawl_cdp::Viewport::ipad(),
            "pixel_7" | "pixel7" | "pixel" => onecrawl_cdp::Viewport::pixel_7(),
            "desktop" => onecrawl_cdp::Viewport::desktop(),
            _ => {
                return Err(format!(
                    "Unknown device: {n}. Available: iphone_14, ipad, pixel_7, desktop"
                ));
            }
        };
        onecrawl_cdp::emulation::set_viewport(&page, &vp)
            .await
            .map_err(|e| e.to_string())?;
        println!(
            "{} Emulating {} ({}×{} @{:.1}x, mobile={}, touch={})",
            "✓".green(),
            n.cyan(),
            vp.width,
            vp.height,
            vp.device_scale_factor,
            vp.is_mobile,
            vp.has_touch
        );
        Ok(())
    })
    .await;
}

pub async fn emulate_user_agent(ua: &str) {
    let ua = ua.to_string();
    with_page(|page| async move {
        onecrawl_cdp::emulation::set_user_agent(&page, &ua)
            .await
            .map_err(|e| e.to_string())?;
        println!("{} User-Agent set", "✓".green());
        Ok(())
    })
    .await;
}

pub async fn emulate_geolocation(lat: f64, lon: f64, accuracy: f64) {
    with_page(|page| async move {
        onecrawl_cdp::emulation::set_geolocation(&page, lat, lon, accuracy)
            .await
            .map_err(|e| e.to_string())?;
        println!(
            "{} Geolocation set to ({}, {}) accuracy={}",
            "✓".green(),
            lat,
            lon,
            accuracy
        );
        Ok(())
    })
    .await;
}

pub async fn emulate_color_scheme(scheme: &str) {
    let s = scheme.to_string();
    with_page(|page| async move {
        onecrawl_cdp::emulation::set_color_scheme(&page, &s)
            .await
            .map_err(|e| e.to_string())?;
        println!("{} Color scheme set to {}", "✓".green(), s.cyan());
        Ok(())
    })
    .await;
}

pub async fn emulate_clear() {
    with_page(|page| async move {
        onecrawl_cdp::emulation::clear_viewport(&page)
            .await
            .map_err(|e| e.to_string())?;
        println!("{} Emulation cleared", "✓".green());
        Ok(())
    })
    .await;
}

pub async fn adv_emulation_orientation(alpha: f64, beta: f64, gamma: f64) {
    with_page(|page| async move {
        let reading = onecrawl_cdp::advanced_emulation::SensorReading { alpha, beta, gamma };
        onecrawl_cdp::advanced_emulation::set_device_orientation(&page, reading)
            .await
            .map_err(|e| e.to_string())?;
        println!(
            "{} Device orientation set (α={alpha}, β={beta}, γ={gamma})",
            "✓".green()
        );
        Ok(())
    })
    .await;
}

pub async fn adv_emulation_permission(name: &str, state: &str) {
    let n = name.to_string();
    let s = state.to_string();
    with_page(|page| async move {
        onecrawl_cdp::advanced_emulation::override_permission(&page, &n, &s)
            .await
            .map_err(|e| e.to_string())?;
        println!("{} Permission '{n}' → {s}", "✓".green());
        Ok(())
    })
    .await;
}

pub async fn adv_emulation_battery(level: f64, charging: bool) {
    with_page(|page| async move {
        onecrawl_cdp::advanced_emulation::set_battery_status(&page, level, charging)
            .await
            .map_err(|e| e.to_string())?;
        println!(
            "{} Battery: {:.0}% {}",
            "✓".green(),
            level * 100.0,
            if charging { "(charging)" } else { "" }
        );
        Ok(())
    })
    .await;
}

pub async fn adv_emulation_connection(effective_type: &str, downlink: f64, rtt: u32) {
    let et = effective_type.to_string();
    with_page(|page| async move {
        onecrawl_cdp::advanced_emulation::set_connection_info(&page, &et, downlink, rtt)
            .await
            .map_err(|e| e.to_string())?;
        println!(
            "{} Connection: {et} ↓{downlink}Mbps RTT={rtt}ms",
            "✓".green()
        );
        Ok(())
    })
    .await;
}

pub async fn adv_emulation_cpu_cores(n: u32) {
    with_page(|page| async move {
        onecrawl_cdp::advanced_emulation::set_hardware_concurrency(&page, n)
            .await
            .map_err(|e| e.to_string())?;
        println!("{} CPU cores → {n}", "✓".green());
        Ok(())
    })
    .await;
}

pub async fn adv_emulation_memory(gb: f64) {
    with_page(|page| async move {
        onecrawl_cdp::advanced_emulation::set_device_memory(&page, gb)
            .await
            .map_err(|e| e.to_string())?;
        println!("{} Device memory → {gb}GB", "✓".green());
        Ok(())
    })
    .await;
}

pub async fn adv_emulation_navigator_info() {
    with_page(|page| async move {
        let info = onecrawl_cdp::advanced_emulation::get_navigator_info(&page)
            .await
            .map_err(|e| e.to_string())?;
        println!(
            "{}",
            serde_json::to_string_pretty(&info).unwrap_or_default()
        );
        Ok(())
    })
    .await;
}

pub async fn fingerprint_apply(name: &str) {
    let n = name.to_string();
    with_page(|page| async move {
        let fp = if n == "random" {
            onecrawl_cdp::tls_fingerprint::random_fingerprint()
        } else {
            onecrawl_cdp::tls_fingerprint::get_profile(&n)
                .ok_or_else(|| format!("Unknown profile: {n}. Use: chrome-win, chrome-mac, firefox-win, firefox-mac, safari-mac, edge-win, random"))?
        };
        let overridden = onecrawl_cdp::tls_fingerprint::apply_fingerprint(&page, &fp)
            .await
            .map_err(|e| e.to_string())?;
        println!("{} Applied fingerprint: {}", "✓".green(), fp.name.cyan());
        println!("  UA: {}", fp.user_agent.dimmed());
        println!("  Platform: {}", fp.platform);
        println!("  Overridden: {}", overridden.join(", "));
        Ok(())
    })
    .await;
}

pub async fn fingerprint_detect() {
    with_page(|page| async move {
        let fp = onecrawl_cdp::tls_fingerprint::detect_fingerprint(&page)
            .await
            .map_err(|e| e.to_string())?;
        println!("{}", serde_json::to_string_pretty(&fp).unwrap_or_default());
        Ok(())
    })
    .await;
}

pub fn fingerprint_list() {
    let profiles = onecrawl_cdp::tls_fingerprint::browser_profiles();
    for p in &profiles {
        println!(
            "  {} — {} ({}×{}, {})",
            p.name.cyan(),
            p.platform,
            p.screen_width,
            p.screen_height,
            p.vendor
        );
    }
    println!("\n{} profiles available", profiles.len());
}
