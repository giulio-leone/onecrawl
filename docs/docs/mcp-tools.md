---
sidebar_position: 4
title: MCP Tools Reference
---

# MCP Tools Reference

OneCrawl exposes **43 MCP tools** across **10 namespaces** for seamless integration with AI agents, coding assistants, and agentic workflows.

Start the MCP server:

```bash
onecrawl mcp --transport stdio
```

Or use SSE transport for remote connections:

```bash
onecrawl mcp --transport sse --port 3001
```

Tools are invoked via the [Model Context Protocol](https://modelcontextprotocol.io/) JSON-RPC interface. Each tool call follows this structure:

```json
{
  "method": "tools/call",
  "params": {
    "name": "navigation.goto",
    "arguments": {
      "url": "https://example.com"
    }
  }
}
```

---

## auth.* — Authentication & WebAuthn

6 tools for managing virtual authenticators and passkeys.

| Tool | Description | Parameters |
|---|---|---|
| `auth.passkey_enable` | Enable the virtual authenticator on the browser | — |
| `auth.passkey_add` | Add a passkey credential | `rpId: string`, `credentialId: string`, `userHandle: string`, `privateKey: string` |
| `auth.passkey_list` | List all registered passkey credentials | — |
| `auth.passkey_log` | Get the authenticator event log | `limit?: number` |
| `auth.passkey_disable` | Disable the virtual authenticator | — |
| `auth.passkey_remove` | Remove a specific passkey credential | `credentialId: string` |

**Example — Enable authenticator and add a credential:**

```json
// Step 1: Enable
{ "name": "auth.passkey_enable", "arguments": {} }

// Step 2: Add credential
{
  "name": "auth.passkey_add",
  "arguments": {
    "rpId": "example.com",
    "credentialId": "cred_abc123",
    "userHandle": "user_456",
    "privateKey": "MIIEvQIBADANBgkq..."
  }
}

// Step 3: Verify
{ "name": "auth.passkey_list", "arguments": {} }
```

---

## navigation.* — Browser Navigation & Interaction

10 tools for page navigation, interaction, and capture.

| Tool | Description | Parameters |
|---|---|---|
| `navigation.goto` | Navigate to a URL | `url: string`, `waitUntil?: string` |
| `navigation.click` | Click an element | `selector: string` |
| `navigation.type` | Type text into an element | `selector: string`, `text: string`, `delay?: number` |
| `navigation.screenshot` | Take a screenshot | `fullPage?: boolean`, `selector?: string`, `format?: string` |
| `navigation.pdf` | Export page as PDF | `landscape?: boolean`, `scale?: number` |
| `navigation.back` | Go back in history | — |
| `navigation.forward` | Go forward in history | — |
| `navigation.reload` | Reload the page | — |
| `navigation.wait` | Wait for a condition | `selector?: string`, `timeout?: number`, `ms?: number` |
| `navigation.evaluate` | Evaluate JavaScript | `expression: string` |

**Example — Navigate and capture:**

```json
{
  "name": "navigation.goto",
  "arguments": { "url": "https://example.com", "waitUntil": "networkidle" }
}
```

```json
{
  "name": "navigation.screenshot",
  "arguments": { "fullPage": true, "format": "png" }
}
```

---

## scraping.* — Data Extraction

10 tools for extracting content from web pages.

| Tool | Description | Parameters |
|---|---|---|
| `scraping.css` | Extract elements by CSS selector | `selector: string`, `attribute?: string`, `limit?: number` |
| `scraping.xpath` | Extract elements by XPath | `expression: string`, `limit?: number` |
| `scraping.find_text` | Find elements containing text | `text: string`, `exact?: boolean` |
| `scraping.text` | Get all visible text from the page | `selector?: string` |
| `scraping.html` | Get HTML content | `selector?: string`, `outer?: boolean` |
| `scraping.markdown` | Convert page to Markdown | `selector?: string` |
| `scraping.structured` | Extract structured data using a schema | `schema: object`, `url?: string` |
| `scraping.stream` | Stream extraction for large pages | `selector: string`, `batchSize?: number` |
| `scraping.detect_forms` | Detect all forms on the page | — |
| `scraping.fill_form` | Fill and optionally submit a form | `formSelector: string`, `fields: object`, `submit?: boolean` |

**Example — Structured extraction:**

```json
{
  "name": "scraping.structured",
  "arguments": {
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
{ "name": "scraping.detect_forms", "arguments": {} }
```

```json
{
  "name": "scraping.fill_form",
  "arguments": {
    "formSelector": "#login-form",
    "fields": {
      "#email": "user@example.com",
      "#password": "s3cret"
    },
    "submit": true
  }
}
```

---

## crawling.* — Site Crawling

5 tools for crawling, sitemaps, and site snapshots.

| Tool | Description | Parameters |
|---|---|---|
| `crawling.spider` | Crawl a site following links | `url: string`, `depth?: number`, `maxPages?: number`, `concurrency?: number` |
| `crawling.robots` | Fetch and parse robots.txt | `url: string` |
| `crawling.sitemap` | Fetch and parse XML sitemaps | `url: string` |
| `crawling.snapshot` | Create a full DOM snapshot | `url?: string` |
| `crawling.compare` | Compare two snapshots | `before: string`, `after: string` |

**Example — Crawl a documentation site:**

```json
{
  "name": "crawling.spider",
  "arguments": {
    "url": "https://docs.example.com",
    "depth": 3,
    "maxPages": 100,
    "concurrency": 5
  }
}
```

---

## stealth.* — Anti-Detection

5 tools for stealth mode and anti-bot bypass.

| Tool | Description | Parameters |
|---|---|---|
| `stealth.inject` | Inject stealth patches into the browser | `level?: string` |
| `stealth.test` | Run detection tests against the current page | `url?: string` |
| `stealth.fingerprint` | Get or randomize the browser fingerprint | `randomize?: boolean` |
| `stealth.block_domains` | Block requests to specified domains | `domains: string[]` |
| `stealth.detect_captcha` | Detect CAPTCHA presence on the page | — |

**Example — Full stealth setup:**

```json
// Inject patches
{
  "name": "stealth.inject",
  "arguments": { "level": "maximum" }
}

// Verify undetectable
{
  "name": "stealth.test",
  "arguments": { "url": "https://bot.sannysoft.com" }
}

// Block tracking domains
{
  "name": "stealth.block_domains",
  "arguments": {
    "domains": ["google-analytics.com", "facebook.net", "doubleclick.net"]
  }
}
```

---

## data.* — Data & Networking

5 tools for HTTP requests, link analysis, and data pipelines.

| Tool | Description | Parameters |
|---|---|---|
| `data.pipeline` | Execute a multi-step data pipeline | `steps: object[]` |
| `data.http_get` | Make an HTTP GET request | `url: string`, `headers?: object` |
| `data.http_post` | Make an HTTP POST request | `url: string`, `body: object`, `headers?: object` |
| `data.links` | Extract all links from the page | `filter?: string`, `absolute?: boolean` |
| `data.graph` | Build a link graph from a starting URL | `url: string`, `depth?: number` |

**Example — Data pipeline:**

```json
{
  "name": "data.pipeline",
  "arguments": {
    "steps": [
      { "action": "goto", "url": "https://api.example.com/data" },
      { "action": "extract", "selector": "table tr" },
      { "action": "transform", "format": "csv" },
      { "action": "save", "path": "output.csv" }
    ]
  }
}
```

---

## automation.* — Rate Limiting & Retry

2 tools for controlling automation pacing.

| Tool | Description | Parameters |
|---|---|---|
| `automation.rate_limit` | Configure rate limiting | `maxPerMinute: number`, `maxPerHour?: number`, `cooldownMs?: number` |
| `automation.retry` | Configure retry behavior | `maxRetries: number`, `backoffMs?: number`, `backoffMultiplier?: number` |

**Example:**

```json
{
  "name": "automation.rate_limit",
  "arguments": {
    "maxPerMinute": 10,
    "maxPerHour": 200,
    "cooldownMs": 5000
  }
}
```

```json
{
  "name": "automation.retry",
  "arguments": {
    "maxRetries": 3,
    "backoffMs": 1000,
    "backoffMultiplier": 2.0
  }
}
```

---

## Crypto Tools

4 standalone tools for cryptographic operations.

| Tool | Description | Parameters |
|---|---|---|
| `encrypt` | Encrypt data with AES-256-GCM | `plaintext: string`, `key: string` |
| `decrypt` | Decrypt AES-256-GCM ciphertext | `ciphertext: string`, `key: string` |
| `generate_pkce` | Generate a PKCE challenge pair | `method?: string` |
| `generate_totp` | Generate a TOTP code | `secret: string`, `digits?: number`, `period?: number` |

**Example — PKCE flow:**

```json
{ "name": "generate_pkce", "arguments": { "method": "S256" } }
// Returns: { "code_verifier": "...", "code_challenge": "...", "method": "S256" }
```

**Example — TOTP code:**

```json
{
  "name": "generate_totp",
  "arguments": { "secret": "JBSWY3DPEHPK3PXP", "digits": 6, "period": 30 }
}
// Returns: { "code": "482931", "remaining_seconds": 17 }
```

---

## Parser & Storage Tools

7 tools for HTML parsing, accessibility analysis, and key-value storage.

| Tool | Description | Parameters |
|---|---|---|
| `parse_accessibility_tree` | Parse HTML into an accessibility tree | `html: string` |
| `query_selector` | Query elements from HTML | `html: string`, `selector: string` |
| `html_extract_text` | Extract text from HTML | `html: string`, `selector?: string` |
| `html_extract_links` | Extract links from HTML | `html: string`, `absolute?: boolean`, `baseUrl?: string` |
| `store_set` | Set a value in the encrypted KV store | `key: string`, `value: string` |
| `store_get` | Get a value from the encrypted KV store | `key: string` |
| `store_list` | List all keys in the store | `prefix?: string` |

**Example — Parse and query HTML:**

```json
{
  "name": "query_selector",
  "arguments": {
    "html": "<div><h1>Title</h1><p>Content</p></div>",
    "selector": "h1"
  }
}
// Returns: [{ "tag": "h1", "text": "Title", "attributes": {} }]
```

**Example — Encrypted storage:**

```json
{ "name": "store_set", "arguments": { "key": "api_token", "value": "sk-abc123..." } }
{ "name": "store_get", "arguments": { "key": "api_token" } }
{ "name": "store_list", "arguments": { "prefix": "api_" } }
```

---

## Tool Summary

| Namespace | Tools | Purpose |
|---|---|---|
| `auth.*` | 6 | WebAuthn / Passkey management |
| `navigation.*` | 10 | Browser navigation, interaction, capture |
| `scraping.*` | 10 | Content extraction, form filling |
| `crawling.*` | 5 | Site crawling, sitemaps, snapshots |
| `stealth.*` | 5 | Anti-detection, fingerprinting |
| `data.*` | 5 | HTTP, link graphs, data pipelines |
| `automation.*` | 2 | Rate limiting, retry logic |
| Crypto | 4 | AES-256-GCM, PKCE, TOTP |
| Parser/Storage | 7 | HTML parsing, accessibility, KV store |
| **Total** | **51** | |
