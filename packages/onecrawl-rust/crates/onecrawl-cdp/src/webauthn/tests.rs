use super::types::*;
use super::cdp_backend::*;
use super::js_backend::*;

fn default_authenticator() -> VirtualAuthenticator {
    VirtualAuthenticator {
        id: "auth-1".into(),
        protocol: "ctap2".into(),
        transport: "internal".into(),
        has_resident_key: true,
        has_user_verification: true,
        is_user_verified: true,
    }
}

fn default_credential() -> VirtualCredential {
    VirtualCredential {
        credential_id: "Y3JlZC0x".into(),
        rp_id: "example.com".into(),
        user_handle: "dXNlci0x".into(),
        sign_count: 0,
    }
}

#[test]
fn test_virtual_authenticator_default() {
    let auth = default_authenticator();
    assert_eq!(auth.id, "auth-1");
    assert_eq!(auth.protocol, "ctap2");
    assert_eq!(auth.transport, "internal");
    assert!(auth.has_resident_key);
    assert!(auth.has_user_verification);
    assert!(auth.is_user_verified);
}

#[test]
fn test_virtual_credential_serialization() {
    let cred = default_credential();
    let json = serde_json::to_string(&cred).unwrap();
    let parsed: VirtualCredential = serde_json::from_str(&json).unwrap();
    assert_eq!(cred, parsed);
}

#[test]
fn test_authenticator_config_json() {
    let auth = default_authenticator();
    let json_val: serde_json::Value = serde_json::to_value(&auth).unwrap();
    assert_eq!(json_val["id"], "auth-1");
    assert_eq!(json_val["protocol"], "ctap2");
    assert_eq!(json_val["transport"], "internal");
    assert_eq!(json_val["has_resident_key"], true);
    assert_eq!(json_val["has_user_verification"], true);
    assert_eq!(json_val["is_user_verified"], true);
}

#[test]
fn test_credential_roundtrip() {
    let cred = VirtualCredential {
        credential_id: "abc123".into(),
        rp_id: "mysite.org".into(),
        user_handle: "aGFuZGxl".into(),
        sign_count: 42,
    };
    let serialized = serde_json::to_string(&cred).unwrap();
    let deserialized: VirtualCredential = serde_json::from_str(&serialized).unwrap();
    assert_eq!(cred, deserialized);
    assert_eq!(deserialized.sign_count, 42);
}

#[test]
fn test_multiple_credentials() {
    let creds = vec![
        VirtualCredential {
            credential_id: "c1".into(),
            rp_id: "a.com".into(),
            user_handle: "u1".into(),
            sign_count: 0,
        },
        VirtualCredential {
            credential_id: "c2".into(),
            rp_id: "b.com".into(),
            user_handle: "u2".into(),
            sign_count: 5,
        },
        VirtualCredential {
            credential_id: "c3".into(),
            rp_id: "a.com".into(),
            user_handle: "u3".into(),
            sign_count: 10,
        },
    ];
    let json = serde_json::to_string(&creds).unwrap();
    let parsed: Vec<VirtualCredential> = serde_json::from_str(&json).unwrap();
    assert_eq!(parsed.len(), 3);
    let a_com: Vec<_> = parsed.iter().filter(|c| c.rp_id == "a.com").collect();
    assert_eq!(a_com.len(), 2);
}

#[test]
fn test_credential_empty_fields() {
    let cred = VirtualCredential {
        credential_id: String::new(),
        rp_id: String::new(),
        user_handle: String::new(),
        sign_count: 0,
    };
    let json = serde_json::to_string(&cred).unwrap();
    let parsed: VirtualCredential = serde_json::from_str(&json).unwrap();
    assert_eq!(parsed.credential_id, "");
    assert_eq!(parsed.rp_id, "");
    assert_eq!(parsed.user_handle, "");
    assert_eq!(parsed.sign_count, 0);
}

#[test]
fn test_credential_special_characters() {
    let cred = VirtualCredential {
        credential_id: "abc+/=def".into(),
        rp_id: "example.com".into(),
        user_handle: "user\"with'quotes".into(),
        sign_count: 0,
    };
    let json = serde_json::to_string(&cred).unwrap();
    let parsed: VirtualCredential = serde_json::from_str(&json).unwrap();
    assert_eq!(cred, parsed);
}

#[test]
fn test_credential_sign_count_max() {
    let cred = VirtualCredential {
        credential_id: "c1".into(),
        rp_id: "a.com".into(),
        user_handle: "u1".into(),
        sign_count: u32::MAX,
    };
    let json = serde_json::to_string(&cred).unwrap();
    let parsed: VirtualCredential = serde_json::from_str(&json).unwrap();
    assert_eq!(parsed.sign_count, u32::MAX);
}

#[test]
fn test_authenticator_all_transports() {
    for transport in &["usb", "nfc", "ble", "internal"] {
        let auth = VirtualAuthenticator {
            id: format!("auth-{transport}"),
            protocol: "ctap2".into(),
            transport: (*transport).into(),
            has_resident_key: true,
            has_user_verification: true,
            is_user_verified: true,
        };
        let json = serde_json::to_string(&auth).unwrap();
        let parsed: VirtualAuthenticator = serde_json::from_str(&json).unwrap();
        assert_eq!(auth, parsed);
        assert_eq!(parsed.transport, *transport);
    }
}

#[test]
fn test_authenticator_minimal_config() {
    let auth = VirtualAuthenticator {
        id: String::new(),
        protocol: "u2f".into(),
        transport: "usb".into(),
        has_resident_key: false,
        has_user_verification: false,
        is_user_verified: false,
    };
    let json_val: serde_json::Value = serde_json::to_value(&auth).unwrap();
    assert_eq!(json_val["has_resident_key"], false);
    assert_eq!(json_val["has_user_verification"], false);
    assert_eq!(json_val["is_user_verified"], false);
}

#[test]
fn test_credential_deserialize_from_js_format() {
    // Simulate what the browser JS returns
    let js_json = r#"{
        "credential_id": "Y3JlZC0x",
        "rp_id": "example.com",
        "user_handle": "dXNlci0x",
        "sign_count": 3
    }"#;
    let cred: VirtualCredential = serde_json::from_str(js_json).unwrap();
    assert_eq!(cred.credential_id, "Y3JlZC0x");
    assert_eq!(cred.rp_id, "example.com");
    assert_eq!(cred.sign_count, 3);
}

#[test]
fn test_multiple_credentials_filter_by_rp_id() {
    let creds = vec![
        VirtualCredential { credential_id: "c1".into(), rp_id: "a.com".into(), user_handle: "u1".into(), sign_count: 0 },
        VirtualCredential { credential_id: "c2".into(), rp_id: "b.com".into(), user_handle: "u2".into(), sign_count: 5 },
        VirtualCredential { credential_id: "c3".into(), rp_id: "a.com".into(), user_handle: "u3".into(), sign_count: 10 },
        VirtualCredential { credential_id: "c4".into(), rp_id: "c.com".into(), user_handle: "u4".into(), sign_count: 1 },
    ];
    let a_com: Vec<_> = creds.iter().filter(|c| c.rp_id == "a.com").collect();
    assert_eq!(a_com.len(), 2);
    let b_com: Vec<_> = creds.iter().filter(|c| c.rp_id == "b.com").collect();
    assert_eq!(b_com.len(), 1);
    let d_com: Vec<_> = creds.iter().filter(|c| c.rp_id == "d.com").collect();
    assert_eq!(d_com.len(), 0);
}

#[test]
fn test_credential_removal_simulation() {
    let mut creds = vec![
        VirtualCredential { credential_id: "c1".into(), rp_id: "a.com".into(), user_handle: "u1".into(), sign_count: 0 },
        VirtualCredential { credential_id: "c2".into(), rp_id: "b.com".into(), user_handle: "u2".into(), sign_count: 5 },
    ];
    let target = "c1";
    let before = creds.len();
    creds.retain(|c| c.credential_id != target);
    assert_eq!(creds.len(), before - 1);
    assert!(creds.iter().all(|c| c.credential_id != target));
}

#[test]
fn test_authenticator_equality() {
    let a = default_authenticator();
    let b = default_authenticator();
    assert_eq!(a, b);
    let mut c = default_authenticator();
    c.protocol = "u2f".into();
    assert_ne!(a, c);
}

#[test]
fn test_virtual_authenticator_protocols() {
    let ctap2 = VirtualAuthenticator {
        id: "ctap2-auth".into(),
        protocol: "ctap2".into(),
        transport: "usb".into(),
        has_resident_key: true,
        has_user_verification: true,
        is_user_verified: false,
    };
    let u2f = VirtualAuthenticator {
        id: "u2f-auth".into(),
        protocol: "u2f".into(),
        transport: "nfc".into(),
        has_resident_key: false,
        has_user_verification: false,
        is_user_verified: false,
    };
    assert_eq!(ctap2.protocol, "ctap2");
    assert_eq!(u2f.protocol, "u2f");
    assert_ne!(ctap2, u2f);
    // Verify serialization preserves protocol variants
    let ctap2_json: serde_json::Value = serde_json::to_value(&ctap2).unwrap();
    let u2f_json: serde_json::Value = serde_json::to_value(&u2f).unwrap();
    assert_eq!(ctap2_json["protocol"], "ctap2");
    assert_eq!(u2f_json["protocol"], "u2f");
    assert_eq!(ctap2_json["transport"], "usb");
    assert_eq!(u2f_json["transport"], "nfc");
}

// ── PasskeyCredential (CDP-native) tests ─────────────────────────────────

fn sample_passkey() -> PasskeyCredential {
    PasskeyCredential {
        credential_id: "Y3JlZElk".to_string(),
        private_key: "cHJpdmF0ZUtleQ==".to_string(),
        rp_id: "x.com".to_string(),
        user_handle: Some("dXNlckhhbmRsZQ==".to_string()),
        sign_count: 0,
        is_resident_credential: true,
    }
}

#[test]
fn test_passkey_credential_roundtrip() {
    let cred = sample_passkey();
    let json = serde_json::to_string(&cred).unwrap();
    let parsed: PasskeyCredential = serde_json::from_str(&json).unwrap();
    assert_eq!(cred, parsed);
}

#[test]
fn test_passkey_credential_no_user_handle() {
    let cred = PasskeyCredential {
        credential_id: "abc".into(),
        private_key: "def".into(),
        rp_id: "twitter.com".into(),
        user_handle: None,
        sign_count: 5,
        is_resident_credential: false,
    };
    let json = serde_json::to_string(&cred).unwrap();
    // user_handle must be omitted from JSON when None
    assert!(!json.contains("user_handle"));
    let parsed: PasskeyCredential = serde_json::from_str(&json).unwrap();
    assert_eq!(parsed.user_handle, None);
    assert_eq!(parsed.sign_count, 5);
}

#[test]
fn test_save_load_passkeys_roundtrip() {
    let creds = vec![sample_passkey(), PasskeyCredential {
        credential_id: "id2".into(),
        private_key: "key2".into(),
        rp_id: "x.com".into(),
        user_handle: None,
        sign_count: 10,
        is_resident_credential: true,
    }];
    let dir = tempfile::tempdir().expect("temp dir");
    let path = dir.path().join("passkeys.json");
    save_passkeys(&path, &creds).expect("save_passkeys");
    let loaded = load_passkeys(&path).expect("load_passkeys");
    assert_eq!(creds, loaded);
}

#[test]
fn test_load_passkeys_invalid_json() {
    let dir = tempfile::tempdir().expect("temp dir");
    let path = dir.path().join("bad.json");
    std::fs::write(&path, b"not json").unwrap();
    assert!(load_passkeys(&path).is_err());
}

#[test]
fn test_load_passkeys_missing_file() {
    let path = std::path::Path::new("/tmp/__onecrawl_no_such_file__.json");
    assert!(load_passkeys(path).is_err());
}

#[test]
fn test_passkey_credential_field_names_in_json() {
    let cred = sample_passkey();
    let val: serde_json::Value = serde_json::to_value(&cred).unwrap();
    assert!(val.get("credential_id").is_some());
    assert!(val.get("private_key").is_some());
    assert!(val.get("rp_id").is_some());
    assert!(val.get("sign_count").is_some());
    assert!(val.get("is_resident_credential").is_some());
}
