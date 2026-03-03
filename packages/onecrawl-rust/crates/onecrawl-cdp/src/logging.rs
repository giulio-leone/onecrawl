//! Structured logging utilities for OneCrawl.

use tracing::{info, warn, error, debug};

/// Log a browser action with structured fields.
pub fn log_action(action: &str, target: &str, duration_ms: u64) {
    info!(action = action, target = target, duration_ms = duration_ms, "browser action");
}

/// Log a warning with context.
pub fn log_warning(component: &str, message: &str) {
    warn!(component = component, message = message, "warning");
}

/// Log an error with context.
pub fn log_error(component: &str, error: &str) {
    error!(component = component, error = error, "error");
}

/// Log a debug message.
pub fn log_debug(component: &str, message: &str) {
    debug!(component = component, message = message);
}
