//! Action safety policy layer for AI agent operations.
//!
//! Provides URL allowlists, rate limiting, destructive action protection,
//! and command-level access control.

use serde::{Deserialize, Serialize};
use std::time::Instant;

/// Operator-defined safety policy for agent actions.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SafetyPolicy {
    /// Allowed domains (if empty, all domains allowed).
    #[serde(default)]
    pub allowed_domains: Vec<String>,
    /// Blocked domains.
    #[serde(default)]
    pub blocked_domains: Vec<String>,
    /// Blocked URL patterns (glob-style with `*` wildcards).
    #[serde(default)]
    pub blocked_url_patterns: Vec<String>,
    /// Maximum actions per session (0 = unlimited).
    #[serde(default)]
    pub max_actions: usize,
    /// Require confirmation for form submissions.
    #[serde(default)]
    pub confirm_form_submit: bool,
    /// Require confirmation for file uploads.
    #[serde(default)]
    pub confirm_file_upload: bool,
    /// Blocked commands (e.g. `["evaluate", "pdf"]`).
    #[serde(default)]
    pub blocked_commands: Vec<String>,
    /// Allowed commands (if empty, all non-blocked commands allowed).
    #[serde(default)]
    pub allowed_commands: Vec<String>,
    /// Rate limit: max actions per minute (0 = unlimited).
    #[serde(default)]
    pub rate_limit_per_minute: usize,
}

/// Runtime safety state that tracks counters and enforces policy.
pub struct SafetyState {
    policy: SafetyPolicy,
    action_count: usize,
    actions_this_minute: usize,
    minute_start: Instant,
}

/// Result of a safety check.
#[derive(Debug, PartialEq, Eq)]
pub enum SafetyCheck {
    /// Action is allowed.
    Allowed,
    /// Action is denied with a reason.
    Denied(String),
    /// Action needs operator confirmation.
    RequiresConfirmation(String),
}

impl SafetyState {
    pub fn new(policy: SafetyPolicy) -> Self {
        Self {
            policy,
            action_count: 0,
            actions_this_minute: 0,
            minute_start: Instant::now(),
        }
    }

    /// Return a reference to the active policy.
    pub fn policy(&self) -> &SafetyPolicy {
        &self.policy
    }

    /// Replace the active policy (counters are preserved).
    pub fn set_policy(&mut self, policy: SafetyPolicy) {
        self.policy = policy;
    }

    /// Check if a URL is allowed by the policy.
    pub fn check_url(&self, url: &str) -> SafetyCheck {
        let domain = match extract_domain(url) {
            Some(d) => d,
            None => return SafetyCheck::Denied(format!("cannot parse domain from URL: {url}")),
        };

        // Blocked domains take precedence.
        for blocked in &self.policy.blocked_domains {
            if domain_matches(&domain, blocked) {
                return SafetyCheck::Denied(format!("domain '{domain}' is blocked by policy"));
            }
        }

        // Blocked URL patterns.
        for pattern in &self.policy.blocked_url_patterns {
            if glob_matches(pattern, url) {
                return SafetyCheck::Denied(format!(
                    "URL matches blocked pattern '{pattern}'"
                ));
            }
        }

        // Allowed domains (empty = allow all).
        if !self.policy.allowed_domains.is_empty() {
            let is_allowed = self
                .policy
                .allowed_domains
                .iter()
                .any(|allowed| domain_matches(&domain, allowed));
            if !is_allowed {
                return SafetyCheck::Denied(format!(
                    "domain '{domain}' is not in the allowed list"
                ));
            }
        }

        SafetyCheck::Allowed
    }

    /// Check if a command is allowed and handle confirmation flags.
    pub fn check_command(&mut self, command: &str) -> SafetyCheck {
        // Max actions check.
        if self.policy.max_actions > 0 && self.action_count >= self.policy.max_actions {
            return SafetyCheck::Denied(format!(
                "session action limit reached ({}/{})",
                self.action_count, self.policy.max_actions
            ));
        }

        // Blocked commands.
        if self.policy.blocked_commands.iter().any(|b| b == command) {
            return SafetyCheck::Denied(format!("command '{command}' is blocked by policy"));
        }

        // Allowed commands (empty = all non-blocked allowed).
        if !self.policy.allowed_commands.is_empty()
            && !self.policy.allowed_commands.iter().any(|a| a == command)
        {
            return SafetyCheck::Denied(format!(
                "command '{command}' is not in the allowed list"
            ));
        }

        // Confirmation for destructive actions.
        if self.policy.confirm_form_submit && command == "fill_form" {
            return SafetyCheck::RequiresConfirmation(
                "form submission requires operator confirmation".into(),
            );
        }
        if self.policy.confirm_file_upload && command == "upload_file" {
            return SafetyCheck::RequiresConfirmation(
                "file upload requires operator confirmation".into(),
            );
        }

        SafetyCheck::Allowed
    }

    /// Check rate limit. Resets the window if a minute has elapsed.
    pub fn check_rate_limit(&mut self) -> SafetyCheck {
        if self.policy.rate_limit_per_minute == 0 {
            return SafetyCheck::Allowed;
        }

        let elapsed = self.minute_start.elapsed();
        if elapsed.as_secs() >= 60 {
            self.actions_this_minute = 0;
            self.minute_start = Instant::now();
        }

        if self.actions_this_minute >= self.policy.rate_limit_per_minute {
            return SafetyCheck::Denied(format!(
                "rate limit exceeded ({}/{} per minute)",
                self.actions_this_minute, self.policy.rate_limit_per_minute
            ));
        }

        SafetyCheck::Allowed
    }

    /// Record that an action was performed (increments counters).
    pub fn record_action(&mut self) {
        self.action_count += 1;

        let elapsed = self.minute_start.elapsed();
        if elapsed.as_secs() >= 60 {
            self.actions_this_minute = 1;
            self.minute_start = Instant::now();
        } else {
            self.actions_this_minute += 1;
        }
    }

    /// Load a safety policy from a JSON file.
    pub fn load_from_file(path: &std::path::Path) -> Result<SafetyPolicy, String> {
        let data = std::fs::read_to_string(path)
            .map_err(|e| format!("failed to read policy file: {e}"))?;
        serde_json::from_str(&data).map_err(|e| format!("failed to parse policy JSON: {e}"))
    }

    /// Current stats as a JSON value.
    pub fn stats(&self) -> serde_json::Value {
        let elapsed_secs = self.minute_start.elapsed().as_secs();
        serde_json::json!({
            "action_count": self.action_count,
            "actions_this_minute": self.actions_this_minute,
            "minute_window_elapsed_secs": elapsed_secs,
            "max_actions": self.policy.max_actions,
            "rate_limit_per_minute": self.policy.rate_limit_per_minute,
            "allowed_domains": self.policy.allowed_domains,
            "blocked_domains": self.policy.blocked_domains,
            "blocked_url_patterns": self.policy.blocked_url_patterns,
            "blocked_commands": self.policy.blocked_commands,
            "allowed_commands": self.policy.allowed_commands,
            "confirm_form_submit": self.policy.confirm_form_submit,
            "confirm_file_upload": self.policy.confirm_file_upload,
        })
    }
}

// ─────────────── helpers ───────────────

/// Extract the domain (host) from a URL string.
fn extract_domain(url: &str) -> Option<String> {
    // Handle protocol-relative URLs.
    let after_scheme = if let Some(idx) = url.find("://") {
        &url[idx + 3..]
    } else {
        url
    };
    let host_port = after_scheme.split('/').next()?;
    let host = host_port.split(':').next()?;
    if host.is_empty() {
        return None;
    }
    Some(host.to_lowercase())
}

/// Check if `domain` matches `pattern` — exact or subdomain match.
/// e.g. `example.com` matches `sub.example.com` and `example.com`.
fn domain_matches(domain: &str, pattern: &str) -> bool {
    let domain = domain.to_lowercase();
    let pattern = pattern.to_lowercase();
    if domain == pattern {
        return true;
    }
    // Subdomain match: domain ends with `.pattern`
    domain.ends_with(&format!(".{pattern}"))
}

/// Simple glob matching with `*` wildcards.
fn glob_matches(pattern: &str, text: &str) -> bool {
    let parts: Vec<&str> = pattern.split('*').collect();
    if parts.len() == 1 {
        return pattern == text;
    }

    let mut pos = 0usize;
    for (i, part) in parts.iter().enumerate() {
        if part.is_empty() {
            continue;
        }
        match text[pos..].find(part) {
            Some(found) => {
                // First segment must match at the start if pattern doesn't start with *.
                if i == 0 && found != 0 {
                    return false;
                }
                pos += found + part.len();
            }
            None => return false,
        }
    }

    // If pattern doesn't end with *, remaining text must be consumed.
    if !pattern.ends_with('*') && pos != text.len() {
        return false;
    }

    true
}

// ─────────────── tests ───────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_allows_all() {
        let state = SafetyState::new(SafetyPolicy::default());
        assert_eq!(state.check_url("https://example.com/page"), SafetyCheck::Allowed);
    }

    #[test]
    fn test_domain_check() {
        let policy = SafetyPolicy {
            allowed_domains: vec!["example.com".into()],
            blocked_domains: vec!["evil.com".into()],
            ..Default::default()
        };
        let state = SafetyState::new(policy);

        // Allowed domain.
        assert_eq!(state.check_url("https://example.com/path"), SafetyCheck::Allowed);
        // Subdomain of allowed domain.
        assert_eq!(state.check_url("https://sub.example.com/path"), SafetyCheck::Allowed);
        // Not in allowed list.
        assert!(matches!(state.check_url("https://other.com"), SafetyCheck::Denied(_)));
        // Blocked domain (blocked takes precedence even if hypothetically allowed).
        assert!(matches!(state.check_url("https://evil.com/hack"), SafetyCheck::Denied(_)));
        // Subdomain of blocked domain.
        assert!(matches!(
            state.check_url("https://sub.evil.com"),
            SafetyCheck::Denied(_)
        ));
    }

    #[test]
    fn test_command_check() {
        let policy = SafetyPolicy {
            blocked_commands: vec!["evaluate".into()],
            allowed_commands: vec!["navigate".into(), "click".into()],
            ..Default::default()
        };
        let mut state = SafetyState::new(policy);

        assert_eq!(state.check_command("navigate"), SafetyCheck::Allowed);
        assert_eq!(state.check_command("click"), SafetyCheck::Allowed);
        assert!(matches!(state.check_command("evaluate"), SafetyCheck::Denied(_)));
        // Not in allowed list.
        assert!(matches!(state.check_command("screenshot"), SafetyCheck::Denied(_)));
    }

    #[test]
    fn test_rate_limit() {
        let policy = SafetyPolicy {
            rate_limit_per_minute: 3,
            ..Default::default()
        };
        let mut state = SafetyState::new(policy);

        for _ in 0..3 {
            assert_eq!(state.check_rate_limit(), SafetyCheck::Allowed);
            state.record_action();
        }
        assert!(matches!(state.check_rate_limit(), SafetyCheck::Denied(_)));
    }

    #[test]
    fn test_url_patterns() {
        let policy = SafetyPolicy {
            blocked_url_patterns: vec!["*admin*".into(), "https://bad.com/*".into()],
            ..Default::default()
        };
        let state = SafetyState::new(policy);

        assert!(matches!(
            state.check_url("https://example.com/admin/panel"),
            SafetyCheck::Denied(_)
        ));
        assert!(matches!(
            state.check_url("https://bad.com/anything"),
            SafetyCheck::Denied(_)
        ));
        assert_eq!(state.check_url("https://good.com/page"), SafetyCheck::Allowed);
    }

    #[test]
    fn test_max_actions() {
        let policy = SafetyPolicy {
            max_actions: 2,
            ..Default::default()
        };
        let mut state = SafetyState::new(policy);

        assert_eq!(state.check_command("navigate"), SafetyCheck::Allowed);
        state.record_action();
        assert_eq!(state.check_command("click"), SafetyCheck::Allowed);
        state.record_action();
        // Third action should be denied.
        assert!(matches!(state.check_command("type"), SafetyCheck::Denied(_)));
    }

    #[test]
    fn test_confirmation_flags() {
        let policy = SafetyPolicy {
            confirm_form_submit: true,
            confirm_file_upload: true,
            ..Default::default()
        };
        let mut state = SafetyState::new(policy);

        assert!(matches!(
            state.check_command("fill_form"),
            SafetyCheck::RequiresConfirmation(_)
        ));
        assert!(matches!(
            state.check_command("upload_file"),
            SafetyCheck::RequiresConfirmation(_)
        ));
        assert_eq!(state.check_command("navigate"), SafetyCheck::Allowed);
    }

    #[test]
    fn test_glob_matches() {
        assert!(glob_matches("*admin*", "https://example.com/admin/panel"));
        assert!(glob_matches("https://bad.com/*", "https://bad.com/anything"));
        assert!(!glob_matches("https://bad.com/*", "https://good.com/page"));
        assert!(glob_matches("*.pdf", "report.pdf"));
        assert!(!glob_matches("*.pdf", "report.txt"));
        assert!(glob_matches("exact", "exact"));
        assert!(!glob_matches("exact", "not-exact"));
    }

    #[test]
    fn test_extract_domain() {
        assert_eq!(extract_domain("https://example.com/path"), Some("example.com".into()));
        assert_eq!(extract_domain("http://sub.test.org:8080/x"), Some("sub.test.org".into()));
        assert_eq!(extract_domain("https://UPPER.COM"), Some("upper.com".into()));
    }

    #[test]
    fn test_load_from_file() {
        let dir = std::env::temp_dir().join("onecrawl_safety_test");
        let _ = std::fs::create_dir_all(&dir);
        let path = dir.join("policy.json");
        std::fs::write(
            &path,
            r#"{"allowed_domains":["example.com"],"max_actions":10}"#,
        )
        .unwrap();

        let policy = SafetyState::load_from_file(&path).unwrap();
        assert_eq!(policy.allowed_domains, vec!["example.com".to_string()]);
        assert_eq!(policy.max_actions, 10);

        let _ = std::fs::remove_dir_all(&dir);
    }
}
