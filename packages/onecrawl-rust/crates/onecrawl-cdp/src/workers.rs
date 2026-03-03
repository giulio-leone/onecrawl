//! Service Worker and Web Worker management via JS APIs.
//!
//! Uses `navigator.serviceWorker.getRegistrations()` to list and unregister workers.

use chromiumoxide::Page;
use onecrawl_core::Result;
use serde::{Deserialize, Serialize};

/// Information about a registered service worker.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServiceWorkerInfo {
    pub script_url: String,
    pub state: String,
    pub scope: String,
}

/// Get all registered service workers.
pub async fn get_service_workers(page: &Page) -> Result<Vec<ServiceWorkerInfo>> {
    let result = page
        .evaluate(
            r#"
            (async () => {
                if (!navigator.serviceWorker) return [];
                try {
                    const regs = await navigator.serviceWorker.getRegistrations();
                    return regs.map(r => ({
                        script_url: r.active ? r.active.scriptURL : (r.installing ? r.installing.scriptURL : (r.waiting ? r.waiting.scriptURL : '')),
                        state: r.active ? r.active.state : (r.installing ? 'installing' : (r.waiting ? 'waiting' : 'unknown')),
                        scope: r.scope
                    }));
                } catch(e) { return []; }
            })()
            "#,
        )
        .await
        .map_err(|e| onecrawl_core::Error::Cdp(format!("get_service_workers failed: {e}")))?;

    let workers: Vec<ServiceWorkerInfo> = result.into_value().unwrap_or_default();

    Ok(workers)
}

/// Unregister all service workers. Returns the number unregistered.
pub async fn unregister_service_workers(page: &Page) -> Result<usize> {
    let result = page
        .evaluate(
            r#"
            (async () => {
                if (!navigator.serviceWorker) return 0;
                try {
                    const regs = await navigator.serviceWorker.getRegistrations();
                    let count = 0;
                    for (const r of regs) {
                        if (await r.unregister()) count++;
                    }
                    return count;
                } catch(e) { return 0; }
            })()
            "#,
        )
        .await
        .map_err(|e| {
            onecrawl_core::Error::Cdp(format!("unregister_service_workers failed: {e}"))
        })?;

    let count: usize = result.into_value().unwrap_or(0);
    Ok(count)
}

/// Get general web worker information as JSON.
pub async fn get_worker_info(page: &Page) -> Result<serde_json::Value> {
    let result = page
        .evaluate(
            r#"
            (async () => {
                const info = { serviceWorkerSupported: !!navigator.serviceWorker, registrations: [] };
                if (navigator.serviceWorker) {
                    try {
                        const regs = await navigator.serviceWorker.getRegistrations();
                        info.registrations = regs.map(r => ({
                            scope: r.scope,
                            updateViaCache: r.updateViaCache,
                            active: r.active ? { scriptURL: r.active.scriptURL, state: r.active.state } : null,
                            installing: r.installing ? { scriptURL: r.installing.scriptURL, state: r.installing.state } : null,
                            waiting: r.waiting ? { scriptURL: r.waiting.scriptURL, state: r.waiting.state } : null
                        }));
                    } catch(e) { info.error = String(e); }
                }
                return info;
            })()
            "#,
        )
        .await
        .map_err(|e| onecrawl_core::Error::Cdp(format!("get_worker_info failed: {e}")))?;

    let val: serde_json::Value = result.into_value().unwrap_or(serde_json::Value::Null);
    Ok(val)
}
