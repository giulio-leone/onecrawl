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
// CapSolver (https://docs.capsolver.com)
// ---------------------------------------------------------------------------

async fn solve_capsolver(
    captcha_type: &str,
    sitekey: &str,
    page_url: &str,
    api_key: &str,
) -> Result<String> {
    let client = reqwest::Client::new();

    let task_type = match captcha_type {
        "cloudflare_turnstile" => "AntiTurnstileTaskProxyLess",
        "recaptcha_v2" => "ReCaptchaV2TaskProxyLess",
        "recaptcha_v3" => "ReCaptchaV3TaskProxyLess",
        "hcaptcha" => "HCaptchaTaskProxyLess",
        other => return Err(Error::Cdp(format!("CapSolver: unsupported type '{other}'"))),
    };

    // Create task
    let mut task = serde_json::json!({
        "type": task_type,
        "websiteURL": page_url,
        "websiteKey": sitekey,
    });
    // Turnstile needs metadata
    if captcha_type == "cloudflare_turnstile" {
        task["metadata"] = serde_json::json!({"type": "turnstile"});
    }

    let create_body = serde_json::json!({
        "clientKey": api_key,
        "task": task,
    });

    let resp = client
        .post("https://api.capsolver.com/createTask")
        .json(&create_body)
        .send()
        .await
        .map_err(|e| Error::Cdp(format!("CapSolver createTask request: {e}")))?;

    let create_resp: serde_json::Value = resp
        .json()
        .await
        .map_err(|e| Error::Cdp(format!("CapSolver createTask parse: {e}")))?;

    if create_resp.get("errorId").and_then(|v| v.as_i64()).unwrap_or(0) != 0 {
        let desc = create_resp.get("errorDescription").and_then(|v| v.as_str()).unwrap_or("unknown");
        return Err(Error::Cdp(format!("CapSolver error: {desc}")));
    }

    let task_id = create_resp
        .get("taskId")
        .and_then(|v| v.as_str())
        .ok_or_else(|| Error::Cdp("CapSolver: no taskId in response".into()))?
        .to_string();

    // Poll for result (max 120s)
    let poll_body = serde_json::json!({
        "clientKey": api_key,
        "taskId": task_id,
    });

    for _ in 0..60 {
        tokio::time::sleep(std::time::Duration::from_secs(2)).await;

        let resp = client
            .post("https://api.capsolver.com/getTaskResult")
            .json(&poll_body)
            .send()
            .await
            .map_err(|e| Error::Cdp(format!("CapSolver poll: {e}")))?;

        let result: serde_json::Value = resp
            .json()
            .await
            .map_err(|e| Error::Cdp(format!("CapSolver poll parse: {e}")))?;

        let status = result.get("status").and_then(|v| v.as_str()).unwrap_or("");

        if status == "ready" {
            let solution = result
                .get("solution")
                .and_then(|s| s.get("token").or_else(|| s.get("gRecaptchaResponse")))
                .and_then(|v| v.as_str())
                .ok_or_else(|| Error::Cdp("CapSolver: no token in solution".into()))?;
            return Ok(solution.to_string());
        }

        if status == "failed" {
            let desc = result.get("errorDescription").and_then(|v| v.as_str()).unwrap_or("unknown");
            return Err(Error::Cdp(format!("CapSolver task failed: {desc}")));
        }
    }

    Err(Error::Cdp("CapSolver: timeout waiting for solution (120s)".into()))
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

    // Submit task
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

    let resp = client
        .post("https://2captcha.com/in.php")
        .form(&params)
        .send()
        .await
        .map_err(|e| Error::Cdp(format!("2captcha submit: {e}")))?;

    let submit: serde_json::Value = resp
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

    // Poll for result
    tokio::time::sleep(std::time::Duration::from_secs(10)).await;

    for _ in 0..40 {
        let resp = client
            .get("https://2captcha.com/res.php")
            .query(&[
                ("key", api_key),
                ("action", "get"),
                ("id", &request_id),
                ("json", "1"),
            ])
            .send()
            .await
            .map_err(|e| Error::Cdp(format!("2captcha poll: {e}")))?;

        let result: serde_json::Value = resp
            .json()
            .await
            .map_err(|e| Error::Cdp(format!("2captcha poll parse: {e}")))?;

        let status = result.get("status").and_then(|v| v.as_i64()).unwrap_or(0);

        if status == 1 {
            let token = result
                .get("request")
                .and_then(|v| v.as_str())
                .ok_or_else(|| Error::Cdp("2captcha: no token in response".into()))?;
            return Ok(token.to_string());
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
    let client = reqwest::Client::new();

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

    let create_body = serde_json::json!({
        "clientKey": api_key,
        "task": task,
    });

    let resp = client
        .post("https://api.anti-captcha.com/createTask")
        .json(&create_body)
        .send()
        .await
        .map_err(|e| Error::Cdp(format!("AntiCaptcha createTask: {e}")))?;

    let create_resp: serde_json::Value = resp
        .json()
        .await
        .map_err(|e| Error::Cdp(format!("AntiCaptcha createTask parse: {e}")))?;

    if create_resp.get("errorId").and_then(|v| v.as_i64()).unwrap_or(0) != 0 {
        let desc = create_resp.get("errorDescription").and_then(|v| v.as_str()).unwrap_or("unknown");
        return Err(Error::Cdp(format!("AntiCaptcha error: {desc}")));
    }

    let task_id = create_resp
        .get("taskId")
        .and_then(|v| v.as_i64())
        .ok_or_else(|| Error::Cdp("AntiCaptcha: no taskId".into()))?;

    // Poll
    let poll_body = serde_json::json!({
        "clientKey": api_key,
        "taskId": task_id,
    });

    for _ in 0..40 {
        tokio::time::sleep(std::time::Duration::from_secs(3)).await;

        let resp = client
            .post("https://api.anti-captcha.com/getTaskResult")
            .json(&poll_body)
            .send()
            .await
            .map_err(|e| Error::Cdp(format!("AntiCaptcha poll: {e}")))?;

        let result: serde_json::Value = resp
            .json()
            .await
            .map_err(|e| Error::Cdp(format!("AntiCaptcha poll parse: {e}")))?;

        let status = result.get("status").and_then(|v| v.as_str()).unwrap_or("");

        if status == "ready" {
            let solution = result
                .get("solution")
                .and_then(|s| s.get("token").or_else(|| s.get("gRecaptchaResponse")))
                .and_then(|v| v.as_str())
                .ok_or_else(|| Error::Cdp("AntiCaptcha: no token in solution".into()))?;
            return Ok(solution.to_string());
        }

        if result.get("errorId").and_then(|v| v.as_i64()).unwrap_or(0) != 0 {
            let desc = result.get("errorDescription").and_then(|v| v.as_str()).unwrap_or("unknown");
            return Err(Error::Cdp(format!("AntiCaptcha task failed: {desc}")));
        }
    }

    Err(Error::Cdp("AntiCaptcha: timeout waiting for solution (120s)".into()))
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
