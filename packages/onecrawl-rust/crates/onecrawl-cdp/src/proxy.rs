//! Proxy configuration and pool management for browser sessions.

use onecrawl_core::Result;
use serde::{Deserialize, Serialize};

/// Single proxy server configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProxyConfig {
    /// Proxy server URL, e.g. "http://proxy:8080" or "socks5://proxy:1080"
    pub server: String,
    pub username: Option<String>,
    pub password: Option<String>,
    /// Bypass list, e.g. "localhost,127.0.0.1"
    pub bypass: Option<String>,
}

/// A pool of proxy configurations with rotation strategy.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProxyPool {
    pub proxies: Vec<ProxyConfig>,
    pub strategy: RotationStrategy,
    pub current_index: usize,
}

/// How to pick the next proxy from the pool.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RotationStrategy {
    RoundRobin,
    Random,
    Sticky,
}

impl ProxyPool {
    pub fn new(proxies: Vec<ProxyConfig>, strategy: RotationStrategy) -> Self {
        Self {
            proxies,
            strategy,
            current_index: 0,
        }
    }

    /// Get the next proxy according to the rotation strategy.
    pub fn next_proxy(&mut self) -> Option<&ProxyConfig> {
        if self.proxies.is_empty() {
            return None;
        }
        match self.strategy {
            RotationStrategy::RoundRobin => {
                let proxy = &self.proxies[self.current_index % self.proxies.len()];
                self.current_index += 1;
                Some(proxy)
            }
            RotationStrategy::Random => {
                let idx = (std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_nanos() as usize)
                    % self.proxies.len();
                Some(&self.proxies[idx])
            }
            RotationStrategy::Sticky => self.proxies.first(),
        }
    }

    /// Get Chrome launch args for the current proxy.
    pub fn chrome_args(&self) -> Vec<String> {
        if let Some(proxy) = self.proxies.first() {
            let mut args = vec![format!("--proxy-server={}", proxy.server)];
            if let Some(ref bypass) = proxy.bypass {
                args.push(format!("--proxy-bypass-list={}", bypass));
            }
            args
        } else {
            vec![]
        }
    }

    /// Serialize pool to JSON for persistence.
    pub fn to_json(&self) -> Result<String> {
        Ok(serde_json::to_string_pretty(self)?)
    }

    /// Deserialize pool from JSON.
    pub fn from_json(json: &str) -> Result<Self> {
        Ok(serde_json::from_str(json)?)
    }
}
