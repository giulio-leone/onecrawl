use colored::Colorize;
use super::super::helpers::{with_page};

// ---------------------------------------------------------------------------
// Keyboard
// ---------------------------------------------------------------------------

pub async fn set_offline(state: &str) {
    with_page(|page| async move {
        let offline = state == "on";
        let js = format!(
            "navigator.onLine !== {} && void 0",
            if offline { "false" } else { "true" }
        );
        page.evaluate(js).await.map_err(|e| e.to_string())?;
        println!("{} Offline mode: {}", "✓".green(), if offline { "ON" } else { "OFF" });
        Ok(())
    })
    .await;
}

pub async fn set_extra_headers(json: &str) {
    let json = json.to_string();
    with_page(|page| async move {
        let headers: serde_json::Value = serde_json::from_str(&json)
            .map_err(|e| format!("Invalid JSON: {e}"))?;
        let js = format!(
            r#"(() => {{
                const h = {};
                const orig = window.fetch;
                window.fetch = (url, opts = {{}}) => {{
                    opts.headers = {{ ...h, ...(opts.headers || {{}}) }};
                    return orig(url, opts);
                }};
                return Object.keys(h).length;
            }})()"#,
            headers
        );
        let v = page.evaluate(js).await.map_err(|e| e.to_string())?;
        println!("{} Set {} extra headers", "✓".green(),
            v.into_value::<serde_json::Value>().unwrap_or_default());
        Ok(())
    })
    .await;
}

pub async fn set_credentials(username: &str, password: &str) {
    let username = username.to_string();
    let password = password.to_string();
    with_page(|page| async move {
        let js = format!(
            r#"(() => {{
                const cred = btoa({u} + ':' + {p});
                const orig = window.fetch;
                window.fetch = (url, opts = {{}}) => {{
                    opts.headers = {{ 'Authorization': 'Basic ' + cred, ...(opts.headers || {{}}) }};
                    return orig(url, opts);
                }};
                return true;
            }})()"#,
            u = serde_json::to_string(&username).unwrap_or_default(),
            p = serde_json::to_string(&password).unwrap_or_default()
        );
        page.evaluate(js).await.map_err(|e| e.to_string())?;
        println!("{} HTTP basic auth set for {}", "✓".green(), username);
        Ok(())
    })
    .await;
}

