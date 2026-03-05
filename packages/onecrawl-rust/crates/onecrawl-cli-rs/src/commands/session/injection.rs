use colored::Colorize;
use onecrawl_cdp::BrowserSession;

use super::core::{load_session, save_session};

/// Connect to a running browser session and import cookies from a JSON file.
/// The file must be in the CookieJar format produced by `cookie export`.

/// Inject the stealth init script persistently via `Page.addScriptToEvaluateOnNewDocument`.
/// This runs before any page's own scripts on every navigation, ensuring:
///   - navigator.webdriver = undefined
///   - navigator.plugins populated
///   - User-Agent, languages, platform match the fingerprint
///   - WebGL vendor/renderer spoofed
///   - chrome.runtime present (so x.com sees a "normal" Chrome extension API)

pub(crate) async fn apply_cookie_import(ws_url: &str, cookie_file: &str) -> Result<(), String> {
    println!("{} Importing cookies from {}...", "→".blue(), cookie_file);
    let session = BrowserSession::connect(ws_url)
        .await
        .map_err(|e| format!("connect for cookie import: {e}"))?;

    let page = session
        .new_page("about:blank")
        .await
        .map_err(|e| format!("open blank page for cookie import: {e}"))?;

    let count = onecrawl_cdp::cookie_jar::load_cookies_from_file(&page, std::path::Path::new(cookie_file))
        .await
        .map_err(|e| format!("load cookies: {e}"))?;

    println!("{} Imported {} cookies", "✓".green(), count);
    Ok(())
}

pub(crate) async fn apply_stealth_persistent(ws_url: &str) -> Result<(), String> {
    let session = BrowserSession::connect(ws_url)
        .await
        .map_err(|e| format!("connect for stealth inject: {e}"))?;

    let page = session
        .new_page("about:blank")
        .await
        .map_err(|e| format!("open blank page for stealth: {e}"))?;

    // Persist this tab's TargetId so connect_to_session() always returns this
    // specific tab — the one where stealth scripts are registered.
    let tab_id = page.target_id().inner().clone();
    let real_ua = session.browser().user_agent().await.ok();
    if let Some(mut info) = load_session() {
        info.active_tab_id = Some(tab_id);
        if real_ua.is_some() {
            info.fingerprint_ua = real_ua.clone();
        }
        let _ = save_session(&info);
    }

    onecrawl_cdp::inject_persistent_stealth(&page, real_ua.as_deref())
        .await
        .map_err(|e| format!("stealth inject: {e}"))?;

    println!("{} Persistent stealth patches registered for all pages", "✓".green());
    Ok(())
}
