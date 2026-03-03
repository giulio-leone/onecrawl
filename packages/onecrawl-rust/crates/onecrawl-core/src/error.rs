use thiserror::Error;

/// Unified error type for all OneCrawl operations.
///
/// Each variant maps to a subsystem so callers can match on the error source
/// and present actionable diagnostics.
#[derive(Error, Debug)]
pub enum OneCrawlError {
    /// Chrome DevTools Protocol / browser-automation failure.
    #[error("CDP error: {0}")]
    Cdp(String),

    /// Cryptographic operation failure (AES-GCM, PBKDF2, TOTP, …).
    #[error("Crypto error: {0}")]
    Crypto(String),

    /// HTML parsing or DOM extraction failure.
    #[error("Parser error: {0}")]
    Parser(String),

    /// Persistent storage (sled) failure.
    #[error("Storage error: {0}")]
    Storage(String),

    /// HTTP server (axum / routes) failure.
    #[error("Server error: {0}")]
    Server(String),

    /// Configuration or invalid-input error.
    #[error("Config error: {0}")]
    Config(String),

    /// A requested resource was not found.
    #[error("Not found: {0}")]
    NotFound(String),

    /// Generic I/O error.
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    /// JSON serialisation / deserialisation error.
    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),
}

// Backward-compatible alias so existing `use onecrawl_core::Error` keeps working.
pub type Error = OneCrawlError;

pub type Result<T> = std::result::Result<T, OneCrawlError>;

/// Explicit full-name alias for public API consumers.
pub type OneCrawlResult<T> = std::result::Result<T, OneCrawlError>;
