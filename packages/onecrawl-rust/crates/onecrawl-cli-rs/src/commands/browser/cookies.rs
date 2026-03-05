use colored::Colorize;
use super::helpers::{with_page};

// Delegates to `cookie_jar::save_cookies_to_file` (or prints JSON to stdout).
// Import cookies from a CookieJar JSON file into the current page.
// Delegates to `cookie_jar::load_cookies_from_file`.
// ---------------------------------------------------------------------------
// Cookie Jar
// ---------------------------------------------------------------------------

pub async fn cookie_get(name: Option<&str>, json: bool) {
    let name = name.map(String::from);
    with_page(|page| async move {
        let cookies = onecrawl_cdp::cookie::get_all_cookies(&page)
            .await
            .map_err(|e| e.to_string())?;
        let filtered: Vec<_> = if let Some(ref n) = name {
            cookies.into_iter().filter(|c| c.name == *n).collect()
        } else {
            cookies
        };
        if json {
            println!(
                "{}",
                serde_json::to_string_pretty(&filtered).unwrap_or_default()
            );
        } else {
            for c in &filtered {
                println!(
                    "{}={} (domain={}, path={}, secure={}, httpOnly={})",
                    c.name.green(),
                    c.value,
                    c.domain,
                    c.path,
                    c.secure,
                    c.http_only
                );
            }
            if filtered.is_empty() {
                println!("{}", "No cookies found".dimmed());
            }
        }
        Ok(())
    })
    .await;
}

pub async fn cookie_set(name: &str, value: &str, domain: Option<&str>, path: Option<&str>) {
    let params = onecrawl_cdp::SetCookieParams {
        name: name.to_string(),
        value: value.to_string(),
        domain: domain.map(String::from),
        path: path.map(String::from),
        expires: None,
        http_only: None,
        secure: None,
        same_site: None,
        url: None,
    };
    with_page(|page| async move {
        onecrawl_cdp::cookie::set_cookie(&page, &params)
            .await
            .map_err(|e| e.to_string())?;
        println!("{} Cookie '{}' set", "✓".green(), params.name);
        Ok(())
    })
    .await;
}

pub async fn cookie_delete(name: &str, domain: &str) {
    let n = name.to_string();
    let d = domain.to_string();
    with_page(|page| async move {
        onecrawl_cdp::cookie::delete_cookies(&page, &n, Some(&d), None)
            .await
            .map_err(|e| e.to_string())?;
        println!("{} Cookie '{}' deleted", "✓".green(), n);
        Ok(())
    })
    .await;
}

pub async fn cookie_clear() {
    with_page(|page| async move {
        onecrawl_cdp::cookie::clear_cookies(&page)
            .await
            .map_err(|e| e.to_string())?;
        println!("{} All cookies cleared", "✓".green());
        Ok(())
    })
    .await;
}

pub async fn cookie_export(output: Option<&str>) {
    cookie_jar_export(output).await;
}

pub async fn cookie_import(path: &str) {
    cookie_jar_import(path).await;
}

pub async fn cookie_jar_export(output: Option<&str>) {
    let output = output.map(String::from);
    with_page(|page| async move {
        if let Some(path) = output {
            let count =
                onecrawl_cdp::cookie_jar::save_cookies_to_file(&page, std::path::Path::new(&path))
                    .await
                    .map_err(|e| e.to_string())?;
            println!("{} Exported {} cookies to {}", "✓".green(), count, path);
        } else {
            let jar = onecrawl_cdp::cookie_jar::export_cookies(&page)
                .await
                .map_err(|e| e.to_string())?;
            println!("{}", serde_json::to_string_pretty(&jar).unwrap_or_default());
        }
        Ok(())
    })
    .await;
}

pub async fn cookie_jar_import(path: &str) {
    let path = path.to_string();
    with_page(|page| async move {
        let count =
            onecrawl_cdp::cookie_jar::load_cookies_from_file(&page, std::path::Path::new(&path))
                .await
                .map_err(|e| e.to_string())?;
        println!("{} Imported {} cookies from {}", "✓".green(), count, path);
        Ok(())
    })
    .await;
}

pub async fn cookie_jar_clear() {
    with_page(|page| async move {
        onecrawl_cdp::cookie_jar::clear_all_cookies(&page)
            .await
            .map_err(|e| e.to_string())?;
        println!("{} All cookies cleared (cookie jar)", "✓".green());
        Ok(())
    })
    .await;
}
