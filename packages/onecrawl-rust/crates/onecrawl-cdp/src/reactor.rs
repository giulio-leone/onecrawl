//! Event Reactor — persistent observer pattern for browser events.
//!
//! Matches real-time browser events (DOM mutations, network, console, navigation,
//! WebSocket, etc.) against configurable rules and dispatches handlers (log,
//! evaluate JS, webhook, screenshot, AI respond, chain, store).

use onecrawl_browser::Page;
use onecrawl_core::{Error, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

use crate::dom_observer::{drain_dom_mutations, start_dom_observer, stop_dom_observer};
use crate::events::{drain_console, drain_errors, observe_console, observe_errors, EventStream, EventType};
use crate::page_watcher::{drain_page_changes, start_page_watcher};
use crate::websocket::{drain_ws_frames, start_ws_recording, WsRecorder};

// ────────────────────────────────────────────────────────────────
//  Types
// ────────────────────────────────────────────────────────────────

/// Event types the reactor can listen for.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum ReactorEventType {
    DomMutation,
    NetworkRequest,
    NetworkResponse,
    Console,
    PageError,
    Navigation,
    Notification,
    WebSocket,
    Timer,
    Custom(String),
}

/// Filter to narrow which events trigger the handler.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventFilter {
    /// CSS selector for DOM mutations.
    pub selector: Option<String>,
    /// Glob pattern for network events.
    pub url_pattern: Option<String>,
    /// Substring match for console / notification content.
    pub message_pattern: Option<String>,
    /// e.g., "error" for console errors only.
    pub event_subtype: Option<String>,
}

/// What to do when an event matches.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "type")]
pub enum ReactorHandler {
    Log {
        format: Option<String>,
        output: Option<String>,
    },
    Evaluate {
        script: String,
    },
    Webhook {
        url: String,
        method: Option<String>,
        headers: Option<HashMap<String, String>>,
    },
    Command {
        cmd: String,
        args: Vec<String>,
    },
    Screenshot {
        path: Option<String>,
    },
    AiRespond {
        model: Option<String>,
        prompt: String,
        max_tokens: Option<u32>,
        actions: Option<Vec<String>>,
    },
    Chain {
        handlers: Vec<ReactorHandler>,
    },
    Store {
        path: String,
    },
}

/// A single event subscription with its handler.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReactorRule {
    pub id: String,
    pub event_type: ReactorEventType,
    pub filter: Option<EventFilter>,
    pub handler: ReactorHandler,
    pub enabled: bool,
    pub max_triggers: Option<u64>,
    pub cooldown_ms: Option<u64>,
    #[serde(default)]
    pub trigger_count: u64,
}

/// Configuration for the reactor.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReactorConfig {
    pub name: String,
    pub rules: Vec<ReactorRule>,
    pub max_events_per_minute: Option<u32>,
    pub buffer_size: Option<usize>,
    #[serde(default)]
    pub persist_events: bool,
    pub event_log_path: Option<String>,
}

/// Matched event ready for handler dispatch.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReactorEvent {
    pub event_type: ReactorEventType,
    pub timestamp: String,
    pub data: serde_json::Value,
    pub matched_rule: String,
}

/// Status of a single rule.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RuleStatus {
    pub id: String,
    pub event_type: String,
    pub enabled: bool,
    pub trigger_count: u64,
    pub last_triggered: Option<String>,
}

/// Status of the reactor.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReactorStatus {
    pub name: String,
    pub running: bool,
    pub rules_count: usize,
    pub total_events: u64,
    pub total_triggers: u64,
    pub uptime_secs: u64,
    pub events_per_minute: f64,
    pub rules: Vec<RuleStatus>,
}

// ────────────────────────────────────────────────────────────────
//  Internal stats
// ────────────────────────────────────────────────────────────────

struct ReactorStats {
    total_events: u64,
    total_triggers: u64,
    started_at: std::time::Instant,
    events_in_last_minute: Vec<std::time::Instant>,
}

impl Default for ReactorStats {
    fn default() -> Self {
        Self {
            total_events: 0,
            total_triggers: 0,
            started_at: std::time::Instant::now(),
            events_in_last_minute: Vec::new(),
        }
    }
}

// ────────────────────────────────────────────────────────────────
//  Reactor
// ────────────────────────────────────────────────────────────────

/// The core event reactor engine.
pub struct Reactor {
    config: ReactorConfig,
    rules: Arc<RwLock<Vec<ReactorRule>>>,
    running: Arc<RwLock<bool>>,
    stats: Arc<RwLock<ReactorStats>>,
    events: Arc<RwLock<Vec<ReactorEvent>>>,
    last_trigger_times: Arc<RwLock<HashMap<String, std::time::Instant>>>,
}

impl Reactor {
    pub fn new(config: ReactorConfig) -> Self {
        let rules = config.rules.clone();
        Self {
            config,
            rules: Arc::new(RwLock::new(rules)),
            running: Arc::new(RwLock::new(false)),
            stats: Arc::new(RwLock::new(ReactorStats::default())),
            events: Arc::new(RwLock::new(Vec::new())),
            last_trigger_times: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Returns a clone of the shared running flag so callers can signal shutdown.
    pub fn running_flag(&self) -> Arc<RwLock<bool>> {
        Arc::clone(&self.running)
    }

    /// Start the reactor loop — polls all event sources and dispatches handlers.
    pub async fn start(&self, page: &Page) -> Result<()> {
        {
            let mut running = self.running.write().await;
            if *running {
                return Err(Error::Cdp("reactor already running".into()));
            }
            *running = true;
        }

        {
            let mut stats = self.stats.write().await;
            *stats = ReactorStats::default();
        }

        let rules = self.rules.read().await;
        let (mut needs_dom, mut needs_console, mut needs_navigation, mut needs_ws) = (false, false, false, false);
        for r in rules.iter().filter(|r| r.enabled) {
            match r.event_type {
                ReactorEventType::DomMutation => needs_dom = true,
                ReactorEventType::Console | ReactorEventType::PageError => needs_console = true,
                ReactorEventType::Navigation => needs_navigation = true,
                ReactorEventType::WebSocket => needs_ws = true,
                _ => {}
            }
        }
        drop(rules);

        if needs_dom {
            start_dom_observer(page, None).await?;
        }

        let event_stream = EventStream::new(
            self.config.buffer_size.unwrap_or(1000).min(10_000),
        );

        if needs_console {
            let tx = event_stream.sender();
            observe_console(page, tx.clone()).await?;
            observe_errors(page, tx).await?;
        }

        if needs_navigation {
            start_page_watcher(page).await?;
        }

        let ws_recorder = WsRecorder::new();
        if needs_ws {
            start_ws_recording(page, &ws_recorder).await?;
        }

        let max_epm = self.config.max_events_per_minute.unwrap_or(60);
        let buffer_cap = self.config.buffer_size.unwrap_or(1000).min(10_000);

        while *self.running.read().await {
            let raw_events = Self::drain_all_events(
                page, needs_dom, needs_console, needs_navigation, needs_ws,
                &event_stream, &ws_recorder,
            ).await;

            // Rate limit check
            {
                let mut stats = self.stats.write().await;
                let now = std::time::Instant::now();
                stats.events_in_last_minute.retain(|t| now.duration_since(*t).as_secs() < 60);
                if stats.events_in_last_minute.len() as u32 >= max_epm {
                    tokio::time::sleep(std::time::Duration::from_millis(100)).await;
                    continue;
                }
            }

            for (event_type, data) in &raw_events {
                let matched = {
                    let rules = self.rules.read().await;
                    let mut result: Option<(String, ReactorHandler, Option<EventFilter>)> = None;
                    for rule in rules.iter() {
                        if !rule.enabled || &rule.event_type != event_type {
                            continue;
                        }
                        if let Some(max) = rule.max_triggers {
                            if rule.trigger_count >= max {
                                continue;
                            }
                        }
                        if let Some(cooldown) = rule.cooldown_ms {
                            let times = self.last_trigger_times.read().await;
                            if let Some(last) = times.get(&rule.id) {
                                if last.elapsed().as_millis() < cooldown as u128 {
                                    continue;
                                }
                            }
                        }
                        if !self.matches_filter(&rule.filter, event_type, data) {
                            continue;
                        }
                        result = Some((
                            rule.id.clone(),
                            rule.handler.clone(),
                            rule.filter.clone(),
                        ));
                        break;
                    }
                    result
                };

                if let Some((rule_id, handler, _filter)) = matched {
                    self.process_matched_event(
                        &rule_id, &handler, event_type, data, page, buffer_cap,
                    ).await;
                }
            }

            if !raw_events.is_empty() {
                let mut stats = self.stats.write().await;
                stats.total_events += raw_events.len() as u64;
            }

            tokio::time::sleep(std::time::Duration::from_millis(100)).await;
        }

        if needs_dom {
            let _ = stop_dom_observer(page).await;
        }

        Ok(())
    }

    /// Drain events from all active sources into a flat list.
    async fn drain_all_events(
        page: &Page,
        needs_dom: bool,
        needs_console: bool,
        needs_navigation: bool,
        needs_ws: bool,
        event_stream: &EventStream,
        ws_recorder: &WsRecorder,
    ) -> Vec<(ReactorEventType, serde_json::Value)> {
        let mut raw_events: Vec<(ReactorEventType, serde_json::Value)> = Vec::new();

        if needs_dom {
            if let Ok(mutations) = drain_dom_mutations(page).await {
                for m in mutations {
                    raw_events.push((
                        ReactorEventType::DomMutation,
                        serde_json::to_value(&m).unwrap_or_default(),
                    ));
                }
            }
        }

        if needs_console {
            let tx = event_stream.sender();
            let _ = drain_console(page, &tx).await;
            let _ = drain_errors(page, &tx).await;

            let mut rx = event_stream.subscribe();
            while let Ok(ev) = rx.try_recv() {
                let rtype = match ev.event_type {
                    EventType::ConsoleMessage => ReactorEventType::Console,
                    EventType::PageError => ReactorEventType::PageError,
                    EventType::NetworkRequest => ReactorEventType::NetworkRequest,
                    EventType::NetworkResponse => ReactorEventType::NetworkResponse,
                    EventType::FrameNavigated => ReactorEventType::Navigation,
                    _ => continue,
                };
                raw_events.push((rtype, ev.data));
            }
        }

        if needs_navigation {
            if let Ok(changes) = drain_page_changes(page).await {
                for c in changes {
                    raw_events.push((
                        ReactorEventType::Navigation,
                        serde_json::to_value(&c).unwrap_or_default(),
                    ));
                }
            }
        }

        if needs_ws {
            if let Ok(_count) = drain_ws_frames(page, ws_recorder).await {
                let frames = ws_recorder.frames().await;
                for f in &frames {
                    raw_events.push((
                        ReactorEventType::WebSocket,
                        serde_json::to_value(f).unwrap_or_default(),
                    ));
                }
                if !frames.is_empty() {
                    ws_recorder.clear().await;
                }
            }
        }

        raw_events
    }

    /// Dispatch handler and update stats/state for a matched event.
    async fn process_matched_event(
        &self,
        rule_id: &str,
        handler: &ReactorHandler,
        event_type: &ReactorEventType,
        data: &serde_json::Value,
        page: &Page,
        buffer_cap: usize,
    ) {
        let reactor_event = ReactorEvent {
            event_type: event_type.clone(),
            timestamp: chrono_now(),
            data: data.clone(),
            matched_rule: rule_id.to_string(),
        };

        let _ = dispatch_handler_boxed(self, handler, &reactor_event, page).await;

        {
            let mut stats = self.stats.write().await;
            stats.total_triggers += 1;
            stats.events_in_last_minute.push(std::time::Instant::now());
        }

        {
            let mut rules_w = self.rules.write().await;
            if let Some(r) = rules_w.iter_mut().find(|r| r.id == rule_id) {
                r.trigger_count += 1;
            }
        }

        {
            let mut times = self.last_trigger_times.write().await;
            times.insert(rule_id.to_string(), std::time::Instant::now());
        }

        {
            let mut events = self.events.write().await;
            events.push(reactor_event);
            let len = events.len();
            if len > buffer_cap {
                events.drain(0..len - buffer_cap);
            }
        }

        if self.config.persist_events {
            if let Some(path) = &self.config.event_log_path {
                let line = serde_json::to_string(data).unwrap_or_default();
                let _ = append_line(path, &line);
            }
        }
    }

    /// Stop the reactor.
    pub async fn stop(&self) -> Result<ReactorStatus> {
        {
            let mut running = self.running.write().await;
            *running = false;
        }
        Ok(self.status().await)
    }

    /// Add a rule at runtime.
    pub async fn add_rule(&self, rule: ReactorRule) -> Result<()> {
        let mut rules = self.rules.write().await;
        if rules.iter().any(|r| r.id == rule.id) {
            return Err(Error::Cdp(format!("rule '{}' already exists", rule.id)));
        }
        rules.push(rule);
        Ok(())
    }

    /// Remove a rule at runtime.
    pub async fn remove_rule(&self, rule_id: &str) -> Result<()> {
        let mut rules = self.rules.write().await;
        let before = rules.len();
        rules.retain(|r| r.id != rule_id);
        if rules.len() == before {
            return Err(Error::Cdp(format!("rule '{}' not found", rule_id)));
        }
        Ok(())
    }

    /// Enable/disable a rule.
    pub async fn toggle_rule(&self, rule_id: &str, enabled: bool) -> Result<()> {
        let mut rules = self.rules.write().await;
        if let Some(r) = rules.iter_mut().find(|r| r.id == rule_id) {
            r.enabled = enabled;
            Ok(())
        } else {
            Err(Error::Cdp(format!("rule '{}' not found", rule_id)))
        }
    }

    /// Get current status.
    pub async fn status(&self) -> ReactorStatus {
        let running = *self.running.read().await;
        let stats = self.stats.read().await;
        let rules = self.rules.read().await;
        let times = self.last_trigger_times.read().await;

        let uptime = stats.started_at.elapsed().as_secs();
        let epm = if uptime > 0 {
            (stats.total_triggers as f64 / uptime as f64) * 60.0
        } else {
            0.0
        };

        ReactorStatus {
            name: self.config.name.clone(),
            running,
            rules_count: rules.len(),
            total_events: stats.total_events,
            total_triggers: stats.total_triggers,
            uptime_secs: uptime,
            events_per_minute: epm,
            rules: rules
                .iter()
                .map(|r| {
                    let last = times.get(&r.id).map(|t| {
                        let secs_ago = t.elapsed().as_secs();
                        format!("{}s ago", secs_ago)
                    });
                    RuleStatus {
                        id: r.id.clone(),
                        event_type: format!("{:?}", r.event_type),
                        enabled: r.enabled,
                        trigger_count: r.trigger_count,
                        last_triggered: last,
                    }
                })
                .collect(),
        }
    }

    /// Get recent matched events.
    pub async fn recent_events(&self, limit: usize) -> Vec<ReactorEvent> {
        let events = self.events.read().await;
        let start = events.len().saturating_sub(limit);
        events[start..].to_vec()
    }

    /// Clear event history.
    pub async fn clear_events(&self) {
        let mut events = self.events.write().await;
        events.clear();
    }

    /// Check if an event matches a filter.
    fn matches_filter(
        &self,
        filter: &Option<EventFilter>,
        event_type: &ReactorEventType,
        data: &serde_json::Value,
    ) -> bool {
        let filter = match filter {
            Some(f) => f,
            None => return true,
        };

        // Selector filter for DOM mutations
        if let Some(sel) = &filter.selector {
            if *event_type == ReactorEventType::DomMutation {
                let target = data["target"].as_str().unwrap_or_default();
                if !target.contains(sel.as_str()) {
                    return false;
                }
            }
        }

        // URL pattern filter for network events
        if let Some(pat) = &filter.url_pattern {
            if matches!(
                event_type,
                ReactorEventType::NetworkRequest | ReactorEventType::NetworkResponse
            ) {
                let url = data["url"].as_str().unwrap_or_default();
                if !glob_match(pat, url) {
                    return false;
                }
            }
        }

        // Message pattern filter
        if let Some(pat) = &filter.message_pattern {
            let msg = data["message"].as_str().unwrap_or_default();
            if !msg.contains(pat.as_str()) {
                return false;
            }
        }

        // Event subtype filter (e.g., "error" for console)
        if let Some(sub) = &filter.event_subtype {
            let level = data["level"].as_str().unwrap_or_default();
            let change_type = data["change_type"].as_str().unwrap_or_default();
            if level != sub.as_str() && change_type != sub.as_str() {
                return false;
            }
        }

        true
    }

    /// Dispatch a handler for a matched event (non-recursive entry).
    #[allow(dead_code)]
    async fn dispatch_handler(
        &self,
        handler: &ReactorHandler,
        event: &ReactorEvent,
        page: &Page,
    ) -> Result<()> {
        dispatch_handler_boxed(self, handler, event, page).await
    }
}

/// Box-pinned handler dispatch to support recursive Chain handlers.
fn dispatch_handler_boxed<'a>(
    reactor: &'a Reactor,
    handler: &'a ReactorHandler,
    event: &'a ReactorEvent,
    page: &'a Page,
) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<()>> + Send + 'a>> {
    Box::pin(async move {
        match handler {
            ReactorHandler::Log { format, output } => {
                let fmt = format.as_deref().unwrap_or("{event}");
                let line = fmt.replace(
                    "{event}",
                    &serde_json::to_string(&event).unwrap_or_default(),
                );
                if let Some(path) = output {
                    append_line(path, &line)?;
                } else {
                    println!("[reactor] {}", line);
                }
            }

            ReactorHandler::Evaluate { script } => {
                // Safety: event data is injected via a JSON variable, never
                // via raw string interpolation.
                let event_json = serde_json::to_string(&event.data)
                    .map_err(|e| Error::Cdp(format!("json encode: {e}")))?;
                let safe_js = format!(
                    r#"(() => {{ const __reactor_event = {event_json}; {script} }})()"#,
                    event_json = event_json,
                    script = script,
                );
                page.evaluate(safe_js.as_str())
                    .await
                    .map_err(|e| Error::Cdp(format!("evaluate failed: {e}")))?;
            }

            ReactorHandler::Webhook {
                url,
                method,
                headers,
            } => {
                let client = reqwest::Client::new();
                let http_method = method.as_deref().unwrap_or("POST");
                let mut req = match http_method.to_uppercase().as_str() {
                    "GET" => client.get(url.as_str()),
                    "PUT" => client.put(url.as_str()),
                    "PATCH" => client.patch(url.as_str()),
                    "DELETE" => client.delete(url.as_str()),
                    _ => client.post(url.as_str()),
                };
                if let Some(hdrs) = headers {
                    for (k, v) in hdrs {
                        req = req.header(k.as_str(), v.as_str());
                    }
                }
                let body = serde_json::to_string(event).unwrap_or_default();
                let _ = req
                    .header("Content-Type", "application/json")
                    .body(body)
                    .send()
                    .await;
            }

            ReactorHandler::Screenshot { path } => {
                let out = path
                    .clone()
                    .unwrap_or_else(|| format!("reactor_screenshot_{}.png", now_epoch_ms()));
                let data = page
                    .screenshot(
                        onecrawl_browser::page::ScreenshotParams::builder()
                            .full_page(true)
                            .build(),
                    )
                    .await
                    .map_err(|e| Error::Cdp(format!("screenshot: {e}")))?;
                std::fs::write(&out, &data)
                    .map_err(|e| Error::Cdp(format!("write screenshot: {e}")))?;
            }

            ReactorHandler::AiRespond {
                prompt,
                model,
                max_tokens,
                actions,
            } => {
                // AI handler is a placeholder — store context for caller to process
                let ai_ctx = serde_json::json!({
                    "prompt": prompt,
                    "model": model,
                    "max_tokens": max_tokens,
                    "actions": actions,
                    "event": event,
                });
                let line = serde_json::to_string(&ai_ctx).unwrap_or_default();
                println!("[reactor:ai] {}", line);
            }

            ReactorHandler::Chain { handlers } => {
                for h in handlers {
                    dispatch_handler_boxed(reactor, h, event, page).await?;
                }
            }

            ReactorHandler::Store { path } => {
                let line = serde_json::to_string(event).unwrap_or_default();
                append_line(path, &line)?;
            }

            ReactorHandler::Command { .. } => {
                return Err(Error::Cdp(
                    "shell command execution disabled for safety".into(),
                ));
            }
        }
        Ok(())
    })
}

// ────────────────────────────────────────────────────────────────
//  Helpers
// ────────────────────────────────────────────────────────────────

fn append_line(path: &str, line: &str) -> Result<()> {
    use std::io::Write;
    let mut file = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)
        .map_err(|e| Error::Cdp(format!("open {path}: {e}")))?;
    writeln!(file, "{}", line).map_err(|e| Error::Cdp(format!("write {path}: {e}")))?;
    Ok(())
}

fn glob_match(pattern: &str, text: &str) -> bool {
    let regex_str = format!(
        "^{}$",
        pattern
            .replace('.', "\\.")
            .replace('*', ".*")
            .replace('?', ".")
    );
    // Simple fallback: substring match if regex fails
    regex_str
        .parse::<std::string::String>()
        .ok()
        .map(|_| {
            let parts: Vec<&str> = pattern.split('*').collect();
            if parts.len() == 1 {
                return text == pattern;
            }
            let mut pos = 0;
            for (i, part) in parts.iter().enumerate() {
                if part.is_empty() {
                    continue;
                }
                match text[pos..].find(part) {
                    Some(idx) => {
                        if i == 0 && idx != 0 {
                            return false;
                        }
                        pos += idx + part.len();
                    }
                    None => return false,
                }
            }
            if !pattern.ends_with('*') {
                return text.len() == pos;
            }
            true
        })
        .unwrap_or(false)
}

fn chrono_now() -> String {
    let d = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default();
    format!("{}.{:03}", d.as_secs(), d.subsec_millis())
}

fn now_epoch_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}

// ────────────────────────────────────────────────────────────────
//  Tests
// ────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn glob_match_basic() {
        assert!(glob_match("*api*", "https://example.com/api/v1/users"));
        assert!(glob_match("*.js", "app.js"));
        assert!(!glob_match("*.js", "app.css"));
        assert!(glob_match("*", "anything"));
        assert!(glob_match("exact", "exact"));
        assert!(!glob_match("exact", "not_exact"));
    }

    #[test]
    fn filter_matches_none() {
        let reactor = Reactor::new(ReactorConfig {
            name: "test".into(),
            rules: vec![],
            max_events_per_minute: None,
            buffer_size: None,
            persist_events: false,
            event_log_path: None,
        });
        assert!(reactor.matches_filter(
            &None,
            &ReactorEventType::Console,
            &serde_json::json!({}),
        ));
    }

    #[test]
    fn filter_message_pattern() {
        let reactor = Reactor::new(ReactorConfig {
            name: "test".into(),
            rules: vec![],
            max_events_per_minute: None,
            buffer_size: None,
            persist_events: false,
            event_log_path: None,
        });
        let filter = EventFilter {
            selector: None,
            url_pattern: None,
            message_pattern: Some("error".into()),
            event_subtype: None,
        };
        assert!(reactor.matches_filter(
            &Some(filter.clone()),
            &ReactorEventType::Console,
            &serde_json::json!({"message": "some error occurred"}),
        ));
        assert!(!reactor.matches_filter(
            &Some(filter),
            &ReactorEventType::Console,
            &serde_json::json!({"message": "all good"}),
        ));
    }

    #[test]
    fn reactor_status_empty() {
        let reactor = Reactor::new(ReactorConfig {
            name: "empty".into(),
            rules: vec![],
            max_events_per_minute: None,
            buffer_size: None,
            persist_events: false,
            event_log_path: None,
        });
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .expect("runtime");
        let status = rt.block_on(reactor.status());
        assert_eq!(status.name, "empty");
        assert!(!status.running);
        assert_eq!(status.rules_count, 0);
    }

    #[test]
    fn add_remove_rule() {
        let reactor = Reactor::new(ReactorConfig {
            name: "test".into(),
            rules: vec![],
            max_events_per_minute: None,
            buffer_size: None,
            persist_events: false,
            event_log_path: None,
        });
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .expect("runtime");
        rt.block_on(async {
            let rule = ReactorRule {
                id: "r1".into(),
                event_type: ReactorEventType::Console,
                filter: None,
                handler: ReactorHandler::Log {
                    format: None,
                    output: None,
                },
                enabled: true,
                max_triggers: None,
                cooldown_ms: None,
                trigger_count: 0,
            };
            reactor.add_rule(rule).await.expect("add rule");
            assert_eq!(reactor.status().await.rules_count, 1);

            // Duplicate should fail
            let dup = ReactorRule {
                id: "r1".into(),
                event_type: ReactorEventType::Console,
                filter: None,
                handler: ReactorHandler::Log {
                    format: None,
                    output: None,
                },
                enabled: true,
                max_triggers: None,
                cooldown_ms: None,
                trigger_count: 0,
            };
            assert!(reactor.add_rule(dup).await.is_err());

            reactor.remove_rule("r1").await.expect("remove rule");
            assert_eq!(reactor.status().await.rules_count, 0);

            // Remove nonexistent should fail
            assert!(reactor.remove_rule("nope").await.is_err());
        });
    }

    #[test]
    fn toggle_rule() {
        let reactor = Reactor::new(ReactorConfig {
            name: "test".into(),
            rules: vec![ReactorRule {
                id: "r1".into(),
                event_type: ReactorEventType::Console,
                filter: None,
                handler: ReactorHandler::Log {
                    format: None,
                    output: None,
                },
                enabled: true,
                max_triggers: None,
                cooldown_ms: None,
                trigger_count: 0,
            }],
            max_events_per_minute: None,
            buffer_size: None,
            persist_events: false,
            event_log_path: None,
        });
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .expect("runtime");
        rt.block_on(async {
            reactor.toggle_rule("r1", false).await.expect("toggle");
            let status = reactor.status().await;
            assert!(!status.rules[0].enabled);

            assert!(reactor.toggle_rule("nope", true).await.is_err());
        });
    }
}
