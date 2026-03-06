# OneCrawl MCP API Reference

> **10 consolidated tools • 239 actions • Action-based dispatch**

All browser automation, crawling, scraping, security, and AI orchestration capabilities are accessed through 10 super-tools. Each tool accepts a uniform `{ action, params }` interface.

---

## Quick Reference

| Tool | Actions | Description |
|------|:-------:|-------------|
| [`browser`](#1-browser) | 92 | Navigation, interaction, scraping, content extraction, offline HTML parsing, multi-tab, DOM events, cookies & storage, network interception, console & errors, device emulation, file operations, shadow DOM, session context, smart forms, self-healing selectors, event reactions, service worker/PWA, offline mode |
| [`crawl`](#2-crawl) | 5 | Web crawling, robots.txt, sitemaps, DOM snapshots |
| [`agent`](#3-agent) | 37 | Command chains, API capture, iframes, remote CDP, safety, screencast, recording, iOS, task decomposition, vision observation, WCAG auditing, accessibility tree, screen reader simulation |
| [`stealth`](#4-stealth) | 12 | Anti-detection patches, fingerprinting, CAPTCHA detection, human behavior simulation |
| [`data`](#5-data) | 26 | Data pipelines, HTTP client, link graphs, network intelligence, structured extraction, WebSocket, SSE, GraphQL subscriptions |
| [`secure`](#6-secure) | 21 | Encryption, PKCE, TOTP, KV store, WebAuthn, OAuth2, session/form auth, MFA |
| [`computer`](#7-computer) | 14 | AI computer-use, smart element resolution, browser pool, multi-browser fleet |
| [`memory`](#8-memory) | 6 | Persistent agent memory across sessions |
| [`automate`](#9-automate) | 19 | Workflow DSL, AI task planning, rate limiting, retry queues, error recovery, session checkpoints, workflow control flow |
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
| | | **Multi-Tab** |
| `new_tab` | `{url?}` | Open new tab, optionally navigate to URL |
| `list_tabs` | — | List all tabs with index, URL, title, active state |
| `switch_tab` | `{index}` | Switch active page by index (0-based) |
| `close_tab` | `{index?}` | Close tab by index (defaults to active) |
| | | **DOM Events** |
| `observe_mutations` | `{selector?, child_list?, attributes?, character_data?, subtree?}` | Start MutationObserver |
| `get_mutations` | — | Get recorded DOM mutations since last call |
| `stop_mutations` | — | Disconnect MutationObserver |
| `wait_for_event` | `{event, selector?, timeout?}` | Wait for DOM event (promise-based, default 30 s timeout) |
| | | **Cookie & Storage** |
| `cookies_get` | `{domain?, name?}` | Get cookies (non-HttpOnly only via `document.cookie`) |
| `cookies_set` | `{name, value, domain, path?, secure?, http_only?, same_site?, expires?}` | Set cookie |
| `cookies_clear` | `{domain?}` | Clear cookies |
| `storage_get` | `{key, storage_type?}` | Get localStorage/sessionStorage value |
| `storage_set` | `{key, value, storage_type?}` | Set localStorage/sessionStorage value |
| `export_session` | `{cookies?, local_storage?, session_storage?}` | Export full session state as JSON |
| `import_session` | `{state}` | Import session state from export JSON |
| | | **Network Interception** |
| `intercept_enable` | `{patterns?}` | Start request interception with URL patterns (glob syntax) |
| `intercept_add_rule` | `{url_pattern, method?, status?, headers?, body?}` | Add mock response rule; returns rule_id |
| `intercept_remove_rule` | `{rule_id}` | Remove interception rule by ID |
| `intercept_list` | — | List active interception rules and status |
| `intercept_disable` | — | Stop interception, clear all rules |
| `block_requests` | `{patterns, resource_types?}` | Block URLs matching patterns |
| | | **Console, Dialog & Error** |
| `console_start` | — | Start capturing console.log/warn/error/info messages |
| `console_get` | `{level?, limit?}` | Get captured messages with optional filter |
| `console_clear` | — | Clear captured console messages and page errors |
| `dialog_handle` | `{accept, prompt_text?}` | Auto-handle JS alert/confirm/prompt dialogs |
| `dialog_get` | — | Get info about last captured dialog |
| `errors_get` | — | Get uncaught JS exceptions and page errors |
| | | **Device Emulation** |
| `emulate_device` | `{device?, width?, height?, user_agent?, device_scale_factor?, has_touch?, is_landscape?}` | Emulate device (presets: iphone-14, pixel-7, ipad-air, etc.) or custom |
| `emulate_geolocation` | `{latitude, longitude, accuracy?}` | Spoof GPS coordinates |
| `emulate_timezone` | `{timezone_id}` | Override timezone (e.g. 'America/New_York') |
| `emulate_media` | `{color_scheme?, reduced_motion?, forced_colors?}` | Override CSS media features |
| `emulate_network` | `{preset?, download_throughput?, upload_throughput?, latency?, offline?}` | Throttle network (presets: offline, 2g, 3g, 4g, wifi, etc.) or custom |
| | | **Interaction** |
| `drag` | `{source, target}` | Drag and drop between elements |
| `hover` | `{selector}` | Mouse hover on element |
| `keyboard` | `{keys, selector?}` | Send keyboard shortcuts/key combinations |
| `select` | `{selector, value?, text?, index?}` | Select dropdown option by value, text, or index |
| | | **File Operations** |
| `upload` | `{selector, file_path}` | Set file on a file input element |
| `download_wait` | `{timeout?, dir?}` | Wait for download to complete |
| `download_list` | — | List detected downloads |
| `download_set_dir` | `{path}` | Set download directory |
| | | **Shadow DOM** |
| `shadow_query` | `{host_selector, inner_selector}` | Query inside shadow DOM |
| `shadow_text` | `{host_selector, inner_selector}` | Get text content from shadow DOM element |
| `deep_query` | `{selector}` | Pierce multiple shadow DOM layers with `>>>` delimiter |
| | | **Session Context** |
| `context_set` | `{key, value}` | Store key/value in persistent page context |
| `context_get` | `{key}` | Retrieve value by key from page context |
| `context_list` | — | List all stored context entries |
| `context_clear` | — | Clear all page context |
| `context_transfer` | `{from_tab, to_tab, keys?}` | Transfer context between tabs |
| | | **Smart Forms** |
| `form_infer` | `{selector?}` | Analyze form fields and infer semantic purpose |
| `form_auto_fill` | `{data, selector?, confidence_threshold?}` | Auto-fill form by matching data keys to fields |
| `form_validate` | — | Check HTML5 form validation state |
| | | **Self-Healing Selectors** |
| `selector_heal` | `{selector, context?}` | Recover broken selector via multiple strategies |
| `selector_alternatives` | `{selector, max_alternatives?}` | Generate multiple selector strategies for an element |
| `selector_validate` | `{selector, expected_role?, expected_text?}` | Validate selector still matches expected element |
| | | **Event-Driven Reactions** |
| `event_subscribe` | `{event_type, filter?}` | Subscribe to page events |
| `event_unsubscribe` | `{event_type}` | Unsubscribe from events |
| `event_poll` | `{event_type?, limit?, clear?}` | Poll buffered events |
| `event_clear` | — | Clear event buffer |
| | | **Service Worker / PWA** |
| `sw_register` | `{script_url, scope?}` | Register service worker |
| `sw_unregister` | `{scope?}` | Unregister service worker |
| `sw_list` | — | List registered service workers |
| `sw_update` | `{scope?}` | Force-update service worker |
| `cache_list` | — | List Cache Storage entries |
| `cache_clear` | — | Clear all cache storage |
| `push_simulate` | `{title, body?, icon?, data?}` | Simulate push notification |
| `offline_mode` | `{enabled, bypass_for?}` | Enable/disable offline mode |

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

##### Multi-Tab

<details>
<summary><strong><code>new_tab</code></strong> — Open new tab</summary>

Open a new browser tab, optionally navigating to a URL. The new tab becomes the active page.

**Params:**

| Name | Type | Required | Description |
|------|------|:--------:|-------------|
| `url` | string | | URL to navigate to in the new tab. If omitted, opens `about:blank`. |

**Response:** `"opened new tab (index <i>) — <url>"`
</details>

<details>
<summary><strong><code>list_tabs</code></strong> — List all tabs</summary>

List all open browser tabs with their index, URL, title, and whether they are the currently active page.

**Params:** None

**Response:** JSON array of `{ "index", "url", "title", "active" }` objects.
</details>

<details>
<summary><strong><code>switch_tab</code></strong> — Switch active tab</summary>

Switch the active page to a different tab by its 0-based index.

**Params:**

| Name | Type | Required | Description |
|------|------|:--------:|-------------|
| `index` | number | ✅ | 0-based tab index (see `list_tabs`) |

**Response:** `"switched to tab <index> — <url>"`
</details>

<details>
<summary><strong><code>close_tab</code></strong> — Close tab</summary>

Close a browser tab by index. If no index is provided, closes the currently active tab. The active page switches to the nearest remaining tab.

**Params:**

| Name | Type | Required | Description |
|------|------|:--------:|-------------|
| `index` | number | | 0-based tab index. Defaults to the active tab. |

**Response:** `"closed tab <index>"`
</details>

##### DOM Events

<details>
<summary><strong><code>observe_mutations</code></strong> — Start MutationObserver</summary>

Start a `MutationObserver` on the current page. Recorded mutations can be retrieved with `get_mutations` and the observer disconnected with `stop_mutations`.

**Params:**

| Name | Type | Required | Description |
|------|------|:--------:|-------------|
| `selector` | string | | CSS selector for the target node (default `body`) |
| `child_list` | boolean | | Observe child additions/removals (default `true`) |
| `attributes` | boolean | | Observe attribute changes (default `false`) |
| `character_data` | boolean | | Observe text content changes (default `false`) |
| `subtree` | boolean | | Observe entire subtree (default `true`) |

**Response:** `"mutation observer started on <selector>"`
</details>

<details>
<summary><strong><code>get_mutations</code></strong> — Get recorded DOM mutations</summary>

Return all DOM mutations recorded since the last `get_mutations` call (or since `observe_mutations` was started). The buffer is cleared after reading.

**Params:** None

**Response:** JSON array of mutation records `{ "type", "target", "addedNodes", "removedNodes", "attributeName", "oldValue" }`.
</details>

<details>
<summary><strong><code>stop_mutations</code></strong> — Disconnect MutationObserver</summary>

Disconnect the active `MutationObserver`. Any unread mutations are discarded.

**Params:** None

**Response:** `"mutation observer stopped"`
</details>

<details>
<summary><strong><code>wait_for_event</code></strong> — Wait for DOM event</summary>

Wait for a specific DOM event on the page or a targeted element. Resolves when the event fires or the timeout is reached.

**Params:**

| Name | Type | Required | Description |
|------|------|:--------:|-------------|
| `event` | string | ✅ | DOM event name (e.g. `click`, `load`, `transitionend`) |
| `selector` | string | | CSS selector for the target element (default `document`) |
| `timeout` | number | | Timeout in milliseconds (default `30000`) |

**Response:** JSON object with event details `{ "type", "target", "timestamp" }`, or error on timeout.
</details>

##### Cookie & Storage

<details>
<summary><strong><code>cookies_get</code></strong> — Get cookies</summary>

Get cookies visible to the page. Non-HttpOnly cookies are read via `document.cookie`; HttpOnly cookies require CDP access.

**Params:**

| Name | Type | Required | Description |
|------|------|:--------:|-------------|
| `domain` | string | | Filter by domain |
| `name` | string | | Filter by cookie name |

**Response:** JSON array of `{ "name", "value", "domain", "path", "secure", "httpOnly", "sameSite", "expires" }` objects.
</details>

<details>
<summary><strong><code>cookies_set</code></strong> — Set cookie</summary>

Set a cookie on the current browser context.

**Params:**

| Name | Type | Required | Description |
|------|------|:--------:|-------------|
| `name` | string | ✅ | Cookie name |
| `value` | string | ✅ | Cookie value |
| `domain` | string | ✅ | Cookie domain |
| `path` | string | | Cookie path (default `/`) |
| `secure` | boolean | | Secure flag (default `false`) |
| `http_only` | boolean | | HttpOnly flag (default `false`) |
| `same_site` | string | | `Strict`, `Lax`, or `None` |
| `expires` | number | | Expiry as Unix timestamp (seconds) |

**Response:** `"cookie '<name>' set for <domain>"`
</details>

<details>
<summary><strong><code>cookies_clear</code></strong> — Clear cookies</summary>

Clear cookies from the browser context. Optionally filter by domain.

**Params:**

| Name | Type | Required | Description |
|------|------|:--------:|-------------|
| `domain` | string | | Clear only cookies for this domain. If omitted, clears all. |

**Response:** `"cleared <n> cookies"`
</details>

<details>
<summary><strong><code>storage_get</code></strong> — Get storage value</summary>

Get a value from `localStorage` or `sessionStorage`.

**Params:**

| Name | Type | Required | Description |
|------|------|:--------:|-------------|
| `key` | string | ✅ | Storage key |
| `storage_type` | string | | `local` or `session` (default `local`) |

**Response:** The stored value as a string, or `null` if the key does not exist.
</details>

<details>
<summary><strong><code>storage_set</code></strong> — Set storage value</summary>

Set a value in `localStorage` or `sessionStorage`.

**Params:**

| Name | Type | Required | Description |
|------|------|:--------:|-------------|
| `key` | string | ✅ | Storage key |
| `value` | string | ✅ | Value to store |
| `storage_type` | string | | `local` or `session` (default `local`) |

**Response:** `"stored '<key>' in <storage_type>Storage"`
</details>

<details>
<summary><strong><code>export_session</code></strong> — Export session state</summary>

Export the full browser session state (cookies, localStorage, sessionStorage) as a JSON blob that can be re-imported with `import_session`.

**Params:**

| Name | Type | Required | Description |
|------|------|:--------:|-------------|
| `cookies` | boolean | | Include cookies (default `true`) |
| `local_storage` | boolean | | Include localStorage (default `true`) |
| `session_storage` | boolean | | Include sessionStorage (default `true`) |

**Response:** JSON object `{ "cookies": [...], "local_storage": {...}, "session_storage": {...} }`.
</details>

<details>
<summary><strong><code>import_session</code></strong> — Import session state</summary>

Import a previously exported session state to restore cookies and storage values.

**Params:**

| Name | Type | Required | Description |
|------|------|:--------:|-------------|
| `state` | object | ✅ | Session state JSON (output of `export_session`) |

**Response:** `"session imported — <n> cookies, <n> localStorage keys, <n> sessionStorage keys"`
</details>

<details>
<summary><strong><code>intercept_enable</code></strong> — Start request interception</summary>

Start intercepting network requests. Optionally filter by URL patterns (glob syntax). Once enabled, matching requests can be mocked via `intercept_add_rule` or blocked via `block_requests`.

**Params:**

| Name | Type | Required | Description |
|------|------|:--------:|-------------|
| `patterns` | string[] | | URL patterns to intercept (glob syntax, e.g. `["**/api/**"]`). Omit to intercept all requests. |

**Response:** `"interception enabled"`
</details>

<details>
<summary><strong><code>intercept_add_rule</code></strong> — Add mock response rule</summary>

Add a rule that returns a mock response for requests matching the given URL pattern.

**Params:**

| Name | Type | Required | Description |
|------|------|:--------:|-------------|
| `url_pattern` | string | ✅ | URL pattern to match (glob syntax) |
| `method` | string | | HTTP method filter (e.g. `"GET"`, `"POST"`) |
| `status` | number | | HTTP status code for the mock response (default `200`) |
| `headers` | object | | Response headers as key-value pairs |
| `body` | string | | Response body string |

**Response:** `"rule added — id: <rule_id>"`
</details>

<details>
<summary><strong><code>intercept_remove_rule</code></strong> — Remove interception rule</summary>

Remove a previously added interception rule by its ID.

**Params:**

| Name | Type | Required | Description |
|------|------|:--------:|-------------|
| `rule_id` | string | ✅ | Rule ID returned by `intercept_add_rule` |

**Response:** `"rule <rule_id> removed"`
</details>

<details>
<summary><strong><code>intercept_list</code></strong> — List interception rules</summary>

List all active interception rules and current interception status.

**Params:** None.

**Response:** JSON object `{ "enabled": true, "rules": [{ "id": "...", "url_pattern": "...", ... }] }`.
</details>

<details>
<summary><strong><code>intercept_disable</code></strong> — Stop interception</summary>

Disable request interception and clear all active rules.

**Params:** None.

**Response:** `"interception disabled — <n> rules cleared"`
</details>

<details>
<summary><strong><code>block_requests</code></strong> — Block URLs matching patterns</summary>

Block network requests matching the given URL patterns. Optionally filter by resource type.

**Params:**

| Name | Type | Required | Description |
|------|------|:--------:|-------------|
| `patterns` | string[] | ✅ | URL patterns to block (glob syntax) |
| `resource_types` | string[] | | Filter by resource type (e.g. `["image", "font", "stylesheet"]`) |

**Response:** `"blocking <n> patterns"`
</details>

<details>
<summary><strong><code>console_start</code></strong> — Start capturing console messages</summary>

Begin capturing browser console output (`console.log`, `console.warn`, `console.error`, `console.info`). Messages are buffered until retrieved with `console_get`.

**Params:** None.

**Response:** `"console capture started"`
</details>

<details>
<summary><strong><code>console_get</code></strong> — Get captured console messages</summary>

Retrieve buffered console messages, optionally filtered by level.

**Params:**

| Name | Type | Required | Description |
|------|------|:--------:|-------------|
| `level` | string | | Filter by level: `"log"`, `"warn"`, `"error"`, `"info"` |
| `limit` | number | | Maximum number of messages to return |

**Response:** JSON array `[{ "level": "error", "text": "...", "timestamp": "..." }, ...]`.
</details>

<details>
<summary><strong><code>console_clear</code></strong> — Clear console messages</summary>

Clear all captured console messages and page errors from the buffer.

**Params:** None.

**Response:** `"console cleared — <n> messages removed"`
</details>

<details>
<summary><strong><code>dialog_handle</code></strong> — Handle JS dialogs</summary>

Configure automatic handling of JavaScript `alert()`, `confirm()`, and `prompt()` dialogs.

**Params:**

| Name | Type | Required | Description |
|------|------|:--------:|-------------|
| `accept` | boolean | ✅ | `true` to accept/OK, `false` to dismiss/cancel |
| `prompt_text` | string | | Text to enter for `prompt()` dialogs |

**Response:** `"dialog handler configured — accept: <accept>"`
</details>

<details>
<summary><strong><code>dialog_get</code></strong> — Get last dialog info</summary>

Get information about the most recently captured JavaScript dialog.

**Params:** None.

**Response:** JSON object `{ "type": "confirm", "message": "Are you sure?", "handled": true, "accepted": true }` or `"no dialog captured"`.
</details>

<details>
<summary><strong><code>errors_get</code></strong> — Get page errors</summary>

Get uncaught JavaScript exceptions and page errors captured during the session.

**Params:** None.

**Response:** JSON array `[{ "message": "...", "stack": "...", "timestamp": "..." }, ...]`.
</details>

<details>
<summary><strong><code>emulate_device</code></strong> — Emulate device</summary>

Emulate a mobile/tablet device using a preset or custom viewport configuration.

**Presets:** `iphone-14`, `iphone-14-pro`, `pixel-7`, `ipad-air`, `galaxy-s23`.

**Params:**

| Name | Type | Required | Description |
|------|------|:--------:|-------------|
| `device` | string | | Preset device name (e.g. `"iphone-14"`) |
| `width` | number | | Custom viewport width (px) |
| `height` | number | | Custom viewport height (px) |
| `user_agent` | string | | Custom User-Agent string |
| `device_scale_factor` | number | | Device pixel ratio (e.g. `2`, `3`) |
| `has_touch` | boolean | | Enable touch events |
| `is_landscape` | boolean | | Use landscape orientation |

**Response:** `"emulating <device> — <width>×<height> @<dpr>x"`
</details>

<details>
<summary><strong><code>emulate_geolocation</code></strong> — Spoof GPS coordinates</summary>

Override the browser's geolocation API to return the specified coordinates.

**Params:**

| Name | Type | Required | Description |
|------|------|:--------:|-------------|
| `latitude` | number | ✅ | Latitude in decimal degrees |
| `longitude` | number | ✅ | Longitude in decimal degrees |
| `accuracy` | number | | Accuracy in meters (default `1`) |

**Response:** `"geolocation set — <lat>, <lng>"`
</details>

<details>
<summary><strong><code>emulate_timezone</code></strong> — Override timezone</summary>

Override the browser's timezone for `Date` objects, `Intl`, and related APIs.

**Params:**

| Name | Type | Required | Description |
|------|------|:--------:|-------------|
| `timezone_id` | string | ✅ | IANA timezone identifier (e.g. `"America/New_York"`, `"Europe/Rome"`) |

**Response:** `"timezone set — <timezone_id>"`
</details>

<details>
<summary><strong><code>emulate_media</code></strong> — Override CSS media features</summary>

Override CSS media features for responsive design testing and accessibility checks.

**Params:**

| Name | Type | Required | Description |
|------|------|:--------:|-------------|
| `color_scheme` | string | | `"light"` or `"dark"` |
| `reduced_motion` | string | | `"reduce"` or `"no-preference"` |
| `forced_colors` | string | | `"active"` or `"none"` |

**Response:** `"media features set"`
</details>

<details>
<summary><strong><code>emulate_network</code></strong> — Throttle network</summary>

Simulate network conditions using a preset or custom throughput/latency values.

**Presets:** `offline`, `2g`, `slow-3g`, `3g`, `4g`, `wifi`.

**Params:**

| Name | Type | Required | Description |
|------|------|:--------:|-------------|
| `preset` | string | | Network preset name |
| `download_throughput` | number | | Download speed in bytes/sec |
| `upload_throughput` | number | | Upload speed in bytes/sec |
| `latency` | number | | Additional latency in ms |
| `offline` | boolean | | Simulate offline mode |

**Response:** `"network emulation — <preset or custom>"`
</details>

<details>
<summary><strong><code>drag</code></strong> — Drag and drop between elements</summary>

Perform a drag-and-drop operation from a source element to a target element. Dispatches mousedown, mousemove, and mouseup events.

**Params:**

| Name | Type | Required | Description |
|------|------|:--------:|-------------|
| `source` | string | ✅ | CSS selector of the element to drag |
| `target` | string | ✅ | CSS selector of the drop target element |

**Response:** `"dragged <source> → <target>"`

**Example:**

```json
{"action":"drag","params":{"source":"#item1","target":"#dropzone"}}
```
</details>

<details>
<summary><strong><code>hover</code></strong> — Mouse hover on element</summary>

Move the mouse over an element, dispatching `mouseenter`, `mouseover`, and `mousemove` events. Useful for triggering hover menus, tooltips, or CSS `:hover` states.

**Params:**

| Name | Type | Required | Description |
|------|------|:--------:|-------------|
| `selector` | string | ✅ | CSS selector of the element to hover |

**Response:** `"hovered <selector>"`

**Example:**

```json
{"action":"hover","params":{"selector":".menu-trigger"}}
```
</details>

<details>
<summary><strong><code>keyboard</code></strong> — Send keyboard shortcuts/key combinations</summary>

Send keyboard shortcuts or key combinations to the page or a specific element. Key names follow the Playwright key format (e.g. `Control+a`, `Enter`, `Shift+Tab`, `Meta+c`).

**Params:**

| Name | Type | Required | Description |
|------|------|:--------:|-------------|
| `keys` | string | ✅ | Key combination (e.g. `"Control+a"`, `"Enter"`, `"Shift+Tab"`) |
| `selector` | string | | CSS selector to focus before sending keys (omit for active element) |

**Response:** `"keyboard <keys> sent"`

**Example:**

```json
{"action":"keyboard","params":{"keys":"Control+a","selector":"#editor"}}
```
</details>

<details>
<summary><strong><code>select</code></strong> — Select dropdown option</summary>

Select an option in a `<select>` element by value, visible text, or numeric index.

**Params:**

| Name | Type | Required | Description |
|------|------|:--------:|-------------|
| `selector` | string | ✅ | CSS selector of the `<select>` element |
| `value` | string | | Option `value` attribute to select |
| `text` | string | | Visible text of the option to select |
| `index` | number | | Zero-based index of the option to select |

**Response:** `{"selected":"<value>","options_count":<n>}`

**Example:**

```json
{"action":"select","params":{"selector":"#country","value":"it"}}
```
</details>

<details>
<summary><strong><code>upload</code></strong> — Set file on input element</summary>

Set a file on a `<input type="file">` element. The file must be accessible on the local filesystem.

**Params:**

| Name | Type | Required | Description |
|------|------|:--------:|-------------|
| `selector` | string | ✅ | CSS selector of the file input element |
| `file_path` | string | ✅ | Absolute path to the file to upload |

**Response:** `"file set on <selector> — <filename>"`

**Example:**

```json
{"action":"upload","params":{"selector":"input[type=file]","file_path":"/tmp/doc.pdf"}}
```
</details>

<details>
<summary><strong><code>download_wait</code></strong> — Wait for download to complete</summary>

Wait for a browser-initiated download to finish. Returns download metadata once complete or times out.

**Params:**

| Name | Type | Required | Description |
|------|------|:--------:|-------------|
| `timeout` | number | | Timeout in milliseconds (default `30000`) |
| `dir` | string | | Directory to save the download |

**Response:** `{"status":"completed","url":"<url>","dir":"<path>"}`

**Example:**

```json
{"action":"download_wait","params":{"timeout":10000}}
```
</details>

<details>
<summary><strong><code>download_list</code></strong> — List detected downloads</summary>

List all downloads detected via the Performance API during the current session.

**Params:** None.

**Response:** `{"downloads":[{"url":"...","size_bytes":...,"duration_ms":...}],"count":<n>}`

**Example:**

```json
{"action":"download_list","params":{}}
```
</details>

<details>
<summary><strong><code>download_set_dir</code></strong> — Set download directory</summary>

Configure the directory where browser downloads are saved.

**Params:**

| Name | Type | Required | Description |
|------|------|:--------:|-------------|
| `path` | string | ✅ | Absolute path to the download directory |

**Response:** `"download dir set to <path>"`

**Example:**

```json
{"action":"download_set_dir","params":{"path":"/tmp/downloads"}}
```
</details>

<details>
<summary><strong><code>shadow_query</code></strong> — Query inside shadow DOM</summary>

Query elements inside a shadow DOM tree. First locates the shadow host, then queries within its shadow root.

**Params:**

| Name | Type | Required | Description |
|------|------|:--------:|-------------|
| `host_selector` | string | ✅ | CSS selector of the shadow host element |
| `inner_selector` | string | ✅ | CSS selector to query inside the shadow root |

**Response:** `{"elements":[{"index":0,"tag":"...","text":"...","id":"...","classes":[],"attributes":{}}],"count":<n>}`

**Example:**

```json
{"action":"shadow_query","params":{"host_selector":"my-element","inner_selector":".inner-btn"}}
```
</details>

<details>
<summary><strong><code>shadow_text</code></strong> — Get text from shadow DOM element</summary>

Extract text content and inner HTML from an element inside a shadow DOM tree.

**Params:**

| Name | Type | Required | Description |
|------|------|:--------:|-------------|
| `host_selector` | string | ✅ | CSS selector of the shadow host element |
| `inner_selector` | string | ✅ | CSS selector of the target element inside the shadow root |

**Response:** `{"text":"...","html":"..."}`

**Example:**

```json
{"action":"shadow_text","params":{"host_selector":"my-element","inner_selector":".title"}}
```
</details>

<details>
<summary><strong><code>deep_query</code></strong> — Pierce multiple shadow DOM layers</summary>

Query elements across multiple shadow DOM boundaries using the `>>>` delimiter. Each segment pierces one shadow root, enabling deep traversal of nested web components.

**Params:**

| Name | Type | Required | Description |
|------|------|:--------:|-------------|
| `selector` | string | ✅ | Piercing selector using `>>>` to cross shadow boundaries (e.g. `"my-app >>> .inner >>> button"`) |

**Response:** `{"elements":[{"index":0,"tag":"...","text":"...","id":"...","classes":[],"attributes":{},"depth":<n>}],"count":<n>}`

**Example:**

```json
{"action":"deep_query","params":{"selector":"my-app >>> .content >>> button"}}
```
</details>

##### Session Context

<details>
<summary><strong><code>context_set</code></strong> — Store key/value in persistent page context</summary>

Store a key/value pair in the persistent page context. Values survive navigation within the same tab and can be transferred between tabs.

**Params:**

| Name | Type | Required | Description |
|------|------|:--------:|-------------|
| `key` | string | ✅ | Context key |
| `value` | any | ✅ | Value to store (string, number, boolean, object, or array) |

**Response:**
```json
{ "stored": true, "key": "user_id", "entries_count": 3 }
```
</details>

<details>
<summary><strong><code>context_get</code></strong> — Retrieve value by key from page context</summary>

Retrieve a previously stored value from the page context by its key.

**Params:**

| Name | Type | Required | Description |
|------|------|:--------:|-------------|
| `key` | string | ✅ | Context key to retrieve |

**Response:**
```json
{ "key": "user_id", "value": "abc-123" }
```
</details>

<details>
<summary><strong><code>context_list</code></strong> — List all stored context entries</summary>

List all key/value pairs currently stored in the page context.

**Params:** None

**Response:**
```json
{ "entries": { "user_id": "abc-123", "token": "xyz" }, "count": 2 }
```
</details>

<details>
<summary><strong><code>context_clear</code></strong> — Clear all page context</summary>

Remove all entries from the persistent page context.

**Params:** None

**Response:**
```json
{ "cleared": true, "entries_removed": 3 }
```
</details>

<details>
<summary><strong><code>context_transfer</code></strong> — Transfer context between tabs</summary>

Copy context entries from one tab to another. Optionally specify which keys to transfer; defaults to all.

**Params:**

| Name | Type | Required | Description |
|------|------|:--------:|-------------|
| `from_tab` | integer | ✅ | Source tab index (0-based) |
| `to_tab` | integer | ✅ | Destination tab index (0-based) |
| `keys` | string[] | | Specific keys to transfer (omit for all) |

**Response:**
```json
{ "transferred": true, "keys": ["user_id", "token"], "from_tab": 0, "to_tab": 1 }
```
</details>

##### Smart Forms

<details>
<summary><strong><code>form_infer</code></strong> — Analyze form fields and infer semantic purpose</summary>

Analyze form fields on the page and infer their semantic purpose (e.g. email, password, phone, address) using field names, labels, placeholders, and input types.

**Params:**

| Name | Type | Required | Description |
|------|------|:--------:|-------------|
| `selector` | string | | CSS selector to target a specific form (defaults to first form on page) |

**Response:**
```json
{
  "fields": [
    { "name": "email", "type": "email", "label": "Email Address", "placeholder": "you@example.com", "required": true, "inferred_purpose": "email" },
    { "name": "pwd", "type": "password", "label": "Password", "placeholder": "", "required": true, "inferred_purpose": "password" }
  ],
  "count": 2
}
```
</details>

<details>
<summary><strong><code>form_auto_fill</code></strong> — Auto-fill form by matching data keys to fields</summary>

Automatically fill form fields by matching data object keys to inferred field purposes. Uses fuzzy matching with a configurable confidence threshold.

**Params:**

| Name | Type | Required | Description |
|------|------|:--------:|-------------|
| `data` | object | ✅ | Key-value pairs to fill (e.g. `{"email": "a@b.com", "password": "secret"}`) |
| `selector` | string | | CSS selector to target a specific form |
| `confidence_threshold` | number | | Minimum match confidence 0–1 (default `0.7`) |

**Response:**
```json
{
  "filled": [
    { "field": "email", "matched_key": "email", "confidence": 0.95 },
    { "field": "pwd", "matched_key": "password", "confidence": 0.85 }
  ],
  "skipped": [],
  "count": 2
}
```
</details>

<details>
<summary><strong><code>form_validate</code></strong> — Check HTML5 form validation state</summary>

Check the HTML5 constraint validation state of all forms on the page. Reports validity per field and overall form validity.

**Params:** None

**Response:**
```json
{
  "valid": false,
  "fields": [
    { "name": "email", "valid": true, "message": "" },
    { "name": "age", "valid": false, "message": "Value must be greater than 0" }
  ],
  "invalid_count": 1
}
```
</details>

<details>
<summary><strong><code>selector_heal</code></strong> — Recover broken selector via multiple strategies</summary>

Attempt to recover a broken CSS selector by trying multiple healing strategies (ID, class, text, aria, structural). Returns the best match and alternatives.

**Params:**

| Name | Type | Required | Description |
|------|------|:--------:|-------------|
| `selector` | string | ✅ | The broken CSS selector to heal |
| `context` | string | | Additional context about the expected element |

**Response:**
```json
{
  "healed": true,
  "original": "#old-button",
  "alternatives": [
    { "selector": "[data-testid='submit']", "strategy": "test-id", "confidence": 0.95 },
    { "selector": "button:has-text('Submit')", "strategy": "text", "confidence": 0.87 }
  ],
  "recommended": "[data-testid='submit']"
}
```
</details>

<details>
<summary><strong><code>selector_alternatives</code></strong> — Generate multiple selector strategies for an element</summary>

Generate multiple CSS selector strategies for a given element. Useful for creating resilient selectors with fallbacks.

**Params:**

| Name | Type | Required | Description |
|------|------|:--------:|-------------|
| `selector` | string | ✅ | CSS selector of the target element |
| `max_alternatives` | number | | Maximum number of alternatives to generate |

**Response:**
```json
{
  "element": { "tag": "button", "id": "submit-btn", "class": "btn primary", "text": "Submit" },
  "strategies": [
    { "type": "id", "selector": "#submit-btn", "specificity": "high", "fragility_score": 0.1 },
    { "type": "text", "selector": "button:has-text('Submit')", "specificity": "medium", "fragility_score": 0.3 }
  ]
}
```
</details>

<details>
<summary><strong><code>selector_validate</code></strong> — Validate selector still matches expected element</summary>

Check whether a CSS selector still matches the expected element on the page. Validates role, text content, and match count.

**Params:**

| Name | Type | Required | Description |
|------|------|:--------:|-------------|
| `selector` | string | ✅ | CSS selector to validate |
| `expected_role` | string | | Expected ARIA role of the element |
| `expected_text` | string | | Expected text content of the element |

**Response:**
```json
{
  "valid": true,
  "matches_count": 1,
  "expected_role_match": true,
  "expected_text_match": true,
  "element_info": { "tag": "button", "role": "button", "text": "Submit" }
}
```
</details>

<details>
<summary><strong><code>event_subscribe</code></strong> — Subscribe to page events</summary>

Subscribe to specific page events (e.g., `click`, `navigation`, `network`, `console`). Events are buffered and can be retrieved via `event_poll`.

**Params:**

| Name | Type | Required | Description |
|------|------|:--------:|-------------|
| `event_type` | string | ✅ | Event type to subscribe to |
| `filter` | string | | Optional filter pattern for events |

**Response:**
```json
{ "event_type": "network", "subscribed": true, "active_subscriptions": ["network", "console"] }
```
</details>

<details>
<summary><strong><code>event_unsubscribe</code></strong> — Unsubscribe from events</summary>

Remove a subscription for a specific event type.

**Params:**

| Name | Type | Required | Description |
|------|------|:--------:|-------------|
| `event_type` | string | ✅ | Event type to unsubscribe from |

**Response:**
```json
{ "event_type": "network", "unsubscribed": true, "remaining_subscriptions": ["console"] }
```
</details>

<details>
<summary><strong><code>event_poll</code></strong> — Poll buffered events</summary>

Retrieve buffered events, optionally filtered by type. Can clear the buffer after reading.

**Params:**

| Name | Type | Required | Description |
|------|------|:--------:|-------------|
| `event_type` | string | | Filter by event type (omit for all) |
| `limit` | number | | Max events to return |
| `clear` | boolean | | Clear buffer after polling (default `false`) |

**Response:**
```json
{
  "events": [
    { "type": "network", "timestamp": "2025-01-15T10:30:00Z", "data": { "url": "https://api.example.com/data", "status": 200 } }
  ],
  "count": 1,
  "has_more": false
}
```
</details>

<details>
<summary><strong><code>event_clear</code></strong> — Clear event buffer</summary>

Clear all buffered events across all subscriptions.

**Params:** None

**Response:**
```json
{ "cleared_count": 42 }
```
</details>

<details>
<summary><strong><code>sw_register</code></strong> — Register service worker</summary>

Register a service worker script at the given URL with an optional scope.

**Params:**

| Name | Type | Required | Description |
|------|------|:--------:|-------------|
| `script_url` | string | ✅ | URL of the service worker script |
| `scope` | string | | Scope URL (defaults to script directory) |

**Response:**
```json
{ "registered": true, "scope": "https://example.com/", "state": "activated" }
```
</details>

<details>
<summary><strong><code>sw_unregister</code></strong> — Unregister service worker</summary>

Unregister the service worker for the given scope.

**Params:**

| Name | Type | Required | Description |
|------|------|:--------:|-------------|
| `scope` | string | | Scope URL (defaults to current page origin) |

**Response:** `"service worker unregistered for scope <scope>"`
</details>

<details>
<summary><strong><code>sw_list</code></strong> — List registered service workers</summary>

List all registered service workers and their status.

**Params:** None

**Response:**
```json
{
  "workers": [
    { "scope": "https://example.com/", "script_url": "/sw.js", "state": "activated", "version_id": "1" }
  ],
  "count": 1
}
```
</details>

<details>
<summary><strong><code>sw_update</code></strong> — Force-update service worker</summary>

Force-update the service worker registration for the given scope, bypassing the browser's 24-hour update check.

**Params:**

| Name | Type | Required | Description |
|------|------|:--------:|-------------|
| `scope` | string | | Scope URL (defaults to current page origin) |

**Response:** `"service worker updated for scope <scope>"`
</details>

<details>
<summary><strong><code>cache_list</code></strong> — List Cache Storage entries</summary>

List all named caches and their entry counts in the Cache Storage API.

**Params:** None

**Response:**
```json
{
  "caches": [
    { "name": "v1-assets", "entry_count": 42 },
    { "name": "v1-api", "entry_count": 15 }
  ],
  "total_caches": 2
}
```
</details>

<details>
<summary><strong><code>cache_clear</code></strong> — Clear all cache storage</summary>

Delete all named caches from Cache Storage.

**Params:** None

**Response:** `"cleared <N> caches"`
</details>

<details>
<summary><strong><code>push_simulate</code></strong> — Simulate push notification</summary>

Simulate a push notification event dispatched to the active service worker.

**Params:**

| Name | Type | Required | Description |
|------|------|:--------:|-------------|
| `title` | string | ✅ | Notification title |
| `body` | string | | Notification body text |
| `icon` | string | | URL of the notification icon |
| `data` | string | | Arbitrary JSON data payload |

**Response:**
```json
{ "dispatched": true, "title": "New message", "sw_handled": true }
```
</details>

<details>
<summary><strong><code>offline_mode</code></strong> — Enable/disable offline mode</summary>

Enable or disable offline mode, optionally allowing specific URL patterns to bypass.

**Params:**

| Name | Type | Required | Description |
|------|------|:--------:|-------------|
| `enabled` | boolean | ✅ | `true` to go offline, `false` to restore connectivity |
| `bypass_for` | string[] | | URL patterns that bypass offline mode (glob syntax) |

**Response:** `"offline mode enabled"` or `"offline mode disabled"`
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

AI agent orchestration — command chains, element screenshots, API capture, iframes, remote CDP, safety policies, skills, screencast, recording, iOS automation, task decomposition, vision observation, and WCAG/accessibility auditing.

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
| | | **Task Decomposition** |
| `task_decompose` | `{goal, context?, max_depth?}` | Break goal into subtasks |
| `task_plan` | `{tasks, strategy?}` | Generate execution plan |
| `task_status` | — | Get task plan status |
| | | **Vision Observation** |
| `vision_describe` | `{selector?, format?}` | Describe page state structurally |
| `vision_locate` | `{description, strategy?}` | Find element by natural language |
| `vision_compare` | `{baseline, current?, threshold?}` | Compare page state against baseline |
| | | **WCAG / Accessibility Auditing** |
| `wcag_audit` | `{level?, selector?}` | Full WCAG compliance audit |
| `aria_tree` | — | Build ARIA accessibility tree |
| `contrast_check` | `{selector?, threshold?}` | Color contrast ratio validation |
| `landmark_nav` | — | List ARIA landmark regions |
| `focus_order` | — | Map tab/focus order |
| `alt_text_audit` | `{selector?, include_decorative?}` | Audit image alt text |
| `heading_structure` | — | Validate heading hierarchy |
| `role_validate` | `{selector?, roles?}` | Validate ARIA roles/properties |
| `keyboard_trap_detect` | — | Detect keyboard focus traps |
| `screen_reader_sim` | `{selector?, max_elements?}` | Simulate screen reader output |

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

<details>
<summary><strong><code>task_decompose</code></strong> — Break goal into subtasks</summary>

Decompose a high-level goal into actionable subtasks with dependency information and complexity estimates.

**Params:**

| Name | Type | Required | Description |
|------|------|:--------:|-------------|
| `goal` | string | ✅ | The high-level goal to decompose |
| `context` | string | | Additional context about the current state |
| `max_depth` | number | | Maximum decomposition depth (default `2`) |

**Response:**
```json
{
  "goal": "Login and scrape dashboard data",
  "subtasks": [
    { "id": "t1", "description": "Navigate to login page", "type": "navigation", "complexity": "low", "dependencies": [] },
    { "id": "t2", "description": "Fill login form", "type": "interaction", "complexity": "medium", "dependencies": ["t1"] },
    { "id": "t3", "description": "Extract dashboard metrics", "type": "extraction", "complexity": "medium", "dependencies": ["t2"] }
  ],
  "count": 3
}
```
</details>

<details>
<summary><strong><code>task_plan</code></strong> — Generate execution plan</summary>

Generate an ordered execution plan from a list of tasks, resolving dependencies and identifying parallelizable steps.

**Params:**

| Name | Type | Required | Description |
|------|------|:--------:|-------------|
| `tasks` | array | ✅ | List of task descriptions or IDs |
| `strategy` | string | | Planning strategy: `sequential`, `parallel`, `adaptive` (default `adaptive`) |

**Response:**
```json
{
  "plan_id": "plan_abc123",
  "strategy": "adaptive",
  "steps": [
    { "order": 1, "task": "Navigate to login page", "dependencies": [], "parallel_safe": true },
    { "order": 2, "task": "Fill login form", "dependencies": ["step_1"], "parallel_safe": false }
  ],
  "total_steps": 2
}
```
</details>

<details>
<summary><strong><code>task_status</code></strong> — Get task plan status</summary>

Retrieve the status of all active task plans including progress and completion state.

**Params:** None

**Response:**
```json
{
  "plans": [
    { "plan_id": "plan_abc123", "strategy": "adaptive", "total_steps": 5, "completed": 3, "status": "running" }
  ],
  "total_plans": 1
}
```
</details>

<details>
<summary><strong><code>vision_describe</code></strong> — Describe page state structurally</summary>

Produce a structural description of the current page state, including visible elements, layout summary, and interactive element count.

**Params:**

| Name | Type | Required | Description |
|------|------|:--------:|-------------|
| `selector` | string | | Scope description to a specific element |
| `format` | string | | Output format: `summary`, `detailed`, `json` (default `summary`) |

**Response:**
```json
{
  "page_title": "Dashboard",
  "url": "https://example.com/dashboard",
  "visible_elements": 47,
  "layout_summary": "Header with nav, main content with 3-column grid, sidebar with filters",
  "interactive_count": 12
}
```
</details>

<details>
<summary><strong><code>vision_locate</code></strong> — Find element by natural language</summary>

Locate a page element using a natural language description. Returns matching elements with confidence scores.

**Params:**

| Name | Type | Required | Description |
|------|------|:--------:|-------------|
| `description` | string | ✅ | Natural language description of the element (e.g. "the blue submit button") |
| `strategy` | string | | Search strategy: `accessibility`, `visual`, `hybrid` (default `hybrid`) |

**Response:**
```json
{
  "found": true,
  "matches": [
    { "selector": "button.btn-primary", "role": "button", "name": "Submit", "confidence": 0.92 },
    { "selector": "#form-submit", "role": "button", "name": "Submit Form", "confidence": 0.78 }
  ]
}
```
</details>

<details>
<summary><strong><code>vision_compare</code></strong> — Compare page state against baseline</summary>

Compare the current page state against a baseline snapshot. Detects visual and structural changes.

**Params:**

| Name | Type | Required | Description |
|------|------|:--------:|-------------|
| `baseline` | string | ✅ | Baseline snapshot identifier or data |
| `current` | string | | Current snapshot (omit to capture live page) |
| `threshold` | number | | Similarity threshold 0–1 (default `0.95`) |

**Response:**
```json
{
  "visual_similarity": 0.87,
  "structural_changes": [
    { "type": "added", "element": "div.notification-banner" },
    { "type": "modified", "element": "span.user-count", "detail": "text changed" }
  ],
  "summary": "2 structural changes detected, visual similarity below threshold"
}
```
</details>

<details>
<summary><strong><code>wcag_audit</code></strong> — Full WCAG compliance audit</summary>

Run a full WCAG compliance audit on the current page or a specific subtree. Reports violations, warnings, and passes grouped by WCAG success criterion.

**Params:**

| Name | Type | Required | Description |
|------|------|:--------:|-------------|
| `level` | string | | WCAG conformance level: `A`, `AA`, `AAA` (default `AA`) |
| `selector` | string | | CSS selector to scope the audit (default: entire page) |

**Response:**
```json
{
  "level": "AA",
  "violations": [
    { "id": "color-contrast", "impact": "serious", "nodes": 3, "description": "Elements must have sufficient color contrast" }
  ],
  "passes": 42,
  "warnings": 2,
  "violations_count": 1
}
```
</details>

<details>
<summary><strong><code>aria_tree</code></strong> — Build ARIA accessibility tree</summary>

Build the full ARIA accessibility tree of the current page, including roles, names, states, and properties.

**Params:** None

**Response:**
```json
{
  "tree": {
    "role": "document",
    "name": "Example Page",
    "children": [
      { "role": "banner", "children": [{ "role": "heading", "name": "Welcome", "level": 1 }] },
      { "role": "main", "children": [] },
      { "role": "contentinfo", "children": [] }
    ]
  },
  "node_count": 87
}
```
</details>

<details>
<summary><strong><code>contrast_check</code></strong> — Color contrast ratio validation</summary>

Check color contrast ratios of text elements against WCAG thresholds.

**Params:**

| Name | Type | Required | Description |
|------|------|:--------:|-------------|
| `selector` | string | | CSS selector to scope check (default: all text elements) |
| `threshold` | number | | Minimum contrast ratio (default `4.5` for AA normal text) |

**Response:**
```json
{
  "total_checked": 24,
  "passing": 21,
  "failing": [
    { "selector": "p.light-gray", "foreground": "#999", "background": "#fff", "ratio": 2.85, "required": 4.5 }
  ]
}
```
</details>

<details>
<summary><strong><code>landmark_nav</code></strong> — List ARIA landmark regions</summary>

List all ARIA landmark regions on the page (banner, navigation, main, complementary, contentinfo, etc.).

**Params:** None

**Response:**
```json
{
  "landmarks": [
    { "role": "banner", "selector": "header", "label": null },
    { "role": "navigation", "selector": "nav.main-nav", "label": "Main" },
    { "role": "main", "selector": "main", "label": null }
  ],
  "count": 3
}
```
</details>

<details>
<summary><strong><code>focus_order</code></strong> — Map tab/focus order</summary>

Map the sequential tab/focus order of all focusable elements on the page.

**Params:** None

**Response:**
```json
{
  "focus_order": [
    { "index": 1, "selector": "a.skip-link", "role": "link", "name": "Skip to content", "tab_index": 0 },
    { "index": 2, "selector": "input#search", "role": "textbox", "name": "Search", "tab_index": 0 }
  ],
  "total_focusable": 18
}
```
</details>

<details>
<summary><strong><code>alt_text_audit</code></strong> — Audit image alt text</summary>

Audit all images on the page for alt text presence, quality, and decorative marking.

**Params:**

| Name | Type | Required | Description |
|------|------|:--------:|-------------|
| `selector` | string | | CSS selector to scope audit (default: all `img` elements) |
| `include_decorative` | boolean | | Include images marked as decorative (default `false`) |

**Response:**
```json
{
  "total_images": 12,
  "with_alt": 10,
  "missing_alt": 1,
  "empty_alt_decorative": 1,
  "issues": [
    { "selector": "img.hero", "src": "/hero.jpg", "issue": "missing alt attribute" }
  ]
}
```
</details>

<details>
<summary><strong><code>heading_structure</code></strong> — Validate heading hierarchy</summary>

Validate the heading hierarchy (h1–h6) for proper nesting and structure.

**Params:** None

**Response:**
```json
{
  "headings": [
    { "level": 1, "text": "Welcome", "selector": "h1" },
    { "level": 2, "text": "Features", "selector": "section h2" },
    { "level": 3, "text": "Speed", "selector": "section h3" }
  ],
  "valid": true,
  "issues": []
}
```
</details>

<details>
<summary><strong><code>role_validate</code></strong> — Validate ARIA roles/properties</summary>

Validate that ARIA roles, states, and properties are used correctly on the page.

**Params:**

| Name | Type | Required | Description |
|------|------|:--------:|-------------|
| `selector` | string | | CSS selector to scope validation (default: all elements with ARIA attributes) |
| `roles` | string[] | | Filter to specific roles (e.g. `["button", "dialog"]`) |

**Response:**
```json
{
  "total_checked": 15,
  "valid": 13,
  "issues": [
    { "selector": "div.modal", "role": "dialog", "issue": "missing required aria-label or aria-labelledby" }
  ]
}
```
</details>

<details>
<summary><strong><code>keyboard_trap_detect</code></strong> — Detect keyboard focus traps</summary>

Detect elements that trap keyboard focus, preventing users from navigating away with Tab or Escape.

**Params:** None

**Response:**
```json
{
  "traps_detected": 1,
  "traps": [
    { "selector": "div.modal-overlay", "reason": "focus cycles within element, no escape handler" }
  ]
}
```
</details>

<details>
<summary><strong><code>screen_reader_sim</code></strong> — Simulate screen reader output</summary>

Simulate screen reader output for the page or a specific subtree, producing the linearized reading order with roles and announcements.

**Params:**

| Name | Type | Required | Description |
|------|------|:--------:|-------------|
| `selector` | string | | CSS selector to scope simulation (default: entire page) |
| `max_elements` | number | | Maximum elements to include (default `100`) |

**Response:**
```json
{
  "output": [
    { "role": "banner", "announcement": "banner landmark" },
    { "role": "heading", "level": 1, "announcement": "Welcome, heading level 1" },
    { "role": "link", "announcement": "Get Started, link" }
  ],
  "element_count": 45
}
```
</details>

---

### 4. `stealth`

Anti-detection and bot evasion — stealth patches, fingerprinting, domain blocking, CAPTCHA detection, human behavior simulation.

#### Actions

| Action | Params | Description |
|--------|--------|-------------|
| `inject` | — | Inject stealth patches into page |
| `test` | — | Test if current page detects bot |
| `fingerprint` | `{user_agent?}` | Generate and apply browser fingerprint |
| `block_domains` | `{domains?, category?}` | Block tracking/ad domains |
| `detect_captcha` | — | Detect CAPTCHAs on page |
| | | **Human Behavior Simulation** |
| `human_delay` | `{min_ms?, max_ms?, pattern?}` | Random human-like delay |
| `human_mouse` | `{target, speed?, curve?}` | Bézier curve mouse movement |
| `human_type` | `{selector, text, speed?, mistakes?}` | Natural typing with typos |
| `human_scroll` | `{direction?, amount?, speed?}` | Human-like scrolling |
| `human_profile` | `{profile?}` | Set behavior profile |
| `stealth_max` | `{features?}` | Enable maximum stealth |
| `stealth_score` | — | Score current page stealth level |

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

<details>
<summary><strong><code>human_delay</code></strong> — Random human-like delay</summary>

Pause execution for a random duration sampled from a human-like distribution.

**Params:**

| Name | Type | Required | Description |
|------|------|:--------:|-------------|
| `min_ms` | number | | Minimum delay in milliseconds (default `200`) |
| `max_ms` | number | | Maximum delay in milliseconds (default `2000`) |
| `pattern` | string | | Distribution pattern: `uniform`, `gaussian`, `reading` (default `gaussian`) |

**Response:** `"delayed <N>ms (pattern: gaussian)"`
</details>

<details>
<summary><strong><code>human_mouse</code></strong> — Bézier curve mouse movement</summary>

Move the mouse to a target element using a realistic Bézier curve path with variable speed.

**Params:**

| Name | Type | Required | Description |
|------|------|:--------:|-------------|
| `target` | string | ✅ | CSS selector or `@ref` of the target element |
| `speed` | number | | Movement speed multiplier (default `1.0`, lower = slower) |
| `curve` | string | | Curve type: `bezier`, `arc`, `linear` (default `bezier`) |

**Response:** `"moved mouse to <target> via bezier curve (<N> points, <M>ms)"`
</details>

<details>
<summary><strong><code>human_type</code></strong> — Natural typing with typos</summary>

Type text into an element with realistic keystroke timing and occasional typos that are corrected.

**Params:**

| Name | Type | Required | Description |
|------|------|:--------:|-------------|
| `selector` | string | ✅ | CSS selector or `@ref` of the input element |
| `text` | string | ✅ | Text to type |
| `speed` | number | | Typing speed in WPM (default `65`) |
| `mistakes` | boolean | | Simulate occasional typos and corrections (default `true`) |

**Response:** `"typed <N> chars into <selector> (human mode, <M>ms, <K> corrections)"`
</details>

<details>
<summary><strong><code>human_scroll</code></strong> — Human-like scrolling</summary>

Scroll the page with variable speed and momentum, mimicking natural scrolling behavior.

**Params:**

| Name | Type | Required | Description |
|------|------|:--------:|-------------|
| `direction` | string | | Scroll direction: `down`, `up`, `left`, `right` (default `down`) |
| `amount` | number | | Approximate scroll distance in pixels (default `500`) |
| `speed` | string | | Speed preset: `slow`, `normal`, `fast` (default `normal`) |

**Response:** `"scrolled <direction> ~<N>px (human mode, <M>ms)"`
</details>

<details>
<summary><strong><code>human_profile</code></strong> — Set behavior profile</summary>

Set a human behavior profile that adjusts timing, accuracy, and patterns for all subsequent human simulation actions.

**Params:**

| Name | Type | Required | Description |
|------|------|:--------:|-------------|
| `profile` | string | | Profile name: `casual`, `fast`, `careful` (default `casual`) |

**Response:**
```json
{
  "profile": "casual",
  "settings": {
    "typing_wpm": 55,
    "mouse_speed": 0.8,
    "mistake_rate": 0.05,
    "scroll_speed": "normal",
    "delay_range_ms": [300, 2500]
  }
}
```
</details>

<details>
<summary><strong><code>stealth_max</code></strong> — Enable maximum stealth</summary>

Enable all available stealth patches and human behavior simulation features for maximum anti-detection.

**Params:**

| Name | Type | Required | Description |
|------|------|:--------:|-------------|
| `features` | string[] | | Specific features to enable (default: all). Options: `patches`, `fingerprint`, `human_mouse`, `human_type`, `human_scroll`, `human_delay` |

**Response:**
```json
{
  "enabled_features": ["patches", "fingerprint", "human_mouse", "human_type", "human_scroll", "human_delay"],
  "patches_applied": 12,
  "profile": "casual"
}
```
</details>

<details>
<summary><strong><code>stealth_score</code></strong> — Score current page stealth level</summary>

Evaluate how well the current page session evades bot detection, returning a score and breakdown.

**Params:** None

**Response:**
```json
{
  "score": 92,
  "max_score": 100,
  "breakdown": {
    "navigator": { "score": 10, "max": 10 },
    "webgl": { "score": 10, "max": 10 },
    "webrtc": { "score": 8, "max": 10 },
    "timing": { "score": 9, "max": 10 },
    "behavior": { "score": 7, "max": 10 }
  },
  "recommendations": ["Enable human_scroll for more natural behavior"]
}
```
</details>

---

### 5. `data`

Data processing, HTTP requests, link analysis, network intelligence, WebSocket, SSE, and GraphQL subscriptions.

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
| `extract_schema` | `{schema_type?}` | Extract JSON-LD, OpenGraph, Twitter Card, microdata |
| `extract_tables` | `{selector?, format?, headers?}` | Extract HTML tables to JSON/CSV |
| `extract_entities` | `{types?, selector?}` | Extract emails, phones, URLs, dates, prices |
| `classify_content` | `{strategy?, selector?}` | Classify page content type and structure |
| `transform_json` | `{data, transform, output_format?}` | Transform JSON (flatten, keys, values, unique) |
| `export_csv` | `{data, columns?, delimiter?}` | Export JSON array to CSV |
| `extract_metadata` | `{include_og?, include_twitter?, include_all?}` | Extract page metadata |
| `extract_feeds` | `{feed_type?}` | Discover RSS, Atom, JSON feeds |
| | | **WebSocket / SSE / GraphQL** |
| `ws_connect` | `{url, protocols?}` | Connect to WebSocket server |
| `ws_intercept` | `{url_pattern?, capture_only?}` | Intercept WebSocket traffic |
| `ws_send` | `{target, message}` | Send WebSocket message |
| `ws_messages` | `{url_filter?, limit?}` | Get captured WebSocket messages |
| `ws_close` | `{target?}` | Close WebSocket connections |
| `sse_listen` | `{url, duration_ms?}` | Listen to Server-Sent Events |
| `sse_messages` | `{url_filter?, limit?}` | Get captured SSE messages |
| `graphql_subscribe` | `{url, query, variables?, duration_ms?}` | GraphQL subscription via WebSocket |

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

<details>
<summary><strong><code>extract_schema</code></strong> — Extract structured data schemas</summary>

Extract JSON-LD, OpenGraph, Twitter Card, and microdata from the current page.

| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| `schema_type` | string | | Type to extract: `all`, `json_ld`, `open_graph`, `twitter_card`, `microdata` (default `all`) |

**Response:** JSON with extracted schema data by type.
</details>

<details>
<summary><strong><code>extract_tables</code></strong> — Extract HTML tables</summary>

Extract HTML tables from the current page and convert to structured data.

| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| `selector` | string | | CSS selector for tables (default `table`) |
| `format` | string | | Output format: `json` or `csv` (default `json`) |
| `headers` | boolean | | Use first row as headers (default `true`) |

**Response:** Array of extracted tables with rows and optional headers.
</details>

<details>
<summary><strong><code>extract_entities</code></strong> — Extract named entities</summary>

Extract emails, phone numbers, URLs, dates, and prices from page content.

| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| `types` | string[] | | Entity types to extract (default all) |
| `selector` | string | | CSS selector to scope extraction |

**Response:** JSON with categorized extracted entities.
</details>

<details>
<summary><strong><code>classify_content</code></strong> — Classify page content</summary>

Analyze and classify the content type and structure of the current page.

| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| `strategy` | string | | Classification strategy: `auto`, `heading`, `semantic` (default `auto`) |
| `selector` | string | | CSS selector to scope classification |

**Response:** JSON with content classification and structural analysis.
</details>

<details>
<summary><strong><code>transform_json</code></strong> — Transform JSON data</summary>

Apply transformations to JSON data (flatten, extract keys/values, unique, field access).

| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| `data` | any | ✅ | JSON data to transform |
| `transform` | string | ✅ | Transform operation: `flatten`, `keys`, `values`, `unique`, `field:<path>` |
| `output_format` | string | | Output format (default `json`) |

**Response:** Transformed JSON data.
</details>

<details>
<summary><strong><code>export_csv</code></strong> — Export JSON to CSV</summary>

Convert a JSON array of objects to CSV format.

| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| `data` | array | ✅ | JSON array to export |
| `columns` | string[] | | Column names to include (default: all keys) |
| `delimiter` | string | | CSV delimiter (default `,`) |

**Response:** CSV string output.
</details>

<details>
<summary><strong><code>extract_metadata</code></strong> — Extract page metadata</summary>

Extract comprehensive metadata from the current page (title, description, canonical, OG, Twitter).

| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| `include_og` | boolean | | Include OpenGraph metadata (default `true`) |
| `include_twitter` | boolean | | Include Twitter Card metadata (default `true`) |
| `include_all` | boolean | | Include all meta tags (default `false`) |

**Response:** JSON with extracted page metadata.
</details>

<details>
<summary><strong><code>extract_feeds</code></strong> — Discover RSS/Atom/JSON feeds</summary>

Discover and extract feed links from the current page.

| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| `feed_type` | string | | Feed type filter: `all`, `rss`, `atom`, `json` (default `all`) |

**Response:** JSON array of discovered feeds with type, URL, and title.
</details>

<details>
<summary><strong><code>ws_connect</code></strong> — Connect to WebSocket server</summary>

Open a WebSocket connection to the specified URL, optionally with sub-protocols.

**Params:**

| Name | Type | Required | Description |
|------|------|:--------:|-------------|
| `url` | string | ✅ | WebSocket URL (`ws://` or `wss://`) |
| `protocols` | string[] | | Sub-protocols to request (e.g. `["graphql-ws"]`) |

**Response:**
```json
{ "connected": true, "url": "wss://example.com/ws", "protocol": "graphql-ws" }
```
</details>

<details>
<summary><strong><code>ws_intercept</code></strong> — Intercept WebSocket traffic</summary>

Start intercepting WebSocket frames on the page, optionally filtering by URL pattern.

**Params:**

| Name | Type | Required | Description |
|------|------|:--------:|-------------|
| `url_pattern` | string | | URL pattern to filter (glob syntax, default: all) |
| `capture_only` | boolean | | If `true`, capture without blocking (default `true`) |

**Response:** `"WebSocket interception started (pattern: <pattern>)"`
</details>

<details>
<summary><strong><code>ws_send</code></strong> — Send WebSocket message</summary>

Send a message through an active WebSocket connection.

**Params:**

| Name | Type | Required | Description |
|------|------|:--------:|-------------|
| `target` | string | ✅ | WebSocket URL or connection identifier |
| `message` | string | ✅ | Message payload (string or JSON) |

**Response:** `"sent <N> bytes to <target>"`
</details>

<details>
<summary><strong><code>ws_messages</code></strong> — Get captured WebSocket messages</summary>

Retrieve captured WebSocket messages from active interception.

**Params:**

| Name | Type | Required | Description |
|------|------|:--------:|-------------|
| `url_filter` | string | | Filter messages by WebSocket URL (glob syntax) |
| `limit` | number | | Maximum messages to return (default `100`) |

**Response:**
```json
{
  "messages": [
    { "url": "wss://example.com/ws", "direction": "received", "data": "{\"type\":\"update\"}", "timestamp": "2025-01-15T10:30:00Z" }
  ],
  "count": 1,
  "truncated": false
}
```
</details>

<details>
<summary><strong><code>ws_close</code></strong> — Close WebSocket connections</summary>

Close one or all active WebSocket connections.

**Params:**

| Name | Type | Required | Description |
|------|------|:--------:|-------------|
| `target` | string | | WebSocket URL to close (omit to close all) |

**Response:** `"closed <N> WebSocket connection(s)"`
</details>

<details>
<summary><strong><code>sse_listen</code></strong> — Listen to Server-Sent Events</summary>

Connect to an SSE endpoint and capture events for the specified duration.

**Params:**

| Name | Type | Required | Description |
|------|------|:--------:|-------------|
| `url` | string | ✅ | SSE endpoint URL |
| `duration_ms` | number | | Listen duration in milliseconds (default `5000`) |

**Response:**
```json
{
  "url": "https://example.com/events",
  "events": [
    { "type": "message", "data": "{\"count\":42}", "id": "1", "timestamp": "2025-01-15T10:30:00Z" }
  ],
  "count": 1,
  "duration_ms": 5000
}
```
</details>

<details>
<summary><strong><code>sse_messages</code></strong> — Get captured SSE messages</summary>

Retrieve previously captured SSE messages.

**Params:**

| Name | Type | Required | Description |
|------|------|:--------:|-------------|
| `url_filter` | string | | Filter by SSE endpoint URL (glob syntax) |
| `limit` | number | | Maximum messages to return (default `100`) |

**Response:**
```json
{
  "messages": [
    { "url": "https://example.com/events", "type": "message", "data": "{\"count\":42}", "id": "1", "timestamp": "2025-01-15T10:30:00Z" }
  ],
  "count": 1,
  "truncated": false
}
```
</details>

<details>
<summary><strong><code>graphql_subscribe</code></strong> — GraphQL subscription via WebSocket</summary>

Subscribe to a GraphQL subscription over WebSocket and capture incoming data for the specified duration.

**Params:**

| Name | Type | Required | Description |
|------|------|:--------:|-------------|
| `url` | string | ✅ | GraphQL WebSocket endpoint URL |
| `query` | string | ✅ | GraphQL subscription query |
| `variables` | string | | Variables as JSON object |
| `duration_ms` | number | | Listen duration in milliseconds (default `5000`) |

**Response:**
```json
{
  "subscription": "subscription { messageAdded { id text } }",
  "events": [
    { "data": { "messageAdded": { "id": "1", "text": "Hello" } }, "timestamp": "2025-01-15T10:30:00Z" }
  ],
  "count": 1,
  "duration_ms": 5000
}
```
</details>

---

### 6. `secure`

Cryptography, encrypted storage, WebAuthn passkey management, and authentication flows.

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
| `auth_oauth2` | `{auth_url, token_url, client_id, redirect_uri?, scopes?, use_pkce?}` | OAuth2 authorization flow with PKCE |
| `auth_session` | `{name, export?, import_data?}` | Export/import browser session |
| `auth_form_login` | `{url, username, password, username_sel?, password_sel?, submit_sel?}` | Automated form-based login |
| `auth_mfa` | `{mfa_type, totp_secret?, code?, code_selector?, submit_selector?}` | Handle MFA/2FA challenges |
| `auth_status` | — | Check authentication status |
| `auth_logout` | — | Clear all auth state |
| `credential_store` | `{label, username, password, domain?, metadata?}` | Store credentials in encrypted vault |
| `credential_get` | `{label}` | Retrieve stored credentials |

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

<details>
<summary><strong><code>auth_oauth2</code></strong> — OAuth2 authorization flow</summary>

Initiate OAuth2 authorization with optional PKCE support. Generates authorization URL and PKCE pair.

| Parameter | Type | Required | Description |
|-----------|------|:--------:|-------------|
| `auth_url` | string | ✅ | Authorization endpoint URL |
| `token_url` | string | ✅ | Token exchange endpoint URL |
| `client_id` | string | ✅ | OAuth2 client ID |
| `redirect_uri` | string | | Redirect URI (default `http://localhost:3000/callback`) |
| `scopes` | string[] | | OAuth2 scopes (default `["openid", "profile", "email"]`) |
| `use_pkce` | boolean | | Enable PKCE S256 challenge (default `true`) |

**Response:** JSON with authorization URL, PKCE pair, and token endpoint.
</details>

<details>
<summary><strong><code>auth_session</code></strong> — Export/import browser session</summary>

Export current browser session (cookies, localStorage) or import a saved session.

| Parameter | Type | Required | Description |
|-----------|------|:--------:|-------------|
| `name` | string | ✅ | Session name identifier |
| `export` | boolean | | Export current session if `true` (default `true`) |
| `import_data` | string | | JSON session data to import |

**Response:** JSON session data (on export) or import confirmation.
</details>

<details>
<summary><strong><code>auth_form_login</code></strong> — Automated form login</summary>

Navigate to a login page and perform automated form-based authentication.

| Parameter | Type | Required | Description |
|-----------|------|:--------:|-------------|
| `url` | string | ✅ | Login page URL |
| `username` | string | ✅ | Username/email to enter |
| `password` | string | ✅ | Password to enter |
| `username_sel` | string | | CSS selector for username field |
| `password_sel` | string | | CSS selector for password field |
| `submit_sel` | string | | CSS selector for submit button |

**Response:** JSON with login result, final URL, and authentication status.
</details>

<details>
<summary><strong><code>auth_mfa</code></strong> — Handle MFA/2FA</summary>

Handle multi-factor authentication challenges with TOTP auto-generation or manual code entry.

| Parameter | Type | Required | Description |
|-----------|------|:--------:|-------------|
| `mfa_type` | string | ✅ | MFA type: `totp`, `sms`, `email` |
| `totp_secret` | string | | Base32-encoded TOTP secret for auto-generation |
| `code` | string | | Manual MFA code to enter |
| `code_selector` | string | | CSS selector for code input field |
| `submit_selector` | string | | CSS selector for submit button |

**Response:** JSON with MFA verification result and status.
</details>

<details>
<summary><strong><code>auth_status</code></strong> — Check auth status</summary>

Check current authentication status including cookies, sessions, and stored credentials.

**Response:** JSON with cookie count, auth sessions, stored credentials count.
</details>

<details>
<summary><strong><code>auth_logout</code></strong> — Clear all auth state</summary>

Clear all authentication state: cookies, localStorage, sessionStorage, and auth sessions.

**Response:** JSON confirming all auth state has been cleared.
</details>

<details>
<summary><strong><code>credential_store</code></strong> — Store credentials</summary>

Store credentials in the encrypted KV vault for later retrieval.

| Parameter | Type | Required | Description |
|-----------|------|:--------:|-------------|
| `label` | string | ✅ | Unique label for the credential |
| `username` | string | ✅ | Username to store |
| `password` | string | ✅ | Password to store |
| `domain` | string | | Associated domain |
| `metadata` | object | | Additional metadata |

**Response:** JSON confirming credential storage.
</details>

<details>
<summary><strong><code>credential_get</code></strong> — Retrieve credentials</summary>

Retrieve stored credentials from the encrypted vault by label.

| Parameter | Type | Required | Description |
|-----------|------|:--------:|-------------|
| `label` | string | ✅ | Credential label to retrieve |

**Response:** JSON with stored credential data or not-found status.
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
| `fleet_spawn` | `{count?, fleet_name?}` | Launch multi-browser fleet |
| `fleet_broadcast` | `{fleet_name, action}` | Send action to all fleet instances |
| `fleet_collect` | `{fleet_name, selector?, attribute?}` | Collect data from all instances |
| `fleet_destroy` | `{fleet_name}` | Terminate fleet |
| `fleet_status` | — | Get all fleet statuses |
| `fleet_balance` | `{fleet_name, urls}` | Distribute URLs across fleet |

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

<details>
<summary><strong><code>fleet_spawn</code></strong> — Launch browser fleet</summary>

Spawn a fleet of parallel browser instances for distributed automation.

| Parameter | Type | Required | Description |
|-----------|------|:--------:|-------------|
| `count` | number | | Number of instances (default 3, max 10) |
| `fleet_name` | string | | Fleet identifier (default `default`) |

**Response:** JSON with fleet name, instance count, and instance IDs.
</details>

<details>
<summary><strong><code>fleet_broadcast</code></strong> — Broadcast action to fleet</summary>

Send the same action to all instances in a browser fleet.

| Parameter | Type | Required | Description |
|-----------|------|:--------:|-------------|
| `fleet_name` | string | ✅ | Fleet to target |
| `action` | string | ✅ | Action to execute on each instance |

**Response:** JSON with per-instance results and success/failure counts.
</details>

<details>
<summary><strong><code>fleet_collect</code></strong> — Collect fleet data</summary>

Collect and aggregate data from all fleet instances.

| Parameter | Type | Required | Description |
|-----------|------|:--------:|-------------|
| `fleet_name` | string | ✅ | Fleet to collect from |
| `selector` | string | | CSS selector to extract |
| `attribute` | string | | Attribute to extract from elements |

**Response:** JSON array with collected data from each instance.
</details>

<details>
<summary><strong><code>fleet_destroy</code></strong> — Terminate fleet</summary>

Destroy all instances in a browser fleet.

| Parameter | Type | Required | Description |
|-----------|------|:--------:|-------------|
| `fleet_name` | string | ✅ | Fleet to destroy |

**Response:** JSON confirming fleet termination.
</details>

<details>
<summary><strong><code>fleet_status</code></strong> — Fleet status overview</summary>

Get status of all active browser fleets.

**Params:** None

**Response:** JSON with fleet names, instance counts, and utilization stats.
</details>

<details>
<summary><strong><code>fleet_balance</code></strong> — Distribute URLs across fleet</summary>

Load-balance a list of URLs across fleet instances for parallel processing.

| Parameter | Type | Required | Description |
|-----------|------|:--------:|-------------|
| `fleet_name` | string | ✅ | Fleet to distribute to |
| `urls` | string[] | ✅ | URLs to distribute |

**Response:** JSON with URL-to-instance assignment mapping.
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
| | | **Error Recovery** |
| `retry_adapt` | `{action, params, max_retries?, strategy?, on_error?, alternative_action?, alternative_params?}` | Smart retry with adaptive strategy |
| `error_classify` | `{error_message}` | Classify an error message into categories |
| `recovery_suggest` | `{error_type, context?}` | Get recovery suggestions for error type |
| `error_history` | — | Get recent error log |
| | | **Session Checkpoints** |
| `checkpoint_save` | `{name, include_cookies?, include_storage?, include_context?}` | Save browser state snapshot |
| `checkpoint_restore` | `{name, restore_url?, restore_cookies?}` | Restore from checkpoint |
| `checkpoint_list` | — | List checkpoints |
| `checkpoint_delete` | `{name}` | Delete checkpoint |
| | | **Workflow Control Flow** |
| `workflow_while` | `{condition, actions, max_iterations?}` | Loop while condition true |
| `workflow_for_each` | `{collection, variable_name?, actions}` | Iterate over collection |
| `workflow_if` | `{condition, then_actions, else_actions?}` | Conditional execution |
| `workflow_variable` | `{name, value?}` | Get/set workflow variable |

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

##### Error Recovery

<details>
<summary><strong><code>retry_adapt</code></strong> — Smart retry with adaptive strategy</summary>

Execute an action with intelligent retry logic. Supports multiple strategies (exponential backoff, linear, immediate) and optional fallback to an alternative action on exhaustion.

**Params:**

| Name | Type | Required | Description |
|------|------|:--------:|-------------|
| `action` | string | ✅ | Primary action to attempt |
| `params` | object | ✅ | Parameters for the primary action |
| `max_retries` | integer | | Maximum retry attempts (default `3`) |
| `strategy` | string | | Retry strategy: `exponential`, `linear`, `immediate` (default `exponential`) |
| `on_error` | string | | Error handling mode: `retry`, `fallback`, `abort` (default `retry`) |
| `alternative_action` | string | | Fallback action if primary exhausts retries (requires `on_error: "fallback"`) |
| `alternative_params` | object | | Parameters for the fallback action |

**Response:**
```json
{
  "strategy": "exponential",
  "max_retries": 3,
  "action": "goto",
  "fallback": { "action": "goto", "params": { "url": "https://cached.example.com" } },
  "plan": ["attempt_1: 0ms", "attempt_2: 1000ms", "attempt_3: 4000ms", "fallback"]
}
```
</details>

<details>
<summary><strong><code>error_classify</code></strong> — Classify an error message</summary>

Classify an error message into a structured category with severity and recoverability assessment.

**Params:**

| Name | Type | Required | Description |
|------|------|:--------:|-------------|
| `error_message` | string | ✅ | The error message to classify |

**Response:**
```json
{
  "category": "network",
  "severity": "medium",
  "recoverable": true,
  "suggestions": ["Check network connectivity", "Retry with exponential backoff", "Verify URL is reachable"]
}
```
</details>

<details>
<summary><strong><code>recovery_suggest</code></strong> — Get recovery suggestions for error type</summary>

Get contextual recovery strategies for a specific error type. Optionally provide additional context for more targeted suggestions.

**Params:**

| Name | Type | Required | Description |
|------|------|:--------:|-------------|
| `error_type` | string | ✅ | Error type or category (e.g. `timeout`, `selector_not_found`, `network`) |
| `context` | object | | Additional context (e.g. `{"url": "...", "selector": "..."}`) |

**Response:**
```json
{
  "error_type": "timeout",
  "strategies": [
    { "name": "increase_timeout", "description": "Double the timeout value", "confidence": 0.8 },
    { "name": "wait_for_network_idle", "description": "Wait for network idle before action", "confidence": 0.7 },
    { "name": "retry_with_reload", "description": "Reload page and retry", "confidence": 0.6 }
  ]
}
```
</details>

<details>
<summary><strong><code>error_history</code></strong> — Get recent error log</summary>

Retrieve the recent error log for the current session. Useful for debugging recurring issues and identifying patterns.

**Params:** None

**Response:**
```json
{
  "errors": [
    { "timestamp": "2025-01-15T10:30:00Z", "category": "timeout", "message": "Element not found within 30s", "action": "wait", "recoverable": true },
    { "timestamp": "2025-01-15T10:29:55Z", "category": "network", "message": "net::ERR_CONNECTION_REFUSED", "action": "goto", "recoverable": true }
  ],
  "count": 2
}
```
</details>

<details>
<summary><strong><code>checkpoint_save</code></strong> — Save browser state snapshot</summary>

Save a named checkpoint of the current browser state, including URL, cookies, storage, and page context.

**Params:**

| Name | Type | Required | Description |
|------|------|:--------:|-------------|
| `name` | string | ✅ | Checkpoint name |
| `include_cookies` | boolean | | Include cookies in snapshot (default `true`) |
| `include_storage` | boolean | | Include localStorage/sessionStorage (default `true`) |
| `include_context` | boolean | | Include page context variables (default `true`) |

**Response:**
```json
{
  "name": "after-login",
  "saved_at": "2025-01-15T10:30:00Z",
  "url": "https://example.com/dashboard",
  "has_cookies": true,
  "has_storage": true,
  "has_context": true
}
```
</details>

<details>
<summary><strong><code>checkpoint_restore</code></strong> — Restore from checkpoint</summary>

Restore browser state from a previously saved checkpoint.

**Params:**

| Name | Type | Required | Description |
|------|------|:--------:|-------------|
| `name` | string | ✅ | Checkpoint name to restore |
| `restore_url` | boolean | | Navigate to saved URL (default `true`) |
| `restore_cookies` | boolean | | Restore cookies (default `true`) |

**Response:**
```json
{
  "name": "after-login",
  "restored_at": "2025-01-15T10:35:00Z",
  "url": "https://example.com/dashboard",
  "cookies_restored": true,
  "storage_restored": true
}
```
</details>

<details>
<summary><strong><code>checkpoint_list</code></strong> — List checkpoints</summary>

List all saved checkpoints for the current session.

**Params:** None

**Response:**
```json
{
  "checkpoints": [
    { "name": "after-login", "saved_at": "2025-01-15T10:30:00Z", "url": "https://example.com/dashboard" },
    { "name": "before-checkout", "saved_at": "2025-01-15T10:32:00Z", "url": "https://example.com/cart" }
  ],
  "count": 2
}
```
</details>

<details>
<summary><strong><code>checkpoint_delete</code></strong> — Delete checkpoint</summary>

Delete a saved checkpoint by name.

**Params:**

| Name | Type | Required | Description |
|------|------|:--------:|-------------|
| `name` | string | ✅ | Checkpoint name to delete |

**Response:**
```json
{ "name": "after-login", "deleted": true }
```
</details>

<details>
<summary><strong><code>workflow_while</code></strong> — Loop while condition true</summary>

Execute a set of actions in a loop while a JavaScript condition evaluates to true. Includes a safety limit on iterations.

**Params:**

| Name | Type | Required | Description |
|------|------|:--------:|-------------|
| `condition` | string | ✅ | JavaScript expression evaluated each iteration |
| `actions` | array | ✅ | List of action objects to execute per iteration |
| `max_iterations` | number | | Safety limit (default `100`) |

**Response:**
```json
{ "iterations_executed": 5, "results": [ /* per-iteration results */ ] }
```
</details>

<details>
<summary><strong><code>workflow_for_each</code></strong> — Iterate over collection</summary>

Execute actions for each item in a collection (CSS selector results, array, or workflow variable).

**Params:**

| Name | Type | Required | Description |
|------|------|:--------:|-------------|
| `collection` | string | ✅ | CSS selector, variable name, or JSON array |
| `variable_name` | string | | Name for the current item variable (default `item`) |
| `actions` | array | ✅ | List of action objects to execute per item |

**Response:**
```json
{ "items_processed": 10, "results": [ /* per-item results */ ] }
```
</details>

<details>
<summary><strong><code>workflow_if</code></strong> — Conditional execution</summary>

Execute one of two action branches based on a JavaScript condition.

**Params:**

| Name | Type | Required | Description |
|------|------|:--------:|-------------|
| `condition` | string | ✅ | JavaScript expression to evaluate |
| `then_actions` | array | ✅ | Actions if condition is true |
| `else_actions` | array | | Actions if condition is false |

**Response:**
```json
{ "condition_value": true, "branch_taken": "then", "results": [ /* branch results */ ] }
```
</details>

<details>
<summary><strong><code>workflow_variable</code></strong> — Get/set workflow variable</summary>

Get or set a workflow variable. If `value` is provided, sets the variable; otherwise returns its current value.

**Params:**

| Name | Type | Required | Description |
|------|------|:--------:|-------------|
| `name` | string | ✅ | Variable name |
| `value` | any | | Value to set (omit to get current value) |

**Response:**
```json
{ "name": "page_count", "value": 42, "action": "get" }
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

*Auto-generated from OneCrawl MCP server source (`onecrawl-mcp-rs`). Total: 10 tools, 239 actions.*
