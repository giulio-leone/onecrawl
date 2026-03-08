//! Agent Memory — persistent cross-session memory for AI agents.
//!
//! Stores page visit history, learned element patterns, domain-specific
//! strategies, and auto-retry knowledge so agents get smarter over time.

use onecrawl_core::{Error, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

/// A single memory entry.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryEntry {
    pub key: String,
    pub value: serde_json::Value,
    pub category: MemoryCategory,
    pub domain: Option<String>,
    pub created_at: u64,
    pub accessed_at: u64,
    pub access_count: u64,
    pub ttl_seconds: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum MemoryCategory {
    PageVisit,
    ElementPattern,
    DomainStrategy,
    RetryKnowledge,
    UserPreference,
    SelectorMapping,
    ErrorPattern,
    Custom,
}

/// Domain-specific learned strategy.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DomainStrategy {
    pub domain: String,
    pub login_selectors: Option<LoginSelectors>,
    pub navigation_patterns: Vec<NavigationPattern>,
    pub known_popups: Vec<PopupPattern>,
    pub rate_limit_info: Option<RateLimitInfo>,
    pub anti_bot_level: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoginSelectors {
    pub username_selector: String,
    pub password_selector: String,
    pub submit_selector: String,
    pub success_indicator: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NavigationPattern {
    pub name: String,
    pub steps: Vec<String>,
    pub success_rate: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PopupPattern {
    pub trigger: String,
    pub dismiss_selector: String,
    pub frequency: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RateLimitInfo {
    pub max_requests_per_minute: u32,
    pub retry_after_seconds: u32,
    pub backoff_strategy: String,
}

/// Page visit record.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PageVisit {
    pub url: String,
    pub title: Option<String>,
    pub timestamp: u64,
    pub duration_ms: u64,
    pub actions_taken: Vec<String>,
    pub success: bool,
}

/// Element pattern learned from interactions.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ElementPattern {
    pub domain: String,
    pub description: String,
    pub primary_selector: String,
    pub fallback_selectors: Vec<String>,
    pub success_count: u64,
    pub failure_count: u64,
}

/// Agent memory store — JSON file-backed persistent memory.
pub struct AgentMemory {
    entries: HashMap<String, MemoryEntry>,
    path: PathBuf,
    max_entries: usize,
}

impl AgentMemory {
    pub fn new(path: impl AsRef<Path>) -> Self {
        Self {
            entries: HashMap::new(),
            path: path.as_ref().to_path_buf(),
            max_entries: 10000,
        }
    }

    pub fn load(path: impl AsRef<Path>) -> Result<Self> {
        let path = path.as_ref().to_path_buf();
        if path.exists() {
            let data = std::fs::read_to_string(&path)
                .map_err(|e| Error::Cdp(format!("failed to read memory: {e}")))?;
            let entries: HashMap<String, MemoryEntry> = serde_json::from_str(&data)
                .map_err(|e| Error::Cdp(format!("failed to parse memory: {e}")))?;
            Ok(Self { entries, path, max_entries: 10000 })
        } else {
            Ok(Self::new(path))
        }
    }

    pub fn save(&self) -> Result<()> {
        if let Some(parent) = self.path.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|e| Error::Cdp(format!("failed to create memory dir: {e}")))?;
        }
        let data = serde_json::to_string_pretty(&self.entries)
            .map_err(|e| Error::Cdp(format!("failed to serialize memory: {e}")))?;
        std::fs::write(&self.path, data)
            .map_err(|e| Error::Cdp(format!("failed to write memory: {e}")))?;
        Ok(())
    }

    pub fn store(&mut self, key: impl Into<String>, value: serde_json::Value, category: MemoryCategory, domain: Option<String>) -> Result<()> {
        let key = key.into();
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        if let Some(existing) = self.entries.get_mut(&key) {
            existing.value = value;
            existing.accessed_at = now;
            existing.access_count += 1;
        } else {
            if self.entries.len() >= self.max_entries {
                self.evict_lru();
            }
            self.entries.insert(key.clone(), MemoryEntry {
                key,
                value,
                category,
                domain,
                created_at: now,
                accessed_at: now,
                access_count: 1,
                ttl_seconds: None,
            });
        }
        self.save()
    }

    pub fn recall(&mut self, key: &str) -> Option<&MemoryEntry> {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        if let Some(entry) = self.entries.get_mut(key) {
            if let Some(ttl) = entry.ttl_seconds {
                if now - entry.created_at > ttl {
                    self.entries.remove(key);
                    return None;
                }
            }
            entry.accessed_at = now;
            entry.access_count += 1;
        }
        self.entries.get(key)
    }

    pub fn search(&self, query: &str, category: Option<MemoryCategory>, domain: Option<&str>) -> Vec<&MemoryEntry> {
        let query_lower = query.to_lowercase();
        self.entries.values()
            .filter(|e| {
                let key_match = e.key.to_lowercase().contains(&query_lower);
                let val_match = e.value.to_string().to_lowercase().contains(&query_lower);
                let cat_match = category.as_ref().map_or(true, |c| &e.category == c);
                let dom_match = domain.map_or(true, |d| e.domain.as_deref() == Some(d));
                (key_match || val_match) && cat_match && dom_match
            })
            .collect()
    }

    pub fn search_by_domain(&self, domain: &str) -> Vec<&MemoryEntry> {
        self.entries.values()
            .filter(|e| e.domain.as_deref() == Some(domain))
            .collect()
    }

    pub fn forget(&mut self, key: &str) -> bool {
        let removed = self.entries.remove(key).is_some();
        if removed { let _ = self.save(); }
        removed
    }

    pub fn clear_domain(&mut self, domain: &str) -> usize {
        let before = self.entries.len();
        self.entries.retain(|_, e| e.domain.as_deref() != Some(domain));
        let removed = before - self.entries.len();
        if removed > 0 { let _ = self.save(); }
        removed
    }

    pub fn clear_all(&mut self) -> usize {
        let count = self.entries.len();
        self.entries.clear();
        let _ = self.save();
        count
    }

    pub fn stats(&self) -> MemoryStats {
        let mut categories: HashMap<String, usize> = HashMap::new();
        let mut domains: HashMap<String, usize> = HashMap::new();
        for entry in self.entries.values() {
            *categories.entry(format!("{:?}", entry.category)).or_default() += 1;
            if let Some(ref d) = entry.domain {
                *domains.entry(d.clone()).or_default() += 1;
            }
        }
        MemoryStats {
            total_entries: self.entries.len(),
            categories,
            domains,
            max_entries: self.max_entries,
        }
    }

    pub fn store_domain_strategy(&mut self, strategy: DomainStrategy) -> Result<()> {
        let key = format!("domain_strategy:{}", strategy.domain);
        let domain = strategy.domain.clone();
        self.store(key, serde_json::to_value(&strategy)?, MemoryCategory::DomainStrategy, Some(domain))
    }

    pub fn recall_domain_strategy(&mut self, domain: &str) -> Option<DomainStrategy> {
        let key = format!("domain_strategy:{}", domain);
        self.recall(&key).and_then(|e| serde_json::from_value(e.value.clone()).ok())
    }

    pub fn record_page_visit(&mut self, visit: PageVisit) -> Result<()> {
        let key = format!("visit:{}:{}", visit.url, visit.timestamp);
        let domain = url_domain(&visit.url);
        self.store(key, serde_json::to_value(&visit)?, MemoryCategory::PageVisit, domain)
    }

    pub fn store_element_pattern(&mut self, pattern: ElementPattern) -> Result<()> {
        let key = format!("pattern:{}:{}", pattern.domain, pattern.description);
        let domain = Some(pattern.domain.clone());
        self.store(key, serde_json::to_value(&pattern)?, MemoryCategory::ElementPattern, domain)
    }

    pub fn recall_element_patterns(&self, domain: &str) -> Vec<ElementPattern> {
        self.entries.values()
            .filter(|e| e.category == MemoryCategory::ElementPattern && e.domain.as_deref() == Some(domain))
            .filter_map(|e| serde_json::from_value(e.value.clone()).ok())
            .collect()
    }

    fn evict_lru(&mut self) {
        if let Some(lru_key) = self.entries.values()
            .min_by_key(|e| (e.accessed_at, e.access_count))
            .map(|e| e.key.clone())
        {
            self.entries.remove(&lru_key);
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryStats {
    pub total_entries: usize,
    pub categories: HashMap<String, usize>,
    pub domains: HashMap<String, usize>,
    pub max_entries: usize,
}

fn url_domain(url: &str) -> Option<String> {
    url.split("://").nth(1)
        .and_then(|rest| rest.split('/').next())
        .map(|h| h.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_memory() -> AgentMemory {
        let dir = tempfile::tempdir().unwrap();
        AgentMemory::new(dir.path().join("memory.json"))
    }

    #[test]
    fn store_and_recall() {
        let mut mem = temp_memory();
        mem.store("key1", serde_json::json!("value1"), MemoryCategory::Custom, None).unwrap();
        let entry = mem.recall("key1").unwrap();
        assert_eq!(entry.value, serde_json::json!("value1"));
        assert_eq!(entry.access_count, 2);
    }

    #[test]
    fn search_by_query() {
        let mut mem = temp_memory();
        mem.store("login:google", serde_json::json!({"selector": "#login"}), MemoryCategory::SelectorMapping, Some("google.com".into())).unwrap();
        mem.store("login:github", serde_json::json!({"selector": ".login-btn"}), MemoryCategory::SelectorMapping, Some("github.com".into())).unwrap();
        let results = mem.search("login", None, None);
        assert_eq!(results.len(), 2);
    }

    #[test]
    fn search_by_domain() {
        let mut mem = temp_memory();
        mem.store("k1", serde_json::json!(1), MemoryCategory::PageVisit, Some("example.com".into())).unwrap();
        mem.store("k2", serde_json::json!(2), MemoryCategory::PageVisit, Some("other.com".into())).unwrap();
        let results = mem.search_by_domain("example.com");
        assert_eq!(results.len(), 1);
    }

    #[test]
    fn forget_entry() {
        let mut mem = temp_memory();
        mem.store("temp", serde_json::json!("data"), MemoryCategory::Custom, None).unwrap();
        assert!(mem.forget("temp"));
        assert!(mem.recall("temp").is_none());
    }

    #[test]
    fn clear_domain() {
        let mut mem = temp_memory();
        mem.store("a", serde_json::json!(1), MemoryCategory::Custom, Some("test.com".into())).unwrap();
        mem.store("b", serde_json::json!(2), MemoryCategory::Custom, Some("test.com".into())).unwrap();
        mem.store("c", serde_json::json!(3), MemoryCategory::Custom, Some("other.com".into())).unwrap();
        assert_eq!(mem.clear_domain("test.com"), 2);
        assert_eq!(mem.entries.len(), 1);
    }

    #[test]
    fn domain_strategy_roundtrip() {
        let mut mem = temp_memory();
        let strategy = DomainStrategy {
            domain: "example.com".into(),
            login_selectors: Some(LoginSelectors {
                username_selector: "#user".into(),
                password_selector: "#pass".into(),
                submit_selector: "#submit".into(),
                success_indicator: Some(".dashboard".into()),
            }),
            navigation_patterns: vec![],
            known_popups: vec![PopupPattern {
                trigger: "page_load".into(),
                dismiss_selector: ".cookie-accept".into(),
                frequency: "always".into(),
            }],
            rate_limit_info: None,
            anti_bot_level: Some("medium".into()),
        };
        mem.store_domain_strategy(strategy.clone()).unwrap();
        let recalled = mem.recall_domain_strategy("example.com").unwrap();
        assert_eq!(recalled.domain, "example.com");
        assert!(recalled.login_selectors.is_some());
    }

    #[test]
    fn element_pattern_store_and_recall() {
        let mut mem = temp_memory();
        let pattern = ElementPattern {
            domain: "shop.com".into(),
            description: "add to cart button".into(),
            primary_selector: ".add-to-cart".into(),
            fallback_selectors: vec!["[data-action='add']".into(), "button.cart".into()],
            success_count: 5,
            failure_count: 1,
        };
        mem.store_element_pattern(pattern).unwrap();
        let patterns = mem.recall_element_patterns("shop.com");
        assert_eq!(patterns.len(), 1);
        assert_eq!(patterns[0].primary_selector, ".add-to-cart");
    }

    #[test]
    fn stats() {
        let mut mem = temp_memory();
        mem.store("a", serde_json::json!(1), MemoryCategory::PageVisit, Some("x.com".into())).unwrap();
        mem.store("b", serde_json::json!(2), MemoryCategory::ElementPattern, Some("y.com".into())).unwrap();
        let stats = mem.stats();
        assert_eq!(stats.total_entries, 2);
        assert_eq!(stats.categories.len(), 2);
        assert_eq!(stats.domains.len(), 2);
    }

    #[test]
    fn eviction_at_max() {
        let dir = tempfile::tempdir().unwrap();
        let mut mem = AgentMemory::new(dir.path().join("mem.json"));
        mem.max_entries = 3;
        mem.store("a", serde_json::json!(1), MemoryCategory::Custom, None).unwrap();
        mem.store("b", serde_json::json!(2), MemoryCategory::Custom, None).unwrap();
        mem.store("c", serde_json::json!(3), MemoryCategory::Custom, None).unwrap();
        // Access "a" and "c" to make them recently used
        let _ = mem.recall("a");
        let _ = mem.recall("c");
        mem.store("d", serde_json::json!(4), MemoryCategory::Custom, None).unwrap();
        assert_eq!(mem.entries.len(), 3);
        // "b" should have been evicted (least recently accessed)
        assert!(mem.recall("b").is_none());
    }

    #[test]
    fn persistence_roundtrip() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("mem.json");
        {
            let mut mem = AgentMemory::new(&path);
            mem.store("persistent_key", serde_json::json!("persistent_value"), MemoryCategory::Custom, None).unwrap();
        }
        let mut mem2 = AgentMemory::load(&path).unwrap();
        let entry = mem2.recall("persistent_key").unwrap();
        assert_eq!(entry.value, serde_json::json!("persistent_value"));
    }

    #[test]
    fn url_domain_parsing() {
        assert_eq!(url_domain("https://example.com/path"), Some("example.com".into()));
        assert_eq!(url_domain("http://sub.domain.com:8080/x"), Some("sub.domain.com:8080".into()));
        assert_eq!(url_domain("invalid"), None);
    }
}
