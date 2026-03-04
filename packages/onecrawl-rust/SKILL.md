# OneCrawl SKILL.md -- AI-Native Browser Automation

OneCrawl is a high-performance Rust monorepo for browser automation, web scraping,
and AI agent tooling. It exposes 180+ CLI commands, 21 HTTP API endpoints, and
43 MCP tools across 8 crates.

---

## Architecture

```
onecrawl-rust/
  crates/
    onecrawl-core/       Shared types, traits, errors
    onecrawl-crypto/     AES-256-GCM, PKCE, TOTP, PBKDF2 (ring)
    onecrawl-parser/     HTML parsing, a11y tree, extraction (lol_html + scraper)
    onecrawl-storage/    Encrypted KV store (sled)
    onecrawl-cdp/        63 CDP modules: stealth, captcha, spider, rate limiter...
    onecrawl-server/     axum HTTP API with multi-instance Chrome management
    onecrawl-cli-rs/     180+ CLI commands via clap
    onecrawl-mcp-rs/     43 MCP tools (stdio + SSE)
  bindings/
    napi/                NAPI-RS -> npm @onecrawl/native
    python/              PyO3 -> pip onecrawl
```

---

## Quick Start

```bash
# Start a headless browser session
onecrawl session start --headless

# Navigate, extract, interact
onecrawl navigate https://example.com
onecrawl get text
onecrawl get title
onecrawl get url
onecrawl eval "document.querySelectorAll('a').length"
onecrawl screenshot --output page.png

# Close session
onecrawl session close
```

---

## CLI Commands (180+)

### Session Management

| Command | Description |
|---------|-------------|
| `session start [--headless]` | Launch browser (headed or headless daemon) |
| `session close` | Stop browser and daemon |
| `session status` | Show current session state |
| `serve --port 9867` | Start HTTP API server |
| `mcp --transport stdio` | Start MCP server |
| `health` | Health check |
| `info` | Version and system info |

### Navigation

| Command | Description |
|---------|-------------|
| `navigate <url> [--wait <ms>]` | Navigate to URL |
| `back` | Go back in history |
| `forward` | Go forward in history |
| `reload` | Reload current page |

### Content Extraction

| Command | Description |
|---------|-------------|
| `get text [--selector <css>]` | Extract visible text |
| `get html [--selector <css>]` | Get raw HTML |
| `get url` | Get current URL |
| `get title` | Get page title |
| `eval <expression>` | Evaluate JavaScript |
| `set-content <html>` | Set page HTML |
| `extract content --format text\|html\|markdown\|json` | Extract content |
| `extract metadata` | Get structured metadata |

### Element Interaction

| Command | Description |
|---------|-------------|
| `click <selector>` | Click element |
| `dblclick <selector>` | Double-click |
| `type <selector> <text>` | Type text key-by-key |
| `fill <selector> <text>` | Fill input (clear + set) |
| `focus <selector>` | Focus element |
| `hover <selector>` | Hover over element |
| `scroll-into-view <selector>` | Scroll element into view |
| `check <selector>` | Check checkbox |
| `uncheck <selector>` | Uncheck checkbox |
| `select-option <selector> <value>` | Select dropdown option |
| `tap <selector>` | Tap (touch) |
| `drag <from> <to>` | Drag and drop |
| `upload <selector> <file>` | Upload file |
| `bounding-box <selector>` | Get bounding box (JSON) |

### Keyboard

| Command | Description |
|---------|-------------|
| `press-key <key>` | Press key (Enter, Tab, Escape...) |
| `key-down <key>` | Hold key down |
| `key-up <key>` | Release key |
| `keyboard-shortcut <keys>` | Send shortcut (Control+a, Meta+c) |

### Screenshot and PDF

| Command | Description |
|---------|-------------|
| `screenshot --output <path> [--full] [--element <css>] [--format png\|jpeg\|webp]` | Capture screenshot |
| `pdf --output <path> [--landscape] [--scale <n>]` | Export PDF |
| `print pdf --output <path> [detailed options]` | Generate PDF with full options |
| `screenshot-diff compare <baseline> <current>` | Compare screenshots |
| `screenshot-diff regression <baseline>` | Visual regression test |

### Accessibility

| Command | Description |
|---------|-------------|
| `a11y tree` | Full accessibility tree |
| `a11y element <selector>` | Element a11y info |
| `a11y audit` | Run accessibility audit |

### Smart Selectors

| Command | Description |
|---------|-------------|
| `select css <selector>` | CSS selector (supports ::text, ::attr) |
| `select xpath <expression>` | XPath selector |
| `select text <text> [--tag]` | Find by text content |
| `select regex <pattern> [--tag]` | Find by regex |
| `select auto-selector <selector>` | Auto-generate unique selector |

### DOM Navigation

| Command | Description |
|---------|-------------|
| `nav parent <sel>` | Parent element |
| `nav children <sel>` | Child elements |
| `nav next-sibling <sel>` | Next sibling |
| `nav prev-sibling <sel>` | Previous sibling |
| `nav siblings <sel>` | All siblings |
| `nav similar <sel>` | Similar elements |
| `nav above <sel>` | Elements above |
| `nav below <sel>` | Elements below |

### Cookies

| Command | Description |
|---------|-------------|
| `cookie get [--name] [--json]` | Get cookies |
| `cookie set <name> <value> [--domain]` | Set cookie |
| `cookie delete <name> <domain>` | Delete cookie |
| `cookie clear` | Clear all cookies |
| `cookie-jar export [--output]` | Export cookies |
| `cookie-jar import <path>` | Import cookies |

### Web Storage

| Command | Description |
|---------|-------------|
| `web-storage local-get` | Get localStorage |
| `web-storage local-set <key> <value>` | Set localStorage item |
| `web-storage local-clear` | Clear localStorage |
| `web-storage session-get` | Get sessionStorage |
| `web-storage session-set <key> <value>` | Set sessionStorage item |
| `web-storage session-clear` | Clear sessionStorage |
| `web-storage indexeddb-list` | List IndexedDB databases |
| `web-storage clear-all` | Clear all site data |

### Emulation

| Command | Description |
|---------|-------------|
| `emulate viewport <width> <height>` | Set viewport |
| `emulate device <name>` | Device preset (iphone_14, ipad, pixel_7...) |
| `emulate user-agent <ua>` | Override user agent |
| `emulate geolocation <lat> <lon>` | Set geolocation |
| `emulate color-scheme <dark\|light>` | Color scheme |
| `emulate clear` | Clear overrides |
| `advanced-emulation orientation <a> <b> <g>` | Device orientation |
| `advanced-emulation permission <name> <state>` | Override permission |
| `advanced-emulation battery <level>` | Battery status |
| `advanced-emulation cpu-cores <n>` | CPU core count |
| `advanced-emulation memory <gb>` | Device memory |

### Network

| Command | Description |
|---------|-------------|
| `network block <types>` | Block resource types |
| `throttle set <profile>` | Named throttle (fast3g, slow3g, wifi...) |
| `throttle custom <dl> <ul> <lat>` | Custom throttle |
| `throttle clear` | Clear throttle |
| `har start\|drain\|export` | HAR recording |
| `ws start\|drain\|export\|connections` | WebSocket capture |
| `network-log start\|drain\|summary\|stop\|export` | Network logging |
| `intercept set <rules_json>` | Request interception |
| `intercept log\|clear` | Interception management |

### Stealth and Anti-Bot

| Command | Description |
|---------|-------------|
| `stealth inject` | Inject stealth patches |
| `antibot inject [--level aggressive]` | Full anti-bot stealth |
| `antibot test` | Run bot detection test |
| `antibot profiles` | List stealth profiles |
| `fingerprint apply <name>` | Apply fingerprint profile |
| `fingerprint detect` | Detect current fingerprint |
| `fingerprint list` | List fingerprint profiles |

### Authentication and Passkeys

| Command | Description |
|---------|-------------|
| `auth passkey-enable [--protocol ctap2]` | Enable virtual authenticator |
| `auth passkey-add [--credential_id] [--rp_id]` | Add credential |
| `auth passkey-list` | List credentials |
| `auth passkey-log` | Operation log |
| `auth passkey-disable` | Disable authenticator |
| `auth passkey-remove --credential_id <id>` | Remove credential |

### Spider and Crawler

| Command | Description |
|---------|-------------|
| `spider crawl <url> [--max_depth 3] [--max_pages 100]` | Crawl website |
| `spider resume <state_file>` | Resume crawl |
| `spider summary <results_file>` | Print summary |
| `robots parse <url>` | Parse robots.txt |
| `robots check <url> <path>` | Check if path is allowed |
| `graph extract [--base_url]` | Extract page links |
| `graph build <edges>` | Build link graph |
| `graph analyze <graph>` | Analyze graph |

### Structured Data

| Command | Description |
|---------|-------------|
| `structured extract-all` | All structured data |
| `structured json-ld` | JSON-LD |
| `structured open-graph` | OpenGraph |
| `structured twitter-card` | Twitter Card |
| `structured metadata` | Page metadata |
| `structured validate <json>` | Validate data |

### Streaming Extraction

```bash
onecrawl stream-extract ".product" \
  --field "name=.title::text" \
  --field "price=.price::text" \
  --paginate ".next-page" \
  --max_pages 10 \
  --format csv \
  --output products.csv
```

### Data Pipeline

```bash
onecrawl pipeline run '<pipeline_json>' '<data_json>' \
  --format csv --output result.csv
onecrawl pipeline validate '<pipeline_json>'
```

### Rate Limiting and Retry

| Command | Description |
|---------|-------------|
| `ratelimit set --preset conservative` | Set rate limiter |
| `ratelimit stats` | Show statistics |
| `ratelimit reset` | Reset counters |
| `retry enqueue <url> <operation>` | Enqueue for retry |
| `retry next\|success\|fail\|stats\|clear` | Manage retry queue |

### Task Scheduling

| Command | Description |
|---------|-------------|
| `schedule add <name> -t <type> --interval <ms>` | Add task |
| `schedule remove\|pause\|resume <id>` | Manage tasks |
| `schedule list\|stats` | View tasks |

### Session Pool

| Command | Description |
|---------|-------------|
| `pool add <name> [--tags]` | Add session |
| `pool next\|stats\|cleanup` | Manage pool |

### Performance and Coverage

| Command | Description |
|---------|-------------|
| `perf trace-start\|trace-stop\|metrics\|timing\|resources` | Performance tracing |
| `coverage js-start\|js-stop\|css-start\|css-report` | Code coverage |
| `console start\|drain\|clear` | Console capture |

### CAPTCHA

| Command | Description |
|---------|-------------|
| `captcha detect` | Detect CAPTCHA |
| `captcha wait [--timeout 30000]` | Wait for CAPTCHA |
| `captcha screenshot` | Screenshot CAPTCHA |
| `captcha inject <solution>` | Inject solution token |

### Adaptive Element Tracking

| Command | Description |
|---------|-------------|
| `adaptive fingerprint <selector>` | Fingerprint DOM element |
| `adaptive relocate <fingerprint_json>` | Relocate element |
| `adaptive track <selectors_json>` | Track multiple elements |

### Offline Operations

| Command | Description |
|---------|-------------|
| `crypto encrypt\|decrypt\|pkce\|totp` | Cryptographic operations |
| `parse a11y\|selector\|text\|links\|metadata` | HTML parsing |
| `storage set\|get\|list\|delete` | Encrypted KV storage |

### Interactive

| Command | Description |
|---------|-------------|
| `shell` | Launch interactive REPL |
| `bench run [--iterations 20]` | CDP benchmark |

---

## HTTP API (21 endpoints)

Default: `http://localhost:9867`

### Instance Management

| Method | Endpoint | Description |
|--------|----------|-------------|
| `POST` | `/instances` | Launch a Chrome instance |
| `GET` | `/instances` | List all instances |
| `GET` | `/instances/{id}` | Get instance info |
| `DELETE` | `/instances/{id}` | Stop instance |

### Tab Management

| Method | Endpoint | Description |
|--------|----------|-------------|
| `POST` | `/instances/{id}/tabs/open` | Open new tab |
| `GET` | `/instances/{id}/tabs` | List instance tabs |
| `GET` | `/tabs` | List all tabs |

### Tab Operations

| Method | Endpoint | Description |
|--------|----------|-------------|
| `POST` | `/tabs/{id}/navigate` | Navigate tab to URL |
| `GET` | `/tabs/{id}/snapshot` | Accessibility snapshot |
| `GET` | `/tabs/{id}/text` | Extract text |
| `GET` | `/tabs/{id}/url` | Get current URL |
| `GET` | `/tabs/{id}/title` | Get page title |
| `GET` | `/tabs/{id}/html` | Get full HTML |
| `POST` | `/tabs/{id}/action` | Execute single action |
| `POST` | `/tabs/{id}/actions` | Execute action batch |
| `POST` | `/tabs/{id}/evaluate` | Evaluate JavaScript |
| `GET` | `/tabs/{id}/screenshot` | Take screenshot (base64) |
| `GET` | `/tabs/{id}/pdf` | Export PDF (base64) |

### Profiles

| Method | Endpoint | Description |
|--------|----------|-------------|
| `GET` | `/profiles` | List browser profiles |
| `POST` | `/profiles` | Create new profile |

### Utility

| Method | Endpoint | Description |
|--------|----------|-------------|
| `GET` | `/health` | Health check |

### Action API

Actions are sent via `POST /tabs/{id}/action`:

```json
{ "Click": { "ref_id": "e5" } }
{ "Type": { "ref_id": "e12", "text": "hello" } }
{ "Fill": { "ref_id": "e12", "text": "value" } }
{ "Press": { "key": "Enter" } }
{ "Hover": { "ref_id": "e3" } }
{ "Focus": { "ref_id": "e3" } }
{ "Scroll": { "ref_id": "e3" } }
{ "Select": { "ref_id": "e7", "value": "option1" } }
{ "Wait": { "ms": 1000 } }
{ "Batch": { "actions": [...] } }
```

Element refs (`e0`, `e1`, ...) come from accessibility snapshots.

### Snapshot Query Parameters

```
GET /tabs/{id}/snapshot?filter=interactive    # buttons, links, inputs only
GET /tabs/{id}/snapshot?compact=true          # minimal output
```

---

## MCP Tools (43)

### Crypto (4 tools)

| Tool | Description |
|------|-------------|
| `encrypt` | AES-256-GCM encryption |
| `decrypt` | AES-256-GCM decryption |
| `generate_pkce` | PKCE S256 challenge pair |
| `generate_totp` | 6-digit TOTP code |

### Parser (4 tools)

| Tool | Description |
|------|-------------|
| `parse_accessibility_tree` | HTML to a11y tree |
| `query_selector` | CSS selector query |
| `html_extract_text` | Extract visible text |
| `html_extract_links` | Extract links with metadata |

### Storage (3 tools)

| Tool | Description |
|------|-------------|
| `store_set` | Set encrypted KV pair |
| `store_get` | Get value by key |
| `store_list_keys` | List all keys |

### Navigation (11 tools)

| Tool | Description |
|------|-------------|
| `navigation.goto` | Navigate to URL |
| `navigation.click` | Click by selector |
| `navigation.type` | Type text into input |
| `navigation.screenshot` | Take screenshot |
| `navigation.pdf` | Export PDF |
| `navigation.back` | Go back |
| `navigation.forward` | Go forward |
| `navigation.reload` | Reload page |
| `navigation.wait` | Wait for selector |
| `navigation.evaluate` | Evaluate JavaScript |
| `navigation.cookies` | Get/set cookies |

### Scraping (9 tools)

| Tool | Description |
|------|-------------|
| `scraping.css` | CSS selector on live DOM |
| `scraping.xpath` | XPath query |
| `scraping.find_text` | Find by text content |
| `scraping.text` | Extract text from live page |
| `scraping.html` | Extract HTML |
| `scraping.markdown` | Extract as Markdown |
| `scraping.structured_data` | Extract JSON-LD, OG, etc. |
| `scraping.detect_forms` | Detect forms and fields |
| `scraping.fill_form` | Fill and submit forms |

### Crawling (5 tools)

| Tool | Description |
|------|-------------|
| `crawling.spider` | Crawl website |
| `crawling.robots` | Parse robots.txt |
| `crawling.sitemap` | Generate XML sitemap |
| `crawling.snapshot` | Take labeled DOM snapshot |
| `crawling.compare` | Compare DOM snapshots |

### Stealth (4 tools)

| Tool | Description |
|------|-------------|
| `stealth.inject` | Inject stealth patches |
| `stealth.test` | Bot detection test |
| `stealth.fingerprint` | Apply browser fingerprint |
| `stealth.block_domains` | Block ad/tracker domains |
| `stealth.detect_captcha` | Detect CAPTCHAs |

### Data (5 tools)

| Tool | Description |
|------|-------------|
| `data.pipeline` | Multi-step data pipeline |
| `data.http_get` | HTTP GET via browser |
| `data.http_post` | HTTP POST via browser |
| `data.links` | Extract links as edges |
| `data.graph` | Analyze link graph |

### Automation (2 tools)

| Tool | Description |
|------|-------------|
| `automation.rate_limit` | Rate limiter status |
| `automation.retry` | Retry queue management |

### Passkey (6 tools)

| Tool | Description |
|------|-------------|
| `auth_passkey_enable` | Enable virtual authenticator |
| `auth_passkey_create` | Add credential |
| `auth_passkey_list` | List credentials |
| `auth_passkey_log` | Operation log |
| `auth_passkey_disable` | Disable authenticator |
| `auth_passkey_remove` | Remove credential |

---

## Performance

Build profile (release):
- `opt-level = 3`, `lto = "thin"`, `codegen-units = 1`, `strip = true`, `panic = "abort"`

Key optimizations:
- Async parallelism via `tokio::join!()` and `futures::join_all()`
- Binary search rate limiter O(log n)
- Zero-copy CSV streaming with `Cow<str>`
- Typed Serialize structs (no `json!()` macros)
- Pre-computed URL endpoints in proxy client
- gzip response compression
- Shared JS templates (6 action functions from 1 template)
- `get_tab_page()` helper (clone Page handle, drop locks)
- MAX_SNAPSHOTS=64 eviction policy

---

## Development

```bash
# Build (excluding python binding)
cargo build -p onecrawl-cdp -p onecrawl-server -p onecrawl-cli-rs -p onecrawl-mcp-rs

# Run tests
cargo test -p onecrawl-cdp -- --include-ignored    # 148 tests

# Run CLI
cargo run -p onecrawl-cli-rs -- <command>

# Start HTTP server
cargo run -p onecrawl-cli-rs -- serve --port 9867

# Start MCP server
cargo run -p onecrawl-cli-rs -- mcp --transport stdio
```
