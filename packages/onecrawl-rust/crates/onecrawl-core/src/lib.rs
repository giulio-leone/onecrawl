//! OneCrawl Core — shared types, traits, and errors.

pub mod error;
pub mod health;
pub mod types;

pub use error::{Error, Result};
pub use health::{ComponentHealth, HealthStatus};
pub use types::*;
