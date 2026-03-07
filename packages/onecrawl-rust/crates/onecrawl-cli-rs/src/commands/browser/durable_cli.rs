use colored::Colorize;
use super::helpers::with_page;

// ---------------------------------------------------------------------------
// Durable Session CLI commands
// ---------------------------------------------------------------------------

pub async fn durable_start(
    name: &str,
    checkpoint_interval: &str,
    persist_state: Option<&str>,
    auto_reconnect: bool,
    max_uptime: Option<&str>,
    on_crash: &str,
) {
    let name = name.to_string();
    let interval = match onecrawl_cdp::parse_duration(checkpoint_interval) {
        Ok(d) => d.as_secs(),
        Err(e) => {
            eprintln!("{} invalid checkpoint-interval: {e}", "✗".red());
            std::process::exit(1);
        }
    };
    let state_path = persist_state
        .map(std::path::PathBuf::from)
        .unwrap_or_else(onecrawl_cdp::DurableSession::default_state_dir);
    let max_uptime_secs = max_uptime.and_then(|s| onecrawl_cdp::parse_duration(s).ok().map(|d| d.as_secs()));
    let crash_policy = match on_crash {
        "stop" => onecrawl_cdp::CrashPolicy::Stop,
        "notify" => onecrawl_cdp::CrashPolicy::Notify,
        _ => onecrawl_cdp::CrashPolicy::Restart,
    };

    with_page(|page| async move {
        let config = onecrawl_cdp::DurableConfig {
            name: name.clone(),
            checkpoint_interval_secs: interval,
            state_path: state_path.clone(),
            auto_reconnect,
            max_uptime_secs,
            on_crash: crash_policy,
            ..onecrawl_cdp::DurableConfig::default()
        };

        let mut session = onecrawl_cdp::DurableSession::new(config)
            .map_err(|e| e.to_string())?;
        let state = session
            .checkpoint(&page)
            .await
            .map_err(|e| e.to_string())?;

        println!("{} Durable session '{}' started", "✓".green(), name);
        println!("  Checkpoint interval: {}s", interval);
        println!("  State path: {}", state_path.display());
        println!("  Auto-reconnect: {}", auto_reconnect);
        if let Some(cp) = &state.last_checkpoint {
            println!("  Initial checkpoint: {}", cp);
        }
        Ok(())
    })
    .await;
}

pub async fn durable_stop(name: &str) {
    let name = name.to_string();
    with_page(|page| async move {
        let state_dir = onecrawl_cdp::DurableSession::default_state_dir();
        let config = onecrawl_cdp::DurableConfig {
            name: name.clone(),
            state_path: state_dir.clone(),
            ..onecrawl_cdp::DurableConfig::default()
        };
        let mut session = onecrawl_cdp::DurableSession::new(config)
            .map_err(|e| e.to_string())?;

        // Final checkpoint
        let _ = session.checkpoint(&page).await;

        // Mark stopped
        session.state.status = onecrawl_cdp::DurableStatus::Stopped;
        let json = serde_json::to_string_pretty(&session.state).unwrap_or_default();
        let path = state_dir.join(format!("{}.state", name));
        std::fs::write(&path, &json).map_err(|e| format!("write state: {e}"))?;

        println!("{} Durable session '{}' stopped", "✓".green(), name);
        Ok(())
    })
    .await;
}

pub async fn durable_checkpoint(name: &str) {
    let name = name.to_string();
    with_page(|page| async move {
        let state_dir = onecrawl_cdp::DurableSession::default_state_dir();
        let config = onecrawl_cdp::DurableConfig {
            name: name.clone(),
            state_path: state_dir,
            ..onecrawl_cdp::DurableConfig::default()
        };
        let mut session = onecrawl_cdp::DurableSession::new(config)
            .map_err(|e| e.to_string())?;
        let state = session
            .checkpoint(&page)
            .await
            .map_err(|e| e.to_string())?;

        println!("{} Checkpoint saved for '{}'", "✓".green(), name);
        if let Some(url) = &state.url {
            println!("  URL: {}", url);
        }
        println!("  Cookies: {}", state.cookies.len());
        println!("  localStorage: {}", state.local_storage.len());
        println!("  sessionStorage: {}", state.session_storage.len());
        Ok(())
    })
    .await;
}

pub async fn durable_restore(name: &str) {
    let name = name.to_string();
    with_page(|page| async move {
        let state_dir = onecrawl_cdp::DurableSession::default_state_dir();
        let config = onecrawl_cdp::DurableConfig {
            name: name.clone(),
            state_path: state_dir,
            ..onecrawl_cdp::DurableConfig::default()
        };
        let mut session = onecrawl_cdp::DurableSession::new(config)
            .map_err(|e| e.to_string())?;
        session.restore(&page).await.map_err(|e| e.to_string())?;

        println!("{} Session '{}' restored", "✓".green(), name);
        if let Some(url) = &session.state.url {
            println!("  URL: {}", url);
        }
        println!("  Cookies: {}", session.state.cookies.len());
        println!("  localStorage: {}", session.state.local_storage.len());
        Ok(())
    })
    .await;
}

pub async fn durable_status(name: Option<&str>) {
    let name = name.unwrap_or("default").to_string();
    let state_dir = onecrawl_cdp::DurableSession::default_state_dir();
    match onecrawl_cdp::DurableSession::get_status(&state_dir, &name) {
        Ok(state) => {
            println!(
                "{}",
                serde_json::to_string_pretty(&state).unwrap_or_default()
            );
        }
        Err(e) => {
            eprintln!("{} {e}", "✗".red());
            std::process::exit(1);
        }
    }
}

pub async fn durable_list() {
    let state_dir = onecrawl_cdp::DurableSession::default_state_dir();
    match onecrawl_cdp::DurableSession::list_sessions(&state_dir) {
        Ok(sessions) => {
            if sessions.is_empty() {
                println!("No saved durable sessions");
                return;
            }
            for s in &sessions {
                println!(
                    "  {} — {:?} — {}",
                    s.name.green(),
                    s.status,
                    s.url.as_deref().unwrap_or("(no URL)")
                );
            }
            println!("\n{} session(s) found", sessions.len());
        }
        Err(e) => {
            eprintln!("{} {e}", "✗".red());
            std::process::exit(1);
        }
    }
}

pub async fn durable_delete(name: &str) {
    let state_dir = onecrawl_cdp::DurableSession::default_state_dir();
    match onecrawl_cdp::DurableSession::delete_session(&state_dir, name) {
        Ok(()) => println!("{} Session '{}' deleted", "✓".green(), name),
        Err(e) => {
            eprintln!("{} {e}", "✗".red());
            std::process::exit(1);
        }
    }
}
