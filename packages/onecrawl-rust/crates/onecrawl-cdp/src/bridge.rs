//! Playwright bridge — spawn a Node.js Playwright process and communicate
//! via JSON messages over stdin/stdout.
//!
//! Designed for complex operations that chromiumoxide cannot handle natively
//! (e.g., file uploads, dialog handling, complex multi-page flows).

use onecrawl_core::{Error, Result};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::process::{Child, Command};

/// A message sent from Rust to the Playwright Node.js process.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BridgeCommand {
    /// Unique request ID for correlating responses.
    pub id: u64,
    /// The Playwright command to execute (e.g., "click", "fill", "upload").
    pub command: String,
    /// Arguments for the command as a JSON object.
    pub args: serde_json::Value,
}

/// A message received from the Playwright Node.js process.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BridgeResponse {
    /// Correlates to the `BridgeCommand.id`.
    pub id: u64,
    /// Whether the command succeeded.
    pub success: bool,
    /// Result data on success, error message on failure.
    pub data: serde_json::Value,
}

/// A bridge to a Node.js process running Playwright for advanced browser ops.
///
/// Communication happens via newline-delimited JSON over stdin/stdout.
/// The Node.js script path is configured at construction time.
pub struct PlaywrightBridge {
    child: Child,
    stdin: tokio::process::ChildStdin,
    reader: BufReader<tokio::process::ChildStdout>,
    next_id: u64,
}

impl PlaywrightBridge {
    /// Spawn the Playwright Node.js bridge process.
    ///
    /// `script_path` should point to a JS file that reads JSON commands from
    /// stdin and writes JSON responses to stdout (one per line).
    pub async fn start(script_path: PathBuf) -> Result<Self> {
        let mut child = Command::new("node")
            .arg(&script_path)
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .spawn()
            .map_err(|e| Error::Browser(format!("failed to spawn playwright bridge: {e}")))?;

        let stdin = child
            .stdin
            .take()
            .ok_or_else(|| Error::Browser("no stdin on bridge process".into()))?;
        let stdout = child
            .stdout
            .take()
            .ok_or_else(|| Error::Browser("no stdout on bridge process".into()))?;

        Ok(Self {
            child,
            stdin,
            reader: BufReader::new(stdout),
            next_id: 1,
        })
    }

    /// Execute a Playwright command and wait for the response.
    pub async fn execute(
        &mut self,
        command: &str,
        args: serde_json::Value,
    ) -> Result<serde_json::Value> {
        let id = self.next_id;
        self.next_id += 1;

        let cmd = BridgeCommand {
            id,
            command: command.to_string(),
            args,
        };

        let mut json = serde_json::to_string(&cmd)
            .map_err(|e| Error::Browser(format!("serialize command failed: {e}")))?;
        json.push('\n');

        self.stdin
            .write_all(json.as_bytes())
            .await
            .map_err(|e| Error::Browser(format!("write to bridge failed: {e}")))?;
        self.stdin
            .flush()
            .await
            .map_err(|e| Error::Browser(format!("flush bridge stdin failed: {e}")))?;

        let mut line = String::new();
        self.reader
            .read_line(&mut line)
            .await
            .map_err(|e| Error::Browser(format!("read from bridge failed: {e}")))?;

        let resp: BridgeResponse = serde_json::from_str(line.trim())
            .map_err(|e| Error::Browser(format!("parse bridge response failed: {e}")))?;

        if resp.id != id {
            return Err(Error::Browser(format!(
                "bridge response ID mismatch: expected {id}, got {}",
                resp.id
            )));
        }

        if !resp.success {
            return Err(Error::Browser(format!(
                "playwright command '{}' failed: {}",
                command, resp.data
            )));
        }

        Ok(resp.data)
    }

    /// Convenience wrapper: `execute_playwright(command, args)`.
    pub async fn execute_playwright(
        &mut self,
        command: &str,
        args: serde_json::Value,
    ) -> Result<serde_json::Value> {
        self.execute(command, args).await
    }

    /// Gracefully shut down the bridge process.
    pub async fn shutdown(mut self) -> Result<()> {
        // Send a shutdown command
        let _ = self.execute("shutdown", serde_json::Value::Null).await;
        let _ = self.child.kill().await;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bridge_command_serde() {
        let cmd = BridgeCommand {
            id: 1,
            command: "click".to_string(),
            args: serde_json::json!({"selector": "#btn"}),
        };
        let json = serde_json::to_string(&cmd).unwrap();
        let parsed: BridgeCommand = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.id, 1);
        assert_eq!(parsed.command, "click");
    }

    #[test]
    fn bridge_response_serde() {
        let resp = BridgeResponse {
            id: 42,
            success: true,
            data: serde_json::json!({"text": "hello"}),
        };
        let json = serde_json::to_string(&resp).unwrap();
        let parsed: BridgeResponse = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.id, 42);
        assert!(parsed.success);
    }
}
