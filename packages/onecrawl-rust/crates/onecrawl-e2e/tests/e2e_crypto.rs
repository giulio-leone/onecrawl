//! E2E tests for the crypto pipeline.
//! Tests the full encrypt → store → retrieve → decrypt cycle.

use onecrawl_core::{EncryptedPayload, TotpAlgorithm, TotpConfig};
use onecrawl_crypto::{aes_gcm, pbkdf2, pkce, totp};
use onecrawl_storage::EncryptedStore;
use tempfile::TempDir;

// ────────────────────── Encrypt → Store → Retrieve → Decrypt ──────────────────────

#[test]
fn e2e_encrypt_store_retrieve_decrypt() {
    // 1. Encrypt data with passphrase
    let plaintext = b"sensitive-token-data-12345";
    let passphrase = "master-password-2024";
    let encrypted = aes_gcm::encrypt(plaintext, passphrase).unwrap();

    // 2. Serialize the encrypted payload for storage
    let payload_json = serde_json::to_vec(&encrypted).unwrap();

    // 3. Store in encrypted storage (double encryption: payload is encrypted, then store encrypts again)
    let dir = TempDir::new().unwrap();
    let store = EncryptedStore::open(dir.path().join("store").as_path(), "store-password").unwrap();
    store.set("token", &payload_json).unwrap();

    // 4. Retrieve from storage
    let retrieved = store.get("token").unwrap().unwrap();
    let retrieved_payload: EncryptedPayload = serde_json::from_slice(&retrieved).unwrap();

    // 5. Decrypt with original passphrase
    let decrypted = aes_gcm::decrypt(&retrieved_payload, passphrase).unwrap();
    assert_eq!(decrypted, plaintext);
}

#[test]
fn e2e_encrypt_decrypt_roundtrip_various_sizes() {
    let passphrase = "test-pass";
    let sizes = [0, 1, 16, 256, 1024, 65536];

    for size in sizes {
        let plaintext = vec![0xABu8; size];
        let encrypted = aes_gcm::encrypt(&plaintext, passphrase).unwrap();
        let decrypted = aes_gcm::decrypt(&encrypted, passphrase).unwrap();
        assert_eq!(decrypted, plaintext, "roundtrip failed for size {size}");
    }
}

#[test]
fn e2e_wrong_passphrase_fails_decrypt() {
    let encrypted = aes_gcm::encrypt(b"secret", "correct").unwrap();
    let result = aes_gcm::decrypt(&encrypted, "wrong");
    assert!(result.is_err());
}

#[test]
fn e2e_different_encryptions_unique_output() {
    let plaintext = b"same data";
    let pass = "same-pass";
    let e1 = aes_gcm::encrypt(plaintext, pass).unwrap();
    let e2 = aes_gcm::encrypt(plaintext, pass).unwrap();
    // Random salt + nonce → different ciphertext each time
    assert_ne!(e1.ciphertext, e2.ciphertext);
    assert_ne!(e1.nonce, e2.nonce);
    assert_ne!(e1.salt, e2.salt);
}

// ────────────────────── PBKDF2 Key Derivation ──────────────────────

#[test]
fn e2e_pbkdf2_deterministic() {
    let salt = [0xABu8; 16];
    let k1 = pbkdf2::derive_key("password", &salt).unwrap();
    let k2 = pbkdf2::derive_key("password", &salt).unwrap();
    assert_eq!(k1, k2);
}

#[test]
fn e2e_pbkdf2_different_salts_different_keys() {
    let k1 = pbkdf2::derive_key("password", &[0x01; 16]).unwrap();
    let k2 = pbkdf2::derive_key("password", &[0x02; 16]).unwrap();
    assert_ne!(k1, k2);
}

#[test]
fn e2e_pbkdf2_different_passwords_different_keys() {
    let salt = [0xAB; 16];
    let k1 = pbkdf2::derive_key("pass-a", &salt).unwrap();
    let k2 = pbkdf2::derive_key("pass-b", &salt).unwrap();
    assert_ne!(k1, k2);
}

#[test]
fn e2e_pbkdf2_verify_key() {
    let salt = [0xCD; 16];
    let key = pbkdf2::derive_key("my-pass", &salt).unwrap();
    assert!(pbkdf2::verify_key("my-pass", &salt, &key));
    assert!(!pbkdf2::verify_key("wrong-pass", &salt, &key));
}

// ────────────────────── PKCE Full Flow ──────────────────────

#[test]
fn e2e_pkce_full_flow() {
    // 1. Generate PKCE pair
    let challenge = pkce::generate_pkce_challenge().unwrap();

    // 2. Verify the challenge matches the verifier
    assert!(pkce::verify_pkce(
        &challenge.code_verifier,
        &challenge.code_challenge
    ));

    // 3. Different verifier should fail
    let challenge2 = pkce::generate_pkce_challenge().unwrap();
    assert!(!pkce::verify_pkce(
        &challenge2.code_verifier,
        &challenge.code_challenge
    ));
}

#[test]
fn e2e_pkce_verifier_length_rfc_compliant() {
    let challenge = pkce::generate_pkce_challenge().unwrap();
    let len = challenge.code_verifier.len();
    assert!(
        (43..=128).contains(&len),
        "verifier length {len} not in RFC 7636 range 43-128"
    );
}

#[test]
fn e2e_pkce_challenge_is_base64url() {
    let challenge = pkce::generate_pkce_challenge().unwrap();
    assert!(!challenge.code_challenge.contains('+'));
    assert!(!challenge.code_challenge.contains('/'));
    assert!(!challenge.code_challenge.contains('='));
}

#[test]
fn e2e_pkce_unique_each_time() {
    let c1 = pkce::generate_pkce_challenge().unwrap();
    let c2 = pkce::generate_pkce_challenge().unwrap();
    assert_ne!(c1.code_verifier, c2.code_verifier);
    assert_ne!(c1.code_challenge, c2.code_challenge);
}

// ────────────────────── TOTP Generate & Verify ──────────────────────

fn test_totp_config(secret: &str) -> TotpConfig {
    TotpConfig {
        secret: secret.to_string(),
        digits: 6,
        period: 30,
        algorithm: TotpAlgorithm::Sha1,
    }
}

#[test]
fn e2e_totp_generate_and_verify() {
    let config = test_totp_config("JBSWY3DPEHPK3PXP");

    // 1. Generate code
    let code = totp::generate_totp(&config).unwrap();
    assert_eq!(code.len(), 6);
    assert!(code.chars().all(|c| c.is_ascii_digit()));

    // 2. Verify immediately (within ±1 step window)
    assert!(totp::verify_totp(&config, &code).unwrap());

    // 3. Wrong code should fail
    assert!(!totp::verify_totp(&config, "000000").unwrap());
}

#[test]
fn e2e_totp_rfc6238_test_vector() {
    let config = test_totp_config("GEZDGNBVGY3TQOJQGEZDGNBVGY3TQOJQ");
    // Known value at time step 1: 287082
    let code = totp::generate_totp_at(&config, 1).unwrap();
    assert_eq!(code, "287082");
}

#[test]
fn e2e_totp_deterministic_same_step() {
    let config = test_totp_config("GEZDGNBVGY3TQOJQGEZDGNBVGY3TQOJQ");
    let c1 = totp::generate_totp_at(&config, 100).unwrap();
    let c2 = totp::generate_totp_at(&config, 100).unwrap();
    assert_eq!(c1, c2);
}

#[test]
fn e2e_totp_different_steps_different_codes() {
    let config = test_totp_config("GEZDGNBVGY3TQOJQGEZDGNBVGY3TQOJQ");
    let c1 = totp::generate_totp_at(&config, 100).unwrap();
    let c2 = totp::generate_totp_at(&config, 101).unwrap();
    assert_ne!(c1, c2);
}

#[test]
fn e2e_totp_sha256_variant() {
    let config = TotpConfig {
        secret: "GEZDGNBVGY3TQOJQGEZDGNBVGY3TQOJQ".to_string(),
        digits: 6,
        period: 30,
        algorithm: TotpAlgorithm::Sha256,
    };
    let code = totp::generate_totp_at(&config, 1).unwrap();
    assert_eq!(code.len(), 6);
    assert!(code.chars().all(|c| c.is_ascii_digit()));
}

#[test]
fn e2e_totp_8_digits() {
    let config = TotpConfig {
        secret: "GEZDGNBVGY3TQOJQGEZDGNBVGY3TQOJQ".to_string(),
        digits: 8,
        period: 30,
        algorithm: TotpAlgorithm::Sha1,
    };
    let code = totp::generate_totp_at(&config, 50000).unwrap();
    assert_eq!(code.len(), 8);
}

#[test]
fn e2e_totp_generate_secret_valid_base32() {
    let secret = totp::generate_secret();
    assert!(!secret.is_empty());
    // Should be usable to generate a code
    let config = test_totp_config(&secret);
    let code = totp::generate_totp(&config).unwrap();
    assert_eq!(code.len(), 6);
}

#[test]
fn e2e_totp_invalid_secret_errors() {
    let config = test_totp_config("!!!invalid!!!");
    assert!(totp::generate_totp(&config).is_err());
}

// ────────────────────── Combined: Crypto + Storage Pipeline ──────────────────────

#[test]
fn e2e_store_multiple_encrypted_tokens() {
    let dir = TempDir::new().unwrap();
    let store = EncryptedStore::open(dir.path().join("multi").as_path(), "pw").unwrap();

    let tokens = [
        ("oauth:access", "eyJhbGciOiJSUzI1NiJ9.access-token"),
        ("oauth:refresh", "eyJhbGciOiJSUzI1NiJ9.refresh-token"),
        ("totp:secret", "JBSWY3DPEHPK3PXP"),
    ];

    // Store all tokens
    for (key, value) in &tokens {
        store.set(key, value.as_bytes()).unwrap();
    }

    // Retrieve and verify all
    for (key, expected) in &tokens {
        let retrieved = store.get(key).unwrap().unwrap();
        assert_eq!(
            String::from_utf8(retrieved).unwrap(),
            *expected,
            "mismatch for key {key}"
        );
    }

    // List by prefix
    let oauth_keys = store.list("oauth:").unwrap();
    assert_eq!(oauth_keys.len(), 2);

    let totp_keys = store.list("totp:").unwrap();
    assert_eq!(totp_keys.len(), 1);
}
