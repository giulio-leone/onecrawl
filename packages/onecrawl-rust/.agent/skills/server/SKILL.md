---
name: server
description: "axum HTTP API server with multi-instance Chrome management, tab lifecycle, accessibility snapshots, action API, profile management, and gzip compression."
---

# HTTP API Server Skill

Multi-instance Chrome browser management via REST API. Launch parallel browser
instances, manage tabs, take accessibility snapshots, execute actions by element
ref, and extract content -- all through a clean HTTP interface.

## Quick Start

```bash
# Start the server
onecrawl serve --port 9867

# Launch a browser instance
curl -X POST http://localhost:9867/instances \
  -H "Content-Type: application/json" \
  -d '{"headless": true}'

# Open a tab and navigate
curl -X POST http://localhost:9867/instances/INST_ID/tabs/open \
  -H "Content-Type: application/json" \
  -d '{"url": "https://example.com"}'

# Get accessibility snapshot
curl http://localhost:9867/tabs/TAB_ID/snapshot?filter=interactive

# Click element by ref
curl -X POST http://localhost:9867/tabs/TAB_ID/action \
  -H "Content-Type: application/json" \
  -d '{"Click": {"ref_id": "e5"}}'
```

## Endpoints (21)

### Instance Management

| Method | Endpoint | Body | Description |
|--------|----------|------|-------------|
| `POST` | `/instances` | `{"headless": bool, "profile": str?}` | Launch Chrome instance |
| `GET` | `/instances` | -- | List all instances |
| `GET` | `/instances/{id}` | -- | Get instance info (status, tabs, profile) |
| `DELETE` | `/instances/{id}` | -- | Stop instance and close all tabs |

### Tab Management

| Method | Endpoint | Body | Description |
|--------|----------|------|-------------|
| `POST` | `/instances/{id}/tabs/open` | `{"url": str?}` | Open new tab |
| `GET` | `/instances/{id}/tabs` | -- | List tabs for instance |
| `GET` | `/tabs` | -- | List all tabs across instances |

### Tab Operations

| Method | Endpoint | Body | Description |
|--------|----------|------|-------------|
| `POST` | `/tabs/{id}/navigate` | `{"url": str}` | Navigate tab |
| `GET` | `/tabs/{id}/snapshot` | -- | Accessibility snapshot |
| `GET` | `/tabs/{id}/text` | -- | Extract visible text |
| `GET` | `/tabs/{id}/url` | -- | Get current URL |
| `GET` | `/tabs/{id}/title` | -- | Get page title |
| `GET` | `/tabs/{id}/html` | -- | Get full HTML |
| `POST` | `/tabs/{id}/action` | Action JSON | Execute single action |
| `POST` | `/tabs/{id}/actions` | Action[] JSON | Execute action batch |
| `POST` | `/tabs/{id}/evaluate` | `{"expression": str}` | Evaluate JavaScript |
| `GET` | `/tabs/{id}/screenshot` | -- | Screenshot (base64 PNG) |
| `GET` | `/tabs/{id}/pdf` | -- | Export PDF (base64) |

### Profiles

| Method | Endpoint | Body | Description |
|--------|----------|------|-------------|
| `GET` | `/profiles` | -- | List browser profiles |
| `POST` | `/profiles` | `{"name": str}` | Create profile |

### Utility

| Method | Endpoint | Description |
|--------|----------|-------------|
| `GET` | `/health` | Health check (`{"status": "ok"}`) |

## Action API

Actions use stable element refs (`e0`, `e1`, ...) from accessibility snapshots.

```json
{"Click": {"ref_id": "e5"}}
{"Type": {"ref_id": "e12", "text": "hello world"}}
{"Fill": {"ref_id": "e12", "text": "new value"}}
{"Press": {"key": "Enter"}}
{"Press": {"key": "Tab", "ref_id": "e3"}}
{"Hover": {"ref_id": "e3"}}
{"Focus": {"ref_id": "e3"}}
{"Scroll": {"ref_id": "e3"}}
{"Select": {"ref_id": "e7", "value": "option1"}}
{"Wait": {"ms": 1000}}
{"Batch": {"actions": [...]}}
```

Batch actions stop on first failure.

## Snapshot Query Parameters

```
GET /tabs/{id}/snapshot                     # Full snapshot
GET /tabs/{id}/snapshot?filter=interactive  # Buttons, links, inputs only
GET /tabs/{id}/snapshot?compact=true        # Minimal output
```

## Architecture

- Built on axum with tower middleware (gzip compression)
- Each instance runs an isolated Chrome process on a unique debug port
- Tabs are identified by globally unique IDs
- `AppState` holds instances behind `RwLock` with MAX_SNAPSHOTS=64 eviction
- `get_tab_page()` helper clones cheap Page channel handles, drops locks
- All responses use typed `#[derive(Serialize)]` structs (no `json!()` macros)
- Profile directories persist cookies, localStorage, history across sessions

## Multi-Agent Safety

The server supports concurrent access from multiple AI agents:
- Each tab has an independent state
- Actions are serialized per-tab
- Snapshot caching prevents redundant DOM walks
- gzip compression reduces bandwidth for large snapshots
