# Event Reactor Skill

## Overview

The Event Reactor is a real-time event monitoring and response engine. It watches browser events (DOM mutations, network requests, console messages, navigation, WebSocket frames) and dispatches handlers when rules match. Supports persistent observers, AI-driven responses, chained handlers, rate limiting, and event journaling.

## Key Files

- `crates/onecrawl-cdp/src/reactor.rs` — Core `Reactor` engine with rule matching and handler dispatch
- `crates/onecrawl-mcp-rs/src/handlers/reactor.rs` — 8 MCP action handlers
- `crates/onecrawl-cli-rs/src/commands/` — CLI commands (via browser agent integration)

## API Reference

### MCP Actions

| Action | Description | Parameters |
|--------|-------------|------------|
| `reactor_start` | Start reactor with rules and configuration | `name`, `rules[]` (each with `id`, `event_type`, `filter?`, `handler`, `enabled?`, `max_triggers?`, `cooldown_ms?`), `max_events_per_minute?`, `buffer_size?`, `persist_events?`, `event_log_path?` |
| `reactor_stop` | Stop the running reactor | _(none)_ |
| `reactor_status` | Get reactor status with rule statistics | _(none)_ |
| `reactor_add_rule` | Add a rule at runtime | `id`, `event_type`, `filter?`, `handler`, `enabled?`, `max_triggers?`, `cooldown_ms?` |
| `reactor_remove_rule` | Remove a rule by ID | `rule_id` |
| `reactor_toggle_rule` | Enable or disable a rule | `rule_id`, `enabled` |
| `reactor_events` | Get recent matched events | `limit?` (default: 50) |
| `reactor_clear` | Clear event history | _(none)_ |

### Event Types

| Type | Description | Filter Field |
|------|-------------|--------------|
| `dom_mutation` | DOM element added/removed/modified | `selector` (CSS) |
| `network_request` | Outgoing HTTP request | `url_pattern` (glob) |
| `network_response` | Incoming HTTP response | `url_pattern` (glob) |
| `console` | Console log/warn/error | `message_pattern` (substring) |
| `page_error` | Uncaught JavaScript error | `message_pattern` |
| `navigation` | Page navigation event | _(none)_ |
| `notification` | Browser notification | `message_pattern` |
| `websocket` | WebSocket frame sent/received | `url_pattern` |
| `timer` | Periodic timer event | _(none)_ |
| `custom(name)` | User-defined event type | _(any filter)_ |

### Handler Types

| Handler | Description | Fields |
|---------|-------------|--------|
| `log` | Format and output event data | `format?`, `output?` (stdout/file path) |
| `evaluate` | Execute JavaScript on the page | `script` |
| `webhook` | Send HTTP request to external URL | `url`, `method?`, `headers?` |
| `command` | Execute shell command (disabled for safety) | `cmd`, `args[]` |
| `screenshot` | Capture page screenshot | `path?` |
| `ai_respond` | Send to AI model for decision | `model?`, `prompt`, `max_tokens?`, `actions?` |
| `chain` | Execute multiple handlers sequentially | `handlers[]` |
| `store` | Persist event data to file | `path` |

### CLI Commands

| Command | Description |
|---------|-------------|
| `onecrawl react start` | Start reactor with rules from config |
| `onecrawl react stop` | Stop the running reactor |
| `onecrawl react add-rule` | Add a rule at runtime |
| `onecrawl react status` | Show reactor status and rule statistics |

### Core Rust API

```rust
use onecrawl_cdp::{Reactor, ReactorConfig, ReactorRule, ReactorEventType, ReactorHandler, EventFilter};

let config = ReactorConfig {
    name: "my-reactor".into(),
    rules: vec![
        ReactorRule {
            id: "catch-errors".into(),
            event_type: ReactorEventType::Console,
            filter: Some(EventFilter {
                message_pattern: Some("error".into()),
                event_subtype: Some("error".into()),
                ..Default::default()
            }),
            handler: ReactorHandler::Screenshot { path: Some("error.png".into()) },
            enabled: true,
            max_triggers: Some(10),
            cooldown_ms: Some(5000),
            trigger_count: 0,
        },
    ],
    max_events_per_minute: Some(60),
    buffer_size: Some(1000),
    persist_events: false,
    event_log_path: None,
};

let reactor = Reactor::new(config);
reactor.start(&page).await?;

// Runtime rule management
reactor.add_rule(new_rule).await?;
reactor.toggle_rule("catch-errors", false).await?;
reactor.remove_rule("catch-errors").await?;

// Status and events
let status = reactor.status().await;
let events = reactor.recent_events(50).await;
reactor.clear_events().await;

// Stop reactor
let final_status = reactor.stop().await?;
```

## Architecture

### Event Loop

The reactor polls multiple event sources in a continuous loop:

1. **DOM Mutations** — Via `MutationObserver` JavaScript injection (filtered by CSS selectors)
2. **Console Messages** — Via CDP `Runtime.consoleAPICalled` (filtered by message pattern/subtype)
3. **Network Events** — Via CDP `Network.requestWillBeSent` / `responseReceived` (filtered by URL glob)
4. **Navigation** — Via CDP `Page.frameNavigated`
5. **WebSocket** — Via CDP `Network.webSocketFrameReceived/Sent`

### Rate Limiting

- **Global**: `max_events_per_minute` (default: 60 EPM)
- **Per-rule cooldown**: `cooldown_ms` — minimum time between triggers
- **Per-rule max triggers**: `max_triggers` — auto-disables after N triggers

### Event Buffer

- Default buffer: 1000 events (max: 10,000)
- Optional disk persistence via `event_log_path`
- Circular buffer evicts oldest events

### Handler Dispatch

Handlers execute asynchronously. The `Chain` handler enables composing multiple actions:

```json
{
  "type": "chain",
  "handlers": [
    { "type": "screenshot", "path": "before.png" },
    { "type": "evaluate", "script": "location.reload()" },
    { "type": "log", "format": "Reloaded after error" }
  ]
}
```

## Best Practices

- Use specific `event_subtype` filters (e.g., `"error"` for console) to avoid noise
- Set `cooldown_ms` on high-frequency rules to prevent handler flooding
- Use `max_triggers` as a safety net for rules that should fire a limited number of times
- Prefer `chain` handlers over multiple rules for correlated actions
- Enable `persist_events` only when you need post-mortem analysis — it has I/O overhead
- Keep `buffer_size` reasonable (1000–5000) for memory efficiency

## Troubleshooting

| Issue | Cause | Solution |
|-------|-------|----------|
| No events captured | Reactor not started or no matching rules | Check `reactor_status`; verify event types and filters |
| Events but no triggers | Filter too restrictive | Relax `selector`, `url_pattern`, or `message_pattern` |
| Handler not firing | Rule disabled or `max_triggers` reached | Check `trigger_count` in status; re-enable with `reactor_toggle_rule` |
| High memory usage | Large buffer + many events | Reduce `buffer_size` or enable `persist_events` to offload to disk |
| Rate limiting active | `max_events_per_minute` exceeded | Increase limit or add per-rule `cooldown_ms` |
| Webhook handler timeout | Target URL slow or unreachable | Check URL accessibility; webhook timeout is 5s default |
