//! OneCrawl CDP — Browser automation via Chrome DevTools Protocol.
//!
//! Wraps `chromiumoxide` to provide high-level browser commands.

pub mod bridge;
pub mod browser;
pub mod element;
pub mod logging;
pub mod navigation;
pub mod network;
pub mod page;
#[cfg(feature = "playwright")]
pub mod playwright_backend;
pub mod screenshot;
pub mod stealth;

pub use browser::BrowserSession;
pub use bridge::PlaywrightBridge;
pub use network::ResourceType;
pub use screenshot::{ImageFormat, PdfOptions, ScreenshotOptions};
pub use stealth::{Fingerprint, generate_fingerprint, get_stealth_init_script};
