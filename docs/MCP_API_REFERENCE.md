# OneCrawl MCP API Reference

> **10 consolidated tools • 108 actions • Action-based dispatch**

All browser automation, crawling, scraping, security, and AI orchestration capabilities are accessed through 10 super-tools. Each tool accepts a uniform `{ action, params }` interface.

---

## Quick Reference

| Tool | Actions | Description |
|------|:-------:|-------------|
| [`browser`](#1-browser) | 26 | Navigation, interaction, scraping, content extraction, offline HTML parsing |
| [`crawl`](#2-crawl) | 5 | Web crawling, robots.txt, sitemaps, DOM snapshots |
| [`agent`](#3-agent) | 21 | Command chains, API capture, iframes, remote CDP, safety, screencast, recording, iOS |
| [`stealth`](#4-stealth) | 5 | Anti-detection patches, fingerprinting, CAPTCHA detection |
| [`data`](#5-data) | 10 | Data pipelines, HTTP client, link graphs, network intelligence |
| [`secure`](#6-secure) | 13 | Encryption, PKCE, TOTP, encrypted KV store, WebAuthn passkeys |
| [`computer`](#7-computer) | 8 | AI computer-use protocol, smart element resolution, browser pool |
| [`memory`](#8-memory) | 6 | Persistent agent memory across sessions |
| [`automate`](#9-automate) | 7 | Workflow DSL, AI task planning, rate limiting, retry queues |
| [`perf`](#10-perf) | 7 | Performance audits, budgets, regression detection, visual regression testing |

---

## Universal Interface

Every tool call uses the same structure:

```json
{
  "action": "<action_name>",
  "params": { /* action-specific parameters */ }
}
```

- **`action`** *(string, required)* — The operation to perform. See each tool's action table.
- **`params`** *(object, optional)* — Action-specific parameters. Omit or pass `{}` for parameterless actions.

**Error format:** If an unknown action is provided, the tool returns an error listing all available actions.

---

## Tools

### 1. `browser`

Browser navigation, interaction, and content extraction. All browser operations in one tool.

A headless Chromium browser is launched automatically on the first call that requires a live page. Actions prefixed with `parse_` operate offline on raw HTML strings and do not require a browser.

#### Actions

| Action | Params | Description |
|--------|--------|-------------|
| `goto` | `{url}` | Navigate to URL |
| `click` | `{selector}` | Click element |
| `type` | `{selector, text}` | Type into input |
| `screenshot` | `{selector?, full_page?}` | Capture screenshot as PNG |
| `pdf` | `{print_background?, format?, landscape?}` | Export page as PDF |
| `back` | — | Navigate back |
| `forward` | — | Navigate forward |
| `reload` | — | Reload page |
| `wait` | `{selector, timeout_ms?}` | Wait for element |
| `evaluate` | `{js}` | Execute JavaScript |
| `snapshot` | `{interactive_only?, cursor?, compact?, depth?, selector?}` | Accessibility tree with @refs |
| `css` | `{selector}` | CSS query on live DOM |
| `xpath` | `{expression}` | XPath query on live DOM |
| `find_text` | `{text, tag?}` | Find element by visible text |
| `text` | `{selector?}` | Extract visible text |
| `html` | `{selector?}` | Extract raw HTML |
| `markdown` | `{selector?}` | Extract as Markdown |
| `structured` | — | Extract JSON-LD, OpenGraph, etc. |
| `stream` | `{item_selector, fields, pagination?}` | Paginated data extraction |
| `detect_forms` | — | Detect forms and fields |
| `fill_form` | `{form_selector, data, submit?}` | Fill and optionally submit form |
| `snapshot_diff` | `{before, after}` | Diff two accessibility snapshots |
| `parse_a11y` | `{html}` | Parse HTML into accessibility tree (offline) |
| `parse_selector` | `{html, selector}` | CSS query on HTML string (offline) |
| `parse_text` | `{html}` | Extract text from HTML (offline) |
| `parse_links` | `{html}` | Extract links from HTML (offline) |

#### Action Details

<details>
<summary><strong><code>goto</code></strong> — Navigate to URL</summary>

Navigate the browser to a URL. Launches headless browser on first call.

**Params:**

| Name | Type | Required | Description |
|------|------|:--------:|-------------|
| `url` | string | ✅ | URL to navigate to |

**Response:** `"navigated to <url> — title: <title>"`
</details>

<details>
<summary><strong><code>click</code></strong> — Click element</summary>

Click a DOM element by CSS selector or accessibility ref.

**Params:**

| Name | Type | Required | Description |
|------|------|:--------:|-------------|
| `selector` | string | ✅ | CSS selector or `@ref` (e.g. `@e1`) of element to click |

**Response:** `"clicked <selector>"`
</details>

<details>
<summary><strong><code>type</code></strong> — Type into input</summary>

Type text into an input element.

**Params:**

| Name | Type | Required | Description |
|------|------|:--------:|-------------|
| `selector` | string | ✅ | CSS selector of target input element |
| `text` | string | ✅ | Text to type |

**Response:** `"typed <N> chars into <selector>"`
</details>

<details>
<summary><strong><code>screenshot</code></strong> — Capture screenshot</summary>

Take a screenshot of the viewport, full page, or a specific element.

**Params:**

| Name | Type | Required | Description |
|------|------|:--------:|-------------|
| `selector` | string | | CSS selector for element screenshot (omit for viewport) |
| `full_page` | boolean | | If `true`, capture the full scrollable page |

**Response:** PNG image as base64 (returned as `image/png` content).
</details>

<details>
<summary><strong><code>pdf</code></strong> — Export page as PDF</summary>

Export the current page as a PDF document.

**Params:**

| Name | Type | Required | Description |
|------|------|:--------:|-------------|
| `print_background` | boolean | | Print background graphics (default `false`) |
| `format` | string | | Paper format: `A4`, `Letter`, etc. (default `A4`) |
| `landscape` | boolean | | Landscape orientation (default `false`) |

**Response:** `"pdf exported (<bytes> bytes, base64 length <len>)"`
</details>

<details>
<summary><strong><code>back</code></strong> — Navigate back</summary>

Navigate to the previous page in browser history.

**Params:** None

**Response:** `"navigated back"`
</details>

<details>
<summary><strong><code>forward</code></strong> — Navigate forward</summary>

Navigate to the next page in browser history.

**Params:** None

**Response:** `"navigated forward"`
</details>

<details>
<summary><strong><code>reload</code></strong> — Reload page</summary>

Reload the current page.

**Params:** None

**Response:** `"page reloaded"`
</details>

<details>
<summary><strong><code>wait</code></strong> — Wait for element</summary>

Wait until a CSS selector appears on the page.

**Params:**

| Name | Type | Required | Description |
|------|------|:--------:|-------------|
| `selector` | string | ✅ | CSS selector to wait for |
| `timeout_ms` | integer | | Timeout in milliseconds (default `30000`) |

**Response:** `"selector <selector> found"`
</details>

<details>
<summary><strong><code>evaluate</code></strong> — Execute JavaScript</summary>

Evaluate arbitrary JavaScript in the page context and return the result.

**Params:**

| Name | Type | Required | Description |
|------|------|:--------:|-------------|
| `js` | string | ✅ | JavaScript code to evaluate in the page context |

**Response:** JSON — the serialized return value of the expression.
</details>

<details>
<summary><strong><code>snapshot</code></strong> — Accessibility snapshot</summary>

Generate an accessibility tree snapshot of the current page. Returns element refs (`@e1`, `@e2`, …) that can be used in place of CSS selectors for `click`, `type`, `wait`, etc.

**Params:**

| Name | Type | Required | Description |
|------|------|:--------:|-------------|
| `interactive_only` | boolean | | Only include interactive elements (buttons, links, inputs) |
| `cursor` | boolean | | Include cursor-interactive elements (`cursor:pointer`, `onclick`, `tabindex`) |
| `compact` | boolean | | Remove empty structural elements for minimal output |
| `depth` | integer | | Max DOM depth to include |
| `selector` | string | | CSS selector to scope snapshot to a subtree |

**Response:**
```json
{
  "snapshot": "<text tree>",
  "refs": { "@e1": "button#submit", ... },
  "total": 42,
  "stats": {
    "lines": 120,
    "chars": 3500,
    "estimated_tokens": 875,
    "total_refs": 42,
    "interactive_refs": 18
  }
}
```
</details>

<details>
<summary><strong><code>css</code></strong> — CSS query on live DOM</summary>

Execute a CSS selector query against the live page DOM. Supports `::text` and `::attr(name)` pseudo-elements.

**Params:**

| Name | Type | Required | Description |
|------|------|:--------:|-------------|
| `selector` | string | ✅ | CSS selector (supports `::text`, `::attr(name)` pseudo-elements) |

**Response:** JSON array of matched elements.
</details>

<details>
<summary><strong><code>xpath</code></strong> — XPath query on live DOM</summary>

Execute an XPath expression against the live page DOM.

**Params:**

| Name | Type | Required | Description |
|------|------|:--------:|-------------|
| `expression` | string | ✅ | XPath expression to evaluate |

**Response:** JSON array of matched elements.
</details>

<details>
<summary><strong><code>find_text</code></strong> — Find by visible text</summary>

Find elements by their visible text content.

**Params:**

| Name | Type | Required | Description |
|------|------|:--------:|-------------|
| `text` | string | ✅ | Text content to search for |
| `tag` | string | | Optional HTML tag to constrain search (e.g. `a`, `button`) |

**Response:** JSON array of matching elements.
</details>

<details>
<summary><strong><code>text</code></strong> — Extract visible text</summary>

Extract visible text content from the page or a specific element.

**Params:**

| Name | Type | Required | Description |
|------|------|:--------:|-------------|
| `selector` | string | | CSS selector (default `body`) |

**Response:** JSON — extracted text content.
</details>

<details>
<summary><strong><code>html</code></strong> — Extract raw HTML</summary>

Extract raw HTML from the page or a specific element.

**Params:**

| Name | Type | Required | Description |
|------|------|:--------:|-------------|
| `selector` | string | | CSS selector (default `body`) |

**Response:** JSON — extracted HTML string.
</details>

<details>
<summary><strong><code>markdown</code></strong> — Extract as Markdown</summary>

Extract page content as Markdown.

**Params:**

| Name | Type | Required | Description |
|------|------|:--------:|-------------|
| `selector` | string | | CSS selector (default `body`) |

**Response:** JSON — extracted Markdown content.
</details>

<details>
<summary><strong><code>structured</code></strong> — Extract structured data</summary>

Extract JSON-LD, OpenGraph, Twitter Card, and other structured data from the current page.

**Params:** None

**Response:** JSON object with all structured data found on the page.
</details>

<details>
<summary><strong><code>stream</code></strong> — Paginated data extraction</summary>

Extract repeating data items from the page, with optional pagination support.

**Params:**

| Name | Type | Required | Description |
|------|------|:--------:|-------------|
| `item_selector` | string | ✅ | CSS selector for repeating item container |
| `fields` | string | ✅ | Field extraction rules as JSON array: `[{"name":"title","selector":"h2","extract":"text"}]` |
| `pagination` | string | | Pagination config as JSON: `{"next_selector":".next","max_pages":5,"delay_ms":1000}` |

**Response:** JSON array of extracted items. With pagination, includes items from all pages.
</details>

<details>
<summary><strong><code>detect_forms</code></strong> — Detect forms</summary>

Detect all forms and their fields on the current page.

**Params:** None

**Response:** JSON array of detected forms with field metadata.
</details>

<details>
<summary><strong><code>fill_form</code></strong> — Fill and submit form</summary>

Fill form fields with values and optionally submit.

**Params:**

| Name | Type | Required | Description |
|------|------|:--------:|-------------|
| `form_selector` | string | ✅ | CSS selector of the form element |
| `data` | string | ✅ | JSON object mapping field selectors to values, e.g. `{"#email":"a@b.com"}` |
| `submit` | boolean | | If `true`, submit the form after filling |

**Response:** JSON — fill result. If `submit` is true, the form is submitted after filling.
</details>

<details>
<summary><strong><code>snapshot_diff</code></strong> — Diff two snapshots</summary>

Compare two accessibility snapshot text outputs (from `snapshot` action) and report differences.

**Params:**

| Name | Type | Required | Description |
|------|------|:--------:|-------------|
| `before` | string | ✅ | Accessibility snapshot text before |
| `after` | string | ✅ | Accessibility snapshot text after |

**Response:** JSON diff of the two snapshots.
</details>

<details>
<summary><strong><code>parse_a11y</code></strong> — Parse HTML to accessibility tree (offline)</summary>

Parse raw HTML into an accessibility tree. Does not require a browser.

**Params:**

| Name | Type | Required | Description |
|------|------|:--------:|-------------|
| `html` | string | ✅ | Raw HTML string |

**Response:** Text rendering of the accessibility tree.
</details>

<details>
<summary><strong><code>parse_selector</code></strong> — CSS query on HTML string (offline)</summary>

Run a CSS selector query against a raw HTML string. Does not require a browser.

**Params:**

| Name | Type | Required | Description |
|------|------|:--------:|-------------|
| `html` | string | ✅ | Raw HTML string |
| `selector` | string | ✅ | CSS selector to query |

**Response:** JSON array of matched elements.
</details>

<details>
<summary><strong><code>parse_text</code></strong> — Extract text from HTML (offline)</summary>

Extract visible text from a raw HTML string. Does not require a browser.

**Params:**

| Name | Type | Required | Description |
|------|------|:--------:|-------------|
| `html` | string | ✅ | Raw HTML string |

**Response:** Extracted text, lines joined by `\n`.
</details>

<details>
<summary><strong><code>parse_links</code></strong> — Extract links from HTML (offline)</summary>

Extract all links from a raw HTML string. Does not require a browser.

**Params:**

| Name | Type | Required | Description |
|------|------|:--------:|-------------|
| `html` | string | ✅ | Raw HTML string |

**Response:** JSON array of `{ "href", "text", "is_external" }` objects.
</details>

---

### 2. `crawl`

Web crawling, robots.txt parsing, sitemap generation, and DOM snapshot management.

#### Actions

| Action | Params | Description |
|--------|--------|-------------|
| `spider` | `{start_urls, max_depth?, max_pages?, same_domain_only?, url_patterns?, exclude_patterns?, delay_ms?}` | Crawl a website |
| `robots` | `{base_url, path?, user_agent?}` | Parse robots.txt |
| `sitemap` | `{base_url, entries, default_changefreq?}` | Generate XML sitemap |
| `dom_snapshot` | `{label}` | Take labeled DOM snapshot |
| `dom_compare` | `{before, after}` | Compare two DOM snapshots |

#### Action Details

<details>
<summary><strong><code>spider</code></strong> — Crawl website</summary>

Crawl a website starting from one or more URLs, following links up to a configurable depth and page limit.

**Params:**

| Name | Type | Required | Description |
|------|------|:--------:|-------------|
| `start_urls` | string[] | ✅ | Starting URL(s) to crawl |
| `max_depth` | integer | | Maximum link depth (default `2`) |
| `max_pages` | integer | | Maximum pages to visit (default `50`) |
| `same_domain_only` | boolean | | Stay on the same domain only (default `true`) |
| `url_patterns` | string[] | | URL patterns to include (regex) |
| `exclude_patterns` | string[] | | URL patterns to exclude (regex) |
| `delay_ms` | integer | | Delay between requests in ms (default `500`) |

**Response:**
```json
{
  "summary": { /* crawl summary stats */ },
  "pages_crawled": 23
}
```
</details>

<details>
<summary><strong><code>robots</code></strong> — Parse robots.txt</summary>

Fetch and parse a site's robots.txt. Optionally check if a specific path is allowed.

**Params:**

| Name | Type | Required | Description |
|------|------|:--------:|-------------|
| `base_url` | string | ✅ | Base URL to fetch robots.txt from |
| `path` | string | | Path to check (e.g. `/admin`) |
| `user_agent` | string | | User-agent string (default `*`) |

**Response:**
```json
{
  "sitemaps": ["https://example.com/sitemap.xml"],
  "crawl_delay": 1.0,
  "path_allowed": true
}
```
</details>

<details>
<summary><strong><code>sitemap</code></strong> — Generate XML sitemap</summary>

Generate an XML sitemap from a list of URL entries.

**Params:**

| Name | Type | Required | Description |
|------|------|:--------:|-------------|
| `base_url` | string | ✅ | Base URL for the sitemap |
| `entries` | string | ✅ | URLs as JSON array: `[{"url":"...","priority":0.8}]` |
| `default_changefreq` | string | | Default change frequency (e.g. `weekly`) |

**Response:** XML sitemap string.
</details>

<details>
<summary><strong><code>dom_snapshot</code></strong> — Take DOM snapshot</summary>

Capture a labeled DOM snapshot of the current page for later comparison.

**Params:**

| Name | Type | Required | Description |
|------|------|:--------:|-------------|
| `label` | string | ✅ | Label to identify this snapshot |

**Response:** `"snapshot '<label>' saved"`
</details>

<details>
<summary><strong><code>dom_compare</code></strong> — Compare DOM snapshots</summary>

Compare two previously taken DOM snapshots by label.

**Params:**

| Name | Type | Required | Description |
|------|------|:--------:|-------------|
| `before` | string | ✅ | Label of the "before" snapshot |
| `after` | string | ✅ | Label of the "after" snapshot |

**Response:** JSON diff of the two snapshots.
</details>

---

### 3. `agent`

AI agent orchestration — command chains, element screenshots, API capture, iframes, remote CDP, safety policies, skills, screencast, recording, and iOS automation.

#### Actions

| Action | Params | Description |
|--------|--------|-------------|
| `execute_chain` | `{commands, stop_on_error?}` | Execute multiple commands sequentially |
| `element_screenshot` | `{selector}` | Screenshot a specific element |
| `api_capture_start` | — | Start capturing API calls (fetch/XHR) |
| `api_capture_summary` | `{clear?}` | Get captured API call summary |
| `iframe_list` | — | List all iframes on page |
| `iframe_snapshot` | `{index, interactive_only?, compact?}` | Accessibility snapshot of an iframe |
| `connect_remote` | `{ws_url, headers?}` | Connect to remote CDP browser |
| `safety_set` | `{allowed_domains?, blocked_domains?, ...}` | Set safety policy |
| `safety_status` | — | Get current safety policy status |
| `skills_list` | — | List available built-in skills |
| `screencast_start` | `{format?, quality?, max_width?, max_height?, every_nth_frame?}` | Start screencast |
| `screencast_stop` | — | Stop screencast |
| `screencast_frame` | `{format?, quality?}` | Get latest screencast frame |
| `recording_start` | `{output?, fps?}` | Start video recording |
| `recording_stop` | — | Stop recording and save |
| `recording_status` | — | Get recording status |
| `ios_devices` | — | List iOS devices |
| `ios_connect` | `{wda_url?, udid?, bundle_id?}` | Connect to iOS device |
| `ios_navigate` | `{url}` | Navigate iOS Safari |
| `ios_tap` | `{x, y}` | Tap on iOS screen |
| `ios_screenshot` | — | Take iOS screenshot |

#### Action Details

<details>
<summary><strong><code>execute_chain</code></strong> — Execute command chain</summary>

Execute multiple tool commands in sequence. Each command references a tool and arguments. Results are collected and returned together.

**Params:**

| Name | Type | Required | Description |
|------|------|:--------:|-------------|
| `commands` | array | ✅ | List of commands. Each: `{ "tool": "navigation.click", "args": { "selector": "#btn" } }` |
| `stop_on_error` | boolean | | Stop on first error (default `true`) |

**Command object:**

| Field | Type | Description |
|-------|------|-------------|
| `tool` | string | Tool name (e.g. `navigation.goto`, `navigation.click`, `navigation.type`) |
| `args` | object | Arguments as JSON object |

**Response:**
```json
{
  "results": [
    { "tool": "navigation.goto", "success": true, "data": { "url": "...", "title": "..." } },
    { "tool": "navigation.click", "success": true, "data": { "clicked": "#btn" } }
  ],
  "completed": 2,
  "total": 2
}
```
</details>

<details>
<summary><strong><code>element_screenshot</code></strong> — Element screenshot</summary>

Take a screenshot of a specific element, returning both the image and the element's bounding box.

**Params:**

| Name | Type | Required | Description |
|------|------|:--------:|-------------|
| `selector` | string | ✅ | CSS selector or `@ref` (e.g. `@e1`) of the element |

**Response:**
```json
{
  "image": "<base64 PNG>",
  "bounds": { "x": 10, "y": 20, "width": 200, "height": 50 }
}
```
</details>

<details>
<summary><strong><code>api_capture_start</code></strong> — Start API capture</summary>

Inject interceptors for `fetch` and `XMLHttpRequest` to capture all API calls made by the page.

**Params:** None

**Response:**
```json
{ "active": true, "entries": 0 }
```
</details>

<details>
<summary><strong><code>api_capture_summary</code></strong> — Get API capture summary</summary>

Retrieve all captured API calls since the last start/clear.

**Params:**

| Name | Type | Required | Description |
|------|------|:--------:|-------------|
| `clear` | boolean | | Clear the captured log after reading (default `false`) |

**Response:**
```json
{
  "total": 5,
  "requests": [
    { "type": "fetch", "method": "GET", "url": "...", "status": 200, "contentType": "application/json" }
  ]
}
```
</details>

<details>
<summary><strong><code>iframe_list</code></strong> — List iframes</summary>

List all iframes on the current page.

**Params:** None

**Response:**
```json
{ "total": 2, "iframes": [ /* iframe metadata */ ] }
```
</details>

<details>
<summary><strong><code>iframe_snapshot</code></strong> — Iframe accessibility snapshot</summary>

Generate an accessibility snapshot inside a specific iframe.

**Params:**

| Name | Type | Required | Description |
|------|------|:--------:|-------------|
| `index` | integer | ✅ | Zero-based index of the iframe |
| `interactive_only` | boolean | | Only include interactive elements |
| `compact` | boolean | | Remove empty structural elements |

**Response:** JSON with `snapshot`, `refs`, `total`, `iframe_index`.
</details>

<details>
<summary><strong><code>connect_remote</code></strong> — Connect to remote CDP</summary>

Connect to a remote Chrome DevTools Protocol endpoint, replacing the local browser session.

**Params:**

| Name | Type | Required | Description |
|------|------|:--------:|-------------|
| `ws_url` | string | ✅ | WebSocket URL (e.g. `ws://127.0.0.1:9222/devtools/browser/...`) |
| `headers` | object | | Optional HTTP headers for WebSocket handshake |

**Response:**
```json
{
  "connected": true,
  "ws_url": "ws://...",
  "info": "Remote browser connected. Subsequent tools will use this session."
}
```
</details>

<details>
<summary><strong><code>safety_set</code></strong> — Set safety policy</summary>

Configure a safety policy to restrict browser automation actions.

**Params:**

| Name | Type | Required | Description |
|------|------|:--------:|-------------|
| `allowed_domains` | string[] | | Allowed domains (empty = all allowed) |
| `blocked_domains` | string[] | | Blocked domains |
| `blocked_url_patterns` | string[] | | Blocked URL patterns (glob with `*`) |
| `max_actions` | integer | | Max actions per session (`0` = unlimited) |
| `confirm_form_submit` | boolean | | Require confirmation for form submissions |
| `confirm_file_upload` | boolean | | Require confirmation for file uploads |
| `blocked_commands` | string[] | | Blocked commands |
| `allowed_commands` | string[] | | Allowed commands (empty = all non-blocked) |
| `rate_limit_per_minute` | integer | | Max actions per minute (`0` = unlimited) |
| `policy_file` | string | | Path to a JSON policy file (overrides other fields) |

**Response:** `{ "status": "policy_set", "policy": { ... } }`
</details>

<details>
<summary><strong><code>safety_status</code></strong> — Safety policy status</summary>

Get the current safety policy configuration and enforcement stats.

**Params:** None

**Response:** JSON with policy stats, or `{ "status": "no_policy" }` if none is set.
</details>

<details>
<summary><strong><code>skills_list</code></strong> — List available skills</summary>

List all built-in skill packages and their tools.

**Params:** None

**Response:** JSON array of skill objects with `name`, `version`, `description`, `tools`.
</details>

<details>
<summary><strong><code>screencast_start</code></strong> — Start screencast</summary>

Start a live screencast stream from the browser.

**Params:**

| Name | Type | Required | Description |
|------|------|:--------:|-------------|
| `format` | string | | Image format: `jpeg` or `png` (default `jpeg`) |
| `quality` | integer | | Compression quality 0–100 (default `60`) |
| `max_width` | integer | | Maximum width in pixels (default `1280`) |
| `max_height` | integer | | Maximum height in pixels (default `720`) |
| `every_nth_frame` | integer | | Capture every N-th frame (default `1`) |

**Response:** `{ "status": "started", "format": "jpeg", "quality": 60, ... }`
</details>

<details>
<summary><strong><code>screencast_stop</code></strong> — Stop screencast</summary>

Stop the active screencast.

**Params:** None

**Response:** `{ "status": "stopped" }`
</details>

<details>
<summary><strong><code>screencast_frame</code></strong> — Get screencast frame</summary>

Capture and return a single screencast frame.

**Params:**

| Name | Type | Required | Description |
|------|------|:--------:|-------------|
| `format` | string | | Image format: `jpeg` or `png` (default `jpeg`) |
| `quality` | integer | | Compression quality 0–100 (default `80`) |

**Response:** `{ "image": "<base64>", "format": "jpeg", "size": 12345 }`
</details>

<details>
<summary><strong><code>recording_start</code></strong> — Start video recording</summary>

Start recording browser interactions as a sequence of frames.

**Params:**

| Name | Type | Required | Description |
|------|------|:--------:|-------------|
| `output` | string | | Output file path (default `recording.webm`) |
| `fps` | integer | | Frames per second (default `5`) |

**Response:** `{ "status": "recording", "output": "recording.webm", "fps": 5 }`
</details>

<details>
<summary><strong><code>recording_stop</code></strong> — Stop recording</summary>

Stop the active recording and save frames to disk.

**Params:** None

**Response:** `{ "status": "saved", "frames": 42, "path": "recording.webm" }`
</details>

<details>
<summary><strong><code>recording_status</code></strong> — Recording status</summary>

Get current recording status.

**Params:** None

**Response:** `{ "status": "recording|stopped|idle", "frames": 42, "fps": 5, "output": "..." }`
</details>

<details>
<summary><strong><code>ios_devices</code></strong> — List iOS devices</summary>

List connected iOS devices.

**Params:** None

**Response:** `{ "devices": [...], "count": 1 }`
</details>

<details>
<summary><strong><code>ios_connect</code></strong> — Connect to iOS device</summary>

Connect to an iOS device via WebDriverAgent for Mobile Safari automation.

**Params:**

| Name | Type | Required | Description |
|------|------|:--------:|-------------|
| `wda_url` | string | | WebDriverAgent URL (default `http://localhost:8100`) |
| `udid` | string | | Device UDID (auto-detect if omitted) |
| `bundle_id` | string | | Bundle ID to automate (default `com.apple.mobilesafari`) |

**Response:** `{ "connected": true, "session_id": "..." }`
</details>

<details>
<summary><strong><code>ios_navigate</code></strong> — Navigate iOS Safari</summary>

Navigate Mobile Safari to a URL.

**Params:**

| Name | Type | Required | Description |
|------|------|:--------:|-------------|
| `url` | string | ✅ | URL to navigate to |

**Response:** `{ "navigated": true, "url": "..." }`
</details>

<details>
<summary><strong><code>ios_tap</code></strong> — Tap on iOS screen</summary>

Tap at specific coordinates on the iOS device screen.

**Params:**

| Name | Type | Required | Description |
|------|------|:--------:|-------------|
| `x` | number | ✅ | X coordinate |
| `y` | number | ✅ | Y coordinate |

**Response:** `{ "tapped": true, "x": 100, "y": 200 }`
</details>

<details>
<summary><strong><code>ios_screenshot</code></strong> — iOS screenshot</summary>

Take a screenshot of the iOS device screen.

**Params:** None

**Response:** `{ "format": "png", "size": 54321, "data": "<base64>" }`
</details>

---

### 4. `stealth`

Anti-detection and bot evasion — stealth patches, fingerprinting, domain blocking, CAPTCHA detection.

#### Actions

| Action | Params | Description |
|--------|--------|-------------|
| `inject` | — | Inject stealth patches into page |
| `test` | — | Test if current page detects bot |
| `fingerprint` | `{user_agent?}` | Generate and apply browser fingerprint |
| `block_domains` | `{domains?, category?}` | Block tracking/ad domains |
| `detect_captcha` | — | Detect CAPTCHAs on page |

#### Action Details

<details>
<summary><strong><code>inject</code></strong> — Inject stealth patches</summary>

Apply all stealth patches to the browser page to avoid bot detection. Patches navigator, WebGL, WebRTC, and other browser fingerprinting vectors.

**Params:** None

**Response:**
```json
{ "patches_applied": 12, "patches": ["navigator.webdriver", "chrome.runtime", ...] }
```
</details>

<details>
<summary><strong><code>test</code></strong> — Bot detection test</summary>

Run bot detection tests on the current page to see if the browser is identified as automated.

**Params:** None

**Response:** JSON with detection test results.
</details>

<details>
<summary><strong><code>fingerprint</code></strong> — Apply fingerprint</summary>

Generate a realistic browser fingerprint and apply it to the current page.

**Params:**

| Name | Type | Required | Description |
|------|------|:--------:|-------------|
| `user_agent` | string | | Optional user-agent override |

**Response:**
```json
{ "user_agent": "Mozilla/5.0 ...", "platform": "Win32" }
```
</details>

<details>
<summary><strong><code>block_domains</code></strong> — Block domains</summary>

Block network requests to specified domains or a built-in category.

**Params:**

| Name | Type | Required | Description |
|------|------|:--------:|-------------|
| `domains` | string[] | | List of domains to block |
| `category` | string | | Built-in category: `ads`, `trackers`, `social` |

> Provide either `domains` or `category` (at least one required).

**Response:** `"<N> domains blocked"`
</details>

<details>
<summary><strong><code>detect_captcha</code></strong> — Detect CAPTCHAs</summary>

Detect the presence of CAPTCHAs (reCAPTCHA, hCaptcha, etc.) on the current page.

**Params:** None

**Response:** JSON with CAPTCHA detection results (type, location, confidence).
</details>

---

### 5. `data`

Data processing, HTTP requests, link analysis, and network intelligence.

#### Actions

| Action | Params | Description |
|--------|--------|-------------|
| `pipeline` | `{name, steps, input}` | Multi-step data pipeline |
| `http_get` | `{url, headers?}` | HTTP GET request |
| `http_post` | `{url, body, headers?}` | HTTP POST request |
| `links` | `{base_url}` | Extract link graph from page |
| `graph` | `{edges}` | Analyze link graph |
| `net_capture` | `{duration_seconds?, api_only?}` | Capture network traffic |
| `net_analyze` | `{capture}` | Analyze captured API traffic |
| `net_sdk` | `{schema, language?}` | Generate API SDK code |
| `net_mock` | `{endpoints, port?}` | Generate mock server config |
| `net_replay` | `{endpoints, name?}` | Generate replay sequence |

#### Action Details

<details>
<summary><strong><code>pipeline</code></strong> — Data processing pipeline</summary>

Execute a multi-step data transformation pipeline on input data. Steps can filter, transform, deduplicate, and reshape data.

**Params:**

| Name | Type | Required | Description |
|------|------|:--------:|-------------|
| `name` | string | ✅ | Pipeline name |
| `steps` | string | ✅ | Pipeline steps as JSON array (see docs for step types) |
| `input` | string | ✅ | Input data as a JSON array of objects with string values |

**Response:** JSON — transformed output data.
</details>

<details>
<summary><strong><code>http_get</code></strong> — HTTP GET request</summary>

Perform an HTTP GET request via the browser context.

**Params:**

| Name | Type | Required | Description |
|------|------|:--------:|-------------|
| `url` | string | ✅ | URL to fetch |
| `headers` | string | | Optional headers as JSON object |

**Response:** JSON — response body, status, headers.
</details>

<details>
<summary><strong><code>http_post</code></strong> — HTTP POST request</summary>

Perform an HTTP POST request via the browser context.

**Params:**

| Name | Type | Required | Description |
|------|------|:--------:|-------------|
| `url` | string | ✅ | URL to post to |
| `body` | string | ✅ | Request body (string) |
| `headers` | string | | Optional headers as JSON object |

**Response:** JSON — response body, status, headers.
</details>

<details>
<summary><strong><code>links</code></strong> — Extract link graph</summary>

Extract all links from the current page and build a link graph.

**Params:**

| Name | Type | Required | Description |
|------|------|:--------:|-------------|
| `base_url` | string | ✅ | Base URL for resolving relative links |

**Response:** JSON array of link edges.
</details>

<details>
<summary><strong><code>graph</code></strong> — Analyze link graph</summary>

Analyze a link graph for structure, connectivity, and statistics.

**Params:**

| Name | Type | Required | Description |
|------|------|:--------:|-------------|
| `edges` | string | ✅ | Link edges as JSON array: `[{"source":"...","target":"..."}]` |

**Response:** JSON — graph analysis with node counts, clusters, PageRank-like stats.
</details>

<details>
<summary><strong><code>net_capture</code></strong> — Capture network traffic</summary>

Inject interceptors and capture all network traffic (fetch + XHR) for a specified duration.

**Params:**

| Name | Type | Required | Description |
|------|------|:--------:|-------------|
| `duration_seconds` | integer | | Capture duration in seconds (default `10`) |
| `api_only` | boolean | | Only capture API calls, exclude static assets (default `true`) |

**Response:**
```json
{ "endpoints": [ /* ApiEndpoint objects */ ], "count": 15, "duration_seconds": 10 }
```
</details>

<details>
<summary><strong><code>net_analyze</code></strong> — Analyze API traffic</summary>

Analyze captured API endpoints to infer schemas, auth patterns, and endpoint templates.

**Params:**

| Name | Type | Required | Description |
|------|------|:--------:|-------------|
| `capture` | string | ✅ | Network capture data JSON (from `net_capture` output) |

**Response:** JSON `ApiSchema` with `base_url`, `endpoints[]`, `auth_pattern`, `total_requests`, `unique_endpoints`.
</details>

<details>
<summary><strong><code>net_sdk</code></strong> — Generate API SDK</summary>

Generate a typed API client SDK from an analyzed API schema.

**Params:**

| Name | Type | Required | Description |
|------|------|:--------:|-------------|
| `schema` | string | ✅ | API schema JSON (from `net_analyze` output) |
| `language` | string | | Target language: `typescript` or `python` (default `typescript`) |

**Response:** `{ "language": "typescript", "code": "...", "endpoints_covered": 8 }`
</details>

<details>
<summary><strong><code>net_mock</code></strong> — Generate mock server</summary>

Generate a mock server configuration from captured endpoints.

**Params:**

| Name | Type | Required | Description |
|------|------|:--------:|-------------|
| `endpoints` | string | ✅ | Captured endpoints JSON (from `net_capture`) |
| `port` | integer | | Port for mock server (default `3001`) |

**Response:** JSON mock server configuration.
</details>

<details>
<summary><strong><code>net_replay</code></strong> — Replay captured requests</summary>

Generate a replay sequence from captured network requests.

**Params:**

| Name | Type | Required | Description |
|------|------|:--------:|-------------|
| `endpoints` | string | ✅ | Captured endpoints JSON (from `net_capture`) |
| `name` | string | | Name for the replay sequence (default `replay_sequence`) |

**Response:** JSON replay sequence definition.
</details>

---

### 6. `secure`

Cryptography, encrypted storage, and WebAuthn passkey management.

#### Actions

| Action | Params | Description |
|--------|--------|-------------|
| `encrypt` | `{plaintext, password}` | AES-256-GCM encryption |
| `decrypt` | `{ciphertext, password}` | AES-256-GCM decryption |
| `pkce` | — | Generate PKCE S256 challenge pair |
| `totp` | `{secret}` | Generate 6-digit TOTP code |
| `kv_set` | `{key, value}` | Store encrypted key-value pair |
| `kv_get` | `{key}` | Retrieve value by key |
| `kv_list` | — | List all stored keys |
| `passkey_enable` | `{protocol?, transport?}` | Enable virtual WebAuthn authenticator |
| `passkey_add` | `{credential_id, rp_id, user_handle?}` | Add passkey credential |
| `passkey_list` | — | List stored passkeys |
| `passkey_log` | — | Get WebAuthn operation log |
| `passkey_disable` | — | Disable authenticator |
| `passkey_remove` | `{credential_id}` | Remove passkey by ID |

#### Action Details

<details>
<summary><strong><code>encrypt</code></strong> — AES-256-GCM encryption</summary>

Encrypt a plaintext string using AES-256-GCM with PBKDF2 key derivation.

**Params:**

| Name | Type | Required | Description |
|------|------|:--------:|-------------|
| `plaintext` | string | ✅ | Plaintext string to encrypt |
| `password` | string | ✅ | Password for key derivation |

**Response:** Base64-encoded ciphertext (salt + nonce + ciphertext).
</details>

<details>
<summary><strong><code>decrypt</code></strong> — AES-256-GCM decryption</summary>

Decrypt a previously encrypted ciphertext.

**Params:**

| Name | Type | Required | Description |
|------|------|:--------:|-------------|
| `ciphertext` | string | ✅ | Base64-encoded ciphertext (salt + nonce + ciphertext) |
| `password` | string | ✅ | Password for key derivation |

**Response:** Decrypted plaintext string.
</details>

<details>
<summary><strong><code>pkce</code></strong> — Generate PKCE challenge</summary>

Generate a PKCE S256 code verifier and code challenge pair for OAuth flows.

**Params:** None

**Response:**
```json
{ "code_verifier": "...", "code_challenge": "..." }
```
</details>

<details>
<summary><strong><code>totp</code></strong> — Generate TOTP code</summary>

Generate a 6-digit TOTP code from a Base32-encoded secret.

**Params:**

| Name | Type | Required | Description |
|------|------|:--------:|-------------|
| `secret` | string | ✅ | Base32-encoded TOTP secret |

**Response:** 6-digit TOTP code string.
</details>

<details>
<summary><strong><code>kv_set</code></strong> — Store encrypted value</summary>

Store a key-value pair in the encrypted store.

**Params:**

| Name | Type | Required | Description |
|------|------|:--------:|-------------|
| `key` | string | ✅ | Storage key |
| `value` | string | ✅ | Value to store |

**Response:** `"stored key \"<key>\""`
</details>

<details>
<summary><strong><code>kv_get</code></strong> — Retrieve value</summary>

Retrieve a value from the encrypted store by key.

**Params:**

| Name | Type | Required | Description |
|------|------|:--------:|-------------|
| `key` | string | ✅ | Storage key to retrieve |

**Response:** The stored value string, or `"key \"<key>\" not found"`.
</details>

<details>
<summary><strong><code>kv_list</code></strong> — List all keys</summary>

List all keys in the encrypted store.

**Params:** None

**Response:** JSON array of key strings.
</details>

<details>
<summary><strong><code>passkey_enable</code></strong> — Enable WebAuthn authenticator</summary>

Enable a virtual WebAuthn authenticator in the browser for passkey testing.

**Params:**

| Name | Type | Required | Description |
|------|------|:--------:|-------------|
| `protocol` | string | | Protocol: `ctap2` or `u2f` (default `ctap2`) |
| `transport` | string | | Transport: `internal`, `usb`, `nfc`, `ble` (default `internal`) |

**Response:** `"Virtual authenticator enabled"`
</details>

<details>
<summary><strong><code>passkey_add</code></strong> — Add passkey credential</summary>

Add a virtual passkey credential to the authenticator.

**Params:**

| Name | Type | Required | Description |
|------|------|:--------:|-------------|
| `credential_id` | string | ✅ | Base64url-encoded credential ID |
| `rp_id` | string | ✅ | Relying party domain (e.g. `example.com`) |
| `user_handle` | string | | Optional base64url-encoded user handle |

**Response:** `"Credential added"`
</details>

<details>
<summary><strong><code>passkey_list</code></strong> — List passkeys</summary>

List all virtual passkey credentials.

**Params:** None

**Response:** JSON array of credential objects.
</details>

<details>
<summary><strong><code>passkey_log</code></strong> — WebAuthn operation log</summary>

Get the log of WebAuthn operations (registrations, authentications).

**Params:** None

**Response:** JSON array of WebAuthn log entries.
</details>

<details>
<summary><strong><code>passkey_disable</code></strong> — Disable authenticator</summary>

Disable the virtual WebAuthn authenticator.

**Params:** None

**Response:** `"Virtual authenticator disabled"`
</details>

<details>
<summary><strong><code>passkey_remove</code></strong> — Remove passkey</summary>

Remove a specific virtual passkey credential by ID.

**Params:**

| Name | Type | Required | Description |
|------|------|:--------:|-------------|
| `credential_id` | string | ✅ | Credential ID to remove |

**Response:** `{ "removed": true }`
</details>

---

### 7. `computer`

AI computer-use protocol, smart element resolution, and browser pool management.

This tool implements the Anthropic Computer Use protocol for AI agent interactions, plus smart fuzzy element finding and a browser pool for multi-instance management.

#### Actions

| Action | Params | Description |
|--------|--------|-------------|
| `act` | `{action, include_screenshot?}` | Perform a computer-use action |
| `observe` | `{include_screenshot?}` | Observe screen state |
| `batch` | `{actions, include_screenshots?, stop_on_error?}` | Execute multiple actions in sequence |
| `smart_find` | `{query}` | Find element by fuzzy description |
| `smart_click` | `{query}` | Click element by fuzzy description |
| `smart_fill` | `{query, value}` | Fill input by fuzzy description |
| `pool_list` | — | List browser pool instances |
| `pool_status` | — | Get pool status and stats |

#### Action Details

<details>
<summary><strong><code>act</code></strong> — Perform computer-use action</summary>

Execute a single computer-use action. The `action` param is a JSON object with a `type` field.

**Params:**

| Name | Type | Required | Description |
|------|------|:--------:|-------------|
| `action` | object | ✅ | Action object with `type` field. Types: `click` (x,y / selector / ref), `type` (text), `key` (key name), `scroll` (x, y, delta_x, delta_y), `navigate` (url), `wait` (ms), `screenshot`, `observe`, `evaluate` (expression), `fill` (selector, value), `select` (selector, value), `drag` (from_x, from_y, to_x, to_y), `done` (result), `fail` (reason) |
| `include_screenshot` | boolean | | Include screenshot in observation (default `false`) |

**Response:** JSON `ActionResult` with `success`, `data`, and optional `screenshot`.
</details>

<details>
<summary><strong><code>observe</code></strong> — Observe screen state</summary>

Get the current screen observation (page structure, interactive elements, URL, title).

**Params:**

| Name | Type | Required | Description |
|------|------|:--------:|-------------|
| `include_screenshot` | boolean | | Include base64 screenshot in observation (default `false`) |

**Response:** JSON observation with page state, elements, and optional screenshot.
</details>

<details>
<summary><strong><code>batch</code></strong> — Batch computer-use actions</summary>

Execute multiple computer-use actions in sequence.

**Params:**

| Name | Type | Required | Description |
|------|------|:--------:|-------------|
| `actions` | array | ✅ | List of action objects (each with a `type` field) |
| `include_screenshots` | boolean | | Include screenshots between actions (default `false`) |
| `stop_on_error` | boolean | | Stop on first error (default `true`) |

**Response:**
```json
{ "total": 5, "executed": 5, "results": [ /* ActionResult[] */ ] }
```
</details>

<details>
<summary><strong><code>smart_find</code></strong> — Find element by description</summary>

Find an element using fuzzy text matching, CSS selectors, or natural language descriptions.

**Params:**

| Name | Type | Required | Description |
|------|------|:--------:|-------------|
| `query` | string | ✅ | Fuzzy text, CSS selector, or element description to search for |

**Response:**
```json
{ "query": "Login button", "matches": [ /* element matches */ ], "count": 2 }
```
</details>

<details>
<summary><strong><code>smart_click</code></strong> — Click by description</summary>

Find and click an element using fuzzy matching.

**Params:**

| Name | Type | Required | Description |
|------|------|:--------:|-------------|
| `query` | string | ✅ | Fuzzy text, CSS selector, or element description to click |

**Response:**
```json
{ "clicked": "button#login", "confidence": 0.95, "strategy": "aria-label" }
```
</details>

<details>
<summary><strong><code>smart_fill</code></strong> — Fill input by description</summary>

Find an input by description and type a value into it.

**Params:**

| Name | Type | Required | Description |
|------|------|:--------:|-------------|
| `query` | string | ✅ | Fuzzy text, CSS selector, or description of the input |
| `value` | string | ✅ | Value to type into the matched input |

**Response:**
```json
{ "filled": "input#email", "value_length": 15, "confidence": 0.92, "strategy": "placeholder" }
```
</details>

<details>
<summary><strong><code>pool_list</code></strong> — List browser pool instances</summary>

List all browser instances in the pool.

**Params:** None

**Response:** `{ "instances": [...], "count": 3 }`
</details>

<details>
<summary><strong><code>pool_status</code></strong> — Pool status</summary>

Get browser pool utilization stats.

**Params:** None

**Response:**
```json
{ "size": 5, "max_size": 10, "idle": 3, "busy": 2 }
```
</details>

---

### 8. `memory`

Persistent agent memory — store, recall, and search knowledge across sessions.

Memory is persisted to `~/.onecrawl/agent_memory.json` and survives across sessions.

#### Actions

| Action | Params | Description |
|--------|--------|-------------|
| `store` | `{key, value, category?, domain?}` | Store a memory entry |
| `recall` | `{key}` | Recall a memory by key |
| `search` | `{query, category?, domain?}` | Search memories |
| `forget` | `{key?, domain?}` | Delete memory entries |
| `domain_strategy` | `{domain, strategy?}` | Store or recall domain-specific strategy |
| `stats` | — | Get memory statistics |

#### Action Details

<details>
<summary><strong><code>store</code></strong> — Store a memory</summary>

Store a key-value memory entry with optional categorization and domain association.

**Params:**

| Name | Type | Required | Description |
|------|------|:--------:|-------------|
| `key` | string | ✅ | Unique key for this memory entry |
| `value` | any | ✅ | JSON value to store |
| `category` | string | | Category: `page_visit`, `element_pattern`, `domain_strategy`, `retry_knowledge`, `user_preference`, `selector_mapping`, `error_pattern`, `custom` |
| `domain` | string | | Domain this memory is associated with (e.g. `example.com`) |

**Response:** `{ "stored": "<key>", "category": "Custom" }`
</details>

<details>
<summary><strong><code>recall</code></strong> — Recall a memory</summary>

Retrieve a specific memory entry by key.

**Params:**

| Name | Type | Required | Description |
|------|------|:--------:|-------------|
| `key` | string | ✅ | Key of the memory entry to recall |

**Response:**
```json
{
  "key": "login-form-selectors",
  "value": { ... },
  "category": "SelectorMapping",
  "domain": "example.com",
  "access_count": 5,
  "created_at": "...",
  "accessed_at": "..."
}
```
Or `{ "key": "...", "found": false }` if not found.
</details>

<details>
<summary><strong><code>search</code></strong> — Search memories</summary>

Search memory entries by query string, optionally filtered by category and domain.

**Params:**

| Name | Type | Required | Description |
|------|------|:--------:|-------------|
| `query` | string | ✅ | Search query (matches against keys and values) |
| `category` | string | | Filter by category |
| `domain` | string | | Filter by domain |

**Response:**
```json
{ "query": "login", "count": 3, "results": [ /* memory entries */ ] }
```
</details>

<details>
<summary><strong><code>forget</code></strong> — Delete memories</summary>

Delete memory entries by key, by domain, or clear all.

**Params:**

| Name | Type | Required | Description |
|------|------|:--------:|-------------|
| `key` | string | | Key to forget |
| `domain` | string | | Domain to clear all memories for |

> If both are omitted, clears **all** memories.

**Response:** `{ "removed": true, "key": "..." }` or `{ "removed": 5, "domain": "..." }` or `{ "removed": 42, "cleared": "all" }`
</details>

<details>
<summary><strong><code>domain_strategy</code></strong> — Domain strategy</summary>

Store or recall a domain-specific automation strategy (login selectors, navigation patterns, anti-bot level, etc.).

**Params:**

| Name | Type | Required | Description |
|------|------|:--------:|-------------|
| `domain` | string | ✅ | Domain to store/recall strategy for |
| `strategy` | object | | Strategy data as JSON (omit to recall existing) |

**Response:** When storing: `{ "stored": true, "domain": "..." }`. When recalling: the strategy object, or `{ "domain": "...", "found": false }`.
</details>

<details>
<summary><strong><code>stats</code></strong> — Memory statistics</summary>

Get memory usage statistics.

**Params:** None

**Response:**
```json
{
  "total_entries": 42,
  "max_entries": 10000,
  "categories": { "SelectorMapping": 15, "PageVisit": 20, ... },
  "domains": { "example.com": 8, ... },
  "utilization": "0.4%"
}
```
</details>

---

### 9. `automate`

Workflow automation, AI task planning, and execution control.

#### Actions

| Action | Params | Description |
|--------|--------|-------------|
| `workflow_validate` | `{workflow}` | Validate a workflow definition |
| `workflow_run` | `{workflow, variables?}` | Execute a workflow |
| `plan` | `{goal, context?}` | Generate automation plan from natural language |
| `execute` | `{plan, context?, max_retries?}` | Execute a generated plan |
| `patterns` | — | List available automation patterns |
| `rate_limit` | `{max_per_second?, max_per_minute?}` | Check/configure rate limiter |
| `retry` | `{url, operation, payload?}` | Enqueue a retry with exponential backoff |

#### Action Details

<details>
<summary><strong><code>workflow_validate</code></strong> — Validate workflow</summary>

Parse and validate a workflow definition without executing it.

**Params:**

| Name | Type | Required | Description |
|------|------|:--------:|-------------|
| `workflow` | string | ✅ | Workflow definition as JSON string |

**Response:**
```json
{ "valid": true, "name": "my-workflow", "steps": 5, "variables": ["url", "username"] }
```
Or `{ "valid": false, "errors": ["..."] }` on validation failure.
</details>

<details>
<summary><strong><code>workflow_run</code></strong> — Execute workflow</summary>

Execute a workflow definition. Supports variable interpolation, conditional steps, loops, error handling, and sub-workflows.

**Params:**

| Name | Type | Required | Description |
|------|------|:--------:|-------------|
| `workflow` | string | ✅ | Workflow JSON string, or file path to workflow JSON |
| `variables` | object | | Override variables as key-value pairs |

**Response:**
```json
{
  "name": "login-flow",
  "status": "success",
  "total_duration_ms": 4500,
  "steps_succeeded": 5,
  "steps_failed": 0,
  "steps_skipped": 1,
  "steps": [ /* StepResult[] */ ],
  "variables": { /* final variable state */ }
}
```
</details>

<details>
<summary><strong><code>plan</code></strong> — Generate automation plan</summary>

Generate a step-by-step automation plan from a natural language goal. Uses built-in patterns and context to create an executable plan.

**Params:**

| Name | Type | Required | Description |
|------|------|:--------:|-------------|
| `goal` | string | ✅ | Natural language goal (e.g. `log into Gmail and check inbox`) |
| `context` | object | | Additional context as key-value pairs (url, credentials, etc.) |

**Response:** JSON `TaskPlan` with `goal`, `steps[]`, `estimated_duration`, `complexity`.
</details>

<details>
<summary><strong><code>execute</code></strong> — Execute plan</summary>

Execute a generated task plan (from `plan` action) or generate and execute from a natural language goal.

**Params:**

| Name | Type | Required | Description |
|------|------|:--------:|-------------|
| `plan` | string | ✅ | Task plan JSON (from `plan` output), or natural language goal |
| `context` | object | | Additional context/variables |
| `max_retries` | integer | | Maximum retries per step (default `2`) |

**Response:**
```json
{
  "goal": "...",
  "status": "success|partial_success|failed",
  "steps_completed": 5,
  "steps_total": 6,
  "steps_results": [ /* StepExecutionResult[] */ ],
  "retries_used": 1,
  "total_duration_ms": 8000
}
```
</details>

<details>
<summary><strong><code>patterns</code></strong> — List automation patterns</summary>

List all built-in automation patterns (login, search, form fill, etc.).

**Params:** None

**Response:**
```json
{
  "patterns": [
    { "category": "authentication", "keywords": ["login", "sign in"], "steps": 4, "template": ["Navigate to login page", ...] }
  ],
  "count": 12
}
```
</details>

<details>
<summary><strong><code>rate_limit</code></strong> — Rate limiter</summary>

Check if the next action is allowed under the current rate limit, and optionally configure limits.

**Params:**

| Name | Type | Required | Description |
|------|------|:--------:|-------------|
| `max_per_second` | number | | Max requests per second (default `2.0`) |
| `max_per_minute` | number | | Max requests per minute (default `60.0`) |

**Response:**
```json
{ "can_proceed": true, "stats": { "requests_this_second": 1, "requests_this_minute": 15, ... } }
```
</details>

<details>
<summary><strong><code>retry</code></strong> — Enqueue retry</summary>

Add a failed operation to the retry queue with exponential backoff.

**Params:**

| Name | Type | Required | Description |
|------|------|:--------:|-------------|
| `url` | string | ✅ | URL to retry |
| `operation` | string | ✅ | Operation label (e.g. `navigate`, `extract`) |
| `payload` | string | | Optional payload string |

**Response:**
```json
{ "id": "retry-abc123", "queue_stats": { "pending": 3, "total": 5, "max_retries": 3 } }
```
</details>

---

### 10. `perf`

Performance monitoring, budgets, regression detection, and visual regression testing.

#### Actions

| Action | Params | Description |
|--------|--------|-------------|
| `audit` | `{url?}` | Collect Core Web Vitals and performance metrics |
| `budget` | `{budget, url?}` | Check performance against budget |
| `compare` | `{baseline, current, threshold_pct?}` | Detect performance regressions |
| `trace` | `{url, settle_ms?}` | Full performance trace with navigation |
| `vrt_run` | `{suite}` | Run visual regression test suite |
| `vrt_compare` | `{url, name, threshold?, selector?, full_page?, baseline_dir?}` | Compare screenshot against baseline |
| `vrt_update` | `{test_name, baseline_dir?}` | Update VRT baseline |

#### Action Details

<details>
<summary><strong><code>audit</code></strong> — Performance audit</summary>

Collect Core Web Vitals (LCP, FID, CLS) and other performance metrics from the current or specified page.

**Params:**

| Name | Type | Required | Description |
|------|------|:--------:|-------------|
| `url` | string | | URL to audit (navigates there first). If omitted, audits current page |

**Response:**
```json
{
  "url": "https://example.com",
  "timestamp": 1700000000,
  "vitals": { "lcp": 1.2, "fid": 50, "cls": 0.05, ... },
  "ratings": { "lcp": "good", "fid": "good", "cls": "good" },
  "navigation_timing": { ... },
  "resource_count": { ... },
  "memory": { ... }
}
```
</details>

<details>
<summary><strong><code>budget</code></strong> — Performance budget check</summary>

Check current page performance against a defined budget.

**Params:**

| Name | Type | Required | Description |
|------|------|:--------:|-------------|
| `budget` | string | ✅ | Budget definition as JSON |
| `url` | string | | URL to check (uses current page if omitted) |

**Response:** JSON with pass/fail for each budget metric, violations list.
</details>

<details>
<summary><strong><code>compare</code></strong> — Regression detection</summary>

Compare two performance snapshots to detect regressions.

**Params:**

| Name | Type | Required | Description |
|------|------|:--------:|-------------|
| `baseline` | string | ✅ | Baseline snapshot JSON (from `audit` output) |
| `current` | string | ✅ | Current snapshot JSON (from `audit` output) |
| `threshold_pct` | number | | Regression threshold percentage (default `10`) |

**Response:**
```json
{
  "baseline_url": "...",
  "current_url": "...",
  "threshold_pct": 10,
  "regressions": [ /* metric regressions */ ],
  "regressed": false,
  "count": 0
}
```
</details>

<details>
<summary><strong><code>trace</code></strong> — Full performance trace</summary>

Navigate to a URL and perform a full performance trace, waiting for the page to settle before collecting metrics.

**Params:**

| Name | Type | Required | Description |
|------|------|:--------:|-------------|
| `url` | string | ✅ | URL to trace |
| `settle_ms` | integer | | Wait time in ms after load for late metrics (default `3000`) |

**Response:**
```json
{
  "url": "...",
  "trace_duration_ms": 5000,
  "settle_ms": 3000,
  "vitals": { ... },
  "ratings": { ... },
  "navigation_timing": { ... },
  "resource_count": { ... },
  "memory": { ... }
}
```
</details>

<details>
<summary><strong><code>vrt_run</code></strong> — Run VRT suite</summary>

Run a full visual regression test suite, comparing screenshots against baselines.

**Params:**

| Name | Type | Required | Description |
|------|------|:--------:|-------------|
| `suite` | string | ✅ | VRT suite definition as JSON, or path to suite JSON file |

**Response:**
```json
{
  "suite_name": "homepage",
  "total": 5,
  "passed": 4,
  "failed": 0,
  "new_baselines": 1,
  "errors": 0,
  "duration_ms": 12000,
  "results": [ /* VrtTestResult[] */ ],
  "junit_xml": "<?xml ..."
}
```
</details>

<details>
<summary><strong><code>vrt_compare</code></strong> — Compare screenshot</summary>

Navigate to a URL, take a screenshot, and compare it against a stored baseline.

**Params:**

| Name | Type | Required | Description |
|------|------|:--------:|-------------|
| `url` | string | ✅ | URL to capture and compare |
| `name` | string | ✅ | Test name for baseline lookup |
| `threshold` | number | | Mismatch threshold percentage (default `0.1`) |
| `selector` | string | | CSS selector for element screenshot |
| `full_page` | boolean | | Capture full scrollable page |
| `baseline_dir` | string | | Baseline directory (default `.vrt/baselines`) |

**Response:** JSON `VrtTestResult` with `status` (passed/failed/new_baseline), `mismatch_percentage`, `diff_path`.
</details>

<details>
<summary><strong><code>vrt_update</code></strong> — Update VRT baseline</summary>

Promote the current screenshot to become the new baseline for a test.

**Params:**

| Name | Type | Required | Description |
|------|------|:--------:|-------------|
| `test_name` | string | ✅ | Test name to update baseline for |
| `baseline_dir` | string | | Baseline directory (default `.vrt/baselines`) |

**Response:**
```json
{ "updated": true, "test_name": "homepage-hero", "baseline_path": ".vrt/baselines/homepage-hero.png", "bytes": 54321 }
```
</details>

---

## Examples

### Navigate and screenshot

```json
// Step 1: Navigate
{ "action": "goto", "params": { "url": "https://example.com" } }

// Step 2: Screenshot
{ "action": "screenshot", "params": { "full_page": true } }
```

### Scrape with pagination

```json
{
  "action": "stream",
  "params": {
    "item_selector": ".product-card",
    "fields": "[{\"name\":\"title\",\"selector\":\"h2\",\"extract\":\"text\"},{\"name\":\"price\",\"selector\":\".price\",\"extract\":\"text\"}]",
    "pagination": "{\"next_selector\":\".next-page\",\"max_pages\":5,\"delay_ms\":1000}"
  }
}
```

### Stealth + crawl workflow

```json
// 1. Apply stealth
{ "action": "inject", "params": {} }

// 2. Apply fingerprint
{ "action": "fingerprint", "params": {} }

// 3. Block trackers
{ "action": "block_domains", "params": { "category": "trackers" } }

// 4. Crawl (use `crawl` tool)
{ "action": "spider", "params": { "start_urls": ["https://example.com"], "max_pages": 100 } }
```

### Execute a command chain

```json
{
  "action": "execute_chain",
  "params": {
    "commands": [
      { "tool": "navigation.goto", "args": { "url": "https://example.com/login" } },
      { "tool": "navigation.type", "args": { "selector": "#email", "text": "user@example.com" } },
      { "tool": "navigation.type", "args": { "selector": "#password", "text": "secret" } },
      { "tool": "navigation.click", "args": { "selector": "button[type=submit]" } }
    ]
  }
}
```

### Performance budget check

```json
{
  "action": "budget",
  "params": {
    "url": "https://example.com",
    "budget": "{\"lcp_ms\":2500,\"cls\":0.1,\"fid_ms\":100,\"resource_count\":{\"script\":10,\"image\":20}}"
  }
}
```

---

## Migration from v2 (108 tools → 10 tools)

The old 108-tool interface mapped 1:1 to individual operations. The new interface groups them by domain:

| Old Tool Name | New Tool | Action |
|--------------|----------|--------|
| `navigation.goto` | `browser` | `goto` |
| `navigation.click` | `browser` | `click` |
| `navigation.type` | `browser` | `type` |
| `navigation.screenshot` | `browser` | `screenshot` |
| `scraping.css` | `browser` | `css` |
| `scraping.xpath` | `browser` | `xpath` |
| `scraping.text` | `browser` | `text` |
| `scraping.html` | `browser` | `html` |
| `scraping.markdown` | `browser` | `markdown` |
| `scraping.structured` | `browser` | `structured` |
| `scraping.stream` | `browser` | `stream` |
| `scraping.detect_forms` | `browser` | `detect_forms` |
| `scraping.fill_form` | `browser` | `fill_form` |
| `crawling.spider` | `crawl` | `spider` |
| `crawling.robots` | `crawl` | `robots` |
| `crawling.sitemap` | `crawl` | `sitemap` |
| `crawling.snapshot` | `crawl` | `dom_snapshot` |
| `crawling.compare` | `crawl` | `dom_compare` |
| `stealth.inject` | `stealth` | `inject` |
| `stealth.test` | `stealth` | `test` |
| `stealth.fingerprint` | `stealth` | `fingerprint` |
| `stealth.block_domains` | `stealth` | `block_domains` |
| `stealth.detect_captcha` | `stealth` | `detect_captcha` |
| `crypto.encrypt` | `secure` | `encrypt` |
| `crypto.decrypt` | `secure` | `decrypt` |
| `crypto.pkce` | `secure` | `pkce` |
| `crypto.totp` | `secure` | `totp` |
| `storage.set` | `secure` | `kv_set` |
| `storage.get` | `secure` | `kv_get` |
| `storage.list` | `secure` | `kv_list` |
| `parser.accessibility` | `browser` | `parse_a11y` |
| `parser.selector` | `browser` | `parse_selector` |
| `parser.text` | `browser` | `parse_text` |
| `parser.links` | `browser` | `parse_links` |
| `data.pipeline` | `data` | `pipeline` |
| `data.http_get` | `data` | `http_get` |
| `data.http_post` | `data` | `http_post` |
| `data.links` | `data` | `links` |
| `data.graph` | `data` | `graph` |
| `automation.rate_limit` | `automate` | `rate_limit` |
| `automation.retry` | `automate` | `retry` |
| `agent.execute_chain` | `agent` | `execute_chain` |
| `agent.element_screenshot` | `agent` | `element_screenshot` |
| `computer.act` | `computer` | `act` |
| `computer.observe` | `computer` | `observe` |
| `computer.batch` | `computer` | `batch` |
| `computer.smart_find` | `computer` | `smart_find` |
| `computer.smart_click` | `computer` | `smart_click` |
| `computer.smart_fill` | `computer` | `smart_fill` |
| `memory.store` | `memory` | `store` |
| `memory.recall` | `memory` | `recall` |
| `memory.search` | `memory` | `search` |
| `memory.forget` | `memory` | `forget` |
| `memory.domain_strategy` | `memory` | `domain_strategy` |
| `memory.stats` | `memory` | `stats` |
| `automation.workflow_validate` | `automate` | `workflow_validate` |
| `automation.workflow_run` | `automate` | `workflow_run` |
| `automation.plan` | `automate` | `plan` |
| `automation.execute` | `automate` | `execute` |
| `automation.patterns` | `automate` | `patterns` |
| `perf.audit` | `perf` | `audit` |
| `perf.budget` | `perf` | `budget` |
| `perf.compare` | `perf` | `compare` |
| `perf.trace` | `perf` | `trace` |
| `perf.vrt_run` | `perf` | `vrt_run` |
| `perf.vrt_compare` | `perf` | `vrt_compare` |
| `perf.vrt_update` | `perf` | `vrt_update` |
| `stealth.inject` | `stealth` | `inject` |
| `stealth.test` | `stealth` | `test` |
| `stealth.fingerprint` | `stealth` | `fingerprint` |
| `stealth.block_domains` | `stealth` | `block_domains` |
| `stealth.detect_captcha` | `stealth` | `detect_captcha` |
| `agent.api_capture_start` | `agent` | `api_capture_start` |
| `agent.api_capture_summary` | `agent` | `api_capture_summary` |
| `agent.iframe_list` | `agent` | `iframe_list` |
| `agent.iframe_snapshot` | `agent` | `iframe_snapshot` |
| `agent.connect_remote` | `agent` | `connect_remote` |
| `agent.safety_set` | `agent` | `safety_set` |
| `agent.safety_status` | `agent` | `safety_status` |
| `agent.skills_list` | `agent` | `skills_list` |
| `agent.screencast_start` | `agent` | `screencast_start` |
| `agent.screencast_stop` | `agent` | `screencast_stop` |
| `agent.screencast_frame` | `agent` | `screencast_frame` |
| `agent.recording_start` | `agent` | `recording_start` |
| `agent.recording_stop` | `agent` | `recording_stop` |
| `agent.recording_status` | `agent` | `recording_status` |
| `agent.ios_devices` | `agent` | `ios_devices` |
| `agent.ios_connect` | `agent` | `ios_connect` |
| `agent.ios_navigate` | `agent` | `ios_navigate` |
| `agent.ios_tap` | `agent` | `ios_tap` |
| `agent.ios_screenshot` | `agent` | `ios_screenshot` |
| `passkey.enable` | `secure` | `passkey_enable` |
| `passkey.add` | `secure` | `passkey_add` |
| `passkey.list` | `secure` | `passkey_list` |
| `passkey.log` | `secure` | `passkey_log` |
| `passkey.disable` | `secure` | `passkey_disable` |
| `passkey.remove` | `secure` | `passkey_remove` |
| `net.capture` | `data` | `net_capture` |
| `net.analyze` | `data` | `net_analyze` |
| `net.sdk` | `data` | `net_sdk` |
| `net.mock` | `data` | `net_mock` |
| `net.replay` | `data` | `net_replay` |
| `crawl.spider` | `crawl` | `spider` |
| `crawl.robots` | `crawl` | `robots` |
| `crawl.sitemap` | `crawl` | `sitemap` |
| `crawl.dom_snapshot` | `crawl` | `dom_snapshot` |
| `crawl.dom_compare` | `crawl` | `dom_compare` |
| `pool.list` | `computer` | `pool_list` |
| `pool.status` | `computer` | `pool_status` |

---

## Error Handling

All tools return errors in a consistent format:

```json
{
  "error": "descriptive error message"
}
```

### Common Error Patterns

| Error | Cause | Resolution |
|-------|-------|------------|
| `"Unknown action: xyz"` | Invalid action name | Check the action name in the reference tables above |
| `"Browser not initialized"` | CDP action called before `goto` | Call `browser` with `goto` action first |
| `"Missing required param: x"` | Required parameter omitted | Add the missing parameter to `params` |
| `"Element not found"` | Selector/ref matches nothing | Verify selector; use `snapshot` to inspect the page |
| `"Navigation timeout"` | Page load exceeded timeout | Increase `timeout` param or check URL accessibility |
| `"Accessibility ref not found"` | Invalid `@eN` reference | Call `snapshot` to refresh refs before using them |

---

## Notes

- **Browser lazy init**: The browser is started on the first action that needs CDP (e.g., `goto`, `click`). Offline actions like `parse_a11y` never start a browser.
- **Accessibility refs**: After calling `snapshot`, elements are assigned refs like `@e1`, `@e2`. Use these in `click`, `type`, `screenshot` (via `element` param) instead of CSS selectors for more reliable targeting.
- **Memory persistence**: Agent memory is stored at `~/.onecrawl/agent_memory.json` and survives across sessions.
- **Stealth patches**: Must call `stealth` → `inject` before navigating to the target site for maximum effectiveness.
- **Rate limiting**: The `automate` → `rate_limit` action configures per-domain request throttling that applies to all subsequent navigation and network actions.

---

*Auto-generated from OneCrawl MCP server source (`onecrawl-mcp-rs`). Total: 10 tools, 108 actions.*
