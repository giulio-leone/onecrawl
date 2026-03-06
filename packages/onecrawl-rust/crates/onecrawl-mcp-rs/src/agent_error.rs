//! Structured error responses for AI agent consumers.
//!
//! Provides machine-readable error codes, recovery hints, and suggestion
//! fields so that agents can programmatically decide how to retry or
//! recover from failures.

use serde::Serialize;

/// A structured, agent-friendly error envelope.
#[derive(Debug, Serialize)]
pub struct AgentError {
    pub code: &'static str,
    pub message: String,
    pub recoverable: bool,
    pub suggestion: Option<String>,
}

/// The target element was not found in the DOM.
pub fn element_not_found(selector: &str) -> AgentError {
    AgentError {
        code: "ELEMENT_NOT_FOUND",
        message: format!("Element not found: {selector}"),
        recoverable: true,
        suggestion: Some(
            "Verify the selector is correct, wait for the element to appear, \
             or take a new snapshot to get updated refs."
                .into(),
        ),
    }
}

/// A page navigation failed.
pub fn navigation_failed(url: &str, reason: &str) -> AgentError {
    AgentError {
        code: "NAVIGATION_FAILED",
        message: format!("Navigation to {url} failed: {reason}"),
        recoverable: true,
        suggestion: Some(
            "Check the URL is valid and the server is reachable. \
             Retry after a short delay."
                .into(),
        ),
    }
}

/// An operation timed out.
pub fn timeout(action: &str) -> AgentError {
    AgentError {
        code: "TIMEOUT",
        message: format!("Timeout waiting for: {action}"),
        recoverable: true,
        suggestion: Some(
            "Increase the timeout, ensure the page has loaded, \
             or check that the expected element/condition exists."
                .into(),
        ),
    }
}

/// A JavaScript evaluation failed.
pub fn eval_failed(reason: &str) -> AgentError {
    AgentError {
        code: "EVAL_FAILED",
        message: format!("JavaScript evaluation failed: {reason}"),
        recoverable: false,
        suggestion: Some(
            "Check the JS expression for syntax errors. \
             Ensure the page context is available."
                .into(),
        ),
    }
}

/// Chain dispatch could not find the requested tool.
pub fn unknown_tool(tool_name: &str) -> AgentError {
    AgentError {
        code: "UNKNOWN_TOOL",
        message: format!("Tool not available in chain dispatch: {tool_name}"),
        recoverable: false,
        suggestion: Some(
            "Only a subset of tools is supported in execute_chain. \
             Supported: navigation.goto, navigation.click, navigation.type, \
             navigation.wait, navigation.evaluate, navigation.snapshot, \
             scraping.css, scraping.text."
                .into(),
        ),
    }
}
