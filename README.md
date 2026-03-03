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
│   ├── onecrawl-cli-rs/     # Native CLI — 200+ commands (clap)
│   └── onecrawl-mcp-rs/     # MCP server — 43 tools (rmcp, stdio + SSE)
├── bindings/
│   ├── napi/                # Node.js via NAPI-RS → @onecrawl/native
│   └── python/              # Python via PyO3 → onecrawl
└── Cargo.toml               # Workspace root
```

## Features

| Category | Highlights |
|----------|-----------|
| **Browser** | Launch, connect, stealth mode, proxy rotation, fingerprint evasion |
| **CDP** | 63 modules: DOM, Network, CSS, Performance, Accessibility, Profiler, Tracing, WebAuthn… |
| **Navigation** | goto, back, forward, reload, wait-for-navigation, screenshot, PDF |
| **Scraping** | CSS selectors, XPath, accessibility tree, readability extraction |
| **Crawling** | Spider, sitemap, link graph, robots.txt, search engines |
| **Auth** | WebAuthn/Passkey virtual authenticator, cookie management, session persistence |
| **Crypto** | AES-256-GCM encryption, PKCE, TOTP, PBKDF2 key derivation |
| **Server** | Multi-instance Chrome, profiles, tabs, accessibility snapshots, action API |
| **MCP** | 43 tools with dot-separated namespacing for AI agent orchestration |

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
const { NativeBrowser } = require('@onecrawl/native');

const browser = await NativeBrowser.launch({ headless: true, stealth: true });
await browser.goto('https://example.com');
const html = await browser.content();
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

43 tools organized by namespace:

| Namespace | Tools |
|-----------|-------|
| `navigation.*` | goto, back, forward, reload, wait_for_navigation, screenshot, pdf |
| `scraping.*` | css, xpath, accessibility_tree, readability, extract_links, extract_media |
| `crawling.*` | spider, sitemap, link_graph, robots, search_engine |
| `stealth.*` | enable, disable, rotate_fingerprint, proxy_health |
| `data.*` | cookies_get, cookies_set, cookies_clear, storage_get, storage_set |
| `automation.*` | click, type, fill, select, hover, scroll, keyboard, mouse |
| `auth.*` | passkey_enable, passkey_create, passkey_get, passkey_remove, passkey_sign_count, passkey_log |

## Development

```bash
cd packages/onecrawl-rust

# Build all crates
cargo build --workspace

# Run tests (248 tests)
cargo test --workspace

# Clippy (0 warnings)
cargo clippy --workspace

# Build release binary (5.8MB optimized)
cargo build --release -p onecrawl-cli-rs
```

## Performance

| Metric | Value |
|--------|-------|
| Release binary | 5.8 MB (LTO + strip) |
| Incremental build | ~1.7s |
| Test suite | 248 tests |
| CDP modules | 63 |
| CLI commands | 200+ |
| MCP tools | 43 |

## License

MIT
