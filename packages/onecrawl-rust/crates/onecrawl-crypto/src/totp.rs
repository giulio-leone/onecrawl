use data_encoding::BASE32_NOPAD;
use onecrawl_core::{Error, Result, TotpAlgorithm, TotpConfig};
use ring::hmac;

/// Generate a TOTP code per RFC 6238.
///
/// Default: SHA-1, 6 digits, 30-second period (LinkedIn-compatible).
pub fn generate_totp(config: &TotpConfig) -> Result<String> {
    if config.period == 0 {
        return Err(Error::Crypto("TOTP period must be > 0".into()));
    }
    generate_totp_at(config, current_time_step(config.period))
}

/// Generate TOTP at a specific time step (for testing).
pub fn generate_totp_at(config: &TotpConfig, time_step: u64) -> Result<String> {
    if config.digits == 0 || config.digits > 9 {
        return Err(Error::Crypto("TOTP digits must be 1-9".into()));
    }
    let secret = BASE32_NOPAD
        .decode(config.secret.to_uppercase().as_bytes())
        .map_err(|e| Error::Crypto(format!("invalid base32 secret: {e}")))?;

    let algorithm = match config.algorithm {
        TotpAlgorithm::Sha1 => hmac::HMAC_SHA1_FOR_LEGACY_USE_ONLY,
        TotpAlgorithm::Sha256 => hmac::HMAC_SHA256,
        TotpAlgorithm::Sha512 => hmac::HMAC_SHA512,
    };

    let key = hmac::Key::new(algorithm, &secret);
    let msg = time_step.to_be_bytes();
    let tag = hmac::sign(&key, &msg);
    let hash = tag.as_ref();

    let offset = (hash[hash.len() - 1] & 0x0F) as usize;
    let binary = u32::from_be_bytes([
        hash[offset] & 0x7F,
        hash[offset + 1],
        hash[offset + 2],
        hash[offset + 3],
    ]);

    let modulus = 10u32.pow(config.digits);
    let code = binary % modulus;

    Ok(format!("{:0>width$}", code, width = config.digits as usize))
}

/// Verify a TOTP code with a ±1 step window.
pub fn verify_totp(config: &TotpConfig, code: &str) -> Result<bool> {
    if config.period == 0 {
        return Err(Error::Crypto("TOTP period must be > 0".into()));
    }
    let current = current_time_step(config.period);

    for offset in [0i64, -1, 1] {
        let step = (current as i64 + offset) as u64;
        let generated = generate_totp_at(config, step)?;
        if constant_time_eq(generated.as_bytes(), code.as_bytes()) {
            return Ok(true);
        }
    }

    Ok(false)
}

fn current_time_step(period: u32) -> u64 {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs();
    now / period as u64
}

fn constant_time_eq(a: &[u8], b: &[u8]) -> bool {
    if a.len() != b.len() {
        return false;
    }
    // Constant-time comparison to prevent timing attacks
    a.iter()
        .zip(b.iter())
        .fold(0u8, |acc, (x, y)| acc | (x ^ y))
        == 0
}

/// Generate a random base32-encoded TOTP secret (20 bytes = 160 bits).
pub fn generate_secret() -> String {
    let mut bytes = [0u8; 20];
    rand::RngCore::fill_bytes(&mut rand::rng(), &mut bytes);
    BASE32_NOPAD.encode(&bytes)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_config(secret: &str) -> TotpConfig {
        TotpConfig {
            secret: secret.to_string(),
            digits: 6,
            period: 30,
            algorithm: TotpAlgorithm::Sha1,
        }
    }

    #[test]
    fn totp_rfc6238_test_vector() {
        // RFC 6238 test: secret "12345678901234567890" (ASCII) = GEZDGNBVGY3TQOJQGEZDGNBVGY3TQOJQ (base32)
        let config = test_config("GEZDGNBVGY3TQOJQGEZDGNBVGY3TQOJQ");

        // Time step 1 (T = 30-59 seconds since epoch, step = 1)
        let code = generate_totp_at(&config, 1).unwrap();
        assert_eq!(code.len(), 6);
        // Known value at T=1: 287082
        assert_eq!(code, "287082");
    }

    #[test]
    fn totp_deterministic_same_step() {
        let config = test_config("GEZDGNBVGY3TQOJQGEZDGNBVGY3TQOJQ");
        let c1 = generate_totp_at(&config, 100).unwrap();
        let c2 = generate_totp_at(&config, 100).unwrap();
        assert_eq!(c1, c2);
    }

    #[test]
    fn totp_different_steps_different_codes() {
        let config = test_config("GEZDGNBVGY3TQOJQGEZDGNBVGY3TQOJQ");
        let c1 = generate_totp_at(&config, 100).unwrap();
        let c2 = generate_totp_at(&config, 101).unwrap();
        assert_ne!(c1, c2);
    }

    #[test]
    fn totp_6_digits() {
        let config = test_config("GEZDGNBVGY3TQOJQGEZDGNBVGY3TQOJQ");
        let code = generate_totp_at(&config, 50000).unwrap();
        assert_eq!(code.len(), 6);
        assert!(code.chars().all(|c| c.is_ascii_digit()));
    }

    #[test]
    fn totp_8_digits() {
        let config = TotpConfig {
            secret: "GEZDGNBVGY3TQOJQGEZDGNBVGY3TQOJQ".to_string(),
            digits: 8,
            period: 30,
            algorithm: TotpAlgorithm::Sha1,
        };
        let code = generate_totp_at(&config, 50000).unwrap();
        assert_eq!(code.len(), 8);
    }

    #[test]
    fn totp_verify_current_step() {
        let config = test_config("GEZDGNBVGY3TQOJQGEZDGNBVGY3TQOJQ");
        let code = generate_totp(&config).unwrap();
        assert!(verify_totp(&config, &code).unwrap());
    }

    #[test]
    fn totp_invalid_secret() {
        let config = test_config("!!!invalid!!!");
        assert!(generate_totp_at(&config, 1).is_err());
    }

    #[test]
    fn totp_sha256() {
        let config = TotpConfig {
            secret: "GEZDGNBVGY3TQOJQGEZDGNBVGY3TQOJQ".to_string(),
            digits: 6,
            period: 30,
            algorithm: TotpAlgorithm::Sha256,
        };
        let code = generate_totp_at(&config, 1).unwrap();
        assert_eq!(code.len(), 6);
    }

    #[test]
    fn generate_secret_valid_base32() {
        let secret = generate_secret();
        assert!(!secret.is_empty());
        assert!(BASE32_NOPAD.decode(secret.as_bytes()).is_ok());
    }
}
