use colored::Colorize;
use super::super::helpers::{with_page};

// on the current page (e.g. x.com Settings → Security → Passkey), then export
// the credential (including private key) to a JSON file.
//
// The credential exported here can later be injected via
// `session start --import-passkey FILE` for fully automated headless passkey auth.
// Store the passkey file path in the active session so that CDP WebAuthn is
// automatically re-enabled and credentials are injected on every
// `connect_to_session()` call (same lifecycle as stealth scripts).
// Passkey Vault (multi-site persistent store)
// Import passkeys from a 1Password export.data JSON file (extracted from .1pux).
// Import passkeys from a FIDO Alliance CXF JSON file.
pub async fn stealth_inject() {
    with_page(|page| async move {
        let fp = onecrawl_cdp::generate_fingerprint();
        let script = onecrawl_cdp::get_stealth_init_script(&fp);
        onecrawl_cdp::page::evaluate_js(&page, &script)
            .await
            .map_err(|e| e.to_string())?;
        // Also override UA
        onecrawl_cdp::emulation::set_user_agent(&page, &fp.user_agent)
            .await
            .map_err(|e| e.to_string())?;
        println!("{} Stealth patches injected", "✓".green());
        println!("  UA: {}", fp.user_agent.dimmed());
        println!("  Viewport: {}×{}", fp.viewport_width, fp.viewport_height);
        Ok(())
    })
    .await;
}

pub async fn stealth_tls_apply(profile: &str) {
    let profile = profile.to_string();
    with_page(|page| async move {
        let fp = onecrawl_cdp::tls_fingerprint::get_profile(&profile)
            .unwrap_or_else(|| onecrawl_cdp::tls_fingerprint::random_fingerprint());
        let applied = onecrawl_cdp::tls_fingerprint::apply_fingerprint(&page, &fp)
            .await
            .map_err(|e| e.to_string())?;
        println!("{} TLS profile '{}' applied", "✓".green(), profile.cyan());
        for patch in &applied {
            println!("  • {}", patch);
        }
        Ok(())
    })
    .await;
}

pub async fn stealth_webrtc_block() {
    with_page(|page| async move {
        let js = r#"
            navigator.mediaDevices.getUserMedia = undefined;
            window.RTCPeerConnection = undefined;
            window.webkitRTCPeerConnection = undefined;
            window.mozRTCPeerConnection = undefined;
            'blocked'
        "#;
        onecrawl_cdp::page::evaluate_js(&page, js)
            .await
            .map_err(|e| e.to_string())?;
        println!("{} WebRTC blocked — IP leaks prevented", "✓".green());
        Ok(())
    })
    .await;
}

pub async fn stealth_battery_spoof(level: f64, charging: bool) {
    with_page(|page| async move {
        let js = format!(
            r#"(function() {{
                const battery = {{
                    charging: {charging},
                    chargingTime: {charging_time},
                    dischargingTime: {discharging_time},
                    level: {level},
                    addEventListener: function() {{}},
                    removeEventListener: function() {{}},
                    dispatchEvent: function() {{ return true; }}
                }};
                navigator.getBattery = function() {{
                    return Promise.resolve(battery);
                }};
                'spoofed'
            }})()"#,
            charging = charging,
            charging_time = if charging { "0" } else { "Infinity" },
            discharging_time = if charging { "Infinity" } else { "3600" },
            level = level,
        );
        onecrawl_cdp::page::evaluate_js(&page, &js)
            .await
            .map_err(|e| e.to_string())?;
        println!(
            "{} BatteryManager spoofed — level: {:.0}%, charging: {}",
            "✓".green(),
            level * 100.0,
            charging
        );
        Ok(())
    })
    .await;
}

pub async fn stealth_sensor_block() {
    with_page(|page| async move {
        let js = r#"
            window.DeviceMotionEvent = undefined;
            window.DeviceOrientationEvent = undefined;
            if (window.AmbientLightSensor) window.AmbientLightSensor = undefined;
            if (window.Accelerometer) window.Accelerometer = undefined;
            if (window.Gyroscope) window.Gyroscope = undefined;
            if (window.Magnetometer) window.Magnetometer = undefined;
            'blocked'
        "#;
        onecrawl_cdp::page::evaluate_js(&page, js)
            .await
            .map_err(|e| e.to_string())?;
        println!("{} Device sensors blocked (Motion, Orientation, AmbientLight)", "✓".green());
        Ok(())
    })
    .await;
}

pub async fn stealth_canvas_advanced(intensity: f64) {
    with_page(|page| async move {
        onecrawl_cdp::antibot::inject_canvas_advanced(&page, intensity)
            .await
            .map_err(|e| e.to_string())?;
        println!(
            "{} Advanced canvas noise applied (intensity: {})",
            "✓".green(),
            format!("{intensity:.2}").cyan()
        );
        Ok(())
    })
    .await;
}

pub async fn stealth_timezone_sync(timezone: &str) {
    let tz = timezone.to_string();
    with_page(|page| async move {
        onecrawl_cdp::antibot::inject_timezone_sync(&page, &tz)
            .await
            .map_err(|e| e.to_string())?;
        println!("{} Timezone synchronized to '{}'", "✓".green(), tz.cyan());
        Ok(())
    })
    .await;
}

pub async fn stealth_font_protect() {
    with_page(|page| async move {
        onecrawl_cdp::antibot::inject_font_protection(&page)
            .await
            .map_err(|e| e.to_string())?;
        println!("{} Font fingerprinting protection enabled", "✓".green());
        Ok(())
    })
    .await;
}

pub async fn stealth_behavior_sim() {
    with_page(|page| async move {
        onecrawl_cdp::antibot::inject_behavior_simulation(&page, 200)
            .await
            .map_err(|e| e.to_string())?;
        println!("{} Behavior simulation started (200ms interval)", "✓".green());
        Ok(())
    })
    .await;
}

pub async fn stealth_behavior_stop() {
    with_page(|page| async move {
        onecrawl_cdp::antibot::stop_behavior_simulation(&page)
            .await
            .map_err(|e| e.to_string())?;
        println!("{} Behavior simulation stopped", "✓".green());
        Ok(())
    })
    .await;
}

pub async fn stealth_rotate() {
    with_page(|page| async move {
        let fp = onecrawl_cdp::tls_fingerprint::random_fingerprint();
        onecrawl_cdp::tls_fingerprint::apply_fingerprint(&page, &fp)
            .await
            .map_err(|e| e.to_string())?;
        let new_fp = onecrawl_cdp::generate_fingerprint();
        let script = onecrawl_cdp::get_stealth_init_script(&new_fp);
        onecrawl_cdp::page::evaluate_js(&page, &script)
            .await
            .map_err(|e| e.to_string())?;
        onecrawl_cdp::emulation::set_user_agent(&page, &new_fp.user_agent)
            .await
            .map_err(|e| e.to_string())?;
        println!("{} Fingerprint rotated", "✓".green());
        println!("  UA: {}", new_fp.user_agent.dimmed());
        println!("  Viewport: {}×{}", new_fp.viewport_width, new_fp.viewport_height);
        Ok(())
    })
    .await;
}

pub async fn stealth_detection_audit() {
    with_page(|page| async move {
        let js = r#"(function() {
            const results = {};
            results.webdriver = navigator.webdriver;
            results.languages = navigator.languages;
            results.plugins = navigator.plugins.length;
            results.hardwareConcurrency = navigator.hardwareConcurrency;
            results.deviceMemory = navigator.deviceMemory || 'N/A';
            results.platform = navigator.platform;
            results.userAgent = navigator.userAgent;
            results.webgl = (function() {
                try {
                    var c = document.createElement('canvas');
                    var gl = c.getContext('webgl');
                    return gl ? gl.getParameter(gl.RENDERER) : 'N/A';
                } catch(e) { return 'error'; }
            })();
            results.chrome = !!window.chrome;
            results.permissions = typeof navigator.permissions !== 'undefined';
            results.battery = typeof navigator.getBattery === 'function';
            results.webrtc = typeof window.RTCPeerConnection !== 'undefined';
            results.canvas = (function() {
                try {
                    var c = document.createElement('canvas');
                    c.width = 200; c.height = 50;
                    var ctx = c.getContext('2d');
                    ctx.fillText('test', 10, 30);
                    return c.toDataURL().length;
                } catch(e) { return 0; }
            })();
            return JSON.stringify(results);
        })()"#;
        let raw = onecrawl_cdp::page::evaluate_js(&page, js)
            .await
            .map_err(|e| e.to_string())?;

        let text = raw.to_string();
        let clean = text.trim_matches('"').replace("\\\"", "\"");
        let audit: serde_json::Value = serde_json::from_str(&clean).unwrap_or_default();

        println!("\n{} Bot Detection Audit\n", "🔍".to_string());

        let webdriver = audit["webdriver"].as_bool().unwrap_or(true);
        let icon = if webdriver { "✗".red() } else { "✓".green() };
        println!("  {} webdriver: {}", icon, webdriver);

        let plugins = audit["plugins"].as_u64().unwrap_or(0);
        let icon = if plugins > 0 { "✓".green() } else { "✗".red() };
        println!("  {} plugins: {}", icon, plugins);

        let webrtc = audit["webrtc"].as_bool().unwrap_or(true);
        let icon = if !webrtc { "✓".green() } else { "⚠".yellow() };
        println!("  {} webrtc exposed: {}", icon, webrtc);

        println!("  {} platform: {}", "ℹ".cyan(), audit["platform"].as_str().unwrap_or("?"));
        println!("  {} hardwareConcurrency: {}", "ℹ".cyan(), audit["hardwareConcurrency"]);
        println!("  {} webgl renderer: {}", "ℹ".cyan(), audit["webgl"].as_str().unwrap_or("?"));
        println!("  {} canvas hash length: {}", "ℹ".cyan(), audit["canvas"]);
        println!("  {} userAgent: {}", "ℹ".cyan(), audit["userAgent"].as_str().unwrap_or("?").dimmed());

        Ok(())
    })
    .await;
}

pub async fn stealth_check() {
    with_page(|page| async move {
        let result = onecrawl_cdp::captcha::stealth_check(&page)
            .await
            .map_err(|e| e.to_string())?;

        let score = result["score"].as_u64().unwrap_or(0);
        let passed = result["passed"].as_u64().unwrap_or(0);
        let failed = result["failed"].as_u64().unwrap_or(0);
        let total = result["total"].as_u64().unwrap_or(0);

        // Header
        let score_color = if score >= 90 {
            "✓".green()
        } else if score >= 70 {
            "⚠".yellow()
        } else {
            "✗".red()
        };
        println!(
            "\n{} Stealth Score: {}% ({}/{} checks passed)\n",
            score_color, score, passed, total
        );

        // Detail each check
        if let Some(checks) = result["checks"].as_array() {
            for check in checks {
                let name = check["name"].as_str().unwrap_or("?");
                let pass = check["pass"].as_bool().unwrap_or(false);
                let detail = check["detail"].as_str().unwrap_or("");
                let icon = if pass { "✓".green() } else { "✗".red() };
                if detail.is_empty() {
                    println!("  {} {}", icon, name);
                } else {
                    println!("  {} {} — {}", icon, name, detail.dimmed());
                }
            }
        }

        if failed > 0 {
            println!(
                "\n{} {} check(s) failed — stealth may be detectable",
                "⚠".yellow(),
                failed
            );
        } else {
            println!("\n{} All checks passed — stealth is solid", "✓".green());
        }

        Ok(())
    })
    .await;
}

