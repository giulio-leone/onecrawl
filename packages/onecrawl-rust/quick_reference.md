# Durable Sessions Quick Reference

## Key File Locations

| Component | Path |
|-----------|------|
| Daemon | `crates/onecrawl-cli-rs/src/commands/daemon/` |
| Harness | `crates/onecrawl-cdp/src/harness.rs` |
| Browser Session | `crates/onecrawl-cdp/src/browser.rs` |
| Server State | `crates/onecrawl-server/src/state.rs` |
| CLI Session | `crates/onecrawl-cli-rs/src/commands/session/` |
| Auth State | `crates/onecrawl-cli-rs/src/commands/browser/media/auth_state.rs` |
| CLI Dispatch | `crates/onecrawl-cli-rs/src/dispatch.rs` |
| MCP Server | `crates/onecrawl-mcp-rs/src/server.rs` |
| MCP Browser Handlers | `crates/onecrawl-mcp-rs/src/handlers/browser.rs` |

---

## Critical Structs for Durable Sessions

### Must Understand:
1. **BrowserSession** - Wraps chromiumoxide::Browser + handler task
2. **CircuitBreaker** - Tracks failures, opens circuit on threshold
3. **SessionInfo** - Persistent session metadata (ws_url, pid, ua, headless flag)
4. **ServerState** - In-memory session registry + tab locks
5. **DaemonState** - Per-session (browser, page) mapping
6. **TabLock** - Multi-agent safety with TTL expiry

### Key Methods:
- `health_check(page)` → JSON with memory, timing, responsiveness
- `checkpoint_save(page, path, name)` → Saves state to JSON file
- `checkpoint_restore(page, path, name)` → Restores state from JSON
- `reconnect_cdp(page, max_retries)` → Auto-reconnect with backoff
- `watchdog_status(page)` → Browser liveness check with timeout

---

## Daemon Architecture

```
┌─ CLI: onecrawl daemon start --headless
│
├─ Spawns detached child (setsid on Unix)
│  └─ Binds Unix socket: /tmp/onecrawl-daemon.sock
│  └─ Launches "default" BrowserSession (eager)
│  └─ Saves PID: /tmp/onecrawl-daemon.pid
│  └─ Spawns 3 background tasks:
│     ├─ Health monitor (60s interval, removes dead sessions)
│     ├─ Signal handler (SIGTERM/SIGINT → shutdown)
│     └─ Idle timeout (30min inactivity → auto-close)
│
└─ CLI: onecrawl daemon exec "goto" url=http://example.com --session=my-session
   └─ Sends JSON-line request over socket
   └─ Daemon creates session if needed
   └─ Returns JSON-line response
```

---

## Checkpoint Format

```json
{
  "name": "checkpoint_name",
  "state": {
    "url": "https://example.com/page",
    "title": "Page Title",
    "scroll": { "x": 100, "y": 500 },
    "localStorage": {
      "theme": "dark",
      "user_id": "12345"
    },
    "sessionStorage": {
      "temp_token": "abc123"
    },
    "timestamp": 1709500000
  },
  "cookies": ["cookie1=val1", "cookie2=val2"],
  "saved_at": "1709500000"
}
```

**Storage**: `{checkpoint_path}/{name}.json` (configurable)

---

## Auth State Format

```json
{
  "url": "https://example.com/dashboard",
  "cookies": "session=abc123; user=john",
  "localStorage": {
    "auth_token": "eyJhbGc...",
    "user_prefs": "{\"theme\": \"dark\"}"
  },
  "sessionStorage": {
    "temp_data": "value"
  }
}
```

**Storage**: `~/.onecrawl/auth-states/{name}.json`

---

## Adding a New Durable Sessions Command

### Step 1: Create CLI subcommand enum
File: `crates/onecrawl-cli-rs/src/cli/durable.rs`
```rust
#[derive(clap::Subcommand)]
pub enum DurableAction {
    Create { name: String },
    Restore { name: String },
    List,
}
```

### Step 2: Export in cli/mod.rs
```rust
mod durable;
pub use durable::DurableAction;

pub(crate) enum Commands {
    Durable { #[command(subcommand)] action: DurableAction },
    // ...
}
```

### Step 3: Add dispatch
File: `dispatch.rs`
```rust
Commands::Durable { action } => commands::durable::handle(action).await,
```

### Step 4: Implement handler
File: `crates/onecrawl-cli-rs/src/commands/durable/mod.rs`
```rust
pub async fn handle(action: crate::cli::DurableAction) {
    match action {
        DurableAction::Create { name } => { /* ... */ },
        DurableAction::Restore { name } => { /* ... */ },
        DurableAction::List => { /* ... */ },
    }
}
```

---

## Adding an MCP Tool Handler

### Pattern (from browser.rs):

```rust
impl OneCrawlMcp {
    pub(crate) async fn durable_checkpoint(
        &self,
        p: CheckpointParams,  // Define struct with serde::Deserialize
    ) -> Result<CallToolResult, McpError> {
        // 1. Enforce safety (optional)
        self.enforce_safety("durable", "checkpoint").await?;
        
        // 2. Ensure page exists
        let page = ensure_page(&self.browser).await?;
        
        // 3. Call implementation (harness.rs)
        let result = onecrawl_cdp::harness::checkpoint_save(
            &page,
            &p.path,
            &p.name,
        ).await.mcp()?;
        
        // 4. Return formatted result
        json_ok(result)  // or text_ok(msg)
    }
}
```

### Auto-registration:
The `#[tool_router]` macro on impl block automatically handles routing.
Method name `durable_checkpoint` maps to MCP action enum.

---

## Health Monitoring Details

### What health_check() Returns:
```json
{
  "url": "https://current.page",
  "title": "Page Title",
  "readyState": "complete",
  "memory": {
    "used_js_heap": 15728640,
    "total_js_heap": 20971520,
    "heap_limit": 2147483648
  },
  "timing": {
    "dom_complete": 1250,
    "load_event": 1500,
    "dom_interactive": 800
  },
  "tab_count": 1,
  "errors": 0,
  "response_time_ms": 145,
  "healthy": true
}
```

### Daemon's Health Monitoring:
- Runs every 60 seconds on all sessions
- Calls `onecrawl_cdp::harness::health_check(page)`
- If health_check fails: session is marked dead and removed
- Saves updated session list to state file

---

## Multi-Session Pattern (Daemon)

```rust
// Daemon has per-session state:
struct DaemonState {
    sessions: HashMap<String, SessionState>,  // name → (browser, page)
}

// When client sends command with --session=my-session:
// 1. If session doesn't exist: launch new BrowserSession
// 2. Get or create page in that session
// 3. Execute command on that page
// 4. Save state (list of session names)

// Session auto-cleanup:
// Health monitor removes sessions that fail health_check
// Daemon auto-shutdown after 30min idle (resets on each command)
```

---

## State Persistence

### Daemon saves to `/tmp/onecrawl-daemon-state.json`:
```json
{
  "sessions": ["default", "session2", "session3"],
  "headless": true
}
```

### Session saves to `/tmp/onecrawl-session.json`:
```json
{
  "ws_url": "ws://127.0.0.1:9222/devtools/browser/abc...",
  "pid": 12345,
  "headless": true,
  "passkey_file": null,
  "fingerprint_ua": "Mozilla/5.0..."
}
```

---

## Error Handling Patterns

### CLI:
```rust
if let Err(e) = result {
    eprintln!("{} Error: {}", "✗".red(), e);
    std::process::exit(1);
}
println!("{} Success", "✓".green());
```

### MCP:
```rust
// Convert onecrawl_cdp::Error to McpError:
onecrawl_cdp::function().await.mcp()?

// Or explicit:
.map_err(|e| McpError::invalid_params(e.to_string(), None))
```

### Daemon:
```rust
// Returns DaemonResponse with success flag and optional error
DaemonResponse {
    id: req.id,
    success: false,
    data: None,
    error: Some("reason".to_string()),
}
```

---

## Testing Durable Sessions (Checklist)

- [ ] Create session with daemon
- [ ] Navigate and take checkpoint
- [ ] Verify JSON file exists with correct structure
- [ ] Restore from checkpoint on new session
- [ ] Verify localStorage/sessionStorage/cookies are restored
- [ ] Verify scroll position is restored
- [ ] Health check on restored session passes
- [ ] Circuit breaker opens after N failures
- [ ] Daemon auto-removes dead sessions
- [ ] Daemon survives parent process exit
- [ ] Session persists across `connect()` calls
- [ ] Multi-session independence (no cross-contamination)

