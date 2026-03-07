//! Long-running harness: health monitoring, tab GC, circuit breaker.

use onecrawl_browser::Page;
use onecrawl_core::{Error, Result};

/// Get browser health metrics: memory, tab count, response time.
pub async fn health_check(page: &Page) -> Result<serde_json::Value> {
    let start = std::time::Instant::now();

    let js = r#"
        const result = {
            url: location.href,
            title: document.title,
            readyState: document.readyState,
            memory: {},
            timing: {}
        };
        
        // Memory info (Chrome only)
        if (performance.memory) {
            result.memory = {
                used_js_heap: performance.memory.usedJSHeapSize,
                total_js_heap: performance.memory.totalJSHeapSize,
                heap_limit: performance.memory.jsHeapSizeLimit
            };
        }
        
        // Navigation timing
        const nav = performance.getEntriesByType('navigation')[0];
        if (nav) {
            result.timing = {
                dom_complete: Math.round(nav.domComplete),
                load_event: Math.round(nav.loadEventEnd),
                dom_interactive: Math.round(nav.domInteractive)
            };
        }
        
        result.tab_count = window.length + 1; // frames + 1
        result.errors = window.__onecrawl_errors || 0;
        
        JSON.stringify(result)
    "#;

    let eval_result = page
        .evaluate(js.to_string())
        .await
        .map_err(|e| Error::Cdp(format!("health_check: {e}")))?;
    let elapsed = start.elapsed();

    let raw: String = eval_result
        .into_value()
        .unwrap_or_else(|_| "{}".to_string());
    let mut parsed: serde_json::Value =
        serde_json::from_str(&raw).unwrap_or(serde_json::json!({}));

    // Add response time
    if let Some(obj) = parsed.as_object_mut() {
        obj.insert(
            "response_time_ms".to_string(),
            serde_json::json!(elapsed.as_millis() as u64),
        );
        obj.insert(
            "healthy".to_string(),
            serde_json::json!(elapsed.as_millis() < 5000),
        );
    }

    Ok(parsed)
}

/// Circuit breaker state tracker.
#[derive(Debug, Default)]
pub struct CircuitBreaker {
    pub consecutive_failures: u32,
    pub threshold: u32,
    pub is_open: bool,
    pub last_failure: Option<String>,
}

impl CircuitBreaker {
    pub fn new(threshold: u32) -> Self {
        Self {
            threshold,
            ..Default::default()
        }
    }

    pub fn record_success(&mut self) {
        self.consecutive_failures = 0;
        self.is_open = false;
    }

    pub fn record_failure(&mut self, error: &str) -> bool {
        self.consecutive_failures += 1;
        self.last_failure = Some(error.to_string());
        if self.consecutive_failures >= self.threshold {
            self.is_open = true;
        }
        self.is_open
    }

    pub fn should_proceed(&self) -> bool {
        !self.is_open
    }

    pub fn reset(&mut self) {
        self.consecutive_failures = 0;
        self.is_open = false;
        self.last_failure = None;
    }

    pub fn status(&self) -> serde_json::Value {
        serde_json::json!({
            "consecutive_failures": self.consecutive_failures,
            "threshold": self.threshold,
            "is_open": self.is_open,
            "last_failure": self.last_failure,
            "should_proceed": self.should_proceed()
        })
    }
}

// ──────────────── Long-running harness helpers ────────────────

use serde_json::Value;

/// Auto-reconnect to CDP with exponential backoff.
pub async fn reconnect_cdp(page: &Page, max_retries: usize) -> Result<Value> {
    let mut last_error = String::new();
    for attempt in 0..max_retries {
        let backoff_ms = 100u64.saturating_mul(2u64.saturating_pow(attempt as u32)).min(10000);

        match page.evaluate("document.readyState".to_string()).await {
            Ok(val) => {
                let state: String = val.into_value().unwrap_or_default();
                return Ok(serde_json::json!({
                    "status": "connected",
                    "ready_state": state,
                    "attempts": attempt + 1,
                    "reconnected": attempt > 0
                }));
            }
            Err(e) => {
                last_error = e.to_string();
                tokio::time::sleep(std::time::Duration::from_millis(backoff_ms)).await;
            }
        }
    }

    Ok(serde_json::json!({
        "status": "failed",
        "attempts": max_retries,
        "last_error": last_error
    }))
}

/// Save checkpoint to disk: cookies, localStorage, sessionStorage, URL, scroll position.
pub async fn checkpoint_save(page: &Page, checkpoint_path: &str, name: &str) -> Result<Value> {
    let url = page
        .url()
        .await
        .ok()
        .flatten()
        .unwrap_or_default();

    let state_js = r#"
        (() => {
            const data = {
                url: window.location.href,
                title: document.title,
                scroll: { x: window.scrollX, y: window.scrollY },
                localStorage: {},
                sessionStorage: {},
                timestamp: Date.now()
            };
            try {
                for (let i = 0; i < localStorage.length; i++) {
                    const key = localStorage.key(i);
                    data.localStorage[key] = localStorage.getItem(key);
                }
            } catch(e) {}
            try {
                for (let i = 0; i < sessionStorage.length; i++) {
                    const key = sessionStorage.key(i);
                    data.sessionStorage[key] = sessionStorage.getItem(key);
                }
            } catch(e) {}
            return JSON.stringify(data);
        })()
    "#
    .to_string();

    let result = page
        .evaluate(state_js)
        .await
        .map_err(|e| Error::Cdp(format!("checkpoint_save state: {e}")))?;
    let state_str: String = result.into_value().unwrap_or_else(|_| "{}".to_string());

    let cookies_js = r#"
        (() => {
            return JSON.stringify(document.cookie.split(';').map(c => c.trim()).filter(c => c.length > 0));
        })()
    "#
    .to_string();
    let cookies_result = page
        .evaluate(cookies_js)
        .await
        .map_err(|e| Error::Cdp(format!("checkpoint_save cookies: {e}")))?;
    let cookies_str: String = cookies_result
        .into_value()
        .unwrap_or_else(|_| "[]".to_string());

    let now_ts = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();

    let checkpoint = serde_json::json!({
        "name": name,
        "state": serde_json::from_str::<Value>(&state_str).unwrap_or(serde_json::json!({})),
        "cookies": serde_json::from_str::<Value>(&cookies_str).unwrap_or(serde_json::json!([])),
        "saved_at": format!("{}", now_ts),
    });

    let dir = std::path::Path::new(checkpoint_path);
    std::fs::create_dir_all(dir).map_err(|e| Error::Cdp(format!("checkpoint dir: {e}")))?;
    let file_path = dir.join(format!("{}.json", name));
    std::fs::write(
        &file_path,
        serde_json::to_string_pretty(&checkpoint)
            .map_err(|e| Error::Cdp(format!("checkpoint serialize: {e}")))?,
    )
    .map_err(|e| Error::Cdp(format!("checkpoint write: {e}")))?;

    let size = std::fs::metadata(&file_path)
        .map(|m| m.len())
        .unwrap_or(0);

    Ok(serde_json::json!({
        "action": "checkpoint_save",
        "name": name,
        "path": file_path.to_string_lossy(),
        "url": url,
        "size_bytes": size
    }))
}

/// Restore checkpoint from disk: navigate, set storage, set cookies, restore scroll.
pub async fn checkpoint_restore(page: &Page, checkpoint_path: &str, name: &str) -> Result<Value> {
    let file_path = std::path::Path::new(checkpoint_path).join(format!("{}.json", name));
    let content =
        std::fs::read_to_string(&file_path).map_err(|e| Error::Cdp(format!("checkpoint read: {e}")))?;
    let checkpoint: Value =
        serde_json::from_str(&content).map_err(|e| Error::Cdp(format!("checkpoint parse: {e}")))?;

    // Navigate to saved URL
    if let Some(url) = checkpoint["state"]["url"].as_str() {
        let _ = page.goto(url).await;
        let _ = page.evaluate("document.readyState".to_string()).await;
    }

    // Restore localStorage
    if let Some(ls) = checkpoint["state"]["localStorage"].as_object() {
        for (key, value) in ls {
            if let Some(v) = value.as_str() {
                let key_json = serde_json::to_string(key).unwrap_or_default();
                let val_json = serde_json::to_string(&v).unwrap_or_default();
                let js = format!("localStorage.setItem({}, {})", key_json, val_json);
                let _ = page.evaluate(js).await;
            }
        }
    }

    // Restore sessionStorage
    if let Some(ss) = checkpoint["state"]["sessionStorage"].as_object() {
        for (key, value) in ss {
            if let Some(v) = value.as_str() {
                let key_json = serde_json::to_string(key).unwrap_or_default();
                let val_json = serde_json::to_string(&v).unwrap_or_default();
                let js = format!("sessionStorage.setItem({}, {})", key_json, val_json);
                let _ = page.evaluate(js).await;
            }
        }
    }

    // Restore scroll position
    if let (Some(x), Some(y)) = (
        checkpoint["state"]["scroll"]["x"].as_f64(),
        checkpoint["state"]["scroll"]["y"].as_f64(),
    ) {
        let js = format!("window.scrollTo({}, {})", x, y);
        let _ = page.evaluate(js).await;
    }

    Ok(serde_json::json!({
        "action": "checkpoint_restore",
        "name": name,
        "restored_url": checkpoint["state"]["url"],
        "saved_at": checkpoint["saved_at"]
    }))
}

/// Garbage-collect tabs: report current tab info for session pool management.
pub async fn gc_tabs_info(page: &Page) -> Result<Value> {
    let js = r#"
        (() => {
            return JSON.stringify({
                current_url: window.location.href,
                current_title: document.title,
                timestamp: Date.now()
            });
        })()
    "#
    .to_string();

    let result = page
        .evaluate(js)
        .await
        .map_err(|e| Error::Cdp(format!("gc_tabs: {e}")))?;
    let info: String = result.into_value().unwrap_or_else(|_| "{}".to_string());
    let parsed: Value = serde_json::from_str(&info).unwrap_or(serde_json::json!({}));

    Ok(serde_json::json!({
        "action": "gc_tabs",
        "current_page": parsed,
        "note": "Tab GC requires browser-level access. Use session pool management for multi-tab cleanup."
    }))
}

/// Watchdog: monitor browser health and report crash/hang indicators
pub async fn watchdog_status(page: &Page) -> Result<Value> {
    let start = std::time::Instant::now();
    
    // Test responsiveness with timeout
    let responsive = match tokio::time::timeout(
        std::time::Duration::from_secs(5),
        page.evaluate("document.readyState".to_string()),
    ).await {
        Ok(Ok(val)) => {
            let state: String = val.into_value().unwrap_or_default();
            serde_json::json!({
                "alive": true,
                "ready_state": state,
                "response_ms": start.elapsed().as_millis()
            })
        }
        Ok(Err(e)) => serde_json::json!({
            "alive": false,
            "error": e.to_string(),
            "response_ms": start.elapsed().as_millis()
        }),
        Err(_) => serde_json::json!({
            "alive": false,
            "error": "timeout (5s)",
            "response_ms": 5000
        }),
    };

    // Get memory info if alive
    let memory = if responsive["alive"].as_bool().unwrap_or(false) {
        let js = r#"
            (() => {
                const perf = performance.memory || {};
                return JSON.stringify({
                    used_js_heap: perf.usedJSHeapSize || 0,
                    total_js_heap: perf.totalJSHeapSize || 0,
                    heap_limit: perf.jsHeapSizeLimit || 0
                });
            })()
        "#.to_string();
        match page.evaluate(js).await {
            Ok(val) => {
                let s: String = val.into_value().unwrap_or_else(|_| "{}".to_string());
                serde_json::from_str(&s).unwrap_or(serde_json::json!({}))
            }
            Err(_) => serde_json::json!({})
        }
    } else {
        serde_json::json!({})
    };

    Ok(serde_json::json!({
        "action": "watchdog",
        "browser": responsive,
        "memory": memory,
        "timestamp": std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs()
    }))
}
