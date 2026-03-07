//! Long-running harness: health monitoring, tab GC, circuit breaker.

use chromiumoxide::Page;
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
