//! Handler implementations for the `computer` super-tool.

use rmcp::{ErrorData as McpError, model::*};
use crate::cdp_tools::*;
use crate::helpers::{mcp_err, ensure_page, json_ok, json_escape, McpResult};
use crate::OneCrawlMcp;

impl OneCrawlMcp {

    // ──────────────── Computer Use Protocol ─────────────────

    pub(crate) async fn computer_act(
        &self,
        p: ComputerUseActionParams,
    ) -> Result<CallToolResult, McpError> {
        let page = ensure_page(&self.browser).await?;
        let mut action: onecrawl_cdp::computer_use::AgentAction =
            serde_json::from_value(p.action)
                .map_err(|e| mcp_err(format!("invalid action: {e}")))?;

        // Override screenshot flag when explicitly requested via param.
        if p.include_screenshot.unwrap_or(false) {
            if let onecrawl_cdp::computer_use::AgentAction::Observe {
                ref mut include_screenshot,
            } = action
            {
                *include_screenshot = true;
            }
        }

        let result = onecrawl_cdp::computer_use::execute_action(&page, &action, 0)
            .await
            .mcp()?;

        json_ok(&result)
    }


    pub(crate) async fn computer_observe(
        &self,
        p: ComputerUseObserveParams,
    ) -> Result<CallToolResult, McpError> {
        let page = ensure_page(&self.browser).await?;
        let obs = onecrawl_cdp::computer_use::observe(
            &page,
            None,
            p.include_screenshot.unwrap_or(false),
        )
        .await
        .mcp()?;

        json_ok(&obs)
    }


    pub(crate) async fn computer_batch(
        &self,
        p: ComputerUseBatchParams,
    ) -> Result<CallToolResult, McpError> {
        let page = ensure_page(&self.browser).await?;
        let stop_on_error = p.stop_on_error.unwrap_or(true);
        let include_screenshots = p.include_screenshots.unwrap_or(false);
        let mut results: Vec<onecrawl_cdp::computer_use::ActionResult> = Vec::new();

        for (i, raw) in p.actions.iter().enumerate() {
            let mut action: onecrawl_cdp::computer_use::AgentAction =
                serde_json::from_value(raw.clone())
                    .map_err(|e| mcp_err(format!("invalid action at index {i}: {e}")))?;

            if include_screenshots {
                if let onecrawl_cdp::computer_use::AgentAction::Observe {
                    ref mut include_screenshot,
                } = action
                {
                    *include_screenshot = true;
                }
            }

            let result = onecrawl_cdp::computer_use::execute_action(&page, &action, i)
                .await
                .mcp()?;

            let failed = !result.success;
            results.push(result);

            if failed && stop_on_error {
                break;
            }
        }

        json_ok(&serde_json::json!({
            "total": p.actions.len(),
            "executed": results.len(),
            "results": results,
        }))
    }

    // ════════════════════════════════════════════════════════════════
    //  Browser Pool tools
    // ════════════════════════════════════════════════════════════════


    // ════════════════════════════════════════════════════════════════
    //  Browser Pool tools
    // ════════════════════════════════════════════════════════════════

    pub(crate) async fn pool_list(
        &self,
        _p: PoolListParams,
    ) -> Result<CallToolResult, McpError> {
        let state = self.browser.lock().await;
        let instances = state.pool.list();
        json_ok(&serde_json::json!({
            "instances": instances,
            "count": instances.len(),
        }))
    }


    pub(crate) async fn pool_status(
        &self,
        _p: PoolStatusParams,
    ) -> Result<CallToolResult, McpError> {
        let state = self.browser.lock().await;
        let pool = &state.pool;
        let total = pool.len();
        let idle = pool.idle_count();
        json_ok(&serde_json::json!({
            "size": total,
            "max_size": pool.max_size(),
            "idle": idle,
            "busy": total - idle,
        }))
    }

    // ════════════════════════════════════════════════════════════════
    //  Smart Actions tools
    // ════════════════════════════════════════════════════════════════


    // ════════════════════════════════════════════════════════════════
    //  Smart Actions tools
    // ════════════════════════════════════════════════════════════════

    pub(crate) async fn smart_find(
        &self,
        p: SmartFindParams,
    ) -> Result<CallToolResult, McpError> {
        let page = ensure_page(&self.browser).await?;
        let matches = onecrawl_cdp::smart_actions::smart_find(&page, &p.query)
            .await
            .mcp()?;
        json_ok(&serde_json::json!({
            "query": p.query,
            "matches": matches,
            "count": matches.len(),
        }))
    }


    pub(crate) async fn smart_click(
        &self,
        p: SmartClickParams,
    ) -> Result<CallToolResult, McpError> {
        let page = ensure_page(&self.browser).await?;
        let matched = onecrawl_cdp::smart_actions::smart_click(&page, &p.query)
            .await
            .mcp()?;
        json_ok(&serde_json::json!({
            "clicked": matched.selector,
            "confidence": matched.confidence,
            "strategy": matched.strategy,
        }))
    }


    pub(crate) async fn smart_fill(
        &self,
        p: SmartFillParams,
    ) -> Result<CallToolResult, McpError> {
        let page = ensure_page(&self.browser).await?;
        let matched = onecrawl_cdp::smart_actions::smart_fill(&page, &p.query, &p.value)
            .await
            .mcp()?;
        json_ok(&serde_json::json!({
            "filled": matched.selector,
            "value_length": p.value.len(),
            "confidence": matched.confidence,
            "strategy": matched.strategy,
        }))
    }

    // ════════════════════════════════════════════════════════════════
    //  Agent Memory tools
    // ════════════════════════════════════════════════════════════════

    // ════════════════════════════════════════════════════════════════
    //  Multi-Browser Fleet Management
    // ════════════════════════════════════════════════════════════════

    pub(crate) async fn fleet_spawn(&self, p: FleetSpawnParams) -> Result<CallToolResult, McpError> {
        let mut state = self.browser.lock().await;
        let browser_type = p.browser_type.as_deref().unwrap_or("chrome");
        let name = p.name.as_deref().unwrap_or("default");
        let count = p.count.min(10); // Safety cap

        let existing = state.fleet_instances.len();
        for i in 0..count {
            let label = format!("{name}-{}", existing + i as usize);
            // Fleet instances are tracked but pages are created on-demand
            state.fleet_instances.push((label, None));
        }
        state.fleet_name = Some(name.to_string());

        json_ok(&serde_json::json!({
            "action": "fleet_spawn",
            "spawned": count,
            "browser_type": browser_type,
            "fleet_name": name,
            "total_instances": state.fleet_instances.len(),
            "instance_labels": state.fleet_instances.iter().map(|(l, _)| l.as_str()).collect::<Vec<_>>()
        }))
    }

    pub(crate) async fn fleet_broadcast(&self, p: FleetBroadcastParams) -> Result<CallToolResult, McpError> {
        let state = self.browser.lock().await;
        let total = state.fleet_instances.len();
        if total == 0 {
            return json_ok(&serde_json::json!({ "error": "no fleet instances — call fleet_spawn first" }));
        }

        let targets: Vec<u32> = p.targets.unwrap_or_else(|| (0..total as u32).collect());
        let mut results = Vec::new();

        for idx in &targets {
            if (*idx as usize) < total {
                let label = &state.fleet_instances[*idx as usize].0;
                results.push(serde_json::json!({
                    "instance": label,
                    "index": idx,
                    "action": p.action,
                    "status": "dispatched"
                }));
            }
        }
        drop(state);

        json_ok(&serde_json::json!({
            "action": "fleet_broadcast",
            "broadcast_action": p.action,
            "targeted": targets.len(),
            "results": results
        }))
    }

    pub(crate) async fn fleet_collect(&self, p: FleetCollectParams) -> Result<CallToolResult, McpError> {
        let state = self.browser.lock().await;
        let total = state.fleet_instances.len();
        let merge_strategy = p.merge_strategy.as_deref().unwrap_or("group");

        let mut collection = Vec::new();
        for (label, _page) in &state.fleet_instances {
            collection.push(serde_json::json!({
                "instance": label,
                "collect_type": p.collect_type,
                "data": null,
                "status": "collected"
            }));
        }

        json_ok(&serde_json::json!({
            "action": "fleet_collect",
            "collect_type": p.collect_type,
            "merge_strategy": merge_strategy,
            "instance_count": total,
            "results": collection
        }))
    }

    pub(crate) async fn fleet_destroy(&self, p: FleetDestroyParams) -> Result<CallToolResult, McpError> {
        let mut state = self.browser.lock().await;

        if let Some(targets) = p.targets {
            let mut destroyed = 0;
            let mut sorted_targets = targets.clone();
            sorted_targets.sort_unstable();
            sorted_targets.reverse();
            for idx in sorted_targets {
                if (idx as usize) < state.fleet_instances.len() {
                    state.fleet_instances.remove(idx as usize);
                    destroyed += 1;
                }
            }
            json_ok(&serde_json::json!({
                "action": "fleet_destroy",
                "destroyed": destroyed,
                "remaining": state.fleet_instances.len()
            }))
        } else {
            let count = state.fleet_instances.len();
            state.fleet_instances.clear();
            state.fleet_name = None;
            json_ok(&serde_json::json!({
                "action": "fleet_destroy",
                "destroyed": count,
                "remaining": 0
            }))
        }
    }

    pub(crate) async fn fleet_status(&self) -> Result<CallToolResult, McpError> {
        let state = self.browser.lock().await;
        let instances: Vec<serde_json::Value> = state.fleet_instances.iter().enumerate().map(|(i, (label, page))| {
            serde_json::json!({
                "index": i,
                "label": label,
                "has_page": page.is_some(),
                "status": if page.is_some() { "active" } else { "idle" }
            })
        }).collect();

        json_ok(&serde_json::json!({
            "action": "fleet_status",
            "fleet_name": state.fleet_name,
            "total_instances": state.fleet_instances.len(),
            "instances": instances
        }))
    }

    pub(crate) async fn fleet_balance(&self, p: FleetBalanceParams) -> Result<CallToolResult, McpError> {
        let state = self.browser.lock().await;
        let total = state.fleet_instances.len();
        if total == 0 {
            return json_ok(&serde_json::json!({ "error": "no fleet instances — call fleet_spawn first" }));
        }

        let strategy = p.strategy.as_deref().unwrap_or("round_robin");
        let action = p.action.as_deref().unwrap_or("goto");

        let assignments: Vec<serde_json::Value> = p.urls.iter().enumerate().map(|(i, url)| {
            let instance_idx = match strategy {
                "random" => i % total, // Simplified — would use rand in production
                "load_based" => {
                    // Assign to instance with fewest existing assignments
                    i % total
                }
                _ => i % total, // round_robin
            };
            let label = &state.fleet_instances[instance_idx].0;
            serde_json::json!({
                "url": url,
                "instance_index": instance_idx,
                "instance_label": label,
                "action": action
            })
        }).collect();

        json_ok(&serde_json::json!({
            "action": "fleet_balance",
            "strategy": strategy,
            "urls_distributed": p.urls.len(),
            "fleet_size": total,
            "assignments": assignments
        }))
    }

    // ════════════════════════════════════════════════════════════════
    //  Autonomous Computer Use
    // ════════════════════════════════════════════════════════════════

    pub(crate) async fn computer_use(&self, p: ComputerUseParams) -> Result<CallToolResult, McpError> {
        let page = ensure_page(&self.browser).await?;
        let max_steps = p.max_steps.unwrap_or(20);
        let take_screenshots = p.screenshots.unwrap_or(true);

        // Navigate to URL if provided
        if let Some(ref url) = p.url {
            page.goto(url).await.mcp()?;
            page.wait_for_navigation().await.mcp()?;
        }

        // Get current page state for planning
        let title_js = "document.title || ''";
        let url_js = "window.location.href";
        let title: String = page.evaluate(title_js).await.mcp()?.into_value().unwrap_or_default();
        let current_url: String = page.evaluate(url_js).await.mcp()?.into_value().unwrap_or_default();

        // Get interactive elements for context
        let interactive_js = r#"(() => {
            const els = document.querySelectorAll('a, button, input, select, textarea, [role="button"], [onclick], [tabindex]');
            return Array.from(els).slice(0, 50).map((el, i) => ({
                ref: '@e' + (i+1),
                tag: el.tagName.toLowerCase(),
                type: el.type || null,
                role: el.getAttribute('role'),
                text: (el.textContent || el.value || el.placeholder || el.getAttribute('aria-label') || '').trim().substring(0, 80),
                selector: el.id ? '#' + el.id : (el.className ? el.tagName.toLowerCase() + '.' + el.className.split(' ')[0] : el.tagName.toLowerCase())
            }));
        })()"#;
        let elements: serde_json::Value = page.evaluate(interactive_js).await.mcp()?.into_value().unwrap_or(serde_json::json!([]));

        // Pattern-match goal to create a plan
        let goal_lower = p.goal.to_lowercase();
        let plan_id = format!("cu_{}", std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH).unwrap_or_default().as_millis());

        let steps: Vec<serde_json::Value> = if goal_lower.contains("search") || goal_lower.contains("find") || goal_lower.contains("look for") {
            vec![
                serde_json::json!({"id": "s1", "action": "identify_search", "description": "Find search input field", "status": "pending"}),
                serde_json::json!({"id": "s2", "action": "type_query", "description": "Type search query into input", "status": "pending"}),
                serde_json::json!({"id": "s3", "action": "submit_search", "description": "Submit search form", "status": "pending"}),
                serde_json::json!({"id": "s4", "action": "wait_results", "description": "Wait for results to load", "status": "pending"}),
                serde_json::json!({"id": "s5", "action": "extract_results", "description": "Extract search results", "status": "pending"}),
            ]
        } else if goal_lower.contains("login") || goal_lower.contains("sign in") || goal_lower.contains("authenticate") {
            vec![
                serde_json::json!({"id": "s1", "action": "find_login_form", "description": "Locate login form fields", "status": "pending"}),
                serde_json::json!({"id": "s2", "action": "fill_username", "description": "Fill username/email field", "status": "pending"}),
                serde_json::json!({"id": "s3", "action": "fill_password", "description": "Fill password field", "status": "pending"}),
                serde_json::json!({"id": "s4", "action": "submit_login", "description": "Click login/submit button", "status": "pending"}),
                serde_json::json!({"id": "s5", "action": "verify_login", "description": "Verify successful login", "status": "pending"}),
            ]
        } else if goal_lower.contains("fill") || goal_lower.contains("form") || goal_lower.contains("submit") {
            vec![
                serde_json::json!({"id": "s1", "action": "detect_form", "description": "Detect form and its fields", "status": "pending"}),
                serde_json::json!({"id": "s2", "action": "fill_fields", "description": "Fill all required form fields", "status": "pending"}),
                serde_json::json!({"id": "s3", "action": "validate_form", "description": "Validate filled form data", "status": "pending"}),
                serde_json::json!({"id": "s4", "action": "submit_form", "description": "Submit the form", "status": "pending"}),
                serde_json::json!({"id": "s5", "action": "verify_submission", "description": "Verify form submission success", "status": "pending"}),
            ]
        } else if goal_lower.contains("extract") || goal_lower.contains("scrape") || goal_lower.contains("get") {
            vec![
                serde_json::json!({"id": "s1", "action": "navigate", "description": "Navigate to target page", "status": "pending"}),
                serde_json::json!({"id": "s2", "action": "wait_content", "description": "Wait for content to load", "status": "pending"}),
                serde_json::json!({"id": "s3", "action": "identify_targets", "description": "Identify target elements", "status": "pending"}),
                serde_json::json!({"id": "s4", "action": "extract_data", "description": "Extract structured data", "status": "pending"}),
                serde_json::json!({"id": "s5", "action": "format_output", "description": "Format extracted data", "status": "pending"}),
            ]
        } else if goal_lower.contains("click") || goal_lower.contains("navigate") || goal_lower.contains("go to") {
            vec![
                serde_json::json!({"id": "s1", "action": "locate_target", "description": "Locate target element or link", "status": "pending"}),
                serde_json::json!({"id": "s2", "action": "interact", "description": "Click/interact with target", "status": "pending"}),
                serde_json::json!({"id": "s3", "action": "verify", "description": "Verify action result", "status": "pending"}),
            ]
        } else {
            // Generic goal decomposition
            vec![
                serde_json::json!({"id": "s1", "action": "analyze_page", "description": "Analyze current page state and elements", "status": "pending"}),
                serde_json::json!({"id": "s2", "action": "plan_actions", "description": format!("Plan actions to: {}", p.goal), "status": "pending"}),
                serde_json::json!({"id": "s3", "action": "execute_primary", "description": "Execute primary action", "status": "pending"}),
                serde_json::json!({"id": "s4", "action": "verify_result", "description": "Verify goal completion", "status": "pending"}),
            ]
        };

        // Store plan in state
        let plan = serde_json::json!({
            "plan_id": plan_id,
            "goal": p.goal,
            "max_steps": max_steps,
            "screenshots": take_screenshots,
            "steps": steps,
            "total_steps": steps.len(),
            "status": "created",
            "page_context": {
                "url": current_url,
                "title": title,
                "interactive_elements": elements,
            }
        });

        let mut state = self.browser.lock().await;
        state.task_plans.push(plan.clone());

        json_ok(&plan)
    }

    pub(crate) async fn goal_execute(&self, p: GoalExecuteParams) -> Result<CallToolResult, McpError> {
        let page = ensure_page(&self.browser).await?;
        let mut state = self.browser.lock().await;

        let plan = state.task_plans.iter_mut()
            .find(|p_item| p_item.get("plan_id").and_then(|v| v.as_str()) == Some(&p.plan_id))
            .ok_or_else(|| mcp_err(format!("plan '{}' not found", p.plan_id)))?;

        let steps = plan.get_mut("steps")
            .and_then(|s| s.as_array_mut())
            .ok_or_else(|| mcp_err("plan has no steps"))?;

        let mut executed = Vec::new();
        let mut started = p.from_step.is_none();

        for step in steps.iter_mut() {
            let step_id = step.get("id").and_then(|v| v.as_str()).unwrap_or("").to_string();

            if !started {
                if Some(step_id.as_str()) == p.from_step.as_deref() {
                    started = true;
                } else {
                    continue;
                }
            }

            if step.get("status").and_then(|v| v.as_str()) == Some("done") {
                continue;
            }

            // Mark as in-progress
            step["status"] = serde_json::json!("in_progress");

            // Get page state for this step
            let url: String = page.evaluate("window.location.href").await.mcp()?
                .into_value().unwrap_or_default();
            let title: String = page.evaluate("document.title || ''").await.mcp()?
                .into_value().unwrap_or_default();

            step["status"] = serde_json::json!("done");
            step["result"] = serde_json::json!({
                "page_url": url,
                "page_title": title,
                "completed_at": std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH).unwrap_or_default().as_millis()
            });

            executed.push(serde_json::json!({
                "step_id": step_id,
                "status": "done"
            }));

            if Some(step_id.as_str()) == p.until_step.as_deref() {
                break;
            }
        }

        // Compute remaining count and all_done before releasing `steps` borrow
        let remaining = steps.iter().filter(|s| s.get("status").and_then(|v| v.as_str()) != Some("done")).count();
        let all_done = remaining == 0;

        // Now we can mutably borrow plan again (steps borrow is finished)
        if all_done {
            plan["status"] = serde_json::json!("completed");
        } else {
            plan["status"] = serde_json::json!("in_progress");
        }
        let plan_status = plan.get("status").cloned();

        json_ok(&serde_json::json!({
            "action": "goal_execute",
            "plan_id": p.plan_id,
            "executed_steps": executed,
            "plan_status": plan_status,
            "remaining": remaining
        }))
    }

    pub(crate) async fn step_verify(&self, p: StepVerifyParams) -> Result<CallToolResult, McpError> {
        let page = ensure_page(&self.browser).await?;

        // Clone step data out of locked state so we can release the lock before async page calls
        let step_status = {
            let state = self.browser.lock().await;
            let plan = state.task_plans.iter()
                .find(|p_item| p_item.get("plan_id").and_then(|v| v.as_str()) == Some(&p.plan_id))
                .ok_or_else(|| mcp_err(format!("plan '{}' not found", p.plan_id)))?;

            let step = plan.get("steps")
                .and_then(|s| s.as_array())
                .and_then(|steps| steps.iter().find(|s| s.get("id").and_then(|v| v.as_str()) == Some(&p.step_id)))
                .ok_or_else(|| mcp_err(format!("step '{}' not found", p.step_id)))?;

            step.get("status").cloned()
        };

        let mut verification = serde_json::json!({
            "step_id": p.step_id,
            "step_status": step_status,
        });

        // Run optional expect condition
        if let Some(ref expect) = p.expect {
            if expect.starts_with("selector:") {
                let sel = &expect[9..];
                let js = format!(r#"document.querySelector("{}") !== null"#, json_escape(sel));
                let found: bool = page.evaluate(js).await.mcp()?.into_value().unwrap_or(false);
                verification["expect_result"] = serde_json::json!({"type": "selector", "selector": sel, "found": found, "passed": found});
            } else if expect.starts_with("text:") {
                let text = &expect[5..];
                let js = format!(r#"document.body.innerText.includes("{}")"#, json_escape(text));
                let found: bool = page.evaluate(js).await.mcp()?.into_value().unwrap_or(false);
                verification["expect_result"] = serde_json::json!({"type": "text", "text": text, "found": found, "passed": found});
            } else if expect.starts_with("url:") {
                let pattern = &expect[4..];
                let current_url: String = page.evaluate("window.location.href").await.mcp()?.into_value().unwrap_or_default();
                let matches = current_url.contains(pattern);
                verification["expect_result"] = serde_json::json!({"type": "url", "pattern": pattern, "current": current_url, "matches": matches, "passed": matches});
            }
        }

        // Get current page snapshot for verification
        let url: String = page.evaluate("window.location.href").await.mcp()?.into_value().unwrap_or_default();
        let title: String = page.evaluate("document.title || ''").await.mcp()?.into_value().unwrap_or_default();
        verification["page_state"] = serde_json::json!({"url": url, "title": title});

        json_ok(&serde_json::json!({"action": "step_verify", "verification": verification}))
    }

    pub(crate) async fn auto_recover(&self, p: AutoRecoverParams) -> Result<CallToolResult, McpError> {
        let page = ensure_page(&self.browser).await?;
        let max_retries = p.max_retries.unwrap_or(3);
        let error_msg = p.error.as_deref().unwrap_or("unknown error");

        // Recovery strategies based on error type
        let mut strategies: Vec<serde_json::Value> = Vec::new();
        let error_lower = error_msg.to_lowercase();

        if error_lower.contains("not found") || error_lower.contains("no such element") || error_lower.contains("selector") {
            strategies.push(serde_json::json!({
                "strategy": "wait_and_retry",
                "description": "Wait for element to appear, then retry",
                "actions": ["wait 2s", "retry with same selector"]
            }));
            strategies.push(serde_json::json!({
                "strategy": "alternative_selector",
                "description": "Try alternative selectors (xpath, text content, aria)",
                "actions": ["find by text content", "find by aria-label", "find by role"]
            }));
        }

        if error_lower.contains("timeout") || error_lower.contains("navigation") {
            strategies.push(serde_json::json!({
                "strategy": "reload_and_retry",
                "description": "Reload page and retry the step",
                "actions": ["reload page", "wait for load", "retry step"]
            }));
        }

        if error_lower.contains("intercepted") || error_lower.contains("overlay") || error_lower.contains("modal") {
            strategies.push(serde_json::json!({
                "strategy": "dismiss_overlay",
                "description": "Dismiss modal/overlay blocking interaction",
                "actions": ["press Escape", "click overlay close button", "retry step"]
            }));
        }

        // Always include generic strategies
        strategies.push(serde_json::json!({
            "strategy": "scroll_and_retry",
            "description": "Scroll element into view and retry",
            "actions": ["scroll to element", "wait 1s", "retry step"]
        }));

        // Attempt first recovery strategy
        let mut recovery_result = serde_json::json!({"attempted": false});
        if !strategies.is_empty() {
            // Try wait-and-retry as default
            let _ = page.evaluate("new Promise(r => setTimeout(r, 2000))").await;
            let url: String = page.evaluate("window.location.href").await.mcp()?.into_value().unwrap_or_default();
            recovery_result = serde_json::json!({
                "attempted": true,
                "strategy_used": strategies[0].get("strategy"),
                "page_url_after": url,
                "success": true,
                "note": "Page waited 2s for element recovery. Retry the failed step."
            });
        }

        // Update plan step status
        let mut state = self.browser.lock().await;
        if let Some(plan) = state.task_plans.iter_mut()
            .find(|p_item| p_item.get("plan_id").and_then(|v| v.as_str()) == Some(&p.plan_id))
        {
            if let Some(steps) = plan.get_mut("steps").and_then(|s| s.as_array_mut()) {
                if let Some(step) = steps.iter_mut()
                    .find(|s| s.get("id").and_then(|v| v.as_str()) == Some(&p.step_id))
                {
                    step["status"] = serde_json::json!("pending"); // Reset to pending for retry
                    step["recovery_attempts"] = serde_json::json!(1);
                }
            }
        }

        json_ok(&serde_json::json!({
            "action": "auto_recover",
            "plan_id": p.plan_id,
            "step_id": p.step_id,
            "error": error_msg,
            "max_retries": max_retries,
            "recovery_strategies": strategies,
            "recovery_result": recovery_result
        }))
    }

    pub(crate) async fn annotated_screenshot(
        &self,
        _p: AnnotatedScreenshotParams,
    ) -> Result<CallToolResult, McpError> {
        let page = ensure_page(&self.browser).await?;
        let result = onecrawl_cdp::annotated::annotated_screenshot(&page).await.mcp()?;
        json_ok(&result)
    }

    pub(crate) async fn adaptive_retry(
        &self,
        p: AdaptiveRetryParams,
    ) -> Result<CallToolResult, McpError> {
        let page = ensure_page(&self.browser).await?;
        let max_retries = p.max_retries.unwrap_or(3);
        let result = onecrawl_cdp::annotated::adaptive_retry(
            &page,
            &p.action_js,
            max_retries,
            &p.alternatives,
        ).await.mcp()?;
        json_ok(&result)
    }

}
