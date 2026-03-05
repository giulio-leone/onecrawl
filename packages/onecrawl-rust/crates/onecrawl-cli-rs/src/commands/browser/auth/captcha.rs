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

async fn get_page_url(page: &onecrawl_cdp::Page) -> String {
    page.evaluate("window.location.href")
        .await
        .ok()
        .and_then(|v| v.into_value().ok())
        .unwrap_or_default()
}

async fn try_api_fallback(
    page: &onecrawl_cdp::Page,
    det: &onecrawl_cdp::captcha::CaptchaDetection,
    context: &str,
) -> std::result::Result<bool, String> {
    let config = match onecrawl_cdp::captcha::load_solver_config() {
        Some(c) => c,
        None => return Ok(false),
    };
    let sitekey = match det.sitekey.as_deref() {
        Some(k) => k,
        None => {
            eprintln!("{} {context} (no sitekey for API fallback)", "✗".red());
            return Ok(false);
        }
    };
    let page_url = get_page_url(page).await;
    println!("  Trying {} API...", config.service.to_string().cyan());
    match onecrawl_cdp::captcha::solve_via_api(&det.captcha_type, sitekey, &page_url, &config).await {
        Ok(token) => {
            let _ = onecrawl_cdp::captcha::inject_solution(page, det, &token).await;
            println!("{} {} solved via {} API", "✓".green(), det.captcha_type.cyan(), config.service);
            Ok(true)
        }
        Err(e) => {
            eprintln!("{} API solve for {} failed: {}", "✗".red(), det.captcha_type, e);
            Ok(false)
        }
    }
}

pub async fn captcha_solve(timeout: u64, use_api: bool) {
    with_page(|page| async move {
        let det = onecrawl_cdp::captcha::detect_captcha(&page)
            .await
            .map_err(|e| e.to_string())?;

        if !det.detected {
            println!("{} No CAPTCHA detected on this page", "⚠".yellow());
            return Ok(());
        }

        println!(
            "🔍 Detected: {} ({}) {}",
            det.captcha_type.cyan(),
            det.provider.dimmed(),
            if use_api { "— using API solver" } else { "— trying browser-native first" }
        );

        if use_api {
            let config = onecrawl_cdp::captcha::load_solver_config();
            if config.is_none() {
                eprintln!("{} No solver API key configured", "✗".red());
                eprintln!("  Create ~/.onecrawl/config.json with one of:");
                eprintln!("    {{\"capsolver_key\": \"CAP-xxx\"}}");
                eprintln!("    {{\"twocaptcha_key\": \"abc123\"}}");
                eprintln!("    {{\"anticaptcha_key\": \"xyz789\"}}");
                std::process::exit(1);
            }
            let config = config.unwrap();
            let sitekey = det.sitekey.as_deref().unwrap_or("");
            if sitekey.is_empty() {
                eprintln!("{} No sitekey found — cannot use API solver", "✗".red());
                std::process::exit(1);
            }
            let page_url = get_page_url(&page).await;
            println!("  Sending to {} (sitekey: {}...)", config.service.to_string().cyan(), &sitekey[..sitekey.len().min(12)]);
            match onecrawl_cdp::captcha::solve_via_api(&det.captcha_type, sitekey, &page_url, &config).await {
                Ok(token) => {
                    let injected = onecrawl_cdp::captcha::inject_solution(&page, &det, &token)
                        .await.map_err(|e| e.to_string())?;
                    if injected {
                        println!("{} {} solved via {} — token injected", "✓".green(), det.captcha_type.cyan(), config.service.to_string().cyan());
                    } else {
                        println!("{} Token received but injection failed — token: {}...", "⚠".yellow(), &token[..token.len().min(40)]);
                    }
                }
                Err(e) => {
                    eprintln!("{} API solve failed: {}", "✗".red(), e);
                    std::process::exit(1);
                }
            }
            return Ok(());
        }

        match det.captcha_type.as_str() {
            "cloudflare_turnstile" => {
                let solved = onecrawl_cdp::captcha::solve_turnstile_native(&page, timeout)
                    .await.map_err(|e| e.to_string())?;
                if solved {
                    println!("{} Turnstile solved (browser-native, free)", "✓".green());
                } else {
                    let msg = format!("Turnstile did not clear within {timeout}ms");
                    if !try_api_fallback(&page, &det, &msg).await.unwrap_or(false) {
                        eprintln!("{} {msg}", "✗".red());
                    }
                }
            }
            "recaptcha_v2" => {
                match onecrawl_cdp::captcha::solve_recaptcha_audio(&page).await {
                    Ok(text) => println!("{} reCAPTCHA solved via audio+Whisper: \"{}\"", "✓".green(), text.dimmed()),
                    Err(e) => {
                        eprintln!("{} reCAPTCHA audio solve failed: {}", "✗".red(), e);
                        eprintln!("  Ensure `whisper` CLI is installed: pip install openai-whisper");
                        eprintln!("  Or use --api flag with a configured solver key");
                    }
                }
            }
            "recaptcha_v3" => {
                println!("{} reCAPTCHA v3 is score-based — stealth mode should provide high score", "ℹ".cyan());
                println!("  No explicit solving needed. If blocked, check stealth with: onecrawl captcha check");
            }
            other => {
                let msg = format!("No free solver for {other}");
                if !try_api_fallback(&page, &det, &msg).await.unwrap_or(false) {
                    println!("{} Use 'captcha inject <token>' with manual/API token", "⚠".yellow());
                }
            }
        }
        Ok(())
    })
    .await;
}

