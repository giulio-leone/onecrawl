//! OneCrawl CDP — Browser automation via Chrome DevTools Protocol.
//!
//! Wraps `chromiumoxide` to provide high-level browser commands.

pub mod browser;
pub mod element;
pub mod navigation;
pub mod page;
pub mod screenshot;

pub use browser::BrowserSession;
