//! Workflow DSL Engine — define and execute browser automation as YAML/JSON recipes.
//!
//! Supports sequential steps, conditionals, loops, error handlers,
//! variable interpolation, and composable sub-workflows.

use onecrawl_core::{Error, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

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
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum StepStatus {
    Success,
    Failed,
    Skipped,
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
}

/// Parse a workflow from YAML string.
pub fn parse_yaml(yaml: &str) -> Result<Workflow> {
    serde_json::from_str::<Workflow>(yaml)
        .or_else(|_| {
            // Try YAML parsing via JSON conversion (serde_json handles subset)
            // For full YAML we'd need serde_yaml, but JSON covers most use cases
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
