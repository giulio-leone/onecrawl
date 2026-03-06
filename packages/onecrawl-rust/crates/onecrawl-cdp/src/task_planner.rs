//! AI Task Planner — converts natural language goals into executable steps.
//!
//! Given a high-level goal like "log into Gmail and check inbox",
//! generates a sequence of browser actions using page context (snapshot),
//! domain memory, and known patterns.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// A planned task with steps.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskPlan {
    pub goal: String,
    pub steps: Vec<PlannedStep>,
    pub strategy: PlanStrategy,
    pub estimated_duration_ms: u64,
    pub confidence: f64,
    pub context_used: Vec<String>,
}

/// A single planned step.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlannedStep {
    pub id: usize,
    pub description: String,
    pub action: PlannedAction,
    pub fallback: Option<Box<PlannedStep>>,
    pub confidence: f64,
}

/// Planned action types.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum PlannedAction {
    Navigate { url: String },
    Click { target: String, strategy: String },
    Type { target: String, text: String, strategy: String },
    Wait { target: String, timeout_ms: u64 },
    Snapshot {},
    Extract { target: String },
    Assert { condition: String },
    SmartClick { query: String },
    SmartFill { query: String, value: String },
    Scroll { direction: String, amount: Option<u32> },
    Screenshot { path: Option<String> },
    MemoryStore { key: String, value: String },
    MemoryRecall { key: String },
    Conditional { condition: String, then_step: Box<PlannedStep>, else_step: Option<Box<PlannedStep>> },
}

/// Planning strategy.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum PlanStrategy {
    Direct,
    Exploratory,
    MemoryAssisted,
    Hybrid,
}

/// Execution result for a planned task.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskExecutionResult {
    pub goal: String,
    pub status: TaskStatus,
    pub steps_completed: usize,
    pub steps_total: usize,
    pub steps_results: Vec<StepExecutionResult>,
    pub retries_used: usize,
    pub total_duration_ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum TaskStatus {
    Success,
    PartialSuccess,
    Failed,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StepExecutionResult {
    pub step_id: usize,
    pub description: String,
    pub status: StepOutcome,
    pub output: Option<serde_json::Value>,
    pub error: Option<String>,
    pub used_fallback: bool,
    pub duration_ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum StepOutcome {
    Success,
    Failed,
    Skipped,
}

/// Goal decomposition patterns.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GoalPattern {
    pub keywords: Vec<String>,
    pub category: GoalCategory,
    pub template_steps: Vec<StepTemplate>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum GoalCategory {
    Navigation,
    Authentication,
    DataExtraction,
    FormFilling,
    Search,
    Purchase,
    Interaction,
    Monitoring,
    Generic,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StepTemplate {
    pub description: String,
    pub action_type: String,
    pub requires_context: bool,
}

/// Built-in goal patterns for common tasks.
pub fn builtin_patterns() -> Vec<GoalPattern> {
    vec![
        GoalPattern {
            keywords: vec!["login".into(), "log in".into(), "sign in".into(), "authenticate".into()],
            category: GoalCategory::Authentication,
            template_steps: vec![
                StepTemplate { description: "Navigate to login page".into(), action_type: "navigate".into(), requires_context: true },
                StepTemplate { description: "Take snapshot to find form fields".into(), action_type: "snapshot".into(), requires_context: false },
                StepTemplate { description: "Fill username/email field".into(), action_type: "smart_fill".into(), requires_context: true },
                StepTemplate { description: "Fill password field".into(), action_type: "smart_fill".into(), requires_context: true },
                StepTemplate { description: "Click submit button".into(), action_type: "smart_click".into(), requires_context: true },
                StepTemplate { description: "Wait for page transition".into(), action_type: "wait".into(), requires_context: false },
                StepTemplate { description: "Verify login success".into(), action_type: "snapshot".into(), requires_context: false },
            ],
        },
        GoalPattern {
            keywords: vec!["search".into(), "find".into(), "look for".into(), "query".into()],
            category: GoalCategory::Search,
            template_steps: vec![
                StepTemplate { description: "Navigate to search page".into(), action_type: "navigate".into(), requires_context: true },
                StepTemplate { description: "Find search input".into(), action_type: "snapshot".into(), requires_context: false },
                StepTemplate { description: "Type search query".into(), action_type: "smart_fill".into(), requires_context: true },
                StepTemplate { description: "Submit search".into(), action_type: "smart_click".into(), requires_context: true },
                StepTemplate { description: "Wait for results".into(), action_type: "wait".into(), requires_context: false },
                StepTemplate { description: "Extract results".into(), action_type: "extract".into(), requires_context: true },
            ],
        },
        GoalPattern {
            keywords: vec!["extract".into(), "scrape".into(), "get data".into(), "collect".into()],
            category: GoalCategory::DataExtraction,
            template_steps: vec![
                StepTemplate { description: "Navigate to target page".into(), action_type: "navigate".into(), requires_context: true },
                StepTemplate { description: "Take snapshot to understand structure".into(), action_type: "snapshot".into(), requires_context: false },
                StepTemplate { description: "Extract target data".into(), action_type: "extract".into(), requires_context: true },
            ],
        },
        GoalPattern {
            keywords: vec!["fill".into(), "form".into(), "submit".into(), "complete".into()],
            category: GoalCategory::FormFilling,
            template_steps: vec![
                StepTemplate { description: "Navigate to form page".into(), action_type: "navigate".into(), requires_context: true },
                StepTemplate { description: "Take snapshot to identify fields".into(), action_type: "snapshot".into(), requires_context: false },
                StepTemplate { description: "Fill form fields".into(), action_type: "smart_fill".into(), requires_context: true },
                StepTemplate { description: "Submit form".into(), action_type: "smart_click".into(), requires_context: true },
                StepTemplate { description: "Verify submission success".into(), action_type: "wait".into(), requires_context: false },
            ],
        },
        GoalPattern {
            keywords: vec!["navigate".into(), "go to".into(), "open".into(), "visit".into()],
            category: GoalCategory::Navigation,
            template_steps: vec![
                StepTemplate { description: "Navigate to URL".into(), action_type: "navigate".into(), requires_context: true },
                StepTemplate { description: "Wait for page load".into(), action_type: "wait".into(), requires_context: false },
                StepTemplate { description: "Take snapshot".into(), action_type: "snapshot".into(), requires_context: false },
            ],
        },
        GoalPattern {
            keywords: vec!["click".into(), "press".into(), "tap".into(), "select".into()],
            category: GoalCategory::Interaction,
            template_steps: vec![
                StepTemplate { description: "Take snapshot to find target".into(), action_type: "snapshot".into(), requires_context: false },
                StepTemplate { description: "Click target element".into(), action_type: "smart_click".into(), requires_context: true },
                StepTemplate { description: "Verify action result".into(), action_type: "snapshot".into(), requires_context: false },
            ],
        },
        GoalPattern {
            keywords: vec!["monitor".into(), "watch".into(), "check".into(), "track".into()],
            category: GoalCategory::Monitoring,
            template_steps: vec![
                StepTemplate { description: "Navigate to page".into(), action_type: "navigate".into(), requires_context: true },
                StepTemplate { description: "Take snapshot".into(), action_type: "snapshot".into(), requires_context: false },
                StepTemplate { description: "Extract status data".into(), action_type: "extract".into(), requires_context: true },
                StepTemplate { description: "Store in memory".into(), action_type: "memory_store".into(), requires_context: true },
            ],
        },
    ]
}

/// Match a goal to the best pattern.
pub fn match_goal(goal: &str) -> (GoalCategory, Vec<StepTemplate>) {
    let goal_lower = goal.to_lowercase();
    let patterns = builtin_patterns();

    let mut best_match: Option<&GoalPattern> = None;
    let mut best_score = 0;

    for pattern in &patterns {
        let score: usize = pattern.keywords.iter()
            .filter(|kw| goal_lower.contains(kw.as_str()))
            .count();
        if score > best_score {
            best_score = score;
            best_match = Some(pattern);
        }
    }

    match best_match {
        Some(pattern) => (pattern.category.clone(), pattern.template_steps.clone()),
        None => (GoalCategory::Generic, vec![
            StepTemplate { description: "Take snapshot to understand page".into(), action_type: "snapshot".into(), requires_context: false },
            StepTemplate { description: "Execute goal action".into(), action_type: "smart_click".into(), requires_context: true },
            StepTemplate { description: "Verify result".into(), action_type: "snapshot".into(), requires_context: false },
        ]),
    }
}

/// Create a task plan from a goal.
pub fn plan_from_goal(goal: &str, context: &HashMap<String, String>) -> TaskPlan {
    let (category, templates) = match_goal(goal);

    let mut steps = Vec::new();
    let mut confidence: f64 = 0.7;

    for (i, template) in templates.iter().enumerate() {
        let action = match template.action_type.as_str() {
            "navigate" => {
                let url = context.get("url").cloned().unwrap_or_else(|| "about:blank".into());
                PlannedAction::Navigate { url }
            }
            "snapshot" => PlannedAction::Snapshot {},
            "smart_fill" => {
                let query = context.get("field").cloned().unwrap_or_else(|| "input".into());
                let value = context.get("value").cloned().unwrap_or_default();
                PlannedAction::SmartFill { query, value }
            }
            "smart_click" => {
                let query = context.get("target").cloned().unwrap_or_else(|| "submit".into());
                PlannedAction::SmartClick { query }
            }
            "extract" => {
                let target = context.get("selector").cloned().unwrap_or_else(|| "body".into());
                PlannedAction::Extract { target }
            }
            "wait" => {
                let target = context.get("wait_for").cloned().unwrap_or_else(|| "body".into());
                PlannedAction::Wait { target, timeout_ms: 10000 }
            }
            "memory_store" => {
                let key = context.get("memory_key").cloned().unwrap_or_else(|| "result".into());
                let value = context.get("memory_value").cloned().unwrap_or_default();
                PlannedAction::MemoryStore { key, value }
            }
            _ => PlannedAction::Snapshot {},
        };

        let fallback = if template.requires_context {
            Some(Box::new(PlannedStep {
                id: i * 100 + 1,
                description: format!("Fallback: take snapshot and retry {}", template.description),
                action: PlannedAction::Snapshot {},
                fallback: None,
                confidence: 0.5,
            }))
        } else {
            None
        };

        steps.push(PlannedStep {
            id: i,
            description: template.description.clone(),
            action,
            fallback,
            confidence: if template.requires_context { 0.6 } else { 0.9 },
        });
    }

    if context.contains_key("url") { confidence += 0.1; }
    if context.contains_key("domain_strategy") { confidence += 0.15; }
    confidence = confidence.min(1.0);

    let strategy = if context.contains_key("domain_strategy") {
        PlanStrategy::MemoryAssisted
    } else if category == GoalCategory::Generic {
        PlanStrategy::Exploratory
    } else {
        PlanStrategy::Direct
    };

    TaskPlan {
        goal: goal.to_string(),
        steps,
        strategy,
        estimated_duration_ms: templates.len() as u64 * 2000,
        confidence,
        context_used: context.keys().cloned().collect(),
    }
}

/// Extract context hints from a natural language goal.
pub fn extract_context(goal: &str) -> HashMap<String, String> {
    let mut context = HashMap::new();

    // Extract URL
    let words: Vec<&str> = goal.split_whitespace().collect();
    for word in &words {
        if word.starts_with("http://") || word.starts_with("https://") {
            context.insert("url".into(), word.to_string());
        }
    }

    // Extract quoted values
    let mut in_quote = false;
    let mut quote_start = 0;
    let mut quotes = Vec::new();
    for (i, c) in goal.chars().enumerate() {
        if c == '"' || c == '\'' {
            if in_quote {
                quotes.push(&goal[quote_start + 1..i]);
                in_quote = false;
            } else {
                quote_start = i;
                in_quote = true;
            }
        }
    }

    if let Some(first) = quotes.first() {
        context.insert("value".into(), first.to_string());
    }
    if let Some(second) = quotes.get(1) {
        context.insert("field".into(), second.to_string());
    }

    // Extract email-like patterns
    for word in &words {
        if word.contains('@') && word.contains('.') {
            context.insert("email".into(), word.to_string());
            context.insert("value".into(), word.to_string());
        }
    }

    context
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn match_login_goal() {
        let (category, steps) = match_goal("log in to my account");
        assert_eq!(category, GoalCategory::Authentication);
        assert!(steps.len() >= 5);
    }

    #[test]
    fn match_search_goal() {
        let (category, _) = match_goal("search for rust programming tutorials");
        assert_eq!(category, GoalCategory::Search);
    }

    #[test]
    fn match_extract_goal() {
        let (category, _) = match_goal("extract all product prices from the page");
        assert_eq!(category, GoalCategory::DataExtraction);
    }

    #[test]
    fn match_navigate_goal() {
        let (category, _) = match_goal("go to https://example.com");
        assert_eq!(category, GoalCategory::Navigation);
    }

    #[test]
    fn match_generic_goal() {
        let (category, steps) = match_goal("do a very unusual thing now");
        assert_eq!(category, GoalCategory::Generic);
        assert!(!steps.is_empty());
    }

    #[test]
    fn extract_url_context() {
        let ctx = extract_context("navigate to https://example.com/login");
        assert_eq!(ctx.get("url").unwrap(), "https://example.com/login");
    }

    #[test]
    fn extract_quoted_values() {
        let ctx = extract_context("type \"hello world\" into search");
        assert_eq!(ctx.get("value").unwrap(), "hello world");
    }

    #[test]
    fn extract_email() {
        let ctx = extract_context("login with user@example.com");
        assert_eq!(ctx.get("email").unwrap(), "user@example.com");
    }

    #[test]
    fn plan_from_login_goal() {
        let mut ctx = HashMap::new();
        ctx.insert("url".into(), "https://example.com/login".into());
        let plan = plan_from_goal("login to my account", &ctx);
        assert_eq!(plan.strategy, PlanStrategy::Direct);
        assert!(plan.confidence >= 0.7);
        assert!(!plan.steps.is_empty());
    }

    #[test]
    fn plan_with_memory_strategy() {
        let mut ctx = HashMap::new();
        ctx.insert("domain_strategy".into(), "exists".into());
        let plan = plan_from_goal("login to site", &ctx);
        assert_eq!(plan.strategy, PlanStrategy::MemoryAssisted);
        assert!(plan.confidence > 0.8);
    }

    #[test]
    fn plan_generic_exploratory() {
        let ctx = HashMap::new();
        let plan = plan_from_goal("do something weird", &ctx);
        assert_eq!(plan.strategy, PlanStrategy::Exploratory);
    }

    #[test]
    fn planned_steps_have_fallbacks() {
        let ctx = HashMap::new();
        let plan = plan_from_goal("login to my account", &ctx);
        let steps_with_fallback = plan.steps.iter().filter(|s| s.fallback.is_some()).count();
        assert!(steps_with_fallback > 0);
    }

    #[test]
    fn builtin_patterns_coverage() {
        let patterns = builtin_patterns();
        assert!(patterns.len() >= 7);
        let categories: Vec<GoalCategory> = patterns.iter().map(|p| p.category.clone()).collect();
        assert!(categories.contains(&GoalCategory::Authentication));
        assert!(categories.contains(&GoalCategory::Search));
        assert!(categories.contains(&GoalCategory::DataExtraction));
        assert!(categories.contains(&GoalCategory::Navigation));
    }

    #[test]
    fn form_goal_match() {
        let (category, _) = match_goal("fill out the registration form and submit");
        assert_eq!(category, GoalCategory::FormFilling);
    }
}
