# Durable Sessions Infrastructure — Complete Index

## 📖 Documentation Files

This analysis provides three complementary documents to understand and implement durable sessions:

### 1. **QUICK_REFERENCE.md** (7.5 KB) — START HERE
   - **Purpose**: Implementation cheat sheet while coding
   - **Read Time**: 15 minutes
   - **Best For**: Developers ready to write code
   - **Contains**:
     - File locations (9 critical files)
     - Struct reference (6 must-understand)
     - Daemon architecture diagram
     - JSON format examples
     - 4-step patterns for adding CLI commands
     - 4-step patterns for adding MCP handlers
     - Testing checklist

   **Use This When**: You're writing code and need to remember:
   - Where to put files
   - What structs to use
   - How to follow the pattern
   - What error handling looks like

### 2. **DURABLE_SESSIONS_REPORT.md** (18 KB) — COMPREHENSIVE ANALYSIS
   - **Purpose**: Deep technical understanding of entire system
   - **Read Time**: 45 minutes
   - **Best For**: Architects, system designers, code reviewers
   - **Contains**:
     - Section 1: Daemon module (daemon, harness, protocol, client)
     - Section 2: Harness module (health check, circuit breaker, checkpoint, watchdog)
     - Section 3: Session/state management (SessionInfo, BrowserSession, ServerState)
     - Section 4: Checkpoint system (save flow, restore flow, format)
     - Section 5: Auth state persistence (functions, format, storage)
     - Section 6: CLI dispatch pattern (structure, flow, adding commands)
     - Section 7: MCP handler pattern (server structure, handler pattern, registration)
     - Implementation roadmap with 5 phases

   **Use This When**: You need to:
   - Understand how each component works
   - See code examples from real codebase
   - Know which structs and methods do what
   - Plan architectural changes

### 3. **DURABLE_SESSIONS_CHECKLIST.md** (1.7 KB) — IMPLEMENTATION ROADMAP
   - **Purpose**: Track implementation progress
   - **Read Time**: 5 minutes
   - **Best For**: Project managers, developers tracking progress
   - **Contains**:
     - Phase 1: Foundation (CLI + basic commands)
     - Quick summary of what's already built
     - What still needs implementation
     - Phases 2-7 overview (detailed in full report)

   **Use This When**: You need to:
   - Know what's already done
   - See what phases remain
   - Track your progress
   - Understand implementation scope

---

## 🗺️ Key Infrastructure Map

### Already Implemented ✅

```
┌─ Daemon (multi-session) ─────────────────────────┐
│  ✅ Unix socket IPC                              │
│  ✅ Session map: HashMap<name, (Browser, Page)>  │
│  ✅ Health monitor: every 60s                    │
│  ✅ Auto-removes dead sessions                   │
│  ✅ State persistence: /tmp/onecrawl-daemon-*.* │
│  ✅ Idle timeout: 30 min auto-shutdown           │
│  ✅ Signal handling: SIGTERM/SIGINT              │
└──────────────────────────────────────────────────┘

┌─ Harness (health + recovery) ───────────────────┐
│  ✅ health_check() → memory, timing, responsive │
│  ✅ checkpoint_save() → JSON snapshot            │
│  ✅ checkpoint_restore() → browser state         │
│  ✅ reconnect_cdp() → auto-reconnect backoff    │
│  ✅ watchdog_status() → liveness test            │
│  ✅ CircuitBreaker → failure tracking            │
└──────────────────────────────────────────────────┘

┌─ Session Management ────────────────────────────┐
│  ✅ SessionInfo struct (metadata tracking)      │
│  ✅ BrowserSession (launch + connect)            │
│  ✅ ServerState (registry + locks)               │
│  ✅ TabLock (multi-agent safety)                 │
│  ✅ Auth state save/load (~/.onecrawl/...)      │
└──────────────────────────────────────────────────┘

┌─ CLI + Dispatch ────────────────────────────────┐
│  ✅ Clap-based command parsing                  │
│  ✅ 100+ command examples                       │
│  ✅ Clear dispatch pattern                      │
│  ✅ Session command module                      │
│  ✅ Daemon command module                       │
└──────────────────────────────────────────────────┘

┌─ MCP Integration ───────────────────────────────┐
│  ✅ #[tool_router] macro registration            │
│  ✅ 100+ handler examples (browser.rs)          │
│  ✅ Safety policy enforcement                   │
│  ✅ Strong type parameter validation            │
│  ✅ Consistent error handling                   │
└──────────────────────────────────────────────────┘
```

### Still Needed ❌

```
┌─ Durable Session Layer ─────────────────────────┐
│  ❌ Named durable session CLI subcommand        │
│  ❌ Session directory structure                 │
│  ❌ Session metadata persistence                │
│  ❌ Create/restore/list/delete commands         │
│  ❌ MCP handlers for durable operations         │
│  ❌ Auto-checkpoint on idle                     │
│  ❌ Session history/versioning                  │
└──────────────────────────────────────────────────┘
```

---

## 🚀 Quick Start (5 minutes)

1. **Read QUICK_REFERENCE.md** to understand the patterns
2. **Look at these 3 files to see examples**:
   - `crates/onecrawl-cli-rs/src/cli/daemon.rs` — How CLI subcommand enums work
   - `crates/onecrawl-cli-rs/src/dispatch.rs` — How commands are routed (lines 1-100)
   - `crates/onecrawl-cli-rs/src/commands/browser/media/auth_state.rs` — How to persist state
3. **Start implementing Phase 1**: Create `crates/onecrawl-cli-rs/src/cli/durable.rs`

---

## 📊 Line Count Summary

| Component | Location | Lines |
|-----------|----------|-------|
| Daemon server loop | daemon/server.rs | 625 |
| Harness (health + checkpoint) | crates/onecrawl-cdp/src/harness.rs | 392 |
| Browser session | crates/onecrawl-cdp/src/browser.rs | 137 |
| CLI dispatch routing | crates/onecrawl-cli-rs/src/dispatch.rs | 1800+ |
| Auth state | commands/browser/media/auth_state.rs | 143 |
| Server state | crates/onecrawl-server/src/state.rs | 100+ |
| MCP server | crates/onecrawl-mcp-rs/src/server.rs | 1810 |
| MCP browser handlers | crates/onecrawl-mcp-rs/src/handlers/browser.rs | 1000+ |

**Total analyzed: ~6000 lines of relevant infrastructure**

---

## 🎯 Next Steps by Role

### For Architects/Leads:
1. Read DURABLE_SESSIONS_REPORT.md (full 18 KB)
2. Review QUICK_REFERENCE.md for patterns
3. Plan Phase 2-7 in DURABLE_SESSIONS_CHECKLIST.md
4. Decide on storage backend (filesystem vs encrypted store)

### For Implementation Developers:
1. Read QUICK_REFERENCE.md (15 min)
2. Clone the pattern: `crates/onecrawl-cli-rs/src/cli/daemon.rs`
3. Create: `crates/onecrawl-cli-rs/src/cli/durable.rs`
4. Implement Phase 1 from CHECKLIST
5. Reference existing code when unsure (patterns are proven)

### For Code Reviewers:
1. Check QUICK_REFERENCE.md for expected patterns
2. Verify DURABLE_SESSIONS_REPORT.md section matches implementation
3. Use testing checklist from QUICK_REFERENCE.md

### For Testers/QA:
1. Use testing checklist in QUICK_REFERENCE.md
2. Follow checkpoint format in QUICK_REFERENCE.md
3. Reference daemon commands in QUICK_REFERENCE.md

---

## 🔍 Finding Code Examples

### CLI Pattern Example
**File**: `crates/onecrawl-cli-rs/src/cli/daemon.rs` (lines 1-34)
```rust
#[derive(Subcommand)]
pub enum DaemonAction {
    Start { headless: bool },
    Stop,
    Status,
    Exec { command: String, args: Vec<String>, session: Option<String> },
    Run { headless: bool },
}
```

### Dispatch Pattern Example
**File**: `crates/onecrawl-cli-rs/src/dispatch.rs` (lines 1-30)
```rust
pub(crate) async fn dispatch(command: Commands) {
    match command {
        Commands::Daemon { action } => match action {
            DaemonAction::Start { headless } => commands::daemon::daemon_start(headless).await,
            // ... other arms
        },
        // ... other commands
    }
}
```

### Handler Pattern Example
**File**: `crates/onecrawl-cli-rs/src/commands/browser/media/auth_state.rs` (lines 16-41)
```rust
pub async fn auth_state_save(name: &str) {
    with_page(|page| async move {
        // Evaluate JS to capture state
        // Write to ~/.onecrawl/auth-states/{name}.json
        // Return success message
    }).await;
}
```

### MCP Handler Pattern Example
**File**: `crates/onecrawl-mcp-rs/src/handlers/browser.rs` (lines 16-43)
```rust
impl OneCrawlMcp {
    pub(crate) async fn navigation_goto(
        &self,
        p: NavigateParams,
    ) -> Result<CallToolResult, McpError> {
        // 1. Enforce safety
        // 2. Ensure page
        // 3. Call CDP function
        // 4. Gather response
        // 5. Return result
    }
}
```

---

## 💾 Key Files to Review (In Priority Order)

### Must Read (Foundation):
1. `crates/onecrawl-cli-rs/src/cli/daemon.rs` — Enum + subcommand pattern
2. `crates/onecrawl-cli-rs/src/commands/daemon/mod.rs` — Handler pattern
3. `crates/onecrawl-cli-rs/src/dispatch.rs` (first 100 lines) — Dispatch pattern

### Should Read (Context):
4. `crates/onecrawl-cdp/src/harness.rs` (lines 157-301) — Checkpoint pattern
5. `crates/onecrawl-cli-rs/src/commands/browser/media/auth_state.rs` — State persistence
6. `crates/onecrawl-cli-rs/src/commands/daemon/server.rs` (lines 1-100) — Daemon structure

### Reference (Optional):
7. `crates/onecrawl-mcp-rs/src/handlers/browser.rs` (first 100 lines) — MCP pattern
8. `crates/onecrawl-server/src/state.rs` (first 100 lines) — Session state tracking

---

## ✅ Verification Checklist

After reading all documents, you should understand:

- [ ] What DaemonState contains
- [ ] How checkpoint_save() works (5 steps)
- [ ] How checkpoint_restore() works (6 steps)
- [ ] Where sessions are persisted
- [ ] How health_check() fails detection works
- [ ] What CircuitBreaker tracks
- [ ] How to add a new CLI subcommand (4 steps)
- [ ] How to add an MCP handler (4 steps)
- [ ] What the session directory structure should be
- [ ] How named sessions are routed in daemon
- [ ] What state needs to persist across crashes
- [ ] How auth state differs from checkpoint
- [ ] Why TabLock is needed
- [ ] How idle timeout works
- [ ] Where all temporary files are stored

If you can answer these, you're ready to implement!

---

## 📞 Quick Reference by Use Case

**"How do I add a new CLI command?"**
→ QUICK_REFERENCE.md § "Adding a New Durable Sessions Command"

**"What does health_check() return?"**
→ QUICK_REFERENCE.md § "Health Monitoring Details"

**"How are sessions stored?"**
→ DURABLE_SESSIONS_REPORT.md § "2. DAEMON MODULE"

**"What's the checkpoint format?"**
→ QUICK_REFERENCE.md § "Checkpoint Format"

**"How do I handle errors?"**
→ QUICK_REFERENCE.md § "Error Handling Patterns"

**"What needs to be tested?"**
→ QUICK_REFERENCE.md § "Testing Durable Sessions (Checklist)"

**"What files already exist for this?"**
→ DURABLE_SESSIONS_REPORT.md § "1. DAEMON MODULE" and "7. MCP HANDLER PATTERN"

**"What's still missing?"**
→ DURABLE_SESSIONS_CHECKLIST.md § "Phase 1-7"

---

## 📈 Implementation Stages

```
Stage 1: Setup (1 day)
  • Read all docs
  • Review examples
  • Plan Phase 1

Stage 2: CLI + Dispatch (2 days)
  • Create durable.rs enum
  • Add to Commands
  • Implement dispatch
  • Test --help

Stage 3: Handler Skeleton (1 day)
  • Create commands/durable/mod.rs
  • Implement basic handle function
  • Add storage module

Stage 4: Core Features (3-5 days)
  • Create, Restore, List, Delete
  • Directory structure
  • Metadata persistence

Stage 5: Daemon Integration (2 days)
  • Auto-checkpoint
  • Dead session recovery
  • State file updates

Stage 6: MCP Integration (2 days)
  • MCP handlers
  • Safety enforcement
  • Parameter validation

Stage 7: Testing + Polish (3 days)
  • Unit tests
  • Integration tests
  • Documentation
```

**Estimated Total: 2-3 weeks** for full implementation

---

## 🎓 Learning Resources in This Package

1. **Pattern Recognition**: See how daemon, session, auth-state work
2. **Code Examples**: Real implementations from 6000+ lines
3. **Struct Definitions**: All key types documented
4. **Error Handling**: Patterns for CLI, MCP, and daemon errors
5. **Testing Guide**: What to verify at each stage
6. **Implementation Checklist**: What to build and in what order

Use these documents to:
- Understand the system holistically
- Implement features confidently
- Review code consistently
- Test thoroughly
- Document your changes

---

**Last Updated**: 2024-03-07
**Status**: Infrastructure Analysis Complete — Ready for Implementation
**Effort Estimate**: 2-3 weeks for full durable sessions feature
