//! Real-time event streaming via CDP.
//!
//! Provides WebSocket and SSE-compatible event channels for
//! browser automation events (network, console, page lifecycle).

use chromiumoxide::Page;
use onecrawl_core::{Error, Result};
use serde::{Deserialize, Serialize};
use tokio::sync::broadcast;

/// Types of browser events that can be streamed.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum EventType {
    ConsoleMessage,
    NetworkRequest,
    NetworkResponse,
    PageLoad,
    PageError,
    DomContentLoaded,
    FrameNavigated,
    Dialog,
    Custom(String),
}

/// A browser event emitted during automation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BrowserEvent {
    /// Event type.
    pub event_type: EventType,
    /// Timestamp (ms since epoch).
    pub timestamp: f64,
    /// Event data as JSON.
    pub data: serde_json::Value,
}

/// An event stream that captures real-time browser events.
pub struct EventStream {
    tx: broadcast::Sender<BrowserEvent>,
    _rx: broadcast::Receiver<BrowserEvent>,
}

impl EventStream {
    /// Create a new event stream with the given capacity.
    pub fn new(capacity: usize) -> Self {
        let (tx, _rx) = broadcast::channel(capacity);
        Self { tx, _rx }
    }

    /// Subscribe to the event stream. Returns a new receiver.
    pub fn subscribe(&self) -> broadcast::Receiver<BrowserEvent> {
        self.tx.subscribe()
    }

    /// Get the sender handle (for injecting events from CDP listeners).
    pub fn sender(&self) -> broadcast::Sender<BrowserEvent> {
        self.tx.clone()
    }

    /// Number of active subscribers.
    pub fn subscriber_count(&self) -> usize {
        self.tx.receiver_count()
    }
}

/// Install console message forwarding into the event stream.
pub async fn observe_console(page: &Page, tx: broadcast::Sender<BrowserEvent>) -> Result<()> {
    let js = r#"
        (() => {
            if (window.__onecrawl_console_observed) return 'already';
            window.__onecrawl_console_observed = true;
            window.__onecrawl_console_log = [];
            const orig = console.log;
            const origWarn = console.warn;
            const origErr = console.error;
            const push = (level, args) => {
                window.__onecrawl_console_log.push({
                    level,
                    message: Array.from(args).map(a => String(a)).join(' '),
                    timestamp: Date.now()
                });
            };
            console.log = function(...args) { push('log', args); orig.apply(console, args); };
            console.warn = function(...args) { push('warn', args); origWarn.apply(console, args); };
            console.error = function(...args) { push('error', args); origErr.apply(console, args); };
            return 'installed';
        })()
    "#;
    page.evaluate(js)
        .await
        .map_err(|e| Error::Cdp(format!("observe_console failed: {e}")))?;

    // Start a polling task to drain console messages
    let page_clone_url = page
        .url()
        .await
        .map_err(|e| Error::Cdp(format!("get url for console: {e}")))?
        .unwrap_or_default()
        .to_string();

    // We store the tx for later polling via `drain_console`
    let _ = tx.send(BrowserEvent {
        event_type: EventType::ConsoleMessage,
        timestamp: now_ms(),
        data: serde_json::json!({
            "message": format!("console observer installed for {page_clone_url}"),
            "level": "info"
        }),
    });

    Ok(())
}

/// Drain buffered console messages into the event stream.
pub async fn drain_console(page: &Page, tx: &broadcast::Sender<BrowserEvent>) -> Result<usize> {
    let result = page
        .evaluate(
            r#"
            (() => {
                const logs = window.__onecrawl_console_log || [];
                window.__onecrawl_console_log = [];
                return logs;
            })()
            "#,
        )
        .await
        .map_err(|e| Error::Cdp(format!("drain_console failed: {e}")))?;

    // Try to parse as Value; if it fails, return 0 (no messages)
    let val: serde_json::Value = match result.into_value() {
        Ok(v) => v,
        Err(_) => return Ok(0),
    };

    let logs = match val {
        serde_json::Value::Array(arr) => arr,
        serde_json::Value::String(s) => {
            serde_json::from_str::<Vec<serde_json::Value>>(&s).unwrap_or_default()
        }
        _ => return Ok(0),
    };
    let count = logs.len();

    for log in logs {
        let _ = tx.send(BrowserEvent {
            event_type: EventType::ConsoleMessage,
            timestamp: log["timestamp"].as_f64().unwrap_or_else(now_ms),
            data: log,
        });
    }

    Ok(count)
}

/// Install page error observation.
pub async fn observe_errors(page: &Page, tx: broadcast::Sender<BrowserEvent>) -> Result<()> {
    let js = r#"
        (() => {
            if (window.__onecrawl_errors_observed) return 'already';
            window.__onecrawl_errors_observed = true;
            window.__onecrawl_page_errors = [];
            window.addEventListener('error', (e) => {
                window.__onecrawl_page_errors.push({
                    message: e.message,
                    filename: e.filename,
                    lineno: e.lineno,
                    colno: e.colno,
                    timestamp: Date.now()
                });
            });
            window.addEventListener('unhandledrejection', (e) => {
                window.__onecrawl_page_errors.push({
                    message: String(e.reason),
                    type: 'unhandledrejection',
                    timestamp: Date.now()
                });
            });
            return 'installed';
        })()
    "#;
    page.evaluate(js)
        .await
        .map_err(|e| Error::Cdp(format!("observe_errors failed: {e}")))?;

    let _ = tx.send(BrowserEvent {
        event_type: EventType::PageError,
        timestamp: now_ms(),
        data: serde_json::json!({"message": "error observer installed"}),
    });

    Ok(())
}

/// Drain buffered page errors into the event stream.
pub async fn drain_errors(page: &Page, tx: &broadcast::Sender<BrowserEvent>) -> Result<usize> {
    let result = page
        .evaluate(
            r#"
            (() => {
                const errs = window.__onecrawl_page_errors || [];
                window.__onecrawl_page_errors = [];
                return errs;
            })()
            "#,
        )
        .await
        .map_err(|e| Error::Cdp(format!("drain_errors failed: {e}")))?;

    let val: serde_json::Value = match result.into_value() {
        Ok(v) => v,
        Err(_) => return Ok(0),
    };

    let errs = match val {
        serde_json::Value::Array(arr) => arr,
        serde_json::Value::String(s) => {
            serde_json::from_str::<Vec<serde_json::Value>>(&s).unwrap_or_default()
        }
        _ => return Ok(0),
    };
    let count = errs.len();

    for err in errs {
        let _ = tx.send(BrowserEvent {
            event_type: EventType::PageError,
            timestamp: err["timestamp"].as_f64().unwrap_or_else(now_ms),
            data: err,
        });
    }

    Ok(count)
}

/// Emit a custom event into the stream.
pub fn emit_custom(
    tx: &broadcast::Sender<BrowserEvent>,
    name: &str,
    data: serde_json::Value,
) -> Result<()> {
    tx.send(BrowserEvent {
        event_type: EventType::Custom(name.to_string()),
        timestamp: now_ms(),
        data,
    })
    .map_err(|e| Error::Cdp(format!("emit_custom failed: {e}")))?;
    Ok(())
}

/// Format a `BrowserEvent` as an SSE-compatible string.
pub fn format_sse(event: &BrowserEvent) -> String {
    let event_name = match &event.event_type {
        EventType::ConsoleMessage => "console",
        EventType::NetworkRequest => "network_request",
        EventType::NetworkResponse => "network_response",
        EventType::PageLoad => "page_load",
        EventType::PageError => "page_error",
        EventType::DomContentLoaded => "dom_content_loaded",
        EventType::FrameNavigated => "frame_navigated",
        EventType::Dialog => "dialog",
        EventType::Custom(name) => name.as_str(),
    };
    let data = serde_json::to_string(&event.data).unwrap_or_default();
    format!("event: {event_name}\ndata: {data}\n\n")
}

fn now_ms() -> f64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as f64
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn event_stream_subscribe() {
        let stream = EventStream::new(16);
        // EventStream holds one internal receiver
        assert_eq!(stream.subscriber_count(), 1);
        let _rx = stream.subscribe();
        assert_eq!(stream.subscriber_count(), 2);
    }

    #[test]
    fn event_stream_send_receive() {
        let stream = EventStream::new(16);
        let mut rx = stream.subscribe();
        let tx = stream.sender();

        let event = BrowserEvent {
            event_type: EventType::PageLoad,
            timestamp: 1234.0,
            data: serde_json::json!({"url": "https://example.com"}),
        };
        tx.send(event.clone()).unwrap();

        let received = rx.try_recv().unwrap();
        assert_eq!(received.event_type, EventType::PageLoad);
        assert_eq!(received.timestamp, 1234.0);
    }

    #[test]
    fn format_sse_output() {
        let event = BrowserEvent {
            event_type: EventType::ConsoleMessage,
            timestamp: 0.0,
            data: serde_json::json!({"message": "hello"}),
        };
        let sse = format_sse(&event);
        assert!(sse.starts_with("event: console\n"));
        assert!(sse.contains("data: "));
        assert!(sse.ends_with("\n\n"));
    }

    #[test]
    fn event_type_serde() {
        let et = EventType::NetworkRequest;
        let json = serde_json::to_string(&et).unwrap();
        assert_eq!(json, "\"network_request\"");
        let parsed: EventType = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed, et);
    }
}
