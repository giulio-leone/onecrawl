use colored::Colorize;
use super::helpers::with_page;

// ---------------------------------------------------------------------------
// SPA commands
// ---------------------------------------------------------------------------

pub async fn spa_nav_watch() {
    with_page(|page| async move {
        let js = r#"
        (() => {
            const events = [];
            const orig_push = history.pushState;
            const orig_replace = history.replaceState;
            history.pushState = function(...args) {
                events.push({ type: 'pushState', url: args[2], ts: Date.now() });
                return orig_push.apply(this, args);
            };
            history.replaceState = function(...args) {
                events.push({ type: 'replaceState', url: args[2], ts: Date.now() });
                return orig_replace.apply(this, args);
            };
            window.addEventListener('hashchange', (e) => {
                events.push({ type: 'hashchange', oldURL: e.oldURL, newURL: e.newURL, ts: Date.now() });
            });
            window.addEventListener('popstate', () => {
                events.push({ type: 'popstate', url: location.href, ts: Date.now() });
            });
            window.__onecrawl_nav_events = events;
            return 'installed';
        })()
        "#;
        let result = page.evaluate(js).await.map_err(|e| e.to_string())?;
        let val: String = result.into_value().unwrap_or_default();
        println!("{} SPA navigation watcher {}", "✓".green(), val);
        Ok(())
    })
    .await;
}

pub async fn framework_detect() {
    with_page(|page| async move {
        let js = r#"
        (() => {
            const detected = {};
            if (window.__NEXT_DATA__) detected.nextjs = window.__NEXT_DATA__.buildId || true;
            if (window.__NUXT__) detected.nuxt = true;
            if (document.querySelector('[data-reactroot], [id="__next"]')) detected.react = true;
            if (document.querySelector('[ng-version]')) {
                detected.angular = document.querySelector('[ng-version]').getAttribute('ng-version');
            }
            if (window.__VUE__) detected.vue = true;
            if (document.querySelector('[data-svelte-h]')) detected.svelte = true;
            if (window.__GATSBY) detected.gatsby = true;
            if (window.__remixContext) detected.remix = true;
            if (window.$nuxt) detected.nuxt = window.$nuxt.$options?._app?.version || true;
            if (document.querySelector('ember-application')) detected.ember = true;
            if (Object.keys(detected).length === 0) detected.unknown = true;
            return detected;
        })()
        "#;
        let result = page.evaluate(js).await.map_err(|e| e.to_string())?;
        let val: serde_json::Value = result.into_value().unwrap_or_default();
        println!("{}", serde_json::to_string_pretty(&val).unwrap_or_default());
        Ok(())
    })
    .await;
}

pub async fn virtual_scroll_detect() {
    with_page(|page| async move {
        let val = onecrawl_cdp::spa::detect_virtual_scroll(&page)
            .await
            .map_err(|e| e.to_string())?;
        println!("{}", serde_json::to_string_pretty(&val).unwrap_or_default());
        Ok(())
    })
    .await;
}

pub async fn virtual_scroll_extract(container: &str, item: &str, max: usize) {
    let container = container.to_owned();
    let item = item.to_owned();
    with_page(|page| async move {
        let items = onecrawl_cdp::spa::extract_virtual_scroll(&page, &container, &item, max)
            .await
            .map_err(|e| e.to_string())?;
        let val = serde_json::json!({ "count": items.len(), "items": items });
        println!("{}", serde_json::to_string_pretty(&val).unwrap_or_default());
        Ok(())
    })
    .await;
}

pub async fn wait_hydration(timeout: u64) {
    with_page(|page| async move {
        let status = onecrawl_cdp::spa::wait_hydration(&page, timeout)
            .await
            .map_err(|e| e.to_string())?;
        println!("{} Hydration: {}", "✓".green(), status);
        Ok(())
    })
    .await;
}

pub async fn wait_animation(selector: &str, timeout: u64) {
    let selector = selector.to_owned();
    with_page(|page| async move {
        let done = onecrawl_cdp::spa::wait_animations(&page, &selector, timeout)
            .await
            .map_err(|e| e.to_string())?;
        if done {
            println!("{} Animations completed", "✓".green());
        } else {
            println!("{} Animations timed out", "⚠".yellow());
        }
        Ok(())
    })
    .await;
}

pub async fn trigger_lazy_load(selector: Option<&str>) {
    let sel = selector
        .unwrap_or("img[data-src], img[loading='lazy']")
        .to_owned();
    with_page(|page| async move {
        let count = onecrawl_cdp::spa::trigger_lazy_load(&page, &sel)
            .await
            .map_err(|e| e.to_string())?;
        println!("{} Triggered lazy load for {} elements", "✓".green(), count);
        Ok(())
    })
    .await;
}

pub async fn wait_network_idle(idle_ms: u64, timeout: u64) {
    with_page(|page| async move {
        let idle = onecrawl_cdp::spa::wait_network_idle(&page, idle_ms, timeout)
            .await
            .map_err(|e| e.to_string())?;
        if idle {
            println!("{} Network idle", "✓".green());
        } else {
            println!("{} Network idle timed out", "⚠".yellow());
        }
        Ok(())
    })
    .await;
}

pub async fn state_inspect(path: Option<&str>) {
    let path = path.map(|s| s.to_owned());
    with_page(|page| async move {
        let val = onecrawl_cdp::spa::state_inspect(&page, path.as_deref())
            .await
            .map_err(|e| e.to_string())?;
        println!("{}", serde_json::to_string_pretty(&val).unwrap_or_default());
        Ok(())
    })
    .await;
}

pub async fn form_wizard_track() {
    with_page(|page| async move {
        let val = onecrawl_cdp::spa::form_wizard_track(&page)
            .await
            .map_err(|e| e.to_string())?;
        println!("{}", serde_json::to_string_pretty(&val).unwrap_or_default());
        Ok(())
    })
    .await;
}

pub async fn dynamic_import_wait(pattern: &str, timeout: u64) {
    let pattern = pattern.to_owned();
    with_page(|page| async move {
        let val = onecrawl_cdp::spa::dynamic_import_wait(&page, &pattern, timeout)
            .await
            .map_err(|e| e.to_string())?;
        println!("{}", serde_json::to_string_pretty(&val).unwrap_or_default());
        Ok(())
    })
    .await;
}

pub async fn parallel_exec(actions: &[String]) {
    let actions = actions.to_vec();
    with_page(|page| async move {
        let val = onecrawl_cdp::spa::parallel_exec(&page, &actions)
            .await
            .map_err(|e| e.to_string())?;
        println!("{}", serde_json::to_string_pretty(&val).unwrap_or_default());
        Ok(())
    })
    .await;
}

// ---------------------------------------------------------------------------
// Harness commands
// ---------------------------------------------------------------------------

pub async fn health_check() {
    with_page(|page| async move {
        let val = onecrawl_cdp::harness::health_check(&page)
            .await
            .map_err(|e| e.to_string())?;
        println!("{}", serde_json::to_string_pretty(&val).unwrap_or_default());
        Ok(())
    })
    .await;
}

pub async fn circuit_breaker(command: &str, error: Option<&str>) {
    let command = command.to_owned();
    let error = error.map(|s| s.to_owned());
    with_page(|page| async move {
        let js = format!(
            r#"
            (() => {{
                if (!window.__onecrawl_cb) {{
                    window.__onecrawl_cb = {{
                        state: 'closed',
                        failures: 0,
                        successes: 0,
                        last_failure: null,
                        threshold: 5
                    }};
                }}
                const cb = window.__onecrawl_cb;
                switch ('{command}') {{
                    case 'status':
                        return cb;
                    case 'record_success':
                        cb.successes++;
                        if (cb.state === 'half-open') cb.state = 'closed';
                        cb.failures = 0;
                        return cb;
                    case 'record_failure':
                        cb.failures++;
                        cb.last_failure = {error_json};
                        if (cb.failures >= cb.threshold) cb.state = 'open';
                        return cb;
                    case 'reset':
                        cb.state = 'closed';
                        cb.failures = 0;
                        cb.successes = 0;
                        cb.last_failure = null;
                        return cb;
                    default:
                        return {{ error: 'unknown command: {command}' }};
                }}
            }})()
            "#,
            command = command,
            error_json = error
                .as_deref()
                .map(|e| format!("\"{}\"", e.replace('"', "\\\"")))
                .unwrap_or_else(|| "null".to_string()),
        );
        let result = page.evaluate(js).await.map_err(|e| e.to_string())?;
        let val: serde_json::Value = result.into_value().unwrap_or_default();
        println!("{}", serde_json::to_string_pretty(&val).unwrap_or_default());
        Ok(())
    })
    .await;
}

pub async fn reconnect_cdp(retries: usize) {
    with_page(|page| async move {
        let val = onecrawl_cdp::harness::reconnect_cdp(&page, retries)
            .await
            .map_err(|e| e.to_string())?;
        println!("{}", serde_json::to_string_pretty(&val).unwrap_or_default());
        Ok(())
    })
    .await;
}

pub async fn gc_tabs() {
    with_page(|page| async move {
        let val = onecrawl_cdp::harness::gc_tabs_info(&page)
            .await
            .map_err(|e| e.to_string())?;
        println!("{}", serde_json::to_string_pretty(&val).unwrap_or_default());
        Ok(())
    })
    .await;
}

pub async fn watchdog() {
    with_page(|page| async move {
        let val = onecrawl_cdp::harness::watchdog_status(&page)
            .await
            .map_err(|e| e.to_string())?;
        println!("{}", serde_json::to_string_pretty(&val).unwrap_or_default());
        Ok(())
    })
    .await;
}
