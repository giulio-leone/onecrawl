pub mod client;
pub mod protocol;
pub mod server;

use colored::Colorize;

/// Start the daemon in a detached child process and return immediately.
pub async fn daemon_start(headless: bool) {
    if client::is_daemon_running() {
        println!(
            "{} Daemon is already running (pid file: {})",
            "✓".green(),
            protocol::PID_FILE.cyan()
        );
        return;
    }

    let exe = match std::env::current_exe() {
        Ok(p) => p,
        Err(e) => {
            eprintln!("{} Failed to determine executable path: {e}", "✗".red());
            std::process::exit(1);
        }
    };

    let mut cmd = std::process::Command::new(&exe);
    cmd.arg("daemon").arg("run");
    if headless {
        cmd.arg("--headless");
    }
    // Detach: redirect stdio to /dev/null so the child survives parent exit.
    cmd.stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null());

    // On Unix, start a new session so the child is fully detached.
    #[cfg(unix)]
    {
        use std::os::unix::process::CommandExt;
        unsafe {
            cmd.pre_exec(|| {
                libc::setsid();
                Ok(())
            });
        }
    }

    match cmd.spawn() {
        Ok(child) => {
            // Give the daemon a moment to write its PID file.
            tokio::time::sleep(std::time::Duration::from_millis(500)).await;
            println!(
                "{} Daemon started (child pid={}, socket={})",
                "✓".green(),
                child.id(),
                protocol::SOCKET_PATH.cyan()
            );
        }
        Err(e) => {
            eprintln!("{} Failed to spawn daemon: {e}", "✗".red());
            std::process::exit(1);
        }
    }
}

/// Stop the running daemon by sending a shutdown command.
pub async fn daemon_stop() {
    if !client::is_daemon_running() {
        println!("{} Daemon is not running", "✗".red());
        return;
    }

    match client::send_command("shutdown", serde_json::Value::Null).await {
        Ok(resp) if resp.success => {
            // Wait briefly for cleanup.
            tokio::time::sleep(std::time::Duration::from_millis(300)).await;
            println!("{} Daemon stopped", "✓".green());
        }
        Ok(resp) => {
            eprintln!(
                "{} Shutdown command failed: {}",
                "✗".red(),
                resp.error.unwrap_or_default()
            );
        }
        Err(e) => {
            eprintln!("{} Could not reach daemon: {e}", "✗".red());
            // Attempt force-kill via PID file.
            if let Ok(pid_str) = std::fs::read_to_string(protocol::PID_FILE) {
                if let Ok(pid) = pid_str.trim().parse::<u32>() {
                    unsafe {
                        libc::kill(pid as libc::pid_t, libc::SIGTERM);
                    }
                    let _ = std::fs::remove_file(protocol::PID_FILE);
                    let _ = std::fs::remove_file(protocol::SOCKET_PATH);
                    eprintln!("{} Sent SIGTERM to pid {pid}", "⚠".yellow());
                }
            }
        }
    }
}

/// Print daemon status.
pub async fn daemon_status() {
    if !client::is_daemon_running() {
        println!("{} Daemon is not running", "●".red());
        return;
    }

    match client::send_command("status", serde_json::Value::Null).await {
        Ok(resp) if resp.success => {
            if let Some(data) = resp.data {
                let pid = data.get("pid").and_then(|v| v.as_u64()).unwrap_or(0);
                let headless = data.get("headless").and_then(|v| v.as_bool()).unwrap_or(false);
                let sessions = data
                    .get("sessions")
                    .and_then(|v| v.as_array())
                    .map(|arr| {
                        arr.iter()
                            .filter_map(|v| v.as_str())
                            .collect::<Vec<_>>()
                            .join(", ")
                    })
                    .unwrap_or_default();

                println!("{} Daemon running", "●".green());
                println!("  PID:      {pid}");
                println!("  Headless: {headless}");
                println!("  Socket:   {}", protocol::SOCKET_PATH.cyan());
                println!("  Sessions: {sessions}");
            }
        }
        Ok(resp) => {
            eprintln!(
                "{} Status check failed: {}",
                "✗".red(),
                resp.error.unwrap_or_default()
            );
        }
        Err(e) => {
            eprintln!("{} Could not reach daemon: {e}", "✗".red());
        }
    }
}

/// Send an arbitrary command to the daemon and print the response.
pub async fn daemon_exec(command: &str, args: Vec<String>, session: Option<String>) {
    if !client::is_daemon_running() {
        eprintln!(
            "{} Daemon is not running. Start it with: {}",
            "✗".red(),
            "onecrawl daemon start".yellow()
        );
        std::process::exit(1);
    }

    // Build args JSON from the positional key=value pairs.
    let args_json = parse_kv_args(&args);

    let resp = match client::send_command_with_session(command, args_json, session).await {
        Ok(r) => r,
        Err(e) => {
            eprintln!("{} {e}", "✗".red());
            std::process::exit(1);
        }
    };

    if resp.success {
        if let Some(data) = resp.data {
            println!("{}", serde_json::to_string_pretty(&data).unwrap_or_default());
        } else {
            println!("{} OK", "✓".green());
        }
    } else {
        eprintln!(
            "{} {}",
            "✗".red(),
            resp.error.unwrap_or_else(|| "unknown error".into())
        );
        std::process::exit(1);
    }
}

/// Parse `["key=value", ...]` into a JSON object. Bare values without `=`
/// are collected into a `"_positional"` array.
fn parse_kv_args(args: &[String]) -> serde_json::Value {
    if args.is_empty() {
        return serde_json::Value::Null;
    }
    let mut map = serde_json::Map::new();
    let mut positional = Vec::new();
    for arg in args {
        if let Some(idx) = arg.find('=') {
            let key = &arg[..idx];
            let val = &arg[idx + 1..];
            map.insert(key.to_string(), serde_json::Value::String(val.to_string()));
        } else {
            positional.push(serde_json::Value::String(arg.clone()));
        }
    }
    if !positional.is_empty() {
        map.insert("_positional".into(), serde_json::Value::Array(positional));
    }
    serde_json::Value::Object(map)
}
