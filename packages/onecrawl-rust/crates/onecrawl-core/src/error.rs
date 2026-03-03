use thiserror::Error;

/// Unified error type for all OneCrawl operations.
#[derive(Error, Debug)]
pub enum Error {
    #[error("crypto error: {0}")]
    Crypto(String),

    #[error("storage error: {0}")]
    Storage(String),

    #[error("parser error: {0}")]
    Parser(String),

    #[error("browser error: {0}")]
    Browser(String),

    #[error("invalid input: {0}")]
    InvalidInput(String),

    #[error("not found: {0}")]
    NotFound(String),

    #[error("io error: {0}")]
    Io(#[from] std::io::Error),

    #[error("serialization error: {0}")]
    Serialization(#[from] serde_json::Error),
}

pub type Result<T> = std::result::Result<T, Error>;
