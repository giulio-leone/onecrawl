# OneCrawl

High-performance browser automation engine written in Rust. Native bindings for Node.js and Python.

## Architecture

```
packages/onecrawl-rust/
├── crates/
│   ├── onecrawl-core/       # Shared types, traits, error handling
│   ├── onecrawl-crypto/     # AES-256-GCM, PKCE, TOTP, PBKDF2 (ring)
│   ├── onecrawl-parser/     # HTML parsing & accessibility tree (lol_html + scraper)
│   ├── onecrawl-storage/    # Encrypted key-value store (sled)
│   ├── onecrawl-cdp/        # Chrome DevTools Protocol — 63 modules (chromiumoxide)
│   ├── onecrawl-server/     # HTTP REST API with multi-instance management (axum)
│   ├── onecrawl-cli-rs/     # Native CLI — ~143 commands (clap v4)
│   └── onecrawl-mcp-rs/     # MCP server — 10 super-tools, ~355 actions (rmcp)
├── bindings/
│   ├── napi/                # Node.js via NAPI-RS → @onecrawl/native (614+ exports)
│   └── python/              # Python via PyO3 → onecrawl (full MCP parity)
└── Cargo.toml               # Workspace root
```

## Features

| Category | Highlights |
|----------|-----------|
| **Browser** | Launch, connect, stealth-by-default, proxy rotation, fingerprint evasion, session config |
| **CDP** | 63 modules: DOM, Network, CSS, Performance, Accessibility, Profiler, Tracing, WebAuthn… |
| **Navigation** | goto, back, forward, reload, wait, screenshot, PDF, multi-tab |
| **Interaction** | click, type, drag & drop, hover, keyboard shortcuts, select, file upload |
| **Scraping** | CSS selectors, XPath, accessibility tree, shadow DOM piercing, streaming extraction |
| **Crawling** | Spider, sitemap, link graph, robots.txt, DOM snapshot diff |
| **Network** | Request interception, mock responses, URL blocking, console capture, dialog handling |
| **Emulation** | Device emulation, geolocation, timezone, media features, network throttling |
| **Auth** | WebAuthn/Passkey virtual authenticator, cookie/session management, import/export |
| **Crypto** | AES-256-GCM encryption, PKCE, TOTP, PBKDF2 key derivation |
| **AI Agent** | Agent memory, workflow DSL, task planner, autonomous computer_use, visual regression testing, performance monitor, agent-in-the-loop pause/resume |
| **Accessibility** | WCAG compliance auditing, ARIA tree, contrast checks, heading structure, keyboard traps, screen reader simulation |
| **Real-Time** | WebSocket connect/intercept/send, Server-Sent Events, GraphQL subscriptions |
| **Human Simulation** | Bézier mouse curves, natural typing with typos, human-like scrolling, behavior profiles |
| **Service Workers** | SW register/unregister/update, Cache Storage management, push simulation, offline mode |
| **Server** | Multi-instance Chrome, profiles, tabs, accessibility snapshots, action API |
| **MCP** | 10 super-tools with ~355 actions for AI agent orchestration |
| **Mobile** | Android automation (26 actions via ADB/UIAutomator2), iOS automation (19 actions via WebDriverAgent) |

## Installation

### CLI (from source)

```bash
cd packages/onecrawl-rust
cargo install --path crates/onecrawl-cli-rs
```

### Node.js

```bash
npm install @onecrawl/native
```

```javascript
import { NativeBrowser } from '@onecrawl/native';

const browser = await NativeBrowser.launch(true); // headless
await browser.goto('https://example.com');
const title = await browser.getTitle();
const screenshot = await browser.screenshot(); // Buffer
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

## CLI Usage

```bash
# Launch browser and navigate
onecrawl launch --stealth
onecrawl goto https://example.com

# Scraping
onecrawl css "h1" --attr textContent
onecrawl xpath "//a[@href]"
onecrawl readability https://example.com

# Crawling
onecrawl spider https://example.com --depth 3
onecrawl sitemap https://example.com/sitemap.xml

# Screenshots & PDF
onecrawl screenshot --full-page -o page.png
onecrawl pdf -o page.pdf

# Authentication
onecrawl auth passkey-enable
onecrawl auth passkey-create --rp-id example.com --user-name admin

# HTTP Server (multi-instance)
onecrawl serve --port 9867
```

## HTTP Server API

Start the server with `onecrawl serve` and manage browser instances via REST:

```bash
# Create a Chrome instance
curl -X POST http://localhost:9867/instances \
  -H 'Content-Type: application/json' \
  -d '{"profile": "default"}'

# Open a tab and navigate
curl -X POST http://localhost:9867/instances/{id}/tabs \
  -d '{"url": "https://example.com"}'

# Get accessibility snapshot (stable element refs)
curl http://localhost:9867/instances/{id}/tabs/{tab}/snapshot

# Execute action by element ref
curl -X POST http://localhost:9867/instances/{id}/tabs/{tab}/action \
  -d '{"ref": "e5", "action": "click"}'

# Get token-efficient text (~800 tokens/page)
curl http://localhost:9867/instances/{id}/tabs/{tab}/text
```

### Endpoints

| Method | Path | Description |
|--------|------|-------------|
| POST | `/instances` | Create Chrome instance |
| GET | `/instances` | List instances |
| DELETE | `/instances/:id` | Stop instance |
| POST | `/instances/:id/tabs` | Open tab |
| GET | `/instances/:id/tabs` | List tabs |
| DELETE | `/instances/:id/tabs/:tab` | Close tab |
| POST | `/instances/:id/tabs/:tab/navigate` | Navigate tab |
| GET | `/instances/:id/tabs/:tab/snapshot` | Accessibility snapshot |
| POST | `/instances/:id/tabs/:tab/action` | Execute action by ref |
| GET | `/instances/:id/tabs/:tab/text` | Token-efficient text |
| POST | `/profiles` | Create profile |
| GET | `/profiles` | List profiles |
| DELETE | `/profiles/:name` | Delete profile |
| GET | `/health` | Health check |

## MCP Integration

10 super-tools with ~355 total actions, using action-based dispatch:

```json
{"action": "goto", "params": {"url": "https://example.com"}}
```

| Super-Tool | Actions | Highlights |
|------------|---------|------------|
| **browser** | 95 | Navigation, interaction, extraction, multi-tab, DOM events, session, network interception, console/dialog, device emulation, drag/drop, file upload, shadow DOM, session context, smart forms, self-healing selectors, event reactions, service worker/PWA, offline mode, session config |
| **crawl** | 5 | Spider, robots.txt, sitemap, DOM snapshot/diff |
| **agent** | 40 | Stealth, fingerprint, anti-bot detection, proxy health, CAPTCHA, CDP cross-origin iframe interaction, task decomposition, vision observation, WCAG auditing, accessibility tree, screen reader simulation |
| **stealth** | 13 | Enable/disable stealth, rotate fingerprint, proxy health, CAPTCHA solving, human behavior simulation |
| **data** | 26 | Cookies, storage, structured data extraction, entity extraction, feeds, WebSocket, SSE, GraphQL subscriptions |
| **secure** | 21 | WebAuthn/Passkey, vault, OAuth2, session/form auth, MFA, credentials |
| **computer** | 18 | AI computer-use, autonomous goal execution, smart element resolution, multi-browser fleet |
| **memory** | 6 | Agent memory: store, recall, search, forget, list, export |
| **automate** | 19 | Workflow DSL: run, validate, list, templates, error recovery, session checkpoints, workflow control flow |
| **perf** | 7 | Performance: audit, metrics, budget, trace, VRT comparison |

> Full API reference: [`docs/MCP_API_REFERENCE.md`](docs/MCP_API_REFERENCE.md)

## Node.js Bindings

`@onecrawl/native` exposes **614+ exports** via NAPI-RS (direct FFI, no child process overhead, full MCP parity):

| Class/Module | Exports | Description |
|-------------|---------|-------------|
| **NativeBrowser** | 580+ | Full browser control: navigation, interaction, scraping, crawling, emulation, auth, network, performance, mobile, agent workflows |
| **NativeStore** | 7 | Encrypted key-value store (sled) |
| **Crypto** | 6 | AES-256-GCM, PKCE, TOTP, PBKDF2 |
| **Parser** | 4 | A11y tree, CSS selector, text/link extraction |
| **Android** | 26 | ADB/UIAutomator2: tap, swipe, scroll, text, app management, screenshot, recording |

Features: TypeScript types (`index.d.ts`), async/await, Buffer support, 33 test files (3,995 lines), 8-platform cross-compilation.

## Python Bindings

`onecrawl` Python package via PyO3 with full MCP action parity:

```python
from onecrawl import Browser

browser = Browser()
browser.launch(headless=True, stealth=True)
browser.goto("https://example.com")

# Agent-in-the-loop: pause workflow for human/AI decision
browser.workflow_pause(reason="Review page before proceeding")
# ... agent or human resumes ...
browser.workflow_resume()

html = browser.content()
browser.close()
```

Features: full MCP action coverage, Android/iOS automation, agent-in-the-loop support, async support.

## Development

```bash
cd packages/onecrawl-rust

# Build all crates
cargo build --workspace

# Run tests (427 tests)
cargo test --workspace --exclude onecrawl-e2e

# Build release binary
cargo build --release -p onecrawl-cli-rs
```

## Metrics

| Metric | Value |
|--------|-------|
| Rust test suite | 427 tests |
| Node.js test suite | 33 files, 3,995 lines |
| CDP modules | 63 |
| CLI commands | ~143 |
| MCP super-tools | 10 (~355 actions) |
| NAPI exports | 614+ (full MCP parity) |
| PyO3 bindings | Full MCP parity |
| Android actions | 26 (ADB/UIAutomator2) |
| iOS actions | 19 (WebDriverAgent) |
| Handler modules | 10 (split architecture) |
| Enum-dispatched actions | ~355 (compile-time exhaustive) |
| Security fixes | 54 issues across 14 review cycles |

## License

MIT
