//! Configurable rate limiter for browser automation operations.
//!
//! Sliding-window based rate limiting with per-second, per-minute, and per-hour caps.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::{SystemTime, UNIX_EPOCH};

/// Configuration for rate limiting thresholds.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RateLimitConfig {
    pub max_requests_per_second: f64,
    pub max_requests_per_minute: f64,
    pub max_requests_per_hour: f64,
    pub burst_size: usize,
    pub cooldown_ms: u64,
}

impl Default for RateLimitConfig {
    fn default() -> Self {
        Self {
            max_requests_per_second: 2.0,
            max_requests_per_minute: 60.0,
            max_requests_per_hour: 1000.0,
            burst_size: 5,
            cooldown_ms: 500,
        }
    }
}

/// Mutable state for a rate limiter instance.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RateLimitState {
    pub config: RateLimitConfig,
    /// Timestamps kept sorted for O(log n) window queries via binary search.
    pub timestamps: Vec<f64>,
    pub total_requests: usize,
    pub total_throttled: usize,
    pub status: &'static str,
}

/// Statistics snapshot of the rate limiter.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RateLimitStats {
    pub total_requests: usize,
    pub total_throttled: usize,
    pub current_rate_per_second: f64,
    pub current_rate_per_minute: f64,
    pub remaining_this_second: f64,
    pub remaining_this_minute: f64,
    pub remaining_this_hour: f64,
    pub status: &'static str,
    pub next_allowed_ms: f64,
}

fn now_ms() -> f64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs_f64()
        * 1000.0
}

fn prune(timestamps: &mut Vec<f64>, now: f64) {
    let cutoff = now - 3_600_000.0; // keep last hour
    // Binary search for the partition point — O(log n) instead of O(n) retain
    let pos = timestamps.partition_point(|&t| t <= cutoff);
    if pos > 0 {
        timestamps.drain(..pos);
    }
}

/// O(log n) count of timestamps within a time window using binary search.
fn count_in_window(timestamps: &[f64], now: f64, window_ms: f64) -> usize {
    let cutoff = now - window_ms;
    let start = timestamps.partition_point(|&t| t <= cutoff);
    timestamps.len() - start
}

/// O(log n) find oldest timestamp within a time window.
fn oldest_in_window(timestamps: &[f64], now: f64, window_ms: f64) -> Option<f64> {
    let cutoff = now - window_ms;
    let start = timestamps.partition_point(|&t| t <= cutoff);
    timestamps.get(start).copied()
}

impl RateLimitState {
    /// Create a new rate limiter with the given configuration.
    pub fn new(config: RateLimitConfig) -> Self {
        Self {
            config,
            timestamps: Vec::new(),
            total_requests: 0,
            total_throttled: 0,
            status: "active",
        }
    }
}

/// Check if a request is allowed right now without recording it.
pub fn can_proceed(state: &RateLimitState) -> bool {
    let now = now_ms();
    let in_second = count_in_window(&state.timestamps, now, 1000.0) as f64;
    let in_minute = count_in_window(&state.timestamps, now, 60_000.0) as f64;
    let in_hour = count_in_window(&state.timestamps, now, 3_600_000.0) as f64;

    in_second < state.config.max_requests_per_second
        && in_minute < state.config.max_requests_per_minute
        && in_hour < state.config.max_requests_per_hour
}

/// Record a request. Returns `true` if the request was allowed, `false` if throttled.
pub fn record_request(state: &mut RateLimitState) -> bool {
    let now = now_ms();
    prune(&mut state.timestamps, now);

    if can_proceed(state) {
        state.timestamps.push(now);
        state.total_requests += 1;
        state.status = "active";
        true
    } else {
        state.total_throttled += 1;
        state.status = "throttled";
        false
    }
}

/// Compute milliseconds to wait before the next request is allowed.
pub fn wait_duration(state: &RateLimitState) -> u64 {
    if can_proceed(state) {
        return 0;
    }

    let now = now_ms();
    let mut wait: f64 = 0.0;

    // Check per-second window
    let in_second = count_in_window(&state.timestamps, now, 1000.0) as f64;
    if in_second >= state.config.max_requests_per_second {
        if let Some(oldest) = oldest_in_window(&state.timestamps, now, 1000.0) {
            let needed = oldest + 1000.0 - now;
            if needed > wait {
                wait = needed;
            }
        }
    }

    // Check per-minute window
    let in_minute = count_in_window(&state.timestamps, now, 60_000.0) as f64;
    if in_minute >= state.config.max_requests_per_minute {
        if let Some(oldest) = oldest_in_window(&state.timestamps, now, 60_000.0) {
            let needed = oldest + 60_000.0 - now;
            if needed > wait {
                wait = needed;
            }
        }
    }

    // Check per-hour window
    let in_hour = count_in_window(&state.timestamps, now, 3_600_000.0) as f64;
    if in_hour >= state.config.max_requests_per_hour {
        if let Some(oldest) = oldest_in_window(&state.timestamps, now, 3_600_000.0) {
            let needed = oldest + 3_600_000.0 - now;
            if needed > wait {
                wait = needed;
            }
        }
    }

    // Apply minimum cooldown
    let cooldown = state.config.cooldown_ms as f64;
    if wait < cooldown && wait > 0.0 {
        wait = cooldown;
    }

    wait.ceil() as u64
}

/// Get current statistics for the rate limiter.
pub fn get_stats(state: &RateLimitState) -> RateLimitStats {
    let now = now_ms();
    let in_second = count_in_window(&state.timestamps, now, 1000.0) as f64;
    let in_minute = count_in_window(&state.timestamps, now, 60_000.0) as f64;
    let in_hour = count_in_window(&state.timestamps, now, 3_600_000.0) as f64;

    RateLimitStats {
        total_requests: state.total_requests,
        total_throttled: state.total_throttled,
        current_rate_per_second: in_second,
        current_rate_per_minute: in_minute,
        remaining_this_second: (state.config.max_requests_per_second - in_second).max(0.0),
        remaining_this_minute: (state.config.max_requests_per_minute - in_minute).max(0.0),
        remaining_this_hour: (state.config.max_requests_per_hour - in_hour).max(0.0),
        status: state.status,
        next_allowed_ms: wait_duration(state) as f64,
    }
}

/// Reset all counters and timestamps.
pub fn reset(state: &mut RateLimitState) {
    state.timestamps.clear();
    state.total_requests = 0;
    state.total_throttled = 0;
    state.status = "active";
}

/// Return a map of preset rate limit configurations.
pub fn presets() -> HashMap<&'static str, RateLimitConfig> {
    let mut map = HashMap::with_capacity(4);
    map.insert(
        "conservative",
        RateLimitConfig {
            max_requests_per_second: 0.5,
            max_requests_per_minute: 20.0,
            max_requests_per_hour: 500.0,
            burst_size: 2,
            cooldown_ms: 2000,
        },
    );
    map.insert(
        "moderate",
        RateLimitConfig {
            max_requests_per_second: 2.0,
            max_requests_per_minute: 60.0,
            max_requests_per_hour: 1000.0,
            burst_size: 5,
            cooldown_ms: 500,
        },
    );
    map.insert(
        "aggressive",
        RateLimitConfig {
            max_requests_per_second: 5.0,
            max_requests_per_minute: 200.0,
            max_requests_per_hour: 5000.0,
            burst_size: 10,
            cooldown_ms: 200,
        },
    );
    map.insert(
        "unlimited",
        RateLimitConfig {
            max_requests_per_second: 1000.0,
            max_requests_per_minute: 60000.0,
            max_requests_per_hour: 3600000.0,
            burst_size: 100,
            cooldown_ms: 0,
        },
    );
    map
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_state_is_active() {
        let state = RateLimitState::new(RateLimitConfig::default());
        assert_eq!(state.status, "active");
        assert!(state.timestamps.is_empty());
    }

    #[test]
    fn test_can_proceed_initially() {
        let state = RateLimitState::new(RateLimitConfig::default());
        assert!(can_proceed(&state));
    }

    #[test]
    fn test_record_request_succeeds() {
        let mut state = RateLimitState::new(RateLimitConfig::default());
        assert!(record_request(&mut state));
        assert_eq!(state.total_requests, 1);
        assert_eq!(state.total_throttled, 0);
    }

    #[test]
    fn test_throttle_after_burst() {
        let mut state = RateLimitState::new(RateLimitConfig {
            max_requests_per_second: 2.0,
            max_requests_per_minute: 100.0,
            max_requests_per_hour: 1000.0,
            burst_size: 2,
            cooldown_ms: 100,
        });
        assert!(record_request(&mut state));
        assert!(record_request(&mut state));
        // Third should be throttled (2/s limit)
        assert!(!record_request(&mut state));
        assert_eq!(state.total_throttled, 1);
        assert_eq!(state.status, "throttled");
    }

    #[test]
    fn test_wait_duration_zero_when_allowed() {
        let state = RateLimitState::new(RateLimitConfig::default());
        assert_eq!(wait_duration(&state), 0);
    }

    #[test]
    fn test_wait_duration_nonzero_when_throttled() {
        let mut state = RateLimitState::new(RateLimitConfig {
            max_requests_per_second: 1.0,
            max_requests_per_minute: 100.0,
            max_requests_per_hour: 1000.0,
            burst_size: 1,
            cooldown_ms: 100,
        });
        record_request(&mut state);
        let wait = wait_duration(&state);
        assert!(wait > 0);
    }

    #[test]
    fn test_get_stats() {
        let mut state = RateLimitState::new(RateLimitConfig::default());
        record_request(&mut state);
        let stats = get_stats(&state);
        assert_eq!(stats.total_requests, 1);
        assert_eq!(stats.total_throttled, 0);
        assert!(stats.current_rate_per_second >= 1.0);
    }

    #[test]
    fn test_reset() {
        let mut state = RateLimitState::new(RateLimitConfig::default());
        record_request(&mut state);
        record_request(&mut state);
        reset(&mut state);
        assert_eq!(state.total_requests, 0);
        assert_eq!(state.total_throttled, 0);
        assert!(state.timestamps.is_empty());
        assert_eq!(state.status, "active");
    }

    #[test]
    fn test_presets_contain_all_keys() {
        let p = presets();
        assert!(p.contains_key("conservative"));
        assert!(p.contains_key("moderate"));
        assert!(p.contains_key("aggressive"));
        assert!(p.contains_key("unlimited"));
        assert!(p["conservative"].max_requests_per_second < p["moderate"].max_requests_per_second);
        assert!(p["moderate"].max_requests_per_second < p["aggressive"].max_requests_per_second);
    }
}
