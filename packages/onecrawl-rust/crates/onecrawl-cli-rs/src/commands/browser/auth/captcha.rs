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

pub async fn captcha_detect() {
    with_page(|page| async move {
        let detection = onecrawl_cdp::captcha::detect_captcha(&page)
            .await
            .map_err(|e| e.to_string())?;
        println!(
            "{}",
            serde_json::to_string_pretty(&detection).unwrap_or_default()
        );
        Ok(())
    })
    .await;
}

pub async fn captcha_wait(timeout: u64) {
    with_page(|page| async move {
        let detection = onecrawl_cdp::captcha::wait_for_captcha(&page, timeout)
            .await
            .map_err(|e| e.to_string())?;
        println!(
            "{}",
            serde_json::to_string_pretty(&detection).unwrap_or_default()
        );
        Ok(())
    })
    .await;
}

pub async fn captcha_screenshot() {
    with_page(|page| async move {
        let detection = onecrawl_cdp::captcha::detect_captcha(&page)
            .await
            .map_err(|e| e.to_string())?;
        if !detection.detected {
            println!("{} No CAPTCHA detected on current page", "⚠".yellow());
            return Ok(());
        }
        let data = onecrawl_cdp::captcha::screenshot_captcha(&page, &detection)
            .await
            .map_err(|e| e.to_string())?;
        println!("{data}");
        Ok(())
    })
    .await;
}

pub async fn captcha_inject(solution: &str) {
    let sol = solution.to_string();
    with_page(|page| async move {
        let detection = onecrawl_cdp::captcha::detect_captcha(&page)
            .await
            .map_err(|e| e.to_string())?;
        if !detection.detected {
            eprintln!("{} No CAPTCHA detected on current page", "✗".red());
            std::process::exit(1);
        }
        let ok = onecrawl_cdp::captcha::inject_solution(&page, &detection, &sol)
            .await
            .map_err(|e| e.to_string())?;
        if ok {
            println!(
                "{} Solution injected for {}",
                "✓".green(),
                detection.captcha_type.cyan()
            );
        } else {
            eprintln!(
                "{} Injection failed for {}",
                "✗".red(),
                detection.captcha_type
            );
            std::process::exit(1);
        }
        Ok(())
    })
    .await;
}

pub fn captcha_types() {
    let types = onecrawl_cdp::captcha::supported_types();
    for (name, desc) in &types {
        println!("  {}: {}", name.cyan(), desc);
    }
}

pub async fn captcha_solve(timeout: u64) {
    with_page(|page| async move {
        // First detect what captcha is present
        let det = onecrawl_cdp::captcha::detect_captcha(&page)
            .await
            .map_err(|e| e.to_string())?;

        if !det.detected {
            println!("{} No CAPTCHA detected on this page", "⚠".yellow());
            return Ok(());
        }

        println!(
            "{} Detected: {} ({}) — attempting browser-native solve...",
            "🔍".to_string(),
            det.captcha_type.cyan(),
            det.provider.dimmed()
        );

        match det.captcha_type.as_str() {
            "cloudflare_turnstile" => {
                let solved = onecrawl_cdp::captcha::solve_turnstile_native(&page, timeout)
                    .await
                    .map_err(|e| e.to_string())?;
                if solved {
                    println!("{} Turnstile solved (browser-native, free)", "✓".green());
                } else {
                    eprintln!("{} Turnstile did not clear within {}ms", "✗".red(), timeout);
                }
            }
            "recaptcha_v2" => {
                match onecrawl_cdp::captcha::solve_recaptcha_audio(&page).await {
                    Ok(text) => {
                        println!(
                            "{} reCAPTCHA solved via audio+Whisper: \"{}\"",
                            "✓".green(),
                            text.dimmed()
                        );
                    }
                    Err(e) => {
                        eprintln!("{} reCAPTCHA audio solve failed: {}", "✗".red(), e);
                        eprintln!("  Ensure `whisper` CLI is installed: pip install openai-whisper");
                    }
                }
            }
            "recaptcha_v3" => {
                println!(
                    "{} reCAPTCHA v3 is score-based — stealth mode should provide high score",
                    "ℹ".cyan()
                );
                println!("  No explicit solving needed. If blocked, check stealth with: onecrawl captcha check");
            }
            other => {
                println!(
                    "{} No free solver available for {} — use 'captcha inject <token>' with manual/API token",
                    "⚠".yellow(),
                    other
                );
            }
        }
        Ok(())
    })
    .await;
}

