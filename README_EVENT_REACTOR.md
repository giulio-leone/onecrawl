# Event Reactor Feature — Implementation Guide

## Overview

Complete reference documentation for implementing the **Event Reactor** feature in OneCrawl's Rust CDP module. This guide covers all 9 key files with full function signatures, type definitions, and integration patterns.

**Generated for:** `/packages/onecrawl-rust/crates/onecrawl-cdp/`

---

## 📚 Documentation Files

### 1. **EVENT_REACTOR_REFERENCE.md** (30 KB)
**Full Technical Reference** — Start here for comprehensive details.

- 1,071 lines of detailed documentation
- Complete function signatures with parameter types and return types
- All struct/enum definitions with all fields
- Use/import statements for each module
- Implementation patterns and checklist
- 9 file sections:
  - lib.rs (module registration)
  - events.rs (EventStream, BrowserEvent)
  - dom_observer.rs (DOM mutation tracking)
  - page_watcher.rs (SPA navigation, title, scroll, resize)
  - websocket.rs (WebSocket frame recording)
  - intercept.rs (request interception & mocking)
  - screencast.rs (CDP screenshot/screencast)
  - agent.rs (autonomous agent loop)
  - Cargo.toml (dependencies)

**When to use:** You need complete implementation details, all available functions, or to understand the full architecture.

---

### 2. **EVENT_REACTOR_QUICK_REFERENCE.md** (9.3 KB)
**Quick-Lookup Implementation Guide** — Start here for quick patterns.

- Function signatures organized by topic
- Copy-paste ready code examples
- Core types with all variants and fields
- **Complete integration pattern** (ready-to-use code)
- Key dependencies list
- Storage locations (window.__onecrawl_* objects)

**When to use:** You need a quick lookup, want to see example patterns, or need copy-paste ready code.

---

### 3. **EVENT_REACTOR_FILES_INDEX.md** (13 KB)
**File Structure & Contents Index** — Overview and summary.

- Summary of each of 9 files
- Data flow diagram
- Integration checklist
- File locations
- Implementation guide cross-reference

**When to use:** You want to understand file organization, see what's in each module, or need an overview before diving deep.

---

### 4. **IMPLEMENTATION_SUMMARY.txt** (10 KB)
**Quick Reference Summary** — At-a-glance checklist.

- Deliverables list
- Files covered with line counts
- Key types and structures
- Common usage patterns
- Storage locations
- Next steps for implementation
- Document locations and cross-references

**When to use:** You want a quick overview or checklist before starting implementation.

---

## 🎯 Quick Start

### For Different Needs:

**"I want to get started right now!"**
→ Read `EVENT_REACTOR_QUICK_REFERENCE.md` → Copy the "Essential Integration Pattern" code

**"I need complete technical details"**
→ Read `EVENT_REACTOR_REFERENCE.md` → Section by section

**"I want an overview first"**
→ Read `IMPLEMENTATION_SUMMARY.txt` or `EVENT_REACTOR_FILES_INDEX.md`

**"I need to look up a specific function"**
→ Use `EVENT_REACTOR_QUICK_REFERENCE.md` for quick lookup, or grep through `EVENT_REACTOR_REFERENCE.md` for details

---

## 📋 9 Files Covered

| # | File | Lines | Purpose |
|---|------|-------|---------|
| 1 | **lib.rs** | 187 | Module registry (48 modules) & 130+ re-exports |
| 2 | **events.rs** | 226 | EventStream, BrowserEvent, event broadcasting |
| 3 | **dom_observer.rs** | 128 | DOM mutation tracking via JS MutationObserver |
| 4 | **page_watcher.rs** | 221 | SPA navigation, title, scroll, resize detection |
| 5 | **websocket.rs** | 173 | WebSocket frame recording & playback |
| 6 | **intercept.rs** | 163 | Request interception & response mocking |
| 7 | **screencast.rs** | 155 | CDP screenshot & live screencast streaming |
| 8 | **agent.rs** | 600+ | Autonomous agent loop & observation |
| 9 | **Cargo.toml** | 34 | Dependencies (chromiumoxide, tokio, serde) |

---

## 🔄 Core Integration Pattern

```rust
// 1. Create event stream
let event_stream = EventStream::new(128);
let tx = event_stream.sender();

// 2. Install observers
start_dom_observer(&page, Some("body")).await?;
start_page_watcher(&page).await?;

// 3. Spawn periodic drain task
tokio::spawn({
    let page = page.clone();
    let tx = tx.clone();
    async move {
        loop {
            tokio::time::sleep(Duration::from_millis(500)).await;
            
            // Drain and emit events
            if let Ok(mutations) = drain_dom_mutations(&page).await {
                for m in mutations {
                    let _ = tx.send(BrowserEvent {
                        event_type: EventType::Custom("dom_mutation".to_string()),
                        timestamp: m.timestamp,
                        data: serde_json::to_value(&m)?,
                    });
                }
            }
        }
    }
});

// 4. Subscribe to events
let mut rx = event_stream.subscribe();
tokio::spawn(async move {
    while let Ok(event) = rx.recv().await {
        // Send to client via WebSocket/SSE
        println!("{}", format_sse(&event));
    }
});

// 5. Cleanup on page close
stop_dom_observer(&page).await?;
stop_page_watcher(&page).await?;
```

---

## 🎪 Key Types Overview

### Event System
- `EventStream` — pub/sub broadcast channel
- `BrowserEvent` — event payload (type, timestamp, data)
- `EventType` enum — 9 variants + Custom

### DOM & Page Tracking
- `DomMutation` — DOM change record
- `PageChange` — navigation/title/scroll/resize change

### Network & Interception
- `InterceptRule` — URL pattern + action
- `InterceptAction` enum — Block | Modify | MockResponse

### WebSocket
- `WsFrame` — captured frame (URL, direction, opcode, payload)
- `WsRecorder` — Arc<Mutex> frame collection

### Screencast
- `ScreencastOptions` — format, quality, dimensions
- `StreamResult` — capture metadata

### Agent & Observation
- `agent_loop()` — multi-step goal execution
- `annotated_observe()` — page state with element coordinates
- `goal_assert()` — semantic verification

---

## 🔗 File Locations

**Base path:** `/Users/giulioleone/Sviluppo/onecrawl-dev/`

**Documentation files:**
- `EVENT_REACTOR_REFERENCE.md`
- `EVENT_REACTOR_QUICK_REFERENCE.md`
- `EVENT_REACTOR_FILES_INDEX.md`
- `IMPLEMENTATION_SUMMARY.txt`
- `README_EVENT_REACTOR.md` (this file)

**Source files:**
- `packages/onecrawl-rust/crates/onecrawl-cdp/src/lib.rs`
- `packages/onecrawl-rust/crates/onecrawl-cdp/src/events.rs`
- `packages/onecrawl-rust/crates/onecrawl-cdp/src/dom_observer.rs`
- `packages/onecrawl-rust/crates/onecrawl-cdp/src/page_watcher.rs`
- `packages/onecrawl-rust/crates/onecrawl-cdp/src/websocket.rs`
- `packages/onecrawl-rust/crates/onecrawl-cdp/src/intercept.rs`
- `packages/onecrawl-rust/crates/onecrawl-cdp/src/screencast.rs`
- `packages/onecrawl-rust/crates/onecrawl-cdp/src/agent.rs`
- `packages/onecrawl-rust/crates/onecrawl-cdp/Cargo.toml`

---

## 📦 Dependencies Summary

**Core:**
- `chromiumoxide = "0.5"` — CDP client
- `tokio` (full features) — Async runtime
- `serde` + `serde_json` — Serialization
- `base64` — Frame encoding
- `futures` — Async utilities
- `tracing` — Logging

**Features:**
- `default = []` (none enabled by default)
- `playwright` (optional, for Playwright backend)

---

## ✅ Implementation Checklist

- [ ] Read one of the documentation files (start with QUICK_REFERENCE.md)
- [ ] Understand EventStream and broadcast pattern
- [ ] Create EventStream instance with capacity
- [ ] Install observers (dom_observer, page_watcher, console, errors, websocket)
- [ ] Spawn periodic drain task (500ms intervals)
- [ ] Convert drained data to BrowserEvent
- [ ] Setup subscriber(s) for broadcasting
- [ ] Setup WebSocket/SSE transport for events
- [ ] Implement cleanup functions (stop_*)
- [ ] Test with actual page navigation and interactions
- [ ] Deploy with tokio async runtime

---

## 🚀 Next Steps

1. **Choose your starting point:**
   - Quick start → `EVENT_REACTOR_QUICK_REFERENCE.md`
   - Deep dive → `EVENT_REACTOR_REFERENCE.md`
   - Overview → `IMPLEMENTATION_SUMMARY.txt`

2. **Copy the integration pattern** from QUICK_REFERENCE.md

3. **Adapt to your architecture:**
   - WebSocket transport?
   - SSE (Server-Sent Events)?
   - File output?
   - Database storage?

4. **Test with chromiumoxide Page instance**

5. **Deploy with error handling and monitoring**

---

## 💡 Common Patterns

### Basic Event Streaming
```rust
let stream = EventStream::new(128);
let tx = stream.sender();
let mut rx = stream.subscribe();
```

### DOM Observation
```rust
start_dom_observer(&page, None).await?;
let mutations = drain_dom_mutations(&page).await?;
```

### Page State Watching
```rust
start_page_watcher(&page).await?;
let changes = drain_page_changes(&page).await?;
```

### Request Interception
```rust
let rules = vec![
    InterceptRule {
        url_pattern: "*api/*".to_string(),
        resource_type: None,
        action: InterceptAction::Block,
    },
];
set_intercept_rules(&page, rules).await?;
```

### Screencast
```rust
let opts = ScreencastOptions::default();
capture_frames_burst(&page, &opts, 10, 100).await?;
```

---

## 📞 Need Help?

1. **Function not found?** → Search in `EVENT_REACTOR_REFERENCE.md`
2. **Need quick syntax?** → Check `EVENT_REACTOR_QUICK_REFERENCE.md`
3. **Want to understand flow?** → Read `EVENT_REACTOR_FILES_INDEX.md`
4. **Have a checklist?** → Use `IMPLEMENTATION_SUMMARY.txt`

---

## 📝 Document Information

- **Total documentation:** 62 KB across 4 files + this README
- **Total code lines referenced:** 1,800+ lines
- **Functions documented:** 40+
- **Types documented:** 15+
- **Examples provided:** 8+ complete patterns

---

**Last Updated:** 2024
**Project:** OneCrawl Rust CDP
**Version:** 3.8.0
