//! E2E tests for the OneCrawl CLI binary.
//! Spawns the `onecrawl` binary as a subprocess and validates commands.

use assert_cmd::Command;
use predicates::prelude::*;

fn cli() -> Command {
    Command::cargo_bin("onecrawl").expect("onecrawl binary not found")
}

// ── Version & Help ──────────────────────────────────────────

#[test]
fn cli_version_shows_semver() {
    cli().arg("version")
        .assert()
        .success()
        .stdout(predicate::str::is_match(r"onecrawl \d+\.\d+\.\d+").unwrap());
}

#[test]
fn cli_help_lists_subcommands() {
    cli().arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains("crypto"))
        .stdout(predicate::str::contains("parse"))
        .stdout(predicate::str::contains("storage"));
}

#[test]
fn cli_health_check() {
    cli().arg("health")
        .assert()
        .success()
        .stdout(predicate::str::is_match(r"\d+\.\d+\.\d+").unwrap());
}

#[test]
fn cli_unknown_command_fails() {
    cli().arg("nonexistent-command-xyz")
        .assert()
        .failure();
}

// ── Parse subcommands (stdin) ───────────────────────────────

#[test]
fn cli_parse_text_from_stdin() {
    cli().args(["parse", "text", "body"])
        .write_stdin("<html><body><h1>Title</h1><p>Content</p></body></html>")
        .assert()
        .success()
        .stdout(predicate::str::contains("Title"))
        .stdout(predicate::str::contains("Content"));
}

#[test]
fn cli_parse_links_from_stdin() {
    cli().args(["parse", "links"])
        .write_stdin(r#"<html><body><a href="https://example.com">Link</a></body></html>"#)
        .assert()
        .success()
        .stdout(predicate::str::contains("https://example.com"));
}

#[test]
fn cli_parse_query_from_stdin() {
    cli().args(["parse", "query", "h1"])
        .write_stdin("<html><body><h1>Hello</h1></body></html>")
        .assert()
        .success()
        .stdout(predicate::str::contains("Hello"));
}

#[test]
fn cli_parse_a11y_from_stdin() {
    cli().args(["parse", "a11y"])
        .write_stdin(r#"<html><body><button>Click Me</button></body></html>"#)
        .assert()
        .success()
        .stdout(predicate::str::contains("button").or(predicate::str::contains("Click Me")));
}

// ── Crypto roundtrip ────────────────────────────────────────

#[test]
fn cli_crypto_encrypt_decrypt_roundtrip() {
    // Encrypt: `crypto encrypt <data> -p <passphrase>`
    let encrypt_output = cli()
        .args(["crypto", "encrypt", "secret message", "--passphrase", "testpass123"])
        .output()
        .expect("encrypt failed");
    assert!(encrypt_output.status.success(), "encrypt failed: {}", String::from_utf8_lossy(&encrypt_output.stderr));

    let payload_json = String::from_utf8(encrypt_output.stdout).unwrap();
    let payload_json = payload_json.trim();

    // Decrypt: `crypto decrypt <json-payload> -p <passphrase>`
    cli().args(["crypto", "decrypt", payload_json, "--passphrase", "testpass123"])
        .assert()
        .success()
        .stdout(predicate::str::contains("secret message"));
}

#[test]
fn cli_crypto_decrypt_wrong_password_fails() {
    let encrypt_output = cli()
        .args(["crypto", "encrypt", "secret", "--passphrase", "rightpass"])
        .output()
        .expect("encrypt failed");
    assert!(encrypt_output.status.success());

    let payload_json = String::from_utf8(encrypt_output.stdout).unwrap();
    let payload_json = payload_json.trim();

    cli().args(["crypto", "decrypt", payload_json, "--passphrase", "wrongpass"])
        .assert()
        .failure();
}

// ── Storage roundtrip ───────────────────────────────────────

#[test]
fn cli_storage_set_get_roundtrip() {
    let dir = tempfile::tempdir().unwrap();
    let db_path = dir.path().join("test-store");
    let db = db_path.to_str().unwrap();

    // Set
    cli().args(["storage", "set", "mykey", "myvalue", "--path", db, "-P", "testpass"])
        .assert()
        .success();

    // Get
    cli().args(["storage", "get", "mykey", "--path", db, "-P", "testpass"])
        .assert()
        .success()
        .stdout(predicate::str::contains("myvalue"));
}

#[test]
fn cli_storage_list_keys() {
    let dir = tempfile::tempdir().unwrap();
    let db_path = dir.path().join("test-store");
    let db = db_path.to_str().unwrap();

    cli().args(["storage", "set", "alpha", "1", "--path", db, "-P", "p"])
        .assert().success();
    cli().args(["storage", "set", "beta", "2", "--path", db, "-P", "p"])
        .assert().success();

    cli().args(["storage", "list", "--path", db, "-P", "p"])
        .assert()
        .success()
        .stdout(predicate::str::contains("alpha"))
        .stdout(predicate::str::contains("beta"));
}

#[test]
fn cli_storage_delete_key() {
    let dir = tempfile::tempdir().unwrap();
    let db_path = dir.path().join("test-store");
    let db = db_path.to_str().unwrap();

    cli().args(["storage", "set", "delme", "val", "--path", db, "-P", "p"])
        .assert().success();
    cli().args(["storage", "delete", "delme", "--path", db, "-P", "p"])
        .assert().success();

    // After delete, get should fail (key not found exits with code 1)
    cli().args(["storage", "get", "delme", "--path", db, "-P", "p"])
        .assert()
        .failure();
}

// ── Crypto PKCE ─────────────────────────────────────────────

#[test]
fn cli_crypto_pkce_generates_pair() {
    cli().args(["crypto", "pkce"])
        .assert()
        .success()
        .stdout(predicate::str::contains("code_verifier"))
        .stdout(predicate::str::contains("code_challenge"));
}
