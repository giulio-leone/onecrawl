//! Hexagonal architecture port traits for browser automation.
//!
//! These traits decouple consumers from the concrete CDP implementation,
//! enabling testing with mocks and future backend swaps.

use async_trait::async_trait;
use crate::error::Result;
use std::collections::HashMap;

mod browser_impl;
mod element_impl;
mod emulation_impl;
mod input_impl;
mod network_impl;
mod page_impl;

// ── Shared types ────────────────────────────────────────────────────────

/// Rectangle for element bounding box (framework-agnostic).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ElementRect {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
}

/// Cookie data (framework-agnostic).
#[derive(Debug, Clone)]
pub struct CookieInfo {
    pub name: String,
    pub value: String,
    pub domain: String,
    pub path: String,
    pub expires: Option<f64>,
    pub http_only: bool,
    pub secure: bool,
    pub same_site: Option<String>,
}

// ── Port traits ─────────────────────────────────────────────────────────

/// Abstraction over browser-level operations (M2-I1).
#[async_trait]
pub trait BrowserPort: Send + Sync {
    async fn new_page(&self, url: &str) -> Result<Box<dyn PagePort>>;
    async fn close_browser(&mut self) -> Result<()>;
    fn websocket_address(&self) -> &str;
    async fn version(&self) -> Result<String>;
    async fn user_agent(&self) -> Result<String>;
    async fn clear_all_cookies(&self) -> Result<()>;
}

/// Abstraction over page-level operations (M2-I2).
#[async_trait]
pub trait PagePort: Send + Sync {
    // Navigation
    async fn goto_url(&self, url: &str) -> Result<()>;
    async fn reload_page(&self) -> Result<()>;
    async fn wait_for_navigation(&self) -> Result<()>;
    async fn current_url(&self) -> Result<Option<String>>;
    async fn page_title(&self) -> Result<Option<String>>;

    // Content
    async fn page_content(&self) -> Result<String>;
    async fn set_page_content(&self, html: &str) -> Result<()>;

    // DOM Selection
    async fn query_selector(&self, selector: &str) -> Result<Box<dyn ElementPort>>;
    async fn query_selector_all(&self, selector: &str) -> Result<Vec<Box<dyn ElementPort>>>;

    // JavaScript Execution
    async fn evaluate_expression(&self, expression: &str) -> Result<serde_json::Value>;
    async fn evaluate_function(
        &self,
        function_declaration: &str,
        args: Vec<serde_json::Value>,
    ) -> Result<serde_json::Value>;

    // Capture
    async fn capture_screenshot(&self) -> Result<Vec<u8>>;
    async fn capture_pdf(&self) -> Result<Vec<u8>>;

    // Page Lifecycle
    async fn activate_page(&self) -> Result<()>;
    async fn close_page(&self) -> Result<()>;

    // Metrics
    async fn page_metrics(&self) -> Result<Vec<(String, f64)>>;
}

/// Abstraction over DOM element operations.
#[async_trait]
pub trait ElementPort: Send + Sync {
    // Interaction
    async fn click_element(&self) -> Result<()>;
    async fn hover_element(&self) -> Result<()>;
    async fn focus_element(&self) -> Result<()>;
    async fn type_text(&self, text: &str) -> Result<()>;
    async fn press_key(&self, key: &str) -> Result<()>;
    async fn scroll_into_view(&self) -> Result<()>;

    // Content
    async fn inner_text(&self) -> Result<Option<String>>;
    async fn inner_html(&self) -> Result<Option<String>>;
    async fn outer_html(&self) -> Result<Option<String>>;

    // Attributes & Properties
    async fn get_attribute(&self, name: &str) -> Result<Option<String>>;
    async fn get_property(&self, name: &str) -> Result<Option<serde_json::Value>>;

    // Selection
    async fn query_selector(&self, selector: &str) -> Result<Box<dyn ElementPort>>;
    async fn query_selector_all(&self, selector: &str) -> Result<Vec<Box<dyn ElementPort>>>;

    // Geometry
    async fn bounding_box(&self) -> Result<ElementRect>;

    // Capture
    async fn capture_screenshot(&self) -> Result<Vec<u8>>;
}

/// Abstraction over network operations (M2-I3).
#[async_trait]
pub trait NetworkPort: Send + Sync {
    async fn set_extra_headers(&self, headers: HashMap<String, String>) -> Result<()>;
    async fn set_request_interception(&self, patterns: &[String]) -> Result<()>;
    /// Set user-agent for network requests (protocol-level header override).
    /// See also [`EmulationPort::set_user_agent_override`] for device-emulation context.
    async fn set_user_agent(&self, ua: &str) -> Result<()>;
    async fn authenticate(&self, username: &str, password: &str) -> Result<()>;
    async fn get_cookies(&self) -> Result<Vec<CookieInfo>>;
    async fn set_cookie(&self, name: &str, value: &str, domain: &str, path: &str) -> Result<()>;
    async fn delete_cookies_by_name(&self, name: &str) -> Result<()>;
    async fn clear_cookies(&self) -> Result<()>;
    async fn enable_stealth(&self) -> Result<()>;
}

/// Abstraction over device emulation (M2-I4).
#[async_trait]
pub trait EmulationPort: Send + Sync {
    async fn set_viewport_size(
        &self,
        width: u32,
        height: u32,
        device_scale_factor: f64,
    ) -> Result<()>;
    /// Set user-agent as part of device emulation (pairs with viewport, geolocation, etc.).
    /// Functionally identical to [`NetworkPort::set_user_agent`] at the CDP level;
    /// separated for semantic clarity when emulating a full device profile.
    async fn set_user_agent_override(&self, ua: &str) -> Result<()>;
    async fn set_geolocation(
        &self,
        latitude: f64,
        longitude: f64,
        accuracy: f64,
    ) -> Result<()>;
    async fn set_timezone_override(&self, timezone_id: &str) -> Result<()>;
    async fn set_locale_override(&self, locale: &str) -> Result<()>;
    async fn set_media_type(&self, media_type: &str) -> Result<()>;
}

/// Abstraction over low-level input dispatch (M2-I5).
#[async_trait]
pub trait InputPort: Send + Sync {
    async fn click_at(&self, x: f64, y: f64) -> Result<()>;
    async fn move_mouse_to(&self, x: f64, y: f64) -> Result<()>;
    async fn mouse_down(&self, x: f64, y: f64) -> Result<()>;
    async fn mouse_up(&self, x: f64, y: f64) -> Result<()>;
    async fn type_keyboard(&self, text: &str) -> Result<()>;
    async fn press_keyboard_key(&self, key: &str) -> Result<()>;
}
