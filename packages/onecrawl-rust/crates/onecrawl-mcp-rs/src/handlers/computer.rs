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

}
