//! External CAPTCHA solver API integration (CapSolver, 2captcha, AntiCaptcha).
//!
//! Sends challenge metadata (sitekey, page URL) to a solver service and
//! returns a solution token that can be injected via `inject_solution`.

use onecrawl_core::{Error, Result};
use serde::{Deserialize, Serialize};

// ---------------------------------------------------------------------------
// Configuration
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SolverConfig {
    pub service: SolverService,
    pub api_key: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SolverService {
    #[serde(alias = "2captcha")]
    TwoCaptcha,
    CapSolver,
    AntiCaptcha,
}

impl std::fmt::Display for SolverService {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::TwoCaptcha => write!(f, "2captcha"),
            Self::CapSolver => write!(f, "capsolver"),
            Self::AntiCaptcha => write!(f, "anticaptcha"),
        }
    }
}

// ---------------------------------------------------------------------------
// Config file loader (~/.onecrawl/config.json)
// ---------------------------------------------------------------------------

/// Load solver config from `~/.onecrawl/config.json`.
///
/// Expected format:
/// ```json
/// { "capsolver_key": "CAP-xxx" }        // or
/// { "twocaptcha_key": "abc123" }         // or
/// { "anticaptcha_key": "xyz789" }
/// ```
pub fn load_solver_config() -> Option<SolverConfig> {
    let home = std::env::var("HOME").ok()?;
    let path = std::path::Path::new(&home).join(".onecrawl").join("config.json");
    let content = std::fs::read_to_string(&path).ok()?;
    let json: serde_json::Value = serde_json::from_str(&content).ok()?;

    if let Some(key) = json.get("capsolver_key").and_then(|v| v.as_str())
        && !key.is_empty() {
            return Some(SolverConfig { service: SolverService::CapSolver, api_key: key.to_string() });
        }
    if let Some(key) = json.get("twocaptcha_key").and_then(|v| v.as_str())
        && !key.is_empty() {
            return Some(SolverConfig { service: SolverService::TwoCaptcha, api_key: key.to_string() });
        }
    if let Some(key) = json.get("anticaptcha_key").and_then(|v| v.as_str())
        && !key.is_empty() {
            return Some(SolverConfig { service: SolverService::AntiCaptcha, api_key: key.to_string() });
        }
    None
}

// ---------------------------------------------------------------------------
// Solver API calls
// ---------------------------------------------------------------------------

/// Solve a CAPTCHA using an external API service.
///
/// - `captcha_type`: `"cloudflare_turnstile"`, `"recaptcha_v2"`, `"recaptcha_v3"`, `"hcaptcha"`
/// - `sitekey`: The site key from the CAPTCHA element
/// - `page_url`: The URL of the page containing the CAPTCHA
/// - `config`: API key and service selection
///
/// Returns the solution token string.
pub async fn solve_via_api(
    captcha_type: &str,
    sitekey: &str,
    page_url: &str,
    config: &SolverConfig,
) -> Result<String> {
    match config.service {
        SolverService::CapSolver => solve_capsolver(captcha_type, sitekey, page_url, &config.api_key).await,
        SolverService::TwoCaptcha => solve_twocaptcha(captcha_type, sitekey, page_url, &config.api_key).await,
        SolverService::AntiCaptcha => solve_anticaptcha(captcha_type, sitekey, page_url, &config.api_key).await,
    }
}

// ---------------------------------------------------------------------------
// Shared JSON-API solver (used by CapSolver and AntiCaptcha)
// ---------------------------------------------------------------------------

struct JsonApiConfig<'a> {
    service_name: &'a str,
    create_url: &'a str,
    poll_url: &'a str,
    api_key: &'a str,
    task: serde_json::Value,
    poll_interval_secs: u64,
    max_polls: usize,
}

/// Generic create-task + poll-result solver for JSON-based APIs.
async fn solve_json_api(cfg: JsonApiConfig<'_>) -> Result<String> {
    let client = reqwest::Client::new();
    let name = cfg.service_name;

    let create_body = serde_json::json!({
        "clientKey": cfg.api_key,
        "task": cfg.task,
    });

    let create_resp: serde_json::Value = client
        .post(cfg.create_url)
        .json(&create_body)
        .send()
        .await
        .map_err(|e| Error::Cdp(format!("{name} createTask: {e}")))?
        .json()
        .await
        .map_err(|e| Error::Cdp(format!("{name} createTask parse: {e}")))?;

    if create_resp.get("errorId").and_then(|v| v.as_i64()).unwrap_or(0) != 0 {
        let desc = create_resp.get("errorDescription").and_then(|v| v.as_str()).unwrap_or("unknown");
        return Err(Error::Cdp(format!("{name} error: {desc}")));
    }

    // Extract task ID (string for CapSolver, int for AntiCaptcha)
    let task_id = create_resp.get("taskId")
        .ok_or_else(|| Error::Cdp(format!("{name}: no taskId in response")))?
        .clone();

    let poll_body = serde_json::json!({
        "clientKey": cfg.api_key,
        "taskId": task_id,
    });

    for _ in 0..cfg.max_polls {
        tokio::time::sleep(std::time::Duration::from_secs(cfg.poll_interval_secs)).await;

        let result: serde_json::Value = client
            .post(cfg.poll_url)
            .json(&poll_body)
            .send()
            .await
            .map_err(|e| Error::Cdp(format!("{name} poll: {e}")))?
            .json()
            .await
            .map_err(|e| Error::Cdp(format!("{name} poll parse: {e}")))?;

        let status = result.get("status").and_then(|v| v.as_str()).unwrap_or("");

        if status == "ready" {
            return result
                .get("solution")
                .and_then(|s| s.get("token").or_else(|| s.get("gRecaptchaResponse")))
                .and_then(|v| v.as_str())
                .map(|s| s.to_string())
                .ok_or_else(|| Error::Cdp(format!("{name}: no token in solution")));
        }

        if status == "failed" || result.get("errorId").and_then(|v| v.as_i64()).unwrap_or(0) != 0 {
            let desc = result.get("errorDescription").and_then(|v| v.as_str()).unwrap_or("unknown");
            return Err(Error::Cdp(format!("{name} task failed: {desc}")));
        }
    }

    Err(Error::Cdp(format!("{name}: timeout waiting for solution")))
}

// ---------------------------------------------------------------------------
// CapSolver (https://docs.capsolver.com)
// ---------------------------------------------------------------------------

async fn solve_capsolver(
    captcha_type: &str,
    sitekey: &str,
    page_url: &str,
    api_key: &str,
) -> Result<String> {
    let task_type = match captcha_type {
        "cloudflare_turnstile" => "AntiTurnstileTaskProxyLess",
        "recaptcha_v2" => "ReCaptchaV2TaskProxyLess",
        "recaptcha_v3" => "ReCaptchaV3TaskProxyLess",
        "hcaptcha" => "HCaptchaTaskProxyLess",
        other => return Err(Error::Cdp(format!("CapSolver: unsupported type '{other}'"))),
    };

    let mut task = serde_json::json!({
        "type": task_type,
        "websiteURL": page_url,
        "websiteKey": sitekey,
    });
    if captcha_type == "cloudflare_turnstile" {
        task["metadata"] = serde_json::json!({"type": "turnstile"});
    }

    solve_json_api(JsonApiConfig {
        service_name: "CapSolver",
        create_url: "https://api.capsolver.com/createTask",
        poll_url: "https://api.capsolver.com/getTaskResult",
        api_key,
        task,
        poll_interval_secs: 2,
        max_polls: 60,
    }).await
}

// ---------------------------------------------------------------------------
// 2captcha (https://2captcha.com/api-docs)
// ---------------------------------------------------------------------------

async fn solve_twocaptcha(
    captcha_type: &str,
    sitekey: &str,
    page_url: &str,
    api_key: &str,
) -> Result<String> {
    let client = reqwest::Client::new();

    let method = match captcha_type {
        "cloudflare_turnstile" => "turnstile",
        "recaptcha_v2" | "recaptcha_v3" => "userrecaptcha",
        "hcaptcha" => "hcaptcha",
        other => return Err(Error::Cdp(format!("2captcha: unsupported type '{other}'"))),
    };

    let mut params = vec![
        ("key", api_key.to_string()),
        ("method", method.to_string()),
        ("sitekey", sitekey.to_string()),
        ("pageurl", page_url.to_string()),
        ("json", "1".to_string()),
    ];

    if captcha_type == "recaptcha_v3" {
        params.push(("version", "v3".to_string()));
        params.push(("action", "verify".to_string()));
        params.push(("min_score", "0.5".to_string()));
    }

    let submit: serde_json::Value = client
        .post("https://2captcha.com/in.php")
        .form(&params)
        .send()
        .await
        .map_err(|e| Error::Cdp(format!("2captcha submit: {e}")))?
        .json()
        .await
        .map_err(|e| Error::Cdp(format!("2captcha submit parse: {e}")))?;

    if submit.get("status").and_then(|v| v.as_i64()).unwrap_or(0) != 1 {
        let msg = submit.get("request").and_then(|v| v.as_str()).unwrap_or("unknown error");
        return Err(Error::Cdp(format!("2captcha error: {msg}")));
    }

    let request_id = submit
        .get("request")
        .and_then(|v| v.as_str())
        .ok_or_else(|| Error::Cdp("2captcha: no request ID".into()))?
        .to_string();

    tokio::time::sleep(std::time::Duration::from_secs(10)).await;

    for _ in 0..40 {
        let result: serde_json::Value = client
            .get("https://2captcha.com/res.php")
            .query(&[("key", api_key), ("action", "get"), ("id", &request_id), ("json", "1")])
            .send()
            .await
            .map_err(|e| Error::Cdp(format!("2captcha poll: {e}")))?
            .json()
            .await
            .map_err(|e| Error::Cdp(format!("2captcha poll parse: {e}")))?;

        if result.get("status").and_then(|v| v.as_i64()).unwrap_or(0) == 1 {
            return result
                .get("request")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string())
                .ok_or_else(|| Error::Cdp("2captcha: no token in response".into()));
        }

        let req = result.get("request").and_then(|v| v.as_str()).unwrap_or("");
        if req != "CAPCHA_NOT_READY" {
            return Err(Error::Cdp(format!("2captcha error: {req}")));
        }

        tokio::time::sleep(std::time::Duration::from_secs(3)).await;
    }

    Err(Error::Cdp("2captcha: timeout waiting for solution (130s)".into()))
}

// ---------------------------------------------------------------------------
// AntiCaptcha (https://anti-captcha.com/apidoc)
// ---------------------------------------------------------------------------

async fn solve_anticaptcha(
    captcha_type: &str,
    sitekey: &str,
    page_url: &str,
    api_key: &str,
) -> Result<String> {
    let task_type = match captcha_type {
        "cloudflare_turnstile" => "TurnstileTaskProxyless",
        "recaptcha_v2" => "RecaptchaV2TaskProxyless",
        "recaptcha_v3" => "RecaptchaV3TaskProxyless",
        "hcaptcha" => "HCaptchaTaskProxyless",
        other => return Err(Error::Cdp(format!("AntiCaptcha: unsupported type '{other}'"))),
    };

    let mut task = serde_json::json!({
        "type": task_type,
        "websiteURL": page_url,
        "websiteKey": sitekey,
    });
    if captcha_type == "recaptcha_v3" {
        task["minScore"] = serde_json::json!(0.5);
        task["pageAction"] = serde_json::json!("verify");
    }

    solve_json_api(JsonApiConfig {
        service_name: "AntiCaptcha",
        create_url: "https://api.anti-captcha.com/createTask",
        poll_url: "https://api.anti-captcha.com/getTaskResult",
        api_key,
        task,
        poll_interval_secs: 3,
        max_polls: 40,
    }).await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_solver_service_display() {
        assert_eq!(SolverService::CapSolver.to_string(), "capsolver");
        assert_eq!(SolverService::TwoCaptcha.to_string(), "2captcha");
        assert_eq!(SolverService::AntiCaptcha.to_string(), "anticaptcha");
    }

    #[test]
    fn test_solver_config_deserialize() {
        let json = r#"{"service":"capsolver","api_key":"test123"}"#;
        let config: SolverConfig = serde_json::from_str(json).unwrap();
        assert_eq!(config.service, SolverService::CapSolver);
        assert_eq!(config.api_key, "test123");
    }

    #[test]
    fn test_solver_config_twocaptcha_alias() {
        let json = r#"{"service":"2captcha","api_key":"test123"}"#;
        let config: SolverConfig = serde_json::from_str(json).unwrap();
        assert_eq!(config.service, SolverService::TwoCaptcha);
    }

    #[test]
    fn test_load_solver_config_missing_file() {
        // The function handles missing file gracefully
        // (returns None when ~/.onecrawl/config.json doesn't exist or has no keys)
        // This test just verifies it doesn't panic
        let _ = load_solver_config();
    }
}
