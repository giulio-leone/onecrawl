# Durable Sessions Implementation Checklist

## Phase 1: Foundation (CLI + Basic Commands)

### CLI Layer
- [ ] Create `crates/onecrawl-cli-rs/src/cli/durable.rs`
  - [ ] Define `DurableAction` enum with: Create, Restore, List, Delete, Status
  - [ ] Add examples in docstrings

- [ ] Update `crates/onecrawl-cli-rs/src/cli/mod.rs`
  - [ ] Add `mod durable;` and `pub use durable::DurableAction;`
  - [ ] Add `Durable { #[command(subcommand)] action: DurableAction }` to Commands enum

- [ ] Update `crates/onecrawl-cli-rs/src/dispatch.rs`
  - [ ] Add match arm: `Commands::Durable { action } => commands::durable::handle(action).await,`

### Handler Layer
- [ ] Create `crates/onecrawl-cli-rs/src/commands/durable/mod.rs`
  - [ ] Implement `async fn handle(action: DurableAction)`

- [ ] Create phase checklist for storage, daemon, MCP integration

## Recommended Reading Order

1. **Quick Start**: QUICK_REFERENCE.md (7.5 KB)
2. **Deep Dive**: DURABLE_SESSIONS_REPORT.md (18 KB)
3. **Implementation**: This checklist

## Key Infrastructure Already Present

✅ Daemon with multi-session support
✅ Health monitoring + watchdog
✅ Checkpoint save/restore (harness.rs)
✅ Auth state persistence (~/.onecrawl/auth-states/)
✅ CLI dispatch pattern (100+ commands)
✅ MCP handler pattern with safety enforcement
✅ CircuitBreaker for failure tracking
✅ Session metadata persistence

## What to Implement

1. `onecrawl durable create {name}` - Create named session
2. `onecrawl durable restore {name}` - Restore from checkpoint
3. `onecrawl durable list` - Show all sessions
4. `onecrawl durable delete {name}` - Remove session
5. MCP handlers for programmatic use
6. Auto-checkpoint on daemon idle
7. Session history/versioning

