---
sidebar_position: 2
title: CLI Reference
---

# CLI Reference

OneCrawl ships with **80+ top-level commands** plus dozens of subcommands. Every command follows the pattern:

```bash
onecrawl <command> [args] [options]
```

Global flags available on all commands:

| Flag | Description |
|---|---|
| `--headless` | Run in headless mode (default: `true`) |
| `--headed` | Run in headed mode |
| `--profile <name>` | Use a named browser profile |
| `--timeout <ms>` | Override the default timeout |
| `--verbose` | Enable verbose logging |

---

## Navigation

| Command | Description |
|---|---|
| `navigate <url>` | Navigate the current page to a URL |
| `back` | Go back in browser history |
| `forward` | Go forward in browser history |
| `reload` | Reload the current page |

```bash
onecrawl navigate "https://example.com"
onecrawl back
onecrawl forward
onecrawl reload
```

---

## Content

| Command | Description |
|---|---|
| `get text` | Get visible text content of the page |
| `get html` | Get the full HTML of the page |
| `get url` | Get the current URL |
| `get title` | Get the page title |
| `eval <expression>` | Evaluate a JavaScript expression and return the result |
| `set-content <html>` | Replace the page content with the given HTML |

```bash
onecrawl get text
onecrawl get html
onecrawl eval "document.querySelectorAll('a').length"
onecrawl set-content "<h1>Hello World</h1>"
```

---

## Interaction

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

```bash
onecrawl click "#submit-btn"
onecrawl type "#search" "OneCrawl"
onecrawl fill "#email" "user@example.com"
onecrawl select-option "#country" "IT"
onecrawl upload "#file-input" ./resume.pdf
```

---

## Keyboard

| Command | Description |
|---|---|
| `press-key <key>` | Press and release a key (e.g., `Enter`, `Tab`) |
| `key-down <key>` | Hold a key down |
| `key-up <key>` | Release a held key |
| `keyboard-shortcut <keys>` | Execute a keyboard shortcut (e.g., `Ctrl+C`) |

```bash
onecrawl press-key Enter
onecrawl keyboard-shortcut "Ctrl+A"
onecrawl key-down Shift
onecrawl press-key ArrowDown
onecrawl key-up Shift
```

---

## Screenshots & PDF

| Command | Description |
|---|---|
| `screenshot [options]` | Capture a screenshot |
| `pdf [options]` | Export the page as a PDF |

### Screenshot Options

| Option | Description |
|---|---|
| `--output <path>` | Output file path |
| `--full` | Capture the full scrollable page |
| `--element <selector>` | Capture a specific element |
| `--format <png\|jpeg\|webp>` | Image format (default: `png`) |
| `--quality <0-100>` | JPEG/WebP quality |

### PDF Options

| Option | Description |
|---|---|
| `--output <path>` | Output file path |
| `--landscape` | Use landscape orientation |
| `--scale <float>` | Scale factor (default: `1.0`) |

```bash
onecrawl screenshot --full --output page.png
onecrawl screenshot --element "#hero" --format jpeg --quality 80
onecrawl pdf --output page.pdf --landscape --scale 0.8
```

---

## Wait

| Command | Description |
|---|---|
| `wait <ms>` | Wait for a fixed duration in milliseconds |
| `wait-for-selector <selector>` | Wait until a selector appears in the DOM |
| `wait-for-url <pattern>` | Wait until the URL matches a pattern |

```bash
onecrawl wait 2000
onecrawl wait-for-selector ".loaded"
onecrawl wait-for-url "**/dashboard**"
```

---

## Browser Features (Subcommands)

These commands use a subcommand pattern: `onecrawl <command> <subcommand> [args]`.

| Command | Subcommands / Description |
|---|---|
| `cookie` | `get`, `get-all`, `set`, `delete`, `clear` — Cookie management |
| `emulate` | `device <name>`, `media <type>`, `timezone <tz>`, `locale <locale>`, `geolocation <lat,lng>` |
| `network` | `throttle <profile>`, `intercept <pattern>`, `block <domains>`, `offline`, `online` |
| `har` | `start`, `stop`, `export <path>` — HAR recording |
| `ws` | `connect <url>`, `send <msg>`, `close` — WebSocket management |
| `coverage` | `start`, `stop`, `report` — Code coverage |
| `a11y` | `snapshot`, `tree`, `audit` — Accessibility tools |
| `throttle` | `cpu <rate>`, `network <profile>` — Performance throttling |
| `perf` | `metrics`, `trace-start`, `trace-stop` — Performance metrics |
| `console` | `start`, `stop`, `messages` — Console log capture |
| `dialog` | `accept [text]`, `dismiss` — Dialog/alert handling |
| `worker` | `list`, `evaluate <id> <expr>` — Service/web worker interaction |
| `dom` | `query <selector>`, `attributes <selector>`, `set-attribute <selector> <attr> <value>` |
| `iframe` | `list`, `switch <index\|selector>`, `switch-main` — iframe navigation |
| `network-log` | `start`, `stop`, `export <path>` — Network request logging |
| `page-watcher` | `start`, `stop` — Watch for page events |
| `print` | `preview`, `options` — Print dialog control |
| `web-storage` | `local-get <key>`, `local-set <key> <value>`, `session-get <key>`, `session-set <key> <value>`, `clear` |
| `auth` | `passkey-enable`, `passkey-add`, `passkey-list`, `passkey-log`, `passkey-disable`, `passkey-remove` |
| `stealth` | `inject`, `test`, `fingerprint`, `block-domains <domains>`, `detect-captcha` |
| `antibot` | `detect`, `bypass`, `status` — Anti-bot detection and bypass |
| `adaptive` | `wait`, `scroll`, `interact` — Adaptive (human-like) automation |

```bash
# Set a cookie
onecrawl cookie set --name "session" --value "abc123" --domain ".example.com"

# Emulate a mobile device
onecrawl emulate device "iPhone 15 Pro"

# Record a HAR file
onecrawl har start
onecrawl navigate "https://example.com"
onecrawl har stop
onecrawl har export network.har

# Run an accessibility audit
onecrawl a11y audit
```

---

## Scraping & Crawling

| Command | Description |
|---|---|
| `select <css-selector>` | Extract text from elements matching a CSS selector |
| `nav <url>` | Navigate and immediately extract text |
| `extract <config>` | Extract structured data using a JSON config |
| `spider <url> [options]` | Crawl a site following links |
| `robots <url>` | Fetch and parse the `robots.txt` |
| `graph <url>` | Build a site link graph |
| `stream-extract <config>` | Stream extraction for large pages |
| `structured <url> <schema>` | Extract data into a structured JSON schema |
| `shell` | Open an interactive REPL for browser automation |

```bash
# CSS selector extraction
onecrawl select "h1" --url "https://example.com"

# Crawl with depth limit
onecrawl spider "https://example.com" --depth 3 --concurrency 5

# Structured extraction
onecrawl structured "https://example.com/products" \
  '{"name": "h2.title", "price": ".price", "image": "img@src"}'
```

---

## Advanced

| Command | Description |
|---|---|
| `fingerprint` | Display or randomize the browser fingerprint |
| `snapshot` | Create a DOM snapshot |
| `http <method> <url>` | Make a raw HTTP request through the browser context |
| `ratelimit <config>` | Configure rate limiting for automation |
| `retry <command>` | Retry a command on failure with backoff |
| `pipeline <file>` | Execute a pipeline of commands from a YAML/JSON file |
| `captcha` | Detect and solve CAPTCHAs |
| `schedule <cron> <command>` | Schedule a command to run on a cron pattern |
| `pool <size>` | Manage a pool of browser instances |
| `geo <lat> <lng>` | Override geolocation |
| `cookie-jar <file>` | Import/export cookies from a cookie jar file |
| `request <url> [options]` | Make an HTTP request (standalone, no browser) |
| `domain <domain>` | Get domain information and DNS records |
| `screenshot-diff <a> <b>` | Visual diff between two screenshots |
| `proxy <url>` | Configure proxy for all requests |
| `proxy-health <url>` | Check proxy health and latency |
| `intercept <pattern> <action>` | Intercept and modify network requests |
| `advanced-emulation <config>` | Fine-grained device/network emulation |
| `tab` | `open [url]`, `list`, `switch <id>`, `close <id>` — Tab management |
| `download <url> [output]` | Download a file |

```bash
# Run a pipeline
onecrawl pipeline scrape-jobs.yaml

# Pool of 5 browsers
onecrawl pool 5 --command "navigate https://example.com && screenshot"

# Proxy with health check
onecrawl proxy "socks5://proxy.example.com:1080"
onecrawl proxy-health "socks5://proxy.example.com:1080"

# Visual regression
onecrawl screenshot-diff before.png after.png --output diff.png
```

---

## Server

| Command | Description |
|---|---|
| `serve [options]` | Start the HTTP API server |
| `mcp [options]` | Start the MCP (Model Context Protocol) server |

| Option | Description |
|---|---|
| `--port <port>` | Port to listen on (default: `9867`) |
| `--bind <address>` | Bind address (default: `127.0.0.1`) |
| `--transport <stdio\|sse>` | MCP transport protocol |

```bash
# Start the HTTP server
onecrawl serve --port 9867

# Start the MCP server for AI agents
onecrawl mcp --transport stdio
```

---

## System

| Command | Description |
|---|---|
| `health` | Check the health of the browser connection |
| `info` | Display system and browser information |
| `bench [command]` | Benchmark a command |
| `version` | Print the OneCrawl version |

```bash
onecrawl health
onecrawl info
onecrawl bench "navigate https://example.com && get text"
onecrawl version
```

---

## Practical Examples

### 1. Login and scrape a dashboard

```bash
onecrawl navigate "https://app.example.com/login" --headed
onecrawl fill "#email" "user@example.com"
onecrawl fill "#password" "s3cret"
onecrawl click "#login-btn"
onecrawl wait-for-url "**/dashboard**"
onecrawl get text
```

### 2. Scrape product prices with stealth mode

```bash
onecrawl stealth inject
onecrawl navigate "https://shop.example.com/products"
onecrawl select ".product-card .price" --json
```

### 3. Record a HAR and export it

```bash
onecrawl har start
onecrawl navigate "https://api.example.com"
onecrawl wait 3000
onecrawl har stop
onecrawl har export api-trace.har
```

### 4. Full-page PDF export with custom scale

```bash
onecrawl navigate "https://example.com/report"
onecrawl wait-for-selector ".report-loaded"
onecrawl pdf --output report.pdf --landscape --scale 0.75
```

### 5. Crawl a site and build a link graph

```bash
onecrawl spider "https://docs.example.com" --depth 2 --output sitemap.json
onecrawl graph "https://docs.example.com" --output graph.json
```

### 6. Accessibility audit

```bash
onecrawl navigate "https://example.com"
onecrawl a11y audit --format json --output a11y-report.json
```

---

## Real-World Workflows

### Workflow 1: Scrape a Product Page

Navigate to a product page, extract structured data, and save it as JSON.

```bash
# Navigate to the product page
onecrawl navigate "https://shop.example.com/product/wireless-headphones"
onecrawl wait-for-selector ".product-details"

# Extract structured data into JSON
onecrawl structured "https://shop.example.com/product/wireless-headphones" \
  '{"name": "h1.product-name", "price": ".price-current", "sku": ".sku-value", "rating": ".star-rating@data-score", "image": "img.product-image@src"}'

# Save the full page text as a backup
onecrawl get text > product-details.txt
```

### Workflow 2: Monitor a Website for Changes

Spider a site, take snapshots, and use scheduled runs to detect visual regressions.

```bash
# Step 1: Crawl and snapshot the baseline
onecrawl spider "https://status.example.com" --depth 1 --output urls.json
onecrawl navigate "https://status.example.com"
onecrawl screenshot --full --output baseline.png

# Step 2: Schedule a daily check
onecrawl schedule "0 9 * * *" "navigate https://status.example.com && screenshot --full --output today.png"

# Step 3: Compare snapshots for visual diffs
onecrawl screenshot-diff baseline.png today.png --output diff.png
```

### Workflow 3: Automate Login with Passkey

Use OneCrawl's built-in passkey support for modern passwordless authentication.

```bash
# Enable virtual passkey authenticator
onecrawl auth passkey-enable

# Navigate to the login page
onecrawl navigate "https://app.example.com/login" --headed

# Fill in the username and trigger passkey auth
onecrawl fill "#username" "user@example.com"
onecrawl click "#login-with-passkey"

# Wait for authentication to complete
onecrawl wait-for-url "**/dashboard**"
onecrawl get text
```

### Workflow 4: Screenshot All Pages of a Site

Spider a website and capture a full-page screenshot of every discovered page.

```bash
# Crawl the site to discover all pages
onecrawl spider "https://docs.example.com" --depth 3 --output sitemap.json

# Loop through each URL and screenshot it
for url in $(cat sitemap.json | jq -r '.[].url'); do
  slug=$(echo "$url" | sed 's|https://||; s|/|_|g')
  onecrawl navigate "$url"
  onecrawl wait 1000
  onecrawl screenshot --full --output "screenshots/${slug}.png"
done
```

### Workflow 5: Extract API Data from Network Traffic

Capture network requests to extract JSON API responses made by a single-page application.

```bash
# Start network logging
onecrawl network-log start

# Navigate and interact to trigger API calls
onecrawl navigate "https://app.example.com/dashboard"
onecrawl wait-for-selector ".data-loaded"
onecrawl click "#load-more"
onecrawl wait 3000

# Stop logging and export
onecrawl network-log stop
onecrawl network-log export network.har

# Filter JSON API responses from the HAR file
cat network.har | jq '[.log.entries[] | select(.response.content.mimeType == "application/json") | {url: .request.url, status: .response.status, body: .response.content.text}]'
```

### Workflow 6: Stealth Scraping with Fingerprint Randomization

Bypass anti-bot protections using stealth injection and fingerprint spoofing.

```bash
# Inject stealth patches (hides automation signals)
onecrawl stealth inject

# Randomize browser fingerprint
onecrawl fingerprint

# Verify stealth is working
onecrawl stealth test

# Navigate with stealth active
onecrawl navigate "https://protected-site.example.com"
onecrawl wait-for-selector ".content-loaded"

# Detect if a CAPTCHA appeared
onecrawl stealth detect-captcha

# Extract the data
onecrawl get text > scraped-content.txt
```
