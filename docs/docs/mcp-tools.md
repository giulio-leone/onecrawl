---
sidebar_position: 4
title: MCP Tools Reference
---

# MCP Tools Reference

OneCrawl exposes **17 MCP super-tools** providing **421 actions** for seamless integration with AI agents, coding assistants, and agentic workflows.

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
    "name": "browser",
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

### 1. `browser` — Browser Navigation, DOM & Interaction

**112 actions** for page navigation, element interaction, DOM manipulation, cookies, screenshots, PDF export, iframes, tabs, and page lifecycle management.

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
| `cookie_get` | Get a specific cookie | `name` |
| `cookie_get_all` | Get all cookies | — |
| `cookie_set` | Set a cookie | `name`, `value`, `domain?`, `path?` |
| `cookie_delete` | Delete a cookie | `name` |
| `cookie_clear` | Clear all cookies | — |
| `viewport` | Set viewport dimensions | `width`, `height` |
| `device` | Emulate a device | `name` |
| `timezone` | Override timezone | `timezone` |
| `locale` | Override locale | `locale` |
| `geolocation` | Override geolocation | `latitude`, `longitude` |
| `media` | Override media type | `type` |
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
| `console_start` | Start console capture | — |
| `console_stop` | Stop console capture | — |
| `console_messages` | Get captured messages | `level?` |
| `coverage_start` | Start code coverage collection | — |
| `coverage_stop` | Stop and get coverage report | — |

**Example — Navigate, fill form, and screenshot:**

```json
{"name": "browser", "arguments": {"action": "goto", "url": "https://example.com", "waitUntil": "networkidle"}}
```

```json
{"name": "browser", "arguments": {"action": "fill", "selector": "#email", "value": "user@example.com"}}
```

```json
{"name": "browser", "arguments": {"action": "screenshot", "fullPage": true, "format": "png"}}
```

---

### 2. `agent` — AI Agent Automation & Workflows

**111 actions** for AI agent orchestration, goal-driven workflows, tool registration, computer-use integration, planning, reflection, and autonomous multi-step task execution.

| Action | Description | Key Parameters |
|---|---|---|
| `run` | Execute a full agent task | `goal`, `maxSteps?`, `model?` |
| `step` | Execute a single agent step | `instruction`, `context?` |
| `plan` | Generate a multi-step plan | `goal`, `constraints?` |
| `observe` | Observe current page state | `strategy?` |
| `act` | Perform an action from observation | `action`, `params?` |
| `reflect` | Reflect on progress toward goal | `goal`, `history` |
| `decide` | Make a branching decision | `options`, `context` |
| `loop` | Run an observe-act loop | `goal`, `maxIterations?`, `stopCondition?` |
| `goal_set` | Set the current agent goal | `goal` |
| `goal_check` | Check if goal is satisfied | — |
| `tool_call` | Call a registered tool | `tool`, `args` |
| `tool_register` | Register a custom tool | `name`, `schema`, `handler` |
| `tool_list` | List registered tools | — |
| `screenshot_act` | Screenshot-based action (computer use) | `instruction` |
| `dom_act` | DOM-based action | `instruction`, `selector?` |
| `extract_act` | Extract data via agent reasoning | `schema`, `url?` |
| `navigate_act` | Navigate via agent reasoning | `destination` |
| `fill_form_act` | Fill a form via agent reasoning | `formGoal` |
| `workflow_create` | Create a reusable workflow | `name`, `steps` |
| `workflow_run` | Run a saved workflow | `name`, `params?` |
| `workflow_list` | List saved workflows | — |
| `context_set` | Set agent context variables | `vars` |
| `context_get` | Get agent context | — |
| `history_get` | Get agent action history | `limit?` |
| `history_clear` | Clear action history | — |

**Example — Run a goal-driven agent task:**

```json
{
  "name": "agent",
  "arguments": {
    "action": "run",
    "goal": "Find the top 5 trending repositories on GitHub and extract their names and star counts",
    "maxSteps": 20
  }
}
```

**Example — Observe-act loop:**

```json
{"name": "agent", "arguments": {"action": "loop", "goal": "Add all items to cart and proceed to checkout", "maxIterations": 15, "stopCondition": "checkout page loaded"}}
```

---

### 3. `data` — Extraction, Parsing & Structured Data

**27 actions** for content extraction using CSS/XPath selectors, structured data parsing (JSON-LD, OpenGraph, Twitter Card), form detection, table extraction, and HTML-to-Markdown conversion.

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
| `accessibility_tree` | Parse into accessibility tree | `html?` |
| `query_selector` | Query elements from parsed HTML | `html`, `selector` |
| `to_markdown` | Convert HTML string to Markdown | `html` |
| `http_get` | Make an HTTP GET request | `url`, `headers?` |
| `http_post` | Make an HTTP POST request | `url`, `body`, `headers?` |
| `download` | Download a file | `url`, `path` |

**Example — Structured extraction from Hacker News:**

```json
{
  "name": "data",
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
{"name": "data", "arguments": {"action": "detect_forms"}}
```

```json
{
  "name": "data",
  "arguments": {
    "action": "fill_form",
    "formSelector": "#login-form",
    "fields": {"#email": "user@example.com", "#password": "s3cret"},
    "submit": true
  }
}
```

---

### 4. `automate` — Task Automation & Workflows

**27 actions** for automation pacing, retry logic, scheduling, browser pooling, multi-step pipelines, checkpoints, conditional branching, and parallel execution.

| Action | Description | Key Parameters |
|---|---|---|
| `rate_limit` | Configure rate limiting | `maxPerMinute`, `maxPerHour?`, `cooldownMs?` |
| `retry` | Configure retry behavior | `maxRetries`, `backoffMs?`, `backoffMultiplier?` |
| `schedule` | Schedule a task | `cron`, `command` |
| `pool` | Manage browser pool | `size`, `command?` |
| `pipeline` | Execute a multi-step pipeline | `steps` |
| `bench` | Benchmark a command | `command` |
| `checkpoint` | Save execution checkpoint | `name`, `data?` |
| `restore` | Restore from checkpoint | `name` |
| `sequence` | Run steps sequentially | `steps` |
| `parallel` | Run steps in parallel | `steps`, `concurrency?` |
| `condition` | Conditional execution | `test`, `then`, `else?` |
| `loop` | Loop with condition | `steps`, `while?`, `maxIterations?` |
| `wait_for` | Wait for a condition | `condition`, `timeout?` |
| `timeout` | Set execution timeout | `ms`, `command` |
| `on_error` | Set error handler | `handler` |
| `batch` | Process items in batches | `items`, `batchSize`, `action` |

**Example — Rate limiting for respectful scraping:**

```json
{
  "name": "automate",
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
  "name": "automate",
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

### 5. `stealth` — Anti-Detection & Fingerprinting

**25 actions** for stealth mode with anti-detection patches, TLS fingerprint impersonation, domain blocking, CAPTCHA detection, proxy management, canvas/WebGL spoofing, and user-agent rotation.

| Action | Description | Key Parameters |
|---|---|---|
| `inject` | Inject all stealth patches | `level?` |
| `test` | Run detection tests | `url?` |
| `fingerprint` | Get or randomize fingerprint | `randomize?` |
| `block_domains` | Block tracking domains | `domains` |
| `detect_captcha` | Detect CAPTCHA presence | — |
| `proxy_set` | Set proxy | `url`, `auth?` |
| `proxy_health` | Check proxy health | `url` |
| `proxy_rotate` | Rotate to next proxy | `pool?` |
| `tls_impersonate` | Impersonate TLS fingerprint | `browser?` |
| `antibot_detect` | Detect anti-bot type | — |
| `antibot_bypass` | Attempt bypass | — |
| `user_agent` | Set or randomize user-agent | `value?`, `randomize?` |
| `canvas_spoof` | Spoof canvas fingerprint | `noise?` |
| `webgl_spoof` | Spoof WebGL fingerprint | `vendor?`, `renderer?` |
| `timezone_spoof` | Spoof timezone | `timezone` |
| `language_spoof` | Spoof navigator language | `languages` |
| `plugin_spoof` | Spoof navigator plugins | `plugins` |

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

### 6. `computer` — OS-Level Interaction & Input Simulation

**24 actions** for OS-level mouse control, keyboard input, screen capture, window management, clipboard access, and system-level automation.

| Action | Description | Key Parameters |
|---|---|---|
| `mouse_move` | Move mouse to coordinates | `x`, `y` |
| `mouse_click` | Click at coordinates | `x`, `y`, `button?` |
| `mouse_double_click` | Double-click at coordinates | `x`, `y` |
| `mouse_drag` | Drag from one point to another | `fromX`, `fromY`, `toX`, `toY` |
| `mouse_scroll` | Scroll at coordinates | `x`, `y`, `deltaX?`, `deltaY` |
| `key_press` | Press a single key | `key` |
| `key_type` | Type a string | `text`, `delay?` |
| `key_combo` | Press a key combination | `keys` |
| `screen_capture` | Capture the screen | `region?`, `format?` |
| `screen_size` | Get screen dimensions | — |
| `window_list` | List open windows | — |
| `window_focus` | Focus a window | `title` or `id` |
| `window_resize` | Resize a window | `id`, `width`, `height` |
| `window_move` | Move a window | `id`, `x`, `y` |
| `clipboard_read` | Read clipboard contents | — |
| `clipboard_write` | Write to clipboard | `text` |
| `find_element` | Find UI element on screen | `description` |
| `wait_for_element` | Wait for UI element to appear | `description`, `timeout?` |

**Example — Click a button found by screen coordinates:**

```json
{"name": "computer", "arguments": {"action": "screen_capture"}}
```

```json
{"name": "computer", "arguments": {"action": "mouse_click", "x": 450, "y": 320}}
```

```json
{"name": "computer", "arguments": {"action": "key_type", "text": "Hello, World!", "delay": 50}}
```

---

### 7. `secure` — Encryption, Auth & Credential Management

**21 actions** for AES-256-GCM encryption/decryption, PKCE challenge generation, TOTP codes, key derivation, hashing, digital signatures, passkey/WebAuthn management, and certificate handling.

| Action | Description | Key Parameters |
|---|---|---|
| `encrypt` | AES-256-GCM encryption | `plaintext`, `key` |
| `decrypt` | AES-256-GCM decryption | `ciphertext`, `key` |
| `derive_key` | PBKDF2-HMAC-SHA256 key derivation | `password`, `salt` |
| `generate_pkce` | Generate PKCE challenge pair | `method?` |
| `generate_totp` | Generate TOTP code | `secret`, `digits?`, `period?` |
| `verify_totp` | Verify a TOTP code | `code`, `secret`, `digits?`, `period?` |
| `hash` | Hash data | `data`, `algorithm?` |
| `sign` | Sign data with private key | `data`, `privateKey`, `algorithm?` |
| `verify` | Verify a signature | `data`, `signature`, `publicKey` |
| `passkey_enable` | Enable virtual authenticator | — |
| `passkey_add` | Add a passkey credential | `rpId`, `credentialId`, `userHandle`, `privateKey` |
| `passkey_list` | List registered credentials | — |
| `passkey_log` | Get authenticator event log | `limit?` |
| `passkey_disable` | Disable authenticator | — |
| `passkey_remove` | Remove a credential | `credentialId` |
| `cert_generate` | Generate self-signed certificate | `cn`, `days?` |
| `cert_verify` | Verify a certificate chain | `cert`, `ca?` |

**Example — PKCE flow:**

```json
{"name": "secure", "arguments": {"action": "generate_pkce", "method": "S256"}}
```

Response:

```json
{"code_verifier": "dBjftJeZ4CVP-mB92K27uhbUJU1p1r_wW1gFWFOEjXk", "code_challenge": "E9Melhoa2OwvFrEMTJguCHaoeK1t8URWbuGJSstw-cM", "method": "S256"}
```

**Example — Passkey authentication flow:**

```json
{"name": "secure", "arguments": {"action": "passkey_enable"}}
```

```json
{
  "name": "secure",
  "arguments": {
    "action": "passkey_add",
    "rpId": "example.com",
    "credentialId": "cred_abc123",
    "userHandle": "user_456",
    "privateKey": "MIIEvQIBADANBgkq..."
  }
}
```

---

### 8. `vault` — Encrypted Credential Storage

**9 actions** for managing an AES-256-GCM encrypted key-value store for credentials, tokens, API keys, and session data with import/export and key rotation.

| Action | Description | Key Parameters |
|---|---|---|
| `set` | Store an encrypted value | `key`, `value` |
| `get` | Retrieve a value | `key` |
| `delete` | Delete a key | `key` |
| `list` | List keys by prefix | `prefix?` |
| `exists` | Check if key exists | `key` |
| `clear` | Clear all stored data | — |
| `export` | Export vault (encrypted) | `path`, `passphrase` |
| `import` | Import vault from file | `path`, `passphrase` |
| `rotate` | Rotate encryption key | `newKey` |

**Example — Credential storage:**

```json
{"name": "vault", "arguments": {"action": "set", "key": "api_token", "value": "sk-abc123..."}}
```

```json
{"name": "vault", "arguments": {"action": "get", "key": "api_token"}}
```

```json
{"name": "vault", "arguments": {"action": "list", "prefix": "api_"}}
```

---

### 9. `plugins` — Plugin Lifecycle & Execution

**9 actions** for registering, enabling, disabling, configuring, and executing plugins that extend OneCrawl functionality.

| Action | Description | Key Parameters |
|---|---|---|
| `register` | Register a new plugin | `name`, `path` or `url` |
| `unregister` | Remove a plugin | `name` |
| `list` | List all plugins | `status?` |
| `execute` | Execute a plugin action | `name`, `action`, `args?` |
| `enable` | Enable a plugin | `name` |
| `disable` | Disable a plugin | `name` |
| `config` | Get or set plugin config | `name`, `config?` |
| `status` | Get plugin status | `name` |
| `reload` | Reload a plugin | `name` |

**Example — Register and execute a plugin:**

```json
{"name": "plugins", "arguments": {"action": "register", "name": "custom-extractor", "path": "./plugins/extractor.js"}}
```

```json
{"name": "plugins", "arguments": {"action": "execute", "name": "custom-extractor", "action": "run", "args": {"url": "https://example.com"}}}
```

---

### 10. `reactor` — Event Observer Pattern

**8 actions** for event-driven automation using the observer pattern — register listeners, emit events, and manage reactive workflows.

| Action | Description | Key Parameters |
|---|---|---|
| `on` | Register an event listener | `event`, `handler` |
| `off` | Remove an event listener | `event`, `handlerId` |
| `once` | Register a one-time listener | `event`, `handler` |
| `emit` | Emit an event | `event`, `data?` |
| `list_listeners` | List registered listeners | `event?` |
| `clear` | Clear all listeners | `event?` |
| `pause` | Pause event processing | — |
| `resume` | Resume event processing | — |

**Example — React to navigation events:**

```json
{"name": "reactor", "arguments": {"action": "on", "event": "page:navigate", "handler": "log_url"}}
```

```json
{"name": "reactor", "arguments": {"action": "emit", "event": "page:navigate", "data": {"url": "https://example.com"}}}
```

---

### 11. `durable` — Crash-Resilient Sessions

**8 actions** for creating crash-resilient browser sessions with automatic checkpointing, recovery, and state persistence across restarts.

| Action | Description | Key Parameters |
|---|---|---|
| `create` | Create a durable session | `name`, `config?` |
| `resume` | Resume a crashed/stopped session | `name` |
| `checkpoint` | Save session checkpoint | `name`, `label?` |
| `destroy` | Destroy a session and its data | `name` |
| `list` | List all durable sessions | `status?` |
| `status` | Get session status | `name` |
| `export` | Export session state | `name`, `path` |
| `import` | Import session state | `path`, `name?` |

**Example — Create and checkpoint a durable session:**

```json
{"name": "durable", "arguments": {"action": "create", "name": "long-scrape", "config": {"autoCheckpoint": true, "intervalMs": 30000}}}
```

```json
{"name": "durable", "arguments": {"action": "checkpoint", "name": "long-scrape", "label": "page-50-complete"}}
```

```json
{"name": "durable", "arguments": {"action": "resume", "name": "long-scrape"}}
```

---

### 12. `events` — Pub/Sub, Webhooks & SSE

**8 actions** for publish/subscribe messaging, webhook registration, and Server-Sent Events (SSE) streaming between OneCrawl and external systems.

| Action | Description | Key Parameters |
|---|---|---|
| `subscribe` | Subscribe to a topic | `topic`, `handler` |
| `unsubscribe` | Unsubscribe from a topic | `topic`, `subscriptionId` |
| `publish` | Publish a message to a topic | `topic`, `data` |
| `webhook_register` | Register a webhook endpoint | `url`, `events`, `secret?` |
| `webhook_remove` | Remove a webhook | `webhookId` |
| `sse_connect` | Connect to an SSE stream | `url` |
| `sse_close` | Close an SSE connection | `connectionId` |
| `list` | List subscriptions and webhooks | — |

**Example — Webhook for crawl completion:**

```json
{
  "name": "events",
  "arguments": {
    "action": "webhook_register",
    "url": "https://api.example.com/hooks/crawl-done",
    "events": ["crawl:complete", "crawl:error"],
    "secret": "whsec_abc123"
  }
}
```

```json
{"name": "events", "arguments": {"action": "publish", "topic": "crawl:complete", "data": {"pages": 150, "duration": 45}}}
```

---

### 13. `studio` — Visual Workflow Builder

**8 actions** for creating, editing, validating, and executing visual automation workflows with a step-based builder interface.

| Action | Description | Key Parameters |
|---|---|---|
| `create` | Create a new workflow | `name`, `description?` |
| `load` | Load an existing workflow | `name` |
| `save` | Save the current workflow | `name?` |
| `run` | Execute a workflow | `name`, `params?` |
| `add_step` | Add a step to a workflow | `type`, `config` |
| `remove_step` | Remove a step by index | `index` |
| `validate` | Validate workflow correctness | `name?` |
| `export` | Export workflow as JSON | `name`, `path?` |

**Example — Build and run a scraping workflow:**

```json
{"name": "studio", "arguments": {"action": "create", "name": "news-scraper", "description": "Scrape top stories from news sites"}}
```

```json
{"name": "studio", "arguments": {"action": "add_step", "type": "navigate", "config": {"url": "https://news.ycombinator.com"}}}
```

```json
{"name": "studio", "arguments": {"action": "add_step", "type": "extract", "config": {"selector": ".athing .titleline > a", "output": "titles"}}}
```

```json
{"name": "studio", "arguments": {"action": "run", "name": "news-scraper"}}
```

---

### 14. `perf` — Performance Audit & Visual Regression

**8 actions** for performance measurement, Lighthouse auditing, visual regression testing (VRT), performance budgets, and tracing.

| Action | Description | Key Parameters |
|---|---|---|
| `metrics` | Get performance metrics | — |
| `trace_start` | Start performance tracing | — |
| `trace_stop` | Stop tracing and get data | `path?` |
| `lighthouse` | Run a Lighthouse audit | `url`, `categories?` |
| `vrt_capture` | Capture a VRT baseline | `name`, `selector?` |
| `vrt_compare` | Compare against VRT baseline | `name`, `threshold?` |
| `budget_check` | Check performance budget | `budgets` |
| `report` | Generate performance report | `format?`, `path?` |

**Example — Lighthouse audit:**

```json
{"name": "perf", "arguments": {"action": "lighthouse", "url": "https://example.com", "categories": ["performance", "accessibility"]}}
```

**Example — Visual regression test:**

```json
{"name": "perf", "arguments": {"action": "vrt_capture", "name": "homepage-hero"}}
```

```json
{"name": "perf", "arguments": {"action": "vrt_compare", "name": "homepage-hero", "threshold": 0.01}}
```

---

### 15. `memory` — Agent Memory Store & Recall

**6 actions** for persisting and recalling agent memories, enabling AI agents to maintain context across sessions and tasks.

| Action | Description | Key Parameters |
|---|---|---|
| `store` | Store a memory | `key`, `value`, `tags?` |
| `recall` | Recall a specific memory | `key` |
| `search` | Search memories by tags or content | `query?`, `tags?`, `limit?` |
| `delete` | Delete a memory | `key` |
| `list` | List all memories | `tags?`, `limit?` |
| `clear` | Clear all memories | — |

**Example — Store and recall agent context:**

```json
{"name": "memory", "arguments": {"action": "store", "key": "login_creds_page", "value": "Login form is at /auth/signin, uses #email and #password fields", "tags": ["auth", "forms"]}}
```

```json
{"name": "memory", "arguments": {"action": "search", "tags": ["auth"], "limit": 5}}
```

---

### 16. `crawl` — Spider, Robots.txt & Sitemap

**5 actions** for spidering sites following links, parsing robots.txt, fetching XML sitemaps, creating DOM snapshots, and comparing page states.

| Action | Description | Key Parameters |
|---|---|---|
| `spider` | Crawl a site following links | `url`, `depth?`, `maxPages?`, `concurrency?` |
| `robots` | Fetch and parse robots.txt | `url` |
| `sitemap` | Fetch and parse XML sitemaps | `url` |
| `snapshot` | Create a full DOM snapshot | `url?` |
| `compare` | Compare two snapshots | `before`, `after` |

**Example — Crawl a documentation site:**

```json
{
  "name": "crawl",
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

### 17. `orchestrator` — Multi-Device Control

**5 actions** for coordinating automation across multiple browser instances or devices, broadcasting commands, and synchronizing state.

| Action | Description | Key Parameters |
|---|---|---|
| `add_device` | Add a device to the pool | `name`, `config` |
| `remove_device` | Remove a device | `name` |
| `broadcast` | Send a command to all devices | `action`, `args?` |
| `sync` | Synchronize state across devices | `source?` |
| `status` | Get status of all devices | — |

**Example — Multi-device testing:**

```json
{"name": "orchestrator", "arguments": {"action": "add_device", "name": "mobile", "config": {"device": "iPhone 15 Pro"}}}
```

```json
{"name": "orchestrator", "arguments": {"action": "add_device", "name": "desktop", "config": {"viewport": {"width": 1920, "height": 1080}}}}
```

```json
{"name": "orchestrator", "arguments": {"action": "broadcast", "action": "goto", "args": {"url": "https://example.com"}}}
```

```json
{"name": "orchestrator", "arguments": {"action": "status"}}
```

---

## Integration with AI Agents

### Claude (Anthropic)

OneCrawl integrates natively with Claude via MCP:

```bash
# Claude automatically discovers OneCrawl tools
# Simply ask Claude to interact with web pages:
# "Go to https://example.com and extract all the links"
# Claude will call browser(goto), then data(links)
```

### Cursor

Add OneCrawl to your `.cursor/mcp.json` and Cursor will discover all 17 super-tools automatically. Use natural language to invoke them:

```bash
# "Navigate to the docs site and take a screenshot"
# Cursor calls: browser(goto) → browser(screenshot)
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
        "description": "Navigate to a URL using the browser super-tool",
        "parameters": {
            "type": "object",
            "properties": {"url": {"type": "string"}},
            "required": ["url"]
        }
    }
}]

# Map function calls to OneCrawl super-tools
def handle_tool_call(name, args):
    if name == "browser_navigate":
        return call_onecrawl("browser", {"action": "goto", "url": args["url"]})
```

---

## Integration Examples

### Python — Full workflow

```python
import requests
import json

ONECRAWL_URL = "http://localhost:3001/call"

def onecrawl(tool: str, **kwargs) -> dict:
    """Call an OneCrawl super-tool."""
    resp = requests.post(ONECRAWL_URL, json={"name": tool, "arguments": kwargs})
    return resp.json()

# 1. Enable stealth mode
onecrawl("stealth", action="inject", level="maximum")

# 2. Navigate to target
onecrawl("browser", action="goto", url="https://example.com", waitUntil="networkidle")

# 3. Extract structured data
result = onecrawl("data", action="structured", schema={
    "products": {
        "_selector": ".product-card",
        "name": "h3",
        "price": ".price",
        "url": "a@href"
    }
})

# 4. Take a screenshot for verification
onecrawl("browser", action="screenshot", fullPage=True, format="png")

# 5. Store results in vault
onecrawl("vault", action="set", key="last_scrape", value=json.dumps(result))

print(json.dumps(result, indent=2))
```

### Node.js — Full workflow

```javascript
const ONECRAWL_URL = "http://localhost:3001/call";

async function onecrawl(tool, args) {
  const resp = await fetch(ONECRAWL_URL, {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify({ name: tool, arguments: args }),
  });
  return resp.json();
}

async function main() {
  // 1. Enable stealth mode
  await onecrawl("stealth", { action: "inject", level: "maximum" });

  // 2. Navigate to target
  await onecrawl("browser", {
    action: "goto",
    url: "https://example.com",
    waitUntil: "networkidle",
  });

  // 3. Extract all links
  const links = await onecrawl("data", { action: "links", absolute: true });

  // 4. Crawl the site
  const crawlResult = await onecrawl("crawl", {
    action: "spider",
    url: "https://example.com",
    depth: 2,
    maxPages: 50,
  });

  // 5. Run performance audit
  const perfReport = await onecrawl("perf", {
    action: "lighthouse",
    url: "https://example.com",
    categories: ["performance", "accessibility"],
  });

  console.log("Links:", links);
  console.log("Pages crawled:", crawlResult);
  console.log("Performance:", perfReport);
}

main().catch(console.error);
```

### LangChain / LlamaIndex

```python
from langchain.tools import Tool

onecrawl_browser = Tool(
    name="browser",
    description="Navigate and interact with web pages",
    func=lambda url: onecrawl("browser", action="goto", url=url)
)

onecrawl_extract = Tool(
    name="data_extract",
    description="Extract structured data from the current page",
    func=lambda schema: onecrawl("data", action="structured", schema=schema)
)

onecrawl_crawl = Tool(
    name="crawl_site",
    description="Spider a website following links",
    func=lambda url: onecrawl("crawl", action="spider", url=url, depth=2)
)
```

---

## Tool Summary

| # | Super-Tool | Actions | Purpose |
|---|---|---|---|
| 1 | `browser` | 112 | Navigation, DOM, cookies, screenshots, tabs, network |
| 2 | `agent` | 111 | AI agent automation, workflows, computer use |
| 3 | `data` | 27 | Extraction, parsing, structured data, HTTP |
| 4 | `automate` | 27 | Task automation, workflows, checkpoints |
| 5 | `stealth` | 25 | Anti-detection, fingerprinting, evasion |
| 6 | `computer` | 24 | OS-level interaction, input simulation |
| 7 | `secure` | 21 | Encryption, auth, credential management |
| 8 | `vault` | 9 | Encrypted credential storage |
| 9 | `plugins` | 9 | Plugin lifecycle, execution |
| 10 | `reactor` | 8 | Event observer pattern |
| 11 | `durable` | 8 | Crash-resilient sessions |
| 12 | `events` | 8 | Pub/sub, webhooks, SSE |
| 13 | `studio` | 8 | Visual workflow builder |
| 14 | `perf` | 8 | Performance audit, VRT |
| 15 | `memory` | 6 | Agent memory store/recall |
| 16 | `crawl` | 5 | Spider, robots.txt, sitemap |
| 17 | `orchestrator` | 5 | Multi-device control |
| | **Total** | **421** | **17 super-tools** |
