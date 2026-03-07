---
sidebar_position: 2
title: CLI Reference
---

# CLI Reference

OneCrawl ships with **409+ commands** organized across 15+ categories. Every command follows the pattern:

```
onecrawl <command> [subcommand] [args] [options]
```

---

## Global Flags

These flags are available on **all** commands:

| Flag | Short | Description |
|---|---|---|
| `--headless` | | Run Chrome in headless mode (default: `true`) |
| `--headed` | `-H` | Run Chrome in headed mode (visible window) |
| `--profile <name>` | `-p` | Use a named browser profile for persistent state |
| `--timeout <ms>` | `-t` | Override the default command timeout (default: `30000`) |
| `--verbose` | `-v` | Enable verbose logging to stderr |
| `--json` | `-j` | Output results as JSON |
| `--quiet` | `-q` | Suppress all non-essential output |
| `--no-color` | | Disable colored output |
| `--config <path>` | `-c` | Path to a config file (YAML/JSON) |
| `--port <port>` | | Chrome DevTools port to connect to |
| `--proxy <url>` | | Route all traffic through a proxy |

---

## Exit Codes

| Code | Meaning |
|---|---|
| `0` | Success |
| `1` | General error (command failed) |
| `2` | Invalid arguments or usage error |
| `3` | Timeout — command exceeded `--timeout` |
| `4` | Element not found — selector did not match any elements |
| `5` | Navigation error — page failed to load |
| `6` | Browser connection error — Chrome not reachable |
| `7` | Authentication error — passkey or credential failure |
| `10` | Pipeline error — step failed during pipeline execution |

Use exit codes in scripts for robust error handling:

```bash
onecrawl navigate "https://example.com" || {
  code=$?
  case $code in
    3) echo "Timeout — page too slow" ;;
    5) echo "Navigation failed — check URL" ;;
    6) echo "Chrome not running" ;;
    *) echo "Unknown error (exit $code)" ;;
  esac
  exit $code
}
```

---

## Navigation (12 commands)

Core browser navigation commands.

| Command | Description |
|---|---|
| `navigate <url>` | Navigate the current page to a URL |
| `nav <url>` | Navigate and immediately extract text |
| `back` | Go back in browser history |
| `forward` | Go forward in browser history |
| `reload` | Reload the current page |
| `reload --hard` | Hard reload (bypass cache) |
| `get url` | Get the current URL |
| `get title` | Get the page title |
| `wait-for-url <pattern>` | Wait until the URL matches a glob pattern |
| `wait-for-selector <selector>` | Wait until a selector appears in the DOM |
| `wait <ms>` | Wait for a fixed duration in milliseconds |
| `close` | Close the browser |

```bash
# Navigate and wait for content
onecrawl navigate "https://example.com"
onecrawl wait-for-selector ".content-loaded"

# Navigate with timeout
onecrawl navigate "https://slow-site.com" --timeout 60000

# History navigation
onecrawl back
onecrawl forward
onecrawl reload --hard
```

---

## Content Extraction (14 commands)

Extract text, HTML, and structured data from pages.

| Command | Description |
|---|---|
| `get text` | Get visible text content of the page |
| `get html` | Get the full HTML of the page |
| `get url` | Get the current URL |
| `get title` | Get the page title |
| `select <css-selector>` | Extract text from elements matching a CSS selector |
| `eval <expression>` | Evaluate a JavaScript expression and return the result |
| `set-content <html>` | Replace the page content with the given HTML |
| `structured <url> <schema>` | Extract data into a structured JSON schema |
| `extract <config>` | Extract structured data using a JSON config |
| `stream-extract <config>` | Stream extraction for large pages |
| `snapshot` | Create a full DOM snapshot |
| `markdown` | Convert current page to Markdown |
| `robots <url>` | Fetch and parse the `robots.txt` |
| `graph <url>` | Build a site link graph |

```bash
# Basic text and HTML extraction
onecrawl get text
onecrawl get html > page.html

# CSS selector extraction
onecrawl select "h1" --url "https://example.com"
onecrawl select ".product-price" --json

# JavaScript evaluation
onecrawl eval "document.querySelectorAll('a').length"
onecrawl eval "JSON.stringify(window.__DATA__)"

# Structured extraction with schema
onecrawl structured "https://example.com/products" \
  '{"name": "h2.title", "price": ".price", "image": "img@src"}'

# Convert page to Markdown (great for LLM consumption)
onecrawl navigate "https://docs.example.com/api"
onecrawl markdown
```

---

## DOM Interaction (22 commands)

Click, type, fill forms, and interact with page elements.

| Command | Description |
|---|---|
| `click <selector>` | Click an element |
| `dblclick <selector>` | Double-click an element |
| `type <selector> <text>` | Type text into an input (keystroke simulation) |
| `fill <selector> <value>` | Set the value of an input field directly |
| `focus <selector>` | Focus an element |
| `hover <selector>` | Hover over an element |
| `scroll-into-view <selector>` | Scroll an element into the viewport |
| `check <selector>` | Check a checkbox |
| `uncheck <selector>` | Uncheck a checkbox |
| `select-option <selector> <value>` | Select a dropdown option by value |
| `tap <selector>` | Tap an element (touch event) |
| `drag <from> <to>` | Drag from one selector to another |
| `upload <selector> <file>` | Upload a file to a file input |
| `bounding-box <selector>` | Get the bounding box of an element |
| `press-key <key>` | Press and release a key (e.g., `Enter`, `Tab`) |
| `key-down <key>` | Hold a key down |
| `key-up <key>` | Release a held key |
| `keyboard-shortcut <keys>` | Execute a keyboard shortcut (e.g., `Ctrl+C`) |
| `dom query <selector>` | Query DOM elements |
| `dom attributes <selector>` | Get element attributes |
| `dom set-attribute <sel> <attr> <val>` | Set an element attribute |
| `dom remove <selector>` | Remove an element from the DOM |

```bash
# Form interactions
onecrawl fill "#email" "user@example.com"
onecrawl fill "#password" "s3cret"
onecrawl click "#login-btn"

# Type with keystroke simulation (useful for autocomplete)
onecrawl type "#search" "OneCrawl" --delay 50

# Select dropdowns and checkboxes
onecrawl select-option "#country" "IT"
onecrawl check "#terms-agree"

# File upload
onecrawl upload "#file-input" ./resume.pdf

# Keyboard shortcuts
onecrawl keyboard-shortcut "Ctrl+A"
onecrawl press-key Enter

# Drag and drop
onecrawl drag "#card-1" "#column-done"
```

---

## Screenshots & PDF (8 commands)

Capture visual output in multiple formats.

| Command | Description |
|---|---|
| `screenshot` | Capture a screenshot |
| `screenshot --full` | Full scrollable page screenshot |
| `screenshot --element <sel>` | Capture a specific element |
| `screenshot-diff <a> <b>` | Visual diff between two screenshots |
| `pdf` | Export the page as a PDF |
| `pdf --landscape` | PDF in landscape orientation |
| `pdf --scale <float>` | PDF with custom scale factor |
| `print preview` | Preview print layout |

### Screenshot Options

| Option | Description |
|---|---|
| `--output <path>` | Output file path |
| `--full` | Capture the full scrollable page |
| `--element <selector>` | Capture a specific element |
| `--format <png\|jpeg\|webp>` | Image format (default: `png`) |
| `--quality 0-100` | JPEG/WebP quality |
| `--clip x,y,w,h` | Clip region |

```bash
# Full-page screenshot
onecrawl screenshot --full --output page.png

# Element screenshot as JPEG
onecrawl screenshot --element "#hero" --format jpeg --quality 80

# PDF export
onecrawl pdf --output page.pdf --landscape --scale 0.8

# Visual regression testing
onecrawl screenshot-diff before.png after.png --output diff.png --threshold 0.1
```

---

## Cookie Management (5 commands)

| Command | Description |
|---|---|
| `cookie get <name>` | Get a specific cookie |
| `cookie get-all` | Get all cookies |
| `cookie set` | Set a cookie |
| `cookie delete <name>` | Delete a specific cookie |
| `cookie clear` | Clear all cookies |

```bash
# Set a cookie
onecrawl cookie set --name "session" --value "abc123" --domain ".example.com"

# Get all cookies as JSON
onecrawl cookie get-all --json

# Export and import cookies
onecrawl cookie get-all --json > cookies.json
onecrawl cookie-jar cookies.json --import
```

---

## Network & HAR (18 commands)

Network throttling, interception, HAR recording, and WebSocket management.

| Command | Description |
|---|---|
| `network throttle <profile>` | Throttle network (e.g., `3g`, `4g`, `slow-3g`) |
| `network intercept <pattern>` | Intercept requests matching a pattern |
| `network block <domains>` | Block requests to specified domains |
| `network offline` | Simulate offline mode |
| `network online` | Restore network connectivity |
| `har start` | Start HAR recording |
| `har stop` | Stop HAR recording |
| `har export <path>` | Export HAR to file |
| `ws connect <url>` | Connect to a WebSocket |
| `ws send <msg>` | Send a WebSocket message |
| `ws close` | Close the WebSocket connection |
| `network-log start` | Start network request logging |
| `network-log stop` | Stop network request logging |
| `network-log export <path>` | Export network log |
| `proxy <url>` | Configure proxy for all requests |
| `proxy-health <url>` | Check proxy health and latency |
| `intercept <pattern> <action>` | Intercept and modify network requests |
| `request <url>` | Make a standalone HTTP request |

```bash
# Simulate slow network
onecrawl network throttle 3g

# Block tracking domains
onecrawl network block "google-analytics.com,facebook.net,doubleclick.net"

# Record and export HAR
onecrawl har start
onecrawl navigate "https://example.com"
onecrawl har stop
onecrawl har export network.har

# WebSocket interaction
onecrawl ws connect "wss://stream.example.com"
onecrawl ws send '{"subscribe": "prices"}'

# Proxy with health check
onecrawl proxy "socks5://proxy.example.com:1080"
onecrawl proxy-health "socks5://proxy.example.com:1080"
```

---

## Emulation (10 commands)

Device, viewport, timezone, locale, and geolocation emulation.

| Command | Description |
|---|---|
| `emulate device <name>` | Emulate a device (e.g., `iPhone 15 Pro`) |
| `emulate viewport <w> <h>` | Set custom viewport dimensions |
| `emulate media <type>` | Override media type (`screen`, `print`) |
| `emulate timezone <tz>` | Override timezone |
| `emulate locale <locale>` | Override locale |
| `emulate geolocation <lat,lng>` | Override geolocation |
| `throttle cpu <rate>` | Throttle CPU speed |
| `throttle network <profile>` | Throttle network speed |
| `advanced-emulation <config>` | Fine-grained device/network emulation |
| `geo <lat> <lng>` | Override geolocation (shortcut) |

```bash
# Mobile device emulation
onecrawl emulate device "iPhone 15 Pro"

# Custom viewport
onecrawl emulate viewport 1920 1080

# Timezone and locale
onecrawl emulate timezone "Asia/Tokyo"
onecrawl emulate locale "ja-JP"

# Geolocation (Rome)
onecrawl emulate geolocation 41.9028,12.4964
```

---

## Stealth & Anti-Bot (12 commands)

Anti-detection, fingerprint manipulation, and bot bypass.

| Command | Description |
|---|---|
| `stealth inject` | Inject all 12 stealth patches into the browser |
| `stealth test` | Run detection tests against current page |
| `stealth fingerprint` | Display or randomize browser fingerprint |
| `stealth block-domains <domains>` | Block requests to specified tracking domains |
| `stealth detect-captcha` | Detect CAPTCHA presence on the page |
| `antibot detect` | Detect anti-bot protection type |
| `antibot bypass` | Attempt to bypass anti-bot protection |
| `antibot status` | Check current anti-bot bypass status |
| `fingerprint` | Display or randomize the browser fingerprint |
| `captcha` | Detect and solve CAPTCHAs |
| `adaptive wait` | Human-like adaptive waiting |
| `adaptive scroll` | Human-like adaptive scrolling |

```bash
# Full stealth setup
onecrawl stealth inject
onecrawl stealth fingerprint --randomize
onecrawl stealth test

# Block tracking + navigate
onecrawl stealth block-domains "google-analytics.com,facebook.net"
onecrawl navigate "https://protected-site.example.com"

# CAPTCHA detection
onecrawl stealth detect-captcha

# Anti-bot bypass
onecrawl antibot detect
onecrawl antibot bypass
```

---

## Authentication & Passkeys (8 commands)

WebAuthn/passkey management for modern passwordless authentication.

| Command | Description |
|---|---|
| `auth passkey-enable` | Enable the virtual authenticator |
| `auth passkey-add` | Add a passkey credential |
| `auth passkey-list` | List all registered credentials |
| `auth passkey-log` | Get the authenticator event log |
| `auth passkey-disable` | Disable the virtual authenticator |
| `auth passkey-remove` | Remove a specific credential |
| `cookie-jar <file>` | Import/export cookies from a cookie jar |
| `cookie-jar <file> --import` | Import cookies from file |

```bash
# Enable virtual passkey authenticator
onecrawl auth passkey-enable

# Navigate and use passkey login
onecrawl navigate "https://app.example.com/login" --headed
onecrawl fill "#username" "user@example.com"
onecrawl click "#login-with-passkey"
onecrawl wait-for-url "**/dashboard**"
```

---

## Accessibility (6 commands)

| Command | Description |
|---|---|
| `a11y snapshot` | Get the accessibility tree snapshot |
| `a11y tree` | Get the full accessibility tree |
| `a11y audit` | Run an accessibility audit |
| `a11y audit --format json` | Audit with JSON output |
| `a11y violations` | List accessibility violations |
| `a11y score` | Get accessibility score |

```bash
# Full accessibility audit
onecrawl navigate "https://example.com"
onecrawl a11y audit --format json --output a11y-report.json

# Get interactive elements snapshot
onecrawl a11y snapshot --filter interactive
```

---

## Tab Management (6 commands)

| Command | Description |
|---|---|
| `tab open [url]` | Open a new tab |
| `tab list` | List all open tabs |
| `tab switch <id>` | Switch to a specific tab |
| `tab close <id>` | Close a specific tab |
| `tab duplicate` | Duplicate the current tab |
| `tab reload <id>` | Reload a specific tab |

```bash
# Multi-tab workflow
onecrawl tab open "https://example.com"
onecrawl tab open "https://example.com/about"
onecrawl tab list --json
onecrawl tab switch 0
```

---

## Crawling & Spidering (8 commands)

| Command | Description |
|---|---|
| `spider <url>` | Crawl a site following links |
| `spider <url> --depth <n>` | Crawl to a specific depth |
| `spider <url> --concurrency <n>` | Parallel crawling |
| `spider <url> --output <file>` | Save results to file |
| `graph <url>` | Build a site link graph |
| `robots <url>` | Fetch and parse `robots.txt` |
| `domain <domain>` | Get domain information and DNS records |
| `download <url> [output]` | Download a file |

```bash
# Crawl with depth and concurrency limits
onecrawl spider "https://docs.example.com" --depth 3 --concurrency 5 --output sitemap.json

# Build a link graph
onecrawl graph "https://example.com" --depth 2 --output graph.json

# Check robots.txt
onecrawl robots "https://example.com"
```

---

## Browser Features (32 commands)

Advanced browser subsystems accessed via subcommand patterns.

| Subsystem | Commands | Description |
|---|---|---|
| `console` | `start`, `stop`, `messages` | Console log capture |
| `dialog` | `accept [text]`, `dismiss` | Dialog/alert handling |
| `iframe` | `list`, `switch <idx>`, `switch-main` | iframe navigation |
| `worker` | `list`, `evaluate <id> <expr>` | Service/web worker interaction |
| `web-storage` | `local-get`, `local-set`, `session-get`, `session-set`, `clear` | Web storage management |
| `coverage` | `start`, `stop`, `report` | Code coverage collection |
| `perf` | `metrics`, `trace-start`, `trace-stop` | Performance metrics & tracing |
| `page-watcher` | `start`, `stop` | Watch for page events |
| `print` | `preview`, `options` | Print dialog control |

```bash
# Capture console logs
onecrawl console start
onecrawl navigate "https://example.com"
onecrawl console messages --json

# Performance metrics
onecrawl perf metrics
onecrawl perf trace-start
onecrawl navigate "https://example.com"
onecrawl perf trace-stop --output trace.json

# Web storage
onecrawl web-storage local-set "theme" "dark"
onecrawl web-storage local-get "theme"
```

---

## Pipeline & Scheduling (6 commands)

| Command | Description |
|---|---|
| `pipeline <file>` | Execute a pipeline from YAML/JSON |
| `schedule <cron> <command>` | Schedule a command on a cron pattern |
| `retry <command>` | Retry a command on failure with backoff |
| `ratelimit <config>` | Configure rate limiting |
| `pool <size>` | Manage a pool of browser instances |
| `bench [command]` | Benchmark a command |

```bash
# Run a pipeline
onecrawl pipeline scrape-jobs.yaml

# Pool of 5 browsers for parallel work
onecrawl pool 5 --command "navigate https://example.com && screenshot"

# Schedule a daily check
onecrawl schedule "0 9 * * *" "navigate https://status.example.com && screenshot --full --output daily.png"

# Benchmark
onecrawl bench "navigate https://example.com && get text"
```

---

## Server (4 commands)

| Command | Description |
|---|---|
| `serve` | Start the HTTP API server |
| `mcp` | Start the MCP server for AI agents |
| `health` | Check browser connection health |
| `info` | Display system and browser information |

```bash
# Start HTTP server
onecrawl serve --port 9867 --bind 0.0.0.0

# Start MCP server
onecrawl mcp --transport stdio
onecrawl mcp --transport sse --port 3001

# System info
onecrawl info
onecrawl version
```

---

## Real-World Workflows

### 1. Login and scrape a dashboard

```bash
onecrawl navigate "https://app.example.com/login" --headed
onecrawl fill "#email" "user@example.com"
onecrawl fill "#password" "s3cret"
onecrawl click "#login-btn"
onecrawl wait-for-url "**/dashboard**"
onecrawl get text
```

### 2. Stealth scraping with fingerprint randomization

```bash
onecrawl stealth inject
onecrawl stealth fingerprint --randomize
onecrawl stealth test
onecrawl navigate "https://protected-site.example.com"
onecrawl wait-for-selector ".content-loaded"
onecrawl stealth detect-captcha
onecrawl get text > scraped-content.txt
```

### 3. Record HAR and analyze API calls

```bash
onecrawl har start
onecrawl navigate "https://app.example.com/dashboard"
onecrawl wait-for-selector ".data-loaded"
onecrawl click "#load-more"
onecrawl wait 3000
onecrawl har stop
onecrawl har export api-trace.har
cat api-trace.har | jq '[.log.entries[] | select(.response.content.mimeType == "application/json") | {url: .request.url, status: .response.status}]'
```

### 4. Screenshot all pages of a site

```bash
onecrawl spider "https://docs.example.com" --depth 3 --output sitemap.json
for url in $(cat sitemap.json | jq -r '.[].url'); do
  slug=$(echo "$url" | sed 's|https://||; s|/|_|g')
  onecrawl navigate "$url"
  onecrawl wait 1000
  onecrawl screenshot --full --output "screenshots/${slug}.png"
done
```

### 5. Passwordless login with passkeys

```bash
onecrawl auth passkey-enable
onecrawl navigate "https://app.example.com/login" --headed
onecrawl fill "#username" "user@example.com"
onecrawl click "#login-with-passkey"
onecrawl wait-for-url "**/dashboard**"
onecrawl get text
```

### 6. Visual regression testing

```bash
# Capture baseline
onecrawl navigate "https://staging.example.com"
onecrawl screenshot --full --output baseline.png

# After deploy, capture current state
onecrawl navigate "https://staging.example.com"
onecrawl screenshot --full --output current.png

# Compare
onecrawl screenshot-diff baseline.png current.png --output diff.png --threshold 0.05
```

### 7. Accessibility audit with JSON report

```bash
onecrawl navigate "https://example.com"
onecrawl a11y audit --format json --output a11y-report.json
onecrawl a11y score
```

### 8. Multi-instance HTTP API workflow

```bash
# Start the server
onecrawl serve --port 9867

# Create instance and automate via REST
INSTANCE=$(curl -s -X POST http://localhost:9867/instances \
  -H "Content-Type: application/json" \
  -d '{"headless": true}' | jq -r '.id')

TAB=$(curl -s -X POST http://localhost:9867/instances/$INSTANCE/tabs/open \
  -H "Content-Type: application/json" \
  -d '{"url": "https://example.com"}' | jq -r '.tab_id')

curl -s "http://localhost:9867/tabs/$TAB/text" | jq -r '.text'
curl -s -X DELETE http://localhost:9867/instances/$INSTANCE
```

---

## Error Handling in Scripts

```bash
#!/usr/bin/env bash
set -euo pipefail

# Navigate with error handling
if ! onecrawl navigate "https://example.com" --timeout 15000; then
  echo "Navigation failed" >&2
  exit 1
fi

# Wait for content with fallback
if ! onecrawl wait-for-selector ".content" --timeout 5000 2>/dev/null; then
  echo "Content not found, trying alternative selector..." >&2
  onecrawl wait-for-selector ".main-content" --timeout 5000
fi

# Extract with JSON output
onecrawl get text --json | jq -r '.text'
```
