use colored::Colorize;
use super::helpers::{with_page, with_session};

// ---------------------------------------------------------------------------
// Download Management
// ---------------------------------------------------------------------------

pub async fn new_page(url: Option<&str>) {
    let url = url.unwrap_or("about:blank").to_string();
    let info = match super::super::session::load_session() {
        Some(i) => i,
        None => {
            eprintln!(
                "{} No active session. Run {} first.",
                "✗".red(),
                "onecrawl session start".yellow()
            );
            std::process::exit(1);
        }
    };
    match onecrawl_cdp::BrowserSession::connect(&info.ws_url).await {
        Ok(session) => match session.new_page(&url).await {
            Ok(_) => println!("{} New page opened: {}", "✓".green(), url.cyan()),
            Err(e) => {
                eprintln!("{} {e}", "✗".red());
                std::process::exit(1);
            }
        },
        Err(e) => {
            eprintln!("{} {e}", "✗".red());
            std::process::exit(1);
        }
    }
}

pub async fn tab_list() {
    with_session(|session, _page| async move {
        let tabs = onecrawl_cdp::tabs::list_tabs(session.browser())
            .await
            .map_err(|e| e.to_string())?;
        println!(
            "{}",
            serde_json::to_string_pretty(&tabs).unwrap_or_default()
        );
        Ok(())
    })
    .await;
}

pub async fn tab_new(url: &str) {
    let url = url.to_string();
    with_session(|session, _page| async move {
        let _page = onecrawl_cdp::tabs::new_tab(session.browser(), &url)
            .await
            .map_err(|e| e.to_string())?;
        println!("{} Opened new tab: {}", "✓".green(), url.cyan());
        Ok(())
    })
    .await;
}

pub async fn tab_close(index: usize) {
    with_session(|session, _page| async move {
        onecrawl_cdp::tabs::close_tab(session.browser(), index)
            .await
            .map_err(|e| e.to_string())?;
        println!("{} Closed tab {}", "✓".green(), index);
        Ok(())
    })
    .await;
}

pub async fn tab_switch(index: usize) {
    with_session(|session, _page| async move {
        let tab = onecrawl_cdp::tabs::get_tab(session.browser(), index)
            .await
            .map_err(|e| e.to_string())?;
        let target_id = tab.target_id().inner().clone();

        // Persist the active tab so every subsequent command uses this tab.
        let mut info = super::super::session::load_session()
            .ok_or_else(|| "No active session".to_string())?;
        info.active_tab_id = Some(target_id);
        super::super::session::save_session(&info)
            .map_err(|e| format!("Failed to save session: {e}"))?;

        println!("{} Switched to tab {}", "✓".green(), index);
        Ok(())
    })
    .await;
}

pub async fn tab_count_cmd() {
    with_session(|session, _page| async move {
        let count = onecrawl_cdp::tabs::tab_count(session.browser())
            .await
            .map_err(|e| e.to_string())?;
        println!("{count}");
        Ok(())
    })
    .await;
}

pub async fn download_set_path(path: &str) {
    let path = path.to_string();
    with_page(|page| async move {
        onecrawl_cdp::downloads::set_download_path(&page, std::path::Path::new(&path))
            .await
            .map_err(|e| e.to_string())?;
        println!("{} Download path set to: {}", "✓".green(), path.cyan());
        Ok(())
    })
    .await;
}

pub async fn download_list() {
    with_page(|page| async move {
        let downloads = onecrawl_cdp::downloads::get_downloads(&page)
            .await
            .map_err(|e| e.to_string())?;
        println!(
            "{}",
            serde_json::to_string_pretty(&downloads).unwrap_or_default()
        );
        Ok(())
    })
    .await;
}

pub async fn download_fetch(url: &str) {
    let url = url.to_string();
    with_page(|page| async move {
        let b64 = onecrawl_cdp::downloads::download_file(&page, &url)
            .await
            .map_err(|e| e.to_string())?;
        println!("{b64}");
        Ok(())
    })
    .await;
}

pub async fn download_wait(timeout_ms: u64) {
    with_page(|page| async move {
        let result = onecrawl_cdp::downloads::wait_for_download(&page, timeout_ms)
            .await
            .map_err(|e| e.to_string())?;
        match result {
            Some(d) => println!("{}", serde_json::to_string_pretty(&d).unwrap_or_default()),
            None => println!("No download detected within {timeout_ms}ms"),
        }
        Ok(())
    })
    .await;
}

pub async fn download_clear() {
    with_page(|page| async move {
        onecrawl_cdp::downloads::clear_downloads(&page)
            .await
            .map_err(|e| e.to_string())?;
        println!("{} Download history cleared", "✓".green());
        Ok(())
    })
    .await;
}
