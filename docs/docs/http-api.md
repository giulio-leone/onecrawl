---
sidebar_position: 3
title: HTTP Server API
---

# HTTP Server API

OneCrawl includes an HTTP API server with **43 routes** for multi-instance Chrome management. The server uses accessibility-based element references (`ref`) for reliable, selector-free automation.

```bash
onecrawl serve --port 9867
```

**Base URL:** `http://localhost:9867`

---

## Quick Start

A complete automation flow in 6 steps:

```bash
# 1. Create a Chrome instance
INSTANCE=$(curl -s -X POST http://localhost:9867/instances \
  -H "Content-Type: application/json" \
  -d '{"headless": true}' | jq -r '.id')

# 2. Open a tab
TAB=$(curl -s -X POST http://localhost:9867/instances/$INSTANCE/tabs/open \
  -H "Content-Type: application/json" \
  -d '{"url": "https://example.com"}' | jq -r '.tab_id')

# 3. Get the accessibility snapshot (interactive elements only)
curl -s "http://localhost:9867/tabs/$TAB/snapshot?filter=interactive"

# 4. Click an element by ref
curl -s -X POST http://localhost:9867/tabs/$TAB/action \
  -H "Content-Type: application/json" \
  -d '{"ref": "e3", "action": "click"}'

# 5. Get the page text
curl -s "http://localhost:9867/tabs/$TAB/text"

# 6. Clean up
curl -s -X DELETE http://localhost:9867/instances/$INSTANCE
```

---

## Authentication

The HTTP server does **not** require authentication by default. For production deployments, use a reverse proxy (nginx, Caddy) or bind to `127.0.0.1`:

```bash
# Bind to localhost only (default)
onecrawl serve --port 9867 --bind 127.0.0.1

# Bind to all interfaces (use with caution)
onecrawl serve --port 9867 --bind 0.0.0.0
```

> **Security note:** The server grants full browser control. Never expose it to the public internet without authentication middleware.

---

## Response Format

All responses use `Content-Type: application/json`. The server supports gzip compression via `Accept-Encoding: gzip`.

### Success Response

```json
{
  "id": "inst_a1b2c3",
  "status": "ok"
}
```

### Error Response

All errors follow a consistent format:

```json
{
  "error": "instance_not_found",
  "message": "No instance with ID 'inst_invalid' exists",
  "status": 404
}
```

| Status Code | Meaning |
|---|---|
| `200` | Success |
| `201` | Created (new instance, tab, or profile) |
| `400` | Bad request — invalid body or missing fields |
| `403` | Forbidden — not the lock owner |
| `404` | Not found — instance, tab, or profile does not exist |
| `409` | Conflict — resource already exists or tab is locked |
| `500` | Internal server error — browser crash or unexpected failure |

---

## Health & Server Info

### `GET /health`

Returns server status, uptime, and resource counts.

```bash
curl http://localhost:9867/health
```

```json
{
  "status": "ok",
  "uptime_seconds": 3621,
  "instances": 2,
  "tabs": 5,
  "version": "3.0.0"
}
```

### `GET /info`

Returns detailed server information including version, capabilities, and resource usage.

```bash
curl http://localhost:9867/info
```

---

## Instance Management

### `POST /instances` — Create Chrome Instance

Launch a new Chrome instance.

| Field | Type | Default | Description |
|---|---|---|---|
| `profile` | `string?` | `null` | Named profile for persistent cookies/storage |
| `headless` | `bool?` | `true` | Run in headless mode |
| `args` | `string[]?` | `[]` | Extra Chrome launch arguments |
| `proxy` | `string?` | `null` | Proxy server URL |

```bash
curl -X POST http://localhost:9867/instances \
  -H "Content-Type: application/json" \
  -d '{"headless": true, "profile": "default"}'
```

```json
{
  "id": "inst_a1b2c3",
  "profile": "default",
  "headless": true,
  "created_at": "2025-01-15T10:30:00Z"
}
```

### `GET /instances` — List All Instances

```bash
curl http://localhost:9867/instances
```

```json
[
  {
    "id": "inst_a1b2c3",
    "profile": "default",
    "headless": true,
    "tabs": 3,
    "created_at": "2025-01-15T10:30:00Z"
  }
]
```

### `GET /instances/:id` — Get Instance Details

```bash
curl http://localhost:9867/instances/inst_a1b2c3
```

```json
{
  "id": "inst_a1b2c3",
  "profile": "default",
  "headless": true,
  "tabs": [
    {"tab_id": "tab_x1y2z3", "url": "https://example.com", "title": "Example"}
  ],
  "memory_mb": 142.5,
  "created_at": "2025-01-15T10:30:00Z"
}
```

### `DELETE /instances/:id` — Stop Instance

```bash
curl -X DELETE http://localhost:9867/instances/inst_a1b2c3
```

```json
{"id": "inst_a1b2c3", "status": "stopped"}
```

### `POST /instances/:id/restart` — Restart Instance

```bash
curl -X POST http://localhost:9867/instances/inst_a1b2c3/restart
```

---

## Tab Management

### `POST /instances/:id/tabs/open` — Open Tab

| Field | Type | Default | Description |
|---|---|---|---|
| `url` | `string?` | `about:blank` | Initial URL to load |

```bash
curl -X POST http://localhost:9867/instances/inst_a1b2c3/tabs/open \
  -H "Content-Type: application/json" \
  -d '{"url": "https://example.com"}'
```

```json
{"tab_id": "tab_x1y2z3", "instance_id": "inst_a1b2c3", "url": "https://example.com"}
```

### `GET /instances/:id/tabs` — List Tabs for Instance

```bash
curl http://localhost:9867/instances/inst_a1b2c3/tabs
```

### `GET /tabs` — List All Tabs (across all instances)

```bash
curl http://localhost:9867/tabs
```

```json
[
  {"tab_id": "tab_x1y2z3", "instance_id": "inst_a1b2c3", "url": "https://example.com", "title": "Example Domain"},
  {"tab_id": "tab_d4e5f6", "instance_id": "inst_a1b2c3", "url": "https://github.com", "title": "GitHub"}
]
```

### `DELETE /tabs/:tab_id` — Close Tab

```bash
curl -X DELETE http://localhost:9867/tabs/tab_x1y2z3
```

---

## Navigation

### `POST /tabs/:tab_id/navigate` — Navigate Tab

```bash
curl -X POST http://localhost:9867/tabs/tab_x1y2z3/navigate \
  -H "Content-Type: application/json" \
  -d '{"url": "https://github.com"}'
```

```json
{"tab_id": "tab_x1y2z3", "url": "https://github.com", "status": 200}
```

### `POST /tabs/:tab_id/back` — Go Back

```bash
curl -X POST http://localhost:9867/tabs/tab_x1y2z3/back
```

### `POST /tabs/:tab_id/forward` — Go Forward

```bash
curl -X POST http://localhost:9867/tabs/tab_x1y2z3/forward
```

### `POST /tabs/:tab_id/reload` — Reload Page

```bash
curl -X POST http://localhost:9867/tabs/tab_x1y2z3/reload
```

---

## Content Extraction

### `GET /tabs/:tab_id/text` — Get Visible Text

Token-efficient text extraction, suitable for LLM consumption.

```bash
curl http://localhost:9867/tabs/tab_x1y2z3/text
```

```json
{"text": "Example Domain\n\nThis domain is for use in illustrative examples...", "length": 234}
```

### `GET /tabs/:tab_id/html` — Get Page HTML

```bash
curl http://localhost:9867/tabs/tab_x1y2z3/html
```

### `GET /tabs/:tab_id/url` — Get Current URL

```bash
curl http://localhost:9867/tabs/tab_x1y2z3/url
```

### `GET /tabs/:tab_id/title` — Get Page Title

```bash
curl http://localhost:9867/tabs/tab_x1y2z3/title
```

### `GET /tabs/:tab_id/markdown` — Get Page as Markdown

```bash
curl http://localhost:9867/tabs/tab_x1y2z3/markdown
```

---

## Accessibility Snapshots

### `GET /tabs/:tab_id/snapshot` — Accessibility Snapshot

Get the accessibility tree for a tab. Interactive elements receive stable `ref` IDs for use in actions.

| Parameter | Values | Description |
|---|---|---|
| `filter` | `interactive`, `all` | Filter elements (default: `all`) |
| `format` | `full`, `compact` | Output format (default: `full`) |

```bash
curl "http://localhost:9867/tabs/tab_x1y2z3/snapshot?filter=interactive&format=compact"
```

```json
{
  "elements": [
    {"ref": "e1", "role": "link", "name": "Home", "href": "/"},
    {"ref": "e2", "role": "textbox", "name": "Search", "value": ""},
    {"ref": "e3", "role": "button", "name": "Submit"}
  ]
}
```

---

## Actions (Element Interaction)

### `POST /tabs/:tab_id/action` — Execute Single Action

Execute an action on an element identified by its accessibility `ref`.

| Field | Type | Description |
|---|---|---|
| `ref` | `string` | Element reference from the accessibility snapshot |
| `action` | `string` | Action: `click`, `fill`, `type`, `hover`, `focus`, `check`, `uncheck`, `select`, `upload` |
| `text` | `string?` | Text value for `fill`, `type`, and `select` actions |
| `path` | `string?` | File path for `upload` action |

```bash
curl -X POST http://localhost:9867/tabs/tab_x1y2z3/action \
  -H "Content-Type: application/json" \
  -d '{"ref": "e2", "action": "fill", "text": "search query"}'
```

```json
{"status": "ok", "ref": "e2", "action": "fill"}
```

### `POST /tabs/:tab_id/actions` — Execute Batch Actions

Execute multiple actions sequentially in a single request.

```bash
curl -X POST http://localhost:9867/tabs/tab_x1y2z3/actions \
  -H "Content-Type: application/json" \
  -d '{
    "actions": [
      {"ref": "e2", "action": "fill", "text": "OneCrawl"},
      {"ref": "e3", "action": "click"}
    ]
  }'
```

```json
{
  "results": [
    {"status": "ok", "ref": "e2", "action": "fill"},
    {"status": "ok", "ref": "e3", "action": "click"}
  ]
}
```

---

## JavaScript Evaluation

### `POST /tabs/:tab_id/evaluate` — Evaluate JavaScript

```bash
curl -X POST http://localhost:9867/tabs/tab_x1y2z3/evaluate \
  -H "Content-Type: application/json" \
  -d '{"expression": "document.querySelectorAll(\"a\").length"}'
```

```json
{"result": 42}
```

---

## Screenshots & PDF

### `GET /tabs/:tab_id/screenshot` — Take Screenshot

Returns base64-encoded PNG.

| Parameter | Values | Description |
|---|---|---|
| `fullPage` | `true`, `false` | Full scrollable page (default: `false`) |
| `format` | `png`, `jpeg`, `webp` | Image format (default: `png`) |
| `quality` | `0-100` | JPEG/WebP quality |
| `selector` | CSS selector | Capture specific element |

```bash
curl "http://localhost:9867/tabs/tab_x1y2z3/screenshot?fullPage=true"
```

```json
{"format": "png", "data": "iVBORw0KGgoAAAANSUhEUgAA..."}
```

### `GET /tabs/:tab_id/pdf` — Export PDF

```bash
curl http://localhost:9867/tabs/tab_x1y2z3/pdf
```

```json
{"format": "pdf", "data": "JVBERi0xLjQKMSAwIG9i..."}
```

---

## Tab Locking (Multi-Agent Safety)

Tab locks prevent concurrent agents from interfering with each other.

### `POST /tabs/:tab_id/lock` — Acquire Lock

```bash
curl -X POST http://localhost:9867/tabs/t1/lock \
  -H "Content-Type: application/json" \
  -d '{"owner": "agent-a", "ttl_secs": 60}'
```

**Success (200):**
```json
{"locked": true, "owner": "agent-a", "ttl_secs": 60}
```

**Conflict (409):**
```json
{"error": "tab_already_locked", "message": "Tab locked by agent-b", "current_owner": "agent-b"}
```

### `DELETE /tabs/:tab_id/lock` — Release Lock

Only the owner can release the lock.

```bash
curl -X DELETE http://localhost:9867/tabs/t1/lock \
  -H "Content-Type: application/json" \
  -d '{"owner": "agent-a"}'
```

### `GET /tabs/:tab_id/lock` — Check Lock Status

```bash
curl http://localhost:9867/tabs/t1/lock
```

```json
{"locked": false}
```

---

## Profile Management

### `GET /profiles` — List Profiles

```bash
curl http://localhost:9867/profiles
```

```json
[
  {"name": "default", "created_at": "2025-01-10T08:00:00Z", "size_mb": 24.3},
  {"name": "stealth", "created_at": "2025-01-12T14:00:00Z", "size_mb": 12.1}
]
```

### `POST /profiles` — Create Profile

```bash
curl -X POST http://localhost:9867/profiles \
  -H "Content-Type: application/json" \
  -d '{"name": "my-profile"}'
```

```json
{"name": "my-profile", "created_at": "2025-01-15T10:45:00Z"}
```

### `DELETE /profiles/:name` — Delete Profile

```bash
curl -X DELETE http://localhost:9867/profiles/my-profile
```

---

## Stealth & Network

### `POST /tabs/:tab_id/stealth/inject` — Inject Stealth Patches

```bash
curl -X POST http://localhost:9867/tabs/tab_x1y2z3/stealth/inject \
  -H "Content-Type: application/json" \
  -d '{"level": "maximum"}'
```

### `POST /tabs/:tab_id/network/throttle` — Network Throttling

```bash
curl -X POST http://localhost:9867/tabs/tab_x1y2z3/network/throttle \
  -H "Content-Type: application/json" \
  -d '{"profile": "3g"}'
```

### `POST /tabs/:tab_id/network/intercept` — Request Interception

```bash
curl -X POST http://localhost:9867/tabs/tab_x1y2z3/network/intercept \
  -H "Content-Type: application/json" \
  -d '{"pattern": "**/*.png", "action": "block"}'
```

---

## HAR & Network Logging

### `POST /tabs/:tab_id/har/start` — Start HAR Recording

```bash
curl -X POST http://localhost:9867/tabs/tab_x1y2z3/har/start
```

### `POST /tabs/:tab_id/har/stop` — Stop HAR Recording

```bash
curl -X POST http://localhost:9867/tabs/tab_x1y2z3/har/stop
```

### `GET /tabs/:tab_id/har/export` — Export HAR Data

```bash
curl http://localhost:9867/tabs/tab_x1y2z3/har/export
```

---

## Cookies

### `GET /tabs/:tab_id/cookies` — Get Cookies

```bash
curl http://localhost:9867/tabs/tab_x1y2z3/cookies
```

### `POST /tabs/:tab_id/cookies` — Set Cookie

```bash
curl -X POST http://localhost:9867/tabs/tab_x1y2z3/cookies \
  -H "Content-Type: application/json" \
  -d '{"name": "session", "value": "abc123", "domain": ".example.com"}'
```

### `DELETE /tabs/:tab_id/cookies` — Clear Cookies

```bash
curl -X DELETE http://localhost:9867/tabs/tab_x1y2z3/cookies
```

---

## WebSocket / SSE Endpoints

### `GET /events` — Server-Sent Events Stream

Subscribe to real-time server events.

```bash
curl -N http://localhost:9867/events
```

Events include:
- `instance.created` / `instance.stopped`
- `tab.opened` / `tab.closed`
- `navigation.completed`
- `error`

### `GET /ws` — WebSocket Connection

Connect via WebSocket for bidirectional real-time communication.

```javascript
const ws = new WebSocket("ws://localhost:9867/ws");
ws.onmessage = (event) => console.log(JSON.parse(event.data));
ws.send(JSON.stringify({action: "subscribe", events: ["navigation.completed"]}));
```

---

## All 43 Routes Summary

| # | Method | Route | Description |
|---|---|---|---|
| 1 | `GET` | `/health` | Health check |
| 2 | `GET` | `/info` | Server information |
| 3 | `POST` | `/instances` | Create Chrome instance |
| 4 | `GET` | `/instances` | List all instances |
| 5 | `GET` | `/instances/:id` | Get instance details |
| 6 | `DELETE` | `/instances/:id` | Stop instance |
| 7 | `POST` | `/instances/:id/restart` | Restart instance |
| 8 | `POST` | `/instances/:id/tabs/open` | Open a new tab |
| 9 | `GET` | `/instances/:id/tabs` | List tabs for instance |
| 10 | `GET` | `/tabs` | List all tabs |
| 11 | `DELETE` | `/tabs/:id` | Close a tab |
| 12 | `POST` | `/tabs/:id/navigate` | Navigate tab to URL |
| 13 | `POST` | `/tabs/:id/back` | Go back |
| 14 | `POST` | `/tabs/:id/forward` | Go forward |
| 15 | `POST` | `/tabs/:id/reload` | Reload page |
| 16 | `GET` | `/tabs/:id/snapshot` | Accessibility snapshot |
| 17 | `GET` | `/tabs/:id/text` | Get visible text |
| 18 | `GET` | `/tabs/:id/html` | Get page HTML |
| 19 | `GET` | `/tabs/:id/url` | Get current URL |
| 20 | `GET` | `/tabs/:id/title` | Get page title |
| 21 | `GET` | `/tabs/:id/markdown` | Get page as Markdown |
| 22 | `POST` | `/tabs/:id/action` | Execute single action |
| 23 | `POST` | `/tabs/:id/actions` | Execute batch actions |
| 24 | `POST` | `/tabs/:id/evaluate` | Evaluate JavaScript |
| 25 | `GET` | `/tabs/:id/screenshot` | Take screenshot |
| 26 | `GET` | `/tabs/:id/pdf` | Export PDF |
| 27 | `POST` | `/tabs/:id/lock` | Acquire tab lock |
| 28 | `DELETE` | `/tabs/:id/lock` | Release tab lock |
| 29 | `GET` | `/tabs/:id/lock` | Check lock status |
| 30 | `GET` | `/tabs/:id/cookies` | Get cookies |
| 31 | `POST` | `/tabs/:id/cookies` | Set cookie |
| 32 | `DELETE` | `/tabs/:id/cookies` | Clear cookies |
| 33 | `POST` | `/tabs/:id/stealth/inject` | Inject stealth patches |
| 34 | `POST` | `/tabs/:id/network/throttle` | Network throttling |
| 35 | `POST` | `/tabs/:id/network/intercept` | Request interception |
| 36 | `POST` | `/tabs/:id/har/start` | Start HAR recording |
| 37 | `POST` | `/tabs/:id/har/stop` | Stop HAR recording |
| 38 | `GET` | `/tabs/:id/har/export` | Export HAR data |
| 39 | `GET` | `/profiles` | List profiles |
| 40 | `POST` | `/profiles` | Create profile |
| 41 | `DELETE` | `/profiles/:name` | Delete profile |
| 42 | `GET` | `/events` | SSE event stream |
| 43 | `GET` | `/ws` | WebSocket connection |

---

## Complete Workflow Script

```bash
#!/usr/bin/env bash
set -euo pipefail

BASE="http://localhost:9867"

echo "=== OneCrawl HTTP API Workflow ==="

# 1. Create a headless Chrome instance
echo "[1/7] Creating Chrome instance..."
INSTANCE=$(curl -sf -X POST "$BASE/instances" \
  -H "Content-Type: application/json" \
  -d '{"headless": true}' | jq -r '.id')
echo "       Instance: $INSTANCE"

# 2. Open a tab
echo "[2/7] Opening tab..."
TAB=$(curl -sf -X POST "$BASE/instances/$INSTANCE/tabs/open" \
  -H "Content-Type: application/json" \
  -d '{"url": "https://example.com"}' | jq -r '.tab_id')
echo "       Tab: $TAB"

# 3. Take accessibility snapshot
echo "[3/7] Getting interactive elements..."
curl -sf "$BASE/tabs/$TAB/snapshot?filter=interactive&format=compact" | jq '.elements[:5]'

# 4. Interact with elements
echo "[4/7] Filling form and submitting..."
curl -sf -X POST "$BASE/tabs/$TAB/actions" \
  -H "Content-Type: application/json" \
  -d '{
    "actions": [
      {"ref": "e2", "action": "fill", "text": "OneCrawl"},
      {"ref": "e3", "action": "click"}
    ]
  }' | jq .

# 5. Wait for results
sleep 2

# 6. Extract page text
echo "[5/7] Extracting text..."
curl -sf "$BASE/tabs/$TAB/text" | jq -r '.text' | head -20

# 7. Screenshot
echo "[6/7] Taking screenshot..."
curl -sf "$BASE/tabs/$TAB/screenshot?fullPage=true" | jq -r '.data' | base64 -d > result.png

# Cleanup
echo "[7/7] Cleaning up..."
curl -sf -X DELETE "$BASE/instances/$INSTANCE" | jq .
echo "Done!"
```
