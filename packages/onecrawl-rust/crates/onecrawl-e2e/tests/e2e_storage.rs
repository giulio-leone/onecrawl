//! E2E tests for encrypted storage.
//! Tests persistence, isolation, bulk operations, and edge cases.

use onecrawl_storage::EncryptedStore;
use tempfile::TempDir;

// ────────────────────── Persistence ──────────────────────

#[test]
fn e2e_storage_persistence() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("persistent_db");

    // 1. Write data and flush
    {
        let store = EncryptedStore::open(&path, "password").unwrap();
        store.set("key1", b"value1").unwrap();
        store.set("key2", b"value2").unwrap();
        store.flush().unwrap();
    }

    // 2. Re-open and verify data persists
    {
        let store = EncryptedStore::open(&path, "password").unwrap();
        assert_eq!(store.get("key1").unwrap().unwrap(), b"value1");
        assert_eq!(store.get("key2").unwrap().unwrap(), b"value2");
    }
}

#[test]
fn e2e_storage_persistence_after_delete() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("delete_persist");

    {
        let store = EncryptedStore::open(&path, "pw").unwrap();
        store.set("a", b"1").unwrap();
        store.set("b", b"2").unwrap();
        store.delete("a").unwrap();
        store.flush().unwrap();
    }

    {
        let store = EncryptedStore::open(&path, "pw").unwrap();
        assert!(store.get("a").unwrap().is_none());
        assert_eq!(store.get("b").unwrap().unwrap(), b"2");
    }
}

// ────────────────────── Isolation ──────────────────────

#[test]
fn e2e_storage_isolation() {
    let dir = TempDir::new().unwrap();

    // Two stores with different passwords at different paths
    let store_a = EncryptedStore::open(dir.path().join("a").as_path(), "pw-a").unwrap();
    let store_b = EncryptedStore::open(dir.path().join("b").as_path(), "pw-b").unwrap();

    store_a.set("shared_key", b"value_a").unwrap();
    store_b.set("shared_key", b"value_b").unwrap();

    assert_eq!(store_a.get("shared_key").unwrap().unwrap(), b"value_a");
    assert_eq!(store_b.get("shared_key").unwrap().unwrap(), b"value_b");
}

#[test]
fn e2e_storage_temp_store_isolation() {
    let store_a = EncryptedStore::open_temp("pw-a").unwrap();
    let store_b = EncryptedStore::open_temp("pw-b").unwrap();

    store_a.set("key", b"A").unwrap();
    store_b.set("key", b"B").unwrap();

    assert_eq!(store_a.get("key").unwrap().unwrap(), b"A");
    assert_eq!(store_b.get("key").unwrap().unwrap(), b"B");
}

// ────────────────────── Bulk Operations ──────────────────────

#[test]
fn e2e_storage_bulk_operations() {
    let dir = TempDir::new().unwrap();
    let store = EncryptedStore::open(dir.path().join("bulk").as_path(), "pw").unwrap();

    // Write 100 entries
    for i in 0..100 {
        store
            .set(&format!("key-{i:03}"), format!("value-{i}").as_bytes())
            .unwrap();
    }

    // Verify all exist
    let keys = store.list("key-").unwrap();
    assert_eq!(keys.len(), 100);

    // Verify specific entries
    assert_eq!(
        store.get("key-000").unwrap().unwrap(),
        b"value-0"
    );
    assert_eq!(
        store.get("key-099").unwrap().unwrap(),
        b"value-99"
    );

    // Delete first 50
    for i in 0..50 {
        assert!(store.delete(&format!("key-{i:03}")).unwrap());
    }

    let remaining = store.list("key-").unwrap();
    assert_eq!(remaining.len(), 50);

    // Verify deleted entries are gone
    assert!(store.get("key-000").unwrap().is_none());
    assert!(!store.contains("key-049").unwrap());

    // Verify remaining entries still exist
    assert!(store.contains("key-050").unwrap());
    assert_eq!(
        store.get("key-050").unwrap().unwrap(),
        b"value-50"
    );
}

// ────────────────────── Key Prefix Listing ──────────────────────

#[test]
fn e2e_storage_prefix_listing() {
    let store = EncryptedStore::open_temp("pw").unwrap();

    store.set("oauth:token:access", b"at").unwrap();
    store.set("oauth:token:refresh", b"rt").unwrap();
    store.set("oauth:pkce:verifier", b"v").unwrap();
    store.set("cookie:li_at", b"c1").unwrap();
    store.set("cookie:JSESSIONID", b"c2").unwrap();

    let oauth = store.list("oauth:").unwrap();
    assert_eq!(oauth.len(), 3);

    let oauth_token = store.list("oauth:token:").unwrap();
    assert_eq!(oauth_token.len(), 2);

    let cookies = store.list("cookie:").unwrap();
    assert_eq!(cookies.len(), 2);

    // Empty prefix lists all
    let all = store.list("").unwrap();
    assert_eq!(all.len(), 5);
}

// ────────────────────── Edge Cases ──────────────────────

#[test]
fn e2e_storage_overwrite() {
    let store = EncryptedStore::open_temp("pw").unwrap();
    store.set("key", b"v1").unwrap();
    store.set("key", b"v2").unwrap();
    assert_eq!(store.get("key").unwrap().unwrap(), b"v2");
}

#[test]
fn e2e_storage_get_nonexistent() {
    let store = EncryptedStore::open_temp("pw").unwrap();
    assert!(store.get("nonexistent").unwrap().is_none());
}

#[test]
fn e2e_storage_delete_nonexistent() {
    let store = EncryptedStore::open_temp("pw").unwrap();
    assert!(!store.delete("nonexistent").unwrap());
}

#[test]
fn e2e_storage_contains() {
    let store = EncryptedStore::open_temp("pw").unwrap();
    store.set("exists", b"yes").unwrap();
    assert!(store.contains("exists").unwrap());
    assert!(!store.contains("nope").unwrap());
}

#[test]
fn e2e_storage_empty_value() {
    let store = EncryptedStore::open_temp("pw").unwrap();
    store.set("empty", b"").unwrap();
    let val = store.get("empty").unwrap().unwrap();
    assert!(val.is_empty());
}

#[test]
fn e2e_storage_large_value() {
    let store = EncryptedStore::open_temp("pw").unwrap();
    let data = vec![0xABu8; 100_000];
    store.set("big", &data).unwrap();
    let retrieved = store.get("big").unwrap().unwrap();
    assert_eq!(retrieved, data);
}

#[test]
fn e2e_storage_binary_value() {
    let store = EncryptedStore::open_temp("pw").unwrap();
    let data: Vec<u8> = (0..=255).collect();
    store.set("binary", &data).unwrap();
    let retrieved = store.get("binary").unwrap().unwrap();
    assert_eq!(retrieved, data);
}

#[test]
fn e2e_storage_special_key_chars() {
    let store = EncryptedStore::open_temp("pw").unwrap();
    let keys = ["key/with/slashes", "key:with:colons", "key.with.dots", "key-with-dashes"];

    for key in keys {
        store.set(key, key.as_bytes()).unwrap();
    }

    for key in keys {
        let val = store.get(key).unwrap().unwrap();
        assert_eq!(val, key.as_bytes(), "failed for key: {key}");
    }
}
