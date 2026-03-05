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
pub async fn antibot_inject(level: &str) {
    let lvl = level.to_string();
    with_page(|page| async move {
        let applied = onecrawl_cdp::antibot::inject_stealth_full(&page)
            .await
            .map_err(|e| e.to_string())?;
        // Filter by profile level
        let profiles = onecrawl_cdp::antibot::stealth_profiles();
        let profile = profiles.iter().find(|p| p.level == lvl);
        let names: Vec<&str> = if let Some(p) = profile {
            applied
                .iter()
                .filter(|a| p.patches.contains(a))
                .map(|s| s.as_str())
                .collect()
        } else {
            applied.iter().map(|s| s.as_str()).collect()
        };
        println!(
            "{} Anti-bot patches injected (level: {})",
            "✓".green(),
            lvl.cyan()
        );
        for n in &names {
            println!("  • {}", n);
        }
        Ok(())
    })
    .await;
}

pub async fn antibot_test() {
    with_page(|page| async move {
        let result = onecrawl_cdp::antibot::bot_detection_test(&page)
            .await
            .map_err(|e| e.to_string())?;
        println!(
            "{}",
            serde_json::to_string_pretty(&result).unwrap_or_default()
        );
        Ok(())
    })
    .await;
}

pub async fn antibot_profiles() {
    let profiles = onecrawl_cdp::antibot::stealth_profiles();
    println!(
        "{}",
        serde_json::to_string_pretty(&profiles).unwrap_or_default()
    );
}

