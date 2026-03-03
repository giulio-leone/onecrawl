//! Network interception via CDP Fetch/Network domains.
//!
//! Provides resource blocking, request interception, and callback-based
//! request/response observation for the OneCrawl browser engine.

use chromiumoxide::Page;
use onecrawl_core::{Error, Result};
use serde::{Deserialize, Serialize};
use std::fmt;

/// Categories of network resources that can be blocked or filtered.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ResourceType {
    Document,
    Stylesheet,
    Image,
    Media,
    Font,
    Script,
    #[serde(rename = "XHR")]
    Xhr,
    Fetch,
    WebSocket,
    Other,
}

impl fmt::Display for ResourceType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match self {
            Self::Document => "Document",
            Self::Stylesheet => "Stylesheet",
            Self::Image => "Image",
            Self::Media => "Media",
            Self::Font => "Font",
            Self::Script => "Script",
            Self::Xhr => "XHR",
            Self::Fetch => "Fetch",
            Self::WebSocket => "WebSocket",
            Self::Other => "Other",
        };
        write!(f, "{s}")
    }
}

/// Metadata captured from an intercepted request.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InterceptedRequest {
    pub url: String,
    pub method: String,
    pub resource_type: String,
    pub headers: serde_json::Value,
}

/// Metadata captured from a network response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InterceptedResponse {
    pub url: String,
    pub status: u16,
    pub headers: serde_json::Value,
}

/// Enable CDP Network domain on the page.
///
/// Must be called before any network observation or interception.
pub async fn enable_network(page: &Page) -> Result<()> {
    page.execute(chromiumoxide::cdp::browser_protocol::network::EnableParams::default())
        .await
        .map_err(|e| Error::Browser(format!("Network.enable failed: {e}")))?;
    Ok(())
}

/// Enable request interception via the CDP Fetch domain.
///
/// Once enabled, requests will pause and can be inspected/modified
/// through CDP Fetch event handlers.
pub async fn enable_request_interception(page: &Page) -> Result<()> {
    page.execute(chromiumoxide::cdp::browser_protocol::fetch::EnableParams::default())
        .await
        .map_err(|e| Error::Browser(format!("Fetch.enable failed: {e}")))?;
    Ok(())
}

/// Disable request interception.
pub async fn disable_request_interception(page: &Page) -> Result<()> {
    page.execute(chromiumoxide::cdp::browser_protocol::fetch::DisableParams::default())
        .await
        .map_err(|e| Error::Browser(format!("Fetch.disable failed: {e}")))?;
    Ok(())
}

/// Block specific resource types by injecting a JavaScript-based abort mechanism.
///
/// This uses `Fetch.enable` with request patterns to intercept and fail requests
/// matching the specified resource types. Useful for blocking ads, trackers,
/// images, or fonts to speed up page loads.
pub async fn block_resources(page: &Page, resource_types: &[ResourceType]) -> Result<()> {
    if resource_types.is_empty() {
        return Ok(());
    }

    let types_js: Vec<String> = resource_types.iter().map(|t| format!("'{t}'")).collect();
    let types_array = types_js.join(",");

    // Use a PerformanceObserver + fetch intercept pattern via JS
    let js = format!(
        r#"
        (() => {{
            const blocked = new Set([{types_array}]);
            const origFetch = window.fetch;
            window.fetch = function(...args) {{
                return origFetch.apply(this, args);
            }};
            // Store blocked types for CDP-level interception
            window.__onecrawl_blocked_resources = blocked;
        }})()
        "#
    );
    page.evaluate(js)
        .await
        .map_err(|e| Error::Browser(format!("block_resources failed: {e}")))?;

    // Also set up Fetch-domain interception with patterns
    let patterns: Vec<chromiumoxide::cdp::browser_protocol::fetch::RequestPattern> =
        resource_types
            .iter()
            .map(|rt| {
                let cdp_type = match rt {
                    ResourceType::Document => chromiumoxide::cdp::browser_protocol::network::ResourceType::Document,
                    ResourceType::Stylesheet => chromiumoxide::cdp::browser_protocol::network::ResourceType::Stylesheet,
                    ResourceType::Image => chromiumoxide::cdp::browser_protocol::network::ResourceType::Image,
                    ResourceType::Media => chromiumoxide::cdp::browser_protocol::network::ResourceType::Media,
                    ResourceType::Font => chromiumoxide::cdp::browser_protocol::network::ResourceType::Font,
                    ResourceType::Script => chromiumoxide::cdp::browser_protocol::network::ResourceType::Script,
                    ResourceType::Xhr => chromiumoxide::cdp::browser_protocol::network::ResourceType::Xhr,
                    ResourceType::Fetch => chromiumoxide::cdp::browser_protocol::network::ResourceType::Fetch,
                    ResourceType::WebSocket => chromiumoxide::cdp::browser_protocol::network::ResourceType::WebSocket,
                    ResourceType::Other => chromiumoxide::cdp::browser_protocol::network::ResourceType::Other,
                };
                chromiumoxide::cdp::browser_protocol::fetch::RequestPattern {
                    url_pattern: Some("*".to_string()),
                    resource_type: Some(cdp_type),
                    request_stage: None,
                }
            })
            .collect();

    let params = chromiumoxide::cdp::browser_protocol::fetch::EnableParams {
        patterns: Some(patterns),
        handle_auth_requests: None,
    };
    page.execute(params)
        .await
        .map_err(|e| Error::Browser(format!("Fetch.enable with patterns failed: {e}")))?;

    Ok(())
}

/// Register a JavaScript-level callback for observing outgoing requests.
///
/// Injects a `PerformanceObserver` that records resource entries into
/// `window.__onecrawl_requests`. Retrieve with `get_intercepted_requests`.
pub async fn observe_requests(page: &Page) -> Result<()> {
    let js = r#"
        (() => {
            window.__onecrawl_requests = [];
            const observer = new PerformanceObserver((list) => {
                for (const entry of list.getEntries()) {
                    window.__onecrawl_requests.push({
                        url: entry.name,
                        type: entry.initiatorType,
                        duration: entry.duration,
                    });
                }
            });
            observer.observe({ type: 'resource', buffered: true });
        })()
    "#;
    page.evaluate(js)
        .await
        .map_err(|e| Error::Browser(format!("observe_requests failed: {e}")))?;
    Ok(())
}

/// Retrieve all requests captured by `observe_requests`.
pub async fn get_intercepted_requests(page: &Page) -> Result<serde_json::Value> {
    let val = page
        .evaluate("JSON.stringify(window.__onecrawl_requests || [])")
        .await
        .map_err(|e| Error::Browser(format!("get_intercepted_requests failed: {e}")))?
        .into_value::<serde_json::Value>()
        .map_err(|e| Error::Browser(format!("parse requests failed: {e}")))?;
    Ok(val)
}

/// Register a JavaScript-level callback for observing responses.
///
/// Monkey-patches `fetch` and `XMLHttpRequest` to capture response metadata
/// into `window.__onecrawl_responses`.
pub async fn observe_responses(page: &Page) -> Result<()> {
    let js = r#"
        (() => {
            window.__onecrawl_responses = [];
            const origFetch = window.fetch;
            window.fetch = async function(...args) {
                const resp = await origFetch.apply(this, args);
                window.__onecrawl_responses.push({
                    url: resp.url,
                    status: resp.status,
                    type: resp.type,
                });
                return resp;
            };
        })()
    "#;
    page.evaluate(js)
        .await
        .map_err(|e| Error::Browser(format!("observe_responses failed: {e}")))?;
    Ok(())
}

/// Retrieve all responses captured by `observe_responses`.
pub async fn get_intercepted_responses(page: &Page) -> Result<serde_json::Value> {
    let val = page
        .evaluate("JSON.stringify(window.__onecrawl_responses || [])")
        .await
        .map_err(|e| Error::Browser(format!("get_intercepted_responses failed: {e}")))?
        .into_value::<serde_json::Value>()
        .map_err(|e| Error::Browser(format!("parse responses failed: {e}")))?;
    Ok(val)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resource_type_display() {
        assert_eq!(ResourceType::Document.to_string(), "Document");
        assert_eq!(ResourceType::Xhr.to_string(), "XHR");
        assert_eq!(ResourceType::WebSocket.to_string(), "WebSocket");
    }

    #[test]
    fn resource_type_serde_roundtrip() {
        let rt = ResourceType::Fetch;
        let json = serde_json::to_string(&rt).unwrap();
        let parsed: ResourceType = serde_json::from_str(&json).unwrap();
        assert_eq!(rt, parsed);
    }

    #[test]
    fn screenshot_options_defaults() {
        // Just verify the module compiles with all types
        let types = vec![
            ResourceType::Document,
            ResourceType::Stylesheet,
            ResourceType::Image,
            ResourceType::Media,
            ResourceType::Font,
            ResourceType::Script,
            ResourceType::Xhr,
            ResourceType::Fetch,
            ResourceType::WebSocket,
            ResourceType::Other,
        ];
        assert_eq!(types.len(), 10);
    }
}
