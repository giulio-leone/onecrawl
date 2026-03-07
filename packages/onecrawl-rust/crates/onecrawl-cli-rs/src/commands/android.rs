use base64::Engine;
use colored::Colorize;
use onecrawl_cdp::android::{AndroidClient, AndroidSessionConfig};

/// Handle `onecrawl android devices`.
pub async fn devices() {
    match AndroidClient::list_devices().await {
        Ok(result) => println!(
            "{} Android devices:\n{}",
            "✓".green(),
            serde_json::to_string_pretty(&result).unwrap_or_default()
        ),
        Err(e) => eprintln!("{} {e}", "✗".red()),
    }
}

/// Handle `onecrawl android connect`.
pub async fn connect(
    server_url: &str,
    serial: Option<&str>,
    package: &str,
    activity: Option<&str>,
) {
    let config = AndroidSessionConfig {
        server_url: server_url.to_string(),
        device_serial: serial.map(|s| s.to_string()),
        package: package.to_string(),
        activity: activity.map(|s| s.to_string()),
    };
    let mut client = AndroidClient::new(config);
    match client.create_session(None, None).await {
        Ok(sid) => println!("{} Android session created: {}", "✓".green(), sid.cyan()),
        Err(e) => eprintln!("{} {e}", "✗".red()),
    }
}

/// Handle `onecrawl android navigate`.
pub async fn navigate(url: &str) {
    let config = AndroidSessionConfig::default();
    let client = AndroidClient::new(config);
    match client.navigate(url).await {
        Ok(()) => println!("{} Navigated to {}", "✓".green(), url.cyan()),
        Err(e) => eprintln!("{} {e}", "✗".red()),
    }
}

/// Handle `onecrawl android tap`.
pub async fn tap(x: f64, y: f64) {
    let config = AndroidSessionConfig::default();
    let client = AndroidClient::new(config);
    match client.tap(x, y).await {
        Ok(()) => println!("{} Tapped at ({}, {})", "✓".green(), x, y),
        Err(e) => eprintln!("{} {e}", "✗".red()),
    }
}

/// Handle `onecrawl android swipe`.
pub async fn swipe(from_x: f64, from_y: f64, to_x: f64, to_y: f64, duration: u64) {
    let config = AndroidSessionConfig::default();
    let client = AndroidClient::new(config);
    match client.swipe(from_x, from_y, to_x, to_y, duration).await {
        Ok(()) => println!(
            "{} Swiped ({},{}) → ({},{}) in {}ms",
            "✓".green(),
            from_x,
            from_y,
            to_x,
            to_y,
            duration
        ),
        Err(e) => eprintln!("{} {e}", "✗".red()),
    }
}

/// Handle `onecrawl android long-press`.
pub async fn long_press(x: f64, y: f64, duration: u64) {
    let config = AndroidSessionConfig::default();
    let client = AndroidClient::new(config);
    match client.long_press(x, y, duration).await {
        Ok(()) => println!(
            "{} Long pressed at ({}, {}) for {}ms",
            "✓".green(),
            x,
            y,
            duration
        ),
        Err(e) => eprintln!("{} {e}", "✗".red()),
    }
}

/// Handle `onecrawl android double-tap`.
pub async fn double_tap(x: f64, y: f64) {
    let config = AndroidSessionConfig::default();
    let client = AndroidClient::new(config);
    match client.double_tap(x, y).await {
        Ok(()) => println!("{} Double tapped at ({}, {})", "✓".green(), x, y),
        Err(e) => eprintln!("{} {e}", "✗".red()),
    }
}

/// Handle `onecrawl android pinch`.
pub async fn pinch(x: f64, y: f64, scale: f64) {
    let config = AndroidSessionConfig::default();
    let client = AndroidClient::new(config);
    match client.pinch(x, y, scale).await {
        Ok(()) => println!(
            "{} Pinched at ({}, {}) scale={}",
            "✓".green(),
            x,
            y,
            scale
        ),
        Err(e) => eprintln!("{} {e}", "✗".red()),
    }
}

/// Handle `onecrawl android type`.
pub async fn type_text(text: &str) {
    let config = AndroidSessionConfig::default();
    let client = AndroidClient::new(config);
    match client.type_text(text).await {
        Ok(()) => println!("{} Typed text", "✓".green()),
        Err(e) => eprintln!("{} {e}", "✗".red()),
    }
}

/// Handle `onecrawl android find`.
pub async fn find(strategy: &str, value: &str) {
    let config = AndroidSessionConfig::default();
    let client = AndroidClient::new(config);
    match client.find_element(strategy, value).await {
        Ok(id) => println!("{} Found element: {}", "✓".green(), id.cyan()),
        Err(e) => eprintln!("{} {e}", "✗".red()),
    }
}

/// Handle `onecrawl android click`.
pub async fn click(element_id: &str) {
    let config = AndroidSessionConfig::default();
    let client = AndroidClient::new(config);
    match client.click_element(element_id).await {
        Ok(()) => println!("{} Clicked element {}", "✓".green(), element_id.cyan()),
        Err(e) => eprintln!("{} {e}", "✗".red()),
    }
}

/// Handle `onecrawl android screenshot`.
pub async fn screenshot(output: &str) {
    let config = AndroidSessionConfig::default();
    let client = AndroidClient::new(config);
    match client.screenshot().await {
        Ok(b64) => {
            match base64::engine::general_purpose::STANDARD.decode(&b64) {
                Ok(bytes) => match std::fs::write(output, &bytes) {
                    Ok(()) => println!(
                        "{} Screenshot saved to {} ({} bytes)",
                        "✓".green(),
                        output.cyan(),
                        bytes.len()
                    ),
                    Err(e) => eprintln!("{} Write failed: {e}", "✗".red()),
                },
                Err(e) => eprintln!("{} Base64 decode failed: {e}", "✗".red()),
            }
        }
        Err(e) => eprintln!("{} {e}", "✗".red()),
    }
}

/// Handle `onecrawl android orientation`.
pub async fn orientation(set: Option<&str>) {
    let config = AndroidSessionConfig::default();
    let client = AndroidClient::new(config);
    if let Some(orient) = set {
        match client.set_orientation(orient).await {
            Ok(()) => println!(
                "{} Orientation set to {}",
                "✓".green(),
                orient.to_uppercase().cyan()
            ),
            Err(e) => eprintln!("{} {e}", "✗".red()),
        }
    } else {
        match client.get_orientation().await {
            Ok(o) => println!("{} Current orientation: {}", "✓".green(), o.cyan()),
            Err(e) => eprintln!("{} {e}", "✗".red()),
        }
    }
}

/// Handle `onecrawl android key`.
pub async fn key(keycode: i32) {
    let config = AndroidSessionConfig::default();
    let client = AndroidClient::new(config);
    match client.press_key(keycode).await {
        Ok(()) => println!("{} Pressed keycode: {}", "✓".green(), keycode),
        Err(e) => eprintln!("{} {e}", "✗".red()),
    }
}

/// Handle `onecrawl android app-launch`.
pub async fn app_launch(package: &str, activity: Option<&str>) {
    let config = AndroidSessionConfig::default();
    let client = AndroidClient::new(config);
    match client.launch_app(package, activity).await {
        Ok(()) => println!("{} Launched {}", "✓".green(), package.cyan()),
        Err(e) => eprintln!("{} {e}", "✗".red()),
    }
}

/// Handle `onecrawl android app-kill`.
pub async fn app_kill(package: &str) {
    let config = AndroidSessionConfig::default();
    let client = AndroidClient::new(config);
    match client.terminate_app(package).await {
        Ok(()) => println!("{} Terminated {}", "✓".green(), package.cyan()),
        Err(e) => eprintln!("{} {e}", "✗".red()),
    }
}

/// Handle `onecrawl android app-state`.
pub async fn app_state(package: &str) {
    let config = AndroidSessionConfig::default();
    let client = AndroidClient::new(config);
    match client.app_state(package).await {
        Ok(state) => {
            let label = match state {
                1 => "not running",
                2 => "background",
                3 => "background suspended",
                4 => "foreground",
                _ => "unknown",
            };
            println!(
                "{} {} state: {} ({})",
                "✓".green(),
                package.cyan(),
                state,
                label
            );
        }
        Err(e) => eprintln!("{} {e}", "✗".red()),
    }
}

/// Handle `onecrawl android install`.
pub async fn install(apk_path: &str) {
    let config = AndroidSessionConfig::default();
    let client = AndroidClient::new(config);
    match client.install_app(apk_path).await {
        Ok(()) => println!("{} Installed {}", "✓".green(), apk_path.cyan()),
        Err(e) => eprintln!("{} {e}", "✗".red()),
    }
}

/// Handle `onecrawl android script`.
pub async fn script(code: &str) {
    let config = AndroidSessionConfig::default();
    let client = AndroidClient::new(config);
    match client.execute_script(code, &[]).await {
        Ok(result) => println!(
            "{} Script result:\n{}",
            "✓".green(),
            serde_json::to_string_pretty(&result).unwrap_or_default()
        ),
        Err(e) => eprintln!("{} {e}", "✗".red()),
    }
}

/// Handle `onecrawl android shell`.
pub async fn shell(serial: &str, command: &str) {
    match AndroidClient::shell(serial, command).await {
        Ok(output) => println!("{} Shell output:\n{}", "✓".green(), output),
        Err(e) => eprintln!("{} {e}", "✗".red()),
    }
}

/// Handle `onecrawl android push`.
pub async fn push(serial: &str, local: &str, remote: &str) {
    match AndroidClient::push_file(serial, local, remote).await {
        Ok(()) => println!(
            "{} Pushed {} → {}",
            "✓".green(),
            local.cyan(),
            remote.cyan()
        ),
        Err(e) => eprintln!("{} {e}", "✗".red()),
    }
}

/// Handle `onecrawl android pull`.
pub async fn pull(serial: &str, remote: &str, local: &str) {
    match AndroidClient::pull_file(serial, remote, local).await {
        Ok(()) => println!(
            "{} Pulled {} → {}",
            "✓".green(),
            remote.cyan(),
            local.cyan()
        ),
        Err(e) => eprintln!("{} {e}", "✗".red()),
    }
}

/// Handle `onecrawl android info`.
pub async fn info(serial: &str) {
    match AndroidClient::device_info(serial).await {
        Ok(info) => println!(
            "{} Device info:\n{}",
            "✓".green(),
            serde_json::to_string_pretty(&info).unwrap_or_default()
        ),
        Err(e) => eprintln!("{} {e}", "✗".red()),
    }
}

/// Handle `onecrawl android battery`.
pub async fn battery(serial: &str) {
    match AndroidClient::battery_info(serial).await {
        Ok(info) => println!(
            "{} Battery info:\n{}",
            "✓".green(),
            serde_json::to_string_pretty(&info).unwrap_or_default()
        ),
        Err(e) => eprintln!("{} {e}", "✗".red()),
    }
}

/// Handle `onecrawl android disconnect`.
pub async fn disconnect() {
    let config = AndroidSessionConfig::default();
    let mut client = AndroidClient::new(config);
    match client.close_session().await {
        Ok(()) => println!("{} Android session closed", "✓".green()),
        Err(e) => eprintln!("{} {e}", "✗".red()),
    }
}

/// Handle `onecrawl android url`.
pub async fn url() {
    let config = AndroidSessionConfig::default();
    let client = AndroidClient::new(config);
    match client.get_url().await {
        Ok(u) => println!("{} URL: {}", "✓".green(), u.cyan()),
        Err(e) => eprintln!("{} {e}", "✗".red()),
    }
}

/// Handle `onecrawl android title`.
pub async fn title() {
    let config = AndroidSessionConfig::default();
    let client = AndroidClient::new(config);
    match client.get_title().await {
        Ok(t) => println!("{} Title: {}", "✓".green(), t.cyan()),
        Err(e) => eprintln!("{} {e}", "✗".red()),
    }
}
