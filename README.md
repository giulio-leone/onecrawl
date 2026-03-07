<div align="center">

# OneCrawl

**The browser automation engine that does everything.**

Rust core · Node.js & Python SDKs · 421 MCP actions · Stealth by default

[![CI](https://github.com/giulio-leone/onecrawl/actions/workflows/rust-ci.yml/badge.svg)](https://github.com/giulio-leone/onecrawl/actions/workflows/rust-ci.yml)
[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE)
[![Crates.io](https://img.shields.io/crates/v/onecrawl-core.svg)](https://crates.io/crates/onecrawl-core)
[![npm](https://img.shields.io/npm/v/@onecrawl/native.svg)](https://www.npmjs.com/package/@onecrawl/native)
[![PyPI](https://img.shields.io/pypi/v/onecrawl.svg)](https://pypi.org/project/onecrawl/)

</div>

---

OneCrawl is a high-performance browser automation engine written in Rust. It ships as a standalone CLI, an MCP server for AI agents, a REST API, and native SDKs for Node.js and Python — all from a single codebase with zero runtime overhead.

```
409+ CLI commands · 97 CDP modules · 43 HTTP endpoints · 391 Node.js methods · 509 Python methods
```

## Why OneCrawl

| Capability | OneCrawl | Playwright | Puppeteer | Selenium |
|---|:---:|:---:|:---:|:---:|
| Language | Rust | Node / Python / .NET / Java | Node | Java / Python / JS / … |
| Stealth & anti-detection | ✅ Built-in | ❌ Plugin | ❌ Plugin | ❌ Plugin |
| MCP server for AI agents | ✅ 421 actions | ❌ | ❌ | ❌ |
| Autonomous AI agent mode | ✅ Goal-based | ❌ | ❌ | ❌ |
| WebAuthn / Passkeys | ✅ Native | ⚠️ Limited | ⚠️ Limited | ⚠️ Limited |
| Mobile (Android + iOS) | ✅ Unified | ⚠️ Android only | ❌ | ✅ Via Appium |
| Multi-device orchestration | ✅ Single workflow | ❌ | ❌ | ❌ |
| Encrypted vault | ✅ AES-256-GCM | ❌ | ❌ | ❌ |
| Durable crash-resilient sessions | ✅ Auto-checkpoint | ❌ | ❌ | ❌ |
| Event reactor (DOM / network) | ✅ AI-powered | ⚠️ Manual | ⚠️ Manual | ❌ |
| Visual workflow builder | ✅ Drag-and-drop | ❌ | ❌ | ❌ |
| Plugin system | ✅ JSON manifest | ❌ | ❌ | ❌ |
| Streaming AI vision | ✅ VLM integration | ❌ | ❌ | ❌ |
| Node.js SDK | ✅ Native FFI | ✅ | ✅ | ✅ |
| Python SDK | ✅ Native FFI | ✅ | ❌ | ✅ |

## Quick Start

### CLI

```bash
# Install from source
cd packages/onecrawl-rust
cargo install --path crates/onecrawl-cli-rs

# Launch, navigate, extract
onecrawl launch --stealth
onecrawl goto https://example.com
onecrawl css "h1" --attr textContent
onecrawl screenshot --full-page -o page.png
```

### Node.js

```bash
npm install @onecrawl/native
```

```javascript
import { NativeBrowser } from '@onecrawl/native';

const browser = await NativeBrowser.launch(true);
await browser.goto('https://example.com');
const title = await browser.getTitle();
const screenshot = await browser.screenshot();
await browser.close();
```

### Python

```bash
pip install onecrawl
```

```python
from onecrawl import Browser

browser = Browser()
browser.launch(headless=True, stealth=True)
browser.goto("https://example.com")
html = browser.content()
browser.close()
```

## Features

### Core Automation

| Category | What you get |
|---|---|
| **Browser Control** | Launch, connect, stealth-by-default, proxy rotation, fingerprint evasion, session config |
| **Navigation** | goto, back, forward, reload, wait, screenshot, PDF export, multi-tab management |
| **Interaction** | Click, type, drag & drop, hover, keyboard shortcuts, select, file upload, smart forms |
| **Scraping** | CSS selectors, XPath, accessibility tree, shadow DOM piercing, streaming extraction |
| **Crawling** | Spider, sitemap, link graph, robots.txt, DOM snapshot diff |
| **Network** | Request interception, mock responses, URL blocking, console capture, dialog handling |
| **Emulation** | Device profiles, geolocation, timezone, media features, network throttling |

### Security & Auth

| Category | What you get |
|---|---|
| **WebAuthn** | Virtual authenticator, passkey create/assert, resident keys |
| **Encrypted Vault** | AES-256-GCM credentials, PBKDF2 key derivation, service templates, workflow injection |
| **Crypto** | AES-256-GCM encryption, PKCE flows, TOTP generation, PBKDF2 |
| **Session Auth** | Cookie management, import/export, OAuth2, form auth, MFA support |

### AI & Agents

| Category | What you get |
|---|---|
| **AI Agent Auto** | Autonomous LLM-driven execution with goal-based planning, self-healing, cost tracking |
| **Computer Use** | AI-powered autonomous goal execution, smart element resolution, multi-browser fleet |
| **Agent Memory** | Store, recall, search, forget, list, export — persistent agent context |
| **Streaming Vision** | Feed screencast frames to vision-language models for continuous understanding |
| **Event Reactor** | React to DOM mutations, network events, console output with AI-powered handlers |

### Platform

| Category | What you get |
|---|---|
| **Durable Sessions** | Crash-resilient sessions with auto-checkpoint, state persistence, configurable policies |
| **Multi-Device** | Orchestrate desktop + Android + iOS from a single workflow JSON |
| **Android** | 26 actions via ADB/UIAutomator2 — tap, swipe, text, app management, recording |
| **iOS** | 19 actions via WebDriverAgent — element interaction, gestures, app lifecycle |
| **Webhooks** | Pub/sub with HMAC-signed delivery, SSE streaming, event journal with replay |
| **Plugins** | JSON manifest architecture with command/action registration and scaffolding |
| **Studio** | Visual workflow builder with drag-and-drop, templates, and JSON export |

### Quality & Performance

| Category | What you get |
|---|---|
| **Accessibility** | WCAG auditing, ARIA tree, contrast checks, heading structure, screen reader simulation |
| **Performance** | Lighthouse auditing, Core Web Vitals, performance budgets, tracing |
| **Visual Regression** | Screenshot comparison, pixel-diff, threshold configuration |
| **Human Simulation** | Bézier mouse curves, natural typing with typos, human-like scrolling |
| **Real-Time** | WebSocket connect/intercept, Server-Sent Events, GraphQL subscriptions |

## Architecture

```
onecrawl (Rust workspace)
│
├─ onecrawl-core            Shared types, traits, error handling
├─ onecrawl-browser         Browser automation engine (internalized chromiumoxide)
├─ onecrawl-protocol        CDP protocol types (internalized chromiumoxide_cdp)
├─ onecrawl-protocol-gen    Protocol code generator (internalized chromiumoxide_pdl)
├─ onecrawl-browser-types   Core browser types (internalized chromiumoxide_types)
├─ onecrawl-cdp             Chrome DevTools Protocol — 97 modules, 662 functions
├─ onecrawl-cli-rs          CLI — 409+ subcommands across 30+ groups (clap v4)
├─ onecrawl-mcp-rs          MCP server — 17 super-tools, 421 actions (rmcp)
├─ onecrawl-server          HTTP REST API — 43 endpoints (axum)
├─ onecrawl-crypto          AES-256-GCM, PKCE, TOTP, PBKDF2 (ring)
├─ onecrawl-parser          HTML parsing & accessibility tree (lol_html + scraper)
├─ onecrawl-storage         Encrypted key-value store (sled)
│
├─ bindings/napi            Node.js SDK — 391 methods via NAPI-RS
└─ bindings/python          Python SDK — 509 methods via PyO3
```

```
┌──────────────────────────────────────────────────────────┐
│                      Consumers                           │
│  CLI  ·  MCP Agents  ·  REST API  ·  Node.js  ·  Python │
└────────────────────────┬─────────────────────────────────┘
                         │
              ┌──────────▼──────────┐
              │   onecrawl-core     │
              │   (types + traits)  │
              └──────────┬──────────┘
                         │
         ┌───────────────┼───────────────┐
         │               │               │
   ┌─────▼─────┐  ┌─────▼─────┐  ┌─────▼──────┐
   │  cdp (97)  │  │  crypto   │  │  storage   │
   │  modules   │  │  (ring)   │  │  (sled)    │
   └─────┬─────┘  └───────────┘  └────────────┘
         │
   ┌─────▼──────────────────────────────────┐
   │         onecrawl-browser               │
   │  (first-party browser automation)      │
   │                                        │
   │  ┌──────────────┐ ┌─────────────────┐  │
   │  │  protocol    │ │  browser-types  │  │
   │  │  (CDP types) │ │  (core types)   │  │
   │  └──────────────┘ └─────────────────┘  │
   └─────┬──────────────────────────────────┘
         │
    ┌────▼────┐
    │ Chrome  │
    │ (CDP)   │
    └─────────┘
```

## MCP Integration

17 super-tools with 421 actions, designed for AI agent orchestration via action-based dispatch:

```json
{ "action": "goto", "params": { "url": "https://example.com" } }
```

| Super-Tool | Actions | Scope |
|---|---:|---|
| **browser** | 112 | Navigation, interaction, extraction, multi-tab, DOM events, network interception, emulation, shadow DOM, service workers, smart forms, self-healing selectors |
| **agent** | 111 | Stealth, fingerprint, anti-bot, proxy health, CAPTCHA, task decomposition, vision observation, WCAG auditing, accessibility tree, screen reader simulation |
| **data** | 27 | Cookies, storage, structured data extraction, entity extraction, feeds, WebSocket, SSE, GraphQL subscriptions |
| **automate** | 27 | Workflow DSL: run, validate, list, templates, error recovery, session checkpoints, control flow |
| **stealth** | 25 | Enable/disable stealth, rotate fingerprint, proxy health, CAPTCHA solving, human behavior simulation |
| **computer** | 24 | AI computer-use, autonomous goal execution, smart element resolution, multi-browser fleet |
| **secure** | 21 | WebAuthn/Passkey, vault, OAuth2, session/form auth, MFA, credentials |
| **vault** | 9 | Encrypted credential management, service templates, workflow variable injection |
| **plugins** | 9 | Plugin lifecycle, manifest validation, command/action registration, scaffolding |
| **reactor** | 8 | DOM mutation observers, network event handlers, console watchers, AI-powered reactions |
| **durable** | 8 | Crash-resilient sessions, auto-checkpoint, state persistence, recovery policies |
| **events** | 8 | Pub/sub, HMAC-signed webhooks, SSE streaming, event journal with replay |
| **studio** | 8 | Visual workflow builder, template library, project management, JSON export |
| **perf** | 8 | Lighthouse audit, Core Web Vitals, performance budgets, tracing, VRT comparison |
| **memory** | 6 | Agent memory: store, recall, search, forget, list, export |
| **crawl** | 5 | Spider, robots.txt, sitemap parsing, DOM snapshot/diff |
| **orchestrator** | 5 | Multi-device coordination, parallel execution, cross-platform workflow dispatch |

> Full API reference: [`docs/MCP_API_REFERENCE.md`](docs/MCP_API_REFERENCE.md)

## SDKs

### Node.js — `@onecrawl/native`

**391 methods** via NAPI-RS. Direct FFI — no child process, no serialization overhead.

```javascript
import { NativeBrowser, NativeStore, Crypto } from '@onecrawl/native';

// Browser automation
const browser = await NativeBrowser.launch(true);
await browser.goto('https://example.com');
await browser.click('#login');
await browser.type('#email', 'user@example.com');
const snapshot = await browser.accessibilitySnapshot();
await browser.close();

// Encrypted storage
const store = new NativeStore('/tmp/data');
store.set('key', 'encrypted-value');

// Crypto utilities
const { verifier, challenge } = Crypto.pkceChallenge();
const totp = Crypto.totpGenerate('base32secret');
```

TypeScript types included. Async/await throughout. 8-platform cross-compilation.

### Python — `onecrawl`

**509 methods** via PyO3. Full MCP action parity with native performance.

```python
from onecrawl import Browser

browser = Browser()
browser.launch(headless=True, stealth=True)
browser.goto("https://example.com")

# Scraping
title = browser.get_title()
links = browser.css_all("a[href]", "href")

# Agent-in-the-loop
browser.workflow_pause(reason="Review before proceeding")
browser.workflow_resume()

# Mobile automation
browser.android_tap(500, 800)
browser.android_swipe(500, 1500, 500, 500)

browser.close()
```

Full MCP action coverage. Android/iOS automation. Async support.

## Server API

Start with `onecrawl serve --port 9867` — 43 endpoints for multi-instance Chrome management.

```bash
# Create instance → navigate → extract
curl -X POST http://localhost:9867/instances \
  -H 'Content-Type: application/json' \
  -d '{"profile": "default"}'

curl -X POST http://localhost:9867/instances/{id}/tabs \
  -d '{"url": "https://example.com"}'

curl http://localhost:9867/instances/{id}/tabs/{tab}/text
```

<details>
<summary><strong>All endpoints</strong></summary>

| Method | Endpoint | Description |
|---|---|---|
| `POST` | `/instances` | Create Chrome instance |
| `GET` | `/instances` | List all instances |
| `DELETE` | `/instances/:id` | Stop instance |
| `POST` | `/instances/:id/tabs` | Open new tab |
| `GET` | `/instances/:id/tabs` | List tabs |
| `DELETE` | `/instances/:id/tabs/:tab` | Close tab |
| `POST` | `/instances/:id/tabs/:tab/navigate` | Navigate to URL |
| `GET` | `/instances/:id/tabs/:tab/snapshot` | Accessibility snapshot (stable refs) |
| `POST` | `/instances/:id/tabs/:tab/action` | Execute action by element ref |
| `GET` | `/instances/:id/tabs/:tab/text` | Token-efficient text (~800 tokens/page) |
| `POST` | `/profiles` | Create browser profile |
| `GET` | `/profiles` | List profiles |
| `DELETE` | `/profiles/:name` | Delete profile |
| `GET` | `/health` | Health check |

</details>

## Development

```bash
cd packages/onecrawl-rust

# Build the entire workspace
cargo build --workspace

# Run unit tests (362+)
cargo test --workspace --exclude onecrawl-e2e

# Run E2E tests (188)
cargo test -p onecrawl-e2e

# Build optimized release binary
cargo build --release -p onecrawl-cli-rs

# Build Node.js bindings
cd bindings/napi && npm run build

# Build Python bindings
cd bindings/python && maturin develop
```

## Metrics

| Metric | Count |
|---|---:|
| MCP actions | **421** across 17 super-tools |
| CLI subcommands | **409+** across 30+ groups |
| CDP modules | **97** public modules, **662** public functions |
| Node.js methods | **391** (NAPI-RS) |
| Python methods | **509** (PyO3) |
| HTTP endpoints | **43** |
| Unit tests | **362+** |
| E2E tests | **188** |
| Security fixes (v3.9.2) | **28** |
| Android actions | **26** (ADB / UIAutomator2) |
| iOS actions | **19** (WebDriverAgent) |

## License

[MIT](LICENSE) — OneCrawl is free and open-source.
