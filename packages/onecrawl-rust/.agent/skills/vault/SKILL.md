# Encrypted Vault Skill

## Overview

The Vault provides secure, encrypted credential storage for browser automation workflows. Secrets are encrypted with AES-256-GCM, key-derived via PBKDF2, and stored in a single encrypted file. Supports categories, expiration, environment variable import, service templates, and workflow export. Memory is protected with `ZeroizeOnDrop`.

## Key Files

- `crates/onecrawl-cdp/src/vault.rs` — Core `Vault` engine with encryption/decryption
- `crates/onecrawl-mcp-rs/src/handlers/vault.rs` — 9 MCP action handlers
- `crates/onecrawl-cli-rs/src/commands/vault.rs` — CLI vault commands

## API Reference

### MCP Actions

| Action | Description | Parameters |
|--------|-------------|------------|
| `vault_create` | Create a new encrypted vault file | `password`, `path?` (default: `~/.onecrawl/vault.enc`) |
| `vault_open` | Open and decrypt an existing vault | `password`, `path?` |
| `vault_set` | Store or update a secret | `password`, `key`, `value`, `category?`, `path?` |
| `vault_get` | Retrieve a secret value | `password`, `key`, `path?` |
| `vault_delete` | Delete a secret | `password`, `key`, `path?` |
| `vault_list` | List entries (no values exposed) | `password`, `category?`, `path?` |
| `vault_use` | Export service credentials for workflow | `password`, `service`, `path?` |
| `vault_change_password` | Re-encrypt vault with new password | `password`, `new_password`, `path?` |
| `vault_import_env` | Import environment variables by prefix | `password`, `prefix?` (default: `ONECRAWL_VAULT_`), `path?` |

### CLI Commands

| Command | Description |
|---------|-------------|
| `onecrawl vault create <path>` | Create new vault (prompts for password) |
| `onecrawl vault open <path>` | Open vault (shows entry count, expired keys) |
| `onecrawl vault set <key> [value]` | Store secret (supports `--category`, `--prompt` for hidden input) |
| `onecrawl vault get <key>` | Retrieve and display secret value |
| `onecrawl vault delete <key>` | Delete a secret |
| `onecrawl vault list` | List entries with columns: KEY, CATEGORY, UPDATED, STATUS |
| `onecrawl vault use <service>` | Export service credentials as key-value pairs |
| `onecrawl vault change-password` | Re-encrypt with new master password |
| `onecrawl vault import-env <prefix>` | Import matching environment variables |

### Core Rust API

```rust
use onecrawl_cdp::{Vault, VaultEntry, VaultEntrySummary};

// Create new vault
let mut vault = Vault::create("~/.onecrawl/vault.enc", "master-password")?;

// Store secrets
vault.set("linkedin.email", "user@example.com", Some("linkedin"))?;
vault.set("linkedin.password", "secret123", Some("linkedin"))?;
vault.save()?;

// Retrieve
let entry = vault.get("linkedin.email");  // Option<&VaultEntry>

// List (safe summaries, no values)
let summaries: Vec<VaultEntrySummary> = vault.list();
let linkedin_only = vault.list_by_category("linkedin");

// Export for workflow automation
let creds: HashMap<String, String> = vault.export_for_workflow("linkedin");
// Returns: {"email": "user@...", "password": "secret123"}

// Import from environment
vault.import_env("ONECRAWL_VAULT_")?;  // ONECRAWL_VAULT_FOO=bar → key "foo"

// Change master password
vault.change_password("new-password")?;

// Check expired entries
let expired: Vec<String> = vault.check_expired();
```

### Service Templates

Pre-defined templates for common services:

| Service | Required Fields |
|---------|----------------|
| `linkedin` | `email`, `password` |
| `github` | `username`, `token` |
| `google` | `email`, `password` |
| `twitter` | `username`, `password` |
| `aws` | `access_key_id`, `secret_access_key` |

## Architecture

### Encryption

```
Master Password → PBKDF2 (100,000 iterations, SHA-256) → AES-256-GCM Key
                                                              ↓
Plaintext JSON ────────────────────────────── AES-256-GCM ── → Ciphertext
              ← Random 12-byte nonce ─────────┘
```

- **Key Derivation**: PBKDF2 with 100,000 iterations and random 32-byte salt
- **Encryption**: AES-256-GCM with random 12-byte nonce per save
- **Storage Format**: JSON wrapper with version, salt (hex), nonce (hex), ciphertext (hex)
- **Memory Safety**: Passphrase field uses `Zeroize` for cleanup on drop

### File Format

```json
{
  "version": 1,
  "salt": "hex...",
  "nonce": "hex...",
  "ciphertext": "hex..."
}
```

### Atomic Writes

Vault saves use a safe write pattern:
1. Write to temporary file (`{path}.tmp`)
2. `fsync` the temp file
3. Rename temp → target (atomic on most filesystems)

This prevents data loss on crash during write.

### Entry Structure

```rust
VaultEntry {
    key: String,            // e.g., "linkedin.email"
    value: String,          // The actual secret
    category: Option<String>, // e.g., "linkedin"
    created_at: String,     // ISO 8601 UTC
    updated_at: String,     // ISO 8601 UTC
    expires_at: Option<String>, // Optional expiration
    metadata: Option<HashMap<String, String>>,
}
```

## Best Practices

- Use dotted key names for organization: `service.field` (e.g., `linkedin.email`)
- Always set `category` to enable per-service listing and `vault_use` export
- Use `vault_use` in automation workflows to extract credentials by service name
- Use `vault_import_env` for CI/CD environments where secrets come from env vars
- Rotate master passwords periodically with `vault_change_password`
- Store vault at default path (`~/.onecrawl/vault.enc`) for seamless integration
- Use `--prompt` flag in CLI to avoid secrets appearing in shell history
- Combine with Durable Sessions to persist authenticated browser state

## Troubleshooting

| Issue | Cause | Solution |
|-------|-------|----------|
| "Wrong password" on open | Incorrect master password | Verify password; no recovery mechanism exists |
| Vault file corrupted | Interrupted write | Restore from backup; use atomic write feature |
| `vault_use` returns empty | Keys not categorized | Ensure entries have `category` matching the service name |
| Expired entries still accessible | Expiration is metadata-only | Use `check_expired()` to audit; delete manually |
| Import finds no env vars | Prefix mismatch | Check prefix matches exactly (default: `ONECRAWL_VAULT_`) |
| Path resolution fails | `~` not expanded | CLI handles `~` expansion; for MCP, use absolute paths |
