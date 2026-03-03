//! Health check utilities.

use serde::{Serialize, Deserialize};
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthStatus {
    pub status: String,
    pub version: String,
    pub uptime_seconds: u64,
    pub components: Vec<ComponentHealth>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComponentHealth {
    pub name: String,
    pub status: String,
    pub latency_ms: Option<u64>,
    pub details: Option<String>,
}

impl HealthStatus {
    pub fn new(start_time: u64) -> Self {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        Self {
            status: "healthy".into(),
            version: env!("CARGO_PKG_VERSION").into(),
            uptime_seconds: now.saturating_sub(start_time),
            components: vec![],
        }
    }

    pub fn add_component(&mut self, name: &str, status: &str, latency_ms: Option<u64>) {
        self.components.push(ComponentHealth {
            name: name.into(),
            status: status.into(),
            latency_ms,
            details: None,
        });
        if status != "healthy" {
            self.status = "degraded".into();
        }
    }

    pub fn is_healthy(&self) -> bool {
        self.status == "healthy"
    }

    pub fn to_json(&self) -> String {
        serde_json::to_string_pretty(self).unwrap_or_default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn health_status_new() {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();
        let h = HealthStatus::new(now);
        assert_eq!(h.status, "healthy");
        assert!(h.is_healthy());
        assert!(h.components.is_empty());
    }

    #[test]
    fn health_degraded_on_unhealthy_component() {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();
        let mut h = HealthStatus::new(now);
        h.add_component("storage", "unhealthy", None);
        assert_eq!(h.status, "degraded");
        assert!(!h.is_healthy());
    }

    #[test]
    fn health_to_json() {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();
        let h = HealthStatus::new(now);
        let json = h.to_json();
        assert!(json.contains("healthy"));
        assert!(json.contains("version"));
    }
}
