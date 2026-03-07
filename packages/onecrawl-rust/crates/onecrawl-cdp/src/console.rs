//! Console message interception via JS monkey-patching.
//!
//! Captures console.log/warn/error/info/debug messages, plus
//! window.onerror and unhandledrejection events.

use onecrawl_browser::Page;
use onecrawl_core::Result;
use serde::{Deserialize, Serialize};

/// A captured console entry.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConsoleEntry {
    pub level: String,
    pub text: String,
    pub timestamp: f64,
    pub args: Vec<String>,
}

/// Monkey-patch console methods to capture entries into `window.__onecrawl_console_entries`.
pub async fn start_console_capture(page: &Page) -> Result<()> {
    let js = r#"
        (() => {
            if (window.__onecrawl_console_active) return 'already';
            window.__onecrawl_console_active = true;
            window.__onecrawl_console_entries = [];

            const levels = ['log', 'warn', 'error', 'info', 'debug'];
            const originals = {};
            levels.forEach(level => {
                originals[level] = console[level].bind(console);
                console[level] = function(...args) {
                    window.__onecrawl_console_entries.push({
                        level: level,
                        text: args.map(a => {
                            try { return typeof a === 'object' ? JSON.stringify(a) : String(a); }
                            catch(_) { return String(a); }
                        }).join(' '),
                        timestamp: Date.now(),
                        args: args.map(a => {
                            try { return typeof a === 'object' ? JSON.stringify(a) : String(a); }
                            catch(_) { return String(a); }
                        })
                    });
                    originals[level].apply(console, args);
                };
            });

            window.addEventListener('error', (evt) => {
                window.__onecrawl_console_entries.push({
                    level: 'error',
                    text: evt.message || String(evt),
                    timestamp: Date.now(),
                    args: [evt.message || '', evt.filename || '', String(evt.lineno || ''), String(evt.colno || '')]
                });
            });

            window.addEventListener('unhandledrejection', (evt) => {
                window.__onecrawl_console_entries.push({
                    level: 'error',
                    text: 'Unhandled rejection: ' + (evt.reason ? String(evt.reason) : 'unknown'),
                    timestamp: Date.now(),
                    args: [evt.reason ? String(evt.reason) : 'unknown']
                });
            });

            return 'installed';
        })()
    "#;

    page.evaluate(js)
        .await
        .map_err(|e| onecrawl_core::Error::Cdp(format!("start_console_capture failed: {e}")))?;

    Ok(())
}

/// Drain captured console entries from the page and return them.
pub async fn drain_console_entries(page: &Page) -> Result<Vec<ConsoleEntry>> {
    let result = page
        .evaluate(
            r#"
            (() => {
                const entries = window.__onecrawl_console_entries || [];
                window.__onecrawl_console_entries = [];
                return entries;
            })()
            "#,
        )
        .await
        .map_err(|e| onecrawl_core::Error::Cdp(format!("drain_console_entries failed: {e}")))?;

    let entries: Vec<ConsoleEntry> = result.into_value().unwrap_or_default();

    Ok(entries)
}

/// Clear the console entry buffer without returning entries.
pub async fn clear_console(page: &Page) -> Result<()> {
    page.evaluate("window.__onecrawl_console_entries = []")
        .await
        .map_err(|e| onecrawl_core::Error::Cdp(format!("clear_console failed: {e}")))?;

    Ok(())
}
