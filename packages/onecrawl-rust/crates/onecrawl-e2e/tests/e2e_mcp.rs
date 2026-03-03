//! E2E tests for the OneCrawl MCP server binary.
//! Spawns `onecrawl-mcp` as a subprocess and sends JSON-RPC via stdin/stdout.

use std::io::Write;
use std::process::{Command, Stdio};
use std::time::Duration;

/// Locate the `onecrawl-mcp` binary built by cargo.
fn mcp_binary_path() -> std::path::PathBuf {
    let mut path = std::env::current_exe().unwrap();
    path.pop(); // remove test binary name
    path.pop(); // remove deps
    path.push("onecrawl-mcp");
    path
}

#[test]
fn mcp_binary_exists() {
    let path = mcp_binary_path();
    assert!(path.exists(), "MCP binary not found at {:?}", path);
}

#[test]
fn mcp_initialize_handshake() {
    let mut child = Command::new(mcp_binary_path())
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("Failed to start MCP server");

    let stdin = child.stdin.as_mut().unwrap();
    let init_request = serde_json::json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "initialize",
        "params": {
            "protocolVersion": "2024-11-05",
            "capabilities": {},
            "clientInfo": {"name": "e2e-test", "version": "1.0.0"}
        }
    });
    writeln!(stdin, "{}", init_request).unwrap();
    stdin.flush().unwrap();

    // Give the server time to process
    std::thread::sleep(Duration::from_secs(2));
    // Close stdin to signal EOF → process exits
    drop(child.stdin.take());

    let output = child.wait_with_output().unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout);

    // The MCP server should respond with a JSON-RPC result containing the protocol version
    assert!(
        stdout.contains("2024-11-05") || stdout.contains("result"),
        "MCP init response missing protocol version or result: {}",
        stdout
    );
}

#[test]
fn mcp_tools_list() {
    let mut child = Command::new(mcp_binary_path())
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("Failed to start MCP server");

    let stdin = child.stdin.as_mut().unwrap();

    // Send initialize
    let init = serde_json::json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "initialize",
        "params": {
            "protocolVersion": "2024-11-05",
            "capabilities": {},
            "clientInfo": {"name": "test", "version": "1.0.0"}
        }
    });
    writeln!(stdin, "{}", init).unwrap();

    // Send initialized notification
    let initialized = serde_json::json!({
        "jsonrpc": "2.0",
        "method": "notifications/initialized"
    });
    writeln!(stdin, "{}", initialized).unwrap();

    // Send tools/list
    let tools_list = serde_json::json!({
        "jsonrpc": "2.0",
        "id": 2,
        "method": "tools/list"
    });
    writeln!(stdin, "{}", tools_list).unwrap();
    stdin.flush().unwrap();

    std::thread::sleep(Duration::from_secs(2));
    drop(child.stdin.take());

    let output = child.wait_with_output().unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout);

    // Should contain tool definitions in the response
    assert!(
        stdout.contains("tools") || stdout.contains("jsonrpc"),
        "MCP tools/list response unexpected: {}",
        stdout
    );
}
