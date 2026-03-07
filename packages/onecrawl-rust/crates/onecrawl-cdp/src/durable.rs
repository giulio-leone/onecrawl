//! Durable Sessions — crash-resilient browser sessions with auto-checkpoint and reconnect.

use chromiumoxide::Page;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{Duration, Instant, SystemTime};

use onecrawl_core::{Error, Result};

// ──────────────────────────── Configuration ────────────────────────────

/// Policy applied when a durable session crashes.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum CrashPolicy {
    Restart,
    Stop,
    Notify,
}

impl Default for CrashPolicy {
    fn default() -> Self {
        Self::Restart
    }
}

/// Configuration for a durable browser session.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DurableConfig {
    pub name: String,
    pub checkpoint_interval_secs: u64,
    pub state_path: PathBuf,
    pub auto_reconnect: bool,
    pub max_reconnect_attempts: u32,
    pub reconnect_delay_secs: u64,
    pub max_uptime_secs: Option<u64>,
    pub on_crash: CrashPolicy,
    pub persist_auth: bool,
    pub persist_scroll: bool,
    pub persist_url: bool,
}

impl Default for DurableConfig {
    fn default() -> Self {
        let state_path = dirs_fallback().join("states");
        Self {
            name: "default".to_string(),
            checkpoint_interval_secs: 30,
            state_path,
            auto_reconnect: true,
            max_reconnect_attempts: 10,
            reconnect_delay_secs: 2,
            max_uptime_secs: None,
            on_crash: CrashPolicy::Restart,
            persist_auth: true,
            persist_scroll: true,
            persist_url: true,
        }
    }
}

/// Fallback for `~/.onecrawl` when `dirs` crate is unavailable.
fn dirs_fallback() -> PathBuf {
    if let Some(home) = std::env::var_os("HOME") {
        PathBuf::from(home).join(".onecrawl")
    } else {
        PathBuf::from("/tmp/.onecrawl")
    }
}

// ──────────────────────────── State ────────────────────────────

/// Current status of a durable session.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum DurableStatus {
    Running,
    Paused,
    Crashed,
    Reconnecting,
    Stopped,
    Checkpointing,
}

/// Persisted state for a durable session.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DurableState {
    pub name: String,
    pub created_at: String,
    pub last_checkpoint: Option<String>,
    pub url: Option<String>,
    pub cookies: Vec<serde_json::Value>,
    pub local_storage: Vec<(String, String)>,
    pub session_storage: Vec<(String, String)>,
    pub scroll_position: Option<(f64, f64)>,
    pub viewport: Option<(u32, u32)>,
    pub reconnect_count: u32,
    pub total_uptime_secs: u64,
    pub cdp_url: Option<String>,
    pub status: DurableStatus,
}

impl DurableState {
    fn new(name: &str) -> Self {
        let now = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        Self {
            name: name.to_string(),
            created_at: now.to_string(),
            last_checkpoint: None,
            url: None,
            cookies: Vec::new(),
            local_storage: Vec::new(),
            session_storage: Vec::new(),
            scroll_position: None,
            viewport: None,
            reconnect_count: 0,
            total_uptime_secs: 0,
            cdp_url: None,
            status: DurableStatus::Stopped,
        }
    }
}

// ──────────────────────────── Session ────────────────────────────

/// A durable browser session with auto-checkpoint and crash recovery.
pub struct DurableSession {
    pub config: DurableConfig,
    pub state: DurableState,
    started_at: Option<Instant>,
    /// Uptime accumulated from previous runs (restored from disk).
    accumulated_uptime_secs: u64,
}

impl DurableSession {
    /// Create a new durable session with the given configuration.
    pub fn new(config: DurableConfig) -> Result<Self> {
        crate::util::validate_safe_name(&config.name)?;
        let state = DurableState::new(&config.name);
        Ok(Self {
            config,
            state,
            started_at: None,
            accumulated_uptime_secs: 0,
        })
    }

    /// Save current browser state to disk.
    pub async fn checkpoint(&mut self, page: &Page) -> Result<DurableState> {
        self.state.status = DurableStatus::Checkpointing;

        let now = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        self.state.last_checkpoint = Some(now.to_string());

        // Capture URL
        if self.config.persist_url {
            self.state.url = page.url().await.ok().flatten();
        }

        // Capture cookies, localStorage, sessionStorage, and scroll in one JS call
        let state_js = r#"
            (() => {
                const data = {
                    cookies: [],
                    localStorage: [],
                    sessionStorage: [],
                    scroll: [0, 0],
                    viewport: [window.innerWidth, window.innerHeight]
                };
                try {
                    data.cookies = document.cookie.split(';')
                        .map(c => c.trim())
                        .filter(c => c.length > 0)
                        .map(c => { const [k,...v] = c.split('='); return {name: k, value: v.join('=')}; });
                } catch(e) {}
                try {
                    for (let i = 0; i < localStorage.length; i++) {
                        const key = localStorage.key(i);
                        data.localStorage.push([key, localStorage.getItem(key)]);
                    }
                } catch(e) {}
                try {
                    for (let i = 0; i < sessionStorage.length; i++) {
                        const key = sessionStorage.key(i);
                        data.sessionStorage.push([key, sessionStorage.getItem(key)]);
                    }
                } catch(e) {}
                data.scroll = [window.scrollX, window.scrollY];
                return JSON.stringify(data);
            })()
        "#;

        let eval_result = page
            .evaluate(state_js.to_string())
            .await
            .map_err(|e| Error::Cdp(format!("durable checkpoint: {e}")))?;
        let raw: String = eval_result
            .into_value()
            .unwrap_or_else(|_| "{}".to_string());

        #[derive(Deserialize)]
        struct CapturedState {
            #[serde(default)]
            cookies: Vec<serde_json::Value>,
            #[serde(default)]
            local_storage: Vec<(String, String)>,
            #[serde(default)]
            session_storage: Vec<(String, String)>,
            #[serde(default)]
            scroll: (f64, f64),
            #[serde(default)]
            viewport: (u32, u32),
        }

        let captured: CapturedState =
            serde_json::from_str(&raw).unwrap_or_else(|_| CapturedState {
                cookies: Vec::new(),
                local_storage: Vec::new(),
                session_storage: Vec::new(),
                scroll: (0.0, 0.0),
                viewport: (0, 0),
            });

        if self.config.persist_auth {
            self.state.cookies = captured.cookies;
            self.state.local_storage = captured.local_storage;
            self.state.session_storage = captured.session_storage;
        }

        if self.config.persist_scroll {
            self.state.scroll_position = Some(captured.scroll);
        }

        self.state.viewport = Some(captured.viewport);

        // Update uptime (accumulate across restores)
        if let Some(started) = self.started_at {
            self.state.total_uptime_secs =
                self.accumulated_uptime_secs + started.elapsed().as_secs();
        }

        self.state.status = DurableStatus::Running;

        // Write state to disk
        self.save_state()?;

        Ok(self.state.clone())
    }

    /// Restore browser state from a saved checkpoint on disk.
    pub async fn restore(&mut self, page: &Page) -> Result<()> {
        self.load_state()?;

        // Navigate to saved URL
        if self.config.persist_url {
            if let Some(ref url) = self.state.url {
                let _ = page.goto(url).await;
                tokio::time::sleep(Duration::from_millis(500)).await;
            }
        }

        // Restore cookies via document.cookie
        if self.config.persist_auth {
            for cookie in &self.state.cookies {
                if let (Some(name), Some(value)) = (
                    cookie.get("name").and_then(|v| v.as_str()),
                    cookie.get("value").and_then(|v| v.as_str()),
                ) {
                    let name_json = serde_json::to_string(name).unwrap_or_default();
                    let value_json = serde_json::to_string(value).unwrap_or_default();
                    let js = format!("document.cookie = {} + '=' + {}", name_json, value_json);
                    let _ = page.evaluate(js).await;
                }
            }

            // Restore localStorage
            for (key, value) in &self.state.local_storage {
                let key_json = serde_json::to_string(key).unwrap_or_default();
                let val_json = serde_json::to_string(value).unwrap_or_default();
                let js = format!("localStorage.setItem({}, {})", key_json, val_json);
                let _ = page.evaluate(js).await;
            }

            // Restore sessionStorage
            for (key, value) in &self.state.session_storage {
                let key_json = serde_json::to_string(key).unwrap_or_default();
                let val_json = serde_json::to_string(value).unwrap_or_default();
                let js = format!("sessionStorage.setItem({}, {})", key_json, val_json);
                let _ = page.evaluate(js).await;
            }
        }

        // Restore scroll position
        if self.config.persist_scroll {
            if let Some((x, y)) = self.state.scroll_position {
                let js = format!("window.scrollTo({}, {})", x, y);
                let _ = page.evaluate(js).await;
            }
        }

        self.state.status = DurableStatus::Running;
        self.accumulated_uptime_secs = self.state.total_uptime_secs;
        self.started_at = Some(Instant::now());

        Ok(())
    }

    /// Start the durable session monitoring loop.
    ///
    /// Periodically checkpoints browser state and monitors health.
    /// Stops when max_uptime is reached, the page becomes unresponsive,
    /// or the session is externally stopped.
    pub async fn start_loop(&mut self, page: Arc<Page>) -> Result<()> {
        self.state.status = DurableStatus::Running;
        self.started_at = Some(Instant::now());

        let checkpoint_interval = Duration::from_secs(self.config.checkpoint_interval_secs);
        let mut last_checkpoint = Instant::now();
        let health_interval = Duration::from_secs(10);
        let mut last_health = Instant::now();
        let mut consecutive_failures: u32 = 0;

        loop {
            let now = Instant::now();

            // Check max uptime
            if let Some(max_secs) = self.config.max_uptime_secs {
                if let Some(started) = self.started_at {
                    if started.elapsed().as_secs() >= max_secs {
                        let _ = self.checkpoint(&page).await;
                        self.state.status = DurableStatus::Stopped;
                        self.save_state().ok();
                        return Ok(());
                    }
                }
            }

            // Health check
            if now.duration_since(last_health) >= health_interval {
                last_health = now;
                match crate::harness::health_check(&page).await {
                    Ok(val) => {
                        let healthy = val
                            .get("healthy")
                            .and_then(|v| v.as_bool())
                            .unwrap_or(false);
                        if healthy {
                            consecutive_failures = 0;
                        } else {
                            consecutive_failures += 1;
                        }
                    }
                    Err(_) => {
                        consecutive_failures += 1;
                    }
                }

                // Handle failures
                if consecutive_failures >= 3 {
                    match self.config.on_crash {
                        CrashPolicy::Restart => {
                            self.state.status = DurableStatus::Reconnecting;
                            self.save_state().ok();

                            if self.config.auto_reconnect {
                                let max_retries =
                                    self.config.max_reconnect_attempts as usize;
                                let result =
                                    crate::harness::reconnect_cdp(&page, max_retries).await;
                                match result {
                                    Ok(val) => {
                                        let status = val
                                            .get("status")
                                            .and_then(|v| v.as_str())
                                            .unwrap_or("failed");
                                        if status == "connected" {
                                            self.state.reconnect_count += 1;
                                            self.state.status = DurableStatus::Running;
                                            consecutive_failures = 0;
                                        } else {
                                            self.state.status = DurableStatus::Crashed;
                                            self.save_state().ok();
                                            return Err(Error::Cdp(
                                                "durable session: reconnect failed".to_string(),
                                            ));
                                        }
                                    }
                                    Err(_) => {
                                        self.state.status = DurableStatus::Crashed;
                                        self.save_state().ok();
                                        return Err(Error::Cdp(
                                            "durable session: reconnect error".to_string(),
                                        ));
                                    }
                                }
                            }
                        }
                        CrashPolicy::Stop => {
                            self.state.status = DurableStatus::Stopped;
                            self.save_state().ok();
                            return Ok(());
                        }
                        CrashPolicy::Notify => {
                            self.state.status = DurableStatus::Crashed;
                            self.save_state().ok();
                            return Err(Error::Cdp(
                                "durable session: browser crashed (notify policy)".to_string(),
                            ));
                        }
                    }
                }
            }

            // Periodic checkpoint
            if now.duration_since(last_checkpoint) >= checkpoint_interval {
                last_checkpoint = now;
                let _ = self.checkpoint(&page).await;
            }

            tokio::time::sleep(Duration::from_secs(1)).await;
        }
    }

    // ── Persistence helpers ─────────────────────────────────────────

    fn state_file_path(&self) -> Result<PathBuf> {
        crate::util::validate_safe_name(&self.config.name)?;
        Ok(self.config
            .state_path
            .join(format!("{}.state", self.config.name)))
    }

    fn save_state(&self) -> Result<()> {
        std::fs::create_dir_all(&self.config.state_path)
            .map_err(|e| Error::Cdp(format!("durable state dir: {e}")))?;
        let json = serde_json::to_string_pretty(&self.state)
            .map_err(|e| Error::Cdp(format!("durable serialize: {e}")))?;
        std::fs::write(self.state_file_path()?, json)
            .map_err(|e| Error::Cdp(format!("durable write: {e}")))?;
        Ok(())
    }

    fn load_state(&mut self) -> Result<()> {
        let path = self.state_file_path()?;
        let content = std::fs::read_to_string(&path)
            .map_err(|e| Error::Cdp(format!("durable read {}: {e}", path.display())))?;
        self.state = serde_json::from_str(&content)
            .map_err(|e| Error::Cdp(format!("durable parse: {e}")))?;
        Ok(())
    }

    /// Update the configuration of a running durable session.
    pub fn update_config(&mut self, config: DurableConfig) {
        self.config = config;
    }

    // ── Static helpers ─────────────────────────────────────────

    /// List all saved durable sessions from the state directory.
    pub fn list_sessions(state_dir: &Path) -> Result<Vec<DurableState>> {
        let mut sessions = Vec::new();
        if !state_dir.exists() {
            return Ok(sessions);
        }
        let entries = std::fs::read_dir(state_dir)
            .map_err(|e| Error::Cdp(format!("list sessions: {e}")))?;
        for entry in entries {
            let entry = match entry {
                Ok(e) => e,
                Err(_) => continue,
            };
            let path = entry.path();
            if path.extension().and_then(|e| e.to_str()) == Some("state") {
                if let Ok(content) = std::fs::read_to_string(&path) {
                    if let Ok(state) = serde_json::from_str::<DurableState>(&content) {
                        sessions.push(state);
                    }
                }
            }
        }
        Ok(sessions)
    }

    /// Delete a saved session state file.
    pub fn delete_session(state_dir: &Path, name: &str) -> Result<()> {
        crate::util::validate_safe_name(name)?;
        let path = state_dir.join(format!("{}.state", name));
        if path.exists() {
            std::fs::remove_file(&path)
                .map_err(|e| Error::Cdp(format!("delete session: {e}")))?;
        }
        Ok(())
    }

    /// Get the status of a named session from disk.
    pub fn get_status(state_dir: &Path, name: &str) -> Result<DurableState> {
        crate::util::validate_safe_name(name)?;
        let path = state_dir.join(format!("{}.state", name));
        let content = std::fs::read_to_string(&path)
            .map_err(|e| Error::Cdp(format!("get status {}: {e}", name)))?;
        let state: DurableState = serde_json::from_str(&content)
            .map_err(|e| Error::Cdp(format!("parse status: {e}")))?;
        Ok(state)
    }

    /// Get the default state directory path.
    pub fn default_state_dir() -> PathBuf {
        dirs_fallback().join("states")
    }
}

// ──────────────────────────── Duration parsing ────────────────────────────

/// Parse a human-readable duration string (e.g. "30s", "5m", "2h", "7d").
pub fn parse_duration(s: &str) -> std::result::Result<Duration, String> {
    let s = s.trim();
    if s.is_empty() {
        return Err("empty duration string".to_string());
    }

    let (num_str, suffix) = if s.ends_with('d') {
        (&s[..s.len() - 1], "d")
    } else if s.ends_with('h') {
        (&s[..s.len() - 1], "h")
    } else if s.ends_with('m') && !s.ends_with("ms") {
        (&s[..s.len() - 1], "m")
    } else if s.ends_with("ms") {
        (&s[..s.len() - 2], "ms")
    } else if s.ends_with('s') {
        (&s[..s.len() - 1], "s")
    } else {
        // Try as plain seconds
        (s, "s")
    };

    let num: u64 = num_str
        .parse()
        .map_err(|_| format!("invalid duration number: '{num_str}'"))?;

    let secs = match suffix {
        "ms" => return Ok(Duration::from_millis(num)),
        "s" => num,
        "m" => num * 60,
        "h" => num * 3600,
        "d" => num * 86400,
        _ => return Err(format!("unknown duration suffix: '{suffix}'")),
    };

    Ok(Duration::from_secs(secs))
}

// ──────────────────────────── Tests ────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_config_has_reasonable_values() {
        let cfg = DurableConfig::default();
        assert_eq!(cfg.name, "default");
        assert_eq!(cfg.checkpoint_interval_secs, 30);
        assert!(cfg.auto_reconnect);
        assert_eq!(cfg.max_reconnect_attempts, 10);
        assert_eq!(cfg.on_crash, CrashPolicy::Restart);
        assert!(cfg.persist_auth);
    }

    #[test]
    fn durable_state_new() {
        let state = DurableState::new("test-session");
        assert_eq!(state.name, "test-session");
        assert_eq!(state.status, DurableStatus::Stopped);
        assert_eq!(state.reconnect_count, 0);
        assert!(state.url.is_none());
    }

    #[test]
    fn parse_duration_variants() {
        assert_eq!(parse_duration("30s").unwrap(), Duration::from_secs(30));
        assert_eq!(parse_duration("5m").unwrap(), Duration::from_secs(300));
        assert_eq!(parse_duration("2h").unwrap(), Duration::from_secs(7200));
        assert_eq!(parse_duration("7d").unwrap(), Duration::from_secs(604800));
        assert_eq!(parse_duration("500ms").unwrap(), Duration::from_millis(500));
        assert_eq!(parse_duration("60").unwrap(), Duration::from_secs(60));
    }

    #[test]
    fn parse_duration_errors() {
        assert!(parse_duration("").is_err());
        assert!(parse_duration("abc").is_err());
    }

    #[test]
    fn crash_policy_serde_roundtrip() {
        let policy = CrashPolicy::Restart;
        let json = serde_json::to_string(&policy).unwrap();
        let back: CrashPolicy = serde_json::from_str(&json).unwrap();
        assert_eq!(back, CrashPolicy::Restart);
    }

    #[test]
    fn durable_state_serde_roundtrip() {
        let state = DurableState::new("test");
        let json = serde_json::to_string(&state).unwrap();
        let back: DurableState = serde_json::from_str(&json).unwrap();
        assert_eq!(back.name, "test");
        assert_eq!(back.status, DurableStatus::Stopped);
    }

    #[test]
    fn list_sessions_empty_dir() {
        let dir = std::env::temp_dir().join("onecrawl-test-empty-durable");
        let _ = std::fs::remove_dir_all(&dir);
        let sessions = DurableSession::list_sessions(&dir).unwrap();
        assert!(sessions.is_empty());
    }

    #[test]
    fn save_and_load_state() {
        let dir = std::env::temp_dir().join("onecrawl-test-durable-save");
        let _ = std::fs::remove_dir_all(&dir);

        let config = DurableConfig {
            name: "save-test".to_string(),
            state_path: dir.clone(),
            ..DurableConfig::default()
        };
        let mut session = DurableSession::new(config).unwrap();
        session.state.url = Some("https://example.com".to_string());
        session.save_state().unwrap();

        let loaded = DurableSession::get_status(&dir, "save-test").unwrap();
        assert_eq!(loaded.url, Some("https://example.com".to_string()));

        // Clean up
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn delete_session_works() {
        let dir = std::env::temp_dir().join("onecrawl-test-durable-delete");
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();

        let path = dir.join("del-test.state");
        std::fs::write(&path, "{}").unwrap();
        assert!(path.exists());

        DurableSession::delete_session(&dir, "del-test").unwrap();
        assert!(!path.exists());

        let _ = std::fs::remove_dir_all(&dir);
    }
}
