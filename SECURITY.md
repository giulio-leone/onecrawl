# Security Policy

## Supported Versions

| Version | Supported |
|---------|-----------|
| 3.9.x | ✅ Active support |
| 3.8.x | ⚠️ Critical fixes only |
| < 3.8 | ❌ No longer supported |

## Reporting a Vulnerability

**Please do not report security vulnerabilities through public GitHub issues.**

Instead, please report them via email:

📧 **security@onecrawl.dev**

Include the following in your report:

- Description of the vulnerability
- Steps to reproduce
- Potential impact
- Suggested fix (if any)

### Response Timeline

| Stage | Timeframe |
|-------|-----------|
| Acknowledgment | Within 48 hours |
| Initial assessment | Within 5 business days |
| Fix and release | Depends on severity |

We will keep you informed of our progress and coordinate disclosure timing with you.

## Security Measures in OneCrawl

OneCrawl v3.9.2 includes **28 security hardening fixes** across the codebase:

### Cryptographic Security

- **HMAC-SHA256 webhook validation** — All outbound webhooks are signed and verified to prevent tampering.
- **PBKDF2 key derivation** — Encrypted Vault uses PBKDF2 with high iteration counts for credential protection.
- **ZeroizeOnDrop** — Sensitive data (keys, tokens, passwords) is zeroed from memory when dropped.

### Input Validation & Sanitization

- **URL validation** — All user-supplied URLs are validated before processing.
- **Path traversal prevention** — File operations are sandboxed to prevent directory traversal attacks.
- **Input length limits** — Bounded inputs to prevent resource exhaustion.

### Data Integrity

- **Atomic file writes** — Session checkpoints and configuration files use atomic writes to prevent corruption.
- **Secure temporary files** — Temporary files are created with restrictive permissions and cleaned up on exit.

### Network Security

- **TLS verification** — Certificate validation is enforced by default for all outbound connections.
- **Proxy authentication** — Secure proxy credential handling with no plaintext storage.

## Security Best Practices for Users

1. **Keep OneCrawl updated** to the latest version.
2. **Use the Encrypted Vault** for storing credentials instead of plaintext configuration.
3. **Enable webhook signing** when integrating with external systems.
4. **Run with least privilege** — avoid running OneCrawl as root/administrator.
5. **Review plugin manifests** before installing third-party plugins.
