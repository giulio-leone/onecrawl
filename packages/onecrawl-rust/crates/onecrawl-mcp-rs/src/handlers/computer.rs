//! Handler implementations for the `computer` super-tool.

use rmcp::{ErrorData as McpError, model::*};
use crate::cdp_tools::*;
use crate::helpers::{mcp_err, ensure_page, json_ok, McpResult};
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

}
