//! Handler implementations for the `automate` super-tool.

use rmcp::{ErrorData as McpError, model::*};
use crate::cdp_tools::*;
use crate::helpers::{mcp_err, ensure_page, json_ok, parse_json_str, json_escape, McpResult};
use crate::types::*;
use crate::OneCrawlMcp;
use std::collections::HashMap;

impl OneCrawlMcp {

    // ════════════════════════════════════════════════════════════════
    //  Workflow DSL tools
    // ════════════════════════════════════════════════════════════════

    pub(crate) async fn workflow_validate(
        &self,
        p: WorkflowValidateParams,
    ) -> Result<CallToolResult, McpError> {
        let workflow = onecrawl_cdp::workflow::parse_json(&p.workflow)
            .mcp()?;
        let errors = onecrawl_cdp::workflow::validate(&workflow);
        if errors.is_empty() {
            json_ok(&serde_json::json!({
                "valid": true,
                "name": workflow.name,
                "steps": workflow.steps.len(),
                "variables": workflow.variables.keys().collect::<Vec<_>>(),
            }))
        } else {
            json_ok(&serde_json::json!({
                "valid": false,
                "errors": errors,
            }))
        }
    }


    pub(crate) async fn workflow_run(
        &self,
        p: WorkflowRunParams,
    ) -> Result<CallToolResult, McpError> {
        let mut workflow = if p.workflow.trim().starts_with('{') {
            onecrawl_cdp::workflow::parse_json(&p.workflow)
                .mcp()?
        } else {
            onecrawl_cdp::workflow::load_from_file(&p.workflow)
                .mcp()?
        };

        // Override variables
        if let Some(overrides) = p.variables {
            for (k, v) in overrides {
                workflow.variables.insert(k, v);
            }
        }

        // Validate first
        let errors = onecrawl_cdp::workflow::validate(&workflow);
        if !errors.is_empty() {
            return json_ok(&serde_json::json!({
                "status": "validation_failed",
                "errors": errors,
            }));
        }

        let page = ensure_page(&self.browser).await?;
        let start = std::time::Instant::now();
        let mut results: Vec<onecrawl_cdp::StepResult> = Vec::new();
        let mut variables = workflow.variables.clone();
        let mut succeeded = 0usize;
        let mut failed = 0usize;
        let mut skipped = 0usize;
        let mut overall_status = onecrawl_cdp::StepStatus::Success;

        for (i, step) in workflow.steps.iter().enumerate() {
            let step_id = if step.id.is_empty() { format!("step_{i}") } else { step.id.clone() };
            let step_name = if step.name.is_empty() { format!("Step {i}") } else { step.name.clone() };

            // Check condition
            if let Some(ref cond) = step.condition {
                let interpolated = onecrawl_cdp::workflow::interpolate(cond, &variables);
                if !onecrawl_cdp::workflow::evaluate_condition(&interpolated, &variables) {
                    results.push(onecrawl_cdp::StepResult {
                        step_id, step_name,
                        status: onecrawl_cdp::StepStatus::Skipped,
                        output: None, error: None, duration_ms: 0,
                        paused: false,
                    });
                    skipped += 1;
                    continue;
                }
            }

            let step_start = std::time::Instant::now();
            let result = self.execute_step(&page, &step.action, &mut variables).await;
            let duration_ms = step_start.elapsed().as_millis() as u64;

            match result {
                Ok(output) => {
                    if let Some(ref save_key) = step.save_as {
                        if let Some(ref out) = output {
                            variables.insert(save_key.clone(), out.clone());
                        }
                    }
                    results.push(onecrawl_cdp::StepResult {
                        step_id, step_name,
                        status: onecrawl_cdp::StepStatus::Success,
                        output, error: None, duration_ms,
                        paused: false,
                    });
                    succeeded += 1;
                }
                Err(e) => {
                    let err_msg = format!("{}", e.message);
                    let error_action = step.on_error.as_ref()
                        .unwrap_or(&workflow.on_error.action);
                    results.push(onecrawl_cdp::StepResult {
                        step_id, step_name,
                        status: onecrawl_cdp::StepStatus::Failed,
                        output: None, error: Some(err_msg.clone()), duration_ms,
                        paused: false,
                    });
                    failed += 1;

                    match error_action {
                        onecrawl_cdp::workflow::StepErrorAction::Stop => {
                            overall_status = onecrawl_cdp::StepStatus::Failed;
                            break;
                        }
                        onecrawl_cdp::workflow::StepErrorAction::Continue |
                        onecrawl_cdp::workflow::StepErrorAction::Skip => continue,
                        onecrawl_cdp::workflow::StepErrorAction::Retry => continue,
                    }
                }
            }
        }

        let total_duration_ms = start.elapsed().as_millis() as u64;
        json_ok(&serde_json::json!({
            "name": workflow.name,
            "status": format!("{:?}", overall_status).to_lowercase(),
            "total_duration_ms": total_duration_ms,
            "steps_succeeded": succeeded,
            "steps_failed": failed,
            "steps_skipped": skipped,
            "steps": results,
            "variables": variables,
        }))
    }

    pub(crate) fn execute_step<'a>(
        &'a self,
        page: &'a chromiumoxide::Page,
        action: &'a onecrawl_cdp::workflow::Action,
        variables: &'a mut HashMap<String, serde_json::Value>,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = std::result::Result<Option<serde_json::Value>, McpError>> + Send + 'a>> {
        Box::pin(async move {
        use onecrawl_cdp::workflow::Action;
        match action {
            Action::Navigate { url } => {
                let url = onecrawl_cdp::workflow::interpolate(url, variables);
                onecrawl_cdp::navigation::goto(page, &url).await.mcp()?;
                let title = onecrawl_cdp::navigation::get_title(page).await.unwrap_or_default();
                Ok(Some(serde_json::json!({ "url": url, "title": title })))
            }
            Action::Click { selector } => {
                let sel = onecrawl_cdp::workflow::interpolate(selector, variables);
                let resolved = onecrawl_cdp::accessibility::resolve_ref(&sel);
                onecrawl_cdp::element::click(page, &resolved).await.mcp()?;
                Ok(Some(serde_json::json!({ "clicked": sel })))
            }
            Action::Type { selector, text } => {
                let sel = onecrawl_cdp::workflow::interpolate(selector, variables);
                let txt = onecrawl_cdp::workflow::interpolate(text, variables);
                let resolved = onecrawl_cdp::accessibility::resolve_ref(&sel);
                onecrawl_cdp::element::type_text(page, &resolved, &txt).await.mcp()?;
                Ok(Some(serde_json::json!({ "typed": txt.len() })))
            }
            Action::WaitForSelector { selector, timeout_ms } => {
                let sel = onecrawl_cdp::workflow::interpolate(selector, variables);
                let resolved = onecrawl_cdp::accessibility::resolve_ref(&sel);
                onecrawl_cdp::navigation::wait_for_selector(page, &resolved, *timeout_ms).await.mcp()?;
                Ok(Some(serde_json::json!({ "found": sel })))
            }
            Action::Screenshot { path, full_page } => {
                let bytes = if full_page.unwrap_or(false) {
                    onecrawl_cdp::screenshot::screenshot_full(page)
                        .await.mcp()?
                } else {
                    onecrawl_cdp::screenshot::screenshot_viewport(page)
                        .await.mcp()?
                };
                if let Some(p) = path {
                    let p = onecrawl_cdp::workflow::interpolate(p, variables);
                    std::fs::write(&p, &bytes).mcp()?;
                    Ok(Some(serde_json::json!({ "saved": p, "bytes": bytes.len() })))
                } else {
                    Ok(Some(serde_json::json!({ "bytes": bytes.len() })))
                }
            }
            Action::Evaluate { js } => {
                let js = onecrawl_cdp::workflow::interpolate(js, variables);
                let result = page.evaluate(js).await.mcp()?;
                let val = result.into_value::<serde_json::Value>().unwrap_or(serde_json::Value::Null);
                Ok(Some(val))
            }
            Action::Extract { selector, attribute } => {
                let sel = onecrawl_cdp::workflow::interpolate(selector, variables);
                let attr_js = if let Some(attr) = attribute {
                    format!(r#"Array.from(document.querySelectorAll({sel_json})).map(e => e.getAttribute({attr_json}))"#,
                        sel_json = json_escape(&sel),
                        attr_json = json_escape(attr))
                } else {
                    format!(r#"Array.from(document.querySelectorAll({sel_json})).map(e => e.textContent.trim())"#,
                        sel_json = json_escape(&sel))
                };
                let result = page.evaluate(attr_js).await.mcp()?;
                let val = result.into_value::<serde_json::Value>().unwrap_or(serde_json::Value::Null);
                Ok(Some(val))
            }
            Action::SmartClick { query } => {
                let q = onecrawl_cdp::workflow::interpolate(query, variables);
                let matched = onecrawl_cdp::smart_actions::smart_click(page, &q).await.mcp()?;
                Ok(Some(serde_json::json!({ "clicked": matched.selector, "confidence": matched.confidence })))
            }
            Action::SmartFill { query, value } => {
                let q = onecrawl_cdp::workflow::interpolate(query, variables);
                let v = onecrawl_cdp::workflow::interpolate(value, variables);
                let matched = onecrawl_cdp::smart_actions::smart_fill(page, &q, &v).await.mcp()?;
                Ok(Some(serde_json::json!({ "filled": matched.selector, "confidence": matched.confidence })))
            }
            Action::Sleep { ms } => {
                tokio::time::sleep(tokio::time::Duration::from_millis(*ms)).await;
                Ok(Some(serde_json::json!({ "slept_ms": ms })))
            }
            Action::SetVariable { name, value } => {
                let interpolated = onecrawl_cdp::workflow::interpolate(&value.to_string(), variables);
                let parsed = serde_json::from_str::<serde_json::Value>(&interpolated)
                    .unwrap_or(serde_json::Value::String(interpolated));
                variables.insert(name.clone(), parsed.clone());
                Ok(Some(serde_json::json!({ "set": name, "value": parsed })))
            }
            Action::Log { message, level } => {
                let msg = onecrawl_cdp::workflow::interpolate(message, variables);
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
                let cond = onecrawl_cdp::workflow::interpolate(condition, variables);
                if onecrawl_cdp::workflow::evaluate_condition(&cond, variables) {
                    Ok(Some(serde_json::json!({ "assert": "passed" })))
                } else {
                    Err(mcp_err(format!("assertion failed: {}", message.as_deref().unwrap_or(&cond))))
                }
            }
            Action::Loop { items: _, variable: _, steps: _ } => {
                Ok(Some(serde_json::json!({ "note": "loop execution requires recursive step runner — use workflow.run for full support" })))
            }
            Action::Conditional { condition, then_steps, else_steps } => {
                let cond = onecrawl_cdp::workflow::interpolate(condition, variables);
                let empty = vec![];
                let branch = if onecrawl_cdp::workflow::evaluate_condition(&cond, variables) {
                    then_steps
                } else {
                    else_steps.as_ref().unwrap_or(&empty)
                };
                let mut last_output = None;
                for step in branch {
                    last_output = self.execute_step(page, &step.action, variables).await?;
                }
                Ok(last_output)
            }
            Action::SubWorkflow { path } => {
                let p = onecrawl_cdp::workflow::interpolate(path, variables);
                Ok(Some(serde_json::json!({ "note": format!("sub-workflow '{}' — use workflow.run to execute", p) })))
            }
            Action::HttpRequest { url, method, headers, body } => {
                let url = onecrawl_cdp::workflow::interpolate(url, variables);
                let method = method.as_deref().unwrap_or("GET");
                let client = reqwest::Client::new();
                let mut req = match method.to_uppercase().as_str() {
                    "POST" => client.post(&url),
                    "PUT" => client.put(&url),
                    "DELETE" => client.delete(&url),
                    "PATCH" => client.patch(&url),
                    _ => client.get(&url),
                };
                if let Some(hdrs) = headers {
                    for (k, v) in hdrs {
                        let v = onecrawl_cdp::workflow::interpolate(v, variables);
                        req = req.header(k.as_str(), v);
                    }
                }
                if let Some(b) = body {
                    let b = onecrawl_cdp::workflow::interpolate(b, variables);
                    req = req.body(b);
                }
                let resp = req.send().await.mcp()?;
                let status = resp.status().as_u16();
                let body_text = resp.text().await.unwrap_or_default();
                let body_val = serde_json::from_str::<serde_json::Value>(&body_text)
                    .unwrap_or(serde_json::Value::String(body_text));
                Ok(Some(serde_json::json!({ "status": status, "body": body_val })))
            }
            Action::Snapshot { compact, interactive_only } => {
                let opts = onecrawl_cdp::accessibility::AgentSnapshotOptions {
                    interactive_only: *interactive_only,
                    compact: *compact,
                    ..Default::default()
                };
                let result = onecrawl_cdp::accessibility::agent_snapshot(page, &opts)
                    .await.mcp()?;
                Ok(Some(serde_json::json!(result)))
            }
            Action::Agent { prompt, options } => {
                let url = onecrawl_cdp::navigation::get_url(page).await.unwrap_or_default();
                let context = onecrawl_cdp::AgentStepContext {
                    step_index: 0,
                    prompt: prompt.clone(),
                    options: options.clone().unwrap_or_default(),
                    url,
                    variables: variables.iter().map(|(k,v)| (k.clone(), v.clone())).collect(),
                };
                Ok(Some(serde_json::to_value(&context).unwrap_or(serde_json::Value::Null)))
            }
        }
        })
    }

    // ════════════════════════════════════════════════════════════════
    //  Standalone Workflow Execution Engine
    // ════════════════════════════════════════════════════════════════

    pub(crate) async fn workflow_execute(
        &self,
        p: WorkflowExecuteParams,
    ) -> Result<CallToolResult, McpError> {
        let mut workflow = if p.workflow.trim().starts_with('{') {
            onecrawl_cdp::workflow::parse_json(&p.workflow).mcp()?
        } else {
            onecrawl_cdp::workflow::load_from_file(&p.workflow).mcp()?
        };
        if let Some(overrides) = p.variables {
            for (k, v) in overrides {
                workflow.variables.insert(k, v);
            }
        }
        let errors = onecrawl_cdp::workflow::validate(&workflow);
        if !errors.is_empty() {
            return json_ok(&serde_json::json!({
                "status": "validation_failed",
                "errors": errors,
            }));
        }
        let page = ensure_page(&self.browser).await?;
        let result = onecrawl_cdp::workflow::execute_workflow(&page, &workflow).await.mcp()?;
        json_ok(&serde_json::to_value(&result).mcp()?)
    }

    pub(crate) async fn workflow_status(
        &self,
        _p: WorkflowStatusParams,
    ) -> Result<CallToolResult, McpError> {
        json_ok(&serde_json::json!({
            "engine": "workflow_executor",
            "version": "1.0",
            "supported_actions": [
                "navigate", "click", "type", "wait_for_selector", "screenshot",
                "evaluate", "extract", "smart_click", "smart_fill", "sleep",
                "set_variable", "log", "assert", "loop", "conditional",
                "sub_workflow", "http_request", "snapshot"
            ],
        }))
    }

    // ════════════════════════════════════════════════════════════════
    //  Network Intelligence tools
    // ════════════════════════════════════════════════════════════════


    // ════════════════════════════════════════════════════════════════
    //  AI Task Planner tools
    // ════════════════════════════════════════════════════════════════

    pub(crate) async fn planner_plan(
        &self,
        p: PlannerPlanParams,
    ) -> Result<CallToolResult, McpError> {
        let mut context = p.context.unwrap_or_default();
        let auto_context = onecrawl_cdp::task_planner::extract_context(&p.goal);
        for (k, v) in auto_context {
            context.entry(k).or_insert(v);
        }

        let plan = onecrawl_cdp::task_planner::plan_from_goal(&p.goal, &context);
        json_ok(&serde_json::to_value(&plan).mcp()?)
    }


    pub(crate) async fn planner_execute(
        &self,
        p: PlannerExecuteParams,
    ) -> Result<CallToolResult, McpError> {
        let plan: onecrawl_cdp::TaskPlan = if p.plan.trim().starts_with('{') {
            parse_json_str(&p.plan, "plan")?
        } else {
            let mut context = p.context.clone().unwrap_or_default();
            let auto_context = onecrawl_cdp::task_planner::extract_context(&p.plan);
            for (k, v) in auto_context {
                context.entry(k).or_insert(v);
            }
            onecrawl_cdp::task_planner::plan_from_goal(&p.plan, &context)
        };

        let page = ensure_page(&self.browser).await?;
        let start = std::time::Instant::now();
        let max_retries = p.max_retries.unwrap_or(2);
        let mut step_results = Vec::new();
        let mut total_retries = 0usize;
        let mut completed = 0usize;

        for step in &plan.steps {
            let step_start = std::time::Instant::now();
            let mut attempt = 0u32;
            let mut last_error = None;
            let mut success = false;
            let mut used_fallback = false;
            let mut output = None;

            while attempt <= max_retries {
                match self.execute_planned_step(&page, &step.action).await {
                    Ok(val) => {
                        output = val;
                        success = true;
                        break;
                    }
                    Err(e) => {
                        last_error = Some(format!("{}", e.message));
                        attempt += 1;
                        total_retries += 1;

                        if attempt > max_retries {
                            if let Some(ref fallback) = step.fallback {
                                if let Ok(val) = self.execute_planned_step(&page, &fallback.action).await {
                                    output = val;
                                    success = true;
                                    used_fallback = true;
                                }
                            }
                        }
                    }
                }
            }

            let duration_ms = step_start.elapsed().as_millis() as u64;
            if success { completed += 1; }

            step_results.push(onecrawl_cdp::task_planner::StepExecutionResult {
                step_id: step.id,
                description: step.description.clone(),
                status: if success {
                    onecrawl_cdp::task_planner::StepOutcome::Success
                } else {
                    onecrawl_cdp::task_planner::StepOutcome::Failed
                },
                output,
                error: if success { None } else { last_error },
                used_fallback,
                duration_ms,
            });
        }

        let status = if completed == plan.steps.len() {
            onecrawl_cdp::TaskStatus::Success
        } else if completed > 0 {
            onecrawl_cdp::TaskStatus::PartialSuccess
        } else {
            onecrawl_cdp::TaskStatus::Failed
        };

        let result = onecrawl_cdp::TaskExecutionResult {
            goal: plan.goal.clone(),
            status,
            steps_completed: completed,
            steps_total: plan.steps.len(),
            steps_results: step_results,
            retries_used: total_retries,
            total_duration_ms: start.elapsed().as_millis() as u64,
        };

        json_ok(&serde_json::to_value(&result).mcp()?)
    }


    pub(crate) async fn planner_patterns(
        &self,
        _p: PlannerPatternsParams,
    ) -> Result<CallToolResult, McpError> {
        let patterns = onecrawl_cdp::task_planner::builtin_patterns();
        let summary: Vec<serde_json::Value> = patterns.iter().map(|p| {
            serde_json::json!({
                "category": format!("{:?}", p.category).to_lowercase(),
                "keywords": p.keywords,
                "steps": p.template_steps.len(),
                "template": p.template_steps.iter().map(|s| &s.description).collect::<Vec<_>>(),
            })
        }).collect();
        json_ok(&serde_json::json!({
            "patterns": summary,
            "count": patterns.len(),
        }))
    }

    // ════════════════════════════════════════════════════════════════
    //  Performance Monitor tools
    // ════════════════════════════════════════════════════════════════


    pub(crate) async fn execute_planned_step(
        &self,
        page: &chromiumoxide::Page,
        action: &onecrawl_cdp::task_planner::PlannedAction,
    ) -> std::result::Result<Option<serde_json::Value>, McpError> {
        use onecrawl_cdp::task_planner::PlannedAction;
        match action {
            PlannedAction::Navigate { url } => {
                onecrawl_cdp::navigation::goto(page, url).await.mcp()?;
                let title = onecrawl_cdp::navigation::get_title(page).await.unwrap_or_default();
                Ok(Some(serde_json::json!({ "navigated": url, "title": title })))
            }
            PlannedAction::Click { target, .. } => {
                let resolved = onecrawl_cdp::accessibility::resolve_ref(target);
                onecrawl_cdp::element::click(page, &resolved).await.mcp()?;
                Ok(Some(serde_json::json!({ "clicked": target })))
            }
            PlannedAction::Type { target, text, .. } => {
                let resolved = onecrawl_cdp::accessibility::resolve_ref(target);
                onecrawl_cdp::element::type_text(page, &resolved, text).await.mcp()?;
                Ok(Some(serde_json::json!({ "typed": text.len() })))
            }
            PlannedAction::Wait { target, timeout_ms } => {
                let resolved = onecrawl_cdp::accessibility::resolve_ref(target);
                onecrawl_cdp::navigation::wait_for_selector(page, &resolved, *timeout_ms).await.mcp()?;
                Ok(Some(serde_json::json!({ "found": target })))
            }
            PlannedAction::Snapshot {} => {
                let opts = onecrawl_cdp::accessibility::AgentSnapshotOptions::default();
                let result = onecrawl_cdp::accessibility::agent_snapshot(page, &opts)
                    .await.mcp()?;
                Ok(Some(serde_json::json!(result)))
            }
            PlannedAction::Extract { target } => {
                let js = format!(
                    r#"Array.from(document.querySelectorAll({sel})).map(e => e.textContent.trim())"#,
                    sel = json_escape(target)
                );
                let result = page.evaluate(js).await.mcp()?;
                let val = result.into_value::<serde_json::Value>().unwrap_or(serde_json::Value::Null);
                Ok(Some(val))
            }
            PlannedAction::Assert { condition } => {
                Ok(Some(serde_json::json!({ "assert": condition, "note": "assertion evaluation requires runtime context" })))
            }
            PlannedAction::SmartClick { query } => {
                let matched = onecrawl_cdp::smart_actions::smart_click(page, query).await.mcp()?;
                Ok(Some(serde_json::json!({ "clicked": matched.selector, "confidence": matched.confidence })))
            }
            PlannedAction::SmartFill { query, value } => {
                let matched = onecrawl_cdp::smart_actions::smart_fill(page, query, value).await.mcp()?;
                Ok(Some(serde_json::json!({ "filled": matched.selector, "confidence": matched.confidence })))
            }
            PlannedAction::Scroll { direction, amount } => {
                let px = amount.unwrap_or(500);
                let js = match direction.as_str() {
                    "up" => format!("window.scrollBy(0, -{})", px),
                    "down" => format!("window.scrollBy(0, {})", px),
                    "left" => format!("window.scrollBy(-{}, 0)", px),
                    "right" => format!("window.scrollBy({}, 0)", px),
                    _ => format!("window.scrollBy(0, {})", px),
                };
                page.evaluate(js).await.mcp()?;
                Ok(Some(serde_json::json!({ "scrolled": direction, "pixels": px })))
            }
            PlannedAction::Screenshot { path } => {
                let data = onecrawl_cdp::screenshot::screenshot_full(page).await.mcp()?;
                if let Some(p) = path {
                    std::fs::write(p, &data).mcp()?;
                }
                Ok(Some(serde_json::json!({ "bytes": data.len() })))
            }
            PlannedAction::MemoryStore { key, value } => {
                Ok(Some(serde_json::json!({ "stored": key, "value": value })))
            }
            PlannedAction::MemoryRecall { key } => {
                Ok(Some(serde_json::json!({ "recalled": key })))
            }
            PlannedAction::Conditional { condition, .. } => {
                Ok(Some(serde_json::json!({ "note": "conditional evaluation", "condition": condition })))
            }
        }
    }

    // ════════════════════════════════════════════════════════════════
    //  Intelligent Error Recovery
    // ════════════════════════════════════════════════════════════════

    pub(crate) async fn retry_adapt(
        &self,
        p: crate::cdp_tools::RetryAdaptParams,
    ) -> Result<CallToolResult, McpError> {
        let max_retries = p.max_retries.unwrap_or(3);
        let strategy = p.strategy.as_deref().unwrap_or("exponential");
        let on_error = p.on_error.as_deref().unwrap_or("retry");

        let delays: Vec<u64> = match strategy {
            "linear" => (1..=max_retries).map(|i| i as u64 * 1000).collect(),
            "immediate" => vec![0; max_retries as usize],
            _ => (0..max_retries).map(|i| 2u64.pow(i) * 1000).collect(),
        };

        json_ok(&serde_json::json!({
            "action": p.action,
            "params": p.params,
            "strategy": {
                "type": strategy,
                "max_retries": max_retries,
                "delays_ms": delays,
                "on_error": on_error,
            },
            "alternative": if on_error == "alternative" {
                serde_json::json!({
                    "action": p.alternative_action,
                    "params": p.alternative_params,
                })
            } else {
                serde_json::Value::Null
            },
            "instructions": format!(
                "Execute '{}' with {} strategy (max {} retries). On error: {}.",
                p.action, strategy, max_retries, on_error
            )
        }))
    }

    pub(crate) async fn error_classify(
        &self,
        p: crate::cdp_tools::ErrorClassifyParams,
    ) -> Result<CallToolResult, McpError> {
        let msg = p.error_message.to_lowercase();

        let (category, severity, retryable) = if msg.contains("not found") || msg.contains("no such element")
            || msg.contains("queryselector") || msg.contains("null reference")
        {
            ("selector_not_found", "medium", true)
        } else if msg.contains("timeout") || msg.contains("timed out")
            || msg.contains("deadline exceeded")
        {
            ("timeout", "medium", true)
        } else if msg.contains("net::err") || msg.contains("network")
            || msg.contains("fetch failed") || msg.contains("econnrefused")
            || msg.contains("dns")
        {
            ("network", "high", true)
        } else if msg.contains("navigation") || msg.contains("navigat")
            || msg.contains("net::err_aborted")
        {
            ("navigation", "medium", true)
        } else if msg.contains("permission") || msg.contains("denied")
            || msg.contains("forbidden") || msg.contains("403")
        {
            ("permission", "high", false)
        } else if msg.contains("crash") || msg.contains("oom")
            || msg.contains("out of memory")
        {
            ("crash", "critical", false)
        } else if msg.contains("syntax") || msg.contains("parse")
            || msg.contains("unexpected token")
        {
            ("syntax", "low", false)
        } else if msg.contains("stale") || msg.contains("detached") {
            ("stale_element", "medium", true)
        } else {
            ("unknown", "medium", false)
        };

        // Record in error history
        {
            let mut state = self.browser.lock().await;
            let ts = {
                let dur = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap_or_default();
                format!("{}s", dur.as_secs())
            };
            state.error_history.push((ts, category.to_string(), p.error_message.clone()));
            if state.error_history.len() > 100 {
                state.error_history.remove(0);
            }
        }

        json_ok(&serde_json::json!({
            "category": category,
            "severity": severity,
            "retryable": retryable,
            "original_message": p.error_message,
        }))
    }

    pub(crate) async fn recovery_suggest(
        &self,
        p: crate::cdp_tools::RecoveryStrategyParams,
    ) -> Result<CallToolResult, McpError> {
        let steps: Vec<serde_json::Value> = match p.error_type.as_str() {
            "selector_not_found" => vec![
                serde_json::json!({"step": 1, "action": "snapshot", "description": "Take accessibility snapshot to find correct selectors"}),
                serde_json::json!({"step": 2, "action": "wait", "description": "Wait for element with increased timeout (5000ms)"}),
                serde_json::json!({"step": 3, "action": "evaluate", "description": "Use document.querySelectorAll to check if element exists in DOM"}),
                serde_json::json!({"step": 4, "action": "css", "description": "Try broader CSS selector or use text-based matching"}),
            ],
            "timeout" => vec![
                serde_json::json!({"step": 1, "action": "wait", "description": "Increase timeout to 30000ms and retry"}),
                serde_json::json!({"step": 2, "action": "reload", "description": "Reload page and retry the action"}),
                serde_json::json!({"step": 3, "action": "emulate_network", "description": "Check if network throttling is active"}),
                serde_json::json!({"step": 4, "action": "errors_get", "description": "Check for JavaScript errors blocking execution"}),
            ],
            "navigation" => vec![
                serde_json::json!({"step": 1, "action": "goto", "description": "Retry navigation with full URL"}),
                serde_json::json!({"step": 2, "action": "evaluate", "description": "Check window.location to verify current page"}),
                serde_json::json!({"step": 3, "action": "back", "description": "Go back and retry navigation"}),
                serde_json::json!({"step": 4, "action": "cookies_clear", "description": "Clear cookies and retry (auth redirect issue)"}),
            ],
            "network" => vec![
                serde_json::json!({"step": 1, "action": "wait", "description": "Wait 5 seconds for network recovery"}),
                serde_json::json!({"step": 2, "action": "reload", "description": "Reload page to retry network requests"}),
                serde_json::json!({"step": 3, "action": "intercept_list", "description": "Check if interception rules are blocking requests"}),
                serde_json::json!({"step": 4, "action": "goto", "description": "Navigate to a known-good URL to test connectivity"}),
            ],
            "permission" => vec![
                serde_json::json!({"step": 1, "action": "cookies_get", "description": "Check authentication cookies"}),
                serde_json::json!({"step": 2, "action": "storage_get", "description": "Check stored auth tokens"}),
                serde_json::json!({"step": 3, "action": "goto", "description": "Navigate to login page"}),
            ],
            "stale_element" => vec![
                serde_json::json!({"step": 1, "action": "wait", "description": "Wait for DOM to stabilize after navigation/mutation"}),
                serde_json::json!({"step": 2, "action": "snapshot", "description": "Take fresh snapshot to get updated selectors"}),
                serde_json::json!({"step": 3, "action": "css", "description": "Re-query the element with same selector"}),
            ],
            _ => vec![
                serde_json::json!({"step": 1, "action": "screenshot", "description": "Take screenshot to assess current state"}),
                serde_json::json!({"step": 2, "action": "errors_get", "description": "Check for page errors"}),
                serde_json::json!({"step": 3, "action": "console_get", "description": "Check console messages for clues"}),
            ],
        };

        json_ok(&serde_json::json!({
            "error_type": p.error_type,
            "recovery_steps": steps,
            "context": p.context,
        }))
    }

    pub(crate) async fn error_history(
        &self,
        _v: serde_json::Value,
    ) -> Result<CallToolResult, McpError> {
        let state = self.browser.lock().await;
        let entries: Vec<serde_json::Value> = state
            .error_history
            .iter()
            .map(|(ts, cat, msg)| {
                serde_json::json!({
                    "timestamp": ts,
                    "category": cat,
                    "message": msg,
                })
            })
            .collect();
        json_ok(&serde_json::json!({
            "errors": entries,
            "total": entries.len(),
        }))
    }

    // ════════════════════════════════════════════════════════════════
    //  Session Checkpoints / Resume
    // ════════════════════════════════════════════════════════════════

    pub(crate) async fn checkpoint_save(
        &self,
        p: CheckpointSaveParams,
    ) -> Result<CallToolResult, McpError> {
        let page = ensure_page(&self.browser).await?;
        let include_cookies = p.include_cookies.unwrap_or(true);
        let include_storage = p.include_storage.unwrap_or(true);
        let include_context = p.include_context.unwrap_or(true);

        // Capture URL
        let url_js = "location.href";
        let url_result = page.evaluate(url_js).await.mcp()?;
        let url = url_result.into_value::<String>().unwrap_or_else(|_| "about:blank".into());

        // Capture cookies via JS
        let cookies = if include_cookies {
            let cookie_result = page.evaluate("document.cookie").await.mcp()?;
            cookie_result.into_value::<String>().ok()
        } else {
            None
        };

        // Capture storage via JS
        let storage = if include_storage {
            let storage_js = r#"(() => {
                const ls = {};
                for (let i = 0; i < localStorage.length; i++) {
                    const k = localStorage.key(i);
                    ls[k] = localStorage.getItem(k);
                }
                const ss = {};
                for (let i = 0; i < sessionStorage.length; i++) {
                    const k = sessionStorage.key(i);
                    ss[k] = sessionStorage.getItem(k);
                }
                return JSON.stringify({ localStorage: ls, sessionStorage: ss });
            })()"#;
            let result = page.evaluate(storage_js).await.mcp()?;
            let raw = result.into_value::<String>().ok();
            raw.and_then(|r| serde_json::from_str(&r).ok())
        } else {
            None::<serde_json::Value>
        };

        // Capture page context
        let context = if include_context {
            let state = self.browser.lock().await;
            Some(serde_json::to_value(&state.page_context).unwrap_or_default())
        } else {
            None
        };

        let now = {
            let d = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap_or_default();
            format!("{}s", d.as_secs())
        };
        let checkpoint = serde_json::json!({
            "url": url,
            "cookies": cookies,
            "storage": storage,
            "context": context,
            "saved_at": now,
        });

        let size_bytes = serde_json::to_string(&checkpoint).unwrap_or_default().len();

        let mut state = self.browser.lock().await;
        state.checkpoints.insert(p.name.clone(), checkpoint);

        json_ok(&serde_json::json!({
            "name": p.name,
            "saved_at": now,
            "url": url,
            "has_cookies": cookies.is_some(),
            "has_storage": storage.is_some(),
            "has_context": context.is_some(),
            "size_bytes": size_bytes
        }))
    }

    pub(crate) async fn checkpoint_restore(
        &self,
        p: CheckpointRestoreParams,
    ) -> Result<CallToolResult, McpError> {
        let page = ensure_page(&self.browser).await?;
        let restore_url = p.restore_url.unwrap_or(true);
        let restore_cookies = p.restore_cookies.unwrap_or(true);

        let checkpoint = {
            let state = self.browser.lock().await;
            state.checkpoints.get(&p.name).cloned()
                .ok_or_else(|| mcp_err(format!("checkpoint '{}' not found", p.name)))?
        };

        let url = checkpoint.get("url").and_then(|u| u.as_str()).unwrap_or("about:blank");
        let mut url_restored = false;
        let mut cookies_restored = false;
        let mut storage_restored = false;
        let mut context_restored = false;

        // Restore URL
        if restore_url {
            let nav_js = format!("location.href = {}", json_escape(url));
            let _ = page.evaluate(nav_js).await;
            url_restored = true;
            // Small wait for navigation
            tokio::time::sleep(std::time::Duration::from_millis(500)).await;
        }

        // Restore cookies
        if restore_cookies {
            if let Some(cookie_str) = checkpoint.get("cookies").and_then(|c| c.as_str()) {
                if !cookie_str.is_empty() {
                    let cookie_js = format!("document.cookie = {}", json_escape(cookie_str));
                    let _ = page.evaluate(cookie_js).await;
                    cookies_restored = true;
                }
            }
        }

        // Restore storage
        if let Some(storage) = checkpoint.get("storage") {
            let storage_str = serde_json::to_string(storage).unwrap_or_else(|_| "{}".into());
            let restore_js = format!(r#"(() => {{
                const data = JSON.parse({storage_json});
                if (data.localStorage) {{
                    Object.entries(data.localStorage).forEach(([k,v]) => localStorage.setItem(k,v));
                }}
                if (data.sessionStorage) {{
                    Object.entries(data.sessionStorage).forEach(([k,v]) => sessionStorage.setItem(k,v));
                }}
                return 'restored';
            }})()"#, storage_json = json_escape(&storage_str));
            let _ = page.evaluate(restore_js).await;
            storage_restored = true;
        }

        // Restore context
        if let Some(context) = checkpoint.get("context") {
            if let Some(ctx_obj) = context.as_object() {
                let mut state = self.browser.lock().await;
                for (k, v) in ctx_obj {
                    state.page_context.insert(k.clone(), v.clone());
                }
                context_restored = true;
            }
        }

        let now = {
            let d = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap_or_default();
            format!("{}s", d.as_secs())
        };
        json_ok(&serde_json::json!({
            "name": p.name,
            "restored_at": now,
            "url": url,
            "url_restored": url_restored,
            "cookies_restored": cookies_restored,
            "storage_restored": storage_restored,
            "context_restored": context_restored
        }))
    }

    pub(crate) async fn checkpoint_list(
        &self,
        _v: serde_json::Value,
    ) -> Result<CallToolResult, McpError> {
        let state = self.browser.lock().await;
        let checkpoints: Vec<serde_json::Value> = state.checkpoints.iter().map(|(name, data)| {
            let url = data.get("url").and_then(|u| u.as_str()).unwrap_or("");
            let saved_at = data.get("saved_at").and_then(|s| s.as_str()).unwrap_or("");
            let size = serde_json::to_string(data).unwrap_or_default().len();
            serde_json::json!({
                "name": name,
                "saved_at": saved_at,
                "url": url,
                "size_bytes": size
            })
        }).collect();
        let count = checkpoints.len();
        json_ok(&serde_json::json!({
            "checkpoints": checkpoints,
            "count": count
        }))
    }

    pub(crate) async fn checkpoint_delete(
        &self,
        p: CheckpointDeleteParams,
    ) -> Result<CallToolResult, McpError> {
        let mut state = self.browser.lock().await;
        let existed = state.checkpoints.remove(&p.name).is_some();
        json_ok(&serde_json::json!({
            "name": p.name,
            "deleted": existed
        }))
    }

    // ════════════════════════════════════════════════════════════════
    //  Extended Workflow DSL
    // ════════════════════════════════════════════════════════════════

    pub(crate) async fn workflow_while(
        &self,
        p: WorkflowWhileParams,
    ) -> Result<CallToolResult, McpError> {
        let page = ensure_page(&self.browser).await?;
        let max_iter = p.max_iterations.unwrap_or(100) as usize;
        let mut iterations = 0usize;
        let mut all_results: Vec<serde_json::Value> = Vec::new();

        loop {
            if iterations >= max_iter { break; }
            // Evaluate condition
            let cond_result = page.evaluate(p.condition.as_str()).await.mcp()?;
            let cond_val = cond_result.into_value::<serde_json::Value>()
                .unwrap_or(serde_json::Value::Bool(false));
            let is_truthy = match &cond_val {
                serde_json::Value::Bool(b) => *b,
                serde_json::Value::Null => false,
                serde_json::Value::Number(n) => n.as_f64().map(|f| f != 0.0).unwrap_or(false),
                serde_json::Value::String(s) => !s.is_empty(),
                _ => true,
            };
            if !is_truthy { break; }

            let mut iter_results = Vec::new();
            for action_val in &p.actions {
                let cmd: ChainCommand = serde_json::from_value(action_val.clone())
                    .map_err(|e| mcp_err(format!("invalid action in workflow_while: {e}")))?;
                match self.dispatch_chain_command(&cmd).await {
                    Ok(r) => iter_results.push(r),
                    Err(e) => iter_results.push(serde_json::json!({"error": e})),
                }
            }
            all_results.push(serde_json::json!(iter_results));
            iterations += 1;
        }

        json_ok(&serde_json::json!({
            "iterations_executed": iterations,
            "results_per_iteration": all_results,
            "max_iterations": max_iter
        }))
    }

    pub(crate) async fn workflow_for_each(
        &self,
        p: WorkflowForEachParams,
    ) -> Result<CallToolResult, McpError> {
        let page = ensure_page(&self.browser).await?;
        let var_name = p.variable_name.as_deref().unwrap_or("item");

        // Try to parse collection as JSON first, else evaluate as JS
        let items: Vec<serde_json::Value> = if let Ok(arr) = serde_json::from_str::<Vec<serde_json::Value>>(&p.collection) {
            arr
        } else {
            let js_result = page.evaluate(p.collection.as_str()).await.mcp()?;
            let raw = js_result.into_value::<serde_json::Value>().unwrap_or_default();
            match raw {
                serde_json::Value::Array(arr) => arr,
                other => vec![other],
            }
        };

        let mut results: Vec<serde_json::Value> = Vec::new();
        for item in &items {
            // Set workflow variable for current item
            {
                let mut state = self.browser.lock().await;
                state.workflow_variables.insert(var_name.to_string(), item.clone());
            }

            let mut iter_results = Vec::new();
            for action_val in &p.actions {
                let cmd: ChainCommand = serde_json::from_value(action_val.clone())
                    .map_err(|e| mcp_err(format!("invalid action in workflow_for_each: {e}")))?;
                match self.dispatch_chain_command(&cmd).await {
                    Ok(r) => iter_results.push(r),
                    Err(e) => iter_results.push(serde_json::json!({"error": e})),
                }
            }
            results.push(serde_json::json!(iter_results));
        }

        json_ok(&serde_json::json!({
            "items_processed": items.len(),
            "results": results
        }))
    }

    pub(crate) async fn workflow_if(
        &self,
        p: WorkflowIfParams,
    ) -> Result<CallToolResult, McpError> {
        let page = ensure_page(&self.browser).await?;

        let cond_result = page.evaluate(p.condition.as_str()).await.mcp()?;
        let cond_val = cond_result.into_value::<serde_json::Value>()
            .unwrap_or(serde_json::Value::Bool(false));
        let is_truthy = match &cond_val {
            serde_json::Value::Bool(b) => *b,
            serde_json::Value::Null => false,
            serde_json::Value::Number(n) => n.as_f64().map(|f| f != 0.0).unwrap_or(false),
            serde_json::Value::String(s) => !s.is_empty(),
            _ => true,
        };

        let actions = if is_truthy {
            &p.then_actions
        } else {
            match &p.else_actions {
                Some(acts) => acts,
                None => return json_ok(&serde_json::json!({
                    "condition_value": cond_val,
                    "branch_taken": "else",
                    "results": []
                })),
            }
        };

        let branch = if is_truthy { "then" } else { "else" };
        let mut results = Vec::new();
        for action_val in actions {
            let cmd: ChainCommand = serde_json::from_value(action_val.clone())
                .map_err(|e| mcp_err(format!("invalid action in workflow_if: {e}")))?;
            match self.dispatch_chain_command(&cmd).await {
                Ok(r) => results.push(r),
                Err(e) => results.push(serde_json::json!({"error": e})),
            }
        }

        json_ok(&serde_json::json!({
            "condition_value": cond_val,
            "branch_taken": branch,
            "results": results
        }))
    }

    pub(crate) async fn workflow_variable(
        &self,
        p: WorkflowVariableParams,
    ) -> Result<CallToolResult, McpError> {
        let mut state = self.browser.lock().await;
        if let Some(value) = p.value {
            state.workflow_variables.insert(p.name.clone(), value.clone());
            json_ok(&serde_json::json!({
                "name": p.name,
                "value": value,
                "action": "set"
            }))
        } else {
            let value = state.workflow_variables.get(&p.name).cloned()
                .unwrap_or(serde_json::Value::Null);
            json_ok(&serde_json::json!({
                "name": p.name,
                "value": value,
                "action": "get"
            }))
        }
    }

    // ════════════════════════════════════════════════════════════════
    //  Long-running harness
    // ════════════════════════════════════════════════════════════════

    pub(crate) async fn reconnect_cdp(
        &self,
        p: ReconnectCdpParams,
    ) -> Result<CallToolResult, McpError> {
        let page = ensure_page(&self.browser).await?;
        let max_retries = p.max_retries.unwrap_or(5);
        let result = onecrawl_cdp::harness::reconnect_cdp(&page, max_retries).await.mcp()?;
        json_ok(&result)
    }

    pub(crate) async fn gc_tabs(
        &self,
        _p: GcTabsParams,
    ) -> Result<CallToolResult, McpError> {
        let page = ensure_page(&self.browser).await?;
        let result = onecrawl_cdp::harness::gc_tabs_info(&page).await.mcp()?;
        json_ok(&result)
    }

    pub(crate) async fn watchdog(
        &self,
        _p: WatchdogParams,
    ) -> Result<CallToolResult, McpError> {
        let page = ensure_page(&self.browser).await?;
        let result = onecrawl_cdp::harness::watchdog_status(&page).await.mcp()?;
        json_ok(&result)
    }

    pub(crate) async fn batch_execute(
        &self,
        p: BatchExecuteParams,
    ) -> Result<CallToolResult, McpError> {
        let page = ensure_page(&self.browser).await?;
        let mut results = Vec::new();
        for (i, cmd) in p.commands.iter().enumerate() {
            let result = page.evaluate(cmd.clone()).await;
            match result {
                Ok(val) => {
                    let v = val
                        .into_value::<serde_json::Value>()
                        .unwrap_or(serde_json::Value::Null);
                    results.push(serde_json::json!({"index": i, "status": "ok", "result": v}));
                }
                Err(e) => {
                    results.push(
                        serde_json::json!({"index": i, "status": "error", "error": e.to_string()}),
                    );
                    if p.stop_on_error.unwrap_or(false) {
                        break;
                    }
                }
            }
        }
        let executed = results.len();
        json_ok(&serde_json::json!({"results": results, "total": p.commands.len(), "executed": executed}))
    }

    // ════════════════════════════════════════════════════════════════
    //  Agent-in-the-Loop: workflow resume & agent decide
    // ════════════════════════════════════════════════════════════════

    pub(crate) async fn workflow_resume(
        &self,
        p: WorkflowResumeParams,
    ) -> Result<CallToolResult, McpError> {
        let page = ensure_page(&self.browser).await?;

        let data = std::fs::read_to_string(&p.file)
            .map_err(|e| mcp_err(format!("failed to read workflow file: {e}")))?;
        let workflow = onecrawl_cdp::workflow::parse_json(&data)
            .map_err(|e| mcp_err(format!("workflow parse error: {e}")))?;

        if p.resume_from >= workflow.steps.len() {
            return Err(mcp_err(format!(
                "resume_from index {} is out of range (workflow has {} steps)",
                p.resume_from, workflow.steps.len()
            )));
        }

        let mut variables = workflow.variables.clone();
        if let Some(ref extra) = p.variables {
            for (k, v) in extra {
                variables.insert(k.clone(), v.clone());
            }
        }
        if let Some(ref updates) = p.decision.updates {
            for (k, v) in updates {
                variables.insert(k.clone(), v.clone());
            }
        }
        variables.insert("__agent_choice".into(), serde_json::json!(p.decision.choice));
        if let Some(ref reasoning) = p.decision.reasoning {
            variables.insert("__agent_reasoning".into(), serde_json::json!(reasoning));
        }

        let start = std::time::Instant::now();
        let mut results: Vec<onecrawl_cdp::StepResult> = Vec::new();
        let mut succeeded = 0usize;
        let mut failed = 0usize;
        let mut skipped = 0usize;
        let mut overall_status = onecrawl_cdp::StepStatus::Success;

        for (i, step) in workflow.steps.iter().enumerate().skip(p.resume_from + 1) {
            let step_id = if step.id.is_empty() { format!("step_{i}") } else { step.id.clone() };
            let step_name = if step.name.is_empty() { format!("Step {i}") } else { step.name.clone() };

            if let Some(ref cond) = step.condition {
                let interpolated = onecrawl_cdp::workflow::interpolate(cond, &variables);
                if !onecrawl_cdp::workflow::evaluate_condition(&interpolated, &variables) {
                    results.push(onecrawl_cdp::StepResult {
                        step_id, step_name,
                        status: onecrawl_cdp::StepStatus::Skipped,
                        output: None, error: None, duration_ms: 0,
                        paused: false,
                    });
                    skipped += 1;
                    continue;
                }
            }

            let step_start = std::time::Instant::now();
            let result = self.execute_step(&page, &step.action, &mut variables).await;
            let duration_ms = step_start.elapsed().as_millis() as u64;

            match result {
                Ok(output) => {
                    if matches!(&step.action, onecrawl_cdp::Action::Agent { .. }) {
                        let agent_context = output.clone();
                        results.push(onecrawl_cdp::StepResult {
                            step_id, step_name,
                            status: onecrawl_cdp::StepStatus::Paused,
                            output, error: None, duration_ms,
                            paused: true,
                        });
                        let total_duration_ms = start.elapsed().as_millis() as u64;
                        return json_ok(&serde_json::json!({
                            "status": "paused",
                            "paused_at": i,
                            "agent_context": agent_context,
                            "steps_completed": results,
                            "variables": variables,
                            "total_duration_ms": total_duration_ms,
                        }));
                    }
                    if let Some(ref save_key) = step.save_as {
                        if let Some(ref out) = output {
                            variables.insert(save_key.clone(), out.clone());
                        }
                    }
                    results.push(onecrawl_cdp::StepResult {
                        step_id, step_name,
                        status: onecrawl_cdp::StepStatus::Success,
                        output, error: None, duration_ms,
                        paused: false,
                    });
                    succeeded += 1;
                }
                Err(e) => {
                    let err_msg = format!("{}", e.message);
                    let error_action = step.on_error.as_ref()
                        .unwrap_or(&workflow.on_error.action);
                    results.push(onecrawl_cdp::StepResult {
                        step_id, step_name,
                        status: onecrawl_cdp::StepStatus::Failed,
                        output: None, error: Some(err_msg.clone()), duration_ms,
                        paused: false,
                    });
                    failed += 1;
                    match error_action {
                        onecrawl_cdp::workflow::StepErrorAction::Stop => {
                            overall_status = onecrawl_cdp::StepStatus::Failed;
                            break;
                        }
                        _ => continue,
                    }
                }
            }
        }

        let total_duration_ms = start.elapsed().as_millis() as u64;
        json_ok(&serde_json::json!({
            "status": format!("{:?}", overall_status).to_lowercase(),
            "resumed_from": p.resume_from,
            "agent_decision": {
                "choice": p.decision.choice,
                "reasoning": p.decision.reasoning,
            },
            "steps": results,
            "variables": variables,
            "total_duration_ms": total_duration_ms,
            "steps_succeeded": succeeded,
            "steps_failed": failed,
            "steps_skipped": skipped,
        }))
    }

    pub(crate) async fn agent_decide(
        &self,
        p: AgentDecideParams,
    ) -> Result<CallToolResult, McpError> {
        let mut response = serde_json::json!({
            "prompt": p.prompt,
            "awaiting_decision": true,
        });
        if let Some(ref opts) = p.options {
            response["options"] = serde_json::json!(opts);
        }
        if let Some(ref ctx) = p.context {
            response["context"] = ctx.clone();
        }
        json_ok(&response)
    }

}
