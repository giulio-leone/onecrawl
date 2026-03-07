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
