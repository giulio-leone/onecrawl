use colored::Colorize;
use super::helpers::{with_page};

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

pub async fn passkey_enable(protocol: &str, transport: &str) {
    let proto = protocol.to_string();
    let trans = transport.to_string();
    with_page(|page| async move {
        let config = onecrawl_cdp::webauthn::VirtualAuthenticator {
            id: format!(
                "auth-{}",
                std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_millis()
            ),
            protocol: proto,
            transport: trans.clone(),
            has_resident_key: true,
            has_user_verification: true,
            is_user_verified: true,
        };
        onecrawl_cdp::webauthn::enable_virtual_authenticator(&page, &config)
            .await
            .map_err(|e| e.to_string())?;
        println!(
            "{} Virtual authenticator enabled (transport: {})",
            "✓".green(),
            trans
        );
        Ok(())
    })
    .await;
}

pub async fn passkey_add(credential_id: &str, rp_id: &str, user_handle: Option<&str>) {
    let cred = onecrawl_cdp::webauthn::VirtualCredential {
        credential_id: credential_id.to_string(),
        rp_id: rp_id.to_string(),
        user_handle: user_handle.unwrap_or_default().to_string(),
        sign_count: 0,
    };
    with_page(|page| async move {
        onecrawl_cdp::webauthn::add_virtual_credential(&page, &cred)
            .await
            .map_err(|e| e.to_string())?;
        println!("{} Credential added: {}", "✓".green(), cred.credential_id);
        Ok(())
    })
    .await;
}

pub async fn passkey_list() {
    with_page(|page| async move {
        let creds = onecrawl_cdp::webauthn::get_virtual_credentials(&page)
            .await
            .map_err(|e| e.to_string())?;
        println!(
            "{}",
            serde_json::to_string_pretty(&creds).unwrap_or_default()
        );
        Ok(())
    })
    .await;
}

pub async fn passkey_log() {
    with_page(|page| async move {
        let log = onecrawl_cdp::webauthn::get_webauthn_log(&page)
            .await
            .map_err(|e| e.to_string())?;
        println!(
            "{}",
            serde_json::to_string_pretty(&log).unwrap_or_default()
        );
        Ok(())
    })
    .await;
}

pub async fn passkey_disable() {
    with_page(|page| async move {
        onecrawl_cdp::webauthn::disable_virtual_authenticator(&page)
            .await
            .map_err(|e| e.to_string())?;
        println!("{} Virtual authenticator disabled", "✓".green());
        Ok(())
    })
    .await;
}

pub async fn passkey_remove(credential_id: &str) {
    let cid = credential_id.to_string();
    with_page(|page| async move {
        let removed = onecrawl_cdp::webauthn::remove_virtual_credential(&page, &cid)
            .await
            .map_err(|e| e.to_string())?;
        if removed {
            println!("{} Credential removed: {cid}", "✓".green());
        } else {
            println!("{} Credential not found: {cid}", "⚠".yellow());
        }
        Ok(())
    })
    .await;
}

pub async fn passkey_register(output: &str, timeout_secs: u64) {
    let output = output.to_string();
    let (_session, page) = match super::super::session::connect_to_session().await {
        Ok(v) => v,
        Err(e) => {
            eprintln!("{} {e}", "✗".red());
            std::process::exit(1);
        }
    };

    // Enable CDP WebAuthn domain
    if let Err(e) = onecrawl_cdp::cdp_enable(&page).await {
        eprintln!("{} WebAuthn.enable failed: {e}", "✗".red());
        std::process::exit(1);
    }

    // Create a CTAP2.1 platform authenticator with auto-presence simulation
    let auth_id = match onecrawl_cdp::cdp_create_authenticator(&page).await {
        Ok(id) => id,
        Err(e) => {
            eprintln!("{} addVirtualAuthenticator failed: {e}", "✗".red());
            std::process::exit(1);
        }
    };

    println!("{} CDP virtual authenticator ready (ID: {})", "✓".green(), auth_id.cyan());
    println!(
        "  {}",
        "Please register a passkey on the current page (e.g. x.com Settings → Security → Passkey)."
            .dimmed()
    );
    println!("  Waiting up to {}s for credential creation…", timeout_secs);

    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(timeout_secs);
    loop {
        tokio::time::sleep(std::time::Duration::from_millis(500)).await;
        match onecrawl_cdp::cdp_get_credentials(&page, &auth_id).await {
            Ok(creds) if !creds.is_empty() => {
                println!("{} {} credential(s) registered", "✓".green(), creds.len());
                let path = std::path::Path::new(&output);
                match onecrawl_cdp::save_passkeys(path, &creds) {
                    Ok(()) => {
                        println!("{} Passkeys saved to {}", "✓".green(), output.cyan());
                        println!(
                            "  {}",
                            "Use `session start --import-passkey FILE` to enable headless passkey auth."
                                .dimmed()
                        );
                    }
                    Err(e) => eprintln!("{} Failed to save passkeys: {e}", "✗".red()),
                }
                return;
            }
            Ok(_) => {} // no credentials yet — keep polling
            Err(e) => {
                eprintln!("{} getCredentials error: {e}", "⚠".yellow());
            }
        }
        if std::time::Instant::now() >= deadline {
            eprintln!("{} Timeout: no passkey registered within {}s", "✗".red(), timeout_secs);
            std::process::exit(1);
        }
    }
}

pub async fn passkey_set_file(file: &str) {
    match super::super::session::load_session() {
        Some(mut info) => {
            info.passkey_file = Some(file.to_string());
            match super::super::session::save_session(&info) {
                Ok(()) => println!(
                    "{} Passkey file set: {} (will be injected on every connect)",
                    "✓".green(),
                    file.cyan()
                ),
                Err(e) => {
                    eprintln!("{} Failed to save session: {e}", "✗".red());
                    std::process::exit(1);
                }
            }
        }
        None => {
            eprintln!("{} No active session. Start one with `session start`.", "✗".red());
            std::process::exit(1);
        }
    }
}

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

pub fn passkey_vault_list() {
    match onecrawl_cdp::load_vault() {
        Ok(vault) => {
            let list = onecrawl_cdp::vault_list(&vault);
            let total = onecrawl_cdp::vault_total(&vault);
            if list.is_empty() {
                println!("{} Passkey vault is empty", "⚠".yellow());
                println!("  Register with: onecrawl auth passkey-register");
                println!("  Import from  : onecrawl passkey import --from bitwarden|1password|cxf --input FILE");
                return;
            }
            println!("{} Passkey vault — {} credential(s) across {} site(s)", "✓".green(), total, list.len());
            println!("{:<35} {}", "rp_id", "credentials");
            println!("{}", "─".repeat(45));
            for (rp_id, count) in &list {
                println!("  {:<33} {}", rp_id, count);
            }
            println!("{}", "─".repeat(45));
            println!("  Vault path: {}", onecrawl_cdp::vault_path().display());
        }
        Err(e) => eprintln!("{} Vault error: {e}", "✗".red()),
    }
}

pub fn passkey_vault_save(input: &str) {
    let path = std::path::Path::new(input);
    let creds = match onecrawl_cdp::load_passkeys(path) {
        Ok(c) => c,
        Err(e) => { eprintln!("{} Cannot read passkey file: {e}", "✗".red()); return; }
    };
    let mut vault = match onecrawl_cdp::load_vault() {
        Ok(v) => v,
        Err(e) => { eprintln!("{} Cannot load vault: {e}", "✗".red()); return; }
    };
    let count = creds.len();
    onecrawl_cdp::vault_add(&mut vault, creds);
    match onecrawl_cdp::save_vault(&vault) {
        Ok(()) => println!("{} Added {count} credential(s) to vault ({})", "✓".green(), onecrawl_cdp::vault_path().display()),
        Err(e) => eprintln!("{} Vault save failed: {e}", "✗".red()),
    }
}

pub fn passkey_vault_remove(credential_id: &str) {
    let mut vault = match onecrawl_cdp::load_vault() {
        Ok(v) => v,
        Err(e) => { eprintln!("{} Cannot load vault: {e}", "✗".red()); return; }
    };
    if onecrawl_cdp::vault_remove(&mut vault, credential_id) {
        match onecrawl_cdp::save_vault(&vault) {
            Ok(()) => println!("{} Removed credential {}", "✓".green(), credential_id),
            Err(e) => eprintln!("{} Vault save failed: {e}", "✗".red()),
        }
    } else {
        eprintln!("{} Credential not found: {}", "✗".red(), credential_id);
    }
}

pub fn passkey_vault_clear_site(rp_id: &str) {
    let mut vault = match onecrawl_cdp::load_vault() {
        Ok(v) => v,
        Err(e) => { eprintln!("{} Cannot load vault: {e}", "✗".red()); return; }
    };
    let removed = onecrawl_cdp::vault_clear_site(&mut vault, rp_id);
    match onecrawl_cdp::save_vault(&vault) {
        Ok(()) => println!("{} Removed {removed} credential(s) for '{rp_id}'", "✓".green()),
        Err(e) => eprintln!("{} Vault save failed: {e}", "✗".red()),
    }
}

pub fn passkey_vault_export(rp_id: &str, output: &str) {
    let vault = match onecrawl_cdp::load_vault() {
        Ok(v) => v,
        Err(e) => { eprintln!("{} Cannot load vault: {e}", "✗".red()); return; }
    };
    let creds = onecrawl_cdp::vault_get(&vault, rp_id);
    if creds.is_empty() {
        eprintln!("{} No credentials found for '{rp_id}'", "✗".red());
        return;
    }
    match onecrawl_cdp::save_passkeys(std::path::Path::new(output), &creds) {
        Ok(()) => println!("{} Exported {} credential(s) for '{}' → {}", "✓".green(), creds.len(), rp_id, output),
        Err(e) => eprintln!("{} Export failed: {e}", "✗".red()),
    }
}

pub fn passkey_import_bitwarden(input: &str, save_to_vault: bool) {
    let creds = match onecrawl_cdp::import_bitwarden(std::path::Path::new(input)) {
        Ok(c) => c,
        Err(e) => { eprintln!("{} Bitwarden import failed: {e}", "✗".red()); return; }
    };
    if creds.is_empty() {
        println!("{} No importable passkeys found (hardware-bound credentials are skipped)", "⚠".yellow());
        return;
    }
    println!("{} Found {} passkey(s):", "✓".green(), creds.len());
    for c in &creds {
        println!("  • {} @ {}", c.credential_id, c.rp_id);
    }
    if save_to_vault {
        _vault_add_and_save(creds);
    }
}

pub fn passkey_import_1password(input: &str, save_to_vault: bool) {
    let creds = match onecrawl_cdp::import_1password_json(std::path::Path::new(input)) {
        Ok(c) => c,
        Err(e) => { eprintln!("{} 1Password import failed: {e}", "✗".red()); return; }
    };
    if creds.is_empty() {
        println!("{} No importable passkeys found", "⚠".yellow());
        return;
    }
    println!("{} Found {} passkey(s):", "✓".green(), creds.len());
    for c in &creds {
        println!("  • {} @ {}", c.credential_id, c.rp_id);
    }
    if save_to_vault {
        _vault_add_and_save(creds);
    }
}

pub fn passkey_import_cxf(input: &str, save_to_vault: bool) {
    let creds = match onecrawl_cdp::import_cxf(std::path::Path::new(input)) {
        Ok(c) => c,
        Err(e) => { eprintln!("{} FIDO CXF import failed: {e}", "✗".red()); return; }
    };
    if creds.is_empty() {
        println!("{} No importable passkeys found (hardware-bound or unsupported type)", "⚠".yellow());
        return;
    }
    println!("{} Found {} passkey(s):", "✓".green(), creds.len());
    for c in &creds {
        println!("  • {} @ {}", c.credential_id, c.rp_id);
    }
    if save_to_vault {
        _vault_add_and_save(creds);
    }
}

fn _vault_add_and_save(creds: Vec<onecrawl_cdp::PasskeyCredential>) {
    let mut vault = match onecrawl_cdp::load_vault() {
        Ok(v) => v,
        Err(e) => { eprintln!("{} Cannot load vault: {e}", "✗".red()); return; }
    };
    let count = creds.len();
    onecrawl_cdp::vault_add(&mut vault, creds);
    match onecrawl_cdp::save_vault(&vault) {
        Ok(()) => println!("{} Saved {count} credential(s) to vault", "✓".green()),
        Err(e) => eprintln!("{} Vault save failed: {e}", "✗".red()),
    }
}
