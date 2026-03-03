//! Anti-detection and stealth layer for OneCrawl CDP.
//!
//! Ports the TypeScript stealth patches (navigator.webdriver, WebGL spoofing,
//! Chrome runtime mocking, fingerprint randomization) to Rust.

pub mod fingerprint;
pub mod scripts;

pub use fingerprint::{Fingerprint, generate_fingerprint};
pub use scripts::get_stealth_init_script;
