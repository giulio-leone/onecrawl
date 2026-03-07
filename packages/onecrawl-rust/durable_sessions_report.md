# OneCrawl Rust: Durable Sessions Infrastructure Report

## Summary
The OneCrawl Rust project has foundational infrastructure for implementing Durable Sessions. A daemon process with multi-session support already exists, health monitoring is in place, and state persistence patterns are partially implemented.

---

## 1. DAEMON MODULE

**Location**: `/Users/giulioleone/Sviluppo/onecrawl-dev/packages/onecrawl-rust/crates/onecrawl-cli-rs/src/commands/daemon/`

### Files:
- **mod.rs** (206 lines): CLI interface for daemon start/stop/status/exec
- **protocol.rs** (34 lines): Request/Response serialization
- **server.rs** (625 lines): Core daemon implementation
- **client.rs**: Client for sending commands to daemon

### Key Structs:

**DaemonRequest** (protocol.rs):
```rust
pub struct DaemonRequest {
    pub id: String,
    pub command: String,
    pub args: serde_json::Value,
    pub session: Option<String>,  // Named session support
}
```

**DaemonResponse** (protocol.rs):
```rust
pub struct DaemonResponse {
    pub id: String,
    pub success: bool,
    pub data: Option<serde_json::Value>,
    pub error: Option<String>,
}
```

**SessionState** (server.rs, private):
```rust
struct SessionState {
    _session: BrowserSession,
    page: Page,
}
```

**DaemonState** (server.rs, private):
```rust
struct DaemonState {
    sessions: HashMap<String, SessionState>,  // Multi-session support
    headless: bool,
}
```

**PersistedState** (server.rs):
```rust
struct PersistedState {
    sessions: Vec<String>,
    headless: bool,
}
```

### Key Methods:

- `daemon_start(headless: bool)` - Spawns daemon as detached background process
  - Uses `setsid()` on Unix for process isolation
  - Redirects stdio to /dev/null
  - Writes PID file at `/tmp/onecrawl-daemon.pid`

- `daemon_status()` - Checks daemon liveness and lists active sessions

- `daemon_stop()` - Graceful shutdown via "shutdown" command

- `daemon_exec(command, args, session)` - Route arbitrary commands to daemon

- `start_daemon(headless)` (server.rs) - Main server loop:
  - Binds Unix socket at `/tmp/onecrawl-daemon.sock`
  - Launches eager "default" browser session
  - Spawns idle-timeout watcher (30 min timeout by default)
  - Spawns SIGTERM/SIGINT handler
  - **Spawns health monitoring task** (runs every 60s)
  - Accepts connections in loop until shutdown notification

- `dispatch_command()` - Routes commands to handlers (ping, status, shutdown, goto, click, etc.)

- `save_state()` - Serializes sessions list to `/tmp/onecrawl-daemon-state.json`

### Key Infrastructure:

- **Socket-based IPC**: Unix domain socket at `/tmp/onecrawl-daemon.sock`
- **JSON-line protocol**: One request/response per line
- **Multi-session**: Map from session name → (BrowserSession, Page)
- **Health monitoring**: Runs `onecrawl_cdp::harness::health_check()` every 60 seconds on all sessions
- **Dead session removal**: Automatically removes sessions that fail health check
- **Idle timeout**: Daemon auto-shuts down after 30 minutes of inactivity
- **Session persistence**: List of active session names saved to JSON file

---

## 2. HARNESS MODULE

**Location**: `/Users/giulioleone/Sviluppo/onecrawl-dev/packages/onecrawl-rust/crates/onecrawl-cdp/src/harness.rs` (392 lines)

### Key Structs:

**CircuitBreaker**:
```rust
pub struct CircuitBreaker {
    pub consecutive_failures: u32,
    pub threshold: u32,
    pub is_open: bool,
    pub last_failure: Option<String>,
}
```

### Key Functions:

1. **`health_check(page: &Page) -> Result<serde_json::Value>`**
   - Executes JavaScript to gather:
     - URL, title, readyState
     - Memory metrics (usedJSHeapSize, totalJSHeapSize, jsHeapSizeLimit)
     - Navigation timing (domComplete, loadEventEnd, domInteractive)
     - Tab count, error count
     - Response time (marked healthy if < 5000ms)
   - Returns JSON with all metrics

2. **`reconnect_cdp(page: &Page, max_retries: usize) -> Result<Value>`**
   - Auto-reconnect with exponential backoff (100ms → 10s capped)
   - Pings page.evaluate() until connected or max retries exhausted
   - Returns connection status, ready_state, attempt count

3. **`checkpoint_save(page: &Page, checkpoint_path: &str, name: &str) -> Result<Value>`**
   - Captures browser state to disk:
     - URL
     - Scroll position (x, y)
     - localStorage (all key-value pairs)
     - sessionStorage (all key-value pairs)
     - Cookies (raw document.cookie string)
     - Timestamp
   - Saves as JSON file: `{checkpoint_path}/{name}.json`
   - Returns file path and size

4. **`checkpoint_restore(page: &Page, checkpoint_path: &str, name: &str) -> Result<Value>`**
   - Restores state from checkpoint:
     - Navigates to saved URL
     - Restores localStorage via JS loop
     - Restores sessionStorage via JS loop
     - Restores cookies via document.cookie assignment
     - Restores scroll position

5. **`gc_tabs_info(page: &Page) -> Result<Value>`**
   - Returns current tab info for session pool management
   - Captures current URL, title, timestamp

6. **`watchdog_status(page: &Page) -> Result<Value>`**
   - Monitors browser responsiveness:
     - Sends test evaluation with 5s timeout
     - Captures memory info if alive
     - Returns alive status, response time, memory metrics

### CircuitBreaker Methods:

- `new(threshold)` - Create with failure threshold
- `record_success()` - Reset failure counter, close circuit
- `record_failure(error)` - Increment counter, open if threshold reached
- `should_proceed()` - Check if circuit is open
- `reset()` - Clear state
- `status()` - Return JSON status object

---

## 3. SESSION/STATE MANAGEMENT

### SessionInfo Struct

**Location**: `/Users/giulioleone/Sviluppo/onecrawl-dev/packages/onecrawl-rust/crates/onecrawl-cli-rs/src/commands/session/core.rs`

```rust
pub struct SessionInfo {
    pub ws_url: String,
    pub pid: Option<u32>,
    pub server_port: Option<u16>,
    pub server_pid: Option<u32>,
    pub default_tab_id: Option<String>,
    pub instance_id: Option<String>,
    pub active_tab_id: Option<String>,
    pub headless: bool,
    pub passkey_file: Option<String>,
    pub passkey_rp_id: Option<String>,
    pub fingerprint_ua: Option<String>,
}
```

### ServerState Struct

**Location**: `/Users/giulioleone/Sviluppo/onecrawl-dev/packages/onecrawl-rust/crates/onecrawl-server/src/state.rs`

```rust
pub struct ServerState {
    pub instances: RwLock<HashMap<String, Instance>>,
    pub profiles: RwLock<HashMap<String, Profile>>,
    pub port: u16,
    pub next_instance_port: RwLock<u16>,
    pub snapshots: RwLock<HashMap<String, Arc<Vec<SnapshotElement>>>>,
    pub tab_index: RwLock<HashMap<String, String>>,  // tab_id → instance_id
    pub tab_locks: RwLock<HashMap<String, TabLock>>, // Per-tab locks for multi-agent safety
}
```

### TabLock Struct

```rust
pub struct TabLock {
    pub owner: String,
    pub acquired_at: Instant,
    pub ttl_secs: u64,
}
```

### BrowserSession Struct

**Location**: `/Users/giulioleone/Sviluppo/onecrawl-dev/packages/onecrawl-rust/crates/onecrawl-cdp/src/browser.rs` (137 lines)

```rust
pub struct BrowserSession {
    browser: Browser,
    _handler_task: tokio::task::JoinHandle<()>,
}
```

**Key Methods**:
- `launch_headless() -> Result<Self>` - New stealth headless browser
- `launch_headed() -> Result<Self>` - New visible browser
- `connect(ws_url: &str) -> Result<Self>` - Reconnect to existing browser
- `connect_with_nav_timeout(ws_url: &str) -> Result<Self>` - Extended 90s timeout for SPAs
- `new_page(url: &str) -> Result<chromiumoxide::Page>` - Create new tab
- `close(self) -> Result<()>` - Close browser

---

## 4. CHECKPOINT SYSTEM (EXISTING)

**Location**: `crates/onecrawl-cdp/src/harness.rs` (lines 157-301)

### Checkpoint Save Flow:
```
checkpoint_save(page, path, name)
  ├─ Evaluate JS to capture:
  │   ├─ URL
  │   ├─ localStorage (loop all keys)
  │   ├─ sessionStorage (loop all keys)
  │   └─ scroll position (x, y)
  ├─ Evaluate JS to get document.cookie
  ├─ Create JSON object with:
  │   ├─ name
  │   ├─ state (URL, title, scroll, storage)
  │   ├─ cookies (raw string array)
  │   └─ saved_at (Unix timestamp)
  ├─ Create dir if needed
  └─ Write to {path}/{name}.json
```

### Checkpoint Restore Flow:
```
checkpoint_restore(page, path, name)
  ├─ Read {path}/{name}.json
  ├─ Navigate to saved URL
  ├─ Loop localStorage entries: localStorage.setItem(k, v)
  ├─ Loop sessionStorage entries: sessionStorage.setItem(k, v)
  ├─ Loop cookies: document.cookie = c
  └─ Restore scroll: window.scrollTo(x, y)
```

**Format**: JSON file with state and cookies
**Storage**: Filesystem-based (configurable path)
**Scope**: Single browser state per checkpoint

---

## 5. AUTH STATE PERSISTENCE

**Location**: `/Users/giulioleone/Sviluppo/onecrawl-dev/packages/onecrawl-rust/crates/onecrawl-cli-rs/src/commands/browser/media/auth_state.rs` (143 lines)

### Directory: `~/.onecrawl/auth-states/`

### Functions:

1. **`auth_state_save(name: &str)`** - Save current page auth state
   - Captures: cookies, localStorage, sessionStorage, URL
   - Saves as `{name}.json`
   - Output: "✓ Auth state saved as '...'"

2. **`auth_state_load(name: &str)`** - Restore named auth state
   - Loads JSON from `~/.onecrawl/auth-states/{name}.json`
   - Sets localStorage entries via JS loop
   - Sets sessionStorage entries via JS loop
   - Sets cookies via document.cookie assignment

3. **`auth_state_list()`** - List saved auth states with sizes

4. **`auth_state_show(name: &str)`** - Display JSON content of auth state

5. **`auth_state_rename(from, to)`** - Rename saved state file

6. **`auth_state_clear(name)`** - Delete saved state

7. **`auth_state_clean()`** - Delete all auth states

### Format:
```json
{
  "url": "https://...",
  "cookies": "cookie1=value1; cookie2=value2",
  "localStorage": {"key1": "value1", ...},
  "sessionStorage": {"key2": "value2", ...}
}
```

---

## 6. CLI DISPATCH PATTERN

**Location**: `/Users/giulioleone/Sviluppo/onecrawl-dev/packages/onecrawl-rust/crates/onecrawl-cli-rs/src/dispatch.rs` (1800+ lines)

### Commands Enum Structure

Located in `/crates/onecrawl-cli-rs/src/cli/mod.rs`:

```rust
#[derive(Subcommand)]
pub(crate) enum Commands {
    // System
    Health,
    Info,
    
    // Session
    Session { action: crate::commands::session::SessionAction },
    
    // Navigation
    Navigate { url: String, wait: u64, wait_cf: bool },
    Back, Forward, Reload,
    
    // Content
    Get { what: String, selector: Option<String>, arg: Option<String> },
    Eval { expression: String },
    SetContent { html: String },
    
    // Interactions
    Click { selector: String },
    Type { selector: String, text: String },
    // ... 100+ more commands
    
    // Daemon
    Daemon { action: DaemonAction },
    
    // Harness
    Harness { action: HarnessAction },
    
    // ... auth, crawl, tabs, etc.
}
```

### Dispatch Flow:

```rust
// In dispatch.rs
pub(crate) async fn dispatch(command: Commands) {
    match command {
        Commands::Health => { ... },
        Commands::Session { action } => commands::session::handle(action).await,
        Commands::Daemon { action } => match action {
            DaemonAction::Start { headless } => commands::daemon::daemon_start(headless).await,
            DaemonAction::Stop => commands::daemon::daemon_stop().await,
            DaemonAction::Status => commands::daemon::daemon_status().await,
            DaemonAction::Exec { command, args, session } => 
                commands::daemon::daemon_exec(&command, args, session).await,
            DaemonAction::Run { headless } => commands::daemon::server::start_daemon(headless).await,
        },
        // ... match all other commands
    }
}
```

### Pattern for Adding New Top-Level Command (e.g., `onecrawl durable`):

1. **Define Subcommand Enum** in `crates/onecrawl-cli-rs/src/cli/durable.rs`:
```rust
#[derive(clap::Subcommand)]
pub enum DurableAction {
    /// Create a durable session
    Create { name: String },
    /// Restore a durable session
    Restore { name: String },
    /// List durable sessions
    List,
    /// Delete durable session
    Delete { name: String },
}
```

2. **Export in** `crates/onecrawl-cli-rs/src/cli/mod.rs`:
```rust
mod durable;
pub use durable::DurableAction;
```

3. **Add to Commands enum** in `cli/mod.rs`:
```rust
pub(crate) enum Commands {
    // ... existing commands
    Durable { 
        #[command(subcommand)]
        action: DurableAction 
    },
    // ... rest
}
```

4. **Add dispatch case** in `dispatch.rs`:
```rust
Commands::Durable { action } => commands::durable::handle(action).await,
```

5. **Create handler** at `crates/onecrawl-cli-rs/src/commands/durable/mod.rs`:
```rust
pub async fn handle(action: crate::cli::DurableAction) {
    match action {
        DurableAction::Create { name } => create_session(&name).await,
        DurableAction::Restore { name } => restore_session(&name).await,
        // ... etc
    }
}
```

---

## 7. MCP HANDLER PATTERN

**Location**: `/Users/giulioleone/Sviluppo/onecrawl-dev/packages/onecrawl-rust/crates/onecrawl-mcp-rs/`

### Server Structure (server.rs)

```rust
#[derive(Clone)]
pub struct OneCrawlMcp {
    #[allow(dead_code)]
    tool_router: ToolRouter<Self>,
    pub(crate) store_path: Arc<String>,
    pub(crate) store_password: Arc<String>,
    pub(crate) browser: SharedBrowser,
}

#[tool_router]  // Procedural macro that generates tool routing
impl OneCrawlMcp {
    pub fn new(store_path: String, store_password: String) -> Self { ... }
    pub(crate) fn open_store(&self) -> Result<EncryptedStore, McpError> { ... }
    async fn enforce_safety(&self, tool_name: &str, action_name: &str) -> Result<(), McpError> { ... }
    // ... 100+ tool handler methods
}
```

### Handler Pattern (browser.rs first 100 lines)

```rust
impl OneCrawlMcp {
    pub(crate) async fn navigation_goto(
        &self,
        p: NavigateParams,  // Strongly-typed parameter struct
    ) -> Result<CallToolResult, McpError> {
        // 1. Enforce safety policy
        let state = self.browser.lock().await;
        if let Some(ref safety) = state.safety {
            match safety.check_url(&p.url) {
                onecrawl_cdp::SafetyCheck::Denied(reason) => {
                    return Err(McpError::invalid_params(
                        format!("safety policy denied URL: {reason}"),
                        None,
                    ));
                }
                _ => {}
            }
        }
        
        // 2. Get or ensure page exists
        let page = ensure_page(&self.browser).await?;
        
        // 3. Call underlying CDP function
        onecrawl_cdp::navigation::goto(&page, &p.url).await.mcp()?;
        
        // 4. Gather context for response
        let title = onecrawl_cdp::navigation::get_title(&page)
            .await
            .unwrap_or_default();
        
        // 5. Return result
        text_ok(format!("navigated to {} — title: {title}", p.url))
    }
    
    pub(crate) async fn navigation_click(
        &self,
        p: ClickParams,
    ) -> Result<CallToolResult, McpError> {
        let page = ensure_page(&self.browser).await?;
        let selector = onecrawl_cdp::accessibility::resolve_ref(&p.selector);
        onecrawl_cdp::element::click(&page, &selector).await.mcp()?;
        text_ok(format!("clicked {}", p.selector))
    }
    
    pub(crate) async fn navigation_screenshot(
        &self,
        p: ScreenshotParams,
    ) -> Result<CallToolResult, McpError> {
        let page = ensure_page(&self.browser).await?;
        let bytes = if let Some(sel) = &p.selector {
            onecrawl_cdp::screenshot::screenshot_element(&page, sel).await.mcp()?
        } else if p.full_page.unwrap_or(false) {
            onecrawl_cdp::screenshot::screenshot_full(&page).await.mcp()?
        } else {
            onecrawl_cdp::screenshot::screenshot_viewport(&page).await.mcp()?
        };
        let b64 = B64.encode(&bytes);
        Ok(CallToolResult::success(vec![Content::image(b64, "image/png")]))
    }
}
```

### Registration Pattern (via #[tool_router] macro)

The `#[tool_router]` attribute on the impl block automatically:
1. Scans all `pub(crate) async fn` methods
2. Maps method names to MCP tool actions (e.g., `navigation_goto` → BrowserAction::NavigationGoto)
3. Generates route matching and parameter parsing
4. Returns `CallToolResult` with formatted output

### Tools Structure in cdp_tools.rs

Parameter structs define the input schema:
```rust
#[derive(serde::Deserialize)]
pub struct NavigateParams {
    pub url: String,
}

#[derive(serde::Deserialize)]
pub struct ClickParams {
    pub selector: String,
}

#[derive(serde::Deserialize)]
pub struct ScreenshotParams {
    #[serde(default)]
    pub selector: Option<String>,
    #[serde(default)]
    pub full_page: Option<bool>,
    #[serde(default)]
    pub format: Option<String>,
}
```

### Helper Functions (helpers.rs)

```rust
pub async fn ensure_page(browser: &SharedBrowser) -> Result<Page, McpError> { ... }
pub fn json_ok(v: serde_json::Value) -> Result<CallToolResult, McpError> { ... }
pub fn text_ok(text: impl Into<String>) -> Result<CallToolResult, McpError> { ... }
pub fn parse_params<T: serde::de::DeserializeOwned>(
    v: serde_json::Value,
    name: &str,
) -> Result<T, McpError> { ... }
```

---

## Implementation Roadmap for Durable Sessions

### Recommended Structure:

1. **Create new CLI subcommand**: `onecrawl durable create|restore|list|delete|checkpoint|replay`

2. **Extend harness.rs** with:
   - `DurableSession` struct wrapping checkpoint metadata + browser session
   - Lifecycle functions: create, persist, restore, cleanup
   - Auto-checkpoint on interval/trigger

3. **Storage layer**: 
   - Extend `onecrawl-storage` crate for encrypted checkpoint storage
   - Or use `~/.onecrawl/durable-sessions/` directory with JSON

4. **Daemon enhancement**:
   - Track session creation time and last activity
   - Implement session pooling with checkpoint recovery
   - Add `durable_create` and `durable_restore` commands

5. **MCP integration**:
   - Add `durable_create`, `durable_restore`, `durable_checkpoint` handlers
   - Implement safety checks for session restore

---

