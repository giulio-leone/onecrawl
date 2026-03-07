# QUICK REFERENCE: Multi-Device Orchestration

## File Paths (Absolute)
```
/Users/giulioleone/Sviluppo/onecrawl-dev/packages/onecrawl-rust/crates/
├── onecrawl-cdp/src/
│   ├── android.rs           (628 lines)  — AndroidClient
│   ├── ios.rs               (673 lines)  — IosClient
│   ├── browser.rs           — BrowserSession
│   ├── browser_pool.rs      — BrowserPool (pool of page handles)
│   ├── tabs.rs              — TabInfo utilities
│   └── workflow.rs          (907 lines)  — Workflow DSL engine
└── onecrawl-server/src/
    ├── instance.rs          — Instance (multi-tab support)
    ├── state.rs             — ServerState (multi-instance tracking)
    ├── action.rs            — Action enum (server-level actions)
    ├── tab.rs               — TabInfo structures
    └── routes/
        ├── instances.rs     — /instances/* routes
        ├── tabs.rs          — /instances/{id}/tabs/* + /tabs/* routes
        ├── actions.rs       — /tabs/{id}/action* routes
        └── mod.rs           — Router setup + route definitions
```

## Core Structs

### Devices (Clients)
| Struct | Location | Methods | Transport |
|--------|----------|---------|-----------|
| **AndroidClient** | android.rs:36 | 23 public async methods | HTTP → UIAutomator2 (Appium) |
| **IosClient** | ios.rs:43 | 20+ public async methods | HTTP → WebDriverAgent (W3C) |
| **BrowserSession** | browser.rs | 4 public async methods | CDP |

### Server/State
| Struct | Location | Purpose |
|--------|----------|---------|
| **Instance** | instance.rs:34 | Single browser instance + multi-tab (HashMap<tab_id, Page>) |
| **ServerState** | state.rs:32 | Multi-instance tracking + tab_index + tab_locks |
| **TabLock** | state.rs:17 | Per-tab locking for multi-agent safety |

### Workflow
| Struct | Location | Purpose |
|--------|----------|---------|
| **Workflow** | workflow.rs:13 | Complete workflow definition |
| **Step** | workflow.rs:28 | Single workflow step |
| **Action** | workflow.rs:51 | 19 action types (enum dispatch) |
| **WorkflowResult** | workflow.rs:131 | Execution result with status |

## Key Methods

### AndroidClient
```
new(config) → Self
create_session(package?, activity?) → String
navigate(url) → ()
tap(x, y) → ()
swipe(from_x, from_y, to_x, to_y, duration_ms) → ()
screenshot() → String (base64)
find_element(strategy, value) → String (element_id)
```

### IosClient
```
new(config) → Self
create_session() → String
navigate(url) → ()
tap(x, y) → ()
swipe(from_x, from_y, to_x, to_y, duration) → ()
screenshot() → Vec<u8> (raw bytes)
find_element(using, value) → String (element_id)
```

### ServerState
```
register_tab(tab_id, instance_id)
unregister_tab(tab_id)
instance_for_tab(tab_id) → Option<String>    // O(1) lookup
lock_tab(tab_id, owner, ttl_secs?) → Result<()>
```

### Workflow Execution
```
execute_workflow(page, workflow) → Result<WorkflowResult>
execute_step(page, action, variables, step_index) → Pin<Box<...>>
```

## Routes (REST API)

### Instance Management
```
POST   /instances                    — Create browser instance
GET    /instances                    — List all instances
GET    /instances/{id}               — Get instance info
DELETE /instances/{id}               — Stop instance
```

### Tab Management
```
POST   /instances/{id}/tabs/open     — Open new tab in instance
GET    /instances/{id}/tabs          — List tabs in instance
GET    /tabs                         — List all tabs (all instances)
POST   /tabs/{tab_id}/navigate       — Navigate tab
```

### Actions
```
POST   /tabs/{tab_id}/action         — Execute single action
POST   /tabs/{tab_id}/actions        — Execute batch actions
```

### Locking
```
POST   /tabs/{tab_id}/lock           — Lock tab
DELETE /tabs/{tab_id}/lock           — Unlock tab
GET    /tabs/{tab_id}/lock           — Get lock info
```

## Workflow Actions (19 types)

```
Navigate { url }
Click { selector }
Type { selector, text }
WaitForSelector { selector, timeout_ms }
Screenshot { path?, full_page? }
Evaluate { js }
Extract { selector, attribute? }
SmartClick { query }
SmartFill { query, value }
Sleep { ms }
SetVariable { name, value }
Log { message, level? }
Assert { condition, message? }
Loop { items, variable, steps }
Conditional { condition, then_steps, else_steps? }
SubWorkflow { path }
HttpRequest { url, method?, headers?, body? }
Snapshot { compact?, interactive_only? }
Agent { prompt, options? }
```

## Execution Features

- ✅ **Sequential steps** with loop
- ✅ **Variable interpolation** `${variable}` → resolved before execution
- ✅ **Conditional execution** — step skipped if condition=false
- ✅ **Retry logic** — max_attempts = 1 + step.retries with delay
- ✅ **Error handling** — Stop/Continue/Retry/Skip per step
- ✅ **Variable capture** — save_as binding
- ✅ **Agent pause** — Pauses workflow at Agent action
- ✅ **Multi-instance** — O(1) tab→instance lookup
- ✅ **Multi-tab** — Per-instance HashMap<tab_id, Page>
- ✅ **Multi-agent safety** — Per-tab locks with TTL

## Action Dispatch Patterns (2 levels)

### Level 1: Workflow Engine (execute_step)
```
Action enum → match on type → CDP operations
e.g., Action::Click → element::click(page, selector)
```

### Level 2: Server API (execute_action)
```
Action enum → match on kind → snapshot-based ref_ids
e.g., Action::Click { ref_id: "e5" } → click_by_index_js(5)
```

## For Multi-Device Orchestration: Missing Pieces

1. **No DeviceClient trait** — Can't dispatch to Android/iOS generically
2. **No device registry** — ServerState lacks HashMap<device_id, Arc<dyn DeviceClient>>
3. **No multi-device workflow actions** — No DeviceNavigate, DeviceTap, etc.
4. **No device routes** — No /devices/* endpoints
5. **No parallel execution** — Workflows are sequential; no concurrent device control

## Implementation Entry Points

### To add trait abstraction:
- Create `crates/onecrawl-cdp/src/device.rs` with `DeviceClient` trait
- Impl trait for `AndroidClient`, `IosClient`, `BrowserSession`

### To add device state:
- Extend `ServerState` in `crates/onecrawl-server/src/state.rs`
- Add `devices: HashMap<device_id, Arc<dyn DeviceClient>>`
- Add `device_orchestrations: HashMap<orchestration_id, Vec<device_id>>`

### To add multi-device actions:
- Extend `Action` enum in `crates/onecrawl-cdp/src/workflow.rs`
- Add `DeviceNavigate { device_id, url }`, etc.

### To add routes:
- Create `crates/onecrawl-server/src/routes/devices.rs`
- Add handlers for device registration, lifecycle, execution

## Example: Multi-Device Workflow JSON

```json
{
  "name": "Cross-Device Login Test",
  "steps": [
    {
      "name": "Login on Android",
      "action": {
        "type": "device_navigate",
        "device_id": "android_dev1",
        "url": "https://example.com/login"
      }
    },
    {
      "name": "Verify on iOS",
      "action": {
        "type": "device_navigate",
        "device_id": "ios_dev2",
        "url": "https://example.com/dashboard"
      }
    },
    {
      "name": "Orchestration (parallel)",
      "action": {
        "type": "orchestration",
        "device_ids": ["android_dev1", "ios_dev2"],
        "parallel": true,
        "steps": [
          { "action": { "type": "screenshot", "device_id": "android_dev1" } },
          { "action": { "type": "screenshot", "device_id": "ios_dev2" } }
        ]
      }
    }
  ]
}
```

(Note: This structure doesn't exist yet — it's what needs to be built)
