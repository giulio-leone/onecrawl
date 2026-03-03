//! OneCrawl Storage — encrypted key-value store backed by sled.

pub mod retry;

use onecrawl_core::{EncryptedPayload, Error, Result};
use sled::Db;

/// Encrypted key-value store using sled + AES-256-GCM.
pub struct EncryptedStore {
    db: Db,
    passphrase: String,
}

impl EncryptedStore {
    /// Open or create an encrypted store at the given path.
    pub fn open(path: &std::path::Path, passphrase: &str) -> Result<Self> {
        let db = sled::open(path).map_err(|e| Error::Storage(format!("sled open failed: {e}")))?;
        Ok(Self {
            db,
            passphrase: passphrase.to_string(),
        })
    }

    /// Open a temporary in-memory store (for testing).
    pub fn open_temp(passphrase: &str) -> Result<Self> {
        let config = sled::Config::new().temporary(true);
        let db = config
            .open()
            .map_err(|e| Error::Storage(format!("sled temp open failed: {e}")))?;
        Ok(Self {
            db,
            passphrase: passphrase.to_string(),
        })
    }

    /// Store a value, encrypting it with AES-256-GCM.
    pub fn set(&self, key: &str, value: &[u8]) -> Result<()> {
        let encrypted = onecrawl_crypto::encrypt(value, &self.passphrase)?;
        let payload = serde_json::to_vec(&encrypted)?;
        self.db
            .insert(key, payload)
            .map_err(|e| Error::Storage(format!("sled insert failed: {e}")))?;
        Ok(())
    }

    /// Retrieve and decrypt a value.
    pub fn get(&self, key: &str) -> Result<Option<Vec<u8>>> {
        let Some(raw) = self
            .db
            .get(key)
            .map_err(|e| Error::Storage(format!("sled get failed: {e}")))?
        else {
            return Ok(None);
        };

        let payload: EncryptedPayload = serde_json::from_slice(&raw)?;
        let decrypted = onecrawl_crypto::decrypt(&payload, &self.passphrase)?;
        Ok(Some(decrypted))
    }

    /// Delete a key.
    pub fn delete(&self, key: &str) -> Result<bool> {
        let existed = self
            .db
            .remove(key)
            .map_err(|e| Error::Storage(format!("sled remove failed: {e}")))?
            .is_some();
        Ok(existed)
    }

    /// List all keys with a given prefix.
    pub fn list(&self, prefix: &str) -> Result<Vec<String>> {
        let keys: Vec<String> = self
            .db
            .scan_prefix(prefix)
            .keys()
            .filter_map(|k| k.ok())
            .filter_map(|k| String::from_utf8(k.to_vec()).ok())
            .collect();
        Ok(keys)
    }

    /// Check if a key exists.
    pub fn contains(&self, key: &str) -> Result<bool> {
        self.db
            .contains_key(key)
            .map_err(|e| Error::Storage(format!("sled contains_key failed: {e}")))
    }

    /// Flush all pending writes to disk.
    pub fn flush(&self) -> Result<()> {
        self.db
            .flush()
            .map_err(|e| Error::Storage(format!("sled flush failed: {e}")))?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_store() -> EncryptedStore {
        EncryptedStore::open_temp("test-passphrase").unwrap()
    }

    #[test]
    fn set_get_roundtrip() {
        let store = temp_store();
        store.set("key1", b"hello world").unwrap();
        let value = store.get("key1").unwrap().unwrap();
        assert_eq!(value, b"hello world");
    }

    #[test]
    fn get_nonexistent_returns_none() {
        let store = temp_store();
        assert!(store.get("missing").unwrap().is_none());
    }

    #[test]
    fn delete_key() {
        let store = temp_store();
        store.set("key1", b"data").unwrap();
        assert!(store.delete("key1").unwrap());
        assert!(store.get("key1").unwrap().is_none());
    }

    #[test]
    fn delete_nonexistent_returns_false() {
        let store = temp_store();
        assert!(!store.delete("missing").unwrap());
    }

    #[test]
    fn list_keys_with_prefix() {
        let store = temp_store();
        store.set("oauth:token:1", b"t1").unwrap();
        store.set("oauth:token:2", b"t2").unwrap();
        store.set("cookie:session", b"s1").unwrap();

        let oauth_keys = store.list("oauth:").unwrap();
        assert_eq!(oauth_keys.len(), 2);

        let cookie_keys = store.list("cookie:").unwrap();
        assert_eq!(cookie_keys.len(), 1);
    }

    #[test]
    fn contains_key() {
        let store = temp_store();
        store.set("exists", b"yes").unwrap();
        assert!(store.contains("exists").unwrap());
        assert!(!store.contains("nope").unwrap());
    }

    #[test]
    fn overwrite_key() {
        let store = temp_store();
        store.set("key", b"v1").unwrap();
        store.set("key", b"v2").unwrap();
        let value = store.get("key").unwrap().unwrap();
        assert_eq!(value, b"v2");
    }

    #[test]
    fn large_value() {
        let store = temp_store();
        let data = vec![0xABu8; 100_000];
        store.set("big", &data).unwrap();
        let retrieved = store.get("big").unwrap().unwrap();
        assert_eq!(retrieved, data);
    }
}
