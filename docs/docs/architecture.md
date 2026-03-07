---
sidebar_position: 7
title: Architecture
---

# Architecture

## Overview

OneCrawl is a Rust monorepo containing **8 core crates** and **2 binding crates** that compile into a single CLI binary, an MCP server, an HTTP API server, and native libraries for Node.js and Python.

All crates live under `packages/onecrawl-rust/crates/`:

```
packages/onecrawl-rust/
├── crates/
│   ├── onecrawl-core/          # Shared types, traits, errors
│   ├── onecrawl-crypto/        # AES-256-GCM, PKCE, TOTP, hashing
│   ├── onecrawl-parser/        # HTML parsing, accessibility tree
│   ├── onecrawl-storage/       # Encrypted key-value store (sled)
│   ├── onecrawl-cdp/           # Chrome DevTools Protocol (97 modules, 662 functions)
│   ├── onecrawl-server/        # HTTP API server (axum, 43 routes)
│   ├── onecrawl-cli-rs/        # CLI (409+ commands)
│   └── onecrawl-mcp-rs/        # MCP server (17 super-tools, 421 actions)
└── bindings/
    ├── napi/                   # Node.js bindings (NAPI-RS, 391 methods)
    └── python/                 # Python bindings (PyO3, 509 methods)
```

---

## Crate Dependency Graph

```
                          ┌─────────────────┐
                          │  onecrawl-core   │
                          │  (types/traits)  │
                          └────────┬─────────┘
               ┌──────────┬───────┼────────┬──────────┐
               │          │       │        │          │
        ┌──────▼───┐ ┌────▼────┐ │  ┌─────▼────┐ ┌───▼──────┐
        │  crypto  │ │ parser  │ │  │ storage  │ │   cdp    │
        │(ring,AES)│ │(lol_html│ │  │  (sled)  │ │(97 CDP   │
        │          │ │ scraper)│ │  │          │ │ modules) │
        └──────┬───┘ └────┬───┘ │  └─────┬────┘ └───┬──────┘
               │          │     │        │           │
               └──────────┴─────┼────────┴───────────┘
                                │
                    ┌───────────┼───────────┐
                    │           │           │
              ┌─────▼─────┐ ┌──▼──────┐ ┌──▼──────┐
              │  server   │ │ cli-rs  │ │ mcp-rs  │
              │  (axum)   │ │ (clap)  │ │ (rmcp)  │
              │ 43 routes │ │ 409+cmd │ │17 tools │
              └───────────┘ └─────────┘ └─────────┘
                    │           │           │
                    └───────────┼───────────┘
                                │
                    ┌───────────┼───────────┐
                    │                       │
              ┌─────▼─────┐          ┌──────▼─────┐
              │bindings/  │          │ bindings/  │
              │  napi     │          │  python    │
              │ 391 meth  │          │ 509 meth   │
              └───────────┘          └────────────┘
```

---

## Core Crates

### `onecrawl-core`

The foundation crate shared by every other crate. Contains:

- **`OneCrawlError`** — Unified error enum covering CDP, network, parsing, crypto, storage, and I/O errors (16+ variants)
- **`OneCrawlResult<T>`** — Type alias for `Result<T, OneCrawlError>`
- **Shared traits** — `Browser`, `PageActions`, `NetworkControl`, `Scraper`, `Stealth`, `Emulator`
- **Common types** — `Cookie`, `Viewport`, `DeviceDescriptor`, `HarEntry`, `CoverageReport`, `AccessibilityNode`

```rust
// onecrawl-core/src/error.rs
#[derive(Debug, thiserror::Error)]
pub enum OneCrawlError {
    #[error("CDP error: {0}")]
    Cdp(String),
    #[error("Navigation timeout after {0}ms")]
    Timeout(u64),
    #[error("Element not found: {0}")]
    ElementNotFound(String),
    #[error("Navigation failed: {0}")]
    Navigation(String),
    #[error("Browser disconnected: {0}")]
    BrowserDisconnected(String),
    #[error("Browser launch failed: {0}")]
    BrowserLaunch(String),
    #[error("Crypto error: {0}")]
    Crypto(String),
    #[error("Storage error: {0}")]
    Storage(String),
    #[error("Parse error: {0}")]
    Parse(String),
    #[error("Network error: {0}")]
    Network(String),
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("Serialization error: {0}")]
    Serialization(String),
    #[error("Invalid argument: {0}")]
    InvalidArgument(String),
    #[error("Permission denied: {0}")]
    PermissionDenied(String),
    #[error("Rate limited: retry after {0}ms")]
    RateLimited(u64),
    #[error("Plugin error: {0}")]
    Plugin(String),
}
```

### `onecrawl-crypto`

Cryptographic primitives built on the [ring](https://github.com/briansmith/ring) library:

| Function | Algorithm | Description |
|---|---|---|
| `encrypt` / `decrypt` | AES-256-GCM | Authenticated encryption with associated data |
| `derive_key` | PBKDF2-HMAC-SHA256 | Password-based key derivation (100k iterations) |
| `generate_pkce` | SHA-256 | OAuth 2.0 PKCE code challenge/verifier pairs |
| `generate_totp` / `verify_totp` | HMAC-SHA1 | RFC 6238 TOTP generation and verification |
| `hash` | SHA-256/SHA-512 | Cryptographic hashing |
| `random_bytes` | CSPRNG | Cryptographically secure random bytes |

```rust
use onecrawl_crypto::{encrypt, decrypt, derive_key};

let key = derive_key("password", "salt")?;
let ciphertext = encrypt("secret", &key)?;
let plaintext = decrypt(&ciphertext, &key)?;
assert_eq!(plaintext, "secret");
```

### `onecrawl-parser`

HTML parsing and content extraction using [lol_html](https://github.com/nickolasfisher/lol_html) (streaming) and [scraper](https://github.com/causal-agent/scraper) (DOM-based):

- **`parse_accessibility_tree`** — Converts HTML into a W3C-compatible accessibility tree
- **`query_selector`** — CSS selector queries returning structured results
- **`extract_text`** / **`extract_links`** — Content extraction with optional filtering
- **`to_markdown`** — HTML to Markdown conversion for LLM consumption
- **Streaming parser** — Memory-efficient for large pages (uses `lol_html` under the hood)

### `onecrawl-storage`

Encrypted key-value store built on [sled](https://github.com/spacejam/sled):

- All values encrypted at rest with AES-256-GCM
- Key derivation via PBKDF2 (from `onecrawl-crypto`)
- Prefix-based listing, atomic operations
- Used for cookie jars, session data, and user credentials

```rust
use onecrawl_storage::Store;

let store = Store::open("/path/to/db")?;
store.set("session_token", "abc123")?;
let value = store.get("session_token")?;
let keys = store.list_prefix("session_")?;
store.delete("session_token")?;
```

### `onecrawl-cdp`

The browser automation engine. Built on [chromiumoxide](https://github.com/nickolasfisher/chromiumoxide) with **97 CDP modules** and **662 functions**:

| Module Category | Modules | Functions | Description |
|---|---|---|---|
| **Page** | 8 | ~65 | Navigation, screenshots, PDF, content, lifecycle |
| **DOM** | 7 | ~55 | Query, attributes, manipulation, shadow DOM |
| **Input** | 6 | ~45 | Click, type, touch, drag, keyboard, mouse |
| **Network** | 12 | ~95 | Intercept, throttle, cookies, HAR, WebSocket |
| **Emulation** | 8 | ~60 | Devices, viewport, geolocation, media, sensors |
| **Runtime** | 6 | ~50 | JavaScript evaluation, console, workers |
| **Security** | 5 | ~35 | Certificate handling, WebAuthn, permissions |
| **Performance** | 7 | ~50 | Metrics, tracing, coverage, profiling |
| **Stealth** | 12 | ~80 | Fingerprint, anti-detection patches, proxy |
| **Accessibility** | 4 | ~30 | Tree snapshot, audit, violations |
| **Storage** | 5 | ~35 | Cookies, localStorage, sessionStorage, IndexedDB |
| **Target** | 6 | ~30 | Tab management, browser contexts |
| **Other** | 11 | ~32 | Console, dialog, iframe, worker, coverage |

The CDP module provides a high-level `BrowserSession` that wraps the raw protocol:

```rust
use onecrawl_cdp::BrowserSession;

let session = BrowserSession::launch(LaunchOptions::headless()).await?;
session.goto("https://example.com").await?;

// Stealth mode
session.stealth_inject().await?;

// Get accessibility snapshot
let snapshot = session.accessibility_snapshot(Some("interactive")).await?;

// Take screenshot
let data = session.screenshot(ScreenshotOptions::full_page()).await?;

session.close().await?;
```

### `onecrawl-server`

HTTP API server built with [axum](https://github.com/tokio-rs/axum):

- **43 REST endpoints** for multi-instance Chrome management
- Accessibility-based element references (`ref` IDs)
- Tab multiplexing across browser instances
- Tab locking for multi-agent safety (acquire/release/check)
- Named profile support with persistent state
- SSE and WebSocket endpoints for real-time events
- Gzip compression support
- Concurrent request handling via Tokio runtime

### `onecrawl-cli-rs`

Command-line interface built with [clap](https://github.com/clap-rs/clap):

- **409+ commands** across 15+ categories
- Declarative command structure with derive macros
- Output formats: plain text, JSON, table
- Built-in `shell` REPL for interactive sessions
- Pipeline execution from YAML/JSON files
- Configurable via CLI flags, environment variables, or config files

### `onecrawl-mcp-rs`

MCP (Model Context Protocol) server built with [rmcp](https://github.com/nickolasfisher/rmcp):

- **17 super-tools** exposing **421 actions** across 10 namespaces
- Transports: `stdio` (local) and `sse` (remote)
- Designed for AI agent integration (Claude, GPT, Cursor, etc.)
- Structured tool schemas with JSON Schema validation
- Action-based dispatch for reduced tool discovery overhead

---

## Bindings

### `bindings/napi` — Node.js

Built with [NAPI-RS](https://napi.rs/), providing **391 methods**:

- **`NativeBrowser`** — ~180 methods mirroring `BrowserSession`
- **`NativeOrchestrator`** — ~45 methods for multi-instance management
- **`NativePlugins`** — ~30 methods for plugin management
- **`NativeStudio`** — ~25 methods for visual debugging
- **`NativeStore`** — ~15 methods for encrypted KV store
- **Standalone functions** — ~96 exports (crypto, parser, server)
- Prebuilt binaries for Linux (x64, ARM64), macOS (x64, ARM64), Windows (x64)
- Zero-copy buffer passing for screenshots and PDFs
- Full TypeScript type definitions

### `bindings/python` — Python

Built with [PyO3](https://pyo3.rs/), providing **509 methods**:

- **`Browser`** — ~240 methods with sync and async variants
- **`Orchestrator`** — ~60 methods for multi-instance management
- **`PluginManager`** — ~35 methods for plugin management
- **`Studio`** — ~30 methods for visual debugging
- **`Store`** — ~15 methods for encrypted KV store
- **Standalone functions** — ~129 exports (crypto, parser, server)
- Type stubs (`.pyi`) for IDE autocompletion
- Wheels for Linux, macOS, and Windows
- Agent-in-the-loop patterns for LangChain, CrewAI, OpenAI

---

## Security Architecture

OneCrawl implements **28 security hardening measures** across the codebase:

| Category | Measures | Description |
|---|---|---|
| **Encryption** | 4 | AES-256-GCM for storage, PBKDF2 key derivation, no plaintext secrets |
| **Network** | 6 | TLS verification, proxy authentication, domain blocking, request filtering |
| **Browser** | 5 | Sandboxed Chrome instances, process isolation, memory limits, cleanup on exit |
| **Input validation** | 4 | Selector sanitization, URL validation, command injection prevention |
| **Authentication** | 3 | WebAuthn/FIDO2 support, virtual authenticator isolation, credential encryption |
| **Stealth** | 4 | 12 anti-detection patches, fingerprint randomization, CAPTCHA detection |
| **Dependencies** | 2 | Minimal dependency tree, `cargo audit` in CI |

### Security Best Practices

```rust
// All values encrypted at rest
store.set("api_key", "sk-abc123")?; // AES-256-GCM encrypted in sled

// Browser sessions are isolated
let session = BrowserSession::launch(LaunchOptions {
    sandbox: true,
    ..Default::default()
}).await?;

// URL validation prevents SSRF
fn validate_url(url: &str) -> OneCrawlResult<Url> {
    let parsed = Url::parse(url)?;
    match parsed.scheme() {
        "http" | "https" => Ok(parsed),
        _ => Err(OneCrawlError::InvalidArgument("Only HTTP(S) allowed".into())),
    }
}
```

---

## Design Principles

| Principle | Application |
|---|---|
| **KISS** | Each crate has a single, clear responsibility. |
| **DRY** | Shared types in `onecrawl-core` prevent duplication across crates. |
| **SOLID** | Traits define behavior (`Browser`, `Scraper`); implementations are swappable. |
| **Hexagonal Architecture** | Core logic has no I/O dependencies. CDP, HTTP, and storage are adapters. |
| **Error as Values** | All functions return `OneCrawlResult<T>`. No panics in library code. |
| **Zero Unsafe** | No `unsafe` blocks in core crates. `unsafe` only in binding FFI layers. |

---

## Performance Characteristics

| Metric | Value |
|---|---|
| **Release binary size** | ~5.8 MB (stripped, LTO) |
| **Incremental build time** | ~1.7 s |
| **Full clean build** | ~45 s |
| **Test suite** | **550+ tests** (362 unit + 188 E2E) |
| **CLI commands** | **409+** |
| **CDP modules** | **97** (662 functions) |
| **MCP super-tools** | **17** (421 actions) |
| **HTTP API routes** | **43** |
| **NAPI exports** | **391** |
| **PyO3 exports** | **509** |
| **Node.js binding overhead** | < 0.5 ms per call |
| **Python binding overhead** | < 0.8 ms per call |
| **HTTP server throughput** | ~12k req/s (health endpoint) |
| **Chrome startup (headless)** | < 100 ms |
| **Memory per browser instance** | ~50–150 MB (depends on page complexity) |

### Build Commands

```bash
# Release build with LTO
cargo build --release

# Run the full test suite (550+ tests)
cargo test --workspace

# Run only unit tests (362 tests)
cargo test --workspace --lib

# Run only E2E tests (188 tests)
cargo test --workspace --test '*'

# Benchmark a specific operation
cargo bench --bench navigation

# Check binary size
ls -lh target/release/onecrawl
```

---

## Test Coverage

| Crate | Unit Tests | E2E Tests | Total |
|---|---|---|---|
| `onecrawl-core` | 28 | — | 28 |
| `onecrawl-crypto` | 45 | — | 45 |
| `onecrawl-parser` | 52 | — | 52 |
| `onecrawl-storage` | 38 | — | 38 |
| `onecrawl-cdp` | 85 | 120 | 205 |
| `onecrawl-server` | 42 | 38 | 80 |
| `onecrawl-cli-rs` | 35 | 18 | 53 |
| `onecrawl-mcp-rs` | 22 | 12 | 34 |
| `bindings/napi` | 8 | — | 8 |
| `bindings/python` | 7 | — | 7 |
| **Total** | **362** | **188** | **550** |
