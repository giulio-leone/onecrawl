# Event Reactor Quick Reference

## Core Types

### `EventStream` (events.rs:38-64)
```rust
pub struct EventStream {
    tx: broadcast::Sender<BrowserEvent>,
    _rx: broadcast::Receiver<BrowserEvent>,
}

// Methods:
pub fn new(capacity: usize) -> Self
pub fn subscribe(&self) -> broadcast::Receiver<BrowserEvent>
pub fn sender(&self) -> broadcast::Sender<BrowserEvent>
pub fn subscriber_count(&self) -> usize
```

### `BrowserEvent` (events.rs:28-35)
```rust
pub struct BrowserEvent {
    pub event_type: EventType,      // ConsoleMessage, NetworkRequest, etc.
    pub timestamp: f64,             // ms since epoch
    pub data: serde_json::Value,    // event payload
}
```

### `EventType` (events.rs:14-24)
```rust
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

---

## Observer Install → Drain → Emit Pattern

### DOM Mutations
```rust
// 1. Install observer
pub async fn start_dom_observer(page: &Page, target_selector: Option<&str>) -> Result<()>

// 2. Drain mutations
pub async fn drain_dom_mutations(page: &Page) -> Result<Vec<DomMutation>>

// 3. Stop observer
pub async fn stop_dom_observer(page: &Page) -> Result<()>
```

**DomMutation fields:**
- `mutation_type: String` — "childList", "attributes", "characterData"
- `target: String` — element tag/id
- `added_nodes: Vec<String>`, `removed_nodes: Vec<String>`
- `attribute_name: Option<String>`, `old_value: Option<String>`, `new_value: Option<String>`
- `timestamp: f64`

### Page Changes (SPA Navigation, Title, Scroll, Resize)
```rust
// 1. Install watcher
pub async fn start_page_watcher(page: &Page) -> Result<()>

// 2. Drain changes
pub async fn drain_page_changes(page: &Page) -> Result<Vec<PageChange>>

// 3. Get state snapshot
pub async fn get_page_state(page: &Page) -> Result<serde_json::Value>

// 4. Stop watcher
pub async fn stop_page_watcher(page: &Page) -> Result<()>
```

**PageChange fields:**
- `change_type: String` — "navigation", "url", "title", "scroll", "resize"
- `old_value: String`, `new_value: String`
- `timestamp: f64`

### Console & Page Errors
```rust
// Install
pub async fn observe_console(page: &Page, tx: broadcast::Sender<BrowserEvent>) -> Result<()>
pub async fn observe_errors(page: &Page, tx: broadcast::Sender<BrowserEvent>) -> Result<()>

// Drain
pub async fn drain_console(page: &Page, tx: &broadcast::Sender<BrowserEvent>) -> Result<usize>
pub async fn drain_errors(page: &Page, tx: &broadcast::Sender<BrowserEvent>) -> Result<usize>
```

### WebSocket Frames
```rust
// 1. Create recorder
let recorder = WsRecorder::new();

// 2. Install
pub async fn start_ws_recording(page: &Page, recorder: &WsRecorder) -> Result<()>

// 3. Drain frames
pub async fn drain_ws_frames(page: &Page, recorder: &WsRecorder) -> Result<usize>

// 4. Export
pub async fn export_ws_frames(recorder: &WsRecorder) -> Result<serde_json::Value>
```

**WsFrame fields:**
- `url: String`
- `direction: WsDirection` — Sent | Received
- `opcode: u32` — 1=text, 2=binary
- `payload: String`
- `timestamp: f64`

---

## Interception & Mocking

### Set Rules
```rust
pub async fn set_intercept_rules(page: &Page, rules: Vec<InterceptRule>) -> Result<()>

pub struct InterceptRule {
    pub url_pattern: String,           // glob: "*api/v1/*"
    pub resource_type: Option<String>, // "Document", "Script", "Image"
    pub action: InterceptAction,
}

pub enum InterceptAction {
    Block,
    Modify {
        headers: Option<HashMap<String, String>>,
    },
    MockResponse {
        status: u16,
        body: String,
        headers: Option<HashMap<String, String>>,
    },
}
```

### Query & Clear
```rust
pub async fn get_intercepted_requests(page: &Page) -> Result<Vec<serde_json::Value>>
pub async fn clear_intercept_rules(page: &Page) -> Result<()>
```

---

## Screencast (Live Frame Streaming)

```rust
pub struct ScreencastOptions {
    pub format: String,              // "jpeg" or "png"
    pub quality: Option<u32>,        // e.g., 60
    pub max_width: Option<u32>,      // e.g., 1280
    pub max_height: Option<u32>,     // e.g., 720
    pub every_nth_frame: Option<u32>,
}

// Start real-time streaming (frames arrive via CDP events)
pub async fn start_screencast(page: &Page, opts: &ScreencastOptions) -> Result<()>

// Stop streaming
pub async fn stop_screencast(page: &Page) -> Result<()>

// Capture single frame
pub async fn capture_frame(page: &Page, opts: &ScreencastOptions) -> Result<Vec<u8>>

// Capture N frames with interval
pub async fn capture_frames_burst(
    page: &Page,
    opts: &ScreencastOptions,
    count: usize,
    interval_ms: u64,
) -> Result<Vec<Vec<u8>>>

// Save to disk
pub async fn stream_to_disk(
    page: &Page,
    opts: &ScreencastOptions,
    output_dir: &str,
    count: usize,
    interval_ms: u64,
) -> Result<StreamResult>

pub struct StreamResult {
    pub frames_captured: usize,
    pub output_dir: String,
    pub files: Vec<String>,      // frame_0001.jpg, etc.
    pub duration_ms: u64,
}
```

---

## Agent Loop & Observation

### Core Agent Loop
```rust
pub async fn agent_loop(
    page: &Page,
    goal: &str,
    max_steps: usize,
    verify_js: Option<&str>,
) -> Result<Value>
// Returns: { status, total_steps, goal, steps: [{ step, url, title, observation, verified }] }
```

### Rich Page Observation
```rust
pub async fn annotated_observe(page: &Page) -> Result<Value>
// Returns: { url, title, viewport, scroll, elements: [{ ref, tag, role, text, bounds }], element_count }

pub async fn think(page: &Page) -> Result<Value>
// Returns: { state, ctas, empty_inputs, forms, recommendations }
```

### Assertions
```rust
pub async fn goal_assert(
    page: &Page,
    assertions: &[(&str, &str)],
) -> Result<Value>
// Types: "url_contains", "url_equals", "title_contains", "element_exists", "text_contains", "element_visible"
```

### Session Context
```rust
pub async fn session_context(
    page: &Page,
    command: &str,  // "set", "get", "get_all", "clear"
    key: Option<&str>,
    value: Option<&str>,
) -> Result<Value>
// Storage: window.__onecrawl_ctx object
```

### Action Chaining
```rust
pub async fn auto_chain(
    page: &Page,
    actions: &[String],
    on_error: &str,  // "skip", "retry", "abort"
    max_retries: usize,
) -> Result<Value>
// Returns: { status, completed_steps, total_steps, results: [{step, status, result|error, attempts}] }
```

---

## Essential Integration Pattern

```rust
// 1. Create event stream
let event_stream = EventStream::new(128);
let tx = event_stream.sender();

// 2. Install observers
start_dom_observer(&page, Some("body")).await?;
start_page_watcher(&page).await?;
start_ws_recording(&page, &recorder).await?;
observe_console(&page, tx.clone()).await?;
observe_errors(&page, tx.clone()).await?;

// 3. Periodic drain task (e.g., every 500ms)
tokio::spawn({
    let page_clone = page.clone();
    let tx_clone = tx.clone();
    let recorder_clone = recorder.clone();
    async move {
        loop {
            tokio::time::sleep(Duration::from_millis(500)).await;
            
            // Drain all sources
            if let Ok(mutations) = drain_dom_mutations(&page_clone).await {
                for m in mutations {
                    let _ = tx_clone.send(BrowserEvent {
                        event_type: EventType::Custom("dom_mutation".to_string()),
                        timestamp: m.timestamp,
                        data: serde_json::to_value(&m).unwrap_or(Value::Null),
                    });
                }
            }
            
            if let Ok(changes) = drain_page_changes(&page_clone).await {
                for c in changes {
                    let _ = tx_clone.send(BrowserEvent {
                        event_type: EventType::Custom(format!("page_{}", c.change_type)),
                        timestamp: c.timestamp,
                        data: serde_json::to_value(&c).unwrap_or(Value::Null),
                    });
                }
            }
            
            if let Ok(_) = drain_ws_frames(&page_clone, &recorder_clone).await {
                // Frames stored in recorder
            }
        }
    }
});

// 4. Subscribe to events (for WebSocket/SSE output)
let mut rx = event_stream.subscribe();
tokio::spawn(async move {
    while let Ok(event) = rx.recv().await {
        // Send to client via WebSocket/SSE
        println!("{}", format_sse(&event));
    }
});

// 5. Clean up on page close
stop_dom_observer(&page).await?;
stop_page_watcher(&page).await?;
clear_intercept_rules(&page).await?;
```

---

## File Locations

| File | Path | Purpose |
|------|------|---------|
| lib.rs | `src/lib.rs` | Module registry & re-exports |
| events.rs | `src/events.rs` | EventStream, BrowserEvent, event draining |
| dom_observer.rs | `src/dom_observer.rs` | DOM mutation tracking |
| page_watcher.rs | `src/page_watcher.rs` | SPA navigation, title, scroll, resize |
| websocket.rs | `src/websocket.rs` | WebSocket frame recording |
| intercept.rs | `src/intercept.rs` | Request interception & mocking |
| screencast.rs | `src/screencast.rs` | CDP screenshot/screencast |
| agent.rs | `src/agent.rs` | Autonomous agent loop |
| Cargo.toml | `Cargo.toml` | Dependencies |

---

## Key Dependencies

```toml
chromiumoxide = "0.5"      # CDP client
tokio = { features = ["full"] }
serde_json = "1"
broadcast channel: tokio::sync::broadcast
```

