//! Workflow DSL Engine — define and execute browser automation as YAML/JSON recipes.
//!
//! Supports sequential steps, conditionals, loops, error handlers,
//! variable interpolation, and composable sub-workflows.

use onecrawl_core::{Error, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::pin::Pin;

/// A complete workflow definition.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Workflow {
    pub name: String,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub version: String,
    #[serde(default)]
    pub variables: HashMap<String, serde_json::Value>,
    pub steps: Vec<Step>,
    #[serde(default)]
    pub on_error: ErrorHandler,
}

/// A single workflow step.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Step {
    #[serde(default)]
    pub id: String,
    #[serde(default)]
    pub name: String,
    pub action: Action,
    #[serde(default)]
    pub condition: Option<String>,
    #[serde(default)]
    pub retries: u32,
    #[serde(default)]
    pub retry_delay_ms: u64,
    #[serde(default)]
    pub timeout_ms: Option<u64>,
    #[serde(default)]
    pub on_error: Option<StepErrorAction>,
    #[serde(default)]
    pub save_as: Option<String>,
}

/// Actions that can be performed.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Action {
    Navigate { url: String },
    Click { selector: String },
    Type { selector: String, text: String },
    WaitForSelector { selector: String, #[serde(default = "default_timeout")] timeout_ms: u64 },
    Screenshot { path: Option<String>, full_page: Option<bool> },
    Evaluate { js: String },
    Extract { selector: String, attribute: Option<String> },
    SmartClick { query: String },
    SmartFill { query: String, value: String },
    Sleep { ms: u64 },
    SetVariable { name: String, value: serde_json::Value },
    Log { message: String, level: Option<String> },
    Assert { condition: String, message: Option<String> },
    Loop { items: LoopSource, variable: String, steps: Vec<Step> },
    Conditional { condition: String, then_steps: Vec<Step>, else_steps: Option<Vec<Step>> },
    SubWorkflow { path: String },
    HttpRequest { url: String, method: Option<String>, headers: Option<HashMap<String, String>>, body: Option<String> },
    Snapshot { #[serde(default)] compact: bool, #[serde(default)] interactive_only: bool },
    Agent { prompt: String, options: Option<Vec<String>> },
}

fn default_timeout() -> u64 { 30000 }

/// Loop iteration source.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum LoopSource {
    Array(Vec<serde_json::Value>),
    Variable(String),
    Range { start: i64, end: i64 },
}

/// Error handling behavior.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ErrorHandler {
    #[serde(default = "default_error_action")]
    pub action: StepErrorAction,
    #[serde(default)]
    pub screenshot: bool,
    #[serde(default)]
    pub log: bool,
}

fn default_error_action() -> StepErrorAction { StepErrorAction::Stop }

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum StepErrorAction {
    Continue,
    #[default]
    Stop,
    Retry,
    Skip,
}

/// Execution result for a step.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StepResult {
    pub step_id: String,
    pub step_name: String,
    pub status: StepStatus,
    pub output: Option<serde_json::Value>,
    pub error: Option<String>,
    pub duration_ms: u64,
    #[serde(default)]
    pub paused: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum StepStatus {
    Success,
    Failed,
    Skipped,
    Paused,
}

/// Complete workflow execution result.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowResult {
    pub name: String,
    pub status: StepStatus,
    pub steps: Vec<StepResult>,
    pub variables: HashMap<String, serde_json::Value>,
    pub total_duration_ms: u64,
    pub steps_succeeded: usize,
    pub steps_failed: usize,
    pub steps_skipped: usize,
    pub paused_at: Option<usize>,
    pub agent_context: Option<serde_json::Value>,
}

/// Parse a workflow from a JSON-compatible string.
///
/// Despite the former name (`parse_yaml`), this only parses JSON.
/// Use `parse_json` for new code; this wrapper exists for backward compatibility.
pub fn parse_json_compat(input: &str) -> Result<Workflow> {
    serde_json::from_str::<Workflow>(input)
        .or_else(|_| {
            Err(Error::Cdp("workflow parse failed: use JSON format or ensure valid JSON".into()))
        })
}

/// Parse a workflow from JSON string.
pub fn parse_json(json: &str) -> Result<Workflow> {
    serde_json::from_str(json)
        .map_err(|e| Error::Cdp(format!("workflow JSON parse failed: {e}")))
}

/// Load workflow from file (auto-detects JSON).
pub fn load_from_file(path: &str) -> Result<Workflow> {
    let content = std::fs::read_to_string(path)
        .map_err(|e| Error::Cdp(format!("failed to read workflow file: {e}")))?;
    parse_json(&content)
}

/// Context provided to an AI agent when a workflow pauses at an agent step.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentStepContext {
    pub step_index: usize,
    pub prompt: String,
    pub options: Vec<String>,
    pub url: String,
    pub variables: HashMap<String, serde_json::Value>,
}

/// Decision returned by an AI agent to resume a paused workflow.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentDecision {
    pub choice: String,
    pub reasoning: Option<String>,
    pub updates: Option<HashMap<String, serde_json::Value>>,
}

/// Interpolate variables in a string template.
/// Replaces `{{var_name}}` with the variable value.
pub fn interpolate(template: &str, variables: &HashMap<String, serde_json::Value>) -> String {
    let mut result = template.to_string();
    for (key, value) in variables {
        let placeholder = format!("{{{{{}}}}}", key);
        let replacement = match value {
            serde_json::Value::String(s) => s.clone(),
            other => other.to_string(),
        };
        result = result.replace(&placeholder, &replacement);
    }
    result
}

/// Evaluate a simple condition expression against variables.
/// Supports: `var == "value"`, `var != "value"`, `var` (truthy check),
/// `!var` (falsy check), `var > N`, `var < N`.
pub fn evaluate_condition(condition: &str, variables: &HashMap<String, serde_json::Value>) -> bool {
    let condition = condition.trim();

    // Negation
    if let Some(inner) = condition.strip_prefix('!') {
        return !evaluate_condition(inner.trim(), variables);
    }

    // Equality
    if let Some((left, right)) = condition.split_once("==") {
        let left_val = resolve_value(left.trim(), variables);
        let right_val = resolve_value(right.trim(), variables);
        return left_val == right_val;
    }

    // Inequality
    if let Some((left, right)) = condition.split_once("!=") {
        let left_val = resolve_value(left.trim(), variables);
        let right_val = resolve_value(right.trim(), variables);
        return left_val != right_val;
    }

    // Greater than
    if let Some((left, right)) = condition.split_once('>') {
        if let (Some(l), Some(r)) = (
            resolve_number(left.trim(), variables),
            resolve_number(right.trim(), variables),
        ) {
            return l > r;
        }
    }

    // Less than
    if let Some((left, right)) = condition.split_once('<') {
        if let (Some(l), Some(r)) = (
            resolve_number(left.trim(), variables),
            resolve_number(right.trim(), variables),
        ) {
            return l < r;
        }
    }

    // Truthy check
    match variables.get(condition) {
        Some(serde_json::Value::Bool(b)) => *b,
        Some(serde_json::Value::Null) => false,
        Some(serde_json::Value::String(s)) => !s.is_empty(),
        Some(serde_json::Value::Number(n)) => n.as_f64().map_or(false, |v| v != 0.0),
        Some(_) => true,
        None => false,
    }
}

fn resolve_value(token: &str, variables: &HashMap<String, serde_json::Value>) -> String {
    let token = token.trim_matches('"').trim_matches('\'');
    if let Some(val) = variables.get(token) {
        match val {
            serde_json::Value::String(s) => s.clone(),
            other => other.to_string(),
        }
    } else {
        token.to_string()
    }
}

fn resolve_number(token: &str, variables: &HashMap<String, serde_json::Value>) -> Option<f64> {
    let token = token.trim();
    if let Ok(n) = token.parse::<f64>() {
        return Some(n);
    }
    variables.get(token).and_then(|v| v.as_f64())
}

/// Validate a workflow definition before execution.
pub fn validate(workflow: &Workflow) -> Vec<String> {
    let mut errors = Vec::new();

    if workflow.name.is_empty() {
        errors.push("workflow name is required".into());
    }

    if workflow.steps.is_empty() {
        errors.push("workflow must have at least one step".into());
    }

    for (i, step) in workflow.steps.iter().enumerate() {
        validate_step(step, &format!("steps[{}]", i), &mut errors);
    }

    errors
}

fn validate_step(step: &Step, path: &str, errors: &mut Vec<String>) {
    match &step.action {
        Action::Navigate { url } if url.is_empty() => {
            errors.push(format!("{path}: navigate requires non-empty url"));
        }
        Action::Click { selector } | Action::WaitForSelector { selector, .. } if selector.is_empty() => {
            errors.push(format!("{path}: action requires non-empty selector"));
        }
        Action::Type { selector, .. } if selector.is_empty() => {
            errors.push(format!("{path}: type requires non-empty selector"));
        }
        Action::Loop { steps, .. } => {
            for (i, sub) in steps.iter().enumerate() {
                validate_step(sub, &format!("{path}.loop[{}]", i), errors);
            }
        }
        Action::Conditional { then_steps, else_steps, .. } => {
            for (i, sub) in then_steps.iter().enumerate() {
                validate_step(sub, &format!("{path}.then[{}]", i), errors);
            }
            if let Some(els) = else_steps {
                for (i, sub) in els.iter().enumerate() {
                    validate_step(sub, &format!("{path}.else[{}]", i), errors);
                }
            }
        }
        _ => {}
    }
}

// ═══════════════════════════════════════════════════════════════════════
//  Standalone Workflow Execution Engine
// ═══════════════════════════════════════════════════════════════════════

/// Execute a parsed workflow against a browser page.
pub async fn execute_workflow(
    page: &chromiumoxide::Page,
    workflow: &Workflow,
) -> Result<WorkflowResult> {
    let start = std::time::Instant::now();
    let mut variables = workflow.variables.clone();
    let mut results: Vec<StepResult> = Vec::new();
    let mut succeeded = 0usize;
    let mut failed = 0usize;
    let mut skipped = 0usize;
    let mut overall_status = StepStatus::Success;

    for (i, step) in workflow.steps.iter().enumerate() {
        let step_id = if step.id.is_empty() { format!("step_{i}") } else { step.id.clone() };
        let step_name = if step.name.is_empty() { format!("Step {i}") } else { step.name.clone() };

        // Evaluate condition — skip if false
        if let Some(ref cond) = step.condition {
            let interpolated = interpolate(cond, &variables);
            if !evaluate_condition(&interpolated, &variables) {
                results.push(StepResult {
                    step_id, step_name,
                    status: StepStatus::Skipped,
                    output: None, error: None, duration_ms: 0,
                    paused: false,
                });
                skipped += 1;
                continue;
            }
        }

        // Retry loop
        let max_attempts = 1 + step.retries;
        let mut last_err: Option<String>;
        let mut step_ok = false;

        for attempt in 0..max_attempts {
            if attempt > 0 && step.retry_delay_ms > 0 {
                tokio::time::sleep(tokio::time::Duration::from_millis(step.retry_delay_ms)).await;
            }

            let step_start = std::time::Instant::now();
            let result = execute_step(page, &step.action, &mut variables, i).await;
            let duration_ms = step_start.elapsed().as_millis() as u64;

            match result {
                Ok(output) => {
                    // Check if this is an agent step — pause workflow
                    if matches!(&step.action, Action::Agent { .. }) {
                        let agent_context = output.clone();
                        results.push(StepResult {
                            step_id, step_name,
                            status: StepStatus::Paused,
                            output, error: None, duration_ms,
                            paused: true,
                        });
                        let total_duration_ms = start.elapsed().as_millis() as u64;
                        return Ok(WorkflowResult {
                            name: workflow.name.clone(),
                            status: StepStatus::Paused,
                            steps: results,
                            variables,
                            total_duration_ms,
                            steps_succeeded: succeeded,
                            steps_failed: failed,
                            steps_skipped: skipped,
                            paused_at: Some(i),
                            agent_context,
                        });
                    }
                    if let Some(ref save_key) = step.save_as {
                        if let Some(ref out) = output {
                            variables.insert(save_key.clone(), out.clone());
                        }
                    }
                    results.push(StepResult {
                        step_id: step_id.clone(), step_name: step_name.clone(),
                        status: StepStatus::Success,
                        output, error: None, duration_ms,
                        paused: false,
                    });
                    succeeded += 1;
                    step_ok = true;
                    break;
                }
                Err(e) => {
                    last_err = Some(e.to_string());
                    // Only push result on final attempt
                    if attempt + 1 >= max_attempts {
                        results.push(StepResult {
                            step_id: step_id.clone(), step_name: step_name.clone(),
                            status: StepStatus::Failed,
                            output: None, error: last_err.clone(), duration_ms,
                            paused: false,
                        });
                        failed += 1;
                    }
                }
            }
        }

        if !step_ok {
            let error_action = step.on_error.as_ref()
                .unwrap_or(&workflow.on_error.action);
            match error_action {
                StepErrorAction::Stop => {
                    overall_status = StepStatus::Failed;
                    break;
                }
                StepErrorAction::Continue | StepErrorAction::Skip | StepErrorAction::Retry => {
                    continue;
                }
            }
        }
    }

    let total_duration_ms = start.elapsed().as_millis() as u64;
    Ok(WorkflowResult {
        name: workflow.name.clone(),
        status: overall_status,
        steps: results,
        variables,
        total_duration_ms,
        steps_succeeded: succeeded,
        steps_failed: failed,
        steps_skipped: skipped,
        paused_at: None,
        agent_context: None,
    })
}

/// Execute a single workflow action against a browser page.
fn execute_step<'a>(
    page: &'a chromiumoxide::Page,
    action: &'a Action,
    variables: &'a mut HashMap<String, serde_json::Value>,
    step_index: usize,
) -> Pin<Box<dyn std::future::Future<Output = Result<Option<serde_json::Value>>> + Send + 'a>> {
    Box::pin(async move {
        match action {
            Action::Navigate { url } => {
                let url = interpolate(url, variables);
                crate::navigation::goto(page, &url).await?;
                let title = crate::navigation::get_title(page).await.unwrap_or_default();
                Ok(Some(serde_json::json!({ "url": url, "title": title })))
            }
            Action::Click { selector } => {
                let sel = interpolate(selector, variables);
                let resolved = crate::accessibility::resolve_ref(&sel);
                crate::element::click(page, &resolved).await?;
                Ok(Some(serde_json::json!({ "clicked": sel })))
            }
            Action::Type { selector, text } => {
                let sel = interpolate(selector, variables);
                let txt = interpolate(text, variables);
                let resolved = crate::accessibility::resolve_ref(&sel);
                crate::element::type_text(page, &resolved, &txt).await?;
                Ok(Some(serde_json::json!({ "typed": txt.len() })))
            }
            Action::WaitForSelector { selector, timeout_ms } => {
                let sel = interpolate(selector, variables);
                let resolved = crate::accessibility::resolve_ref(&sel);
                crate::navigation::wait_for_selector(page, &resolved, *timeout_ms).await?;
                Ok(Some(serde_json::json!({ "found": sel })))
            }
            Action::Screenshot { path, full_page } => {
                let bytes = if full_page.unwrap_or(false) {
                    crate::screenshot::screenshot_full(page).await?
                } else {
                    crate::screenshot::screenshot_viewport(page).await?
                };
                if let Some(p) = path {
                    let p = interpolate(p, variables);
                    std::fs::write(&p, &bytes)
                        .map_err(|e| Error::Cdp(format!("failed to write screenshot: {e}")))?;
                    Ok(Some(serde_json::json!({ "saved": p, "bytes": bytes.len() })))
                } else {
                    Ok(Some(serde_json::json!({ "bytes": bytes.len() })))
                }
            }
            Action::Evaluate { js } => {
                let js = interpolate(js, variables);
                let result = page.evaluate(js)
                    .await
                    .map_err(|e| Error::Cdp(e.to_string()))?;
                let val = result.into_value::<serde_json::Value>().unwrap_or(serde_json::Value::Null);
                Ok(Some(val))
            }
            Action::Extract { selector, attribute } => {
                let sel = interpolate(selector, variables);
                let sel_json = serde_json::to_string(&sel).unwrap_or_default();
                let attr_js = if let Some(attr) = attribute {
                    let attr_json = serde_json::to_string(attr).unwrap_or_default();
                    format!(r#"Array.from(document.querySelectorAll({sel_json})).map(e => e.getAttribute({attr_json}))"#)
                } else {
                    format!(r#"Array.from(document.querySelectorAll({sel_json})).map(e => e.textContent.trim())"#)
                };
                let result = page.evaluate(attr_js)
                    .await
                    .map_err(|e| Error::Cdp(e.to_string()))?;
                let val = result.into_value::<serde_json::Value>().unwrap_or(serde_json::Value::Null);
                Ok(Some(val))
            }
            Action::SmartClick { query } => {
                let q = interpolate(query, variables);
                let matched = crate::smart_actions::smart_click(page, &q).await?;
                Ok(Some(serde_json::json!({ "clicked": matched.selector, "confidence": matched.confidence })))
            }
            Action::SmartFill { query, value } => {
                let q = interpolate(query, variables);
                let v = interpolate(value, variables);
                let matched = crate::smart_actions::smart_fill(page, &q, &v).await?;
                Ok(Some(serde_json::json!({ "filled": matched.selector, "confidence": matched.confidence })))
            }
            Action::Sleep { ms } => {
                tokio::time::sleep(tokio::time::Duration::from_millis(*ms)).await;
                Ok(Some(serde_json::json!({ "slept_ms": ms })))
            }
            Action::SetVariable { name, value } => {
                let interpolated = interpolate(&value.to_string(), variables);
                let parsed = serde_json::from_str::<serde_json::Value>(&interpolated)
                    .unwrap_or(serde_json::Value::String(interpolated));
                variables.insert(name.clone(), parsed.clone());
                Ok(Some(serde_json::json!({ "set": name, "value": parsed })))
            }
            Action::Log { message, level } => {
                let msg = interpolate(message, variables);
                let lvl = level.as_deref().unwrap_or("info");
                match lvl {
                    "error" => tracing::error!("[workflow] {}", msg),
                    "warn" => tracing::warn!("[workflow] {}", msg),
                    "debug" => tracing::debug!("[workflow] {}", msg),
                    _ => tracing::info!("[workflow] {}", msg),
                }
                Ok(Some(serde_json::json!({ "logged": msg, "level": lvl })))
            }
            Action::Assert { condition, message } => {
                let cond = interpolate(condition, variables);
                if evaluate_condition(&cond, variables) {
                    Ok(Some(serde_json::json!({ "assert": "passed" })))
                } else {
                    Err(Error::Cdp(format!(
                        "assertion failed: {}",
                        message.as_deref().unwrap_or(&cond)
                    )))
                }
            }
            Action::Loop { items, variable, steps } => {
                let values: Vec<serde_json::Value> = match items {
                    LoopSource::Array(arr) => arr.clone(),
                    LoopSource::Variable(var_name) => {
                        let interpolated = interpolate(var_name, variables);
                        match variables.get(&interpolated) {
                            Some(serde_json::Value::Array(arr)) => arr.clone(),
                            _ => vec![],
                        }
                    }
                    LoopSource::Range { start, end } => {
                        let count = (*end).saturating_sub(*start).max(0);
                        if count > 100_000 {
                            return Err(Error::Cdp(format!(
                                "loop range too large: {count} items (max 100000)"
                            )));
                        }
                        (*start..*end).map(|i| serde_json::json!(i)).collect()
                    }
                };
                let mut last_output = None;
                for item in &values {
                    variables.insert(variable.clone(), item.clone());
                    for step in steps {
                        if let Some(ref cond) = step.condition {
                            let interpolated = interpolate(cond, variables);
                            if !evaluate_condition(&interpolated, variables) {
                                continue;
                            }
                        }
                        last_output = execute_step(page, &step.action, variables, 0).await?;
                        if let Some(ref save_key) = step.save_as {
                            if let Some(ref out) = last_output {
                                variables.insert(save_key.clone(), out.clone());
                            }
                        }
                    }
                }
                Ok(last_output)
            }
            Action::Conditional { condition, then_steps, else_steps } => {
                let cond = interpolate(condition, variables);
                let empty = vec![];
                let branch = if evaluate_condition(&cond, variables) {
                    then_steps
                } else {
                    else_steps.as_ref().unwrap_or(&empty)
                };
                let mut last_output = None;
                for step in branch {
                    if let Some(ref cond) = step.condition {
                        let interpolated = interpolate(cond, variables);
                        if !evaluate_condition(&interpolated, variables) {
                            continue;
                        }
                    }
                    last_output = execute_step(page, &step.action, variables, 0).await?;
                    if let Some(ref save_key) = step.save_as {
                        if let Some(ref out) = last_output {
                            variables.insert(save_key.clone(), out.clone());
                        }
                    }
                }
                Ok(last_output)
            }
            Action::SubWorkflow { path } => {
                let p = interpolate(path, variables);
                let resolved = std::path::Path::new(&p)
                    .canonicalize()
                    .map_err(|e| Error::Cdp(format!("invalid sub-workflow path: {e}")))?;
                let allowed = std::env::current_dir()
                    .unwrap_or_else(|_| std::path::PathBuf::from("."))
                    .canonicalize()
                    .unwrap_or_else(|_| std::path::PathBuf::from("/"));
                if !resolved.starts_with(&allowed) {
                    return Err(Error::Cdp(format!(
                        "sub-workflow path escapes allowed directory: {}",
                        resolved.display()
                    )));
                }
                let sub = load_from_file(resolved.to_str().unwrap_or(&p))?;
                let sub_result = Box::pin(execute_workflow(page, &sub)).await?;
                // Merge sub-workflow variables back
                for (k, v) in &sub_result.variables {
                    variables.insert(k.clone(), v.clone());
                }
                Ok(Some(serde_json::to_value(&sub_result)
                    .unwrap_or(serde_json::Value::Null)))
            }
            Action::HttpRequest { url, method, headers, body } => {
                let url = interpolate(url, variables);
                let method_str = method.as_deref().unwrap_or("GET");
                let client = reqwest::Client::new();
                let mut req = match method_str.to_uppercase().as_str() {
                    "POST" => client.post(&url),
                    "PUT" => client.put(&url),
                    "DELETE" => client.delete(&url),
                    "PATCH" => client.patch(&url),
                    _ => client.get(&url),
                };
                if let Some(hdrs) = headers {
                    for (k, v) in hdrs {
                        let v = interpolate(v, variables);
                        req = req.header(k.as_str(), v);
                    }
                }
                if let Some(b) = body {
                    let b = interpolate(b, variables);
                    req = req.body(b);
                }
                let resp = req.send().await
                    .map_err(|e| Error::Cdp(format!("HTTP request failed: {e}")))?;
                let status = resp.status().as_u16();
                let body_text = resp.text().await.unwrap_or_default();
                let body_val = serde_json::from_str::<serde_json::Value>(&body_text)
                    .unwrap_or(serde_json::Value::String(body_text));
                Ok(Some(serde_json::json!({ "status": status, "body": body_val })))
            }
            Action::Agent { prompt, options } => {
                let url = crate::navigation::get_url(page).await.unwrap_or_default();
                let context = AgentStepContext {
                    step_index,
                    prompt: prompt.clone(),
                    options: options.clone().unwrap_or_default(),
                    url,
                    variables: variables.iter().map(|(k,v)| (k.clone(), v.clone())).collect(),
                };
                Ok(Some(serde_json::to_value(&context).unwrap_or(serde_json::Value::Null)))
            }
            Action::Snapshot { compact, interactive_only } => {
                let opts = crate::accessibility::AgentSnapshotOptions {
                    interactive_only: *interactive_only,
                    compact: *compact,
                    ..Default::default()
                };
                let result = crate::accessibility::agent_snapshot(page, &opts).await?;
                Ok(Some(serde_json::json!(result)))
            }
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_simple_workflow() {
        let json = r##"{
            "name": "Login Flow",
            "description": "Automate login to example.com",
            "steps": [
                { "name": "Go to login", "action": { "type": "navigate", "url": "https://example.com/login" } },
                { "name": "Enter username", "action": { "type": "type", "selector": "#username", "text": "user@test.com" } },
                { "name": "Enter password", "action": { "type": "type", "selector": "#password", "text": "secret" } },
                { "name": "Click login", "action": { "type": "click", "selector": "#submit" } },
                { "name": "Wait for dashboard", "action": { "type": "wait_for_selector", "selector": ".dashboard" } }
            ]
        }"##;
        let wf = parse_json(json).unwrap();
        assert_eq!(wf.name, "Login Flow");
        assert_eq!(wf.steps.len(), 5);
    }

    #[test]
    fn interpolation() {
        let mut vars = HashMap::new();
        vars.insert("name".into(), serde_json::json!("Alice"));
        vars.insert("count".into(), serde_json::json!(42));
        assert_eq!(interpolate("Hello {{name}}, you have {{count}} items", &vars), "Hello Alice, you have 42 items");
    }

    #[test]
    fn condition_equality() {
        let mut vars = HashMap::new();
        vars.insert("status".into(), serde_json::json!("success"));
        assert!(evaluate_condition("status == \"success\"", &vars));
        assert!(!evaluate_condition("status == \"failed\"", &vars));
        assert!(evaluate_condition("status != \"failed\"", &vars));
    }

    #[test]
    fn condition_truthy() {
        let mut vars = HashMap::new();
        vars.insert("logged_in".into(), serde_json::json!(true));
        vars.insert("count".into(), serde_json::json!(5));
        vars.insert("empty".into(), serde_json::json!(""));
        assert!(evaluate_condition("logged_in", &vars));
        assert!(evaluate_condition("count", &vars));
        assert!(!evaluate_condition("empty", &vars));
        assert!(!evaluate_condition("nonexistent", &vars));
    }

    #[test]
    fn condition_negation() {
        let mut vars = HashMap::new();
        vars.insert("done".into(), serde_json::json!(false));
        assert!(evaluate_condition("!done", &vars));
    }

    #[test]
    fn condition_comparison() {
        let mut vars = HashMap::new();
        vars.insert("retries".into(), serde_json::json!(3));
        assert!(evaluate_condition("retries > 2", &vars));
        assert!(!evaluate_condition("retries > 5", &vars));
        assert!(evaluate_condition("retries < 10", &vars));
    }

    #[test]
    fn validate_workflow() {
        let wf = Workflow {
            name: "test".into(),
            description: String::new(),
            version: "1.0".into(),
            variables: HashMap::new(),
            steps: vec![
                Step {
                    id: "s1".into(),
                    name: "navigate".into(),
                    action: Action::Navigate { url: String::new() },
                    condition: None,
                    retries: 0,
                    retry_delay_ms: 0,
                    timeout_ms: None,
                    on_error: None,
                    save_as: None,
                },
            ],
            on_error: ErrorHandler::default(),
        };
        let errors = validate(&wf);
        assert_eq!(errors.len(), 1);
        assert!(errors[0].contains("non-empty url"));
    }

    #[test]
    fn validate_empty_workflow() {
        let wf = Workflow {
            name: String::new(),
            description: String::new(),
            version: String::new(),
            variables: HashMap::new(),
            steps: vec![],
            on_error: ErrorHandler::default(),
        };
        let errors = validate(&wf);
        assert!(errors.len() >= 2); // name + no steps
    }

    #[test]
    fn complex_workflow_with_loop_and_conditional() {
        let json = r#"{
            "name": "Scrape Products",
            "variables": { "base_url": "https://shop.com", "max_pages": 5 },
            "steps": [
                {
                    "name": "Navigate",
                    "action": { "type": "navigate", "url": "{{base_url}}/products" }
                },
                {
                    "name": "Loop pages",
                    "action": {
                        "type": "loop",
                        "items": { "start": 1, "end": 5 },
                        "variable": "page",
                        "steps": [
                            { "name": "Extract", "action": { "type": "extract", "selector": ".product-name" } },
                            {
                                "name": "Check next",
                                "action": {
                                    "type": "conditional",
                                    "condition": "page < 5",
                                    "then_steps": [
                                        { "name": "Next page", "action": { "type": "click", "selector": ".next-page" } }
                                    ]
                                }
                            }
                        ]
                    }
                }
            ]
        }"#;
        let wf = parse_json(json).unwrap();
        assert_eq!(wf.name, "Scrape Products");
        assert_eq!(wf.variables.len(), 2);
        assert_eq!(wf.steps.len(), 2);
    }

    #[test]
    fn step_error_handling_parse() {
        let json = r##"{
            "name": "Resilient",
            "steps": [
                {
                    "name": "Risky click",
                    "action": { "type": "click", "selector": "#maybe" },
                    "retries": 3,
                    "retry_delay_ms": 1000,
                    "on_error": "continue"
                }
            ],
            "on_error": { "action": "stop", "screenshot": true, "log": true }
        }"##;
        let wf = parse_json(json).unwrap();
        assert_eq!(wf.steps[0].retries, 3);
        assert!(wf.on_error.screenshot);
    }

    #[test]
    fn http_request_action() {
        let json = r#"{
            "name": "API Check",
            "steps": [
                {
                    "name": "Call API",
                    "action": {
                        "type": "http_request",
                        "url": "https://api.example.com/status",
                        "method": "GET",
                        "headers": { "Authorization": "Bearer {{token}}" }
                    },
                    "save_as": "api_result"
                }
            ]
        }"#;
        let wf = parse_json(json).unwrap();
        match &wf.steps[0].action {
            Action::HttpRequest { url, method, .. } => {
                assert_eq!(url, "https://api.example.com/status");
                assert_eq!(method.as_deref(), Some("GET"));
            }
            _ => panic!("expected HttpRequest action"),
        }
    }
}
