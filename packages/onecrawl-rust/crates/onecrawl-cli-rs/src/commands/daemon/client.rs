use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::UnixStream;

use super::protocol::*;

/// Send a single command to the running daemon and return its response.
pub async fn send_command(
    command: &str,
    args: serde_json::Value,
) -> Result<DaemonResponse, Box<dyn std::error::Error>> {
    send_command_with_session(command, args, None).await
}

/// Send a command targeting a specific named session.
pub async fn send_command_with_session(
    command: &str,
    args: serde_json::Value,
    session: Option<String>,
) -> Result<DaemonResponse, Box<dyn std::error::Error>> {
    let stream = UnixStream::connect(SOCKET_PATH).await?;
    let (reader, mut writer) = stream.into_split();

    let req = DaemonRequest {
        id: format!("{:x}", std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos()),
        command: command.to_string(),
        args,
        session,
    };

    let mut buf = serde_json::to_vec(&req)?;
    buf.push(b'\n');
    writer.write_all(&buf).await?;

    let mut lines = BufReader::new(reader).lines();
    let line = lines
        .next_line()
        .await?
        .ok_or("daemon closed connection without response")?;

    let resp: DaemonResponse = serde_json::from_str(&line)?;
    Ok(resp)
}

/// Check whether the daemon process is running by reading the PID file
/// and verifying the process exists.
pub fn is_daemon_running() -> bool {
    let pid_str = match std::fs::read_to_string(PID_FILE) {
        Ok(s) => s.trim().to_string(),
        Err(_) => return false,
    };
    let pid: u32 = match pid_str.parse() {
        Ok(p) => p,
        Err(_) => return false,
    };
    // `kill(pid, 0)` checks if the process exists without sending a signal.
    unsafe { libc::kill(pid as libc::pid_t, 0) == 0 }
}
