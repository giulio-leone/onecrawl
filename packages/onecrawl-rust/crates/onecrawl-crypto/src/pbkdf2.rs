use onecrawl_core::{Error, Result};
use ring::pbkdf2;
use zeroize::Zeroize;

const ITERATIONS: u32 = 100_000;
const KEY_LEN: usize = 32; // AES-256

/// Derive a 256-bit key from passphrase + salt using PBKDF2-HMAC-SHA256.
///
/// Returns a 32-byte key suitable for AES-256-GCM.
/// The caller is responsible for zeroizing the key after use.
pub fn derive_key(passphrase: &str, salt: &[u8]) -> Result<[u8; KEY_LEN]> {
    if passphrase.is_empty() {
        return Err(Error::Config("passphrase cannot be empty".into()));
    }
    if salt.len() < 8 {
        return Err(Error::Config("salt must be at least 8 bytes".into()));
    }

    let mut key = [0u8; KEY_LEN];
    pbkdf2::derive(
        pbkdf2::PBKDF2_HMAC_SHA256,
        std::num::NonZeroU32::new(ITERATIONS).unwrap(),
        salt,
        passphrase.as_bytes(),
        &mut key,
    );

    Ok(key)
}

/// Verify a passphrase against a known derived key.
pub fn verify_key(passphrase: &str, salt: &[u8], expected_key: &[u8]) -> bool {
    if let Ok(mut derived) = derive_key(passphrase, salt) {
        let matches = derived.len() == expected_key.len()
            && derived
                .iter()
                .zip(expected_key.iter())
                .fold(0u8, |acc, (a, b)| acc | (a ^ b))
                == 0;
        derived.zeroize();
        matches
    } else {
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn derive_key_deterministic() {
        let salt = b"test-salt-16byte";
        let k1 = derive_key("password", salt).unwrap();
        let k2 = derive_key("password", salt).unwrap();
        assert_eq!(k1, k2);
    }

    #[test]
    fn different_passphrase_different_key() {
        let salt = b"test-salt-16byte";
        let k1 = derive_key("password1", salt).unwrap();
        let k2 = derive_key("password2", salt).unwrap();
        assert_ne!(k1, k2);
    }

    #[test]
    fn different_salt_different_key() {
        let k1 = derive_key("password", b"salt-one-16byte!").unwrap();
        let k2 = derive_key("password", b"salt-two-16byte!").unwrap();
        assert_ne!(k1, k2);
    }

    #[test]
    fn empty_passphrase_rejected() {
        assert!(derive_key("", b"valid-salt-16!").is_err());
    }

    #[test]
    fn short_salt_rejected() {
        assert!(derive_key("pass", b"short").is_err());
    }

    #[test]
    fn verify_key_correct() {
        let salt = b"test-salt-16byte";
        let key = derive_key("mypass", salt).unwrap();
        assert!(verify_key("mypass", salt, &key));
    }

    #[test]
    fn verify_key_wrong_passphrase() {
        let salt = b"test-salt-16byte";
        let key = derive_key("mypass", salt).unwrap();
        assert!(!verify_key("wrong", salt, &key));
    }
}
