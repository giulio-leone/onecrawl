<div align="center">

# 🦀 OneCrawl

**High-Performance Browser Automation in Rust**

*A Rust-powered browser automation and web scraping engine built on chromiumoxide,
with first-class Node.js and Python bindings.*

[![Rust](https://img.shields.io/badge/Rust-2024_Edition-000000?style=flat-square&logo=rust&logoColor=white)](https://www.rust-lang.org/)
[![Node.js](https://img.shields.io/badge/Node.js-NAPI--RS-339933?style=flat-square&logo=node.js&logoColor=white)](https://napi.rs/)
[![Python](https://img.shields.io/badge/Python-PyO3-3776AB?style=flat-square&logo=python&logoColor=white)](https://pyo3.rs/)
[![License: MIT](https://img.shields.io/badge/License-MIT-blue?style=flat-square)](LICENSE)

**59 CDP Modules** · **258 Node.js Methods** · **260 Python Methods** · **200+ CLI Commands** · **146 Rust Tests**

Linux x86_64 · macOS x86_64 · macOS ARM (Apple Silicon)

</div>

---

## What is OneCrawl?

OneCrawl is a **Rust-native browser automation and web scraping framework** built on top of [chromiumoxide](https://github.com/nickel-org/chromiumoxide) and the Chrome DevTools Protocol (CDP). It provides a unified API across three interfaces:

- **NAPI-RS** — 258 methods for Node.js, compiled to native addons
- **PyO3** — 260 methods for Python, distributed as maturin-built wheels
- **CLI** — 200+ commands for shell scripting and automation pipelines

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
- **Streaming** — Schema-based extraction with automatic pagination
- **Structured Data** — JSON-LD, OpenGraph, and Twitter Card parsing

### 📝 Form Filling
- Auto-fill forms with fuzzy field matching
- Intelligent field detection and value mapping

### 🕷️ Crawling & Discovery
- Spider framework with pause/resume support
- Link graph analysis and visualization
- Robots.txt parsing and compliance
- XML sitemap generation and parsing
- DOM snapshot capture with diff comparison

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
- Browser-session HTTP client (fetch with browser cookies/TLS)
- Request queue with configurable retry and concurrency

### 🛡️ Anti-Bot & Safety
- Sliding-window rate limiter with 4 presets
- Retry queue with exponential backoff
- Domain blocker with 5 blocklist categories
- Request queue with concurrency control

### 🥷 Stealth & Anti-Detection
- Browser fingerprint injection (WebGL, Canvas, AudioContext, fonts)
- TLS fingerprint impersonation (6 browser profiles)
- 12 stealth patches with 3 detection profiles
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

### 📊 Data Processing
- Filter, transform, deduplicate, and sort pipelines
- Element fingerprinting with adaptive relocation
- Persistent cookie jar across sessions

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
│   ├── onecrawl-cdp/         # 59 CDP modules (browser automation engine)
│   ├── onecrawl-cli-rs/      # 200+ CLI commands (clap v4)
│   ├── onecrawl-mcp-rs/      # MCP server (stdio + SSE transport)
│   ├── onecrawl-benchmark/   # Benchmark harness
│   └── onecrawl-e2e/         # End-to-end integration tests
├── bindings/
│   ├── napi/                 # Node.js bindings (258 NAPI-RS methods)
│   └── python/               # Python bindings (260 PyO3 methods)
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

All 59 CDP modules in `crates/onecrawl-cdp/src/`:

| # | Module | Category | Description |
|---|--------|----------|-------------|
| | **Core Browser Control** | | |
| 1 | `browser` | Core | Session lifecycle, launch, connect, close |
| 2 | `page` | Core | Page HTML, evaluate JS, page metrics |
| 3 | `navigation` | Navigation | Navigate, reload, back, forward, wait-for-navigation |
| 4 | `element` | Interaction | Click, type, fill, clear, focus, hover, scroll-into-view |
| 5 | `input` | Interaction | Mouse events, touch events, drag-and-drop |
| 6 | `keyboard` | Interaction | Key press, key combos, text input, IME simulation |
| 7 | `tabs` | Core | Multi-tab management, create, switch, close |
| | **Stealth & Anti-Detection** | | |
| 8 | `antibot` | Stealth | 12 stealth patches, 3 detection profiles |
| 9 | `tls_fingerprint` | Stealth | TLS fingerprint impersonation (6 browser profiles) |
| 10 | `advanced_emulation` | Emulation | Battery, orientation, permissions, CPU/memory pressure |
| | **Scraping & Extraction** | | |
| 11 | `selectors` | Selectors | CSS ::text/::attr, XPath, text, regex, find-similar |
| 12 | `dom_nav` | DOM | Parent, children, siblings, above, below traversal |
| 13 | `extract` | Extraction | Text, HTML, Markdown, JSON content extraction |
| 14 | `streaming` | Extraction | Schema-based extraction with automatic pagination |
| 15 | `structured_data` | Extraction | JSON-LD, OpenGraph, Twitter Card parsing |
| 16 | `form_filler` | Interaction | Auto-fill forms with fuzzy field matching |
| | **Crawling & Discovery** | | |
| 17 | `spider` | Crawling | Crawler framework with pause/resume support |
| 18 | `link_graph` | Crawling | Link graph analysis and visualization |
| 19 | `robots` | Crawling | Robots.txt parser and compliance checker |
| 20 | `sitemap` | Crawling | XML sitemap generation and parsing |
| 21 | `snapshot` | Crawling | DOM snapshot capture with diff comparison |
| | **Network & HTTP** | | |
| 22 | `network` | Network | Request/response inspection, headers, status codes |
| 23 | `http_client` | Network | Browser-session fetch (cookies/TLS preserved) |
| 24 | `intercept` | Network | Request interception, modification, blocking |
| 25 | `har` | Network | HAR recording, export, request/response capture |
| 26 | `websocket` | Network | WebSocket frame capture, send, close, inspect |
| 27 | `network_log` | Network | Network activity logging with filters and export |
| 28 | `throttle` | Emulation | Network throttling profiles (3G, 4G, custom) |
| 29 | `proxy` | Proxy | Pool management, rotation (RoundRobin/Random/Sticky) |
| | **Anti-Bot & Safety** | | |
| 30 | `rate_limiter` | Safety | Sliding-window rate limiter with 4 presets |
| 31 | `retry_queue` | Safety | Retry queue with exponential backoff |
| 32 | `domain_blocker` | Safety | Domain blocker with 5 blocklist categories |
| 33 | `request_queue` | Network | Request queue with retry, concurrency, backoff |
| | **Data Processing** | | |
| 34 | `data_pipeline` | Processing | Filter, transform, deduplicate, sort pipelines |
| 35 | `cookie_jar` | Storage | Encrypted cookie persistence across sessions |
| 36 | `adaptive` | Processing | Element fingerprinting with adaptive relocation |
| | **Monitoring & Testing** | | |
| 37 | `coverage` | Monitoring | JS and CSS code coverage collection |
| 38 | `benchmark` | Performance | Benchmark suite with percentile reporting |
| 39 | `screenshot_diff` | Screenshot | Visual regression with pixel diff and threshold |
| 40 | `tracing_cdp` | Performance | Chrome tracing with category filters and export |
| 41 | `page_watcher` | Monitoring | URL and content change detection polling |
| 42 | `console` | Monitoring | Console message interception and filtering |
| 43 | `dialog` | Monitoring | Alert, confirm, prompt, beforeunload handling |
| | **Infrastructure** | | |
| 44 | `events` | Core | Event subscription, lifecycle hooks, custom events |
| 45 | `downloads` | Browser | Download management, path control, wait-for-download |
| 46 | `iframe` | Iframes | List frames, eval in frame, get frame content |
| 47 | `print` | PDF | PDF generation with 14 configurable options |
| 48 | `web_storage` | Storage | localStorage, sessionStorage, IndexedDB, clear data |
| 49 | `webauthn` | Auth | WebAuthn/FIDO2 virtual authenticator and passkeys |
| 50 | `dom_observer` | DOM | MutationObserver-based DOM change tracking |
| 51 | `workers` | Workers | Service Worker register, unregister, inspect |
| 52 | `geofencing` | Emulation | Geofencing simulation and boundary events |
| 53 | `shell` | Infrastructure | Shell command execution and process management |
| | **Bindings** | | |
| 54 | `bridge` | Core | CDP bridge and raw command dispatch |
| 55 | `playwright_backend` | Core | Playwright-compatible backend adapter |
| 56 | `accessibility` | Accessibility | A11y tree extraction, WCAG audit, element properties |
| 57 | `cookie` | Storage | Get, set, delete cookies per domain |
| 58 | `emulation` | Emulation | Viewport, device presets, geolocation (8 cities) |
| 59 | `screenshot` | Screenshot | Full-page, element, viewport capture (PNG/JPEG/WebP) |

### Grand Totals

| Metric | Count |
|--------|-------|
| CDP modules | 59 |
| NAPI methods | 258 |
| PyO3 methods | 260 |
| CLI commands | 200+ |
| Rust unit tests | 146 |
| NAPI test files | 31 |
| PyO3 test files | 28 |

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
# Run all Rust tests (146 tests)
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
