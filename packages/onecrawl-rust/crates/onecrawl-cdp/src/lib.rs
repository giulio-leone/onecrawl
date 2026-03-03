//! OneCrawl CDP — Browser automation via Chrome DevTools Protocol.
//!
//! Wraps `chromiumoxide` to provide high-level browser commands.

pub mod browser;
pub mod element;
pub mod navigation;
pub mod page;
pub mod screenshot;
pub mod stealth;

pub use browser::BrowserSession;
pub use stealth::{Fingerprint, generate_fingerprint, get_stealth_init_script};
