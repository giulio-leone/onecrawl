use colored::Colorize;
use super::super::helpers::{with_page};

// ---------------------------------------------------------------------------
// Passkey / WebAuthn
// ---------------------------------------------------------------------------

/// Enable a CDP real virtual authenticator, wait for a passkey to be registered
/// on the current page (e.g. x.com Settings → Security → Passkey), then export
/// the credential (including private key) to a JSON file.
///
/// The credential exported here can later be injected via
/// `session start --import-passkey FILE` for fully automated headless passkey auth.

/// Store the passkey file path in the active session so that CDP WebAuthn is
/// automatically re-enabled and credentials are injected on every
/// `connect_to_session()` call (same lifecycle as stealth scripts).

// ---------------------------------------------------------------------------
// Stealth
// ---------------------------------------------------------------------------

// ---------------------------------------------------------------------------
// Anti-Bot
// ---------------------------------------------------------------------------

// ---------------------------------------------------------------------------
// Captcha
// ---------------------------------------------------------------------------

// ---------------------------------------------------------------------------
// Passkey Vault (multi-site persistent store)
// ---------------------------------------------------------------------------

/// List all sites and credential counts in the passkey vault.

/// Add credentials from a native passkey JSON file to the vault.

/// Remove a specific credential from the vault by its credential_id.

/// Remove all credentials for a specific rp_id from the vault.

/// Export vault credentials for a site to a passkey JSON file.

/// Import passkeys from a Bitwarden unencrypted JSON export.

/// Import passkeys from a 1Password export.data JSON file (extracted from .1pux).

/// Import passkeys from a FIDO Alliance CXF JSON file.

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

