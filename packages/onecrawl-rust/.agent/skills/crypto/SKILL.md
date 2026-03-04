---
name: crypto
description: "AES-256-GCM encryption, PKCE S256 challenges, TOTP code generation, PBKDF2 key derivation, WebAuthn/passkey virtual authenticator, and encrypted KV storage."
---

# Crypto & Authentication Skill

Cryptographic operations, passkey/WebAuthn simulation, and encrypted storage.
All crypto uses the `ring` crate for FIPS-grade primitives.

## Modules

| Module | Purpose |
|--------|---------|
| `onecrawl-crypto` | AES-256-GCM, PKCE S256, TOTP, PBKDF2 |
| `onecrawl-storage` | sled-backed encrypted KV store |
| `webauthn` | Virtual WebAuthn authenticator (CTAP2/U2F) |

## CLI Commands

### Encryption

```bash
# Encrypt text with password
onecrawl crypto encrypt --plaintext "secret data" --password "mypass"
# Output: base64-encoded ciphertext (salt + nonce + ciphertext)

# Decrypt
onecrawl crypto decrypt --ciphertext "base64..." --password "mypass"
```

### PKCE (Proof Key for Code Exchange)

```bash
# Generate S256 challenge pair for OAuth flows
onecrawl crypto pkce
# Output: { "code_verifier": "...", "code_challenge": "..." }
```

### TOTP (Time-based One-Time Password)

```bash
# Generate 6-digit code from base32 secret
onecrawl crypto totp --secret "JBSWY3DPEHPK3PXP"
# Output: 123456
```

### Encrypted Storage

```bash
onecrawl storage set mykey "sensitive value"
onecrawl storage get mykey
onecrawl storage list
onecrawl storage delete mykey
```

### WebAuthn / Passkeys

```bash
# Enable virtual authenticator
onecrawl auth passkey-enable --protocol ctap2 --transport internal

# Add a credential
onecrawl auth passkey-add \
  --credential_id "cred123" \
  --rp_id "example.com"

# List stored credentials
onecrawl auth passkey-list

# View operation log (sign count, assertions)
onecrawl auth passkey-log

# Remove a credential
onecrawl auth passkey-remove --credential_id "cred123"

# Disable authenticator
onecrawl auth passkey-disable
```

## MCP Tools

| Tool | Description |
|------|-------------|
| `encrypt` | AES-256-GCM encrypt (returns base64) |
| `decrypt` | AES-256-GCM decrypt |
| `generate_pkce` | PKCE S256 challenge pair |
| `generate_totp` | 6-digit TOTP code |
| `store_set` | Set encrypted KV pair |
| `store_get` | Get value by key |
| `store_list_keys` | List all stored keys |
| `auth_passkey_enable` | Enable virtual authenticator |
| `auth_passkey_create` | Add passkey credential |
| `auth_passkey_list` | List credentials |
| `auth_passkey_log` | View operation log |
| `auth_passkey_disable` | Disable authenticator |
| `auth_passkey_remove` | Remove credential by ID |

## How Passkeys Work

1. `passkey-enable` creates a virtual WebAuthn authenticator via CDP
2. The authenticator intercepts `navigator.credentials.create()` and `.get()` calls
3. Credentials are stored in-memory with sign counters
4. Supports both CTAP2 and U2F protocols
5. Transport options: internal, usb, nfc, ble

This enables testing passkey flows without physical hardware keys.

## Encryption Details

- **Algorithm**: AES-256-GCM (authenticated encryption)
- **Key derivation**: PBKDF2-HMAC-SHA256 with random 16-byte salt
- **Nonce**: Random 12-byte nonce per encryption
- **Wire format**: `salt(16) || nonce(12) || ciphertext(N)`, base64-encoded
- **Storage**: sled embedded database with per-value encryption
