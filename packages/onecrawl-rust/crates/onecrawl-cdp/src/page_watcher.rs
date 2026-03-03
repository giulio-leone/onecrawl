//! Page state change watcher via JS event listeners + MutationObserver.
//!
//! Watches for navigation (pushState/replaceState/popstate), title changes,
//! scroll, and resize events, recording them into `window.__onecrawl_page_changes`.

use chromiumoxide::Page;
use onecrawl_core::Result;
use serde::{Deserialize, Serialize};

/// A recorded page state change.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PageChange {
    pub change_type: String,
    pub old_value: String,
    pub new_value: String,
    pub timestamp: f64,
}

/// Install watchers for pushState/replaceState/popstate, title MutationObserver,
/// scroll, and resize events.
pub async fn start_page_watcher(page: &Page) -> Result<()> {
    let js = r#"
        (() => {
            if (window.__onecrawl_page_watcher_active) return 'already';
            window.__onecrawl_page_watcher_active = true;
            window.__onecrawl_page_changes = [];

            let lastUrl = location.href;
            let lastTitle = document.title;
            let lastScrollX = window.scrollX;
            let lastScrollY = window.scrollY;
            let lastWidth = window.innerWidth;
            let lastHeight = window.innerHeight;

            function pushChange(type, oldVal, newVal) {
                window.__onecrawl_page_changes.push({
                    change_type: type,
                    old_value: String(oldVal),
                    new_value: String(newVal),
                    timestamp: Date.now()
                });
            }

            // ── Navigation: pushState / replaceState ────────────────
            const origPush = history.pushState.bind(history);
            const origReplace = history.replaceState.bind(history);

            history.pushState = function(...args) {
                const oldUrl = lastUrl;
                const result = origPush(...args);
                lastUrl = location.href;
                if (oldUrl !== lastUrl) pushChange('navigation', oldUrl, lastUrl);
                return result;
            };
            history.replaceState = function(...args) {
                const oldUrl = lastUrl;
                const result = origReplace(...args);
                lastUrl = location.href;
                if (oldUrl !== lastUrl) pushChange('url', oldUrl, lastUrl);
                return result;
            };

            window.__onecrawl_orig_pushState = origPush;
            window.__onecrawl_orig_replaceState = origReplace;

            // ── Navigation: popstate ────────────────────────────────
            window.addEventListener('popstate', () => {
                const oldUrl = lastUrl;
                lastUrl = location.href;
                if (oldUrl !== lastUrl) pushChange('navigation', oldUrl, lastUrl);
            });

            // ── Title: MutationObserver on <title> ──────────────────
            const titleEl = document.querySelector('title');
            if (titleEl) {
                window.__onecrawl_title_observer = new MutationObserver(() => {
                    const newTitle = document.title;
                    if (newTitle !== lastTitle) {
                        pushChange('title', lastTitle, newTitle);
                        lastTitle = newTitle;
                    }
                });
                window.__onecrawl_title_observer.observe(titleEl, {
                    childList: true,
                    characterData: true,
                    subtree: true
                });
            }

            // ── Scroll ──────────────────────────────────────────────
            let scrollTimer = null;
            window.addEventListener('scroll', () => {
                clearTimeout(scrollTimer);
                scrollTimer = setTimeout(() => {
                    const newX = window.scrollX;
                    const newY = window.scrollY;
                    const old = lastScrollX + ',' + lastScrollY;
                    const cur = newX + ',' + newY;
                    if (old !== cur) {
                        pushChange('scroll', old, cur);
                        lastScrollX = newX;
                        lastScrollY = newY;
                    }
                }, 150);
            }, { passive: true });

            // ── Resize ──────────────────────────────────────────────
            let resizeTimer = null;
            window.addEventListener('resize', () => {
                clearTimeout(resizeTimer);
                resizeTimer = setTimeout(() => {
                    const newW = window.innerWidth;
                    const newH = window.innerHeight;
                    const old = lastWidth + 'x' + lastHeight;
                    const cur = newW + 'x' + newH;
                    if (old !== cur) {
                        pushChange('resize', old, cur);
                        lastWidth = newW;
                        lastHeight = newH;
                    }
                }, 150);
            }, { passive: true });

            return 'installed';
        })()
    "#;

    page.evaluate(js)
        .await
        .map_err(|e| onecrawl_core::Error::Browser(format!("start_page_watcher failed: {e}")))?;

    Ok(())
}

/// Drain accumulated page changes.
pub async fn drain_page_changes(page: &Page) -> Result<Vec<PageChange>> {
    let result = page
        .evaluate(
            r#"
            (() => {
                const changes = window.__onecrawl_page_changes || [];
                window.__onecrawl_page_changes = [];
                return changes;
            })()
            "#,
        )
        .await
        .map_err(|e| onecrawl_core::Error::Browser(format!("drain_page_changes failed: {e}")))?;

    let changes: Vec<PageChange> = result.into_value().unwrap_or_default();
    Ok(changes)
}

/// Stop the page watcher and clean up.
pub async fn stop_page_watcher(page: &Page) -> Result<()> {
    page.evaluate(
        r#"
        (() => {
            if (window.__onecrawl_title_observer) {
                window.__onecrawl_title_observer.disconnect();
                window.__onecrawl_title_observer = null;
            }
            if (window.__onecrawl_orig_pushState) {
                history.pushState = window.__onecrawl_orig_pushState;
                delete window.__onecrawl_orig_pushState;
            }
            if (window.__onecrawl_orig_replaceState) {
                history.replaceState = window.__onecrawl_orig_replaceState;
                delete window.__onecrawl_orig_replaceState;
            }
            window.__onecrawl_page_watcher_active = false;
        })()
        "#,
    )
    .await
    .map_err(|e| onecrawl_core::Error::Browser(format!("stop_page_watcher failed: {e}")))?;

    Ok(())
}

/// Get a snapshot of the current page state.
pub async fn get_page_state(page: &Page) -> Result<serde_json::Value> {
    let result = page
        .evaluate(
            r#"
            (() => {
                const timing = performance.timing || {};
                const navStart = timing.navigationStart || 0;
                return {
                    url: location.href,
                    title: document.title,
                    ready_state: document.readyState,
                    scroll_x: window.scrollX,
                    scroll_y: window.scrollY,
                    viewport_width: window.innerWidth,
                    viewport_height: window.innerHeight,
                    document_width: document.documentElement.scrollWidth,
                    document_height: document.documentElement.scrollHeight,
                    element_count: document.querySelectorAll('*').length,
                    image_count: document.images.length,
                    link_count: document.links.length,
                    form_count: document.forms.length,
                    performance_timing: {
                        dom_content_loaded: timing.domContentLoadedEventEnd
                            ? timing.domContentLoadedEventEnd - navStart : 0,
                        load_event: timing.loadEventEnd
                            ? timing.loadEventEnd - navStart : 0,
                        dom_interactive: timing.domInteractive
                            ? timing.domInteractive - navStart : 0
                    }
                };
            })()
            "#,
        )
        .await
        .map_err(|e| onecrawl_core::Error::Browser(format!("get_page_state failed: {e}")))?;

    let state: serde_json::Value = result.into_value().unwrap_or_default();
    Ok(state)
}
