---
sidebar_position: 3
title: HTTP Server API
---

# HTTP Server API

Start the OneCrawl HTTP server to manage multiple Chrome instances and tabs via a REST API:

```bash
onecrawl serve --port 9867
```

The server provides **multi-instance Chrome management** with accessibility-based element references (`ref`). Each Chrome instance can have multiple tabs, and every tab exposes an accessibility snapshot where interactive elements are identified by stable `ref` IDs — enabling reliable, selector-free automation.

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

## Endpoints

### 1. `GET /health` — Health Check

Returns server status and uptime.

**Response:**

```json
{
  "status": "ok",
  "uptime_seconds": 3621,
  "instances": 2,
  "tabs": 5
}
```

```bash
curl http://localhost:9867/health
```

---

### 2. `POST /instances` — Create Chrome Instance

Launch a new Chrome instance.

**Request Body:**

```json
{
  "profile": "default",
  "headless": true
}
```

| Field | Type | Default | Description |
|---|---|---|---|
| `profile` | `string?` | `null` | Named profile for persistent cookies/storage |
| `headless` | `bool?` | `true` | Run in headless mode |

**Response:**

```json
{
  "id": "inst_a1b2c3",
  "profile": "default",
  "headless": true,
  "created_at": "2025-01-15T10:30:00Z"
}
```

```bash
curl -X POST http://localhost:9867/instances \
  -H "Content-Type: application/json" \
  -d '{"headless": true}'
```

---

### 3. `GET /instances` — List Instances

Returns all running Chrome instances.

**Response:**

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

```bash
curl http://localhost:9867/instances
```

---

### 4. `GET /instances/:id` — Get Instance Info

Get detailed information about a specific instance.

**Response:**

```json
{
  "id": "inst_a1b2c3",
  "profile": "default",
  "headless": true,
  "tabs": [
    { "tab_id": "tab_x1y2z3", "url": "https://example.com", "title": "Example" }
  ],
  "memory_mb": 142.5,
  "created_at": "2025-01-15T10:30:00Z"
}
```

```bash
curl http://localhost:9867/instances/inst_a1b2c3
```

---

### 5. `DELETE /instances/:id` — Stop Instance

Stop a Chrome instance and close all its tabs.

**Response:**

```json
{
  "id": "inst_a1b2c3",
  "status": "stopped"
}
```

```bash
curl -X DELETE http://localhost:9867/instances/inst_a1b2c3
```

---

### 6. `POST /instances/:id/tabs/open` — Open Tab

Open a new tab in a Chrome instance.

**Request Body:**

```json
{
  "url": "https://example.com"
}
```

| Field | Type | Default | Description |
|---|---|---|---|
| `url` | `string?` | `about:blank` | Initial URL to load |

**Response:**

```json
{
  "tab_id": "tab_x1y2z3",
  "instance_id": "inst_a1b2c3",
  "url": "https://example.com"
}
```

```bash
curl -X POST http://localhost:9867/instances/inst_a1b2c3/tabs/open \
  -H "Content-Type: application/json" \
  -d '{"url": "https://example.com"}'
```

---

### 7. `GET /instances/:id/tabs` — List Tabs for Instance

Returns all tabs belonging to a specific instance.

**Response:**

```json
[
  {
    "tab_id": "tab_x1y2z3",
    "url": "https://example.com",
    "title": "Example Domain"
  }
]
```

```bash
curl http://localhost:9867/instances/inst_a1b2c3/tabs
```

---

### 8. `GET /tabs` — List All Tabs

Returns all tabs across all instances.

**Response:**

```json
[
  {
    "tab_id": "tab_x1y2z3",
    "instance_id": "inst_a1b2c3",
    "url": "https://example.com",
    "title": "Example Domain"
  },
  {
    "tab_id": "tab_d4e5f6",
    "instance_id": "inst_a1b2c3",
    "url": "https://github.com",
    "title": "GitHub"
  }
]
```

```bash
curl http://localhost:9867/tabs
```

---

### 9. `POST /tabs/:tab_id/navigate` — Navigate Tab

Navigate an existing tab to a new URL.

**Request Body:**

```json
{
  "url": "https://github.com"
}
```

**Response:**

```json
{
  "tab_id": "tab_x1y2z3",
  "url": "https://github.com",
  "status": 200
}
```

```bash
curl -X POST http://localhost:9867/tabs/tab_x1y2z3/navigate \
  -H "Content-Type: application/json" \
  -d '{"url": "https://github.com"}'
```

---

### 10. `GET /tabs/:tab_id/snapshot` — Accessibility Snapshot

Get the accessibility tree for a tab. Use `?filter=interactive` to return only interactive elements (buttons, links, inputs). Use `?format=compact` for a compressed format.

**Query Parameters:**

| Parameter | Values | Description |
|---|---|---|
| `filter` | `interactive`, `all` | Filter elements (default: `all`) |
| `format` | `full`, `compact` | Output format (default: `full`) |

**Response (compact, interactive):**

```json
{
  "elements": [
    { "ref": "e1", "role": "link", "name": "Home", "href": "/" },
    { "ref": "e2", "role": "textbox", "name": "Search", "value": "" },
    { "ref": "e3", "role": "button", "name": "Submit" }
  ]
}
```

```bash
curl "http://localhost:9867/tabs/tab_x1y2z3/snapshot?filter=interactive&format=compact"
```

---

### 11. `GET /tabs/:tab_id/text` — Get Text

Token-efficient text extraction. Returns visible text content, suitable for LLM consumption.

**Response:**

```json
{
  "text": "Example Domain\n\nThis domain is for use in illustrative examples...",
  "length": 234
}
```

```bash
curl http://localhost:9867/tabs/tab_x1y2z3/text
```

---

### 12. `POST /tabs/:tab_id/action` — Execute Action

Execute a single action on an element by its accessibility `ref`.

**Request Body:**

```json
{
  "ref": "e2",
  "action": "fill",
  "text": "search query"
}
```

| Field | Type | Description |
|---|---|---|
| `ref` | `string` | Element reference from the accessibility snapshot |
| `action` | `string` | Action to perform: `click`, `fill`, `type`, `hover`, `focus`, `check`, `uncheck`, `select` |
| `text` | `string?` | Text value for `fill`, `type`, and `select` actions |

**Response:**

```json
{
  "status": "ok",
  "ref": "e2",
  "action": "fill"
}
```

```bash
curl -X POST http://localhost:9867/tabs/tab_x1y2z3/action \
  -H "Content-Type: application/json" \
  -d '{"ref": "e2", "action": "fill", "text": "search query"}'
```

---

### 13. `POST /tabs/:tab_id/actions` — Execute Batch Actions

Execute multiple actions sequentially in a single request.

**Request Body:**

```json
{
  "actions": [
    { "ref": "e2", "action": "fill", "text": "OneCrawl" },
    { "ref": "e3", "action": "click" }
  ]
}
```

**Response:**

```json
{
  "results": [
    { "status": "ok", "ref": "e2", "action": "fill" },
    { "status": "ok", "ref": "e3", "action": "click" }
  ]
}
```

```bash
curl -X POST http://localhost:9867/tabs/tab_x1y2z3/actions \
  -H "Content-Type: application/json" \
  -d '{"actions": [{"ref": "e2", "action": "fill", "text": "OneCrawl"}, {"ref": "e3", "action": "click"}]}'
```

---

### 14. `POST /tabs/:tab_id/evaluate` — Evaluate JavaScript

Execute arbitrary JavaScript in the tab's context and return the result.

**Request Body:**

```json
{
  "expression": "document.querySelectorAll('a').length"
}
```

**Response:**

```json
{
  "result": 42
}
```

```bash
curl -X POST http://localhost:9867/tabs/tab_x1y2z3/evaluate \
  -H "Content-Type: application/json" \
  -d '{"expression": "document.title"}'
```

---

### 15. `GET /tabs/:tab_id/screenshot` — Take Screenshot

Capture a screenshot of the tab as a base64-encoded PNG.

**Response:**

```json
{
  "format": "png",
  "data": "iVBORw0KGgoAAAANSUhEUgAA..."
}
```

```bash
curl http://localhost:9867/tabs/tab_x1y2z3/screenshot
```

---

### 16. `GET /tabs/:tab_id/pdf` — Export PDF

Export the current page as a PDF document (base64-encoded).

**Response:**

```json
{
  "format": "pdf",
  "data": "JVBERi0xLjQKMSAwIG9i..."
}
```

```bash
curl http://localhost:9867/tabs/tab_x1y2z3/pdf
```

---

### 17. `GET /profiles` — List Profiles

List all saved browser profiles.

**Response:**

```json
[
  {
    "name": "default",
    "created_at": "2025-01-10T08:00:00Z",
    "size_mb": 24.3
  },
  {
    "name": "stealth",
    "created_at": "2025-01-12T14:00:00Z",
    "size_mb": 12.1
  }
]
```

```bash
curl http://localhost:9867/profiles
```

---

### 18. `POST /profiles` — Create Profile

Create a new named browser profile for persistent cookies and storage.

**Request Body:**

```json
{
  "name": "my-profile"
}
```

**Response:**

```json
{
  "name": "my-profile",
  "created_at": "2025-01-15T10:45:00Z"
}
```

```bash
curl -X POST http://localhost:9867/profiles \
  -H "Content-Type: application/json" \
  -d '{"name": "my-profile"}'
```

---

### 19. `GET /tabs/:tab_id/url` — Get Current URL

Returns the current page URL.

### 20. `GET /tabs/:tab_id/title` — Get Page Title

Returns the current page title.

### 21. `GET /tabs/:tab_id/html` — Get Page HTML

Returns the full HTML content of the page.

### 22. `POST /tabs/:tab_id/lock` — Acquire Tab Lock

Lock a tab for exclusive access (multi-agent safety).

```bash
curl -X POST http://localhost:9867/tabs/t1/lock \
  -H "Content-Type: application/json" \
  -d '{"owner": "agent-a", "ttl_secs": 60}'
```

**Response (200):** `{"locked": true, "owner": "agent-a", "ttl_secs": 60}`
**Response (409 Conflict):** `{"error": "tab already locked", "current_owner": "agent-b"}`

### 23. `DELETE /tabs/:tab_id/lock` — Release Tab Lock

Release a previously acquired lock. Only the owner can release.

```bash
curl -X DELETE http://localhost:9867/tabs/t1/lock \
  -H "Content-Type: application/json" \
  -d '{"owner": "agent-a"}'
```

**Response (200):** `{"unlocked": true}`
**Response (403):** `{"error": "not the lock owner"}`

### 24. `GET /tabs/:tab_id/lock` — Check Tab Lock Status

Check if a tab is currently locked.

```bash
curl http://localhost:9867/tabs/t1/lock
```

**Response (unlocked):** `{"locked": false}`
**Response (locked):** `{"locked": true, "owner": "agent-a", "ttl_secs": 60}`

---

## Error Handling

All error responses follow a consistent format:

```json
{
  "error": "instance_not_found",
  "message": "No instance with ID 'inst_invalid' exists",
  "status": 404
}
```

| Status Code | Meaning |
|---|---|
| `400` | Bad request (invalid body, missing fields) |
| `404` | Resource not found (instance, tab, profile) |
| `409` | Conflict (profile already exists) |
| `500` | Internal server error |

### Error Response Examples

**404 — Resource Not Found:**

```json
{
  "error": "instance_not_found",
  "message": "No instance with ID 'inst_invalid' exists",
  "status": 404
}
```

**400 — Bad Request:**

```json
{
  "error": "bad_request",
  "message": "Missing required field 'url' in request body",
  "status": 400
}
```

**500 — Internal Server Error:**

```json
{
  "error": "browser_crash",
  "message": "Chrome instance 'inst_a1b2c3' terminated unexpectedly",
  "status": 500
}
```

**409 — Conflict:**

```json
{
  "error": "profile_exists",
  "message": "A profile named 'my-profile' already exists",
  "status": 409
}
```

---

## Complete Workflow Example

The following bash script demonstrates a full automation workflow: create an instance, open a tab, navigate, take a snapshot, interact with elements, and extract text.

```bash
#!/usr/bin/env bash
set -euo pipefail

BASE="http://localhost:9867"

echo "=== OneCrawl HTTP API Workflow ==="

# 1. Create a headless Chrome instance
echo "[1/6] Creating Chrome instance..."
INSTANCE=$(curl -sf -X POST "$BASE/instances" \
  -H "Content-Type: application/json" \
  -d '{"headless": true}' | jq -r '.id')
echo "       Instance: $INSTANCE"

# 2. Open a tab and navigate to the target URL
echo "[2/6] Opening tab..."
TAB=$(curl -sf -X POST "$BASE/instances/$INSTANCE/tabs/open" \
  -H "Content-Type: application/json" \
  -d '{"url": "https://example.com"}' | jq -r '.tab_id')
echo "       Tab: $TAB"

# 3. Navigate to a specific page
echo "[3/6] Navigating to target page..."
curl -sf -X POST "$BASE/tabs/$TAB/navigate" \
  -H "Content-Type: application/json" \
  -d '{"url": "https://example.com/search"}' | jq .

# 4. Take an accessibility snapshot to find interactive elements
echo "[4/6] Taking accessibility snapshot..."
SNAPSHOT=$(curl -sf "$BASE/tabs/$TAB/snapshot?filter=interactive&format=compact")
echo "$SNAPSHOT" | jq '.elements[:5]'

# 5. Interact: fill a search box and click submit
echo "[5/6] Filling form and clicking submit..."
curl -sf -X POST "$BASE/tabs/$TAB/actions" \
  -H "Content-Type: application/json" \
  -d '{
    "actions": [
      {"ref": "e2", "action": "fill", "text": "OneCrawl"},
      {"ref": "e3", "action": "click"}
    ]
  }' | jq .

# Wait for results to load
sleep 2

# 6. Extract the page text
echo "[6/6] Extracting page text..."
curl -sf "$BASE/tabs/$TAB/text" | jq -r '.text' | head -20

# Cleanup: destroy the instance
echo "=== Cleaning up ==="
curl -sf -X DELETE "$BASE/instances/$INSTANCE" | jq .
echo "Done!"
```
