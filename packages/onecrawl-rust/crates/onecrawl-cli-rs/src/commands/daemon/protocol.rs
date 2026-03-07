use serde::{Deserialize, Serialize};

/// Request sent from CLI client to the daemon over Unix socket.
#[derive(Debug, Serialize, Deserialize)]
pub struct DaemonRequest {
    pub id: String,
    pub command: String,
    pub args: serde_json::Value,
    /// Named session — defaults to `"default"` when absent.
    #[serde(default = "default_session_name")]
    pub session: Option<String>,
}

/// Response sent from the daemon back to the CLI client.
#[derive(Debug, Serialize, Deserialize)]
pub struct DaemonResponse {
    pub id: String,
    pub success: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

fn default_session_name() -> Option<String> {
    Some("default".to_string())
}

pub const SOCKET_PATH: &str = "/tmp/onecrawl-daemon.sock";
pub const PID_FILE: &str = "/tmp/onecrawl-daemon.pid";
pub const STATE_FILE: &str = "/tmp/onecrawl-daemon-state.json";
/// Default idle timeout before the daemon shuts itself down (seconds).
pub const DEFAULT_IDLE_TIMEOUT_SECS: u64 = 1800; // 30 minutes
