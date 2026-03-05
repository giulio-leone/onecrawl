use super::types::*;
use super::stealth_check::supported_types;
use super::solve::base64_decode;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let cfg = CaptchaConfig::default();
        assert!(cfg.auto_detect);
        assert_eq!(cfg.wait_timeout_ms, 30000);
        assert!(cfg.solver_api_key.is_none());
        assert!(cfg.solver_service.is_none());
    }

    #[test]
    fn test_supported_types_count() {
        let types = supported_types();
        assert_eq!(types.len(), 7);
    }

    #[test]
    fn test_supported_types_names() {
        let types = supported_types();
        let names: Vec<&str> = types.iter().map(|(n, _)| n.as_str()).collect();
        assert!(names.contains(&"recaptcha_v2"));
        assert!(names.contains(&"hcaptcha"));
        assert!(names.contains(&"cloudflare_turnstile"));
        assert!(names.contains(&"funcaptcha"));
        assert!(names.contains(&"image"));
    }

    #[test]
    fn test_detection_serialize_none() {
        let det = CaptchaDetection {
            detected: false,
            captcha_type: "none".into(),
            provider: "".into(),
            selector: None,
            sitekey: None,
            confidence: 0.0,
        };
        let json = serde_json::to_string(&det).unwrap();
        assert!(json.contains("\"detected\":false"));
        let parsed: CaptchaDetection = serde_json::from_str(&json).unwrap();
        assert!(!parsed.detected);
        assert_eq!(parsed.captcha_type, "none");
    }

    #[test]
    fn test_detection_serialize_recaptcha() {
        let det = CaptchaDetection {
            detected: true,
            captcha_type: "recaptcha_v2".into(),
            provider: "google".into(),
            selector: Some(".g-recaptcha".into()),
            sitekey: Some("6Le-test".into()),
            confidence: 0.95,
        };
        let json = serde_json::to_string(&det).unwrap();
        let parsed: CaptchaDetection = serde_json::from_str(&json).unwrap();
        assert!(parsed.detected);
        assert_eq!(parsed.sitekey.as_deref(), Some("6Le-test"));
    }

    #[test]
    fn test_result_serialize() {
        let res = CaptchaResult {
            captcha_type: "hcaptcha".into(),
            solved: true,
            solution: Some("token123".into()),
            duration_ms: 1500.0,
            method: "api".into(),
        };
        let json = serde_json::to_string(&res).unwrap();
        let parsed: CaptchaResult = serde_json::from_str(&json).unwrap();
        assert!(parsed.solved);
        assert_eq!(parsed.method, "api");
    }

    #[test]
    fn test_config_serialize() {
        let cfg = CaptchaConfig {
            auto_detect: false,
            wait_timeout_ms: 5000,
            solver_api_key: Some("key123".into()),
            solver_service: Some("2captcha".into()),
        };
        let json = serde_json::to_string(&cfg).unwrap();
        let parsed: CaptchaConfig = serde_json::from_str(&json).unwrap();
        assert!(!parsed.auto_detect);
        assert_eq!(parsed.solver_service.as_deref(), Some("2captcha"));
    }

    #[test]
    fn test_detection_all_types_deserialize() {
        for captcha_type in &[
            "recaptcha_v2",
            "recaptcha_v3",
            "hcaptcha",
            "cloudflare_turnstile",
            "funcaptcha",
            "text",
            "image",
            "unknown",
            "none",
        ] {
            let json = format!(
                r#"{{"detected":true,"captcha_type":"{}","provider":"test","selector":null,"sitekey":null,"confidence":0.5}}"#,
                captcha_type
            );
            let parsed: CaptchaDetection = serde_json::from_str(&json).unwrap();
            assert_eq!(parsed.captcha_type, *captcha_type);
        }
    }

    #[test]
    fn test_base64_decode_simple() {
        // "hello" in base64 is "aGVsbG8="
        let decoded = base64_decode("aGVsbG8=").unwrap();
        assert_eq!(decoded, b"hello");
    }

    #[test]
    fn test_base64_decode_no_padding() {
        // "hello" without padding
        let decoded = base64_decode("aGVsbG8").unwrap();
        assert_eq!(decoded, b"hello");
    }

    #[test]
    fn test_base64_decode_empty() {
        let decoded = base64_decode("").unwrap();
        assert!(decoded.is_empty());
    }

    #[test]
    fn test_base64_decode_binary() {
        // Known binary: [0xFF, 0x00, 0xAB] = "/wCr"
        let decoded = base64_decode("/wCr").unwrap();
        assert_eq!(decoded, vec![0xFF, 0x00, 0xAB]);
    }

    #[test]
    fn test_base64_decode_with_newlines() {
        // Decoder should skip \n and \r
        let decoded = base64_decode("aGVs\nbG8=\r").unwrap();
        assert_eq!(decoded, b"hello");
    }

    #[test]
    fn test_base64_decode_invalid_char() {
        let result = base64_decode("aGVs!G8=");
        assert!(result.is_err());
    }
}
