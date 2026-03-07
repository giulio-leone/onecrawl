# Multi-Device Orchestration Infrastructure Report
## OneCrawl Rust Codebase Analysis

---

## 1. ANDROID CLIENT (UIAutomator2 Protocol)

**File:** `/Users/giulioleone/Sviluppo/onecrawl-dev/packages/onecrawl-rust/crates/onecrawl-cdp/src/android.rs`  
**Lines:** 628

### Struct Definition
```rust
pub struct AndroidClient {
    config: AndroidSessionConfig,
    session_id: Option<String>,
    client: reqwest::Client,
}

pub struct AndroidSessionConfig {
    pub server_url: String,        // Default: http://localhost:4723
    pub device_serial: Option<String>,
    pub package: String,           // Default: com.android.chrome
    pub activity: Option<String>,
}
```

### Instantiation
```rust
impl AndroidClient {
    pub fn new(config: AndroidSessionConfig) -> Self {
        Self {
            config,
            session_id: None,
            client: reqwest::Client::new(),
        }
    }
}
```

### Key Methods (23 public async methods)
- **Session Management:**
  - `create_session(package, activity) -> Result<String>` — Create UIAutomator2 session
  - `close_session() -> Result<()>` — Destroy session

- **Navigation:**
  - `navigate(url: &str) -> Result<()>` — Navigate Chrome to URL
  - `get_url() -> Result<String>` — Get current URL
  - `get_title() -> Result<String>` — Get page title
  - `back() -> Result<()>` — Press back button

- **Touch Gestures:**
  - `tap(x, y) -> Result<()>` — Single tap
  - `swipe(from_x, from_y, to_x, to_y, duration_ms) -> Result<()>` — Swipe gesture
  - `long_press(x, y, duration_ms) -> Result<()>` — Long press
  - `double_tap(x, y) -> Result<()>` — Double tap
  - `pinch(x, y, scale) -> Result<()>` — Pinch zoom

- **Element Interaction:**
  - `find_element(strategy: &str, value: &str) -> Result<String>` — Locator strategies
  - `click_element(element_id) -> Result<()>` — Click element
  - `element_text(element_id) -> Result<String>` — Get element text
  - `type_text(text) -> Result<()>` — Type into focused element

- **Device Control:**
  - `screenshot() -> Result<String>` — Base64 PNG
  - `set_orientation(orientation: &str) -> Result<()>` — Portrait/Landscape
  - `get_orientation() -> Result<String>`
  - `press_key(keycode: i32) -> Result<()>` — ADB keycodes

### Transport
- HTTP client via `reqwest::Client`
- UIAutomator2 server protocol (Appium-compatible)
- JSON request/response bodies

---

## 2. iOS CLIENT (WebDriverAgent Protocol)

**File:** `/Users/giulioleone/Sviluppo/onecrawl-dev/packages/onecrawl-rust/crates/onecrawl-cdp/src/ios.rs`  
**Lines:** 673

### Struct Definition
```rust
pub struct IosClient {
    config: IosSessionConfig,
    session_id: Option<String>,
    client: reqwest::Client,
}

pub struct IosSessionConfig {
    pub wda_url: String,           // Default: http://localhost:8100
    pub device_udid: Option<String>,
    pub bundle_id: String,         // Default: com.apple.mobilesafari
}

pub struct IosDevice {
    pub udid: String,
    pub name: String,
    pub platform: String,          // "iOS" or "tvOS"
    pub version: String,
    pub is_simulator: bool,
}
```

### Instantiation
```rust
impl IosClient {
    pub fn new(config: IosSessionConfig) -> Self {
        Self {
            config,
            session_id: None,
            client: reqwest::Client::new(),
        }
    }
}
```

### Key Methods (20+ public async methods)
- **Session Management:**
  - `create_session() -> Result<String>` — Create WDA session
  - `close_session() -> Result<()>` — Destroy session

- **Navigation:**
  - `navigate(url: &str) -> Result<()>` — Navigate Safari
  - `get_url() -> Result<String>`
  - `get_title() -> Result<String>`

- **Touch Gestures:**
  - `tap(x, y) -> Result<()>`
  - `swipe(from_x, from_y, to_x, to_y, duration) -> Result<()>`
  - `pinch(x, y, scale, velocity) -> Result<()>`
  - `long_press(x, y, duration_ms) -> Result<()>`
  - `double_tap(x, y) -> Result<()>`

- **Element Interaction:**
  - `find_element(using: &str, value: &str) -> Result<String>` — Locator strategies
  - `click_element(element_id) -> Result<()>`
  - `type_text(element_id, text) -> Result<()>`
  - `scroll_to_element(using, value) -> Result<()>`

- **Device Control:**
  - `screenshot() -> Result<Vec<u8>>` — Raw PNG bytes
  - `page_source() -> Result<String>` — Accessibility tree XML
  - `set_orientation(orientation) -> Result<()>`
  - `get_orientation() -> Result<String>`
  - `get_screen_size() -> Result<Value>`

### Transport
- HTTP client via `reqwest::Client`
- WebDriverAgent protocol (W3C WebDriver compatible)
- JSON request/response bodies
- Base64 encoding for binary data (screenshots)

---

## 3. BROWSER/TAB MANAGEMENT

### Browser Session (lib wrapper)
**File:** `crates/onecrawl-cdp/src/browser.rs` (lines 1-100)

```rust
pub struct BrowserSession {
    browser: Browser,
    _handler_task: tokio::task::JoinHandle<()>,
}

impl BrowserSession {
    pub async fn launch_headless() -> Result<Self>
    pub async fn launch_headed() -> Result<Self>
    pub async fn connect(ws_url: &str) -> Result<Self>
    pub async fn connect_with_nav_timeout(ws_url: &str) -> Result<Self>
}
```

### Tab Management (Server-level)
**File:** `crates/onecrawl-server/src/instance.rs` (lines 34-45)

```rust
pub struct Instance {
    pub id: String,
    pub browser: Browser,
    pub _handler_task: tokio::task::JoinHandle<()>,
    pub profile: Option<String>,
    pub headless: bool,
    pub port: u16,
    pub start_time: String,
    pub tabs: RwLock<HashMap<String, Page>>,  // tab_id -> Page
    pub tab_counter: RwLock<u32>,
}
```

**File:** `crates/onecrawl-cdp/src/tabs.rs` (lines 1-80)

```rust
pub struct TabInfo {
    pub index: usize,
    pub url: String,
    pub title: String,
    pub target_id: String,
}
```

### Browser Pool (Multi-instance pooling)
**File:** `crates/onecrawl-cdp/src/browser_pool.rs` (lines 1-100)

```rust
pub struct BrowserPool {
    instances: HashMap<String, PoolEntry>,  // id -> PoolEntry
    max_size: usize,
}

pub enum BrowserStatus {
    Idle,
    Busy,
    Error,
    Closed,
}

pub struct BrowserInstance {
    pub id: String,
    pub status: BrowserStatus,
    pub url: Option<String>,
    pub created_at: u64,
}

impl BrowserPool {
    pub fn new(max_size: usize) -> Self
    pub fn add(&mut self, id: String, page: Page) -> Result<()>
    pub fn get(&self, id: &str) -> Option<&Page>
    pub fn remove(&mut self, id: &str) -> Option<Page>
    pub fn list(&self) -> Vec<BrowserInstance>
    pub fn get_idle(&self) -> Option<(&str, &Page)>
}
```

---

## 4. WORKFLOW ENGINE (DSL)

**File:** `crates/onecrawl-cdp/src/workflow.rs` (907 lines)

### Workflow Definition
```rust
pub struct Workflow {
    pub name: String,
    pub description: String,
    pub version: String,
    pub variables: HashMap<String, Value>,
    pub steps: Vec<Step>,
    pub on_error: ErrorHandler,
}

pub struct Step {
    pub id: String,
    pub name: String,
    pub action: Action,
    pub condition: Option<String>,          // Conditional execution
    pub retries: u32,                       // Retry logic
    pub retry_delay_ms: u64,
    pub timeout_ms: Option<u64>,
    pub on_error: Option<StepErrorAction>,
    pub save_as: Option<String>,            // Variable binding
}
```

### Action Types (Enum dispatch pattern)
```rust
pub enum Action {
    Navigate { url: String },
    Click { selector: String },
    Type { selector: String, text: String },
    WaitForSelector { selector: String, timeout_ms: u64 },
    Screenshot { path: Option<String>, full_page: Option<bool> },
    Evaluate { js: String },
    Extract { selector: String, attribute: Option<String> },
    SmartClick { query: String },
    SmartFill { query: String, value: String },
    Sleep { ms: u64 },
    SetVariable { name: String, value: Value },
    Log { message: String, level: Option<String> },
    Assert { condition: String, message: Option<String> },
    Loop { items: LoopSource, variable: String, steps: Vec<Step> },
    Conditional { condition: String, then_steps: Vec<Step>, else_steps: Option<Vec<Step>> },
    SubWorkflow { path: String },
    HttpRequest { url: String, method: Option<String>, headers: Option<HashMap<String, String>>, body: Option<String> },
    Snapshot { compact: bool, interactive_only: bool },
    Agent { prompt: String, options: Option<Vec<String>> },
}

pub enum LoopSource {
    Array(Vec<Value>),
    Variable(String),
    Range { start: i64, end: i64 },
}
```

### Execution Engine
```rust
// Primary execution function
pub async fn execute_workflow(
    page: &chromiumoxide::Page,
    workflow: &Workflow,
) -> Result<WorkflowResult>

// Step execution dispatcher (Pattern matching)
fn execute_step<'a>(
    page: &'a chromiumoxide::Page,
    action: &'a Action,
    variables: &'a mut HashMap<String, Value>,
    step_index: usize,
) -> Pin<Box<dyn std::future::Future<Output = Result<Option<Value>>> + Send + 'a>>
```

### Execution Features (Lines 331-445)
- **Sequential execution** with loop through steps
- **Conditional evaluation** (line 348) — skips steps if condition=false
- **Retry logic** (line 363) — max_attempts = 1 + step.retries with delay
- **Variable interpolation** (line 349, 472) — `${variable}` substitution
- **Error handling** (line 432) — Stop/Continue/Retry/Skip per step
- **Agent pause** (line 379) — Pauses workflow at Agent action
- **Variable capture** (line 401) — save_as binding

### Result Structure
```rust
pub struct WorkflowResult {
    pub name: String,
    pub status: StepStatus,
    pub steps: Vec<StepResult>,
    pub variables: HashMap<String, Value>,
    pub total_duration_ms: u64,
    pub steps_succeeded: usize,
    pub steps_failed: usize,
    pub steps_skipped: usize,
    pub paused_at: Option<usize>,
    pub agent_context: Option<Value>,
}

pub struct StepResult {
    pub step_id: String,
    pub step_name: String,
    pub status: StepStatus,
    pub output: Option<Value>,
    pub error: Option<String>,
    pub duration_ms: u64,
    pub paused: bool,
}

pub enum StepStatus {
    Success,
    Failed,
    Skipped,
    Paused,
}
```

---

## 5. MULTI-TAB / MULTI-INSTANCE SUPPORT

### Server State Management
**File:** `crates/onecrawl-server/src/state.rs` (lines 32-100)

```rust
pub struct ServerState {
    pub instances: RwLock<HashMap<String, Instance>>,
    pub profiles: RwLock<HashMap<String, Profile>>,
    pub port: u16,
    pub next_instance_port: RwLock<u16>,
    pub snapshots: RwLock<HashMap<String, Arc<Vec<SnapshotElement>>>>,
    pub tab_index: RwLock<HashMap<String, String>>,  // tab_id -> instance_id
    pub tab_locks: RwLock<HashMap<String, TabLock>>, // Multi-agent safety
}

pub struct TabLock {
    pub owner: String,
    pub acquired_at: Instant,
    pub ttl_secs: u64,
}

impl ServerState {
    pub async fn register_tab(&self, tab_id: &str, instance_id: &str)
    pub async fn unregister_tab(&self, tab_id: &str)
    pub async fn instance_for_tab(&self, tab_id: &str) -> Option<String>  // O(1) lookup
    pub async fn cache_snapshot(&self, tab_id: String, elements: Arc<Vec<SnapshotElement>>)
    pub async fn lock_tab(&self, tab_id: &str, owner: &str, ttl_secs: Option<u64>) -> Result<(), String>
}
```

### Instance Tracking
- **Multiple instances:** HashMap<instance_id, Instance>
- **Port allocation:** Sequential port assignment (port, port+1, port+2...)
- **Per-instance tabs:** Instance::tabs = HashMap<tab_id, Page>
- **Tab reverse index:** Tab lookup without instance iteration

---

## 6. SERVER ROUTES FOR INSTANCES & TABS

**File:** `crates/onecrawl-server/src/routes/mod.rs` (lines 79-100+)

### Instance Routes
```rust
POST   /instances                     → create_instance
GET    /instances                     → list_instances
GET    /instances/{id}                → get_instance
DELETE /instances/{id}                → stop_instance

POST   /instances/{id}/tabs/open      → open_tab
GET    /instances/{id}/tabs           → get_instance_tabs
```

### Tab Routes
```rust
GET    /tabs                          → list_all_tabs
POST   /tabs/{tab_id}/navigate        → navigate_tab
GET    /tabs/{tab_id}/snapshot        → get_snapshot
GET    /tabs/{tab_id}/text            → get_text

POST   /tabs/{tab_id}/action          → execute_action (single)
POST   /tabs/{tab_id}/actions         → execute_actions (batch)
```

### Tab Locking Routes (Multi-agent safety)
```rust
POST   /tabs/{tab_id}/lock            → lock_tab
DELETE /tabs/{tab_id}/lock            → unlock_tab
GET    /tabs/{tab_id}/lock            → get_tab_lock
```

### Instance Creation Logic
**File:** `crates/onecrawl-server/src/routes/instances.rs` (lines 25-52)

```rust
pub async fn create_instance(
    State(state): State<AppState>,
    Json(req): Json<CreateInstanceRequest>,
) -> ApiResult<InstanceResponse> {
    let headless = req.headless.unwrap_or(true);
    let id = format!("inst_{}", uuid::Uuid::new_v4().as_simple());
    
    let mut port_guard = state.next_instance_port.write().await;
    let port = *port_guard;
    *port_guard += 1;
    drop(port_guard);
    
    let instance = Instance::launch(id.clone(), headless, port, req.profile.clone(), user_data_dir)
        .await
        .map_err(|e| api_err(StatusCode::INTERNAL_SERVER_ERROR, &e))?;
    
    let info = instance.info().await;
    state.instances.write().await.insert(id, instance);
    
    Ok(Json(InstanceResponse { instance: info }))
}
```

### Tab Opening Logic
**File:** `crates/onecrawl-server/src/routes/tabs.rs` (lines 26-73)

```rust
pub async fn open_tab(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Json(req): Json<OpenTabRequest>,
) -> ApiResult<TabResponse> {
    let url_str = req.url.as_deref().unwrap_or("about:blank");
    
    let (page, tab_id) = {
        let instances = state.instances.read().await;
        let inst = instances.get(&id)?;
        
        let page = inst.browser.new_page(url_str).await?;
        
        let mut counter = inst.tab_counter.write().await;
        *counter += 1;
        let tab_id = format!("tab_{}_{}", inst.id, counter);
        drop(counter);
        
        inst.tabs.write().await.insert(tab_id.clone(), page.clone());
        (page, tab_id)
    }; // lock dropped before CDP I/O
    
    let tab_url = page.url().await.ok().flatten().unwrap_or_default();
    let tab_title = page.evaluate("document.title").await...;
    
    state.register_tab(&tab_id, &id).await;
    
    Ok(Json(TabResponse { tab: info }))
}
```

---

## 7. WORKFLOW DSL (JSON/YAML Schema)

### Workflow JSON Example
```json
{
  "name": "Example Workflow",
  "description": "Demonstrates workflow DSL",
  "version": "1.0",
  "variables": {
    "search_term": "OneCrawl",
    "wait_time": 3000
  },
  "steps": [
    {
      "id": "step_1",
      "name": "Navigate to Google",
      "action": {
        "type": "navigate",
        "url": "https://google.com"
      }
    },
    {
      "id": "step_2",
      "name": "Search",
      "action": {
        "type": "click",
        "selector": "e5"
      }
    },
    {
      "id": "step_3",
      "name": "Type search term",
      "action": {
        "type": "type",
        "selector": "e5",
        "text": "${search_term}"
      }
    },
    {
      "id": "step_4",
      "name": "Wait for results",
      "action": {
        "type": "wait_for_selector",
        "selector": "e10",
        "timeout_ms": 5000
      },
      "condition": "${search_term}",
      "retries": 2,
      "retry_delay_ms": 500,
      "save_as": "search_result"
    },
    {
      "id": "step_5",
      "name": "Extract links",
      "action": {
        "type": "extract",
        "selector": "a",
        "attribute": "href"
      }
    },
    {
      "id": "step_6",
      "name": "Loop through items",
      "action": {
        "type": "loop",
        "items": {
          "start": 0,
          "end": 5
        },
        "variable": "i",
        "steps": [
          {
            "action": {
              "type": "sleep",
              "ms": "${wait_time}"
            }
          }
        ]
      }
    },
    {
      "id": "step_7",
      "name": "Conditional step",
      "action": {
        "type": "conditional",
        "condition": "${search_result}",
        "then_steps": [
          {
            "action": { "type": "log", "message": "Success!" }
          }
        ],
        "else_steps": [
          {
            "action": { "type": "log", "message": "Failed!" }
          }
        ]
      }
    }
  ],
  "on_error": {
    "action": "stop",
    "screenshot": true,
    "log": true
  }
}
```

### Supported Action Types
| Action | Parameters | Purpose |
|--------|-----------|---------|
| `navigate` | `url` | Navigate to URL |
| `click` | `selector` | Click element (ref_id like "e5") |
| `type` | `selector`, `text` | Type into element |
| `wait_for_selector` | `selector`, `timeout_ms` | Wait for element |
| `screenshot` | `path?`, `full_page?` | Capture page |
| `evaluate` | `js` | Execute JavaScript |
| `extract` | `selector`, `attribute?` | Extract data |
| `smart_click` | `query` | AI-powered click |
| `smart_fill` | `query`, `value` | AI-powered fill |
| `sleep` | `ms` | Delay execution |
| `set_variable` | `name`, `value` | Store variable |
| `log` | `message`, `level?` | Log message |
| `assert` | `condition`, `message?` | Assert condition |
| `loop` | `items`, `variable`, `steps` | Loop execution |
| `conditional` | `condition`, `then_steps`, `else_steps?` | If/else logic |
| `sub_workflow` | `path` | Execute sub-workflow |
| `http_request` | `url`, `method?`, `headers?`, `body?` | HTTP call |
| `snapshot` | `compact?`, `interactive_only?` | Page snapshot |
| `agent` | `prompt`, `options?` | AI agent control |

---

## 8. ACTION DISPATCH PATTERN (Server-level)

**File:** `crates/onecrawl-server/src/action.rs` (lines 1-65)

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind")]
pub enum Action {
    #[serde(rename = "click")]
    Click { ref_id: String },
    #[serde(rename = "type")]
    Type { ref_id: String, text: String },
    #[serde(rename = "fill")]
    Fill { ref_id: String, text: String },
    #[serde(rename = "press")]
    Press { key: String, ref_id: Option<String> },
    #[serde(rename = "hover")]
    Hover { ref_id: String },
    #[serde(rename = "focus")]
    Focus { ref_id: String },
    #[serde(rename = "scroll")]
    Scroll { ref_id: Option<String>, pixels: Option<i64> },
    #[serde(rename = "select")]
    Select { ref_id: String, value: String },
    #[serde(rename = "wait")]
    Wait { time: u64 },
    #[serde(rename = "actions")]
    Batch { actions: Vec<Action> },
}
```

**File:** `crates/onecrawl-server/src/routes/actions.rs` (lines 12-120)

```rust
pub async fn execute_action(
    State(state): State<AppState>,
    Path(tab_id): Path<String>,
    headers: HeaderMap,
    Json(action): Json<Action>,
) -> ApiResult<ActionResult> {
    let owner = headers.get("x-agent-owner").and_then(|v| v.to_str().ok());
    let page = get_tab_page(&state, &tab_id, owner).await?;
    let result = execute_single_action(&page, &action).await;
    Ok(Json(result))
}

async fn execute_single_action(
    page: &chromiumoxide::Page,
    action: &Action,
) -> ActionResult {
    match action {
        Action::Click { ref_id } => eval_ref_action(page, ref_id, click_by_index_js, "click").await,
        Action::Type { ref_id, text } => eval_ref_action(page, ref_id, |i| type_by_index_js(i, text), "type").await,
        Action::Fill { ref_id, text } => ...,
        Action::Press { key, ref_id } => ...,
        Action::Hover { ref_id } => ...,
        Action::Focus { ref_id } => ...,
        Action::Scroll { ref_id, pixels } => ...,
        Action::Select { ref_id, value } => ...,
        Action::Wait { time } => ...,
        Action::Batch { actions } => {
            for action in actions {
                let result = execute_single_action(page, action).await;
                if !result.success { break; }
            }
        }
    }
}
```

---

## 9. CRATE STRUCTURE & DEPENDENCIES

### Workspace Layout
```
crates/
├── onecrawl-cdp/              # Browser automation via CDP
│   ├── src/
│   │   ├── android.rs         ← AndroidClient
│   │   ├── ios.rs             ← IosClient
│   │   ├── browser.rs         ← BrowserSession
│   │   ├── browser_pool.rs    ← BrowserPool
│   │   ├── tabs.rs            ← TabInfo utilities
│   │   ├── workflow.rs        ← Workflow DSL engine
│   │   └── lib.rs
│   └── Cargo.toml
│
├── onecrawl-server/           # HTTP API server
│   ├── src/
│   │   ├── instance.rs        ← Instance struct (multi-tab support)
│   │   ├── state.rs           ← ServerState (multi-instance tracking)
│   │   ├── action.rs          ← Action dispatch enum
│   │   ├── tab.rs             ← TabInfo/NavigateRequest
│   │   ├── routes/
│   │   │   ├── instances.rs   ← /instances/* routes
│   │   │   ├── tabs.rs        ← /tabs/* routes
│   │   │   ├── actions.rs     ← /tabs/{id}/action* routes
│   │   │   └── mod.rs         ← Router setup
│   │   ├── serve.rs           ← HTTP server
│   │   └── lib.rs
│   └── Cargo.toml
│
└── onecrawl-core/             # Error types
    └── src/lib.rs
```

### Key Dependencies
- **chromiumoxide** (0.8) — CDP protocol, Browser/Page handles
- **tokio** — Async runtime, RwLock for concurrent access
- **reqwest** — HTTP client for Android/iOS servers
- **serde/serde_json** — Serialization for workflows, requests/responses
- **axum** — HTTP routing framework (server)

---

## 10. KEY PATTERNS FOR MULTI-DEVICE ORCHESTRATION

### Pattern 1: Device Client Abstraction
Both Android and iOS clients follow identical HTTP + session pattern:
```rust
pub struct {Device}Client {
    config: {Device}SessionConfig,
    session_id: Option<String>,
    client: reqwest::Client,
}
```
→ **To add:** Unify under trait `DeviceClient` for dispatch

### Pattern 2: Multi-Instance Tracking
```rust
ServerState {
    instances: HashMap<instance_id, Instance>,
    tab_index: HashMap<tab_id, instance_id>,  // O(1) lookup
    tab_locks: HashMap<tab_id, TabLock>,      // Multi-agent safety
}
```
→ **To add:** `device_index: HashMap<device_id, DeviceClient>` for cross-device orchestration

### Pattern 3: Workflow as Portable Recipe
Workflow JSON/YAML can execute sequentially across steps:
- Parse once, execute many times
- Variables persist across steps
- Conditional/loop/retry logic is built-in
→ **To add:** Multi-device workflow steps that dispatch to Android/iOS clients

### Pattern 4: Action Dispatch at Multiple Levels
1. **Workflow level** (execute_step) — pattern matches Action enum → executes via CDP
2. **Server level** (execute_action) — pattern matches Action enum → executes via CDP
→ **To add:** Device-aware dispatch that routes Action to AndroidClient/IosClient based on device_type

### Pattern 5: Async/Await with Lock Management
- Instance/tab operations use RwLock (read-heavy, write-rare)
- Locks dropped before CDP I/O (non-blocking pattern)
- Per-tab locks prevent concurrent agent access (line 98-100, state.rs)
→ **To add:** Device-level locks to coordinate cross-device workflows

---

## 11. IMPLEMENTATION ROADMAP FOR MULTI-DEVICE ORCHESTRATION

### Phase 1: Device Client Unification
```rust
// New file: crates/onecrawl-cdp/src/device.rs
#[async_trait::async_trait]
pub trait DeviceClient: Send + Sync {
    async fn navigate(&self, url: &str) -> Result<()>;
    async fn tap(&self, x: f64, y: f64) -> Result<()>;
    async fn get_url(&self) -> Result<String>;
    async fn screenshot(&self) -> Result<Vec<u8>>;
    // ... 15+ methods
}

impl DeviceClient for AndroidClient { ... }
impl DeviceClient for IosClient { ... }
impl DeviceClient for BrowserSession { ... }  // Desktop as Device
```

### Phase 2: Multi-Device Orchestration State
```rust
// Extended in crates/onecrawl-server/src/state.rs
pub struct ServerState {
    // Existing:
    instances: RwLock<HashMap<String, Instance>>,
    
    // New:
    devices: RwLock<HashMap<String, Arc<dyn DeviceClient>>>,
    device_index: RwLock<HashMap<String, String>>,  // device_id -> type (android|ios|browser)
    device_orchestrations: RwLock<HashMap<String, Vec<String>>>,  // orchestration_id -> [device_ids]
}
```

### Phase 3: Multi-Device Workflow Steps
```rust
// Extended in crates/onecrawl-cdp/src/workflow.rs
pub enum Action {
    // Existing browser actions
    Navigate { url: String },
    ...
    
    // New multi-device actions:
    DeviceNavigate { device_id: String, url: String },
    DeviceTap { device_id: String, x: f64, y: f64 },
    DeviceScreenshot { device_id: String, path: Option<String> },
    DeviceType { device_id: String, text: String },
    
    // Orchestration actions:
    Orchestration { 
        device_ids: Vec<String>,
        parallel: bool,  // true = concurrent, false = sequential
        steps: Vec<Step>,
    },
}
```

### Phase 4: Multi-Device Routes
```rust
// New in crates/onecrawl-server/src/routes/devices.rs
POST   /devices/register          → register_android|ios
POST   /devices/{id}/action       → execute_device_action
GET    /devices/{id}/screenshot   → get_device_screenshot
POST   /orchestrations            → create_orchestration
POST   /orchestrations/{id}/execute → execute_multi_device_workflow
```

---

## Summary

**Current Infrastructure:**
- ✅ Desktop: Browser/Tab management via CDP (Instance model)
- ✅ Android: UIAutomator2 HTTP client (AndroidClient)
- ✅ iOS: WebDriverAgent HTTP client (IosClient)
- ✅ Workflow DSL: Portable recipe engine with conditionals/loops
- ✅ Action dispatch: Pattern-matched execution on single device
- ✅ Multi-tab: Native support via Instance.tabs HashMap
- ✅ Multi-instance: ServerState tracks instances + tab index

**Gaps for Multi-Device Orchestration:**
- ❌ No device abstraction trait (Android/iOS/Browser unification)
- ❌ No orchestration state (device registry, cross-device workflow history)
- ❌ No workflow actions for device coordination (parallel execution, cross-device synchronization)
- ❌ No routes for device registration/lifecycle management
- ❌ No device-aware action dispatcher (which device executes which action)

**Key Files to Modify:**
1. `crates/onecrawl-cdp/src/workflow.rs` — Add multi-device Action variants
2. `crates/onecrawl-server/src/state.rs` — Add device registry + orchestration tracking
3. `crates/onecrawl-server/src/routes/mod.rs` — Add device management routes
4. New: `crates/onecrawl-cdp/src/device.rs` — DeviceClient trait
5. New: `crates/onecrawl-server/src/routes/devices.rs` — Device routes
