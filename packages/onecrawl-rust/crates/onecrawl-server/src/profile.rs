use serde::{Deserialize, Serialize};
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Profile {
    pub id: String,
    pub name: String,
    pub path: PathBuf,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateProfileRequest {
    pub name: String,
}

impl Profile {
    pub fn new(name: &str) -> Result<Self, String> {
        if name.is_empty() || name.contains('/') || name.contains('\\') || name.contains("..") {
            return Err("invalid profile name: must not be empty or contain path separators or '..'".into());
        }
        let id = format!("prof_{:08x}", hash_str(name));
        let path = home_dir()
            .join(".onecrawl")
            .join("profiles")
            .join(name);
        // Ensure the directory exists.
        let _ = std::fs::create_dir_all(&path);
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        Ok(Self {
            id,
            name: name.to_string(),
            path,
            created_at: format!("{now}"),
        })
    }

    pub fn user_data_dir(&self) -> PathBuf {
        self.path.clone()
    }
}

fn hash_str(s: &str) -> u64 {
    let mut hasher = DefaultHasher::new();
    s.hash(&mut hasher);
    hasher.finish()
}

fn home_dir() -> PathBuf {
    std::env::var("HOME")
        .or_else(|_| std::env::var("USERPROFILE"))
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from("."))
}
