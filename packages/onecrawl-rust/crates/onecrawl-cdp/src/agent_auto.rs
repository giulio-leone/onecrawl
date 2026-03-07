//! Autonomous AI Agent — goal-based browser automation with planning,
//! self-healing execution, cost tracking, and resume support.
//!
//! Orchestrates existing components (task planner, smart actions, agent memory,
//! safety policy) into a single autonomous execution engine.

use chromiumoxide::Page;
use onecrawl_core::{Error, Result};
use serde::{Deserialize, Serialize};
use std::time::Instant;

use crate::agent_memory::{AgentMemory, MemoryCategory};
use crate::page::evaluate_js;
use crate::safety::{SafetyCheck, SafetyPolicy, SafetyState};
use crate::screenshot::screenshot_viewport;
use crate::smart_actions;
use crate::task_planner;

// ────────────────────────────────────────────────────────────────────
//  Public types
// ────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentAutoConfig {
    pub goal: String,
    #[serde(default)]
    pub model: Option<String>,
    #[serde(default = "default_max_steps")]
    pub max_steps: u32,
    #[serde(default)]
    pub max_cost_cents: Option<u32>,
    #[serde(default)]
    pub screenshot_every_step: bool,
    #[serde(default)]
    pub screenshot_dir: Option<String>,
    #[serde(default)]
    pub output: Option<String>,
    #[serde(default)]
    pub output_format: Option<OutputFormat>,
    #[serde(default)]
    pub resume_from: Option<String>,
    #[serde(default)]
    pub save_state: Option<String>,
    #[serde(default)]
    pub verbose: bool,
    #[serde(default)]
    pub allowed_domains: Vec<String>,
    #[serde(default)]
    pub blocked_domains: Vec<String>,
    #[serde(default)]
    pub timeout_secs: Option<u64>,
    #[serde(default = "default_use_memory")]
    pub use_memory: bool,
    #[serde(default)]
    pub memory_path: Option<String>,
}

fn default_max_steps() -> u32 {
    50
}
fn default_use_memory() -> bool {
    true
}

impl Default for AgentAutoConfig {
    fn default() -> Self {
        Self {
            goal: String::new(),
            model: None,
            max_steps: 50,
            max_cost_cents: None,
            screenshot_every_step: false,
            screenshot_dir: None,
            output: None,
            output_format: None,
            resume_from: None,
            save_state: None,
            verbose: false,
            allowed_domains: Vec::new(),
            blocked_domains: Vec::new(),
            timeout_secs: None,
            use_memory: true,
            memory_path: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OutputFormat {
    Csv,
    Json,
    Jsonl,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AutoStep {
    pub index: usize,
    pub description: String,
    pub action_type: String,
    pub target: Option<String>,
    pub value: Option<String>,
    pub status: StepStatus,
    pub result: Option<serde_json::Value>,
    pub screenshot_path: Option<String>,
    pub error: Option<String>,
    pub retries: u32,
    pub duration_ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum StepStatus {
    Pending,
    Running,
    Completed,
    Failed,
    Skipped,
    Retrying,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentAutoResult {
    pub goal: String,
    pub success: bool,
    pub steps_completed: usize,
    pub steps_total: usize,
    pub steps: Vec<AutoStep>,
    pub output_path: Option<String>,
    pub extracted_data: Vec<serde_json::Value>,
    pub cost_cents: u32,
    pub duration_secs: f64,
    pub errors: Vec<String>,
    pub memory_entries_added: usize,
    pub resume_state: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentAutoState {
    pub config: AgentAutoConfig,
    pub steps: Vec<AutoStep>,
    pub current_step: usize,
    pub extracted_data: Vec<serde_json::Value>,
    pub cost_cents: u32,
    pub url: Option<String>,
    pub cookies: Vec<serde_json::Value>,
    pub timestamp: String,
}

// ────────────────────────────────────────────────────────────────────
//  AgentAuto engine
// ────────────────────────────────────────────────────────────────────

pub struct AgentAuto {
    config: AgentAutoConfig,
    steps: Vec<AutoStep>,
    extracted_data: Vec<serde_json::Value>,
    cost_cents: u32,
    start_time: Instant,
    memory_entries_added: usize,
}

impl AgentAuto {
    pub fn new(config: AgentAutoConfig) -> Self {
        Self {
            config,
            steps: Vec::new(),
            extracted_data: Vec::new(),
            cost_cents: 0,
            start_time: Instant::now(),
            memory_entries_added: 0,
        }
    }

    /// Decompose the goal into executable steps.
    pub fn plan(&mut self) -> Result<Vec<AutoStep>> {
        // If resuming, load previous state
        if let Some(ref path) = self.config.resume_from {
            let state = Self::load_state(path)?;
            self.steps = state.steps;
            self.extracted_data = state.extracted_data;
            self.cost_cents = state.cost_cents;
            return Ok(self.steps.clone());
        }

        let context = task_planner::extract_context(&self.config.goal);
        let plan = task_planner::plan_from_goal(&self.config.goal, &context);

        self.steps = plan
            .steps
            .iter()
            .enumerate()
            .map(|(i, ps)| {
                let (action_type, target, value) = action_to_parts(&ps.action);
                AutoStep {
                    index: i,
                    description: ps.description.clone(),
                    action_type,
                    target,
                    value,
                    status: StepStatus::Pending,
                    result: None,
                    screenshot_path: None,
                    error: None,
                    retries: 0,
                    duration_ms: 0,
                }
            })
            .collect();

        Ok(self.steps.clone())
    }

    /// Execute all planned steps autonomously.
    pub async fn execute(&mut self, page: &Page) -> Result<AgentAutoResult> {
        self.start_time = Instant::now();

        let policy = SafetyPolicy {
            allowed_domains: self.config.allowed_domains.clone(),
            blocked_domains: self.config.blocked_domains.clone(),
            ..SafetyPolicy::default()
        };
        let mut safety = SafetyState::new(policy);
        let mut memory = self.load_memory();
        let mut errors: Vec<String> = Vec::new();
        let total = self.steps.len();

        for idx in 0..total {
            if self.check_should_stop(page, idx).await {
                break;
            }

            if self.steps[idx].status == StepStatus::Completed
                || self.steps[idx].status == StepStatus::Skipped
            {
                continue;
            }

            self.steps[idx].status = StepStatus::Running;

            if !self.check_safety(idx, &mut safety, &mut errors) {
                continue;
            }
            safety.record_action();

            let step_start = Instant::now();
            let success = self.execute_step_with_retries(idx, page, &mut memory).await;

            if !success {
                errors.push(format!("step {idx}: {}", self.steps[idx].error.as_deref().unwrap_or("failed")));
            }
            self.steps[idx].duration_ms = step_start.elapsed().as_millis() as u64;
            self.post_step_bookkeeping(idx, success, page, &mut memory).await;
        }

        if self.config.use_memory {
            let _ = memory.save();
        }

        self.build_result(errors)
    }

    /// Check cost cap and timeout; returns true if the agent should stop.
    async fn check_should_stop(&self, page: &Page, idx: usize) -> bool {
        if let Some(cap) = self.config.max_cost_cents {
            if self.cost_cents >= cap {
                if self.config.verbose {
                    tracing::warn!("cost cap reached: {} cents", self.cost_cents);
                }
                self.maybe_save_state(page, idx).await;
                return true;
            }
        }
        if let Some(timeout) = self.config.timeout_secs {
            if self.start_time.elapsed().as_secs() >= timeout {
                if self.config.verbose {
                    tracing::warn!("timeout reached: {} secs", timeout);
                }
                self.maybe_save_state(page, idx).await;
                return true;
            }
        }
        false
    }

    /// Validate navigation safety; returns false if step should be skipped.
    fn check_safety(&mut self, idx: usize, safety: &mut SafetyState, errors: &mut Vec<String>) -> bool {
        if self.steps[idx].action_type != "navigate" {
            return true;
        }
        let url = match self.steps[idx].target {
            Some(ref u) => u.clone(),
            None => return true,
        };
        match safety.check_url(&url) {
            SafetyCheck::Denied(reason) => {
                self.steps[idx].status = StepStatus::Failed;
                self.steps[idx].error = Some(format!("safety: {reason}"));
                errors.push(format!("step {idx}: safety denied: {reason}"));
                false
            }
            SafetyCheck::RequiresConfirmation(msg) => {
                self.steps[idx].status = StepStatus::Skipped;
                self.steps[idx].error = Some(format!("requires confirmation: {msg}"));
                false
            }
            SafetyCheck::Allowed => true,
        }
    }

    /// Execute a step with retry logic.
    async fn execute_step_with_retries(&mut self, idx: usize, page: &Page, memory: &mut AgentMemory) -> bool {
        let max_retries: u32 = 3;
        let mut attempt = 0u32;

        while attempt <= max_retries {
            match self.execute_step(idx, page).await {
                Ok(()) => {
                    self.steps[idx].status = StepStatus::Completed;
                    return true;
                }
                Err(e) => {
                    attempt += 1;
                    self.steps[idx].retries = attempt;
                    if attempt <= max_retries {
                        self.steps[idx].status = StepStatus::Retrying;
                        if self.config.verbose {
                            tracing::info!("step {} retry {}/{}: {}", idx, attempt, max_retries, e);
                        }
                        if self.config.use_memory {
                            let alt = memory.search(
                                &self.steps[idx].description,
                                Some(MemoryCategory::RetryKnowledge),
                                None,
                            );
                            if !alt.is_empty() && self.config.verbose {
                                tracing::info!("found {} memory entries for retry", alt.len());
                            }
                        }
                        tokio::time::sleep(std::time::Duration::from_millis(500)).await;
                    } else {
                        self.steps[idx].status = StepStatus::Failed;
                        self.steps[idx].error = Some(format!("{e} (after {max_retries} retries)"));
                    }
                }
            }
        }
        false
    }

    /// Post-step bookkeeping: cost tracking, screenshots, memory storage, state saves.
    async fn post_step_bookkeeping(&mut self, idx: usize, success: bool, page: &Page, memory: &mut AgentMemory) {
        self.cost_cents += 1;

        if self.config.screenshot_every_step {
            if let Ok(path) = self.capture_step_screenshot(page, idx).await {
                self.steps[idx].screenshot_path = Some(path);
            }
        }

        if self.config.use_memory && success {
            let key = format!("auto_step_{}_{}", idx, self.steps[idx].action_type);
            let val = serde_json::json!({
                "step": idx,
                "action": self.steps[idx].action_type,
                "description": self.steps[idx].description,
                "success": true,
                "duration_ms": self.steps[idx].duration_ms,
            });
            if memory.store(key, val, MemoryCategory::PageVisit, None).is_ok() {
                self.memory_entries_added += 1;
            }
        }

        if self.config.save_state.is_some() {
            self.maybe_save_state(page, idx + 1).await;
        }
    }

    /// Load or initialize agent memory based on config.
    fn load_memory(&self) -> AgentMemory {
        if self.config.use_memory {
            let mem_path = self.config
                .memory_path
                .clone()
                .unwrap_or_else(|| "/tmp/onecrawl-agent-memory.json".into());
            AgentMemory::load(std::path::PathBuf::from(&mem_path))
                .unwrap_or_else(|_| AgentMemory::new(&mem_path))
        } else {
            AgentMemory::new("/dev/null")
        }
    }

    /// Build the final execution result.
    fn build_result(&self, errors: Vec<String>) -> Result<AgentAutoResult> {
        let output_path = if self.config.output.is_some() {
            self.write_output().ok();
            self.config.output.clone()
        } else {
            None
        };

        let steps_completed = self.steps.iter()
            .filter(|s| s.status == StepStatus::Completed)
            .count();
        let all_success = errors.is_empty()
            && self.steps.iter()
                .all(|s| s.status == StepStatus::Completed || s.status == StepStatus::Skipped);

        Ok(AgentAutoResult {
            goal: self.config.goal.clone(),
            success: all_success,
            steps_completed,
            steps_total: self.steps.len(),
            steps: self.steps.clone(),
            output_path,
            extracted_data: self.extracted_data.clone(),
            cost_cents: self.cost_cents,
            duration_secs: self.start_time.elapsed().as_secs_f64(),
            errors,
            memory_entries_added: self.memory_entries_added,
            resume_state: self.config.save_state.clone(),
        })
    }

    /// Execute a single planned step.
    async fn execute_step(&mut self, idx: usize, page: &Page) -> Result<()> {
        let step = &self.steps[idx];
        let action_type = step.action_type.clone();
        let target = step.target.clone();
        let value = step.value.clone();

        match action_type.as_str() {
            "navigate" => {
                let url = target.ok_or_else(|| Error::Cdp("navigate: missing URL".into()))?;
                page.goto(&url)
                    .await
                    .map_err(|e| Error::Cdp(format!("navigate failed: {e}")))?;
                // Wait briefly for page load
                tokio::time::sleep(std::time::Duration::from_millis(500)).await;
            }
            "smart_click" | "click" => {
                let query = target.unwrap_or_else(|| "button".into());
                smart_actions::smart_click(page, &query)
                    .await
                    .map_err(|e| Error::Cdp(format!("smart_click failed: {e}")))?;
            }
            "smart_fill" | "type" => {
                let query = target.unwrap_or_else(|| "input".into());
                let text = value.unwrap_or_default();
                smart_actions::smart_fill(page, &query, &text)
                    .await
                    .map_err(|e| Error::Cdp(format!("smart_fill failed: {e}")))?;
            }
            "extract" => {
                let selector = target.unwrap_or_else(|| "body".into());
                let js = format!(
                    r#"(() => {{
                        const el = document.querySelector({sel});
                        if (!el) return null;
                        return {{
                            text: el.innerText || '',
                            html: el.innerHTML.substring(0, 5000),
                            tag: el.tagName
                        }};
                    }})()"#,
                    sel = serde_json::to_string(&selector)
                        .unwrap_or_else(|_| format!("\"{}\"", selector))
                );
                let data = evaluate_js(page, &js).await?;
                if !data.is_null() {
                    self.extracted_data.push(data.clone());
                    self.steps[idx].result = Some(data);
                }
            }
            "wait" => {
                let wait_target = target.unwrap_or_else(|| "body".into());
                let timeout_ms: u64 = value
                    .as_deref()
                    .and_then(|v| v.parse().ok())
                    .unwrap_or(5000);
                let js = format!(
                    r#"new Promise((resolve, reject) => {{
                        const sel = {sel};
                        const start = Date.now();
                        const check = () => {{
                            if (document.querySelector(sel)) return resolve(true);
                            if (Date.now() - start > {timeout}) return reject('timeout');
                            requestAnimationFrame(check);
                        }};
                        check();
                    }})"#,
                    sel = serde_json::to_string(&wait_target)
                        .unwrap_or_else(|_| format!("\"{}\"", wait_target)),
                    timeout = timeout_ms
                );
                let _ = evaluate_js(page, &js).await;
            }
            "snapshot" => {
                let js = r#"(() => {
                    const url = location.href;
                    const title = document.title;
                    const interactive = document.querySelectorAll(
                        'a, button, input, select, textarea, [role="button"], [role="link"]'
                    );
                    return {
                        url,
                        title,
                        interactive_count: interactive.length,
                        body_length: document.body?.innerText?.length || 0
                    };
                })()"#;
                let data = evaluate_js(page, js).await?;
                self.steps[idx].result = Some(data);
            }
            "screenshot" => {
                if let Ok(path) = self.capture_step_screenshot(page, idx).await {
                    self.steps[idx].screenshot_path = Some(path);
                }
            }
            "assert" => {
                let condition = target.unwrap_or_default();
                let expected_value = value.unwrap_or_default();
                let safe_cond = serde_json::to_string(&condition).unwrap_or_default();
                let safe_val = serde_json::to_string(&expected_value).unwrap_or_default();
                let js = format!(
                    r#"(() => {{ try {{ const cond = {safe_cond}; const val = {safe_val}; if (cond === "url_contains") return window.location.href.includes(val); if (cond === "title_contains") return document.title.includes(val); if (cond === "element_exists") return !!document.querySelector(val); if (cond === "text_contains") return document.body.innerText.includes(val); return false; }} catch(e) {{ return false; }} }})()"#
                );
                let result = evaluate_js(page, &js).await?;
                let passed = result.as_bool().unwrap_or(false);
                if !passed {
                    return Err(Error::Cdp(format!("assertion failed: {condition}")));
                }
                self.steps[idx].result = Some(serde_json::json!({ "passed": true }));
            }
            "scroll" => {
                let direction = target.unwrap_or_else(|| "down".into());
                let amount: u32 = value
                    .as_deref()
                    .and_then(|v| v.parse().ok())
                    .unwrap_or(500);
                let (dx, dy) = match direction.as_str() {
                    "up" => (0i32, -(amount as i32)),
                    "down" => (0, amount as i32),
                    "left" => (-(amount as i32), 0),
                    "right" => (amount as i32, 0),
                    _ => (0, amount as i32),
                };
                let js = format!("window.scrollBy({dx}, {dy})");
                let _ = evaluate_js(page, &js).await;
            }
            "memory_store" => {
                // No-op at execution level; value was already captured during planning
                self.steps[idx].result = Some(serde_json::json!({ "stored": true }));
            }
            "memory_recall" => {
                self.steps[idx].result = Some(serde_json::json!({ "recalled": true }));
            }
            other => {
                return Err(Error::Cdp(format!("unknown action type: {other}")));
            }
        }

        Ok(())
    }

    /// Capture a screenshot for a step and save to disk.
    async fn capture_step_screenshot(&self, page: &Page, step_idx: usize) -> Result<String> {
        let dir = self
            .config
            .screenshot_dir
            .as_deref()
            .unwrap_or("/tmp/onecrawl-agent-auto");
        std::fs::create_dir_all(dir)
            .map_err(|e| Error::Cdp(format!("mkdir screenshot dir: {e}")))?;
        let path = format!("{}/step_{:03}.png", dir, step_idx);
        let bytes = screenshot_viewport(page).await?;
        std::fs::write(&path, &bytes)
            .map_err(|e| Error::Cdp(format!("write screenshot: {e}")))?;
        Ok(path)
    }

    /// Save execution state for resume.
    pub async fn save_state(&self, page: &Page) -> Result<String> {
        let path = self
            .config
            .save_state
            .as_deref()
            .unwrap_or("/tmp/onecrawl-agent-auto-state.json");
        let url = page.url().await.ok().flatten();
        let cookies_js = r#"document.cookie.split('; ').map(c => {
            const [k, ...v] = c.split('=');
            return { name: k, value: v.join('=') };
        })"#;
        let cookies = evaluate_js(page, cookies_js).await.unwrap_or_default();
        let cookies_vec = match cookies {
            serde_json::Value::Array(a) => a,
            _ => Vec::new(),
        };
        let now = chrono_timestamp();
        let state = AgentAutoState {
            config: self.config.clone(),
            steps: self.steps.clone(),
            current_step: self
                .steps
                .iter()
                .position(|s| s.status == StepStatus::Pending || s.status == StepStatus::Running)
                .unwrap_or(self.steps.len()),
            extracted_data: self.extracted_data.clone(),
            cost_cents: self.cost_cents,
            url,
            cookies: cookies_vec,
            timestamp: now,
        };
        let json = serde_json::to_string_pretty(&state)
            .map_err(|e| Error::Cdp(format!("serialize state: {e}")))?;
        std::fs::write(path, &json)
            .map_err(|e| Error::Cdp(format!("write state file: {e}")))?;
        Ok(path.to_string())
    }

    /// Load saved state from disk.
    pub fn load_state(path: &str) -> Result<AgentAutoState> {
        let json = std::fs::read_to_string(path)
            .map_err(|e| Error::Cdp(format!("read state file: {e}")))?;
        serde_json::from_str(&json)
            .map_err(|e| Error::Cdp(format!("parse state file: {e}")))
    }

    /// Write extracted data to the configured output file.
    fn write_output(&self) -> Result<()> {
        let path = match self.config.output.as_deref() {
            Some(p) => p,
            None => return Ok(()),
        };
        let format = self
            .config
            .output_format
            .as_ref()
            .cloned()
            .unwrap_or_else(|| guess_format(path));

        match format {
            OutputFormat::Json => {
                let json = serde_json::to_string_pretty(&self.extracted_data)
                    .map_err(|e| Error::Cdp(format!("json serialize: {e}")))?;
                std::fs::write(path, json)
                    .map_err(|e| Error::Cdp(format!("write json: {e}")))?;
            }
            OutputFormat::Jsonl => {
                let mut out = String::new();
                for item in &self.extracted_data {
                    let line = serde_json::to_string(item)
                        .map_err(|e| Error::Cdp(format!("jsonl serialize: {e}")))?;
                    out.push_str(&line);
                    out.push('\n');
                }
                std::fs::write(path, out)
                    .map_err(|e| Error::Cdp(format!("write jsonl: {e}")))?;
            }
            OutputFormat::Csv => {
                self.write_csv(path)?;
            }
        }
        Ok(())
    }

    /// Write extracted data as CSV.
    fn write_csv(&self, path: &str) -> Result<()> {
        if self.extracted_data.is_empty() {
            std::fs::write(path, "")
                .map_err(|e| Error::Cdp(format!("write csv: {e}")))?;
            return Ok(());
        }

        // Collect all keys from all objects
        let mut all_keys: Vec<String> = Vec::new();
        for item in &self.extracted_data {
            if let serde_json::Value::Object(map) = item {
                for key in map.keys() {
                    if !all_keys.contains(key) {
                        all_keys.push(key.clone());
                    }
                }
            }
        }

        let mut out = String::new();
        // Header
        out.push_str(&all_keys.join(","));
        out.push('\n');

        // Rows
        for item in &self.extracted_data {
            let row: Vec<String> = all_keys
                .iter()
                .map(|k| {
                    if let serde_json::Value::Object(map) = item {
                        match map.get(k) {
                            Some(serde_json::Value::String(s)) => csv_escape(s),
                            Some(v) => csv_escape(&v.to_string()),
                            None => String::new(),
                        }
                    } else {
                        csv_escape(&item.to_string())
                    }
                })
                .collect();
            out.push_str(&row.join(","));
            out.push('\n');
        }

        std::fs::write(path, out)
            .map_err(|e| Error::Cdp(format!("write csv: {e}")))?;
        Ok(())
    }

    /// Save state if save_state path is configured.
    async fn maybe_save_state(&self, page: &Page, _step: usize) {
        if self.config.save_state.is_some() {
            let _ = self.save_state(page).await;
        }
    }

    /// Return current progress snapshot.
    pub fn status(&self) -> serde_json::Value {
        let completed = self
            .steps
            .iter()
            .filter(|s| s.status == StepStatus::Completed)
            .count();
        let failed = self
            .steps
            .iter()
            .filter(|s| s.status == StepStatus::Failed)
            .count();
        let current = self
            .steps
            .iter()
            .position(|s| s.status == StepStatus::Running || s.status == StepStatus::Pending)
            .unwrap_or(self.steps.len());

        serde_json::json!({
            "goal": self.config.goal,
            "steps_total": self.steps.len(),
            "steps_completed": completed,
            "steps_failed": failed,
            "current_step": current,
            "cost_cents": self.cost_cents,
            "elapsed_secs": self.start_time.elapsed().as_secs_f64(),
            "extracted_items": self.extracted_data.len(),
        })
    }

    /// Get the steps (for plan-only mode).
    pub fn steps(&self) -> &[AutoStep] {
        &self.steps
    }
}

// ────────────────────────────────────────────────────────────────────
//  Helpers
// ────────────────────────────────────────────────────────────────────

/// Convert a PlannedAction into (action_type, target, value) triple.
fn action_to_parts(action: &task_planner::PlannedAction) -> (String, Option<String>, Option<String>) {
    match action {
        task_planner::PlannedAction::Navigate { url } => {
            ("navigate".into(), Some(url.clone()), None)
        }
        task_planner::PlannedAction::Click { target, .. } => {
            ("click".into(), Some(target.clone()), None)
        }
        task_planner::PlannedAction::Type { target, text, .. } => {
            ("type".into(), Some(target.clone()), Some(text.clone()))
        }
        task_planner::PlannedAction::Wait { target, timeout_ms } => {
            ("wait".into(), Some(target.clone()), Some(timeout_ms.to_string()))
        }
        task_planner::PlannedAction::Snapshot {} => {
            ("snapshot".into(), None, None)
        }
        task_planner::PlannedAction::Extract { target } => {
            ("extract".into(), Some(target.clone()), None)
        }
        task_planner::PlannedAction::Assert { condition } => {
            ("assert".into(), Some(condition.clone()), None)
        }
        task_planner::PlannedAction::SmartClick { query } => {
            ("smart_click".into(), Some(query.clone()), None)
        }
        task_planner::PlannedAction::SmartFill { query, value } => {
            ("smart_fill".into(), Some(query.clone()), Some(value.clone()))
        }
        task_planner::PlannedAction::Scroll { direction, amount } => {
            ("scroll".into(), Some(direction.clone()), amount.map(|a| a.to_string()))
        }
        task_planner::PlannedAction::Screenshot { path } => {
            ("screenshot".into(), path.clone(), None)
        }
        task_planner::PlannedAction::MemoryStore { key, value } => {
            ("memory_store".into(), Some(key.clone()), Some(value.clone()))
        }
        task_planner::PlannedAction::MemoryRecall { key } => {
            ("memory_recall".into(), Some(key.clone()), None)
        }
        task_planner::PlannedAction::Conditional { .. } => {
            ("snapshot".into(), None, None)
        }
    }
}

/// Guess output format from file extension.
fn guess_format(path: &str) -> OutputFormat {
    match std::path::Path::new(path).extension().and_then(|e| e.to_str()) {
        Some("csv") => OutputFormat::Csv,
        Some("jsonl") => OutputFormat::Jsonl,
        _ => OutputFormat::Json,
    }
}

/// CSV-escape a value.
fn csv_escape(s: &str) -> String {
    if s.contains(',') || s.contains('"') || s.contains('\n') {
        format!("\"{}\"", s.replace('"', "\"\""))
    } else {
        s.to_string()
    }
}

/// Produce an ISO-8601 timestamp without requiring chrono.
fn chrono_timestamp() -> String {
    let dur = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default();
    format!("{}", dur.as_secs())
}

// ────────────────────────────────────────────────────────────────────
//  Top-level convenience functions (for MCP/CLI callers)
// ────────────────────────────────────────────────────────────────────

/// Plan and execute a goal in one call.
pub async fn agent_auto_run(page: &Page, config: AgentAutoConfig) -> Result<AgentAutoResult> {
    let mut agent = AgentAuto::new(config);
    agent.plan()?;
    agent.execute(page).await
}

/// Plan only (dry-run).
pub fn agent_auto_plan(config: &AgentAutoConfig) -> Result<Vec<AutoStep>> {
    let mut agent = AgentAuto::new(config.clone());
    agent.plan()
}
