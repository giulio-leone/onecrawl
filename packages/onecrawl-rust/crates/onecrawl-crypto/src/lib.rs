//! OneCrawl Crypto — AES-256-GCM, PKCE S256, TOTP (RFC 6238), PBKDF2.
//!
//! All operations use the `ring` crate for FIPS-grade cryptography.

pub mod aes_gcm;
pub mod pbkdf2;
pub mod pkce;
pub mod totp;

pub use aes_gcm::{decrypt, encrypt};
pub use pbkdf2::derive_key;
pub use pkce::generate_pkce_challenge;
pub use totp::generate_totp;
