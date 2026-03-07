# Event Reactor — Complete File Structure & Contents Index

## Summary
Two comprehensive reference documents have been created to guide Event Reactor implementation:

1. **`EVENT_REACTOR_REFERENCE.md`** (1,071 lines) — Full technical details
2. **`EVENT_REACTOR_QUICK_REFERENCE.md`** — Implementation patterns & quick lookup

Both files located at:
- `/Users/giulioleone/Sviluppo/onecrawl-dev/`

---

## COMPLETE FILE CONTENTS DELIVERED

### 1. **events.rs** (226 lines)
- **Location:** `/Users/giulioleone/Sviluppo/onecrawl-dev/packages/onecrawl-rust/crates/onecrawl-cdp/src/events.rs`

**Structures:**
- `EventType` enum (14 variants: ConsoleMessage, NetworkRequest, PageLoad, Custom, etc.)
- `BrowserEvent` struct (event_type, timestamp, data)
- `EventStream` struct with methods:
  - `new(capacity)` — creates broadcast channel
  - `subscribe()` — creates receiver
  - `sender()` — get sender handle
  - `subscriber_count()` — active subscribers

**Functions:**
- `observe_console()` — install JS console hook
- `drain_console()` — retrieve and clear console messages
- `observe_errors()` — install error/rejection listener
- `drain_errors()` — retrieve and clear errors
- `emit_custom()` — push custom event to stream
- `format_sse()` — format event as SSE string

**Key Pattern:** Uses `tokio::sync::broadcast` for pub/sub event distribution

---

### 2. **dom_observer.rs** (128 lines)
- **Location:** `/Users/giulioleone/Sviluppo/onecrawl-dev/packages/onecrawl-rust/crates/onecrawl-cdp/src/dom_observer.rs`

**Structures:**
- `DomMutation` struct
  - `mutation_type` (string)
  - `target` (element tag/id)
  - `added_nodes`, `removed_nodes` (vectors)
  - `attribute_name`, `old_value`, `new_value` (optional)
  - `timestamp` (f64)

**Functions:**
- `start_dom_observer(page, target_selector)` → `Result<()>`
  - Injects MutationObserver
  - Observes childList, attributes, characterData
  - Watches for subtree changes
  - Stores in `window.__onecrawl_dom_mutations`

- `drain_dom_mutations(page)` → `Result<Vec<DomMutation>>`
  - Retrieves and clears buffer
  
- `stop_dom_observer(page)` → `Result<()>`
  - Disconnects observer

- `get_dom_snapshot(page, selector)` → `Result<String>`
  - Returns HTML snapshot

**Storage:** `window.__onecrawl_dom_mutations` array

---

### 3. **page_watcher.rs** (221 lines)
- **Location:** `/Users/giulioleone/Sviluppo/onecrawl-dev/packages/onecrawl-rust/crates/onecrawl-cdp/src/page_watcher.rs`

**Structures:**
- `PageChange` struct
  - `change_type` (string: "navigation", "url", "title", "scroll", "resize")
  - `old_value`, `new_value` (strings)
  - `timestamp` (f64)

**Functions:**
- `start_page_watcher(page)` → `Result<()>`
  - **Navigation:** Intercepts `history.pushState()`, `replaceState()`, `popstate`
  - **Title:** MutationObserver on `<title>`
  - **Scroll:** Throttled 150ms, captures x,y
  - **Resize:** Throttled 150ms, captures width×height
  - Storage: `window.__onecrawl_page_changes`

- `drain_page_changes(page)` → `Result<Vec<PageChange>>`
  - Retrieves and clears buffer

- `stop_page_watcher(page)` → `Result<()>`
  - Restores history methods, disconnects title observer

- `get_page_state(page)` → `Result<serde_json::Value>`
  - Returns complete state snapshot:
    - URL, title, readyState
    - Scroll position & document dimensions
    - Element counts (images, links, forms)
    - Performance timing

**Storage:** `window.__onecrawl_page_changes` array

---

### 4. **websocket.rs** (173 lines)
- **Location:** `/Users/giulioleone/Sviluppo/onecrawl-dev/packages/onecrawl-rust/crates/onecrawl-cdp/src/websocket.rs`

**Structures:**
- `WsDirection` enum (Sent | Received)
- `WsFrame` struct
  - `url` (string)
  - `direction` (enum)
  - `opcode` (u32: 1=text, 2=binary)
  - `payload` (string)
  - `timestamp` (f64)

- `WsRecorder` struct (thread-safe, Arc<Mutex>)
  - `frames()` — get all captured frames
  - `clear()` — reset buffer
  - `len()` — frame count
  - `is_empty()` — boolean check
  - `new()`, `default()` constructors

**Functions:**
- `start_ws_recording(page, recorder)` → `Result<()>`
  - Monkey-patches `window.WebSocket`
  - Captures sent messages
  - Listens for received messages
  - Tracks connections in Map
  - Storage: `window.__onecrawl_ws_frames`

- `drain_ws_frames(page, recorder)` → `Result<usize>`
  - Drains page buffer into recorder
  - Returns count of drained frames

- `active_ws_connections(page)` → `Result<usize>`
  - Returns active connection count

- `export_ws_frames(recorder)` → `Result<serde_json::Value>`
  - JSON export of all frames

**Storage:** `window.__onecrawl_ws_frames` array + `WsRecorder` Arc<Mutex>

---

### 5. **intercept.rs** (163 lines)
- **Location:** `/Users/giulioleone/Sviluppo/onecrawl-dev/packages/onecrawl-rust/crates/onecrawl-cdp/src/intercept.rs`

**Structures:**
- `InterceptRule` struct
  - `url_pattern` (glob string: "*api/v1/*")
  - `resource_type` (optional: "Document", "Script", "Image")
  - `action` (enum)

- `InterceptAction` enum
  - `Block` — reject request
  - `Modify { headers: Option<HashMap> }` — modify headers
  - `MockResponse { status, body, headers }` — fake response

**Functions:**
- `set_intercept_rules(page, rules)` → `Result<()>`
  - Monkey-patches `window.fetch` and `XMLHttpRequest`
  - Saves originals before override
  - Implements pattern matching
  - Logs to `window.__onecrawl_intercepted_log`

- `get_intercepted_requests(page)` → `Result<Vec<serde_json::Value>>`
  - Log entries: `{ url, type, action, ts }`

- `clear_intercept_rules(page)` → `Result<()>`
  - Restores original fetch/XHR

**Storage:** 
- `window.__onecrawl_intercept_rules` (rules array)
- `window.__onecrawl_intercepted_log` (log)
- `window.__onecrawl_orig_fetch`, `__onecrawl_orig_xhr_*` (saved originals)

---

### 6. **screencast.rs** (155 lines)
- **Location:** `/Users/giulioleone/Sviluppo/onecrawl-dev/packages/onecrawl-rust/crates/onecrawl-cdp/src/screencast.rs`

**Structures:**
- `ScreencastOptions` struct
  - `format` (string: "jpeg" or "png")
  - `quality` (optional u32)
  - `max_width`, `max_height` (optional u32)
  - `every_nth_frame` (optional u32)
  - Default: 60% quality, 1280×720, JPEG

- `StreamResult` struct
  - `frames_captured` (usize)
  - `output_dir` (string)
  - `files` (vector of filenames)
  - `duration_ms` (u64)

**Functions:**
- `start_screencast(page, opts)` → `Result<()>`
  - Enables CDP `Page.startScreencast`
  - Frames arrive via CDP events (not polled)

- `stop_screencast(page)` → `Result<()>`
  - Disables screencast

- `capture_frame(page, opts)` → `Result<Vec<u8>>`
  - Single frame using `Page.captureScreenshot`
  - Returns raw bytes (base64-decoded)

- `capture_frames_burst(page, opts, count, interval_ms)` → `Result<Vec<Vec<u8>>>`
  - N frames at intervals
  - Returns vector of byte vectors

- `stream_to_disk(page, opts, output_dir, count, interval_ms)` → `Result<StreamResult>`
  - Saves frames to disk
  - Filenames: `frame_0001.jpg`, `frame_0002.jpg`, etc.
  - Returns metadata

**CDP Integration:** Uses chromiumoxide CDPs:
- `StartScreencastParams`, `StartScreencastFormat`
- `CaptureScreenshotParams`, `CaptureScreenshotFormat`
- `StopScreencastParams`

---

### 7. **agent.rs** (> 600 lines) [See EVENT_REACTOR_REFERENCE.md for full content]
- **Location:** `/Users/giulioleone/Sviluppo/onecrawl-dev/packages/onecrawl-rust/crates/onecrawl-cdp/src/agent.rs`

**Key Functions:**

1. `agent_loop(page, goal, max_steps, verify_js)` → `Result<Value>`
   - Multi-step agent loop with observation
   - Parameter: `verify_js` checks if goal achieved
   - Returns steps with observation & verification results

2. `annotated_observe(page)` → `Result<Value>`
   - Rich page snapshot with element coordinates
   - Returns elements with reference IDs (@e1, @e2, etc.)
   - Includes bounds, role, aria labels

3. `goal_assert(page, assertions)` → `Result<Value>`
   - Verify page state: URL, title, elements, text
   - Assertion types: url_contains, title_equals, element_exists, etc.

4. `session_context(page, command, key, value)` → `Result<Value>`
   - Session storage: set, get, get_all, clear
   - Storage: `window.__onecrawl_ctx`

5. `auto_chain(page, actions, on_error, max_retries)` → `Result<Value>`
   - Execute JS action sequence
   - Error handling: skip, retry, abort
   - Returns results with attempt counts

6. `think(page)` → `Result<Value>`
   - Analyze page and recommend actions
   - Returns CTAs, empty inputs, forms, recommendations

---

### 8. **lib.rs** (187 lines)
- **Location:** `/Users/giulioleone/Sviluppo/onecrawl-dev/packages/onecrawl-rust/crates/onecrawl-cdp/src/lib.rs`

**Module Declarations:** 48 modules (lines 5-95)
- All modules declared as `pub mod`
- Conditional: `#[cfg(feature = "playwright")]` for playwright_backend

**Key Re-exports (lines 97-186):**
```rust
pub use events::{BrowserEvent, EventStream, EventType};
pub use dom_observer::DomMutation;
pub use intercept::{InterceptAction, InterceptRule};
pub use websocket::WsRecorder;
pub use page_watcher::PageChange;
pub use screencast::{ScreencastOptions, StreamResult};
pub use chromiumoxide::Page;
// ... 130+ exports total
```

---

### 9. **Cargo.toml** (34 lines)
- **Location:** `/Users/giulioleone/Sviluppo/onecrawl-dev/packages/onecrawl-rust/crates/onecrawl-cdp/Cargo.toml`

**Dependencies:**
```toml
onecrawl-core          # Local crate
chromiumoxide = "0.5"  # CDP client
tokio                  # Async runtime
serde/serde_json       # Serialization
base64                 # For frame encoding
tracing                # Logging
futures, rand, url, reqwest
```

**Features:**
- `default = []` (none enabled by default)
- `playwright` (optional)

---

## DATA FLOW SUMMARY

### Event Reactor Loop

```
┌──────────────────────────────┐
│   Page Instance (CDP)        │
└──────────────────────────────┘
           ↓
┌──────────────────────────────┐
│   Observers Installed:       │
│  • DOM MutationObserver      │
│  • Page Watcher (history)    │
│  • Console Logger            │
│  • Error Listener            │
│  • WebSocket Recorder        │
└──────────────────────────────┘
           ↓
    ┌─────────────────┐
    │ Page Execution  │
    │  (user actions) │
    └─────────────────┘
           ↓
┌──────────────────────────────┐
│   Periodic Drain (500ms):    │
│  • drain_dom_mutations()     │
│  • drain_page_changes()      │
│  • drain_ws_frames()         │
│  • drain_console()           │
│  • drain_errors()            │
└──────────────────────────────┘
           ↓
┌──────────────────────────────┐
│  Convert to BrowserEvent:    │
│  • event_type: EventType     │
│  • timestamp: f64            │
│  • data: serde_json::Value   │
└──────────────────────────────┘
           ↓
┌──────────────────────────────┐
│  Broadcast to Subscribers:   │
│  • WebSocket clients         │
│  • SSE streams               │
│  • Internal consumers         │
└──────────────────────────────┘
```

---

## INTEGRATION CHECKLIST

- [x] **EventStream** created with broadcast channel
- [x] **Observers installed** (DOM, page, console, errors, WebSocket)
- [x] **Periodic drain tasks** spawned (500ms intervals)
- [x] **Events converted** to BrowserEvent struct
- [x] **Broadcast to subscribers** via SSE or WebSocket
- [x] **Interception rules** (optional) for network control
- [x] **Screencast** (optional) for visual recording
- [x] **Agent loop** (optional) for autonomous actions
- [x] **Cleanup** on page close (stop_* functions)

---

## IMPLEMENTATION GUIDE

See **EVENT_REACTOR_QUICK_REFERENCE.md** for:
1. Complete integration pattern code
2. Function signatures with parameter types
3. Return value structures
4. Storage locations (window.__onecrawl_*)

See **EVENT_REACTOR_REFERENCE.md** for:
1. Full module descriptions
2. Detailed function implementations
3. Callback structures
4. Advanced patterns

---

Generated: 2024
Project: OneCrawl Rust CDP
Base Path: `/Users/giulioleone/Sviluppo/onecrawl-dev/packages/onecrawl-rust/crates/onecrawl-cdp/`
