---
sidebar_position: 4
title: MCP Tools Reference
---

# MCP Tools Reference

OneCrawl exposes **17 MCP super-tools** providing **421 actions** across **10 namespaces** for seamless integration with AI agents, coding assistants, and agentic workflows.

Each super-tool groups related actions into a single callable tool with an `action` parameter, reducing tool discovery overhead while keeping full functionality.

---

## Quick Start

### Start the MCP server

**stdio transport** — for local AI agents (Claude Desktop, Cursor, etc.):

```bash
onecrawl mcp --transport stdio
```

**SSE transport** — for remote connections:

```bash
onecrawl mcp --transport sse --port 3001
```

### Configure in Claude Desktop

Add to `~/.claude/claude_desktop_config.json`:

```json
{
  "mcpServers": {
    "onecrawl": {
      "command": "onecrawl",
      "args": ["mcp", "--transport", "stdio"]
    }
  }
}
```

### Configure in Cursor

Add to `.cursor/mcp.json`:

```json
{
  "mcpServers": {
    "onecrawl": {
      "command": "onecrawl",
      "args": ["mcp", "--transport", "stdio"]
    }
  }
}
```

### Configure with SSE (remote)

```json
{
  "mcpServers": {
    "onecrawl": {
      "url": "http://localhost:3001/sse"
    }
  }
}
```

---

## JSON-RPC Protocol

All tools are invoked via the [Model Context Protocol](https://modelcontextprotocol.io/) JSON-RPC interface:

```json
{
  "jsonrpc": "2.0",
  "id": 1,
  "method": "tools/call",
  "params": {
    "name": "navigation",
    "arguments": {
      "action": "goto",
      "url": "https://example.com"
    }
  }
}
```

Response format:

```json
{
  "jsonrpc": "2.0",
  "id": 1,
  "result": {
    "content": [
      {
        "type": "text",
        "text": "{\"url\": \"https://example.com\", \"status\": 200, \"title\": \"Example Domain\"}"
      }
    ]
  }
}
```

---

## Super-Tool Reference

### 1. `navigation` — Browser Navigation & Interaction

**32 actions** for page navigation, element interaction, keyboard input, screenshots, PDF export, iframes, tabs, and page lifecycle management.

| Action | Description | Key Parameters |
|---|---|---|
| `goto` | Navigate to a URL | `url`, `waitUntil?` |
| `click` | Click an element | `selector` or `ref` |
| `type` | Type text with keystroke simulation | `selector`, `text`, `delay?` |
| `fill` | Set input value directly | `selector`, `value` |
| `hover` | Hover over an element | `selector` |
| `focus` | Focus an element | `selector` |
| `screenshot` | Take a screenshot | `fullPage?`, `selector?`, `format?` |
| `pdf` | Export page as PDF | `landscape?`, `scale?` |
| `back` | Go back in history | — |
| `forward` | Go forward in history | — |
| `reload` | Reload the page | `hard?` |
| `wait` | Wait for a condition | `selector?`, `timeout?`, `ms?` |
| `evaluate` | Evaluate JavaScript | `expression` |
| `get_text` | Get visible text content | `selector?` |
| `get_html` | Get HTML content | `selector?`, `outer?` |
| `get_url` | Get the current URL | — |
| `get_title` | Get the page title | — |
| `set_content` | Replace page HTML | `html` |
| `press_key` | Press a keyboard key | `key` |
| `keyboard_shortcut` | Execute key combination | `keys` |
| `check` | Check a checkbox | `selector` |
| `uncheck` | Uncheck a checkbox | `selector` |
| `select_option` | Select dropdown option | `selector`, `value` |
| `upload` | Upload a file | `selector`, `path` |
| `drag` | Drag and drop | `from`, `to` |
| `scroll_into_view` | Scroll element into view | `selector` |
| `bounding_box` | Get element dimensions | `selector` |
| `iframe_list` | List all iframes | — |
| `iframe_switch` | Switch to iframe context | `index` or `selector` |
| `tab_open` | Open a new tab | `url?` |
| `tab_list` | List all tabs | — |
| `tab_switch` | Switch to a tab | `id` |

**Example — Navigate, fill form, and screenshot:**

```json
{"name": "navigation", "arguments": {"action": "goto", "url": "https://example.com", "waitUntil": "networkidle"}}
```

```json
{"name": "navigation", "arguments": {"action": "fill", "selector": "#email", "value": "user@example.com"}}
```

```json
{"name": "navigation", "arguments": {"action": "screenshot", "fullPage": true, "format": "png"}}
```

---

### 2. `scraping` — Data Extraction

**28 actions** including smart selectors (CSS/XPath with `::text`/`::attr`), DOM navigation, content extraction, streaming extraction with pagination, and structured data parsing (JSON-LD, OpenGraph, Twitter Card).

| Action | Description | Key Parameters |
|---|---|---|
| `css` | Extract by CSS selector | `selector`, `attribute?`, `limit?` |
| `xpath` | Extract by XPath | `expression`, `limit?` |
| `find_text` | Find elements by text content | `text`, `exact?` |
| `text` | Get all visible text | `selector?` |
| `html` | Get HTML content | `selector?`, `outer?` |
| `markdown` | Convert page to Markdown | `selector?` |
| `structured` | Extract with JSON schema | `schema`, `url?` |
| `stream` | Stream extraction for large pages | `selector`, `batchSize?` |
| `detect_forms` | Detect all forms | — |
| `fill_form` | Fill and submit a form | `formSelector`, `fields`, `submit?` |
| `json_ld` | Extract JSON-LD structured data | — |
| `opengraph` | Extract OpenGraph metadata | — |
| `twitter_card` | Extract Twitter Card data | — |
| `table` | Extract table data | `selector`, `format?` |
| `links` | Extract all links | `filter?`, `absolute?` |
| `images` | Extract all images | `filter?` |
| `meta` | Extract meta tags | — |

**Example — Structured extraction from Hacker News:**

```json
{
  "name": "scraping",
  "arguments": {
    "action": "structured",
    "url": "https://news.ycombinator.com",
    "schema": {
      "stories": {
        "_selector": ".athing",
        "title": ".titleline > a",
        "url": ".titleline > a@href",
        "score": "+tr .score"
      }
    }
  }
}
```

**Example — Detect and fill a form:**

```json
{"name": "scraping", "arguments": {"action": "detect_forms"}}
```

```json
{
  "name": "scraping",
  "arguments": {
    "action": "fill_form",
    "formSelector": "#login-form",
    "fields": {"#email": "user@example.com", "#password": "s3cret"},
    "submit": true
  }
}
```

---

### 3. `crawling` — Site Crawling

**12 actions** for spidering sites, parsing sitemaps, creating DOM snapshots, and comparing page states.

| Action | Description | Key Parameters |
|---|---|---|
| `spider` | Crawl a site following links | `url`, `depth?`, `maxPages?`, `concurrency?` |
| `robots` | Fetch and parse robots.txt | `url` |
| `sitemap` | Fetch and parse XML sitemaps | `url` |
| `snapshot` | Create a full DOM snapshot | `url?` |
| `compare` | Compare two snapshots | `before`, `after` |
| `links` | Extract all links from page | `filter?`, `absolute?` |
| `graph` | Build a link graph | `url`, `depth?` |
| `domain` | Get domain info and DNS | `domain` |

**Example — Crawl a documentation site:**

```json
{
  "name": "crawling",
  "arguments": {
    "action": "spider",
    "url": "https://docs.example.com",
    "depth": 3,
    "maxPages": 100,
    "concurrency": 5
  }
}
```

---

### 4. `stealth` — Anti-Detection

**18 actions** for stealth mode with 12 anti-detection patches, TLS fingerprint impersonation, domain blocking, CAPTCHA detection, and proxy health monitoring.

| Action | Description | Key Parameters |
|---|---|---|
| `inject` | Inject all 12 stealth patches | `level?` |
| `test` | Run detection tests | `url?` |
| `fingerprint` | Get or randomize fingerprint | `randomize?` |
| `block_domains` | Block tracking domains | `domains` |
| `detect_captcha` | Detect CAPTCHA presence | — |
| `proxy_set` | Set proxy | `url` |
| `proxy_health` | Check proxy health | `url` |
| `tls_impersonate` | Impersonate TLS fingerprint | `browser?` |
| `antibot_detect` | Detect anti-bot type | — |
| `antibot_bypass` | Attempt bypass | — |

**Example — Full stealth setup:**

```json
{"name": "stealth", "arguments": {"action": "inject", "level": "maximum"}}
```

```json
{"name": "stealth", "arguments": {"action": "test", "url": "https://bot.sannysoft.com"}}
```

```json
{"name": "stealth", "arguments": {"action": "block_domains", "domains": ["google-analytics.com", "facebook.net", "doubleclick.net"]}}
```

---

### 5. `network` — Network Interception

**24 actions** for network interception, throttling, HAR recording, WebSocket capture, network logging, domain blocking, proxy management, and request queuing.

| Action | Description | Key Parameters |
|---|---|---|
| `throttle` | Throttle network speed | `profile` or `download`, `upload`, `latency` |
| `intercept` | Intercept matching requests | `pattern`, `action` |
| `block` | Block domains | `domains` |
| `offline` | Go offline | — |
| `online` | Restore connectivity | — |
| `har_start` | Start HAR recording | — |
| `har_stop` | Stop HAR recording | — |
| `har_export` | Export HAR to file | `path` |
| `ws_connect` | Connect to WebSocket | `url` |
| `ws_send` | Send WebSocket message | `message` |
| `ws_close` | Close WebSocket | — |
| `log_start` | Start network logging | — |
| `log_stop` | Stop network logging | — |
| `log_export` | Export network log | `path` |

**Example — Record HAR:**

```json
{"name": "network", "arguments": {"action": "har_start"}}
```

```json
{"name": "navigation", "arguments": {"action": "goto", "url": "https://example.com"}}
```

```json
{"name": "network", "arguments": {"action": "har_stop"}}
```

```json
{"name": "network", "arguments": {"action": "har_export", "path": "trace.har"}}
```

---

### 6. `crypto` — Cryptographic Operations

**12 actions** for AES-256-GCM encryption/decryption, PKCE challenge generation, TOTP codes, key derivation, and hashing.

| Action | Description | Key Parameters |
|---|---|---|
| `encrypt` | AES-256-GCM encryption | `plaintext`, `key` |
| `decrypt` | AES-256-GCM decryption | `ciphertext`, `key` |
| `derive_key` | PBKDF2-HMAC-SHA256 key derivation | `password`, `salt` |
| `generate_pkce` | Generate PKCE challenge pair | `method?` |
| `generate_totp` | Generate TOTP code | `secret`, `digits?`, `period?` |
| `verify_totp` | Verify a TOTP code | `code`, `secret`, `digits?`, `period?` |
| `hash` | Hash data | `data`, `algorithm?` |

**Example — PKCE flow:**

```json
{"name": "crypto", "arguments": {"action": "generate_pkce", "method": "S256"}}
```

Response:

```json
{"code_verifier": "dBjftJeZ4CVP-mB92K27uhbUJU1p1r_wW1gFWFOEjXk", "code_challenge": "E9Melhoa2OwvFrEMTJguCHaoeK1t8URWbuGJSstw-cM", "method": "S256"}
```

**Example — TOTP code:**

```json
{"name": "crypto", "arguments": {"action": "generate_totp", "secret": "JBSWY3DPEHPK3PXP", "digits": 6, "period": 30}}
```

Response:

```json
{"code": "482931", "remaining_seconds": 17}
```

---

### 7. `parser` — HTML Parsing & Analysis

**14 actions** for HTML parsing, accessibility tree construction, CSS/XPath queries, text/link extraction, and content transformation.

| Action | Description | Key Parameters |
|---|---|---|
| `accessibility_tree` | Parse HTML into accessibility tree | `html` |
| `query_selector` | Query elements from HTML | `html`, `selector` |
| `extract_text` | Extract text from HTML | `html`, `selector?` |
| `extract_links` | Extract links from HTML | `html`, `absolute?`, `baseUrl?` |
| `extract_images` | Extract images from HTML | `html`, `baseUrl?` |
| `extract_meta` | Extract meta tags | `html` |
| `to_markdown` | Convert HTML to Markdown | `html` |

**Example — Parse and query:**

```json
{
  "name": "parser",
  "arguments": {
    "action": "query_selector",
    "html": "<div><h1>Title</h1><p>Content</p></div>",
    "selector": "h1"
  }
}
```

Response:

```json
[{"tag": "h1", "text": "Title", "attributes": {}}]
```

---

### 8. `storage` — Encrypted Key-Value Store

**8 actions** for managing an AES-256-GCM encrypted key-value store for credentials, tokens, and session data.

| Action | Description | Key Parameters |
|---|---|---|
| `set` | Store an encrypted value | `key`, `value` |
| `get` | Retrieve a value | `key` |
| `delete` | Delete a key | `key` |
| `list` | List keys by prefix | `prefix?` |
| `exists` | Check if key exists | `key` |
| `clear` | Clear all stored data | — |

**Example — Credential storage:**

```json
{"name": "storage", "arguments": {"action": "set", "key": "api_token", "value": "sk-abc123..."}}
```

```json
{"name": "storage", "arguments": {"action": "get", "key": "api_token"}}
```

```json
{"name": "storage", "arguments": {"action": "list", "prefix": "api_"}}
```

---

### 9. `automation` — Rate Limiting & Retry

**8 actions** for controlling automation pacing, retry logic, scheduling, and pooling.

| Action | Description | Key Parameters |
|---|---|---|
| `rate_limit` | Configure rate limiting | `maxPerMinute`, `maxPerHour?`, `cooldownMs?` |
| `retry` | Configure retry behavior | `maxRetries`, `backoffMs?`, `backoffMultiplier?` |
| `schedule` | Schedule a task | `cron`, `command` |
| `pool` | Manage browser pool | `size`, `command?` |
| `pipeline` | Execute a multi-step pipeline | `steps` |
| `bench` | Benchmark a command | `command` |

**Example — Rate limiting for respectful scraping:**

```json
{
  "name": "automation",
  "arguments": {
    "action": "rate_limit",
    "maxPerMinute": 10,
    "maxPerHour": 200,
    "cooldownMs": 5000
  }
}
```

**Example — Data pipeline:**

```json
{
  "name": "automation",
  "arguments": {
    "action": "pipeline",
    "steps": [
      {"action": "goto", "url": "https://api.example.com/data"},
      {"action": "extract", "selector": "table tr"},
      {"action": "transform", "format": "csv"},
      {"action": "save", "path": "output.csv"}
    ]
  }
}
```

---

### 10. `data` — HTTP & Data Operations

**10 actions** for making HTTP requests, extracting links, building site graphs, and data transformation.

| Action | Description | Key Parameters |
|---|---|---|
| `http_get` | Make an HTTP GET request | `url`, `headers?` |
| `http_post` | Make an HTTP POST request | `url`, `body`, `headers?` |
| `http_put` | Make an HTTP PUT request | `url`, `body`, `headers?` |
| `http_delete` | Make an HTTP DELETE request | `url`, `headers?` |
| `links` | Extract links from current page | `filter?`, `absolute?` |
| `graph` | Build a link graph | `url`, `depth?` |
| `download` | Download a file | `url`, `path` |

**Example — HTTP GET:**

```json
{"name": "data", "arguments": {"action": "http_get", "url": "https://api.example.com/status"}}
```

---

### 11. `auth` — Authentication & WebAuthn

**12 actions** for managing virtual authenticators, passkeys, cookies, and session state.

| Action | Description | Key Parameters |
|---|---|---|
| `passkey_enable` | Enable virtual authenticator | — |
| `passkey_add` | Add a passkey credential | `rpId`, `credentialId`, `userHandle`, `privateKey` |
| `passkey_list` | List registered credentials | — |
| `passkey_log` | Get authenticator event log | `limit?` |
| `passkey_disable` | Disable authenticator | — |
| `passkey_remove` | Remove a credential | `credentialId` |
| `cookie_get` | Get a specific cookie | `name` |
| `cookie_get_all` | Get all cookies | — |
| `cookie_set` | Set a cookie | `name`, `value`, `domain?`, `path?` |
| `cookie_delete` | Delete a cookie | `name` |
| `cookie_clear` | Clear all cookies | — |

**Example — Passkey authentication flow:**

```json
{"name": "auth", "arguments": {"action": "passkey_enable"}}
```

```json
{
  "name": "auth",
  "arguments": {
    "action": "passkey_add",
    "rpId": "example.com",
    "credentialId": "cred_abc123",
    "userHandle": "user_456",
    "privateKey": "MIIEvQIBADANBgkq..."
  }
}
```

```json
{"name": "auth", "arguments": {"action": "passkey_list"}}
```

---

### 12. `accessibility` — Accessibility Auditing

**8 actions** for accessibility tree snapshots, auditing, and violation detection.

| Action | Description | Key Parameters |
|---|---|---|
| `snapshot` | Get accessibility tree snapshot | `filter?` |
| `tree` | Get full accessibility tree | — |
| `audit` | Run accessibility audit | `format?` |
| `violations` | List a11y violations | — |
| `score` | Get accessibility score | — |

**Example:**

```json
{"name": "accessibility", "arguments": {"action": "snapshot", "filter": "interactive"}}
```

---

### 13. `emulation` — Device & Environment

**14 actions** for emulating devices, viewports, timezone, locale, geolocation, and media types.

| Action | Description | Key Parameters |
|---|---|---|
| `device` | Emulate a device | `name` |
| `viewport` | Set viewport dimensions | `width`, `height` |
| `timezone` | Override timezone | `timezone` |
| `locale` | Override locale | `locale` |
| `geolocation` | Override geolocation | `latitude`, `longitude` |
| `media` | Override media type | `type` |
| `cpu_throttle` | Throttle CPU | `rate` |

**Example:**

```json
{"name": "emulation", "arguments": {"action": "device", "name": "iPhone 15 Pro"}}
```

---

### 14. `console` — Console & Logging

**6 actions** for capturing and managing browser console output.

| Action | Description | Key Parameters |
|---|---|---|
| `start` | Start console capture | — |
| `stop` | Stop console capture | — |
| `messages` | Get captured messages | `level?` |
| `clear` | Clear captured messages | — |

---

### 15. `coverage` — Code Coverage

**6 actions** for JavaScript/CSS code coverage analysis.

| Action | Description | Key Parameters |
|---|---|---|
| `start` | Start coverage collection | — |
| `stop` | Stop and get report | — |
| `report` | Generate coverage report | `format?` |

---

### 16. `performance` — Performance Metrics

**8 actions** for performance measurement, tracing, and benchmarking.

| Action | Description | Key Parameters |
|---|---|---|
| `metrics` | Get performance metrics | — |
| `trace_start` | Start performance tracing | — |
| `trace_stop` | Stop tracing and get data | `path?` |
| `bench` | Benchmark an operation | `command` |

---

### 17. `server` — HTTP Server Management

**6 actions** for managing the HTTP API server programmatically.

| Action | Description | Key Parameters |
|---|---|---|
| `start` | Start HTTP server | `port?`, `bind?` |
| `stop` | Stop HTTP server | — |
| `status` | Get server status | — |
| `info` | Get server info | — |

---

## Integration with AI Agents

### Claude (Anthropic)

OneCrawl integrates natively with Claude via MCP:

```bash
# Claude automatically discovers OneCrawl tools
# Simply ask Claude to interact with web pages:
# "Go to https://example.com and extract all the links"
# Claude will call navigation.goto, then scraping.links
```

### OpenAI GPT (via function calling)

Use the SSE transport to expose OneCrawl as an API:

```python
import openai
import requests

# Start OneCrawl MCP with SSE
# onecrawl mcp --transport sse --port 3001

def call_onecrawl(tool_name, arguments):
    response = requests.post("http://localhost:3001/call", json={
        "name": tool_name,
        "arguments": arguments
    })
    return response.json()

# Use with OpenAI function calling
tools = [{
    "type": "function",
    "function": {
        "name": "browser_navigate",
        "description": "Navigate to a URL",
        "parameters": {
            "type": "object",
            "properties": {"url": {"type": "string"}},
            "required": ["url"]
        }
    }
}]
```

### LangChain / LlamaIndex

```python
from langchain.tools import Tool

onecrawl_navigate = Tool(
    name="navigate",
    description="Navigate a browser to a URL",
    func=lambda url: call_onecrawl("navigation", {"action": "goto", "url": url})
)
```

---

## Tool Summary

| # | Super-Tool | Actions | Purpose |
|---|---|---|---|
| 1 | `navigation` | 32 | Browser navigation, interaction, capture |
| 2 | `scraping` | 28 | Content extraction, forms, structured data |
| 3 | `crawling` | 12 | Site crawling, sitemaps, snapshots |
| 4 | `stealth` | 18 | Anti-detection, fingerprinting, proxy |
| 5 | `network` | 24 | Interception, throttling, HAR, WebSocket |
| 6 | `crypto` | 12 | AES-256-GCM, PKCE, TOTP |
| 7 | `parser` | 14 | HTML parsing, accessibility tree |
| 8 | `storage` | 8 | Encrypted KV store |
| 9 | `automation` | 8 | Rate limiting, retry, pipelines |
| 10 | `data` | 10 | HTTP requests, link graphs |
| 11 | `auth` | 12 | WebAuthn, passkeys, cookies |
| 12 | `accessibility` | 8 | A11y auditing, snapshots |
| 13 | `emulation` | 14 | Device, viewport, timezone |
| 14 | `console` | 6 | Console log capture |
| 15 | `coverage` | 6 | Code coverage |
| 16 | `performance` | 8 | Metrics, tracing |
| 17 | `server` | 6 | HTTP server management |
| | **Total** | **421** | **17 super-tools** |
