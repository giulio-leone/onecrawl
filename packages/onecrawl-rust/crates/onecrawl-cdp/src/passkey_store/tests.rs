use super::vault::*;
use super::importers::*;
use crate::webauthn::PasskeyCredential;

fn sample_cred(id: &str, rp_id: &str) -> PasskeyCredential {
    PasskeyCredential {
        credential_id: id.to_string(),
        private_key: "dGVzdA==".to_string(),
        rp_id: rp_id.to_string(),
        user_handle: None,
        sign_count: 0,
        is_resident_credential: true,
    }
}

// ── Vault operations ──────────────────────────────────────────────────────

#[test]
fn vault_add_and_get() {
    let mut v = PasskeyVault::default();
    vault_add(&mut v, vec![sample_cred("aaa", "x.com")]);
    let got = vault_get(&v, "x.com");
    assert_eq!(got.len(), 1);
    assert_eq!(got[0].credential_id, "aaa");
}

#[test]
fn vault_add_deduplicates_by_credential_id() {
    let mut v = PasskeyVault::default();
    vault_add(&mut v, vec![sample_cred("aaa", "x.com")]);
    vault_add(&mut v, vec![sample_cred("aaa", "x.com")]); // duplicate
    vault_add(&mut v, vec![sample_cred("bbb", "x.com")]);
    assert_eq!(vault_get(&v, "x.com").len(), 2);
}

#[test]
fn vault_remove_by_credential_id() {
    let mut v = PasskeyVault::default();
    vault_add(&mut v, vec![sample_cred("aaa", "x.com"), sample_cred("bbb", "x.com")]);
    assert!(vault_remove(&mut v, "aaa"));
    assert_eq!(vault_get(&v, "x.com").len(), 1);
}

#[test]
fn vault_remove_cleans_empty_entries() {
    let mut v = PasskeyVault::default();
    vault_add(&mut v, vec![sample_cred("aaa", "x.com")]);
    vault_remove(&mut v, "aaa");
    assert!(v.credentials.is_empty());
}

#[test]
fn vault_clear_site() {
    let mut v = PasskeyVault::default();
    vault_add(&mut v, vec![sample_cred("aaa", "x.com"), sample_cred("bbb", "github.com")]);
    let removed = super::vault_clear_site(&mut v, "x.com");
    assert_eq!(removed, 1);
    assert!(vault_get(&v, "x.com").is_empty());
    assert_eq!(vault_get(&v, "github.com").len(), 1);
}

#[test]
fn vault_list_sorted() {
    let mut v = PasskeyVault::default();
    vault_add(&mut v, vec![sample_cred("a", "z.com"), sample_cred("b", "a.com")]);
    let list = vault_list(&v);
    assert_eq!(list[0].0, "a.com");
    assert_eq!(list[1].0, "z.com");
}

#[test]
fn vault_total() {
    let mut v = PasskeyVault::default();
    vault_add(&mut v, vec![sample_cred("a", "x.com"), sample_cred("b", "y.com"), sample_cred("c", "y.com")]);
    assert_eq!(super::vault_total(&v), 3);
}

#[test]
fn vault_roundtrip_json() {
    let mut v = PasskeyVault::default();
    vault_add(&mut v, vec![sample_cred("cred1", "example.com")]);
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("vault.json");
    let json = serde_json::to_string_pretty(&v).unwrap();
    std::fs::write(&path, &json).unwrap();
    let loaded: PasskeyVault = serde_json::from_str(&json).unwrap();
    assert_eq!(vault_get(&loaded, "example.com").len(), 1);
}

// ── Bitwarden import ──────────────────────────────────────────────────────

#[test]
fn import_bitwarden_parses_fido2_credentials() {
    let json = r#"{
        "items": [{
            "name": "X.com",
            "type": 1,
            "login": {
                "fido2Credentials": [{
                    "credentialId": "Y3JlZElk",
                    "keyValue": "cHJpdktleQ==",
                    "rpId": "x.com",
                    "userHandle": "dXNlckhhbmRsZQ==",
                    "counter": "5",
                    "discoverable": "true"
                }]
            }
        }]
    }"#;
    let dir = tempfile::tempdir().unwrap();
    let p = dir.path().join("bw.json");
    std::fs::write(&p, json).unwrap();
    let creds = import_bitwarden(&p).unwrap();
    assert_eq!(creds.len(), 1);
    assert_eq!(creds[0].credential_id, "Y3JlZElk");
    assert_eq!(creds[0].private_key, "cHJpdktleQ==");
    assert_eq!(creds[0].rp_id, "x.com");
    assert_eq!(creds[0].sign_count, 5);
    assert!(creds[0].is_resident_credential);
}

#[test]
fn import_bitwarden_skips_no_private_key() {
    let json = r#"{"items": [{"login": {"fido2Credentials": [{"credentialId": "abc", "rpId": "x.com", "counter": "0", "discoverable": "true"}]}}]}"#;
    let dir = tempfile::tempdir().unwrap();
    let p = dir.path().join("bw.json");
    std::fs::write(&p, json).unwrap();
    let creds = import_bitwarden(&p).unwrap();
    assert!(creds.is_empty(), "hardware-bound cred should be skipped");
}

#[test]
fn import_bitwarden_empty_items() {
    let json = r#"{"items": []}"#;
    let dir = tempfile::tempdir().unwrap();
    let p = dir.path().join("bw.json");
    std::fs::write(&p, json).unwrap();
    assert!(import_bitwarden(&p).unwrap().is_empty());
}

// ── 1Password import ──────────────────────────────────────────────────────

#[test]
fn import_1password_json_parses_passkey_category() {
    let json = r#"{
        "accounts": [{
            "vaults": [{
                "items": [{
                    "overview": {"categoryUuid": "119"},
                    "details": {
                        "loginFields": [
                            {"name": "credentialId", "value": "Y3JlZElk"},
                            {"name": "keyValue",     "value": "cHJpdktleQ=="},
                            {"name": "rpId",         "value": "x.com"},
                            {"name": "userHandle",   "value": "dXNlcg=="},
                            {"name": "counter",      "value": "3"}
                        ]
                    }
                }]
            }]
        }]
    }"#;
    let dir = tempfile::tempdir().unwrap();
    let p = dir.path().join("export.data");
    std::fs::write(&p, json).unwrap();
    let creds = import_1password_json(&p).unwrap();
    assert_eq!(creds.len(), 1);
    assert_eq!(creds[0].rp_id, "x.com");
    assert_eq!(creds[0].sign_count, 3);
}

#[test]
fn import_1password_json_skips_non_passkey_items() {
    let json = r#"{"accounts":[{"vaults":[{"items":[{"overview":{"categoryUuid":"001"},"details":{"loginFields":[]}}]}]}]}"#;
    let dir = tempfile::tempdir().unwrap();
    let p = dir.path().join("export.data");
    std::fs::write(&p, json).unwrap();
    assert!(import_1password_json(&p).unwrap().is_empty());
}

// ── FIDO CXF import ───────────────────────────────────────────────────────

#[test]
fn import_cxf_parses_passkey_entries() {
    let json = r#"{
        "version": "1.0",
        "credentials": [{
            "type": "passkey",
            "data": {
                "id": "Y3JlZElk",
                "relyingPartyId": "x.com",
                "userId": "dXNlcg==",
                "userName": "user@x.com",
                "privateKey": "cHJpdktleQ==",
                "signatureCounter": 7
            }
        }]
    }"#;
    let dir = tempfile::tempdir().unwrap();
    let p = dir.path().join("cxf.json");
    std::fs::write(&p, json).unwrap();
    let creds = import_cxf(&p).unwrap();
    assert_eq!(creds.len(), 1);
    assert_eq!(creds[0].rp_id, "x.com");
    assert_eq!(creds[0].sign_count, 7);
}

#[test]
fn import_cxf_skips_non_passkey_type() {
    let json = r#"{"version":"1.0","credentials":[{"type":"totp","data":{"id":"abc","relyingPartyId":"x.com","privateKey":"key"}}]}"#;
    let dir = tempfile::tempdir().unwrap();
    let p = dir.path().join("cxf.json");
    std::fs::write(&p, json).unwrap();
    assert!(import_cxf(&p).unwrap().is_empty());
}

#[test]
fn import_cxf_skips_no_private_key() {
    let json = r#"{"version":"1.0","credentials":[{"type":"passkey","data":{"id":"abc","relyingPartyId":"x.com"}}]}"#;
    let dir = tempfile::tempdir().unwrap();
    let p = dir.path().join("cxf.json");
    std::fs::write(&p, json).unwrap();
    assert!(import_cxf(&p).unwrap().is_empty());
}
