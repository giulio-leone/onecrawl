use base64::{Engine as _, engine::general_purpose::STANDARD as B64};
use onecrawl_core::{EncryptedPayload, Error, Result};
use rand::RngCore;
use ring::aead::{self, Aad, BoundKey, NONCE_LEN, Nonce, NonceSequence, UnboundKey};
use zeroize::Zeroize;

struct SingleNonce(Option<[u8; NONCE_LEN]>);

impl NonceSequence for SingleNonce {
    fn advance(&mut self) -> std::result::Result<Nonce, ring::error::Unspecified> {
        self.0
            .take()
            .map(Nonce::assume_unique_for_key)
            .ok_or(ring::error::Unspecified)
    }
}

/// Encrypt plaintext with AES-256-GCM.
///
/// Uses PBKDF2-derived key from passphrase + random salt.
/// Returns an `EncryptedPayload` with base64-encoded fields.
pub fn encrypt(plaintext: &[u8], passphrase: &str) -> Result<EncryptedPayload> {
    let mut salt = [0u8; 16];
    rand::rng().fill_bytes(&mut salt);

    let mut key_bytes = super::pbkdf2::derive_key(passphrase, &salt)?;

    let mut nonce_bytes = [0u8; NONCE_LEN];
    rand::rng().fill_bytes(&mut nonce_bytes);

    let unbound_key = UnboundKey::new(&aead::AES_256_GCM, &key_bytes)
        .map_err(|e| Error::Crypto(format!("key creation failed: {e}")))?;

    key_bytes.zeroize();

    let nonce_seq = SingleNonce(Some(nonce_bytes));
    let mut sealing_key = aead::SealingKey::new(unbound_key, nonce_seq);

    let mut in_out = plaintext.to_vec();
    sealing_key
        .seal_in_place_append_tag(Aad::empty(), &mut in_out)
        .map_err(|e| Error::Crypto(format!("encryption failed: {e}")))?;

    Ok(EncryptedPayload {
        ciphertext: B64.encode(&in_out),
        nonce: B64.encode(nonce_bytes),
        salt: B64.encode(salt),
    })
}

/// Decrypt an `EncryptedPayload` with AES-256-GCM.
pub fn decrypt(payload: &EncryptedPayload, passphrase: &str) -> Result<Vec<u8>> {
    let salt = B64
        .decode(&payload.salt)
        .map_err(|e| Error::Crypto(format!("invalid salt base64: {e}")))?;
    let nonce_bytes: [u8; NONCE_LEN] = B64
        .decode(&payload.nonce)
        .map_err(|e| Error::Crypto(format!("invalid nonce base64: {e}")))?
        .try_into()
        .map_err(|_| Error::Crypto("invalid nonce length".into()))?;
    let mut ciphertext = B64
        .decode(&payload.ciphertext)
        .map_err(|e| Error::Crypto(format!("invalid ciphertext base64: {e}")))?;

    let mut key_bytes = super::pbkdf2::derive_key(passphrase, &salt)?;

    let unbound_key = UnboundKey::new(&aead::AES_256_GCM, &key_bytes)
        .map_err(|e| Error::Crypto(format!("key creation failed: {e}")))?;

    key_bytes.zeroize();

    let nonce_seq = SingleNonce(Some(nonce_bytes));
    let mut opening_key = aead::OpeningKey::new(unbound_key, nonce_seq);

    let plaintext = opening_key
        .open_in_place(Aad::empty(), &mut ciphertext)
        .map_err(|_| Error::Crypto("decryption failed: invalid key or corrupted data".into()))?;

    Ok(plaintext.to_vec())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn encrypt_decrypt_roundtrip() {
        let plaintext = b"hello onecrawl rust crypto!";
        let passphrase = "test-passphrase-2024";

        let encrypted = encrypt(plaintext, passphrase).unwrap();
        let decrypted = decrypt(&encrypted, passphrase).unwrap();

        assert_eq!(decrypted, plaintext);
    }

    #[test]
    fn decrypt_wrong_passphrase_fails() {
        let plaintext = b"secret data";
        let encrypted = encrypt(plaintext, "correct-pass").unwrap();
        let result = decrypt(&encrypted, "wrong-pass");
        assert!(result.is_err());
    }

    #[test]
    fn different_encryptions_produce_different_output() {
        let plaintext = b"same input";
        let pass = "same-pass";
        let e1 = encrypt(plaintext, pass).unwrap();
        let e2 = encrypt(plaintext, pass).unwrap();
        // Different nonces and salts → different ciphertext
        assert_ne!(e1.ciphertext, e2.ciphertext);
    }

    #[test]
    fn encrypt_empty_plaintext() {
        let encrypted = encrypt(b"", "pass").unwrap();
        let decrypted = decrypt(&encrypted, "pass").unwrap();
        assert_eq!(decrypted, b"");
    }

    #[test]
    fn encrypt_large_plaintext() {
        let plaintext = vec![0xABu8; 1_000_000]; // 1MB
        let encrypted = encrypt(&plaintext, "pass").unwrap();
        let decrypted = decrypt(&encrypted, "pass").unwrap();
        assert_eq!(decrypted, plaintext);
    }
}
