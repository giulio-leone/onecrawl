<div align="center">

# 🦀 OneCrawl

**High-Performance Browser Automation in Rust**

*A Rust-powered browser automation and web scraping engine built on chromiumoxide,
with first-class Node.js and Python bindings.*

[![Rust](https://img.shields.io/badge/Rust-2024_Edition-000000?style=flat-square&logo=rust&logoColor=white)](https://www.rust-lang.org/)
[![Node.js](https://img.shields.io/badge/Node.js-NAPI--RS-339933?style=flat-square&logo=node.js&logoColor=white)](https://napi.rs/)
[![Python](https://img.shields.io/badge/Python-PyO3-3776AB?style=flat-square&logo=python&logoColor=white)](https://pyo3.rs/)
[![License: MIT](https://img.shields.io/badge/License-MIT-blue?style=flat-square)](LICENSE)

**42 CDP Modules** · **144+ Node.js Methods** · **178+ Python Methods** · **152+ CLI Commands** · **116 Rust Tests**

Linux x86_64 · macOS x86_64 · macOS ARM (Apple Silicon)

</div>

---

## What is OneCrawl?

OneCrawl is a **Rust-native browser automation and web scraping framework** built on top of [chromiumoxide](https://github.com/nickel-org/chromiumoxide) and the Chrome DevTools Protocol (CDP). It provides a unified API across three interfaces:

- **NAPI-RS** — 144+ methods for Node.js, compiled to native addons
- **PyO3** — 178+ methods for Python, distributed as maturin-built wheels
- **CLI** — 152+ commands for shell scripting and automation pipelines

Everything runs in a single Rust process — no child process spawning, no bridge overhead, no separate browser driver. CDP commands are dispatched directly over WebSocket with zero-copy serialization where possible.

---

## Features

### 🖥️ Browser Core
Session management, multi-tab orchestration, and page lifecycle control. Launch headless or headed browsers with full CDP access.

### 🧭 Navigation & Interaction
Navigate, click, type, fill forms, scroll, drag-and-drop, file upload, and keyboard simulation. Full input pipeline with human-like timing support.

### 🎯 Smart Selectors *(Scrapling-inspired)*
Go beyond basic CSS and XPath with an extended selector engine:

| Selector | Description |
|---|---|
| `css 'h1::text'` | Extract text content directly |
| `css 'a::attr(href)'` | Extract attribute values |
| `xpath '//div[@class="item"]'` | Standard XPath selectors |
| `text "Buy Now"` | Find elements by visible text |
| `regex "\d{3}-\d{4}"` | Find elements matching a regex pattern |
| `find-similar '.product'` | Find DOM-similar elements automatically |
| `auto-selector` | AI-assisted selector generation |

### 🌳 DOM Navigation
Traverse the DOM tree programmatically — parent, children, siblings, elements above/below. Includes a DOM observer for mutation tracking and full DOM snapshots.

### 📄 Content Extraction
Extract content in multiple formats:
- **Text** — Clean text extraction with whitespace normalization
- **HTML** — Raw or outer HTML of any element
- **Markdown** — Structured markdown conversion
- **JSON** — Structured data extraction with CSS selectors
- **Metadata** — Page title, description, OpenGraph, JSON-LD

### 📸 Screenshot & PDF
- Full-page and element-level screenshots (PNG/JPEG/WebP)
- **Visual regression** — pixel-diff against baseline images with threshold control
- **Enhanced PDF** generation with 14 configurable options (margins, headers, footers, page ranges, scale)

### 🌐 Network
Full network layer control:
- Request interception and modification
- Response mocking for testing
- Network activity logging with filters
- HAR recording and export
- WebSocket frame capture
- Request queue with configurable retry and concurrency

### 🥷 Stealth & Anti-Detection
- Browser fingerprint injection (WebGL, Canvas, AudioContext, fonts)
- TLS fingerprint impersonation
- Stealth patches (navigator overrides, permission spoofing, plugin emulation)
- Rebrowser-compatible patch pipeline

### 🍪 Cookies & Storage
- Cookie jar with encrypted persistence (AES-256-GCM)
- `localStorage` / `sessionStorage` read/write
- IndexedDB access
- Clear all site data per origin

### 📱 Emulation
- Viewport and device emulation (iPhone, iPad, Pixel, custom)
- Geolocation spoofing with **8 city presets** (NYC, London, Tokyo, etc.)
- Network throttling (3G, 4G, custom profiles)
- Battery status emulation
- Screen orientation override
- Permission overrides
- CPU throttling and memory pressure simulation

### 🔐 Authentication
- **WebAuthn / FIDO2** passkey simulation
- Virtual authenticator device management
- Credential creation and assertion flows

### ⚡ Performance
- Chrome DevTools tracing with category filters
- **Core Web Vitals** collection (LCP, FID, CLS, TTFB, FCP)
- Built-in benchmark suite with percentile reporting (p50, p95, p99)

### ♿ Accessibility
- Full accessibility tree extraction
- **WCAG audit** with violation reporting
- Per-element accessibility properties (role, name, description)

### 📡 Monitoring
- Console message interception (log, warn, error, debug)
- JavaScript dialog handling (alert, confirm, prompt, beforeunload)
- Page watcher for URL/content change detection
- Code coverage collection (JS and CSS)

### 🔄 Proxy
- Proxy pool management
- Rotation strategies: **Round-Robin**, **Random**, **Sticky**
- Per-request proxy assignment

### ⚙️ Workers & Iframes
- Service Worker lifecycle management (register, unregister, inspect)
- Iframe enumeration, content extraction, and JavaScript evaluation within frames

---

## Quick Start

### Node.js

```js
const { NativeBrowser } = require('@onecrawl/core');

const browser = await NativeBrowser.launch({ headless: true });
await browser.navigate('https://example.com');

// Smart selectors (Scrapling-style)
const titles = await browser.cssSelect('h1::text');
const links = await browser.cssSelect('a::attr(href)');

// Extract as markdown
const content = await browser.extract(null, 'markdown');

// Visual regression testing
const diff = await browser.visualRegression('./baseline.png');

await browser.close();
```

### Python

```python
from onecrawl import Browser

browser = Browser()
browser.launch(headless=True)
browser.navigate('https://example.com')

# Smart selectors
titles = browser.css_select('h1::text')
links = browser.xpath_select('//a/@href')

# Find structurally similar elements
products = browser.css_select('.product')
similar = browser.find_similar('.product')

browser.close()
```

### CLI

```bash
# Start a browser session
onecrawl session start --headless
onecrawl navigate https://example.com

# Smart selectors
onecrawl select css 'h1::text'
onecrawl select xpath '//a/@href'
onecrawl select text "Buy Now" --tag button

# Extract content in multiple formats
onecrawl extract markdown --output page.md
onecrawl extract json --selector '.product' --output data.json

# Screenshot & visual regression
onecrawl screenshot --output page.png
onecrawl screenshot-diff regression baseline.png

# Network monitoring
onecrawl network-log start
onecrawl navigate https://api.example.com
onecrawl network-log summary

# Stealth mode
onecrawl stealth inject

# Clean up
onecrawl session close
```

---

## Architecture

```
onecrawl-rust/
├── crates/
│   ├── onecrawl-core/        # Shared types, traits, error hierarchy
│   ├── onecrawl-crypto/      # AES-256-GCM, PKCE, TOTP, PBKDF2
│   ├── onecrawl-parser/      # lol_html + scraper: streaming HTML parsing
│   ├── onecrawl-storage/     # sled-based encrypted key-value store
│   ├── onecrawl-cdp/         # 42 CDP modules (browser automation engine)
│   ├── onecrawl-cli-rs/      # 152+ CLI commands (clap v4)
│   ├── onecrawl-mcp-rs/      # MCP server (stdio + SSE transport)
│   ├── onecrawl-benchmark/   # Benchmark harness
│   └── onecrawl-e2e/         # End-to-end integration tests
├── bindings/
│   ├── napi/                 # Node.js bindings (144+ NAPI-RS methods)
│   └── python/               # Python bindings (178+ PyO3 methods)
├── scripts/                  # Build & release automation
├── Cargo.toml                # Workspace manifest
└── Makefile                  # Build orchestration
```

### Crate Dependency Graph

```
onecrawl-cli-rs ──┐
onecrawl-mcp-rs ──┤
bindings/napi ────┼──▶ onecrawl-cdp ──▶ onecrawl-parser ──▶ onecrawl-core
bindings/python ──┘         │                                      ▲
                            ├──▶ onecrawl-crypto ─────────────────┘
                            └──▶ onecrawl-storage ──▶ onecrawl-crypto
```

---

## Module Reference

All 42 CDP modules in `crates/onecrawl-cdp/src/`:

| # | Module | Functions | Category | Description |
|---|--------|-----------|----------|-------------|
| 1 | `accessibility` | 3 | Accessibility | A11y tree extraction, WCAG audit, element properties |
| 2 | `advanced_emulation` | 7 | Emulation | Battery, orientation, permissions, CPU/memory pressure |
| 3 | `benchmark` | 4 | Performance | Benchmark suite with percentile reporting |
| 4 | `bridge` | 4 | Core | CDP bridge and raw command dispatch |
| 5 | `browser` | 7 | Core | Session lifecycle, launch, connect, close |
| 6 | `console` | 3 | Monitoring | Console message interception and filtering |
| 7 | `cookie` | 5 | Storage | Get, set, delete cookies per domain |
| 8 | `cookie_jar` | 6 | Storage | Encrypted cookie persistence across sessions |
| 9 | `coverage` | 4 | Monitoring | JS and CSS code coverage collection |
| 10 | `dialog` | 3 | Monitoring | Alert, confirm, prompt, beforeunload handling |
| 11 | `dom_nav` | 8 | DOM | Parent, children, siblings, above, below traversal |
| 12 | `dom_observer` | 4 | DOM | MutationObserver-based DOM change tracking |
| 13 | `downloads` | 5 | Browser | Download management, path control, wait-for-download |
| 14 | `element` | 12 | Interaction | Click, type, fill, clear, focus, hover, scroll-into-view |
| 15 | `emulation` | 10 | Emulation | Viewport, device presets, geolocation (8 cities) |
| 16 | `events` | 10 | Core | Event subscription, lifecycle hooks, custom events |
| 17 | `extract` | 4 | Extraction | Text, HTML, Markdown, JSON content extraction |
| 18 | `geofencing` | 5 | Emulation | Geofencing simulation and boundary events |
| 19 | `har` | 8 | Network | HAR recording, export, request/response capture |
| 20 | `iframe` | 3 | Iframes | List frames, eval in frame, get frame content |
| 21 | `input` | 4 | Interaction | Mouse events, touch events, drag-and-drop |
| 22 | `intercept` | 3 | Network | Request interception, modification, blocking |
| 23 | `keyboard` | 5 | Interaction | Key press, key combos, text input, IME simulation |
| 24 | `navigation` | 9 | Navigation | Navigate, reload, back, forward, wait-for-navigation |
| 25 | `network` | 8 | Network | Request/response inspection, headers, status codes |
| 26 | `network_log` | 5 | Network | Network activity logging with filters and export |
| 27 | `page` | 3 | Core | Page HTML, evaluate JS, page metrics |
| 28 | `page_watcher` | 4 | Monitoring | URL and content change detection polling |
| 29 | `playwright_backend` | 7 | Core | Playwright-compatible backend adapter |
| 30 | `print` | 2 | PDF | PDF generation with 14 configurable options |
| 31 | `proxy` | 5 | Proxy | Pool management, rotation (RoundRobin/Random/Sticky) |
| 32 | `request_queue` | 4 | Network | Request queue with retry, concurrency, backoff |
| 33 | `screenshot` | 6 | Screenshot | Full-page, element, viewport capture (PNG/JPEG/WebP) |
| 34 | `screenshot_diff` | 3 | Screenshot | Visual regression with pixel diff and threshold |
| 35 | `selectors` | 5 | Selectors | CSS ::text/::attr, XPath, text, regex, find-similar |
| 36 | `tabs` | 5 | Core | Multi-tab management, create, switch, close |
| 37 | `throttle` | 4 | Emulation | Network throttling profiles (3G, 4G, custom) |
| 38 | `tracing_cdp` | 5 | Performance | Chrome tracing with category filters and export |
| 39 | `web_storage` | 8 | Storage | localStorage, sessionStorage, IndexedDB, clear data |
| 40 | `webauthn` | 6 | Auth | WebAuthn/FIDO2 virtual authenticator and passkeys |
| 41 | `websocket` | 9 | Network | WebSocket frame capture, send, close, inspect |
| 42 | `workers` | 3 | Workers | Service Worker register, unregister, inspect |

> **Total: 228 public functions across 42 modules**

---

## Benchmarks

OneCrawl is designed for throughput. The built-in benchmark suite measures real CDP operations:

```bash
onecrawl bench run
```

| Operation | p50 | p95 | p99 |
|---|---|---|---|
| Navigate + DOMContentLoaded | ~120ms | ~280ms | ~450ms |
| CSS Select (100 elements) | ~8ms | ~18ms | ~35ms |
| Full-page Screenshot (PNG) | ~45ms | ~95ms | ~140ms |
| Extract Markdown | ~12ms | ~28ms | ~50ms |
| Cookie Get/Set cycle | ~2ms | ~5ms | ~8ms |

> Benchmarks measured on macOS ARM (M-series), headless Chromium, localhost targets.

---

## Building

### Prerequisites

- **Rust** ≥ 1.85 (2024 edition)
- **Node.js** ≥ 18 (for NAPI bindings)
- **Python** ≥ 3.9 + [maturin](https://github.com/PyO3/maturin) (for Python bindings)
- **Chromium** or Chrome installed (for runtime)

### Build Commands

```bash
# Build everything (CLI + Node.js + Python)
make build-all

# Individual targets
make build-cli        # Rust CLI binary
make build-napi       # Node.js native addon
make build-python     # Python wheel

# Or directly with cargo
cargo build --release --package onecrawl-cli-rs

# Node.js addon
cd bindings/napi && npm run build

# Python wheel
cd bindings/python && maturin build --release

# Install CLI locally
make install
```

---

## Testing

```bash
# Run all Rust tests (116 tests)
cargo test --workspace

# Run with output
cargo test --workspace -- --nocapture

# Node.js binding tests
cd bindings/napi && npm test

# Python binding tests
cd bindings/python && pytest

# Lint & format check
make check

# Run benchmarks
make bench
```

---

## CI/CD

OneCrawl uses **GitHub Actions** for continuous integration across all supported platforms:

| Workflow | Trigger | Targets |
|---|---|---|
| `check` | Every push & PR | `cargo check`, `clippy`, `fmt` |
| `test` | Every push & PR | `cargo test --workspace` on Linux x86_64, macOS x86_64, macOS ARM |
| `build` | Release tags | CLI binary + NAPI addon + Python wheel for all 3 platforms |
| `bench` | Weekly / manual | Benchmark suite with regression tracking |

Artifacts are built for:
- **Linux** x86_64 (GNU)
- **macOS** x86_64 (Intel)
- **macOS** aarch64 (Apple Silicon)

---

## License

[MIT](LICENSE) © [Giulio Leone](https://github.com/giulio-leone)
