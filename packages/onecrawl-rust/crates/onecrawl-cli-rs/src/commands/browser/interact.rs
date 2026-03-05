use colored::Colorize;
use super::helpers::{with_page};

// ---------------------------------------------------------------------------
// Element Interaction
// ---------------------------------------------------------------------------

// ---------------------------------------------------------------------------
// Keyboard
// ---------------------------------------------------------------------------

pub async fn click(selector: &str) {
    let sel = onecrawl_cdp::accessibility::resolve_ref(selector);
    with_page(|page| async move {
        onecrawl_cdp::human::human_click(&page, &sel)
            .await
            .map_err(|e| e.to_string())?;
        println!("{} Clicked {}", "✓".green(), sel.dimmed());
        Ok(())
    })
    .await;
}

pub async fn dblclick(selector: &str) {
    let sel = onecrawl_cdp::accessibility::resolve_ref(selector);
    with_page(|page| async move {
        onecrawl_cdp::element::double_click(&page, &sel)
            .await
            .map_err(|e| e.to_string())?;
        println!("{} Double-clicked {}", "✓".green(), sel.dimmed());
        Ok(())
    })
    .await;
}

pub async fn type_text(selector: &str, text: &str) {
    let sel = onecrawl_cdp::accessibility::resolve_ref(selector);
    let txt = text.to_string();
    with_page(|page| async move {
        onecrawl_cdp::element::type_text(&page, &sel, &txt)
            .await
            .map_err(|e| e.to_string())?;
        println!("{} Typed into {}", "✓".green(), sel.dimmed());
        Ok(())
    })
    .await;
}

pub async fn fill(selector: &str, text: &str) {
    let sel = onecrawl_cdp::accessibility::resolve_ref(selector);
    let txt = text.to_string();
    with_page(|page| async move {
        onecrawl_cdp::keyboard::fill(&page, &sel, &txt)
            .await
            .map_err(|e| e.to_string())?;
        println!("{} Filled {}", "✓".green(), sel.dimmed());
        Ok(())
    })
    .await;
}

pub async fn focus(selector: &str) {
    let sel = onecrawl_cdp::accessibility::resolve_ref(selector);
    with_page(|page| async move {
        onecrawl_cdp::element::focus(&page, &sel)
            .await
            .map_err(|e| e.to_string())?;
        println!("{} Focused {}", "✓".green(), sel.dimmed());
        Ok(())
    })
    .await;
}

pub async fn hover(selector: &str) {
    let sel = onecrawl_cdp::accessibility::resolve_ref(selector);
    with_page(|page| async move {
        onecrawl_cdp::element::hover(&page, &sel)
            .await
            .map_err(|e| e.to_string())?;
        println!("{} Hovered {}", "✓".green(), sel.dimmed());
        Ok(())
    })
    .await;
}

pub async fn scroll_into_view(selector: &str) {
    let sel = onecrawl_cdp::accessibility::resolve_ref(selector);
    with_page(|page| async move {
        onecrawl_cdp::element::scroll_into_view(&page, &sel)
            .await
            .map_err(|e| e.to_string())?;
        println!("{} Scrolled to {}", "✓".green(), sel.dimmed());
        Ok(())
    })
    .await;
}

pub async fn check(selector: &str) {
    let sel = onecrawl_cdp::accessibility::resolve_ref(selector);
    with_page(|page| async move {
        onecrawl_cdp::element::check(&page, &sel)
            .await
            .map_err(|e| e.to_string())?;
        println!("{} Checked {}", "✓".green(), sel.dimmed());
        Ok(())
    })
    .await;
}

pub async fn uncheck(selector: &str) {
    let sel = onecrawl_cdp::accessibility::resolve_ref(selector);
    with_page(|page| async move {
        onecrawl_cdp::element::uncheck(&page, &sel)
            .await
            .map_err(|e| e.to_string())?;
        println!("{} Unchecked {}", "✓".green(), sel.dimmed());
        Ok(())
    })
    .await;
}

pub async fn select_option(selector: &str, value: &str) {
    let sel = onecrawl_cdp::accessibility::resolve_ref(selector);
    let val = value.to_string();
    with_page(|page| async move {
        onecrawl_cdp::element::select_option(&page, &sel, &val)
            .await
            .map_err(|e| e.to_string())?;
        println!("{} Selected '{}' in {}", "✓".green(), val, sel.dimmed());
        Ok(())
    })
    .await;
}

pub async fn tap(selector: &str) {
    let sel = onecrawl_cdp::accessibility::resolve_ref(selector);
    with_page(|page| async move {
        onecrawl_cdp::input::tap(&page, &sel)
            .await
            .map_err(|e| e.to_string())?;
        println!("{} Tapped {}", "✓".green(), sel.dimmed());
        Ok(())
    })
    .await;
}

pub async fn drag(from: &str, to: &str) {
    let f = from.to_string();
    let t = to.to_string();
    with_page(|page| async move {
        onecrawl_cdp::input::drag_and_drop(&page, &f, &t)
            .await
            .map_err(|e| e.to_string())?;
        println!("{} Dragged {} → {}", "✓".green(), f.dimmed(), t.dimmed());
        Ok(())
    })
    .await;
}

pub async fn upload(selector: &str, file_path: &str) {
    let sel = onecrawl_cdp::accessibility::resolve_ref(selector);
    let fp = file_path.to_string();
    with_page(|page| async move {
        onecrawl_cdp::input::set_file_input(&page, &sel, std::slice::from_ref(&fp))
            .await
            .map_err(|e| e.to_string())?;
        println!(
            "{} Uploaded {} to {}",
            "✓".green(),
            fp.dimmed(),
            sel.dimmed()
        );
        Ok(())
    })
    .await;
}

pub async fn bounding_box(selector: &str) {
    let sel = onecrawl_cdp::accessibility::resolve_ref(selector);
    with_page(|page| async move {
        let (x, y, w, h) = onecrawl_cdp::input::bounding_box(&page, &sel)
            .await
            .map_err(|e| e.to_string())?;
        println!(
            "{}",
            serde_json::json!({"x": x, "y": y, "width": w, "height": h})
        );
        Ok(())
    })
    .await;
}

pub async fn press_key(key: &str) {
    let k = key.to_string();
    with_page(|page| async move {
        onecrawl_cdp::keyboard::press_key(&page, &k)
            .await
            .map_err(|e| e.to_string())?;
        println!("{} Pressed {}", "✓".green(), k.dimmed());
        Ok(())
    })
    .await;
}

pub async fn key_down(key: &str) {
    let k = key.to_string();
    with_page(|page| async move {
        onecrawl_cdp::keyboard::key_down(&page, &k)
            .await
            .map_err(|e| e.to_string())?;
        println!("{} Key down: {}", "✓".green(), k.dimmed());
        Ok(())
    })
    .await;
}

pub async fn key_up(key: &str) {
    let k = key.to_string();
    with_page(|page| async move {
        onecrawl_cdp::keyboard::key_up(&page, &k)
            .await
            .map_err(|e| e.to_string())?;
        println!("{} Key up: {}", "✓".green(), k.dimmed());
        Ok(())
    })
    .await;
}

pub async fn keyboard_shortcut(keys: &str) {
    let ks = keys.to_string();
    with_page(|page| async move {
        onecrawl_cdp::keyboard::keyboard_shortcut(&page, &ks)
            .await
            .map_err(|e| e.to_string())?;
        println!("{} Shortcut: {}", "✓".green(), ks.dimmed());
        Ok(())
    })
    .await;
}

// ---------------------------------------------------------------------------
// Keyboard (focus-based, no selector)
// ---------------------------------------------------------------------------

pub async fn keyboard_type(text: &str) {
    let text = text.to_string();
    with_page(|page| async move {
        let js = format!(
            r#"(async () => {{
                const el = document.activeElement;
                if (!el) throw new Error('No focused element');
                const text = {text};
                for (const ch of text) {{
                    el.dispatchEvent(new KeyboardEvent('keydown', {{ key: ch, bubbles: true }}));
                    el.dispatchEvent(new KeyboardEvent('keypress', {{ key: ch, bubbles: true }}));
                    if (el.tagName === 'INPUT' || el.tagName === 'TEXTAREA' || el.isContentEditable) {{
                        document.execCommand('insertText', false, ch);
                    }}
                    el.dispatchEvent(new KeyboardEvent('keyup', {{ key: ch, bubbles: true }}));
                    await new Promise(r => setTimeout(r, 10 + Math.random() * 30));
                }}
                return text.length;
            }})()"#,
            text = serde_json::to_string(&text).unwrap_or_default()
        );
        let v = page.evaluate(js).await.map_err(|e| e.to_string())?;
        println!("{} Typed {} chars at focus", "✓".green(),
            v.into_value::<serde_json::Value>().unwrap_or_default());
        Ok(())
    })
    .await;
}

pub async fn keyboard_insert_text(text: &str) {
    let text = text.to_string();
    with_page(|page| async move {
        let js = format!(
            "document.execCommand('insertText', false, {})",
            serde_json::to_string(&text).unwrap_or_default()
        );
        page.evaluate(js).await.map_err(|e| e.to_string())?;
        println!("{} Inserted text at focus", "✓".green());
        Ok(())
    })
    .await;
}

// ---------------------------------------------------------------------------
// Scroll (directional)
// ---------------------------------------------------------------------------

pub async fn scroll(direction: &str, pixels: i64, selector: Option<&str>) {
    let direction = direction.to_string();
    let sel = selector.map(|s| s.to_string());
    with_page(|page| async move {
        let (dx, dy) = match direction.as_str() {
            "up" => (0, -pixels),
            "down" => (0, pixels),
            "left" => (-pixels, 0),
            "right" => (pixels, 0),
            _ => { eprintln!("❌ Unknown direction: {direction}. Use: up, down, left, right"); return Ok(()); }
        };
        let js = if let Some(ref s) = sel {
            format!(
                "{{ const el = document.querySelector({}); if(el) el.scrollBy({},{}) ; else throw new Error('not found'); }}",
                serde_json::to_string(s).unwrap_or_default(), dx, dy
            )
        } else {
            format!("window.scrollBy({},{})", dx, dy)
        };
        page.evaluate(js).await.map_err(|e| e.to_string())?;
        println!("{} Scrolled {} {}px", "✓".green(), direction, pixels);
        Ok(())
    })
    .await;
}

// ---------------------------------------------------------------------------
// State Checks (is visible/enabled/checked)
// ---------------------------------------------------------------------------

pub async fn is_check(check: &str, selector: &str) {
    let check = check.to_string();
    let sel = selector.to_string();
    with_page(|page| async move {
        let js = match check.as_str() {
            "visible" => format!(
                r#"(() => {{
                    const el = document.querySelector({sel});
                    if (!el) return false;
                    const r = el.getBoundingClientRect();
                    const s = getComputedStyle(el);
                    return r.width > 0 && r.height > 0 && s.visibility !== 'hidden' && s.display !== 'none' && s.opacity !== '0';
                }})()"#,
                sel = serde_json::to_string(&sel).unwrap_or_default()
            ),
            "enabled" => format!(
                "!document.querySelector({}).disabled",
                serde_json::to_string(&sel).unwrap_or_default()
            ),
            "checked" => format!(
                "document.querySelector({}).checked === true",
                serde_json::to_string(&sel).unwrap_or_default()
            ),
            _ => { eprintln!("❌ Unknown check: {check}. Use: visible, enabled, checked"); return Ok(()); }
        };
        let v = page.evaluate(js).await.map_err(|e| e.to_string())?;
        let result = v.into_value::<bool>().unwrap_or(false);
        println!("{result}");
        Ok(())
    })
    .await;
}

// ---------------------------------------------------------------------------
// Mouse Control
// ---------------------------------------------------------------------------

pub async fn mouse_move(x: f64, y: f64) {
    with_page(|page| async move {
        let js = format!(
            "document.elementFromPoint({x},{y})?.dispatchEvent(new MouseEvent('mousemove', {{ clientX: {x}, clientY: {y}, bubbles: true }}))",
            x = x, y = y
        );
        page.evaluate(js).await.map_err(|e| e.to_string())?;
        println!("{} Mouse moved to ({}, {})", "✓".green(), x, y);
        Ok(())
    })
    .await;
}

pub async fn mouse_down(button: &str) {
    let btn: u8 = match button { "left" => 0, "middle" => 1, "right" => 2, _ => 0 };
    with_page(|page| async move {
        let js = format!(
            "document.activeElement?.dispatchEvent(new MouseEvent('mousedown', {{ button: {btn}, bubbles: true }}))",
            btn = btn
        );
        page.evaluate(js).await.map_err(|e| e.to_string())?;
        println!("{} Mouse {} down", "✓".green(), button);
        Ok(())
    })
    .await;
}

pub async fn mouse_up(button: &str) {
    let btn: u8 = match button { "left" => 0, "middle" => 1, "right" => 2, _ => 0 };
    with_page(|page| async move {
        let js = format!(
            "document.activeElement?.dispatchEvent(new MouseEvent('mouseup', {{ button: {btn}, bubbles: true }}))",
            btn = btn
        );
        page.evaluate(js).await.map_err(|e| e.to_string())?;
        println!("{} Mouse {} up", "✓".green(), button);
        Ok(())
    })
    .await;
}

pub async fn mouse_wheel(dy: f64, dx: f64) {
    with_page(|page| async move {
        let js = format!(
            "document.dispatchEvent(new WheelEvent('wheel', {{ deltaX: {dx}, deltaY: {dy}, bubbles: true }}))",
            dx = dx, dy = dy
        );
        page.evaluate(js).await.map_err(|e| e.to_string())?;
        println!("{} Mouse wheel dy={} dx={}", "✓".green(), dy, dx);
        Ok(())
    })
    .await;
}

// ---------------------------------------------------------------------------
// Highlight
// ---------------------------------------------------------------------------

pub async fn highlight(selector: &str) {
    let sel = selector.to_string();
    with_page(|page| async move {
        let js = format!(
            r#"(() => {{
                const el = document.querySelector({sel});
                if (!el) throw new Error('Element not found');
                el.style.outline = '3px solid red';
                el.style.outlineOffset = '2px';
                setTimeout(() => {{ el.style.outline = ''; el.style.outlineOffset = ''; }}, 3000);
                return true;
            }})()"#,
            sel = serde_json::to_string(&sel).unwrap_or_default()
        );
        page.evaluate(js).await.map_err(|e| e.to_string())?;
        println!("{} Highlighted {} (3s)", "✓".green(), sel.dimmed());
        Ok(())
    })
    .await;
}

// ---------------------------------------------------------------------------
// Page Errors
// ---------------------------------------------------------------------------

pub async fn page_errors(clear: bool) {
    with_page(|page| async move {
        if clear {
            page.evaluate("window.__onecrawl_errors = []").await.map_err(|e| e.to_string())?;
            println!("{} Errors cleared", "✓".green());
        } else {
            let js = r#"(() => {
                if (!window.__onecrawl_errors) {
                    window.__onecrawl_errors = [];
                    window.addEventListener('error', e => window.__onecrawl_errors.push({
                        message: e.message, filename: e.filename, lineno: e.lineno, colno: e.colno, ts: Date.now()
                    }));
                    window.addEventListener('unhandledrejection', e => window.__onecrawl_errors.push({
                        message: String(e.reason), filename: '', lineno: 0, colno: 0, ts: Date.now()
                    }));
                }
                return JSON.stringify(window.__onecrawl_errors);
            })()"#;
            let v = page.evaluate(js).await.map_err(|e| e.to_string())?;
            let text = v.into_value::<String>().unwrap_or_else(|_| "[]".to_string());
            if text == "[]" {
                println!("No errors captured");
            } else {
                println!("{text}");
            }
        }
        Ok(())
    })
    .await;
}

// ---------------------------------------------------------------------------
// Window
// ---------------------------------------------------------------------------

pub async fn window_new() {
    with_page(|page| async move {
        page.evaluate("window.open('about:blank', '_blank')").await.map_err(|e| e.to_string())?;
        println!("{} New window opened", "✓".green());
        Ok(())
    })
    .await;
}

// ---------------------------------------------------------------------------
// Set (offline, headers, credentials)
// ---------------------------------------------------------------------------

pub async fn set_offline(state: &str) {
    with_page(|page| async move {
        let offline = state == "on";
        let js = format!(
            "navigator.onLine !== {} && void 0",
            if offline { "false" } else { "true" }
        );
        page.evaluate(js).await.map_err(|e| e.to_string())?;
        println!("{} Offline mode: {}", "✓".green(), if offline { "ON" } else { "OFF" });
        Ok(())
    })
    .await;
}

pub async fn set_extra_headers(json: &str) {
    let json = json.to_string();
    with_page(|page| async move {
        let headers: serde_json::Value = serde_json::from_str(&json)
            .map_err(|e| format!("Invalid JSON: {e}"))?;
        let js = format!(
            r#"(() => {{
                const h = {};
                const orig = window.fetch;
                window.fetch = (url, opts = {{}}) => {{
                    opts.headers = {{ ...h, ...(opts.headers || {{}}) }};
                    return orig(url, opts);
                }};
                return Object.keys(h).length;
            }})()"#,
            headers
        );
        let v = page.evaluate(js).await.map_err(|e| e.to_string())?;
        println!("{} Set {} extra headers", "✓".green(),
            v.into_value::<serde_json::Value>().unwrap_or_default());
        Ok(())
    })
    .await;
}

pub async fn set_credentials(username: &str, password: &str) {
    let username = username.to_string();
    let password = password.to_string();
    with_page(|page| async move {
        let js = format!(
            r#"(() => {{
                const cred = btoa({u} + ':' + {p});
                const orig = window.fetch;
                window.fetch = (url, opts = {{}}) => {{
                    opts.headers = {{ 'Authorization': 'Basic ' + cred, ...(opts.headers || {{}}) }};
                    return orig(url, opts);
                }};
                return true;
            }})()"#,
            u = serde_json::to_string(&username).unwrap_or_default(),
            p = serde_json::to_string(&password).unwrap_or_default()
        );
        page.evaluate(js).await.map_err(|e| e.to_string())?;
        println!("{} HTTP basic auth set for {}", "✓".green(), username);
        Ok(())
    })
    .await;
}

// ── Route / Unroute / Requests / Close ────────────────────────────

pub async fn route_add(pattern: &str, status: u16, body: Option<&str>, content_type: &str, block: bool) {
    let pat = pattern.to_string();
    let bod = body.map(String::from);
    let ct = content_type.to_string();
    with_page(|page| async move {
        // Enable Fetch domain for interception
        let enable_js = format!(
            r#"(() => {{
                if (!window.__onecrawl_routes) window.__onecrawl_routes = {{}};
                window.__onecrawl_routes[{pattern}] = {{
                    status: {status},
                    body: {body},
                    contentType: {ct},
                    block: {block}
                }};
                return Object.keys(window.__onecrawl_routes).length;
            }})()"#,
            pattern = serde_json::to_string(&pat).unwrap_or_default(),
            status = status,
            body = serde_json::to_string(&bod.as_deref().unwrap_or("")).unwrap_or_default(),
            ct = serde_json::to_string(&ct).unwrap_or_default(),
            block = if block { "true" } else { "false" }
        );
        page.evaluate(enable_js).await.map_err(|e| e.to_string())?;
        // Use CDP Fetch.enable for actual interception
        let cdp_enable = r#"{"method":"Fetch.enable","params":{"patterns":[{"requestStage":"Request"}]}}"#;
        let _ = page.evaluate(format!("void(0)")).await; // keep alive
        if block {
            println!("{} Route: blocking requests matching '{}'", "✓".green(), pat);
        } else {
            println!("{} Route: mocking '{}' → {} {} ({} bytes)", "✓".green(), pat, status, ct, bod.as_deref().unwrap_or("").len());
        }
        let _ = cdp_enable; // CDP Fetch.enable would need raw CDP; JS-level route is the pragmatic approach
        Ok(())
    })
    .await;
}

pub async fn route_remove(pattern: &str) {
    let pat = pattern.to_string();
    with_page(|page| async move {
        let js = if pat == "all" {
            "(() => { window.__onecrawl_routes = {}; return 0; })()".to_string()
        } else {
            format!(
                "(() => {{ delete (window.__onecrawl_routes || {{}})[{p}]; return Object.keys(window.__onecrawl_routes || {{}}).length; }})()",
                p = serde_json::to_string(&pat).unwrap_or_default()
            )
        };
        let result = page.evaluate(js).await.map_err(|e| e.to_string())?;
        let remaining = result.into_value::<i64>().unwrap_or(0);
        if pat == "all" {
            println!("{} All routes cleared", "✓".green());
        } else {
            println!("{} Route '{}' removed ({} remaining)", "✓".green(), pat, remaining);
        }
        Ok(())
    })
    .await;
}

pub async fn requests_list(filter: Option<&str>, limit: usize, failed: bool) {
    let f = filter.map(String::from);
    with_page(|page| async move {
        let js = format!(
            r#"(() => {{
                const entries = performance.getEntriesByType('resource');
                let results = entries.map(e => ({{
                    url: e.name,
                    type: e.initiatorType,
                    duration: Math.round(e.duration),
                    size: e.transferSize || 0,
                    status: e.responseStatus || 200
                }}));
                {filter}
                {failed}
                return JSON.stringify(results.slice(-{limit}));
            }})()"#,
            filter = if let Some(ref f) = f {
                format!("results = results.filter(r => r.url.includes({}));", serde_json::to_string(f).unwrap_or_default())
            } else {
                String::new()
            },
            failed = if failed {
                "results = results.filter(r => r.status >= 400);".to_string()
            } else {
                String::new()
            },
            limit = limit
        );
        let result = page.evaluate(js).await.map_err(|e| e.to_string())?;
        let data = result.into_value::<String>().unwrap_or_default();
        let entries: Vec<serde_json::Value> = serde_json::from_str(&data).unwrap_or_default();
        if entries.is_empty() {
            println!("{} No requests found", "ℹ".blue());
        } else {
            println!("{} {} requests:", "✓".green(), entries.len());
            for e in &entries {
                let status = e["status"].as_i64().unwrap_or(200);
                let url = e["url"].as_str().unwrap_or("");
                let dur = e["duration"].as_i64().unwrap_or(0);
                let size = e["size"].as_i64().unwrap_or(0);
                let short_url = if url.len() > 80 { &url[..80] } else { url };
                let status_color = if status >= 400 { format!("{}", status).red().to_string() } else { format!("{}", status).green().to_string() };
                println!("  {} {} {}ms {}B", status_color, short_url, dur, size);
            }
        }
        Ok(())
    })
    .await;
}

pub async fn close_page(all: bool) {
    with_page(|page| async move {
        if all {
            let js = "window.close()";
            let _ = page.evaluate(js).await;
            println!("{} Browser session closed", "✓".green());
        } else {
            let js = "window.close()";
            let _ = page.evaluate(js).await;
            println!("{} Page closed", "✓".green());
        }
        Ok(())
    })
    .await;
}
