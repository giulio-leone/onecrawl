# Durable Sessions Skill

## Overview

Durable Sessions provide persistent browser sessions with automatic checkpointing, crash recovery, and state resumption. Sessions survive process restarts by serializing browser state (URL, cookies, localStorage, sessionStorage, scroll position, viewport) to disk.

## Key Files

- `crates/onecrawl-cdp/src/durable.rs` — Core `DurableSession` engine with checkpoint/restore logic
- `crates/onecrawl-mcp-rs/src/handlers/durable.rs` — 8 MCP action handlers
- `crates/onecrawl-cli-rs/src/commands/session/` — CLI session management commands

## API Reference

### MCP Actions

| Action | Description | Parameters |
|--------|-------------|------------|
| `durable_start` | Start a new durable session with monitoring loop | `name`, `on_crash?` (restart/stop/notify), `checkpoint_interval_secs?`, `state_path?`, `auto_reconnect?`, `max_reconnect_attempts?`, `max_uptime_secs?`, `persist_auth?` |
| `durable_stop` | Stop session with final checkpoint | `name` |
| `durable_checkpoint` | Manually trigger state checkpoint | `name` |
| `durable_restore` | Restore session from saved checkpoint | `name` |
| `durable_status` | Get session status (defaults to "default") | `name?` |
| `durable_list` | List all saved sessions | _(none)_ |
| `durable_delete` | Delete a saved session state file | `name` |
| `durable_config` | Update session configuration at runtime | `name`, `checkpoint_interval_secs?`, `auto_reconnect?`, `on_crash?`, `max_uptime_secs?` |

### CLI Commands

| Command | Description |
|---------|-------------|
| `onecrawl session start` | Start a durable browser session |
| `onecrawl session stop` | Stop and checkpoint a session |
| `onecrawl session status` | Show session status |
| `onecrawl session list` | List all saved sessions |

### Core Rust API

```rust
use onecrawl_cdp::{DurableSession, DurableConfig, CrashPolicy, DurableStatus};

// Create session with defaults
let config = DurableConfig {
    name: "my-session".into(),
    checkpoint_interval_secs: 30,
    auto_reconnect: true,
    max_reconnect_attempts: 10,
    reconnect_delay_secs: 2,
    on_crash: CrashPolicy::Restart,
    persist_auth: true,
    persist_scroll: true,
    persist_url: true,
    state_path: DurableSession::default_state_dir(),
    max_uptime_secs: None,
};
let mut session = DurableSession::new(config)?;

// Checkpoint current state
let state = session.checkpoint(&page).await?;

// Restore from checkpoint
session.restore(&page).await?;

// Start monitoring loop (auto-checkpoint + health check)
session.start_loop(Arc::new(page)).await?;

// List/delete sessions
let sessions = DurableSession::list_sessions(&state_dir)?;
DurableSession::delete_session(&state_dir, "old-session")?;
```

## Architecture

### State Persistence

State is saved as JSON files in `~/.onecrawl/states/{name}.json`. Each checkpoint captures:

- **URL** — Current page URL
- **Cookies** — Full cookie jar as JSON
- **localStorage** — All key-value pairs
- **sessionStorage** — All key-value pairs
- **Scroll position** — (x, y) coordinates
- **Viewport** — (width, height)
- **CDP URL** — WebSocket debug URL for reconnection
- **Metadata** — Created/checkpoint timestamps, uptime, reconnect count

### Monitoring Loop

`start_loop()` runs a background task that:
1. Checkpoints state every `checkpoint_interval_secs` (default: 30s)
2. Monitors browser health via CDP connection
3. On crash: applies `CrashPolicy` (Restart / Stop / Notify)
4. Tracks reconnection attempts up to `max_reconnect_attempts`
5. Enforces optional `max_uptime_secs` limit

### Enums

- **CrashPolicy**: `Restart` | `Stop` | `Notify`
- **DurableStatus**: `Running` | `Paused` | `Crashed` | `Reconnecting` | `Stopped` | `Checkpointing`

## Best Practices

- Use descriptive session `name` values (e.g., `linkedin-scraper`) — they become filenames
- Set `checkpoint_interval_secs` based on data volatility: 10–30s for auth-heavy flows, 60–120s for stable sessions
- Enable `persist_auth: true` (default) to preserve login state across restarts
- Use `max_uptime_secs` for long-running sessions to force periodic clean restarts
- Combine with the Vault skill to store credentials used during session restoration

## Troubleshooting

| Issue | Cause | Solution |
|-------|-------|----------|
| State file not found | Session name mismatch or deleted | Check `durable_list` output; verify `~/.onecrawl/states/` |
| Restore fails silently | Cookies from different domain | Ensure checkpoint URL matches restore target |
| Reconnection loop | Browser process fully terminated | Increase `reconnect_delay_secs`; check CDP port availability |
| Checkpoint too slow | Large localStorage/sessionStorage | Reduce stored data or increase checkpoint interval |
| `max_reconnect_attempts` exceeded | Persistent browser crash | Check system resources; use `CrashPolicy::Notify` to investigate |
