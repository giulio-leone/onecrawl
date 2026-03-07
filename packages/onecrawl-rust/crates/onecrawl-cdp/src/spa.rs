//! SPA (Single Page Application) interaction helpers.

use chromiumoxide::Page;
use onecrawl_core::{Error, Result};

/// Detect virtual/windowed scroll containers (react-window, tanstack-virtual, etc.)
/// Returns info about detected virtual lists.
pub async fn detect_virtual_scroll(page: &Page) -> Result<serde_json::Value> {
    let js = r#"
        const results = [];
        
        // Detect common virtual scroll patterns
        // react-window / react-virtualized
        const rwContainers = document.querySelectorAll('[style*="overflow"][style*="height"]');
        for (const el of rwContainers) {
            const style = getComputedStyle(el);
            const children = el.children;
            if (children.length > 0 && style.overflow !== 'visible') {
                const totalHeight = el.scrollHeight;
                const visibleHeight = el.clientHeight;
                const childCount = children.length;
                
                if (totalHeight > visibleHeight * 2 && childCount < 100) {
                    const firstChild = children[0];
                    const lastChild = children[children.length - 1];
                    const transform = firstChild?.style?.transform || '';
                    
                    results.push({
                        type: 'virtual_scroll',
                        selector: el.id ? `#${el.id}` : el.className ? `.${el.className.split(' ')[0]}` : el.tagName.toLowerCase(),
                        visible_items: childCount,
                        scroll_height: totalHeight,
                        visible_height: visibleHeight,
                        estimated_total: Math.round(totalHeight / (visibleHeight / childCount)),
                        has_transform: transform.length > 0
                    });
                }
            }
        }
        
        // Detect IntersectionObserver-based lazy loading
        const lazyImages = document.querySelectorAll('img[data-src], img[loading="lazy"], [data-lazy]');
        if (lazyImages.length > 0) {
            results.push({
                type: 'lazy_loading',
                count: lazyImages.length,
                selector: 'img[data-src], img[loading="lazy"], [data-lazy]'
            });
        }
        
        JSON.stringify(results)
    "#;

    let result = page
        .evaluate(js.to_string())
        .await
        .map_err(|e| Error::Cdp(format!("detect_virtual_scroll: {e}")))?;
    let raw: String = result.into_value().unwrap_or_else(|_| "[]".to_string());
    let parsed: serde_json::Value =
        serde_json::from_str(&raw).unwrap_or(serde_json::json!([]));
    Ok(parsed)
}

/// Auto-scroll a virtual list to materialize and extract all items.
pub async fn extract_virtual_scroll(
    page: &Page,
    container_selector: &str,
    item_selector: &str,
    max_items: usize,
) -> Result<Vec<String>> {
    let js = format!(
        r#"
        (async () => {{
            const container = document.querySelector('{container_selector}');
            if (!container) return JSON.stringify({{ error: 'Container not found' }});
            
            const items = new Set();
            const maxItems = {max_items};
            let lastCount = 0;
            let stableRounds = 0;
            
            // Scroll through the container collecting items
            for (let i = 0; i < 500 && items.size < maxItems; i++) {{
                const currentItems = container.querySelectorAll('{item_selector}');
                for (const item of currentItems) {{
                    items.add(item.textContent.trim());
                }}
                
                // Check if we're getting new items
                if (items.size === lastCount) {{
                    stableRounds++;
                    if (stableRounds > 5) break; // No new items after 5 scrolls
                }} else {{
                    stableRounds = 0;
                    lastCount = items.size;
                }}
                
                // Scroll down
                container.scrollTop += container.clientHeight * 0.8;
                await new Promise(r => setTimeout(r, 200));
            }}
            
            return JSON.stringify(Array.from(items));
        }})()
    "#
    );

    let result = page
        .evaluate(js)
        .await
        .map_err(|e| Error::Cdp(format!("extract_virtual_scroll: {e}")))?;
    let raw: String = result.into_value().unwrap_or_else(|_| "[]".to_string());
    let items: Vec<String> = serde_json::from_str(&raw).unwrap_or_default();
    Ok(items)
}

/// Wait for framework hydration to complete.
/// Supports Next.js, Nuxt, Remix, Gatsby, and generic React hydration.
pub async fn wait_hydration(page: &Page, timeout_ms: u64) -> Result<String> {
    let js = format!(
        r#"
        new Promise((resolve) => {{
            const start = Date.now();
            const timeout = {timeout_ms};
            
            function check() {{
                // Next.js: __NEXT_DATA__ exists and router is ready
                if (window.__NEXT_DATA__ && window.__next_f) {{
                    resolve('nextjs');
                    return;
                }}
                // Nuxt: __NUXT__ hydrated
                if (window.__NUXT__ && window.__NUXT__.state) {{
                    resolve('nuxt');
                    return;
                }}
                // Remix: hydrated flag
                if (window.__remixContext && document.querySelector('[data-remix-hydrated]')) {{
                    resolve('remix');
                    return;
                }}
                // React: check if root has been hydrated (no SSR mismatch)
                if (window.__REACT_DEVTOOLS_GLOBAL_HOOK__) {{
                    const roots = window.__REACT_DEVTOOLS_GLOBAL_HOOK__.getFiberRoots?.(1);
                    if (roots && roots.size > 0) {{
                        resolve('react');
                        return;
                    }}
                }}
                // Generic: document.readyState + no pending fetches
                if (document.readyState === 'complete') {{
                    resolve('generic');
                    return;
                }}
                
                if (Date.now() - start > timeout) {{
                    resolve('timeout');
                    return;
                }}
                
                requestAnimationFrame(check);
            }}
            check();
        }})
    "#
    );

    let result = page
        .evaluate(js)
        .await
        .map_err(|e| Error::Cdp(format!("wait_hydration: {e}")))?;
    let framework: String = result
        .into_value()
        .unwrap_or_else(|_| "unknown".to_string());
    Ok(framework)
}

/// Wait for all CSS animations/transitions to complete on a target element.
pub async fn wait_animations(page: &Page, selector: &str, timeout_ms: u64) -> Result<bool> {
    let js = format!(
        r#"
        new Promise((resolve) => {{
            const el = document.querySelector('{selector}');
            if (!el) {{ resolve(false); return; }}
            
            const timeout = setTimeout(() => resolve(false), {timeout_ms});
            
            const anims = el.getAnimations();
            if (anims.length === 0) {{
                clearTimeout(timeout);
                resolve(true);
                return;
            }}
            
            Promise.all(anims.map(a => a.finished)).then(() => {{
                clearTimeout(timeout);
                resolve(true);
            }}).catch(() => {{
                clearTimeout(timeout);
                resolve(false);
            }});
        }})
    "#
    );

    let result = page
        .evaluate(js)
        .await
        .map_err(|e| Error::Cdp(format!("wait_animations: {e}")))?;
    let done: bool = result.into_value().unwrap_or(false);
    Ok(done)
}

/// Smart network idle: wait until no network requests are pending for a threshold.
pub async fn wait_network_idle(page: &Page, idle_ms: u64, timeout_ms: u64) -> Result<bool> {
    let js = format!(
        r#"
        new Promise((resolve) => {{
            let pending = 0;
            let idleTimer = null;
            const timeout = setTimeout(() => resolve(false), {timeout_ms});
            
            const origFetch = window.fetch;
            const origXHROpen = XMLHttpRequest.prototype.open;
            const origXHRSend = XMLHttpRequest.prototype.send;
            
            function checkIdle() {{
                if (pending <= 0) {{
                    if (!idleTimer) {{
                        idleTimer = setTimeout(() => {{
                            clearTimeout(timeout);
                            // Restore originals
                            window.fetch = origFetch;
                            XMLHttpRequest.prototype.open = origXHROpen;
                            XMLHttpRequest.prototype.send = origXHRSend;
                            resolve(true);
                        }}, {idle_ms});
                    }}
                }} else if (idleTimer) {{
                    clearTimeout(idleTimer);
                    idleTimer = null;
                }}
            }}
            
            window.fetch = function(...args) {{
                pending++;
                return origFetch.apply(this, args).finally(() => {{
                    pending--;
                    checkIdle();
                }});
            }};
            
            XMLHttpRequest.prototype.send = function(...args) {{
                pending++;
                this.addEventListener('loadend', () => {{
                    pending--;
                    checkIdle();
                }});
                return origXHRSend.apply(this, args);
            }};
            
            checkIdle(); // Start checking immediately
        }})
    "#
    );

    let result = page
        .evaluate(js)
        .await
        .map_err(|e| Error::Cdp(format!("wait_network_idle: {e}")))?;
    let idle: bool = result.into_value().unwrap_or(false);
    Ok(idle)
}

/// Force-trigger lazy loading by scrolling elements into view.
pub async fn trigger_lazy_load(page: &Page, selector: &str) -> Result<usize> {
    let js = format!(
        r#"
        (async () => {{
            const elements = document.querySelectorAll('{selector}');
            let triggered = 0;
            for (const el of elements) {{
                el.scrollIntoView({{ behavior: 'instant', block: 'center' }});
                triggered++;
                await new Promise(r => setTimeout(r, 100));
            }}
            // Scroll back to top
            window.scrollTo(0, 0);
            return triggered;
        }})()
    "#
    );

    let result = page
        .evaluate(js)
        .await
        .map_err(|e| Error::Cdp(format!("trigger_lazy_load: {e}")))?;
    let count: usize = result.into_value().unwrap_or(0);
    Ok(count)
}

/// Inspect SPA state stores (Redux, Zustand, Pinia, MobX)
pub async fn state_inspect(page: &Page, store_path: Option<&str>) -> Result<serde_json::Value> {
    let path = store_path.unwrap_or("");
    let js = format!(r#"
        (() => {{
            const stores = {{}};
            
            // Redux
            try {{
                if (window.__REDUX_DEVTOOLS_EXTENSION__) stores.redux_devtools = true;
                if (window.__store) stores.redux_window = typeof window.__store.getState === 'function';
                const reduxEl = document.querySelector('[data-reactroot]');
                if (reduxEl && reduxEl._reactInternalInstance) stores.react_internal = true;
            }} catch(e) {{}}
            
            // Try to get Redux state
            try {{
                if (window.__store && window.__store.getState) {{
                    const state = window.__store.getState();
                    if ('{path}') {{
                        const parts = '{path}'.split('.');
                        let val = state;
                        for (const p of parts) {{ if (val) val = val[p]; }}
                        stores.redux_state = val;
                    }} else {{
                        stores.redux_state = Object.keys(state);
                    }}
                }}
            }} catch(e) {{ stores.redux_error = e.message; }}
            
            // Zustand
            try {{
                const zustandStores = Object.entries(window).filter(([k,v]) => v && typeof v.getState === 'function' && typeof v.subscribe === 'function');
                if (zustandStores.length > 0) {{
                    stores.zustand = {{}};
                    zustandStores.forEach(([name, store]) => {{
                        stores.zustand[name] = Object.keys(store.getState());
                    }});
                }}
            }} catch(e) {{}}
            
            // Next.js
            try {{
                if (window.__NEXT_DATA__) stores.nextjs = {{ page: window.__NEXT_DATA__.page, buildId: window.__NEXT_DATA__.buildId }};
            }} catch(e) {{}}
            
            // Nuxt
            try {{
                if (window.__nuxt || window.__NUXT__) stores.nuxt = true;
                if (window.__NUXT_DATA__) stores.nuxt_data = true;
            }} catch(e) {{}}
            
            // Vue/Pinia
            try {{
                if (window.__vue_app__) stores.vue = true;
                if (window.__pinia) {{
                    stores.pinia = Object.keys(window.__pinia.state.value || {{}});
                }}
            }} catch(e) {{}}
            
            return JSON.stringify(stores);
        }})()
    "#);

    let result = page
        .evaluate(js)
        .await
        .map_err(|e| Error::Cdp(format!("state_inspect: {e}")))?;
    let raw: String = result.into_value().unwrap_or_else(|_| "{}".to_string());
    Ok(serde_json::from_str(&raw).unwrap_or(serde_json::json!({})))
}

/// Track multi-step form wizard state
pub async fn form_wizard_track(page: &Page) -> Result<serde_json::Value> {
    let js = r#"
        (() => {
            const forms = document.querySelectorAll('form');
            const wizards = [];
            
            forms.forEach((form, fi) => {
                const inputs = form.querySelectorAll('input, select, textarea');
                const fieldsets = form.querySelectorAll('fieldset');
                const steps = form.querySelectorAll('[data-step], .step, .wizard-step, [class*="step"]');
                
                const formData = {};
                inputs.forEach(input => {
                    const name = input.name || input.id || `field_${input.type}`;
                    if (input.type === 'checkbox' || input.type === 'radio') {
                        formData[name] = input.checked;
                    } else {
                        formData[name] = input.value || '';
                    }
                });
                
                // Detect current step
                let currentStep = -1;
                let totalSteps = Math.max(steps.length, fieldsets.length);
                steps.forEach((step, si) => {
                    const visible = step.offsetParent !== null || step.style.display !== 'none';
                    if (visible) currentStep = si + 1;
                });
                
                // Check for progress indicators
                const progress = form.querySelector('progress, [role="progressbar"], .progress');
                
                wizards.push({
                    form_index: fi,
                    action: form.action || '',
                    method: form.method || 'get',
                    total_fields: inputs.length,
                    filled_fields: Array.from(inputs).filter(i => i.value || i.checked).length,
                    current_step: currentStep,
                    total_steps: totalSteps,
                    has_progress: !!progress,
                    data: formData,
                    valid: form.checkValidity()
                });
            });
            
            return JSON.stringify({
                forms_found: wizards.length,
                wizards
            });
        })()
    "#.to_string();

    let result = page
        .evaluate(js)
        .await
        .map_err(|e| Error::Cdp(format!("form_wizard_track: {e}")))?;
    let raw: String = result.into_value().unwrap_or_else(|_| "{}".to_string());
    Ok(serde_json::from_str(&raw).unwrap_or(serde_json::json!({})))
}

/// Wait for dynamic imports / code-split chunks to load
pub async fn dynamic_import_wait(page: &Page, module_pattern: &str, timeout_ms: u64) -> Result<serde_json::Value> {
    let js = format!(r#"
        new Promise((resolve) => {{
            const start = Date.now();
            const pattern = '{}';
            const timeout = {};
            
            // Monitor performance entries for script loading
            const check = () => {{
                const entries = performance.getEntriesByType('resource')
                    .filter(e => e.initiatorType === 'script' && e.name.includes(pattern));
                
                if (entries.length > 0) {{
                    resolve(JSON.stringify({{
                        loaded: true,
                        chunks: entries.map(e => ({{ url: e.name, duration: Math.round(e.duration), size: e.transferSize || 0 }})),
                        wait_ms: Date.now() - start
                    }}));
                    return;
                }}
                
                if (Date.now() - start > timeout) {{
                    // Return what we found even on timeout
                    const all = performance.getEntriesByType('resource')
                        .filter(e => e.initiatorType === 'script')
                        .map(e => e.name);
                    resolve(JSON.stringify({{
                        loaded: false,
                        available_scripts: all.slice(-20),
                        wait_ms: timeout
                    }}));
                    return;
                }}
                
                setTimeout(check, 200);
            }};
            check();
        }})
    "#, module_pattern.replace('\'', "\\'"), timeout_ms);

    let result = page
        .evaluate(js)
        .await
        .map_err(|e| Error::Cdp(format!("dynamic_import_wait: {e}")))?;
    let raw: String = result.into_value().unwrap_or_else(|_| "{}".to_string());
    Ok(serde_json::from_str(&raw).unwrap_or(serde_json::json!({})))
}

/// Execute multiple JS actions in parallel
pub async fn parallel_exec(page: &Page, actions: &[String]) -> Result<serde_json::Value> {
    let actions_json: Vec<String> = actions.iter().enumerate().map(|(i, a)| {
        format!(r#"
            (async () => {{
                try {{
                    const r = await (async () => {{ {} }})();
                    return {{ index: {}, status: 'fulfilled', value: r === undefined ? null : r }};
                }} catch(e) {{
                    return {{ index: {}, status: 'rejected', reason: e.message }};
                }}
            }})()
        "#, a, i, i)
    }).collect();

    let js = format!(r#"
        (async () => {{
            const results = await Promise.allSettled([{}]);
            return JSON.stringify(results.map((r, i) => ({{
                index: i,
                status: r.status,
                value: r.status === 'fulfilled' ? r.value : null,
                reason: r.status === 'rejected' ? r.reason : null
            }})));
        }})()
    "#, actions_json.join(","));

    let result = page
        .evaluate(js)
        .await
        .map_err(|e| Error::Cdp(format!("parallel_exec: {e}")))?;
    let raw: String = result.into_value().unwrap_or_else(|_| "[]".to_string());
    let parsed: serde_json::Value = serde_json::from_str(&raw).unwrap_or(serde_json::json!([]));

    Ok(serde_json::json!({
        "action": "parallel_exec",
        "total": actions.len(),
        "results": parsed
    }))
}
