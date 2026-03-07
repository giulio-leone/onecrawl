# Event Reactor Feature Implementation Reference
## OneCrawl Rust CDP Module Structure

---

## PROJECT STRUCTURE

### Directory: `/Users/giulioleone/Sviluppo/onecrawl-dev/packages/onecrawl-rust/crates/onecrawl-cdp/src/`

**All 48 modules:**
accessibility.rs, adaptive.rs, adaptive_fetch.rs, advanced_emulation.rs, agent.rs, agent_memory.rs, android.rs, annotated.rs, antibot.rs, benchmark.rs, bridge.rs, browser.rs, browser_pool.rs, captcha/, console.rs, cookie.rs, cookie_jar.rs, coverage.rs, data_pipeline/, dialog.rs, dom_nav.rs, dom_observer.rs, domain_blocker.rs, downloads.rs, durable.rs, element.rs, emulation.rs, events.rs, extract.rs, form_filler.rs, geofencing.rs, har.rs, harness.rs, http_client.rs, human.rs, iframe.rs, input.rs, intercept.rs, ios.rs, keyboard.rs, lib.rs, link_graph.rs, navigation.rs, network.rs, network_intel.rs, network_log.rs, page.rs, page_watcher.rs, passkey_store/, perf_monitor.rs, pixel_diff.rs, playwright_backend.rs, print.rs, proxy.rs, proxy_health.rs, rate_limiter.rs, recording.rs, request_queue.rs, retry_queue.rs, robots.rs, safety.rs, scheduler.rs, screencast.rs, screenshot.rs, screenshot_diff.rs, selectors.rs, session_pool.rs, shell.rs, sitemap.rs, skills.rs, smart_actions.rs, snapshot.rs, snapshot_diff.rs, spa.rs, spider.rs, stealth/, streaming.rs, structured_data.rs, tabs.rs, task_planner.rs, throttle.rs, tls_fingerprint.rs, tracing_cdp.rs, vrt.rs, web_storage.rs, webauthn/, websocket.rs, workers.rs, workflow.rs

---

## 1. LIB.RS — MODULE REGISTRATION

**File:** `/Users/giulioleone/Sviluppo/onecrawl-dev/packages/onecrawl-rust/crates/onecrawl-cdp/src/lib.rs`

### Module Declarations (Lines 5-95)

```rust
pub mod accessibility;
pub mod adaptive;
pub mod agent;
pub mod agent_memory;
pub mod android;
pub mod annotated;
pub mod browser_pool;
pub mod computer_use;
pub mod adaptive_fetch;
pub mod advanced_emulation;
pub mod antibot;
pub mod benchmark;
pub mod bridge;
pub mod browser;
pub mod captcha;
pub mod console;
pub mod cookie;
pub mod cookie_jar;
pub mod coverage;
pub mod data_pipeline;
pub mod dialog;
pub mod dom_nav;
pub mod dom_observer;
pub mod durable;
pub mod domain_blocker;
pub mod downloads;
pub mod element;
pub mod emulation;
pub mod events;
pub mod extract;
pub mod form_filler;
pub mod geofencing;
pub mod http_client;
pub mod har;
pub mod harness;
pub mod human;
pub mod iframe;
pub mod input;
pub mod ios;
pub mod intercept;
pub mod keyboard;
pub mod link_graph;
pub mod navigation;
pub mod network;
pub mod network_intel;
pub mod network_log;
pub mod page;
pub mod page_watcher;
pub mod perf_monitor;
#[cfg(feature = "playwright")]
pub mod playwright_backend;
pub mod print;
pub mod proxy;
pub mod recording;
pub mod screencast;
pub mod proxy_health;
pub mod rate_limiter;
pub mod request_queue;
pub mod retry_queue;
pub mod robots;
pub mod scheduler;
pub mod screenshot;
pub mod screenshot_diff;
pub mod snapshot_diff;
pub mod spa;
pub mod selectors;
pub mod session_pool;
pub mod shell;
pub mod skills;
pub mod sitemap;
pub mod smart_actions;
pub mod snapshot;
pub mod spider;
pub mod stealth;
pub mod streaming;
pub mod structured_data;
pub mod tabs;
pub mod task_planner;
pub mod throttle;
pub mod tls_fingerprint;
pub mod tracing_cdp;
pub mod web_storage;
pub mod passkey_store;
pub mod safety;
pub mod webauthn;
pub mod websocket;
pub mod workers;
pub mod vrt;
pub mod pixel_diff;
pub mod workflow;
```

### Re-exports (Lines 97-186)
Key re-exports relevant to Event Reactor:
```rust
pub use browser_pool::{BrowserInstance, BrowserPool, BrowserStatus, SharedPool, new_shared_pool};
pub use events::{BrowserEvent, EventStream, EventType};
pub use dom_observer::DomMutation;
pub use intercept::{InterceptAction, InterceptRule};
pub use websocket::WsRecorder;
pub use page_watcher::PageChange;
pub use screencast::{ScreencastOptions, StreamResult};
pub use recording::{RecordingState, SharedRecording, new_shared_recording, VideoResult};

// ... additional exports ...
pub use chromiumoxide::Page;  // Line 186
```

---

## 2. EVENTS.RS — EVENT STREAMING INFRASTRUCTURE

**File:** `/Users/giulioleone/Sviluppo/onecrawl-dev/packages/onecrawl-rust/crates/onecrawl-cdp/src/events.rs`

### Imports
```rust
use chromiumoxide::Page;
use onecrawl_core::{Error, Result};
use serde::{Deserialize, Serialize};
use tokio::sync::broadcast;
```

### EventType Enum (Lines 14-24)
```rust
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum EventType {
    ConsoleMessage,
    NetworkRequest,
    NetworkResponse,
    PageLoad,
    PageError,
    DomContentLoaded,
    FrameNavigated,
    Dialog,
    Custom(String),
}
```

### BrowserEvent Struct (Lines 28-35)
```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BrowserEvent {
    /// Event type.
    pub event_type: EventType,
    /// Timestamp (ms since epoch).
    pub timestamp: f64,
    /// Event data as JSON.
    pub data: serde_json::Value,
}
```

### EventStream Struct (Lines 38-64)
```rust
pub struct EventStream {
    tx: broadcast::Sender<BrowserEvent>,
    _rx: broadcast::Receiver<BrowserEvent>,
}

impl EventStream {
    /// Create a new event stream with the given capacity.
    pub fn new(capacity: usize) -> Self {
        let (tx, _rx) = broadcast::channel(capacity);
        Self { tx, _rx }
    }

    /// Subscribe to the event stream. Returns a new receiver.
    pub fn subscribe(&self) -> broadcast::Receiver<BrowserEvent> {
        self.tx.subscribe()
    }

    /// Get the sender handle (for injecting events from CDP listeners).
    pub fn sender(&self) -> broadcast::Sender<BrowserEvent> {
        self.tx.clone()
    }

    /// Number of active subscribers.
    pub fn subscriber_count(&self) -> usize {
        self.tx.receiver_count()
    }
}
```

### Key Public Functions

#### Console Observation (Lines 67-112)
```rust
pub async fn observe_console(page: &Page, tx: broadcast::Sender<BrowserEvent>) -> Result<()>
pub async fn drain_console(page: &Page, tx: &broadcast::Sender<BrowserEvent>) -> Result<usize>
```

#### Error Observation (Lines 155-232)
```rust
pub async fn observe_errors(page: &Page, tx: broadcast::Sender<BrowserEvent>) -> Result<()>
pub async fn drain_errors(page: &Page, tx: &broadcast::Sender<BrowserEvent>) -> Result<usize>
```

#### Custom Events (Lines 235-247)
```rust
pub fn emit_custom(
    tx: &broadcast::Sender<BrowserEvent>,
    name: &str,
    data: serde_json::Value,
) -> Result<()>
```

#### SSE Formatting (Lines 250-264)
```rust
pub fn format_sse(event: &BrowserEvent) -> String {
    // Returns SSE-compatible string: "event: {name}\ndata: {json}\n\n"
}
```

#### Internal Helper (Line 266-271)
```rust
fn now_ms() -> f64  // Returns current time in milliseconds since epoch
```

---

## 3. DOM_OBSERVER.RS — DOM MUTATION TRACKING

**File:** `/Users/giulioleone/Sviluppo/onecrawl-dev/packages/onecrawl-rust/crates/onecrawl-cdp/src/dom_observer.rs`

### Imports
```rust
use chromiumoxide::Page;
use onecrawl_core::Result;
use serde::{Deserialize, Serialize};
```

### DomMutation Struct (Lines 12-21)
```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DomMutation {
    pub mutation_type: String,
    pub target: String,
    pub added_nodes: Vec<String>,
    pub removed_nodes: Vec<String>,
    pub attribute_name: Option<String>,
    pub old_value: Option<String>,
    pub new_value: Option<String>,
    pub timestamp: f64,
}
```

### Public Functions (Signatures with Parameters and Return Types)

```rust
pub async fn start_dom_observer(
    page: &Page,
    target_selector: Option<&str>
) -> Result<()>
```
- **Purpose:** Injects MutationObserver into page, observing childList, attributes, characterData
- **Parameters:**
  - `page: &Page` — CDP page instance
  - `target_selector: Option<&str>` — CSS selector or `document.*` expression; defaults to `document.body`
- **Returns:** `Result<()>`
- **Implementation:** Injects JS that monitors:
  - Child node additions/removals
  - Attribute changes with old/new values
  - Character data mutations
  - Stores in `window.__onecrawl_dom_mutations` array

```rust
pub async fn drain_dom_mutations(page: &Page) -> Result<Vec<DomMutation>>
```
- **Purpose:** Retrieves all buffered mutations and clears the buffer
- **Parameters:** `page: &Page`
- **Returns:** `Result<Vec<DomMutation>>` — array of mutation records
- **Implementation:** Evaluates JS to extract and clear `window.__onecrawl_dom_mutations`

```rust
pub async fn stop_dom_observer(page: &Page) -> Result<()>
```
- **Purpose:** Disconnects the MutationObserver and cleans up
- **Parameters:** `page: &Page`
- **Returns:** `Result<()>`
- **Implementation:** Calls `disconnect()` on observer and nullifies reference

```rust
pub async fn get_dom_snapshot(
    page: &Page,
    selector: Option<&str>
) -> Result<String>
```
- **Purpose:** Returns HTML snapshot of DOM at a point in time
- **Parameters:**
  - `page: &Page`
  - `selector: Option<&str>` — CSS selector; if None, returns entire `outerHTML`
- **Returns:** `Result<String>` — HTML string

---

## 4. INTERCEPT.RS — REQUEST INTERCEPTION & MOCKING

**File:** `/Users/giulioleone/Sviluppo/onecrawl-dev/packages/onecrawl-rust/crates/onecrawl-cdp/src/intercept.rs`

### Imports
```rust
use chromiumoxide::Page;
use onecrawl_core::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
```

### InterceptRule Struct (Lines 10-17)
```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InterceptRule {
    /// Glob-style URL pattern, e.g. "*api/v1/*"
    pub url_pattern: String,
    /// Optional resource type filter: "Document", "Script", "Image", etc.
    pub resource_type: Option<String>,
    /// Action to take when a request matches.
    pub action: InterceptAction,
}
```

### InterceptAction Enum (Lines 20-34)
```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum InterceptAction {
    /// Block the request entirely.
    Block,
    /// Modify outgoing headers.
    Modify {
        headers: Option<HashMap<String, String>>,
    },
    /// Return a fake response without hitting the network.
    MockResponse {
        status: u16,
        body: String,
        headers: Option<HashMap<String, String>>,
    },
}
```

### Public Functions

```rust
pub async fn set_intercept_rules(
    page: &Page,
    rules: Vec<InterceptRule>
) -> Result<()>
```
- **Purpose:** Registers interception rules via `window.fetch` and `XMLHttpRequest` monkey-patching
- **Parameters:**
  - `page: &Page` — CDP page instance
  - `rules: Vec<InterceptRule>` — vector of interception rules
- **Returns:** `Result<()>`
- **Implementation:**
  - Stores original `fetch` and XHR methods
  - Overrides `window.fetch` to check rules and block, mock, or modify headers
  - Overrides `XMLHttpRequest.prototype.open` and `.send` similarly
  - Maintains log in `window.__onecrawl_intercepted_log`

```rust
pub async fn get_intercepted_requests(page: &Page) -> Result<Vec<serde_json::Value>>
```
- **Purpose:** Retrieves log of all intercepted requests
- **Parameters:** `page: &Page`
- **Returns:** `Result<Vec<serde_json::Value>>` — array of interception log entries
- **Fields per entry:** `url`, `type` ('fetch'|'xhr'), `action` (enum variant), `ts` (timestamp)

```rust
pub async fn clear_intercept_rules(page: &Page) -> Result<()>
```
- **Purpose:** Restores original `fetch` and XHR, clears all rules
- **Parameters:** `page: &Page`
- **Returns:** `Result<()>`

---

## 5. WEBSOCKET.RS — WEBSOCKET FRAME RECORDING

**File:** `/Users/giulioleone/Sviluppo/onecrawl-dev/packages/onecrawl-rust/crates/onecrawl-cdp/src/websocket.rs`

### Imports
```rust
use chromiumoxide::Page;
use onecrawl_core::{Error, Result};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::Mutex;
```

### WsDirection Enum (Lines 14-17)
```rust
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum WsDirection {
    Sent,
    Received,
}
```

### WsFrame Struct (Lines 20-27)
```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WsFrame {
    pub url: String,
    pub direction: WsDirection,
    pub opcode: u32,
    pub payload: String,
    pub timestamp: f64,
}
```

### WsRecorder Struct (Lines 30-66)
```rust
#[derive(Clone)]
pub struct WsRecorder {
    frames: Arc<Mutex<Vec<WsFrame>>>,
}

impl Default for WsRecorder {
    fn default() -> Self {
        Self::new()
    }
}

impl WsRecorder {
    pub fn new() -> Self {
        Self {
            frames: Arc::new(Mutex::new(Vec::new())),
        }
    }

    /// Get all captured frames.
    pub async fn frames(&self) -> Vec<WsFrame> {
        self.frames.lock().await.clone()
    }

    /// Clear all frames.
    pub async fn clear(&self) {
        self.frames.lock().await.clear();
    }

    /// Number of captured frames.
    pub async fn len(&self) -> usize {
        self.frames.lock().await.len()
    }

    /// Returns true if no frames have been captured.
    pub async fn is_empty(&self) -> bool {
        self.frames.lock().await.is_empty()
    }
}
```

### Public Functions

```rust
pub async fn start_ws_recording(
    page: &Page,
    _recorder: &WsRecorder
) -> Result<()>
```
- **Purpose:** Monkey-patches `window.WebSocket` to capture sent/received frames
- **Parameters:**
  - `page: &Page` — CDP page instance
  - `_recorder: &WsRecorder` — recorder to store frames (currently not used in injection)
- **Returns:** `Result<()>`
- **Implementation:**
  - Captures WebSocket URL and connection tracking
  - Listens to 'message' events → captures as "received"
  - Intercepts `.send()` calls → captures as "sent"
  - Stores frames in `window.__onecrawl_ws_frames` array
  - Tracks opcodes: 1=text, 2=binary

```rust
pub async fn drain_ws_frames(
    page: &Page,
    recorder: &WsRecorder
) -> Result<usize>
```
- **Purpose:** Drains buffered WebSocket frames from page and appends to recorder
- **Parameters:**
  - `page: &Page`
  - `recorder: &WsRecorder` — where to store frames
- **Returns:** `Result<usize>` — count of drained frames
- **Implementation:**
  - Evaluates JS to retrieve `window.__onecrawl_ws_frames`
  - Clears the page-side buffer
  - Extends recorder's internal mutex-protected `Vec<WsFrame>`

```rust
pub async fn active_ws_connections(page: &Page) -> Result<usize>
```
- **Purpose:** Returns count of active WebSocket connections
- **Parameters:** `page: &Page`
- **Returns:** `Result<usize>` — connection count
- **Implementation:** Evaluates `window.__onecrawl_ws_connections.size`

```rust
pub async fn export_ws_frames(recorder: &WsRecorder) -> Result<serde_json::Value>
```
- **Purpose:** Exports all recorded frames as JSON
- **Parameters:** `recorder: &WsRecorder`
- **Returns:** `Result<serde_json::Value>` — JSON array of frames

---

## 6. PAGE_WATCHER.RS — PAGE STATE CHANGE DETECTION

**File:** `/Users/giulioleone/Sviluppo/onecrawl-dev/packages/onecrawl-rust/crates/onecrawl-cdp/src/page_watcher.rs`

### Imports
```rust
use chromiumoxide::Page;
use onecrawl_core::Result;
use serde::{Deserialize, Serialize};
```

### PageChange Struct (Lines 12-17)
```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PageChange {
    pub change_type: String,
    pub old_value: String,
    pub new_value: String,
    pub timestamp: f64,
}
```

### Public Functions

```rust
pub async fn start_page_watcher(page: &Page) -> Result<()>
```
- **Purpose:** Installs watchers for SPA navigation, title, scroll, and resize
- **Parameters:** `page: &Page`
- **Returns:** `Result<()>`
- **Watches:**
  - **Navigation:** Intercepts `history.pushState()`, `history.replaceState()`, and `popstate` events
  - **Title Changes:** MutationObserver on `<title>` element
  - **Scroll:** Throttled scroll listener (150ms debounce), captures x,y coordinates
  - **Resize:** Throttled resize listener (150ms debounce), captures viewport width×height
- **Storage:** Records all changes in `window.__onecrawl_page_changes` array

```rust
pub async fn drain_page_changes(page: &Page) -> Result<Vec<PageChange>>
```
- **Purpose:** Retrieves accumulated page changes and clears buffer
- **Parameters:** `page: &Page`
- **Returns:** `Result<Vec<PageChange>>` — array of state changes
- **Change Types:**
  - `"navigation"` — URL changed via navigation
  - `"url"` — URL changed via replaceState
  - `"title"` — Document title changed
  - `"scroll"` — Scroll position changed
  - `"resize"` — Viewport dimensions changed

```rust
pub async fn stop_page_watcher(page: &Page) -> Result<()>
```
- **Purpose:** Stops watchers and restores original history methods
- **Parameters:** `page: &Page`
- **Returns:** `Result<()>`
- **Cleanup:** Disconnects title MutationObserver, restores `pushState`/`replaceState`

```rust
pub async fn get_page_state(page: &Page) -> Result<serde_json::Value>
```
- **Purpose:** Returns comprehensive page state snapshot
- **Parameters:** `page: &Page`
- **Returns:** `Result<serde_json::Value>` with fields:
  - `url`, `title`, `ready_state`
  - `scroll_x`, `scroll_y`
  - `viewport_width`, `viewport_height`
  - `document_width`, `document_height`
  - `element_count`, `image_count`, `link_count`, `form_count`
  - `performance_timing` → `dom_content_loaded`, `load_event`, `dom_interactive`

---

## 7. SCREENCAST.RS — CDP SCREENCAST FRAMES

**File:** `/Users/giulioleone/Sviluppo/onecrawl-dev/packages/onecrawl-rust/crates/onecrawl-cdp/src/screencast.rs`

### Imports
```rust
use chromiumoxide::cdp::browser_protocol::page::{
    CaptureScreenshotFormat, CaptureScreenshotParams, StartScreencastFormat,
    StartScreencastParams, StopScreencastParams,
};
use chromiumoxide::Page;
use onecrawl_core::{Error, Result};
use serde::{Deserialize, Serialize};
```

### ScreencastOptions Struct (Lines 12-30)
```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScreencastOptions {
    pub format: String,  // "jpeg" or "png"
    pub quality: Option<u32>,
    pub max_width: Option<u32>,
    pub max_height: Option<u32>,
    pub every_nth_frame: Option<u32>,
}

impl Default for ScreencastOptions {
    fn default() -> Self {
        Self {
            format: "jpeg".to_string(),
            quality: Some(60),
            max_width: Some(1280),
            max_height: Some(720),
            every_nth_frame: Some(1),
        }
    }
}
```

### StreamResult Struct (Lines 148-154)
```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StreamResult {
    pub frames_captured: usize,
    pub output_dir: String,
    pub files: Vec<String>,
    pub duration_ms: u64,
}
```

### Public Functions

```rust
pub async fn start_screencast(
    page: &Page,
    opts: &ScreencastOptions
) -> Result<()>
```
- **Purpose:** Enables CDP `Page.startScreencast` for real-time frame streaming
- **Parameters:**
  - `page: &Page`
  - `opts: &ScreencastOptions` — format, quality, max dimensions, frame interval
- **Returns:** `Result<()>`
- **Note:** Frames arrive asynchronously via CDP `Page.screencastFrame` events

```rust
pub async fn stop_screencast(page: &Page) -> Result<()>
```
- **Purpose:** Disables screencast
- **Parameters:** `page: &Page`
- **Returns:** `Result<()>`

```rust
pub async fn capture_frame(
    page: &Page,
    opts: &ScreencastOptions
) -> Result<Vec<u8>>
```
- **Purpose:** Captures single frame using CDP `Page.captureScreenshot`
- **Parameters:**
  - `page: &Page`
  - `opts: &ScreencastOptions`
- **Returns:** `Result<Vec<u8>>` — raw image bytes (base64-decoded)

```rust
pub async fn capture_frames_burst(
    page: &Page,
    opts: &ScreencastOptions,
    count: usize,
    interval_ms: u64
) -> Result<Vec<Vec<u8>>>
```
- **Purpose:** Captures N frames at regular intervals
- **Parameters:**
  - `page: &Page`
  - `opts: &ScreencastOptions`
  - `count: usize` — number of frames
  - `interval_ms: u64` — sleep between frames (0 = no sleep)
- **Returns:** `Result<Vec<Vec<u8>>>` — array of frame byte vectors

```rust
pub async fn stream_to_disk(
    page: &Page,
    opts: &ScreencastOptions,
    output_dir: &str,
    count: usize,
    interval_ms: u64
) -> Result<StreamResult>
```
- **Purpose:** Captures frames and writes to disk with metadata
- **Parameters:**
  - `page: &Page`
  - `opts: &ScreencastOptions`
  - `output_dir: &str` — filesystem path
  - `count: usize`
  - `interval_ms: u64`
- **Returns:** `Result<StreamResult>` with:
  - `frames_captured: usize`
  - `output_dir: String`
  - `files: Vec<String>` — filenames (frame_0001.jpg, etc.)
  - `duration_ms: u64` — total capture time
- **File Naming:** `frame_{:04}.{ext}` where ext is "png" or "jpg"

---

## 8. AGENT.RS — AUTONOMOUS AGENT LOOP

**File:** `/Users/giulioleone/Sviluppo/onecrawl-dev/packages/onecrawl-rust/crates/onecrawl-cdp/src/agent.rs`

### Imports
```rust
use chromiumoxide::Page;
use onecrawl_core::{Error, Result};
use serde_json::Value;
use crate::page::evaluate_js;
```

### Public Functions (Signatures)

```rust
pub async fn agent_loop(
    page: &Page,
    goal: &str,
    max_steps: usize,
    verify_js: Option<&str>,
) -> Result<Value>
```
- **Purpose:** Execute multi-step agent loop with observation and verification
- **Parameters:**
  - `page: &Page` — CDP page
  - `goal: &str` — goal description
  - `max_steps: usize` — max iterations
  - `verify_js: Option<&str>` — JS predicate to check if goal achieved (returns bool)
- **Returns:** `Result<Value>` with structure:
  ```json
  {
    "status": "goal_achieved" | "max_steps_reached",
    "total_steps": usize,
    "goal": str,
    "steps": [
      {
        "step": usize,
        "url": str,
        "title": str,
        "observation": { "total_interactive": usize, "visible_interactive": usize, "forms": usize, "body_text_length": usize },
        "goal": str,
        "verified": bool,
        "verify_result": Value
      }
    ]
  }
  ```

```rust
pub async fn goal_assert(
    page: &Page,
    assertions: &[(&str, &str)],
) -> Result<Value>
```
- **Purpose:** Semantic verification of page goals
- **Parameters:**
  - `page: &Page`
  - `assertions: &[(&str, &str)]` — array of (type, value) tuples
- **Assertion Types:**
  - `"url_contains"`, `"url_equals"`
  - `"title_contains"`, `"title_equals"`
  - `"element_exists"` — CSS selector
  - `"text_contains"` — body text
  - `"element_visible"` — CSS selector visible in viewport
- **Returns:** `Result<Value>` with:
  ```json
  {
    "all_passed": bool,
    "assertions": [{"type": str, "value": str, "passed": bool}],
    "context": {"url": str, "title": str}
  }
  ```

```rust
pub async fn annotated_observe(page: &Page) -> Result<Value>
```
- **Purpose:** Get page state with element coordinates and reference IDs
- **Parameters:** `page: &Page`
- **Returns:** `Result<Value>` with:
  ```json
  {
    "url": str,
    "title": str,
    "viewport": {"width": u32, "height": u32},
    "scroll": {"x": f64, "y": f64, "max_y": f64},
    "elements": [
      {
        "ref": "@e1", "@e2", etc.,
        "tag": str,
        "role": str,
        "text": str,
        "aria_label": str,
        "placeholder": str,
        "type": str,
        "href": str,
        "name": str,
        "id": str,
        "value": str,
        "disabled": bool,
        "checked": bool,
        "bounds": {"x": i32, "y": i32, "width": i32, "height": i32, "center_x": i32, "center_y": i32}
      }
    ],
    "element_count": usize,
    "timestamp": f64
  }
  ```

```rust
pub async fn session_context(
    page: &Page,
    command: &str,
    key: Option<&str>,
    value: Option<&str>,
) -> Result<Value>
```
- **Purpose:** Store/retrieve session-scoped context in page
- **Parameters:**
  - `page: &Page`
  - `command: &str` — `"set"`, `"get"`, `"get_all"`, `"clear"`
  - `key: Option<&str>` — context key (for set/get)
  - `value: Option<&str>` — value to set (for set)
- **Returns:** `Result<Value>` — result object with action, key, value/context
- **Storage:** `window.__onecrawl_ctx` object

```rust
pub async fn auto_chain(
    page: &Page,
    actions: &[String],
    on_error: &str,
    max_retries: usize,
) -> Result<Value>
```
- **Purpose:** Execute chain of JS actions with error recovery
- **Parameters:**
  - `page: &Page`
  - `actions: &[String]` — JS code snippets to execute sequentially
  - `on_error: &str` — `"skip"`, `"retry"`, or `"abort"`
  - `max_retries: usize` — retry count per action
- **Returns:** `Result<Value>` with:
  ```json
  {
    "status": "all_success" | "partial",
    "completed_steps": usize,
    "total_steps": usize,
    "results": [
      {
        "step": usize,
        "status": "success" | "skipped" | "aborted" | "failed",
        "result": str,
        "error": str,
        "attempts": usize
      }
    ]
  }
  ```

```rust
pub async fn think(page: &Page) -> Result<Value>
```
- **Purpose:** Structured reasoning: analyze page state and recommend actions
- **Parameters:** `page: &Page`
- **Returns:** `Result<Value>` with analysis:
  - `state` — url, title, readyState, scroll, viewport
  - `ctas` — visible buttons/CTAs with text (up to 10)
  - `empty_inputs` — required but empty form inputs
  - `forms` — form count
  - `recommendations` — suggested actions as strings

```rust
pub async fn click_at_coords(
    page: &Page,
    x: f64,
    y: f64,
) -> Result<Value>
```
- **Purpose:** Click at absolute page coordinates
- **Parameters:**
  - `page: &Page`
  - `x: f64`, `y: f64` — coordinates
- **Returns:** `Result<Value>` — click result

```rust
pub async fn input_replay(
    page: &Page,
    /* ... params ... */
) -> Result<Value>
```
- **Purpose:** Replay input sequence (clicks, typing, etc.)
- **Parameters:** (see source for full signature)
- **Returns:** `Result<Value>` — replay result

---

## 9. CARGO.TOML — DEPENDENCIES

**File:** `/Users/giulioleone/Sviluppo/onecrawl-dev/packages/onecrawl-rust/crates/onecrawl-cdp/Cargo.toml`

```toml
[package]
name = "onecrawl-cdp"
description = "Chrome DevTools Protocol browser automation for OneCrawl"
version.workspace = true
edition.workspace = true
license.workspace = true
repository.workspace = true
authors.workspace = true

[lints.rust]
unexpected_cfgs = { level = "allow", check-cfg = ['cfg(feature, values("zip"))'] }

[features]
default = []
playwright = ["dep:playwright"]

[dependencies]
onecrawl-core = { path = "../onecrawl-core" }
chromiumoxide = { workspace = true }
tokio = { workspace = true }
futures = { workspace = true }
serde = { workspace = true }
serde_json = { workspace = true }
base64 = { workspace = true }
tracing = { workspace = true }
rand = { workspace = true }
url = { workspace = true }
reqwest = { workspace = true }
playwright = { workspace = true, optional = true }

[dev-dependencies]
pretty_assertions = { workspace = true }
tempfile = { workspace = true }
```

### Workspace Dependencies (from root Cargo.toml)
```toml
[workspace.dependencies]
# Core async/serialization
tokio = { version = "1", features = ["full"] }
serde = { version = "1", features = ["derive"] }
serde_json = "1"
futures = "0.3"
tracing = "0.1"

# Common utilities
base64 = "0.22"
rand = "0.8"
url = "2.5"
reqwest = "0.11"
chromiumoxide = "0.5"

# Optional
playwright = "0.20"

# Dev
pretty_assertions = "1.4"
tempfile = "3"
```

---

## EVENT REACTOR INTEGRATION PATTERNS

Based on the codebase structure, here's how to implement an Event Reactor:

### 1. **Core Event Loop Integration**
- Use `EventStream` with `broadcast::Sender<BrowserEvent>`
- Multiple subscribers can listen to same channel
- Emit events from:
  - Console observer (`observe_console` → `drain_console`)
  - Error observer (`observe_errors` → `drain_errors`)
  - DOM observer (`start_dom_observer` → `drain_dom_mutations`)
  - Page watcher (`start_page_watcher` → `drain_page_changes`)
  - WebSocket recorder (`start_ws_recording` → `drain_ws_frames`)

### 2. **Common Pattern: Install → Drain → Emit**
```rust
// 1. Install observer on page
start_dom_observer(&page, Some("body")).await?;

// 2. Perform actions...
page.navigate(...).await?;

// 3. Drain mutations periodically
let mutations = drain_dom_mutations(&page).await?;

// 4. Convert to BrowserEvent and emit
for mutation in mutations {
    event_stream.sender().send(BrowserEvent {
        event_type: EventType::Custom("dom_mutation".to_string()),
        timestamp: mutation.timestamp,
        data: serde_json::to_value(&mutation)?,
    })?;
}
```

### 3. **Broadcasting Architecture**
- `EventStream::new(capacity)` creates broadcast channel
- `.subscribe()` creates new receiver for WebSocket/SSE clients
- `.sender()` gets handle for injecting events from listeners
- `.subscriber_count()` checks active listeners

### 4. **Interception Layer**
- Use `set_intercept_rules()` with `InterceptRule` vector
- Supports URL glob patterns, resource type filters
- Actions: Block, Modify headers, MockResponse
- Log queries via `get_intercepted_requests()`

### 5. **Timing and Debouncing**
- Page watcher uses 150ms debounce for scroll/resize
- DOM observer captures all changes in real-time
- Consider polling intervals for draining (e.g., every 500ms)

---

## KEY TYPES SUMMARY

| Type | Module | Purpose |
|------|--------|---------|
| `EventStream` | events | Broadcast channel for browser events |
| `BrowserEvent` | events | Individual event with type, timestamp, data |
| `EventType` | events | Enum: ConsoleMessage, NetworkRequest, PageLoad, etc. |
| `DomMutation` | dom_observer | Captured DOM change record |
| `PageChange` | page_watcher | Navigation/title/scroll/resize change |
| `InterceptRule` | intercept | Request interception matcher + action |
| `InterceptAction` | intercept | Block, Modify, or MockResponse |
| `WsFrame` | websocket | WebSocket message record |
| `WsRecorder` | websocket | Collects WsFrame in Arc<Mutex> |
| `ScreencastOptions` | screencast | Frame format, quality, dimensions |
| `StreamResult` | screencast | Metadata about saved frames |

---

## INTEGRATION CHECKLIST

To implement Event Reactor:
- [ ] Create EventStream instance
- [ ] Subscribe to events via `.subscribe()`
- [ ] Install observers (console, errors, dom, page, ws)
- [ ] Create periodic drain task (tokio::spawn)
- [ ] Emit BrowserEvents from drained data
- [ ] Setup interception rules (optional)
- [ ] Setup screencast stream (optional)
- [ ] Forward events to client via WebSocket/SSE
- [ ] Cleanup: call stop_* functions on page close
- [ ] Aggregate events with agent_loop/annotated_observe

