# Event Bus & Webhooks Skill

## Overview

The Event Bus provides a pub/sub messaging system with HMAC-signed webhook delivery, Server-Sent Events (SSE) streaming, event journaling, and replay capabilities. It enables decoupled communication between OneCrawl components and external systems via HTTP webhooks with automatic retry logic.

## Key Files

- `crates/onecrawl-cdp/src/event_bus.rs` — Core `EventBus` with pub/sub, webhooks, and SSE
- `crates/onecrawl-mcp-rs/src/handlers/events.rs` — 8 MCP action handlers
- `crates/onecrawl-cli-rs/src/commands/events.rs` — CLI event commands

## API Reference

### MCP Actions

| Action | Description | Parameters |
|--------|-------------|------------|
| `events_emit` | Emit an event to the bus | `event_type`, `source?` (default: "mcp"), `data?` (JSON), `metadata?` (key-value map) |
| `events_subscribe` | Subscribe a webhook to event pattern | `event_pattern` (glob), `url`, `method?` (POST), `headers?`, `secret?` (HMAC key), `retry_count?` (default: 3), `retry_delay_ms?` (default: 1000) |
| `events_unsubscribe` | Remove a webhook subscription | `id` |
| `events_list_subscriptions` | List all active webhook subscriptions | _(none)_ |
| `events_recent` | Get recent events from journal | `limit?` (default: 50) |
| `events_replay` | Replay events matching pattern since timestamp | `event_pattern` (glob), `since?` (ISO 8601) |
| `events_stats` | Get event bus statistics | _(none)_ |
| `events_clear` | Clear event journal | _(none)_ |

### CLI Commands

| Command | Description |
|---------|-------------|
| `onecrawl events listen <port>` | Start event bus listener on port |
| `onecrawl events emit <type> [data]` | Emit event (with `--source`) |
| `onecrawl events subscribe <pattern> <webhook>` | Subscribe webhook (`--secret` for HMAC) |
| `onecrawl events unsubscribe <id>` | Remove subscription |
| `onecrawl events list` | List active subscriptions |
| `onecrawl events recent [limit]` | Show recent events |
| `onecrawl events replay <pattern> [--since]` | Replay matching events |
| `onecrawl events stats` | Show bus statistics |
| `onecrawl events clear` | Clear event journal |

### Core Rust API

```rust
use onecrawl_cdp::{EventBus, EventBusConfig, BusEvent, WebhookSubscription};

let bus = EventBus::new(EventBusConfig {
    max_journal_size: 10_000,
    max_subscriptions: 100,
    webhook_timeout_ms: 5000,
    enable_journal: true,
    journal_path: None,
});

// Emit event
bus.emit(BusEvent {
    id: generate_id(),
    event_type: "page:loaded".into(),
    source: "crawler".into(),
    timestamp: iso_now(),
    data: serde_json::json!({"url": "https://example.com"}),
    metadata: None,
}).await?;

// Subscribe webhook
let sub_id = bus.subscribe_webhook(WebhookSubscription {
    id: String::new(),  // auto-generated
    event_pattern: "page:*".into(),
    url: "https://hooks.example.com/onecrawl".into(),
    secret: Some("my-hmac-key".into()),
    active: true,
    retry_count: 3,
    retry_delay_ms: 1000,
    ..Default::default()
}).await?;

// Real-time streaming
let mut rx = bus.subscribe_stream();
while let Ok(event) = rx.recv().await {
    println!("Event: {}", event.event_type);
}

// Replay events
let events = bus.replay("page:*", Some("2024-01-01T00:00:00Z")).await?;

// Statistics
let stats = bus.stats().await;
```

### Event Pattern Matching (Glob)

| Pattern | Matches | Example |
|---------|---------|---------|
| `page:loaded` | Exact match | `page:loaded` ✓ |
| `page:*` | Single segment wildcard | `page:loaded` ✓, `page:error` ✓ |
| `page:**` | Multi-segment wildcard | `page:loaded:fast` ✓ |
| `**` | Match all events | Everything ✓ |

### SSE Format

```
event: bus_event
data: {"id":"evt-1234","event_type":"page:loaded","source":"crawler","timestamp":"...","data":{}}
```

Use `format_bus_sse()` to produce SSE-formatted output.

## Architecture

### Event Flow

```
Emitter → EventBus → [Pattern Match] → Webhook Delivery
              ↓                              ↓
          Journal                    HMAC-SHA256 Signing
              ↓                              ↓
         Broadcast ──→ SSE Stream       HTTP POST + Retry
```

### Webhook Delivery

1. **Pattern Match**: Event type matched against subscription glob patterns
2. **HMAC Signing**: If `secret` is set, adds `X-Signature: sha256=<hex>` header
3. **HTTP Request**: POST (configurable method) with event JSON body
4. **Retry Logic**: Exponential backoff (`retry_delay_ms * attempt`), max 10 retries
5. **Error Tracking**: `last_error` and `trigger_count` updated per subscription

### SSRF Protection

Webhook URLs are validated to block:
- `localhost` / `127.0.0.1` / `::1`
- Cloud metadata endpoints (`169.254.169.254`)
- Private IP ranges (`10.x.x.x`, `172.16-31.x.x`, `192.168.x.x`)

### Journal

- Circular buffer with configurable max size (default: 10,000 entries)
- Each entry records: event, delivered-to list, per-subscription delivery status
- Supports time-based replay with ISO 8601 `since` parameter

### Configuration Defaults

| Setting | Default | Max |
|---------|---------|-----|
| `max_journal_size` | 10,000 | — |
| `max_subscriptions` | 100 | — |
| `webhook_timeout_ms` | 5,000 | — |
| `retry_count` | 3 | 10 |
| `retry_delay_ms` | 1,000 | — |

### Concurrency

All state is wrapped in `Arc<RwLock<T>>` for thread-safe concurrent access. The broadcast channel (`tokio::sync::broadcast`) enables zero-copy event streaming to multiple consumers.

## Best Practices

- Use hierarchical event types (`page:loaded`, `page:error`, `network:request`) for glob matching
- Set `secret` on webhook subscriptions for HMAC verification in receiving services
- Use `events_replay` for debugging — replay events after the fact with pattern filters
- Keep `max_journal_size` reasonable (5,000–10,000) to avoid memory bloat
- Use SSE streaming (`subscribe_stream`) for real-time monitoring UIs
- Emit metadata with events for structured filtering downstream
- Clean journal periodically with `events_clear` in long-running sessions

## Troubleshooting

| Issue | Cause | Solution |
|-------|-------|----------|
| Webhook not delivering | URL blocked by SSRF protection | Use publicly routable URLs; no localhost |
| Webhook delivery failed | Target unreachable or timeout | Check `last_error` in subscription list; increase `webhook_timeout_ms` |
| Events lost | Journal size exceeded | Increase `max_journal_size` or consume events faster |
| Max subscriptions reached | 100 subscription limit | Unsubscribe unused webhooks; increase `max_subscriptions` |
| HMAC verification fails | Secret mismatch | Ensure same secret on both sender and receiver |
| Replay returns empty | Wrong pattern or `since` timestamp | Use `**` pattern to verify events exist; check timestamp format |
