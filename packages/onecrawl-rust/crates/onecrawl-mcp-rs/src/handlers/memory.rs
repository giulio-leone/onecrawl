//! Handler implementations for the `memory` super-tool.

use rmcp::{ErrorData as McpError, model::*};
use crate::cdp_tools::*;
use crate::helpers::{mcp_err, json_ok, parse_memory_category, McpResult};
use crate::types::*;
use crate::OneCrawlMcp;

impl OneCrawlMcp {

    // ════════════════════════════════════════════════════════════════
    //  Agent Memory tools
    // ════════════════════════════════════════════════════════════════

    pub(crate) fn ensure_memory(state: &mut BrowserState) -> &mut onecrawl_cdp::AgentMemory {
        if state.memory.is_none() {
            let home = std::env::var("HOME").unwrap_or_else(|_| ".".into());
            let path = std::path::PathBuf::from(home).join(".onecrawl").join("agent_memory.json");
            state.memory = Some(
                onecrawl_cdp::AgentMemory::load(&path).unwrap_or_else(|_| onecrawl_cdp::AgentMemory::new(&path))
            );
        }
        state.memory.as_mut().unwrap()
    }


    pub(crate) async fn memory_store(
        &self,
        p: MemoryStoreParams,
    ) -> Result<CallToolResult, McpError> {
        let mut state = self.browser.lock().await;
        let category = parse_memory_category(p.category.as_deref())
            .unwrap_or(onecrawl_cdp::MemoryCategory::Custom);
        let mem = Self::ensure_memory(&mut state);
        mem.store(&p.key, p.value.clone(), category, p.domain.clone())
            .mcp()?;
        json_ok(&serde_json::json!({
            "stored": p.key,
            "category": format!("{:?}", mem.recall(&p.key).map(|e| &e.category)),
        }))
    }


    pub(crate) async fn memory_recall(
        &self,
        p: MemoryRecallParams,
    ) -> Result<CallToolResult, McpError> {
        let mut state = self.browser.lock().await;
        let mem = Self::ensure_memory(&mut state);
        match mem.recall(&p.key) {
            Some(entry) => json_ok(&serde_json::json!({
                "key": entry.key,
                "value": entry.value,
                "category": format!("{:?}", entry.category),
                "domain": entry.domain,
                "access_count": entry.access_count,
                "created_at": entry.created_at,
                "accessed_at": entry.accessed_at,
            })),
            None => json_ok(&serde_json::json!({ "key": p.key, "found": false })),
        }
    }


    pub(crate) async fn memory_search(
        &self,
        p: MemorySearchParams,
    ) -> Result<CallToolResult, McpError> {
        let mut state = self.browser.lock().await;
        let category = parse_memory_category(p.category.as_deref());
        let mem = Self::ensure_memory(&mut state);
        let results = mem.search(&p.query, category, p.domain.as_deref());
        let entries: Vec<serde_json::Value> = results.iter().map(|e| {
            serde_json::json!({
                "key": e.key,
                "value": e.value,
                "category": format!("{:?}", e.category),
                "domain": e.domain,
                "access_count": e.access_count,
            })
        }).collect();
        json_ok(&serde_json::json!({
            "query": p.query,
            "count": entries.len(),
            "results": entries,
        }))
    }


    pub(crate) async fn memory_forget(
        &self,
        p: MemoryForgetParams,
    ) -> Result<CallToolResult, McpError> {
        let mut state = self.browser.lock().await;
        let mem = Self::ensure_memory(&mut state);
        if let Some(key) = &p.key {
            let removed = mem.forget(key);
            json_ok(&serde_json::json!({ "removed": removed, "key": key }))
        } else if let Some(domain) = &p.domain {
            let count = mem.clear_domain(domain);
            json_ok(&serde_json::json!({ "removed": count, "domain": domain }))
        } else {
            let count = mem.clear_all();
            json_ok(&serde_json::json!({ "removed": count, "cleared": "all" }))
        }
    }


    pub(crate) async fn memory_domain_strategy(
        &self,
        p: MemoryDomainStrategyParams,
    ) -> Result<CallToolResult, McpError> {
        let mut state = self.browser.lock().await;
        let mem = Self::ensure_memory(&mut state);
        if let Some(strategy_val) = p.strategy {
            let strategy: onecrawl_cdp::DomainStrategy = serde_json::from_value(strategy_val)
                .map_err(|e| mcp_err(format!("invalid strategy JSON: {e}")))?;
            mem.store_domain_strategy(strategy)
                .mcp()?;
            json_ok(&serde_json::json!({ "stored": true, "domain": p.domain }))
        } else {
            match mem.recall_domain_strategy(&p.domain) {
                Some(strategy) => json_ok(&serde_json::json!({
                    "domain": strategy.domain,
                    "login_selectors": strategy.login_selectors,
                    "navigation_patterns": strategy.navigation_patterns,
                    "known_popups": strategy.known_popups,
                    "rate_limit_info": strategy.rate_limit_info,
                    "anti_bot_level": strategy.anti_bot_level,
                })),
                None => json_ok(&serde_json::json!({ "domain": p.domain, "found": false })),
            }
        }
    }


    pub(crate) async fn memory_stats(
        &self,
        _p: MemoryStatsParams,
    ) -> Result<CallToolResult, McpError> {
        let mut state = self.browser.lock().await;
        let mem = Self::ensure_memory(&mut state);
        let stats = mem.stats();
        json_ok(&serde_json::json!({
            "total_entries": stats.total_entries,
            "max_entries": stats.max_entries,
            "categories": stats.categories,
            "domains": stats.domains,
            "utilization": format!("{:.1}%", (stats.total_entries as f64 / stats.max_entries as f64) * 100.0),
        }))
    }

    // ════════════════════════════════════════════════════════════════
    //  Workflow DSL tools
    // ════════════════════════════════════════════════════════════════

}
