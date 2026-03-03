use serde::{Deserialize, Serialize};

/// Encrypted payload wrapper (AES-256-GCM).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EncryptedPayload {
    /// Base64-encoded ciphertext
    pub ciphertext: String,
    /// Base64-encoded 12-byte nonce
    pub nonce: String,
    /// Base64-encoded 16-byte salt (for PBKDF2 key derivation)
    pub salt: String,
}

/// PKCE challenge pair for OAuth 2.1.
#[derive(Debug, Clone)]
pub struct PkceChallenge {
    pub code_verifier: String,
    pub code_challenge: String,
}

/// TOTP configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TotpConfig {
    /// Base32-encoded secret
    pub secret: String,
    /// Number of digits (default: 6)
    pub digits: u32,
    /// Time step in seconds (default: 30)
    pub period: u32,
    /// Hash algorithm (default: SHA1)
    pub algorithm: TotpAlgorithm,
}

impl Default for TotpConfig {
    fn default() -> Self {
        Self {
            secret: String::new(),
            digits: 6,
            period: 30,
            algorithm: TotpAlgorithm::Sha1,
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum TotpAlgorithm {
    Sha1,
    Sha256,
    Sha512,
}

/// OAuth 2.1 tokens.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OAuthTokens {
    pub access_token: String,
    pub refresh_token: Option<String>,
    pub token_type: String,
    pub expires_in: Option<u64>,
    pub scope: Option<String>,
}

/// Storage trait — port for the hexagonal architecture.
pub trait StoragePort: Send + Sync {
    fn get(&self, key: &str) -> onecrawl_core::Result<Option<Vec<u8>>>;
    fn set(&self, key: &str, value: &[u8]) -> onecrawl_core::Result<()>;
    fn delete(&self, key: &str) -> onecrawl_core::Result<()>;
    fn list(&self, prefix: &str) -> onecrawl_core::Result<Vec<String>>;
}

// Re-export for convenience
mod onecrawl_core {
    pub type Result<T> = std::result::Result<T, super::super::error::Error>;
}
