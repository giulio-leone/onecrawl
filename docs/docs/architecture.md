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
│   ├── onecrawl-crypto/        # AES-256-GCM, PKCE, TOTP
│   ├── onecrawl-parser/        # HTML parsing, accessibility tree
│   ├── onecrawl-storage/       # Encrypted key-value store
│   ├── onecrawl-cdp/           # Chrome DevTools Protocol (63 modules)
│   ├── onecrawl-server/        # HTTP API server (axum)
│   ├── onecrawl-cli-rs/        # CLI (80+ commands)
│   └── onecrawl-mcp-rs/        # MCP server (51 tools)
└── bindings/
    ├── napi/                   # Node.js bindings (NAPI-RS)
    └── python/                 # Python bindings (PyO3)
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
        │(ring,AES)│ │(lol_html│ │  │  (sled)  │ │(chromium │
        │          │ │ scraper)│ │  │          │ │  oxide)  │
        └──────┬───┘ └────┬───┘ │  └─────┬────┘ └───┬──────┘
               │          │     │        │           │
               └──────────┴─────┼────────┴───────────┘
                                │
                    ┌───────────┼───────────┐
                    │           │           │
              ┌─────▼─────┐ ┌──▼──────┐ ┌──▼──────┐
              │  server   │ │ cli-rs  │ │ mcp-rs  │
              │  (axum)   │ │ (clap)  │ │ (rmcp)  │
              └───────────┘ └─────────┘ └─────────┘
                    │           │           │
                    └───────────┼───────────┘
                                │
                    ┌───────────┼───────────┐
                    │                       │
              ┌─────▼─────┐          ┌──────▼─────┐
              │bindings/  │          │ bindings/  │
              │  napi     │          │  python    │
              │(NAPI-RS)  │          │  (PyO3)    │
              └───────────┘          └────────────┘
```

---

## Core Crates

### `onecrawl-core`

The foundation crate shared by every other crate. Contains:

- **`OneCrawlError`** — Unified error enum covering CDP, network, parsing, crypto, storage, and I/O errors
- **`OneCrawlResult<T>`** — Type alias for `Result<T, OneCrawlError>`
- **Shared traits** — `Browser`, `PageActions`, `NetworkControl`, `Scraper`
- **Common types** — `Cookie`, `Viewport`, `DeviceDescriptor`, `HarEntry`, `CoverageReport`

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
    #[error("Crypto error: {0}")]
    Crypto(String),
    // ... 12 more variants
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

```rust
use onecrawl_crypto::{encrypt, decrypt, derive_key};

let key = derive_key("password", "salt")?;
let ciphertext = encrypt("secret", &key)?;
let plaintext = decrypt(&ciphertext, &key)?;
```

### `onecrawl-parser`

HTML parsing and content extraction using [lol_html](https://github.com/nickolasfisher/lol_html) (streaming) and [scraper](https://github.com/causal-agent/scraper) (DOM-based):

- **`parse_accessibility_tree`** — Converts HTML into a W3C-compatible accessibility tree
- **`query_selector`** — CSS selector queries returning structured results
- **`extract_text`** / **`extract_links`** — Content extraction with optional filtering
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
```

### `onecrawl-cdp`

The browser automation engine. Built on [chromiumoxide](https://github.com/nickolasfisher/chromiumoxide) with **63 CDP modules**:

| Module Category | Examples |
|---|---|
| Page | Navigation, screenshots, PDF, content |
| DOM | Query, attributes, manipulation |
| Input | Click, type, touch, drag |
| Network | Intercept, throttle, cookies, HAR |
| Emulation | Devices, viewport, geolocation, media |
| Runtime | JavaScript evaluation, console, workers |
| Security | Certificate handling, WebAuthn |
| Performance | Metrics, tracing, coverage |
| Stealth | Fingerprint, anti-detection patches |
| Accessibility | Tree snapshot, audit |

The CDP module provides a high-level `BrowserSession` that wraps the raw protocol:

```rust
use onecrawl_cdp::BrowserSession;

let session = BrowserSession::launch(LaunchOptions::headless()).await?;
session.goto("https://example.com").await?;
let text = session.get_text(None).await?;
session.screenshot(ScreenshotOptions::full_page()).await?;
session.close().await?;
```

### `onecrawl-server`

HTTP API server built with [axum](https://github.com/tokio-rs/axum):

- **18 REST endpoints** for multi-instance Chrome management
- Accessibility-based element references (`ref` IDs)
- Tab multiplexing across browser instances
- Named profile support with persistent state
- Concurrent request handling via Tokio runtime

### `onecrawl-cli-rs`

Command-line interface built with [clap](https://github.com/clap-rs/clap):

- **80+ top-level commands** plus dozens of subcommands
- Declarative command structure with derive macros
- Output formats: plain text, JSON, table
- Built-in `shell` REPL for interactive sessions
- Pipeline execution from YAML/JSON files

### `onecrawl-mcp-rs`

MCP (Model Context Protocol) server built with [rmcp](https://github.com/nickolasfisher/rmcp):

- **51 tools** across 9 namespaces
- Transports: `stdio` (local) and `sse` (remote)
- Designed for AI agent integration (Claude, GPT, etc.)
- Structured tool schemas with JSON Schema validation

---

## Bindings

### `bindings/napi` — Node.js

Built with [NAPI-RS](https://napi.rs/), providing ~130 methods:

- `NativeBrowser` class mirroring the Rust `BrowserSession`
- Standalone crypto, parser, server, and store functions
- Prebuilt binaries for Linux (x64, ARM64), macOS (x64, ARM64), Windows (x64)
- Zero-copy buffer passing for screenshots and PDFs

### `bindings/python` — Python

Built with [PyO3](https://pyo3.rs/), providing ~130 methods:

- `Browser` class with full async/await support via `asyncio`
- Standalone functions matching the Node.js API
- Type stubs (`.pyi`) for IDE autocompletion
- Wheels for Linux, macOS, and Windows

---

## Design Principles

| Principle | Application |
|---|---|
| **KISS** | Each crate has a single, clear responsibility. No crate exceeds ~3k LOC. |
| **DRY** | Shared types in `onecrawl-core` prevent duplication across crates. |
| **SOLID** | Traits define behavior (`Browser`, `Scraper`); implementations are swappable. |
| **Hexagonal Architecture** | Core logic has no I/O dependencies. CDP, HTTP, and storage are adapters. |
| **Error as Values** | All functions return `OneCrawlResult<T>`. No panics in library code. |
| **Zero Unsafe** | No `unsafe` blocks in core crates. `unsafe` only in binding FFI layers (NAPI-RS, PyO3). |

---

## Performance

| Metric | Value |
|---|---|
| Release binary size | **~5.8 MB** (stripped, LTO) |
| Incremental build time | **~1.7s** |
| Full clean build | **~45s** |
| Test suite | **248 tests** (unit + integration) |
| Node.js binding overhead | **< 0.5ms** per call |
| Python binding overhead | **< 0.8ms** per call |
| HTTP server throughput | **~12k req/s** (health endpoint) |
| Memory per browser instance | **~50-150 MB** (depends on page complexity) |

Build with maximum optimization:

```bash
# Release build with LTO
cargo build --release

# Run the full test suite
cargo test --workspace

# Benchmark a specific operation
cargo bench --bench navigation
```
