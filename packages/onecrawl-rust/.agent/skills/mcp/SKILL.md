---
name: mcp
description: "Model Context Protocol server with 43 tools across 10 namespaces: crypto, parser, storage, navigation, scraping, crawling, stealth, data, automation, and passkeys."
---

# MCP Server Skill

OneCrawl exposes 43 tools via the Model Context Protocol (MCP). The server
supports stdio and SSE transports, auto-launches a headless browser on first
browser tool call, and uses typed Serialize structs for all responses.

## Quick Start

```bash
# Start MCP server (stdio transport, default)
onecrawl mcp

# Start with SSE transport
onecrawl mcp --transport sse
```

## Tool Naming Convention

Tools follow MCP best practices (spec 2025-11-25):
- Names use `namespace.action` format (e.g., `navigation.goto`, `scraping.css`)
- Names are 1-128 characters, case-sensitive
- Only ASCII letters, digits, underscore, hyphen, dot
- Each tool has a `description` and typed `inputSchema`

## Tools by Namespace

### crypto (4 tools)

| Tool | Input | Description |
|------|-------|-------------|
| `encrypt` | `{plaintext, password}` | AES-256-GCM encryption, returns base64 |
| `decrypt` | `{ciphertext, password}` | Decrypt base64 ciphertext |
| `generate_pkce` | -- | PKCE S256 challenge pair |
| `generate_totp` | `{secret}` | 6-digit TOTP from base32 secret |

### parser (4 tools)

| Tool | Input | Description |
|------|-------|-------------|
| `parse_accessibility_tree` | `{html}` | HTML to accessibility tree text |
| `query_selector` | `{html, selector}` | CSS selector query, returns JSON |
| `html_extract_text` | `{html}` | Extract visible text |
| `html_extract_links` | `{html}` | Extract links with href, text, is_external |

### storage (3 tools)

| Tool | Input | Description |
|------|-------|-------------|
| `store_set` | `{key, value}` | Set encrypted KV pair |
| `store_get` | `{key}` | Retrieve value by key |
| `store_list_keys` | -- | List all stored keys |

### navigation (11 tools)

| Tool | Input | Description |
|------|-------|-------------|
| `navigation.goto` | `{url, wait_ms?}` | Navigate browser to URL |
| `navigation.click` | `{selector}` | Click element by CSS selector |
| `navigation.type` | `{selector, text}` | Type text into element |
| `navigation.screenshot` | `{full_page?}` | Take screenshot (base64 PNG) |
| `navigation.pdf` | `{landscape?, scale?}` | Export PDF (base64) |
| `navigation.back` | -- | Go back in history |
| `navigation.forward` | -- | Go forward in history |
| `navigation.reload` | -- | Reload page |
| `navigation.wait` | `{selector, timeout_ms?}` | Wait for selector |
| `navigation.evaluate` | `{expression}` | Evaluate JavaScript |
| `navigation.cookies` | `{action, name?, value?, domain?}` | Cookie management |

### scraping (9 tools)

| Tool | Input | Description |
|------|-------|-------------|
| `scraping.css` | `{selector}` | CSS query on live DOM |
| `scraping.xpath` | `{expression}` | XPath query |
| `scraping.find_text` | `{text, tag?}` | Find elements by text |
| `scraping.text` | `{selector?}` | Extract text from live page |
| `scraping.html` | `{selector?}` | Extract HTML |
| `scraping.markdown` | `{selector?}` | Extract as Markdown |
| `scraping.structured_data` | -- | JSON-LD, OpenGraph, Twitter Card |
| `scraping.detect_forms` | -- | Detect forms and fields |
| `scraping.fill_form` | `{fields, submit?}` | Fill and submit forms |

### crawling (5 tools)

| Tool | Input | Description |
|------|-------|-------------|
| `crawling.spider` | `{start_url, max_depth?, max_pages?}` | Crawl website |
| `crawling.robots` | `{base_url, user_agent?, path?}` | Parse robots.txt |
| `crawling.sitemap` | `{base_url, entries}` | Generate XML sitemap |
| `crawling.snapshot` | `{label}` | Take labeled DOM snapshot |
| `crawling.compare` | `{label_a, label_b}` | Compare two snapshots |

### stealth (5 tools)

| Tool | Input | Description |
|------|-------|-------------|
| `stealth.inject` | -- | Inject all stealth patches |
| `stealth.test` | -- | Bot detection test (score + results) |
| `stealth.fingerprint` | `{user_agent?}` | Apply browser fingerprint |
| `stealth.block_domains` | `{domains?, category?}` | Block ad/tracker domains |
| `stealth.detect_captcha` | -- | Detect CAPTCHAs on page |

### data (5 tools)

| Tool | Input | Description |
|------|-------|-------------|
| `data.pipeline` | `{pipeline, data}` | Multi-step data pipeline |
| `data.http_get` | `{url, headers?}` | HTTP GET via browser |
| `data.http_post` | `{url, body?, content_type?}` | HTTP POST via browser |
| `data.links` | `{base_url?}` | Extract links as directed edges |
| `data.graph` | `{edges}` | Analyze link graph |

### automation (2 tools)

| Tool | Input | Description |
|------|-------|-------------|
| `automation.rate_limit` | `{max_per_second?, max_per_minute?}` | Rate limiter status |
| `automation.retry` | `{url, operation, payload?}` | Enqueue for retry |

### passkey (6 tools)

| Tool | Input | Description |
|------|-------|-------------|
| `auth_passkey_enable` | `{protocol?, transport?}` | Enable virtual authenticator |
| `auth_passkey_create` | `{credential_id, rp_id, user_handle?}` | Add credential |
| `auth_passkey_list` | -- | List credentials |
| `auth_passkey_log` | -- | Operation log |
| `auth_passkey_disable` | -- | Disable authenticator |
| `auth_passkey_remove` | `{credential_id}` | Remove credential |

## Error Handling

Following MCP spec best practices:
- **Protocol errors**: JSON-RPC errors for unknown tools, malformed requests
- **Tool execution errors**: `isError: true` with actionable feedback
- All errors include descriptive messages for LLM self-correction

## Architecture

- Built on `rmcp` crate with `#[tool]` and `#[tool_router]` proc macros
- Browser state shared via `SharedBrowser` (Arc<Mutex<BrowserState>>)
- Auto-launches headless browser on first browser tool call
- All responses use typed Serialize structs (no `json!()` macros)
- `json_ok()` and `text_ok()` helpers for consistent response formatting
- `ensure_page()` handles lazy browser initialization
- Rate limiter and retry queue initialized on first use
