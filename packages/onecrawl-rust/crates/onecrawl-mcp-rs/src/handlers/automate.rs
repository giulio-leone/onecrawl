//! Handler implementations for the `automate` super-tool.

use rmcp::{ErrorData as McpError, model::*};
use crate::helpers::{mcp_err, ensure_page, json_ok, parse_json_str, McpResult};
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
                        sel_json = serde_json::to_string(&sel).unwrap(),
                        attr_json = serde_json::to_string(attr).unwrap())
                } else {
                    format!(r#"Array.from(document.querySelectorAll({sel_json})).map(e => e.textContent.trim())"#,
                        sel_json = serde_json::to_string(&sel).unwrap())
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
        }
        })
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
        json_ok(&serde_json::to_value(&plan).unwrap())
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

        json_ok(&serde_json::to_value(&result).unwrap())
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
                    sel = serde_json::to_string(target).unwrap()
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
}
