use colored::Colorize;
use onecrawl_cdp::ios::{IosClient, IosSessionConfig};

/// Handle `onecrawl ios devices`.
pub fn devices() {
    match IosClient::list_devices() {
        Ok(devs) => {
            if devs.is_empty() {
                println!("{}", "No available iOS simulators found.".yellow());
            } else {
                println!("{} Found {} device(s):\n", "✓".green(), devs.len());
                for d in &devs {
                    println!(
                        "  {} — {} ({})",
                        d.name.cyan(),
                        d.udid.dimmed(),
                        d.version
                    );
                }
            }
            println!(
                "\n{}",
                serde_json::to_string_pretty(&devs).unwrap_or_default()
            );
        }
        Err(e) => eprintln!("{} {e}", "✗".red()),
    }
}

/// Handle `onecrawl ios connect`.
pub async fn connect(wda_url: &str, udid: Option<&str>) {
    let config = IosSessionConfig {
        wda_url: wda_url.to_string(),
        device_udid: udid.map(|s| s.to_string()),
        ..Default::default()
    };
    let mut client = IosClient::new(config);
    match client.create_session().await {
        Ok(sid) => println!("{} iOS session created: {}", "✓".green(), sid.cyan()),
        Err(e) => eprintln!("{} {e}", "✗".red()),
    }
}

/// Handle `onecrawl ios navigate`.
pub async fn navigate(url: &str) {
    let config = IosSessionConfig::default();
    let client = IosClient::new(config);
    match client.navigate(url).await {
        Ok(()) => println!("{} Navigated to {}", "✓".green(), url.cyan()),
        Err(e) => eprintln!("{} {e}", "✗".red()),
    }
}

/// Handle `onecrawl ios tap`.
pub async fn tap(x: f64, y: f64) {
    let config = IosSessionConfig::default();
    let client = IosClient::new(config);
    match client.tap(x, y).await {
        Ok(()) => println!("{} Tapped at ({}, {})", "✓".green(), x, y),
        Err(e) => eprintln!("{} {e}", "✗".red()),
    }
}

/// Handle `onecrawl ios screenshot`.
pub async fn screenshot(output: &str) {
    let config = IosSessionConfig::default();
    let client = IosClient::new(config);
    match client.screenshot().await {
        Ok(bytes) => match std::fs::write(output, &bytes) {
            Ok(()) => println!(
                "{} Screenshot saved to {} ({} bytes)",
                "✓".green(),
                output.cyan(),
                bytes.len()
            ),
            Err(e) => eprintln!("{} Write failed: {e}", "✗".red()),
        },
        Err(e) => eprintln!("{} {e}", "✗".red()),
    }
}

/// Handle `onecrawl ios disconnect`.
pub async fn disconnect() {
    let config = IosSessionConfig::default();
    let mut client = IosClient::new(config);
    match client.close_session().await {
        Ok(()) => println!("{} iOS session closed", "✓".green()),
        Err(e) => eprintln!("{} {e}", "✗".red()),
    }
}

/// Handle `onecrawl ios pinch`.
pub async fn pinch(x: f64, y: f64, scale: f64, velocity: f64) {
    let config = IosSessionConfig::default();
    let client = IosClient::new(config);
    match client.pinch(x, y, scale, velocity).await {
        Ok(()) => println!("{} Pinched at ({}, {}) scale={}", "✓".green(), x, y, scale),
        Err(e) => eprintln!("{} {e}", "✗".red()),
    }
}

/// Handle `onecrawl ios long-press`.
pub async fn long_press(x: f64, y: f64, duration: u64) {
    let config = IosSessionConfig::default();
    let client = IosClient::new(config);
    match client.long_press(x, y, duration).await {
        Ok(()) => println!("{} Long pressed at ({}, {}) for {}ms", "✓".green(), x, y, duration),
        Err(e) => eprintln!("{} {e}", "✗".red()),
    }
}

/// Handle `onecrawl ios double-tap`.
pub async fn double_tap(x: f64, y: f64) {
    let config = IosSessionConfig::default();
    let client = IosClient::new(config);
    match client.double_tap(x, y).await {
        Ok(()) => println!("{} Double tapped at ({}, {})", "✓".green(), x, y),
        Err(e) => eprintln!("{} {e}", "✗".red()),
    }
}

/// Handle `onecrawl ios orientation`.
pub async fn orientation(set: Option<&str>) {
    let config = IosSessionConfig::default();
    let client = IosClient::new(config);
    if let Some(orient) = set {
        match client.set_orientation(orient).await {
            Ok(()) => println!("{} Orientation set to {}", "✓".green(), orient.to_uppercase().cyan()),
            Err(e) => eprintln!("{} {e}", "✗".red()),
        }
    } else {
        match client.get_orientation().await {
            Ok(o) => println!("{} Current orientation: {}", "✓".green(), o.cyan()),
            Err(e) => eprintln!("{} {e}", "✗".red()),
        }
    }
}

/// Handle `onecrawl ios app-launch`.
pub async fn app_launch(bundle_id: &str) {
    let config = IosSessionConfig::default();
    let client = IosClient::new(config);
    match client.launch_app(bundle_id).await {
        Ok(()) => println!("{} Launched {}", "✓".green(), bundle_id.cyan()),
        Err(e) => eprintln!("{} {e}", "✗".red()),
    }
}

/// Handle `onecrawl ios app-kill`.
pub async fn app_kill(bundle_id: &str) {
    let config = IosSessionConfig::default();
    let client = IosClient::new(config);
    match client.terminate_app(bundle_id).await {
        Ok(()) => println!("{} Terminated {}", "✓".green(), bundle_id.cyan()),
        Err(e) => eprintln!("{} {e}", "✗".red()),
    }
}

/// Handle `onecrawl ios app-state`.
pub async fn app_state(bundle_id: &str) {
    let config = IosSessionConfig::default();
    let client = IosClient::new(config);
    match client.app_state(bundle_id).await {
        Ok(state) => {
            let label = match state {
                1 => "not running",
                2 => "background suspended",
                3 => "background",
                4 => "foreground",
                _ => "unknown",
            };
            println!("{} {} state: {} ({})", "✓".green(), bundle_id.cyan(), state, label);
        }
        Err(e) => eprintln!("{} {e}", "✗".red()),
    }
}

/// Handle `onecrawl ios lock`.
pub async fn lock() {
    let config = IosSessionConfig::default();
    let client = IosClient::new(config);
    match client.lock_device().await {
        Ok(()) => println!("{} Device locked", "✓".green()),
        Err(e) => eprintln!("{} {e}", "✗".red()),
    }
}

/// Handle `onecrawl ios unlock`.
pub async fn unlock() {
    let config = IosSessionConfig::default();
    let client = IosClient::new(config);
    match client.unlock_device().await {
        Ok(()) => println!("{} Device unlocked", "✓".green()),
        Err(e) => eprintln!("{} {e}", "✗".red()),
    }
}

/// Handle `onecrawl ios home`.
pub async fn home() {
    let config = IosSessionConfig::default();
    let client = IosClient::new(config);
    match client.home_button().await {
        Ok(()) => println!("{} Home button pressed", "✓".green()),
        Err(e) => eprintln!("{} {e}", "✗".red()),
    }
}

/// Handle `onecrawl ios button`.
pub async fn button(name: &str) {
    let config = IosSessionConfig::default();
    let client = IosClient::new(config);
    match client.press_button(name).await {
        Ok(()) => println!("{} Pressed button: {}", "✓".green(), name.cyan()),
        Err(e) => eprintln!("{} {e}", "✗".red()),
    }
}

/// Handle `onecrawl ios battery`.
pub async fn battery() {
    let config = IosSessionConfig::default();
    let client = IosClient::new(config);
    match client.battery_info().await {
        Ok(info) => println!(
            "{} Battery info:\n{}",
            "✓".green(),
            serde_json::to_string_pretty(&info).unwrap_or_default()
        ),
        Err(e) => eprintln!("{} {e}", "✗".red()),
    }
}

/// Handle `onecrawl ios info`.
pub async fn info() {
    let config = IosSessionConfig::default();
    let client = IosClient::new(config);
    match client.device_info().await {
        Ok(info) => println!(
            "{} Device info:\n{}",
            "✓".green(),
            serde_json::to_string_pretty(&info).unwrap_or_default()
        ),
        Err(e) => eprintln!("{} {e}", "✗".red()),
    }
}

/// Handle `onecrawl ios simulator`.
pub async fn simulator(
    action: &str,
    udid: Option<&str>,
    device_type: Option<&str>,
    runtime: Option<&str>,
) {
    match IosClient::simulator_action(action, udid, device_type, runtime).await {
        Ok(result) => println!(
            "{} Simulator {}:\n{}",
            "✓".green(),
            action.cyan(),
            serde_json::to_string_pretty(&result).unwrap_or_default()
        ),
        Err(e) => eprintln!("{} {e}", "✗".red()),
    }
}

/// Handle `onecrawl ios url`.
pub async fn url() {
    let config = IosSessionConfig::default();
    let client = IosClient::new(config);
    match client.get_url().await {
        Ok(u) => println!("{} URL: {}", "✓".green(), u.cyan()),
        Err(e) => eprintln!("{} {e}", "✗".red()),
    }
}

/// Handle `onecrawl ios title`.
pub async fn title() {
    let config = IosSessionConfig::default();
    let client = IosClient::new(config);
    match client.get_title().await {
        Ok(t) => println!("{} Title: {}", "✓".green(), t.cyan()),
        Err(e) => eprintln!("{} {e}", "✗".red()),
    }
}

/// Handle `onecrawl ios script`.
pub async fn script(code: &str) {
    let config = IosSessionConfig::default();
    let client = IosClient::new(config);
    match client.execute_script(code, &[]).await {
        Ok(result) => println!(
            "{} Script result:\n{}",
            "✓".green(),
            serde_json::to_string_pretty(&result).unwrap_or_default()
        ),
        Err(e) => eprintln!("{} {e}", "✗".red()),
    }
}

/// Handle `onecrawl ios cookies`.
pub async fn cookies() {
    let config = IosSessionConfig::default();
    let client = IosClient::new(config);
    match client.get_cookies().await {
        Ok(cookies) => println!(
            "{} Cookies:\n{}",
            "✓".green(),
            serde_json::to_string_pretty(&cookies).unwrap_or_default()
        ),
        Err(e) => eprintln!("{} {e}", "✗".red()),
    }
}
