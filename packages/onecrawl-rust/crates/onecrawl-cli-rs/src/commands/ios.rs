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
