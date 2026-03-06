# OneCrawl MCP API Reference

> **98 tools across 17 namespaces** for browser automation, web scraping, anti-detection, and AI-driven computer use.

---

## Quick Start

OneCrawl exposes an MCP (Model Context Protocol) server over **stdio**. Connect via any MCP-compatible client:

```json
{
  "mcpServers": {
    "onecrawl": {
      "command": "onecrawl-mcp",
      "args": [],
      "env": {
        "ONECRAWL_STORE_PATH": "./store.db",
        "ONECRAWL_STORE_PASSWORD": "your-secret"
      }
    }
  }
}
```

A headless Chromium browser is launched automatically on the first CDP tool call. All tools return JSON via MCP `CallToolResult`.

---

## Tool Namespaces

| Namespace | Tools | Description |
|-----------|-------|-------------|
| [computer.*](#computer) | 3 | AI Computer Use Protocol — act, observe, batch |
| [smart.*](#smart) | 3 | Self-Healing Element Resolution — fuzzy find, click, fill |
| [navigation.*](#navigation) | 11 | Browser Control — goto, click, type, screenshot, snapshot… |
| [agent.*](#agent) | 21 | Agentic Capabilities — chains, iframes, recording, iOS, safety… |
| [scraping.*](#scraping) | 11 | Data Extraction — CSS, XPath, text, markdown, forms, streaming… |
| [pool.*](#pool) | 2 | Multi-Browser Orchestration — list, status |
| [stealth.*](#stealth) | 5 | Anti-Detection — patches, fingerprint, domain blocking, CAPTCHA |
| [crawling.*](#crawling) | 5 | Web Crawling — spider, robots.txt, sitemap, DOM snapshots |
| [data.*](#data) | 5 | Data Pipeline — HTTP, link graph, pipeline transforms |
| [crypto.*](#crypto) | 4 | Encryption & TOTP — AES-256-GCM, PKCE, TOTP |
| [parser.*](#parser) | 4 | HTML Parsing — a11y tree, selectors, text, links |
| [storage.*](#storage) | 3 | Encrypted Key-Value Storage |
| [auth.*](#auth) | 6 | WebAuthn / Passkeys — virtual authenticator simulation |
| [automation.*](#automation) | 2 | Rate Limiting & Retry Queues |
| [memory.*](#memory) | 6 | Agent Memory — persistent cross-session memory, domain strategies |
| [workflow.*](#workflow) | 2 | Workflow DSL — JSON workflow definitions with steps, conditionals, loops |
| [net.*](#net) | 5 | Network Intelligence — API discovery, SDK generation, mock servers |

---

<a id="computer"></a>
## computer.* — AI Computer Use Protocol

The computer-use protocol lets AI agents interact with web pages through a unified act → observe loop.

---

### computer.act

Execute a browser action and return the page observation.

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `action` | object | ✅ | Action to perform. JSON object with a `"type"` field. |
| `include_screenshot` | boolean | | Include screenshot in observation (default: false) |

**Action Types:**

| Type | Fields | Description |
|------|--------|-------------|
| `click` | `x`, `y` / `selector` / `ref` | Click by coordinates, CSS selector, or `@ref` |
| `type` | `text` | Type text into focused element |
| `key` | `key` | Press a key (e.g. `"Enter"`, `"Tab"`) |
| `scroll` | `x`, `y`, `delta_x`, `delta_y` | Scroll the page |
| `navigate` | `url` | Navigate to a URL |
| `wait` | `ms` | Wait for a duration |
| `screenshot` | — | Take a screenshot |
| `observe` | `include_screenshot` | Get page observation without action |
| `evaluate` | `expression` | Evaluate JavaScript expression |
| `fill` | `selector`, `value` | Fill an input by selector |
| `select` | `selector`, `value` | Select a dropdown option |
| `drag` | `from_x`, `from_y`, `to_x`, `to_y` | Drag from one point to another |
| `done` | `result` | Signal task completion |
| `fail` | `reason` | Signal task failure |

**Returns:**
```json
{
  "success": true,
  "observation": {
    "url": "https://example.com",
    "title": "Example Page",
    "snapshot": "[e1] button \"Submit\"\n[e2] input[type=text] @e2 \"Search\"",
    "interactive_count": 12,
    "screenshot": "base64...",
    "viewport": { "width": 1280, "height": 720 }
  },
  "elapsed_ms": 145
}
```

**Example — Login flow:**
```json
{
  "action": { "type": "navigate", "url": "https://app.example.com/login" }
}
```
```json
{
  "action": { "type": "fill", "selector": "#email", "value": "user@example.com" }
}
```
```json
{
  "action": { "type": "fill", "selector": "#password", "value": "secret" }
}
```
```json
{
  "action": { "type": "click", "selector": "button[type=submit]" }
}
```

---

### computer.observe

Observe current browser state without taking any action. Pure observation for AI planning.

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `include_screenshot` | boolean | | Include base64 screenshot in observation |

**Returns:**
```json
{
  "url": "https://example.com",
  "title": "Example Page",
  "snapshot": "[e1] button \"Submit\"...",
  "interactive_count": 8,
  "viewport": { "width": 1280, "height": 720 }
}
```

---

### computer.batch

Execute a sequence of browser actions and return observations after each step. Efficient for multi-step workflows.

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `actions` | array\<object\> | ✅ | List of actions. Each is a JSON object with a `"type"` field. |
| `include_screenshots` | boolean | | Include screenshots between actions (default: false) |
| `stop_on_error` | boolean | | Stop on first error (default: true) |

**Returns:**
```json
{
  "total": 3,
  "executed": 3,
  "results": [
    { "success": true, "observation": { "url": "...", "title": "..." } },
    { "success": true, "observation": { "url": "...", "title": "..." } },
    { "success": true, "observation": { "url": "...", "title": "..." } }
  ]
}
```

---

<a id="smart"></a>
## smart.* — Self-Healing Element Resolution

Smart tools use fuzzy matching, ARIA roles, and multiple strategies to find elements — resilient to DOM changes.

---

### smart.find

Find elements using fuzzy text, ARIA roles, attributes, or CSS selectors. Returns ranked matches with confidence scores.

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `query` | string | ✅ | Fuzzy text, CSS selector, or element description to search for |

**Returns:**
```json
{
  "query": "submit button",
  "matches": [
    { "selector": "button.submit-btn", "confidence": 0.95, "strategy": "aria-label" },
    { "selector": "input[type=submit]", "confidence": 0.78, "strategy": "text-content" }
  ],
  "count": 2
}
```

**Example — Find a search box:**
```json
{ "query": "search input" }
```

---

### smart.click

Find the best matching element using fuzzy text, ARIA roles, or CSS selectors, then click it.

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `query` | string | ✅ | Fuzzy text, CSS selector, or element description to click |

**Returns:**
```json
{
  "clicked": "button.submit-btn",
  "confidence": 0.95,
  "strategy": "aria-label"
}
```

---

### smart.fill

Find an input element using fuzzy text, placeholder, or CSS selector, then type the given value into it.

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `query` | string | ✅ | Fuzzy text, CSS selector, or element description of the input to fill |
| `value` | string | ✅ | Value to type into the matched input |

**Returns:**
```json
{
  "filled": "input#email",
  "value_length": 16,
  "confidence": 0.92,
  "strategy": "placeholder"
}
```

---

<a id="navigation"></a>
## navigation.* — Browser Control

Core browser control tools. A headless browser is launched automatically on first call.

---

### navigation.goto

Navigate the browser to a URL. Launches a headless browser on first call.

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `url` | string | ✅ | URL to navigate to |

**Returns:** `"navigated to https://example.com — title: Example"`

---

### navigation.click

Click an element on the page by CSS selector or `@ref` (e.g. `@e1` from snapshot).

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `selector` | string | ✅ | CSS selector of element to click |

> **Tip:** Use `@ref` notation from `navigation.snapshot` (e.g. `@e1`) for stable element targeting.

**Returns:** `"clicked @e1"`

---

### navigation.type

Type text into an input element identified by CSS selector or `@ref`.

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `selector` | string | ✅ | CSS selector of target input element |
| `text` | string | ✅ | Text to type |

**Returns:** `"typed 12 chars into #search"`

---

### navigation.screenshot

Take a screenshot of the current page as base64-encoded PNG. Optionally target a specific element or capture the full scrollable page.

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `selector` | string | | CSS selector for element screenshot (omit for viewport) |
| `full_page` | boolean | | If true, capture the full scrollable page |

**Returns:** Base64 PNG image content (`Content::image`).

---

### navigation.pdf

Export the current page as a PDF document. Returns base64-encoded PDF data.

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `print_background` | boolean | | Print background graphics (default: false) |
| `format` | string | | Paper format: `"A4"`, `"Letter"`, etc. (default: `"A4"`) |
| `landscape` | boolean | | Landscape orientation (default: false) |

**Returns:** `"pdf exported (45230 bytes, base64 length 60308)"`

---

### navigation.back

Navigate back in browser history.

**Parameters:** None

**Returns:** `"navigated back"`

---

### navigation.forward

Navigate forward in browser history.

**Parameters:** None

**Returns:** `"navigated forward"`

---

### navigation.reload

Reload the current page.

**Parameters:** None

**Returns:** `"page reloaded"`

---

### navigation.wait

Wait for a CSS selector or `@ref` to appear in the DOM within an optional timeout.

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `selector` | string | ✅ | CSS selector to wait for |
| `timeout_ms` | integer | | Timeout in milliseconds (default: 30000) |

**Returns:** `"selector .loaded found"`

---

### navigation.evaluate

Evaluate arbitrary JavaScript in the browser page context. Returns the result as JSON.

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `js` | string | ✅ | JavaScript code to evaluate in the page context |

**Returns:** JSON result of the evaluated expression.

**Example:**
```json
{ "js": "document.querySelectorAll('a').length" }
```

---

### navigation.snapshot

Take an AI-optimized accessibility snapshot of the page. Returns element refs (`@e1`, `@e2`…) that can be used with click, fill, type commands.

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `interactive_only` | boolean | | Only include interactive elements (buttons, links, inputs) |
| `cursor` | boolean | | Include cursor-interactive elements (cursor:pointer, onclick, tabindex) |
| `compact` | boolean | | Remove empty structural elements for minimal output |
| `depth` | integer | | Max DOM depth to include |
| `selector` | string | | CSS selector to scope snapshot to a subtree |

**Returns:**
```json
{
  "snapshot": "html\n  body\n    nav\n      a @e1 \"Home\"\n      a @e2 \"About\"\n    main\n      h1 \"Welcome\"\n      button @e3 \"Sign In\"",
  "refs": { "e1": "a.nav-home", "e2": "a.nav-about", "e3": "button.sign-in" },
  "total": 42,
  "stats": {
    "lines": 15,
    "chars": 420,
    "estimated_tokens": 105,
    "total_refs": 42,
    "interactive_refs": 8
  }
}
```

> **Tip:** Use `interactive_only: true` to get a focused snapshot of only actionable elements, drastically reducing token usage.

---

<a id="agent"></a>
## agent.* — Agentic Capabilities

Advanced agent tools for chaining, screencasting, iframe handling, remote browsers, iOS automation, and session safety.

---

### agent.execute_chain

Execute multiple commands in sequence. Supported commands: `navigation.goto`, `navigation.click`, `navigation.type`, `navigation.wait`, `navigation.evaluate`, `navigation.snapshot`, `scraping.css`, `scraping.text`.

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `commands` | array\<ChainCommand\> | ✅ | List of commands to execute in sequence |
| `stop_on_error` | boolean | | Stop on first error (default: true) |

**ChainCommand schema:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `tool` | string | ✅ | Tool name to execute (e.g. `"navigation.click"`) |
| `args` | object | ✅ | Arguments as JSON object |

**Returns:**
```json
{
  "results": [
    { "tool": "navigation.goto", "success": true, "data": { "url": "...", "title": "..." } },
    { "tool": "navigation.click", "success": true, "data": { "clicked": "@e3" } }
  ],
  "completed": 2,
  "total": 2
}
```

**Example — Navigate and extract:**
```json
{
  "commands": [
    { "tool": "navigation.goto", "args": { "url": "https://example.com" } },
    { "tool": "navigation.snapshot", "args": { "interactive_only": true } },
    { "tool": "scraping.text", "args": { "selector": "main" } }
  ]
}
```

---

### agent.element_screenshot

Take a screenshot of a specific element by CSS selector or `@ref`. Returns base64 PNG with element bounds.

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `selector` | string | ✅ | CSS selector or `@ref` (e.g. `@e1`) of the element to screenshot |

**Returns:**
```json
{
  "image": "base64...",
  "bounds": { "x": 100, "y": 200, "width": 300, "height": 50 }
}
```

---

### agent.api_capture_start

Inject a fetch/XHR interceptor to capture all API calls made by the page. Call `agent.api_capture_summary` to read the log.

**Parameters:** None

**Returns:**
```json
{ "active": true, "entries": 0 }
```

---

### agent.api_capture_summary

Get a summary of all network API calls (fetch/XHR) captured since `api_capture_start`. Per-page, resets on navigation.

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `clear` | boolean | | Clear the captured log after reading (default: false) |

**Returns:**
```json
{
  "total": 5,
  "requests": [
    { "type": "fetch", "method": "GET", "url": "/api/users", "status": 200, "contentType": "application/json", "timestamp": 1700000000 }
  ]
}
```

---

### agent.iframe_list

List all iframes on the current page with metadata (src, name, id, dimensions, sandbox).

**Parameters:** None

**Returns:**
```json
{
  "total": 2,
  "iframes": [
    { "src": "https://widget.example.com", "name": "payment", "id": "payment-frame", "width": 400, "height": 300 }
  ]
}
```

---

### agent.iframe_snapshot

Take an accessibility snapshot inside a specific iframe by index. Use `agent.iframe_list` first to discover available iframes.

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `index` | integer | ✅ | Zero-based index of the iframe to snapshot |
| `interactive_only` | boolean | | Only include interactive elements |
| `compact` | boolean | | Remove empty structural elements for minimal output |

**Returns:**
```json
{
  "snapshot": "body\n  input @f0e1 \"Card number\"...",
  "refs": { "f0e1": "input#card-number" },
  "total": 6,
  "iframe_index": 0
}
```

---

### agent.connect_remote

Connect to a remote CDP WebSocket endpoint (e.g. Browserbase, BrowserCloud). Subsequent tools will use the remote browser.

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `ws_url` | string | ✅ | WebSocket URL of the remote CDP endpoint (e.g. `ws://127.0.0.1:9222/devtools/browser/...`) |
| `headers` | object | | Optional HTTP headers for the WebSocket handshake |

**Returns:**
```json
{
  "connected": true,
  "ws_url": "wss://connect.browserbase.com/...",
  "info": "Remote browser connected. Subsequent tools will use this session."
}
```

---

### agent.safety_policy_set

Set or update the safety policy for this session. Controls allowed/blocked domains, URL patterns, commands, rate limits, and confirmation requirements.

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `allowed_domains` | array\<string\> | | Allowed domains (if empty, all domains allowed) |
| `blocked_domains` | array\<string\> | | Blocked domains |
| `blocked_url_patterns` | array\<string\> | | Blocked URL patterns (glob-style with `*` wildcards) |
| `max_actions` | integer | | Maximum actions per session (0 = unlimited) |
| `confirm_form_submit` | boolean | | Require confirmation for form submissions |
| `confirm_file_upload` | boolean | | Require confirmation for file uploads |
| `blocked_commands` | array\<string\> | | Blocked commands |
| `allowed_commands` | array\<string\> | | Allowed commands (if empty, all non-blocked allowed) |
| `rate_limit_per_minute` | integer | | Rate limit: max actions per minute (0 = unlimited) |
| `policy_file` | string | | Path to a JSON policy file to load (overrides other fields) |

**Returns:**
```json
{
  "status": "policy_set",
  "policy": { "allowed_domains": ["example.com"], "max_actions": 100, "..." : "..." }
}
```

---

### agent.safety_status

Get current safety state: active policy, action counts, rate limit window, and all constraints.

**Parameters:** None

**Returns:**
```json
{
  "status": "active",
  "actions_taken": 12,
  "max_actions": 100,
  "rate_limit_per_minute": 30,
  "allowed_domains": ["example.com"]
}
```

---

### agent.skills_list

List all available skill packages (built-in and discovered). Returns name, version, description, and tool list for each skill.

**Parameters:** None

**Returns:**
```json
[
  {
    "name": "linkedin-auth",
    "version": "1.0.0",
    "description": "LinkedIn authentication and session management",
    "tools": [{ "name": "login", "description": "Login to LinkedIn" }],
    "requires": [],
    "author": "onecrawl",
    "source": "built-in"
  }
]
```

---

### agent.screencast_start

Start live browser screencast via CDP `Page.startScreencast`. Configure format, quality, resolution, and frame rate.

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `format` | string | | Image format: `"jpeg"` or `"png"` |
| `quality` | integer | | Compression quality 0–100 (jpeg only) |
| `max_width` | integer | | Maximum width in pixels |
| `max_height` | integer | | Maximum height in pixels |
| `every_nth_frame` | integer | | Capture every N-th frame |

**Returns:**
```json
{
  "status": "started",
  "format": "jpeg",
  "quality": 60,
  "max_width": 1280,
  "max_height": 720,
  "every_nth_frame": 1
}
```

---

### agent.screencast_stop

Stop the active browser screencast.

**Parameters:** None

**Returns:**
```json
{ "status": "stopped" }
```

---

### agent.screencast_frame

Capture a single screencast frame as base64-encoded image data.

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `format` | string | | Image format: `"jpeg"` or `"png"` (default: `"jpeg"`) |
| `quality` | integer | | Compression quality 0–100 |

**Returns:**
```json
{
  "image": "base64...",
  "format": "jpeg",
  "size": 24576
}
```

---

### agent.recording_start

Start recording the browser session. Frames are captured and stored in memory until stopped.

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `output` | string | | Output file path (e.g. `"recording.webm"`) |
| `fps` | integer | | Frames per second |

**Returns:**
```json
{
  "status": "recording",
  "output": "recording.webm",
  "fps": 5
}
```

---

### agent.recording_stop

Stop recording, save frames as image sequence, and return the output path.

**Parameters:** None

**Returns:**
```json
{
  "status": "saved",
  "frames": 42,
  "path": "/tmp/recording-frames/"
}
```

---

### agent.recording_status

Get the current recording state (idle, recording, or stopped) with frame count.

**Parameters:** None

**Returns:**
```json
{
  "status": "recording",
  "frames": 15,
  "fps": 5,
  "output": "recording.webm"
}
```

---

### agent.ios_devices

List available iOS simulator devices (via `xcrun simctl`). Returns device name, UDID, platform, and version.

**Parameters:** None

**Returns:**
```json
{
  "devices": [
    { "name": "iPhone 15 Pro", "udid": "A1B2C3...", "platform": "iOS", "version": "17.2" }
  ],
  "count": 3
}
```

---

### agent.ios_connect

Start an iOS Safari session via WebDriverAgent. Session persists for subsequent `ios_*` calls.

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `wda_url` | string | | WebDriverAgent URL (default: `http://localhost:8100`) |
| `udid` | string | | Device UDID (auto-detect if omitted) |
| `bundle_id` | string | | Bundle ID to automate (default: `com.apple.mobilesafari`) |

**Returns:**
```json
{ "connected": true, "session_id": "abc-123" }
```

---

### agent.ios_navigate

Navigate Mobile Safari to a URL. Requires an active iOS session.

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `url` | string | ✅ | URL to navigate to in Mobile Safari |

**Returns:**
```json
{ "navigated": true, "url": "https://example.com" }
```

---

### agent.ios_tap

Tap at screen coordinates on the iOS device. Requires an active iOS session.

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `x` | number | ✅ | X coordinate |
| `y` | number | ✅ | Y coordinate |

**Returns:**
```json
{ "tapped": true, "x": 195.0, "y": 400.0 }
```

---

### agent.ios_screenshot

Take a screenshot of the iOS device screen. Returns base64-encoded image data. Requires an active iOS session.

**Parameters:** None

**Returns:**
```json
{
  "format": "png",
  "size": 102400,
  "data": "base64..."
}
```

---

<a id="scraping"></a>
## scraping.* — Data Extraction

Extract data from live browser pages using CSS, XPath, text search, structured data, and streaming schemas.

---

### scraping.css

Query the live DOM with a CSS selector. Supports `::text` and `::attr(name)` pseudo-elements. Returns JSON array of matching elements.

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `selector` | string | ✅ | CSS selector to query (supports `::text`, `::attr(name)` pseudo-elements) |

**Example selectors:**
- `h1::text` — extract text from all `<h1>` elements
- `a::attr(href)` — extract `href` attribute from all links
- `.product .price::text` — scoped text extraction

**Returns:** JSON array of matched elements/values.

---

### scraping.xpath

Query the live DOM with an XPath expression. Returns JSON array of matching elements.

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `expression` | string | ✅ | XPath expression to evaluate |

**Returns:** JSON array of matched elements.

---

### scraping.find_text

Find elements by visible text content. Optionally restrict search to a specific HTML tag.

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `text` | string | ✅ | Text content to search for |
| `tag` | string | | Optional HTML tag to constrain search (e.g. `"a"`, `"button"`) |

**Returns:** JSON array of matched elements.

---

### scraping.text

Extract visible text content from the live page, optionally scoped to a CSS selector.

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `selector` | string | | CSS selector (default: `"body"`) |

**Returns:** JSON with extracted text content.

---

### scraping.html

Extract raw HTML from the live page, optionally scoped to a CSS selector.

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `selector` | string | | CSS selector (default: `"body"`) |

**Returns:** JSON with extracted HTML content.

---

### scraping.markdown

Extract page content as clean Markdown, optionally scoped to a CSS selector.

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `selector` | string | | CSS selector (default: `"body"`) |

**Returns:** JSON with Markdown content.

---

### scraping.structured

Extract structured data from the page including JSON-LD, OpenGraph, Twitter Card, and meta tags.

**Parameters:** None

**Returns:**
```json
{
  "json_ld": [{ "@type": "Product", "name": "Widget" }],
  "opengraph": { "og:title": "Example", "og:image": "..." },
  "twitter_card": { "twitter:card": "summary" },
  "meta": { "description": "..." }
}
```

---

### scraping.stream

Schema-based extraction of repeating items using field rules with optional pagination support.

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `item_selector` | string | ✅ | CSS selector for repeating item container |
| `fields` | string | ✅ | Field extraction rules as JSON array: `[{"name":"title","selector":"h2","extract":"text"}]` |
| `pagination` | string | | Optional pagination config: `{"next_selector":".next","max_pages":5,"delay_ms":1000}` |

**Example:**
```json
{
  "item_selector": ".product-card",
  "fields": "[{\"name\":\"title\",\"selector\":\"h2\",\"extract\":\"text\"},{\"name\":\"price\",\"selector\":\".price\",\"extract\":\"text\"},{\"name\":\"url\",\"selector\":\"a\",\"extract\":\"attr:href\"}]",
  "pagination": "{\"next_selector\":\".next-page\",\"max_pages\":3,\"delay_ms\":1000}"
}
```

**Returns:** JSON array of extracted items.

---

### scraping.detect_forms

Detect all forms on the current page and enumerate their fields, types, and attributes.

**Parameters:** None

**Returns:** JSON array of form descriptors with field metadata.

---

### scraping.fill_form

Fill form fields by selector-to-value mapping and optionally submit the form.

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `form_selector` | string | ✅ | CSS selector of the form element |
| `data` | string | ✅ | JSON object mapping field selectors to values, e.g. `{"#email":"a@b.com"}` |
| `submit` | boolean | | If true, submit the form after filling |

**Returns:** JSON with fill results.

---

### scraping.snapshot_diff

Compute a line-level unified diff between two accessibility snapshot texts. Returns additions, removals, unchanged count, and the unified diff output.

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `before` | string | ✅ | Accessibility snapshot text before (from `navigation.snapshot`) |
| `after` | string | ✅ | Accessibility snapshot text after (from `navigation.snapshot`) |

**Returns:**
```json
{
  "additions": 3,
  "removals": 1,
  "unchanged": 28,
  "diff": "--- before\n+++ after\n@@ -5,3 +5,5 @@..."
}
```

---

<a id="pool"></a>
## pool.* — Multi-Browser Orchestration

Manage a pool of browser instances for parallel crawling and testing.

---

### pool.list

List all browser instances in the pool with their ID, status, current URL, and creation time.

**Parameters:** None

**Returns:**
```json
{
  "instances": [
    { "id": "b1", "status": "idle", "url": "about:blank", "created_at": "..." }
  ],
  "count": 2
}
```

---

### pool.status

Get pool statistics: current size, max size, idle count, and busy count.

**Parameters:** None

**Returns:**
```json
{
  "size": 3,
  "max_size": 10,
  "idle": 2,
  "busy": 1
}
```

---

<a id="stealth"></a>
## stealth.* — Anti-Detection

Bypass bot detection with stealth patches, fingerprint spoofing, domain blocking, and CAPTCHA detection.

---

### stealth.inject

Inject comprehensive stealth anti-bot patches into the browser page. Returns list of applied patches.

**Parameters:** None

**Returns:**
```json
{
  "patches_applied": 12,
  "patches": [
    "navigator.webdriver",
    "chrome.runtime",
    "permissions.query",
    "plugins.length",
    "languages",
    "platform",
    "hardwareConcurrency",
    "deviceMemory",
    "webgl.vendor",
    "webgl.renderer",
    "canvas.fingerprint",
    "audio.fingerprint"
  ]
}
```

---

### stealth.test

Run bot detection tests on the current page. Returns a detection score and detailed test results.

**Parameters:** None

**Returns:** JSON with detection score (0–100, lower = more human-like) and detailed test results.

---

### stealth.fingerprint

Generate and apply a realistic browser fingerprint with configurable user-agent to evade bot detection.

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `user_agent` | string | | Optional user-agent override |

**Returns:**
```json
{
  "user_agent": "Mozilla/5.0 (Windows NT 10.0; Win64; x64)...",
  "platform": "Win32"
}
```

---

### stealth.block_domains

Block network requests to specified domains or a built-in category such as ads, trackers, or social widgets.

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `domains` | array\<string\> | | List of domains to block |
| `category` | string | | Block an entire built-in category: `"ads"`, `"trackers"`, `"social"` |

> **Note:** Provide either `domains` or `category`, not both.

**Returns:** `"42 domains blocked"`

---

### stealth.detect_captcha

Detect CAPTCHAs on the current page. Returns the CAPTCHA type, provider, and confidence score.

**Parameters:** None

**Returns:**
```json
{
  "detected": true,
  "type": "recaptcha_v2",
  "provider": "Google",
  "confidence": 0.98,
  "selector": ".g-recaptcha"
}
```

---

<a id="crawling"></a>
## crawling.* — Web Crawling

Crawl websites, check robots.txt, generate sitemaps, and compare DOM snapshots.

---

### crawling.spider

Crawl a website starting from one or more seed URLs. Follows links with configurable depth, domain, and pattern filters.

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `start_urls` | array\<string\> | ✅ | Starting URL(s) to crawl |
| `max_depth` | integer | | Maximum link depth (default: 2) |
| `max_pages` | integer | | Maximum pages to visit (default: 50) |
| `same_domain_only` | boolean | | Stay on the same domain only (default: true) |
| `url_patterns` | array\<string\> | | URL patterns to include (regex) |
| `exclude_patterns` | array\<string\> | | URL patterns to exclude (regex) |
| `delay_ms` | integer | | Delay between requests in ms (default: 500) |

**Returns:**
```json
{
  "summary": {
    "total_pages": 23,
    "total_links": 156,
    "errors": 1,
    "domains": ["example.com"]
  },
  "pages_crawled": 23
}
```

---

### crawling.robots

Fetch and parse robots.txt for a domain. Optionally test if a specific path is allowed for a given user-agent.

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `base_url` | string | ✅ | Base URL to fetch robots.txt from |
| `path` | string | | Path to check (e.g. `"/admin"`) |
| `user_agent` | string | | User-agent string (default: `"*"`) |

**Returns:**
```json
{
  "sitemaps": ["https://example.com/sitemap.xml"],
  "crawl_delay": 2.0,
  "path_allowed": false
}
```

---

### crawling.sitemap

Generate a standards-compliant XML sitemap from a list of URL entries with priority and changefreq.

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `base_url` | string | ✅ | Base URL for the sitemap |
| `entries` | string | ✅ | URLs to include as JSON array: `[{"url":"...","priority":0.8}]` |
| `default_changefreq` | string | | Default change frequency (e.g. `"weekly"`) |

**Returns:** XML sitemap string.

---

### crawling.snapshot

Take a labeled DOM snapshot of the current page for later comparison with `crawling.compare`.

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `label` | string | ✅ | Label to identify this snapshot for later comparison |

**Returns:** `"snapshot 'before' saved"`

---

### crawling.compare

Compare two previously taken DOM snapshots by label and return a structured diff report.

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `before` | string | ✅ | Label of the "before" snapshot |
| `after` | string | ✅ | Label of the "after" snapshot |

**Returns:** JSON structured diff report with additions, removals, and changes.

---

<a id="data"></a>
## data.* — Data Pipeline

Transform, analyze, and transport data with pipelines, HTTP requests, and link graph analysis.

---

### data.pipeline

Execute a multi-step data pipeline with filter, transform, sort, and deduplicate operations on JSON input.

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `name` | string | ✅ | Pipeline name |
| `steps` | string | ✅ | Pipeline steps as JSON array (see docs for step types) |
| `input` | string | ✅ | Input data as a JSON array of objects with string values |

**Returns:** JSON array of transformed items.

---

### data.http_get

Perform an HTTP GET request through the browser session. Returns status code, headers, and response body.

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `url` | string | ✅ | URL to fetch |
| `headers` | string | | Optional headers as JSON object |

**Returns:**
```json
{
  "status": 200,
  "headers": { "content-type": "application/json" },
  "body": "..."
}
```

---

### data.http_post

Perform an HTTP POST request through the browser session. Returns status code, headers, and response body.

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `url` | string | ✅ | URL to post to |
| `body` | string | ✅ | Request body (string) |
| `headers` | string | | Optional headers as JSON object |

**Returns:**
```json
{
  "status": 201,
  "headers": { "content-type": "application/json" },
  "body": "..."
}
```

---

### data.links

Extract all links from the live page and return as directed edges suitable for graph analysis.

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `base_url` | string | ✅ | Base URL for resolving relative links |

**Returns:** JSON array of `{ "source": "...", "target": "..." }` link edges.

---

### data.graph

Analyze a link graph to compute stats, find orphan pages, identify hubs, and detect broken links.

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `edges` | string | ✅ | Link edges as JSON array: `[{"source":"...","target":"..."}]` |

**Returns:**
```json
{
  "total_nodes": 45,
  "total_edges": 156,
  "orphans": ["https://example.com/old-page"],
  "hubs": [{ "url": "https://example.com", "outgoing": 23 }]
}
```

---

<a id="crypto"></a>
## crypto.* — Encryption & TOTP

AES-256-GCM encryption, PKCE challenge generation, and TOTP code generation.

---

### crypto.encrypt

Encrypt text with AES-256-GCM. Returns base64-encoded ciphertext (salt + nonce + ct).

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `plaintext` | string | ✅ | Plaintext string to encrypt |
| `password` | string | ✅ | Password for key derivation |

**Returns:** Base64-encoded string (16-byte salt + 12-byte nonce + ciphertext).

---

### crypto.decrypt

Decrypt base64-encoded AES-256-GCM ciphertext (salt + nonce + ct).

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `ciphertext` | string | ✅ | Base64-encoded ciphertext (salt + nonce + ciphertext) |
| `password` | string | ✅ | Password for key derivation |

**Returns:** Decrypted plaintext string.

---

### crypto.generate_pkce

Generate a PKCE S256 challenge pair (code_verifier + code_challenge).

**Parameters:** None

**Returns:**
```json
{
  "code_verifier": "dBjftJeZ4CVP-mB92K27uhbUJU1p1r_wW1gFWFOEjXk",
  "code_challenge": "E9Melhoa2OwvFrEMTJguCHaoeK1t8URWbuGJSstw-cM"
}
```

---

### crypto.generate_totp

Generate a 6-digit TOTP code from a base32 secret.

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `secret` | string | ✅ | Base32-encoded TOTP secret |

**Returns:** 6-digit TOTP code string (e.g. `"482913"`).

---

<a id="parser"></a>
## parser.* — HTML Parsing

Parse raw HTML strings without a browser — lightweight and fast.

---

### parser.parse_a11y_tree

Parse HTML into an accessibility tree (text representation).

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `html` | string | ✅ | Raw HTML string |

**Returns:** Text representation of the accessibility tree.

---

### parser.query_selector

Query HTML with a CSS selector. Returns JSON array of matching elements.

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `html` | string | ✅ | Raw HTML string |
| `selector` | string | ✅ | CSS selector to query |

**Returns:** JSON array of matching elements.

---

### parser.extract_text

Extract visible text from HTML.

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `html` | string | ✅ | Raw HTML string |

**Returns:** Extracted text lines joined by newlines.

---

### parser.extract_links

Extract all links from HTML. Returns JSON array with href, text, is_external.

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `html` | string | ✅ | Raw HTML string |

**Returns:**
```json
[
  { "href": "https://example.com", "text": "Example", "is_external": true },
  { "href": "/about", "text": "About Us", "is_external": false }
]
```

---

<a id="storage"></a>
## storage.* — Encrypted Key-Value Storage

Persistent AES-encrypted key-value store backed by a local database file.

---

### storage.set

Store a key-value pair in encrypted storage.

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `key` | string | ✅ | Storage key |
| `value` | string | ✅ | Value to store |

**Returns:** `"stored key \"session_token\""`

---

### storage.get

Retrieve a value from encrypted storage by key.

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `key` | string | ✅ | Storage key to retrieve |

**Returns:** The stored value string, or `"key \"x\" not found"` if missing.

---

### storage.list_keys

List all keys in encrypted storage.

**Parameters:** None

**Returns:** JSON array of key strings.

---

<a id="auth"></a>
## auth.* — WebAuthn / Passkeys

Simulate WebAuthn/FIDO2 passkey flows with a virtual authenticator for testing.

---

### auth.passkey_enable

Enable a virtual WebAuthn authenticator for passkey simulation.

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `protocol` | string | | Protocol: `"ctap2"` or `"u2f"` (default: `"ctap2"`) |
| `transport` | string | | Transport: `"internal"`, `"usb"`, `"nfc"`, `"ble"` (default: `"internal"`) |

**Returns:** `"Virtual authenticator enabled"`

---

### auth.passkey_add

Add a passkey credential to the virtual authenticator.

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `credential_id` | string | ✅ | Base64url-encoded credential ID |
| `rp_id` | string | ✅ | Relying party domain (e.g. `"example.com"`) |
| `user_handle` | string | | Optional base64url-encoded user handle |

**Returns:** `"Credential added"`

---

### auth.passkey_list

List all stored passkey credentials.

**Parameters:** None

**Returns:** JSON array of credential objects.

---

### auth.passkey_log

Get the WebAuthn operation log.

**Parameters:** None

**Returns:** JSON array of WebAuthn operations.

---

### auth.passkey_disable

Disable the virtual WebAuthn authenticator.

**Parameters:** None

**Returns:** `"Virtual authenticator disabled"`

---

### auth.passkey_remove

Remove a passkey credential by ID.

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `credential_id` | string | ✅ | Credential ID to remove |

**Returns:**
```json
{ "removed": true }
```

---

<a id="automation"></a>
## automation.* — Rate Limiting & Retry Queues

Manage request rate limits and retry queues for resilient automation.

---

### automation.rate_limit

Check rate limiter status and whether new requests can proceed. Initializes the limiter on first call.

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `max_per_second` | number | | Max requests per second (default: 2.0) |
| `max_per_minute` | number | | Max requests per minute (default: 60.0) |

**Returns:**
```json
{
  "can_proceed": true,
  "stats": {
    "requests_this_second": 1,
    "requests_this_minute": 23,
    "total_requests": 156
  }
}
```

---

### automation.retry

Enqueue a failed URL or operation into the retry queue with exponential backoff and jitter.

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `url` | string | ✅ | URL to retry |
| `operation` | string | ✅ | Operation label (e.g. `"navigate"`, `"extract"`) |
| `payload` | string | | Optional payload string |

**Returns:**
```json
{
  "id": "retry-abc123",
  "queue_stats": {
    "pending": 3,
    "completed": 12,
    "failed": 1,
    "total": 16
  }
}
```

---

## Common Patterns

### Pattern 1: Login Flow with computer.act

```
1. computer.act  → { "type": "navigate", "url": "https://app.example.com/login" }
2. computer.observe  → get snapshot, find email field @e3
3. computer.act  → { "type": "fill", "selector": "@e3", "value": "user@example.com" }
4. computer.act  → { "type": "fill", "selector": "@e5", "value": "password123" }
5. computer.act  → { "type": "click", "selector": "@e7" }  // submit button
6. computer.observe  → verify redirect to dashboard
```

### Pattern 2: Scraping with Snapshot + Smart Click

```
1. navigation.goto     → navigate to target page
2. stealth.inject      → apply anti-bot patches
3. navigation.snapshot → get page structure with @refs
4. smart.click         → query: "Accept cookies" (fuzzy match)
5. scraping.stream     → extract product data with field schema
6. scraping.structured → get JSON-LD/OpenGraph metadata
```

### Pattern 3: Visual Regression with Snapshot Diff

```
1. navigation.goto     → navigate to page
2. crawling.snapshot   → label: "before"
3. // ... perform actions that change the page ...
4. crawling.snapshot   → label: "after"
5. crawling.compare    → before: "before", after: "after"
```

Or with accessibility snapshots:
```
1. navigation.snapshot → save snapshot text as "before"
2. // ... perform actions ...
3. navigation.snapshot → save snapshot text as "after"
4. scraping.snapshot_diff → compare the two snapshot texts
```

### Pattern 4: Multi-Step Chain Execution

```json
{
  "commands": [
    { "tool": "navigation.goto", "args": { "url": "https://example.com" } },
    { "tool": "navigation.wait", "args": { "selector": ".content", "timeout_ms": 5000 } },
    { "tool": "navigation.snapshot", "args": { "interactive_only": true, "compact": true } },
    { "tool": "scraping.css", "args": { "selector": "h1::text" } }
  ],
  "stop_on_error": true
}
```

### Pattern 5: Safe Crawling with Rate Limiting

```
1. agent.safety_policy_set → allowed_domains: ["example.com"], max_actions: 200
2. automation.rate_limit   → max_per_second: 1.0, max_per_minute: 30
3. crawling.spider         → start_urls: ["https://example.com"], max_depth: 3
4. // On failure:
5. automation.retry         → enqueue failed URL with backoff
```

### Pattern 6: iOS Mobile Safari Testing

```
1. agent.ios_devices    → list available simulators
2. agent.ios_connect    → connect to iPhone 15 Pro simulator
3. agent.ios_navigate   → url: "https://m.example.com"
4. agent.ios_screenshot → capture mobile rendering
5. agent.ios_tap        → x: 195, y: 400
```

### Pattern 7: API Monitoring During Interaction

```
1. navigation.goto           → navigate to SPA
2. agent.api_capture_start   → start intercepting fetch/XHR
3. smart.click               → "Load more" button
4. navigation.wait           → wait for ".results" selector
5. agent.api_capture_summary → review all API calls made
```

### Pattern 8: Remote Browser (Browserbase/BrowserCloud)

```
1. agent.connect_remote → ws_url: "wss://connect.browserbase.com/session/..."
2. navigation.goto      → all subsequent commands use the remote browser
3. navigation.snapshot  → works exactly as local
4. scraping.text        → extract from remote page
```

---

## Memory

Agent Memory tools for persistent cross-session knowledge. Data is stored in `~/.onecrawl/agent_memory.json` and persists across sessions. Agents use memory to learn domain-specific patterns, selectors, strategies, and improve over time.

### `memory.store`

Store a memory entry — persists data across sessions.

| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| `key` | string | ✅ | Unique key for this memory entry |
| `value` | any | ✅ | JSON value to store |
| `category` | string | — | Category: `page_visit`, `element_pattern`, `domain_strategy`, `retry_knowledge`, `user_preference`, `selector_mapping`, `error_pattern`, `custom` |
| `domain` | string | — | Domain this memory is associated with (e.g. `example.com`) |

**Returns:**
```json
{
  "stored": "login:google",
  "category": "Some(SelectorMapping)"
}
```

### `memory.recall`

Recall a specific memory entry by key.

| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| `key` | string | ✅ | Key of the memory entry to recall |

**Returns:**
```json
{
  "key": "login:google",
  "value": {"selector": "#login-btn"},
  "category": "SelectorMapping",
  "domain": "google.com",
  "access_count": 5,
  "created_at": 1700000000,
  "accessed_at": 1700001000
}
```

### `memory.search`

Search agent memory by query text with optional filters.

| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| `query` | string | ✅ | Search query (matches against keys and values) |
| `category` | string | — | Filter by category |
| `domain` | string | — | Filter by domain |

**Returns:**
```json
{
  "query": "login",
  "count": 3,
  "results": [
    { "key": "login:google", "value": {...}, "category": "SelectorMapping", "domain": "google.com", "access_count": 5 }
  ]
}
```

### `memory.forget`

Forget a specific memory entry by key, or clear all memories for a domain.

| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| `key` | string | — | Key to forget |
| `domain` | string | — | Domain to clear all memories for |

If neither `key` nor `domain` is provided, clears **all** memory.

**Returns:**
```json
{ "removed": 3, "domain": "example.com" }
```

### `memory.domain_strategy`

Store or recall domain-specific strategies (login selectors, navigation patterns, popup handlers, rate limits).

| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| `domain` | string | ✅ | Domain to store/recall strategy for |
| `strategy` | object | — | Strategy data as JSON. Omit to recall existing. |

**Strategy format:**
```json
{
  "domain": "example.com",
  "login_selectors": {
    "username_selector": "#user",
    "password_selector": "#pass",
    "submit_selector": "#submit",
    "success_indicator": ".dashboard"
  },
  "navigation_patterns": [],
  "known_popups": [
    { "trigger": "page_load", "dismiss_selector": ".cookie-accept", "frequency": "always" }
  ],
  "rate_limit_info": { "max_requests_per_minute": 60, "retry_after_seconds": 30, "backoff_strategy": "exponential" },
  "anti_bot_level": "medium"
}
```

### `memory.stats`

Get memory utilization statistics.

| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| — | — | — | No parameters |

**Returns:**
```json
{
  "total_entries": 42,
  "max_entries": 10000,
  "categories": { "SelectorMapping": 15, "PageVisit": 20, "DomainStrategy": 7 },
  "domains": { "google.com": 10, "github.com": 8 },
  "utilization": "0.4%"
}
```

---

## Workflow

Workflow DSL tools for defining and executing browser automation as JSON recipes. Supports 17 action types: `navigate`, `click`, `type`, `wait_for_selector`, `screenshot`, `evaluate`, `extract`, `smart_click`, `smart_fill`, `sleep`, `set_variable`, `log`, `assert`, `loop`, `conditional`, `sub_workflow`, `http_request`, `snapshot`.

### `workflow.validate`

Validate a workflow definition before execution.

| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| `workflow` | string | ✅ | Workflow definition as JSON string |

**Returns:**
```json
{
  "valid": true,
  "name": "Login Flow",
  "steps": 5,
  "variables": ["base_url", "username"]
}
```

### `workflow.run`

Execute a complete workflow. Supports variable interpolation (`{{var_name}}`), conditionals, loops, error handling, and step chaining.

| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| `workflow` | string | ✅ | Workflow JSON string or file path |
| `variables` | object | — | Override variables as key-value pairs |

**Example workflow:**
```json
{
  "name": "Login Flow",
  "variables": { "base_url": "https://example.com", "username": "user@test.com" },
  "steps": [
    { "name": "Navigate", "action": { "type": "navigate", "url": "{{base_url}}/login" } },
    { "name": "Type username", "action": { "type": "type", "selector": "#email", "text": "{{username}}" } },
    { "name": "Type password", "action": { "type": "type", "selector": "#password", "text": "secret123" } },
    { "name": "Submit", "action": { "type": "click", "selector": "#submit" } },
    { "name": "Verify", "action": { "type": "wait_for_selector", "selector": ".dashboard" } },
    { "name": "Take screenshot", "action": { "type": "screenshot", "path": "result.png" } }
  ],
  "on_error": { "action": "stop", "screenshot": true, "log": true }
}
```

**Returns:**
```json
{
  "name": "Login Flow",
  "status": "success",
  "total_duration_ms": 3200,
  "steps_succeeded": 6,
  "steps_failed": 0,
  "steps_skipped": 0,
  "steps": [...],
  "variables": { "base_url": "https://example.com", "username": "user@test.com" }
}
```

---

## Net

Network Intelligence tools for API reverse engineering. Capture page traffic, discover API schemas, generate SDK stubs, mock server configs, and replay sequences.

### `net.capture`

Capture network traffic from the current page by intercepting fetch() and XMLHttpRequest calls.

| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| `duration_seconds` | number | — | Capture duration (default: 10) |
| `api_only` | boolean | — | Exclude static assets (default: true) |

**Returns:**
```json
{
  "endpoints": [
    {
      "method": "GET",
      "url": "https://api.example.com/users",
      "path": "/users",
      "base_url": "https://api.example.com",
      "status_code": 200,
      "content_type": "application/json",
      "response_body": [{"id": 1, "name": "Alice"}],
      "timing_ms": 150,
      "category": "rest"
    }
  ],
  "count": 5,
  "duration_seconds": 10
}
```

### `net.analyze`

Analyze captured traffic to discover API schemas, auth patterns, and endpoint templates.

| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| `capture` | string | ✅ | Endpoints JSON from `net.capture` output |

**Returns:**
```json
{
  "base_url": "https://api.example.com",
  "endpoints": [
    {
      "method": "GET",
      "path": "/users/{id}",
      "path_params": ["id"],
      "query_params": [],
      "response_body_schema": { "type": "object", "properties": { "id": { "type": "integer" }, "name": { "type": "string" } } },
      "status_codes": [200],
      "call_count": 3,
      "avg_latency_ms": 145.0
    }
  ],
  "auth_pattern": { "type": "bearer", "header": "Authorization" },
  "total_requests": 12,
  "unique_endpoints": 5
}
```

### `net.sdk`

Generate an SDK client from an API schema. Supports TypeScript and Python.

| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| `schema` | string | ✅ | API schema JSON from `net.analyze` |
| `language` | string | — | `typescript` (default) or `python` |

**Returns:**
```json
{
  "language": "typescript",
  "code": "export class ApiClient {\n  ...\n}",
  "endpoints_covered": 5
}
```

### `net.mock`

Generate a mock server configuration from captured endpoints.

| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| `endpoints` | string | ✅ | Endpoints JSON from `net.capture` |
| `port` | number | — | Port for mock server (default: 3001) |

**Returns:**
```json
{
  "port": 3001,
  "endpoints": [
    { "method": "GET", "path": "/users", "response_status": 200, "response_body": [...] }
  ],
  "default_response": { "method": "GET", "path": "*", "response_status": 404 }
}
```

### `net.replay`

Generate a replay sequence from captured network traffic for reproducing API call patterns.

| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| `endpoints` | string | ✅ | Endpoints JSON from `net.capture` |
| `name` | string | — | Name for the sequence (default: `replay_sequence`) |

**Returns:**
```json
{
  "name": "checkout_flow",
  "steps": [
    { "method": "GET", "url": "https://api.example.com/cart", "expected_status": 200, "delay_before_ms": 50 },
    { "method": "POST", "url": "https://api.example.com/checkout", "expected_status": 201, "delay_before_ms": 100 }
  ],
  "total_duration_ms": 150
}
```

---

## Error Handling

All tools return errors via MCP's `ErrorData` format:

```json
{
  "code": -1,
  "message": "element not found: @e99"
}
```

Chain commands include structured step-level errors:

```json
{
  "tool": "navigation.click",
  "success": false,
  "error": {
    "message": "element not found",
    "code": "CHAIN_STEP_FAILED"
  }
}
```

### Common Error Codes

| Error | Cause | Solution |
|-------|-------|----------|
| `element not found` | Selector/ref doesn't match any element | Use `navigation.snapshot` to discover valid refs |
| `browser error` | Browser session issue | Call `navigation.goto` to reinitialize |
| `timeout` | Operation exceeded time limit | Increase `timeout_ms` or check page load |
| `invalid base64` | Malformed encrypted data | Verify data was produced by `crypto.encrypt` |
| `selector must not be empty` | Empty string passed as selector | Provide a valid CSS selector or `@ref` |

---

## Notes

- **Browser lifecycle:** The headless browser starts on the first CDP tool call and persists for the session.
- **`@ref` notation:** Refs from `navigation.snapshot` (e.g. `@e1`) can be used in any tool accepting a CSS selector — they auto-resolve to the actual DOM selector.
- **Stealth:** Call `stealth.inject` early, before navigating to detection-heavy sites.
- **Parser vs. Scraping:** `parser.*` works on raw HTML strings (no browser needed); `scraping.*` operates on the live browser DOM.
- **Safety policies:** Use `agent.safety_policy_set` to constrain agent behavior in production deployments.
- **Agent Memory:** Memory persists in `~/.onecrawl/agent_memory.json`. Use `memory.store` to teach the agent, `memory.recall` to retrieve, `memory.search` to find patterns.
- **Workflow DSL:** Workflows support `{{variable}}` interpolation. Use `workflow.validate` before `workflow.run` in production.
- **Network Intelligence:** The capture→analyze→sdk/mock/replay pipeline enables full API reverse engineering from browser traffic.
