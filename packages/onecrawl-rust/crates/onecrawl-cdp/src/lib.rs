//! OneCrawl CDP — Browser automation via Chrome DevTools Protocol.
//!
//! Wraps `chromiumoxide` to provide high-level browser commands.

pub mod bridge;
pub mod browser;
pub mod cookie;
pub mod coverage;
pub mod element;
pub mod emulation;
pub mod events;
pub mod har;
pub mod input;
pub mod keyboard;
pub mod navigation;
pub mod network;
pub mod page;
#[cfg(feature = "playwright")]
pub mod playwright_backend;
pub mod screenshot;
pub mod stealth;
pub mod websocket;

pub use browser::BrowserSession;
pub use bridge::PlaywrightBridge;
pub use cookie::{Cookie, SetCookieParams};
pub use coverage::CoverageReport;
pub use emulation::Viewport;
pub use events::{BrowserEvent, EventStream, EventType};
pub use har::HarRecorder;
pub use network::ResourceType;
pub use screenshot::{ImageFormat, PdfOptions, ScreenshotOptions};
pub use stealth::{Fingerprint, generate_fingerprint, get_stealth_init_script};
pub use websocket::WsRecorder;

// Re-export chromiumoxide::Page for downstream consumers
pub use chromiumoxide::Page;
