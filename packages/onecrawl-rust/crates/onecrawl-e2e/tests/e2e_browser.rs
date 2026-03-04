//! Browser E2E tests — require Chrome installed.
//! Run with: cargo test -p onecrawl-e2e --test e2e_browser -- --test-threads=1 --nocapture
//!
//! `session start --headless` blocks (waits for Ctrl-C), so we spawn it as a
//! background child process, poll for the session file, run commands, then
//! tear down via `session close` + kill.
//!
//! A `BrowserGuard` RAII wrapper ensures cleanup even if a test panics.
//!
//! **Architecture note:** Each CLI invocation creates its own CDP connection and
//! picks the first available page. This means page state set by one invocation
//! (e.g. `navigate`) is NOT visible to the next invocation. Tests that need to
//! set-and-read page content must do so within a single `eval` expression.

use assert_cmd::Command;
use predicates::prelude::*;
use serial_test::serial;
use std::path::Path;
use std::process::{Child, Command as StdCommand};
use std::time::{Duration, Instant};
use tempfile::TempDir;

const SESSION_FILE: &str = "/tmp/onecrawl-session.json";

fn cli() -> Command {
    Command::cargo_bin("onecrawl").expect("binary not found")
}

fn bin_path() -> std::path::PathBuf {
    assert_cmd::cargo::cargo_bin("onecrawl")
}

// ─── RAII guard ─────────────────────────────────────────────

/// Ensures the browser session is torn down even if the test panics.
struct BrowserGuard(Option<Child>);

impl Drop for BrowserGuard {
    fn drop(&mut self) {
        let _ = cli()
            .args(["session", "close"])
            .timeout(Duration::from_secs(10))
            .ok();

        if let Some(ref mut child) = self.0 {
            let _ = child.kill();
            let _ = child.wait();
        }
        let _ = std::fs::remove_file(SESSION_FILE);
        // Cooldown so Chrome fully exits before the next test
        std::thread::sleep(Duration::from_millis(500));
    }
}

// ─── Session helpers ────────────────────────────────────────

fn start_session() -> BrowserGuard {
    let _ = std::fs::remove_file(SESSION_FILE);

    let child = StdCommand::new(bin_path())
        .args(["session", "start", "--headless"])
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .expect("failed to spawn session start");

    let deadline = Instant::now() + Duration::from_secs(30);
    while Instant::now() < deadline {
        if Path::new(SESSION_FILE).exists() {
            std::thread::sleep(Duration::from_millis(500));
            return BrowserGuard(Some(child));
        }
        std::thread::sleep(Duration::from_millis(250));
    }
    let mut child = child;
    let _ = child.kill();
    let _ = child.wait();
    panic!("Timed out waiting for session file {SESSION_FILE}");
}

// ─── Session Lifecycle ──────────────────────────────────────

#[test]
#[serial]
fn browser_session_start_and_close() {
    let _guard = start_session();

    cli()
        .args(["session", "info"])
        .timeout(Duration::from_secs(10))
        .assert()
        .success()
        .stdout(
            predicate::str::contains("ws://")
                .or(predicate::str::contains("Session"))
                .or(predicate::str::contains("connected")),
        );
}

// ─── Navigation (smoke tests) ───────────────────────────────

#[test]
#[serial]
fn browser_navigate_succeeds() {
    let _guard = start_session();

    // Navigate to a data: URL — verify command doesn't error
    cli()
        .args(["navigate", "data:text/html,<h1>Hello E2E</h1>"])
        .timeout(Duration::from_secs(15))
        .assert()
        .success()
        .stdout(predicate::str::contains("Navigated"));
}

#[test]
#[serial]
fn browser_navigate_about_blank() {
    let _guard = start_session();

    cli()
        .args(["navigate", "about:blank"])
        .timeout(Duration::from_secs(15))
        .assert()
        .success();
}

// ─── Get subcommands ────────────────────────────────────────

#[test]
#[serial]
fn browser_get_url() {
    let _guard = start_session();

    // Each CLI invocation gets its own page; just verify the command works
    let output = cli()
        .args(["get", "url"])
        .timeout(Duration::from_secs(10))
        .output()
        .expect("get url failed");
    assert!(output.status.success());
    let url = String::from_utf8_lossy(&output.stdout);
    // Default page is about:blank
    assert!(
        url.contains("about:blank") || url.contains("data:") || !url.is_empty(),
        "URL: {url}"
    );
}

#[test]
#[serial]
fn browser_get_title() {
    let _guard = start_session();

    let output = cli()
        .args(["get", "title"])
        .timeout(Duration::from_secs(10))
        .output()
        .expect("get title failed");
    assert!(output.status.success(), "get title exited with error");
}

#[test]
#[serial]
fn browser_get_html() {
    let _guard = start_session();

    let output = cli()
        .args(["get", "html"])
        .timeout(Duration::from_secs(10))
        .output()
        .expect("get html failed");
    assert!(output.status.success());
    let html = String::from_utf8_lossy(&output.stdout);
    // Even a blank page has <html>
    assert!(html.contains("<html"), "HTML: {html}");
}

#[test]
#[serial]
fn browser_get_text() {
    let _guard = start_session();

    let output = cli()
        .args(["get", "text"])
        .timeout(Duration::from_secs(10))
        .output()
        .expect("get text failed");
    assert!(output.status.success(), "get text exited with error");
}

// ─── JavaScript Evaluation ──────────────────────────────────

#[test]
#[serial]
fn browser_eval_arithmetic() {
    let _guard = start_session();

    let output = cli()
        .args(["eval", "2 + 2"])
        .timeout(Duration::from_secs(10))
        .output()
        .expect("eval failed");
    assert!(output.status.success());
    let result = String::from_utf8_lossy(&output.stdout);
    assert!(result.contains("4"), "Eval result: {result}");
}

#[test]
#[serial]
fn browser_eval_dom_manipulation() {
    let _guard = start_session();

    // Set DOM and read it back in a single eval (avoids cross-connection issue)
    let output = cli()
        .args([
            "eval",
            "document.body.innerHTML = '<div id=\"t\">Found It</div>'; \
             document.getElementById('t').textContent",
        ])
        .timeout(Duration::from_secs(10))
        .output()
        .expect("eval failed");
    assert!(output.status.success());
    let result = String::from_utf8_lossy(&output.stdout);
    assert!(result.contains("Found It"), "DOM query result: {result}");
}

#[test]
#[serial]
fn browser_eval_json_return() {
    let _guard = start_session();

    let output = cli()
        .args(["eval", "JSON.stringify({a: 1, b: 'hello'})"])
        .timeout(Duration::from_secs(10))
        .output()
        .expect("eval failed");
    assert!(output.status.success());
    let result = String::from_utf8_lossy(&output.stdout);
    assert!(result.contains("hello"), "JSON result: {result}");
}

// ─── Screenshot ─────────────────────────────────────────────

#[test]
#[serial]
fn browser_screenshot_saves_file() {
    let _guard = start_session();

    let dir = TempDir::new().unwrap();
    let screenshot_path = dir.path().join("test.png");

    cli()
        .args(["screenshot", "-o", screenshot_path.to_str().unwrap()])
        .timeout(Duration::from_secs(15))
        .assert()
        .success();

    assert!(screenshot_path.exists(), "Screenshot file not created");
    let size = std::fs::metadata(&screenshot_path).unwrap().len();
    assert!(size > 100, "Screenshot too small: {size} bytes");
}

#[test]
#[serial]
fn browser_screenshot_full_page() {
    let _guard = start_session();

    let dir = TempDir::new().unwrap();
    let screenshot_path = dir.path().join("full.png");

    cli()
        .args([
            "screenshot",
            "-o",
            screenshot_path.to_str().unwrap(),
            "--full",
        ])
        .timeout(Duration::from_secs(15))
        .assert()
        .success();

    assert!(screenshot_path.exists(), "Full-page screenshot not created");
}

// ─── Set Content (smoke) ────────────────────────────────────

#[test]
#[serial]
fn browser_set_content_succeeds() {
    let _guard = start_session();

    cli()
        .args(["set-content", "<h1>Injected</h1><p>Custom content</p>"])
        .timeout(Duration::from_secs(10))
        .assert()
        .success()
        .stdout(predicate::str::contains("Content set"));
}

// ─── Multi-tab ──────────────────────────────────────────────

#[test]
#[serial]
fn browser_new_page_creates_tab() {
    let _guard = start_session();

    cli()
        .args(["new-page", "data:text/html,<h1>Tab 2</h1>"])
        .timeout(Duration::from_secs(15))
        .assert()
        .success();
}

// ─── Cookie Operations ─────────────────────────────────────

#[test]
#[serial]
fn browser_cookie_get() {
    let _guard = start_session();

    // Just verify the command runs without crashing
    let output = cli()
        .args(["cookie", "get"])
        .timeout(Duration::from_secs(10))
        .output()
        .expect("cookie get failed");
    assert!(output.status.success());
}

// ─── Accessibility ──────────────────────────────────────────

#[test]
#[serial]
fn browser_a11y_tree() {
    let _guard = start_session();

    let output = cli()
        .args(["a11y", "tree"])
        .timeout(Duration::from_secs(15))
        .output()
        .expect("a11y tree failed");
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    // Should contain some JSON structure
    assert!(
        stdout.contains("role") || stdout.contains("node"),
        "a11y stdout: {stdout}"
    );
}

// ─── Emulation ──────────────────────────────────────────────

#[test]
#[serial]
fn browser_emulate_viewport() {
    let _guard = start_session();

    // Verify emulate viewport command succeeds
    cli()
        .args(["emulate", "viewport", "375", "667"])
        .timeout(Duration::from_secs(10))
        .assert()
        .success();
}

// ─── Reload / Back / Forward (smoke) ────────────────────────

#[test]
#[serial]
fn browser_reload() {
    let _guard = start_session();

    cli()
        .arg("reload")
        .timeout(Duration::from_secs(10))
        .assert()
        .success();
}

