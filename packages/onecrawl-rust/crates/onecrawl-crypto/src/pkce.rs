use base64::{Engine as _, engine::general_purpose::URL_SAFE_NO_PAD as B64URL};
use onecrawl_core::{PkceChallenge, Result};
use rand::RngCore;
use ring::digest;

/// Generate a PKCE code_verifier and S256 code_challenge.
///
/// Per RFC 7636: code_verifier is 43-128 chars of [A-Z][a-z][0-9]-._~
/// code_challenge = BASE64URL(SHA256(code_verifier))
pub fn generate_pkce_challenge() -> Result<PkceChallenge> {
    let mut random_bytes = [0u8; 32];
    rand::rng().fill_bytes(&mut random_bytes);

    let code_verifier = B64URL.encode(random_bytes);
    let digest = digest::digest(&digest::SHA256, code_verifier.as_bytes());
    let code_challenge = B64URL.encode(digest.as_ref());

    Ok(PkceChallenge {
        code_verifier,
        code_challenge,
    })
}

/// Verify a code_challenge matches the code_verifier using S256.
pub fn verify_pkce(code_verifier: &str, expected_challenge: &str) -> bool {
    let digest = digest::digest(&digest::SHA256, code_verifier.as_bytes());
    let computed_challenge = B64URL.encode(digest.as_ref());
    computed_challenge == expected_challenge
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pkce_roundtrip() {
        let challenge = generate_pkce_challenge().unwrap();
        assert!(verify_pkce(
            &challenge.code_verifier,
            &challenge.code_challenge
        ));
    }

    #[test]
    fn pkce_verifier_length() {
        let challenge = generate_pkce_challenge().unwrap();
        let len = challenge.code_verifier.len();
        assert!(
            (43..=128).contains(&len),
            "verifier length {len} not in 43-128"
        );
    }

    #[test]
    fn pkce_different_each_time() {
        let c1 = generate_pkce_challenge().unwrap();
        let c2 = generate_pkce_challenge().unwrap();
        assert_ne!(c1.code_verifier, c2.code_verifier);
        assert_ne!(c1.code_challenge, c2.code_challenge);
    }

    #[test]
    fn pkce_wrong_verifier_fails() {
        let challenge = generate_pkce_challenge().unwrap();
        assert!(!verify_pkce("wrong-verifier", &challenge.code_challenge));
    }

    #[test]
    fn pkce_challenge_is_base64url() {
        let challenge = generate_pkce_challenge().unwrap();
        // Base64URL: no +, /, or = characters
        assert!(!challenge.code_challenge.contains('+'));
        assert!(!challenge.code_challenge.contains('/'));
        assert!(!challenge.code_challenge.contains('='));
    }
}
