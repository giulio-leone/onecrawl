use colored::Colorize;
use onecrawl_cdp::Page;

/// Run a browser command against the active session. Handles connect + page retrieval.
pub async fn with_page<F, Fut>(f: F)
where
    F: FnOnce(Page) -> Fut,
    Fut: std::future::Future<Output = Result<(), String>>,
{
    let (_session, page) = match super::session::connect_to_session().await {
        Ok(v) => v,
        Err(e) => {
            eprintln!("{} {e}", "✗".red());
            std::process::exit(1);
        }
    };
    if let Err(e) = f(page).await {
        eprintln!("{} {e}", "✗".red());
        std::process::exit(1);
    }
}

// ---------------------------------------------------------------------------
// Navigation
// ---------------------------------------------------------------------------

pub async fn navigate(url: &str, wait: u64) {
    with_page(|page| async move {
        onecrawl_cdp::navigation::goto(&page, url)
            .await
            .map_err(|e| e.to_string())?;
        if wait > 0 {
            onecrawl_cdp::navigation::wait_ms(wait).await;
        }
        println!("{} Navigated to {}", "✓".green(), url.cyan());
        Ok(())
    })
    .await;
}

pub async fn back() {
    with_page(|page| async move {
        onecrawl_cdp::navigation::go_back(&page)
            .await
            .map_err(|e| e.to_string())?;
        println!("{} Navigated back", "✓".green());
        Ok(())
    })
    .await;
}

pub async fn forward() {
    with_page(|page| async move {
        onecrawl_cdp::navigation::go_forward(&page)
            .await
            .map_err(|e| e.to_string())?;
        println!("{} Navigated forward", "✓".green());
        Ok(())
    })
    .await;
}

pub async fn reload() {
    with_page(|page| async move {
        onecrawl_cdp::navigation::reload(&page)
            .await
            .map_err(|e| e.to_string())?;
        println!("{} Page reloaded", "✓".green());
        Ok(())
    })
    .await;
}

// ---------------------------------------------------------------------------
// Content
// ---------------------------------------------------------------------------

pub async fn get(what: &str, selector: Option<&str>) {
    with_page(|page| async move {
        match what {
            "url" => {
                let url = onecrawl_cdp::navigation::get_url(&page)
                    .await
                    .map_err(|e| e.to_string())?;
                println!("{url}");
            }
            "title" => {
                let title = onecrawl_cdp::navigation::get_title(&page)
                    .await
                    .map_err(|e| e.to_string())?;
                println!("{title}");
            }
            "html" => {
                if let Some(sel) = selector {
                    let val = onecrawl_cdp::page::evaluate_js(
                        &page,
                        &format!(
                            "document.querySelector('{}')?.outerHTML || ''",
                            sel.replace('\'', "\\'")
                        ),
                    )
                    .await
                    .map_err(|e| e.to_string())?;
                    println!("{}", val.as_str().unwrap_or(&val.to_string()));
                } else {
                    let html = onecrawl_cdp::page::get_content(&page)
                        .await
                        .map_err(|e| e.to_string())?;
                    println!("{html}");
                }
            }
            "text" => {
                if let Some(sel) = selector {
                    let text = onecrawl_cdp::element::get_text(&page, sel)
                        .await
                        .map_err(|e| e.to_string())?;
                    println!("{text}");
                } else {
                    let val = onecrawl_cdp::page::evaluate_js(
                        &page,
                        "document.body?.innerText || ''",
                    )
                    .await
                    .map_err(|e| e.to_string())?;
                    println!("{}", val.as_str().unwrap_or(&val.to_string()));
                }
            }
            other => return Err(format!("Unknown target: {other}. Use: text, html, url, title")),
        }
        Ok(())
    })
    .await;
}

pub async fn eval(expression: &str) {
    with_page(|page| async move {
        let val = onecrawl_cdp::page::evaluate_js(&page, expression)
            .await
            .map_err(|e| e.to_string())?;
        match &val {
            serde_json::Value::String(s) => println!("{s}"),
            serde_json::Value::Null => println!("undefined"),
            other => println!("{}", serde_json::to_string_pretty(other).unwrap_or_default()),
        }
        Ok(())
    })
    .await;
}

pub async fn set_content(html: &str) {
    let html = html.to_string();
    with_page(|page| async move {
        onecrawl_cdp::page::set_content(&page, &html)
            .await
            .map_err(|e| e.to_string())?;
        println!("{} Content set", "✓".green());
        Ok(())
    })
    .await;
}

// ---------------------------------------------------------------------------
// Element Interaction
// ---------------------------------------------------------------------------

pub async fn click(selector: &str) {
    let sel = selector.to_string();
    with_page(|page| async move {
        onecrawl_cdp::element::click(&page, &sel)
            .await
            .map_err(|e| e.to_string())?;
        println!("{} Clicked {}", "✓".green(), sel.dimmed());
        Ok(())
    })
    .await;
}

pub async fn dblclick(selector: &str) {
    let sel = selector.to_string();
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
    let sel = selector.to_string();
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
    let sel = selector.to_string();
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
    let sel = selector.to_string();
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
    let sel = selector.to_string();
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
    let sel = selector.to_string();
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
    let sel = selector.to_string();
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
    let sel = selector.to_string();
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
    let sel = selector.to_string();
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
    let sel = selector.to_string();
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
    let sel = selector.to_string();
    let fp = file_path.to_string();
    with_page(|page| async move {
        onecrawl_cdp::input::set_file_input(&page, &sel, &[fp.clone()])
            .await
            .map_err(|e| e.to_string())?;
        println!("{} Uploaded {} to {}", "✓".green(), fp.dimmed(), sel.dimmed());
        Ok(())
    })
    .await;
}

pub async fn bounding_box(selector: &str) {
    let sel = selector.to_string();
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

// ---------------------------------------------------------------------------
// Keyboard
// ---------------------------------------------------------------------------

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
// Screenshot / PDF
// ---------------------------------------------------------------------------

pub async fn screenshot(
    output: &str,
    full: bool,
    element: Option<&str>,
    format: &str,
    quality: Option<u32>,
) {
    let out = output.to_string();
    let elem = element.map(String::from);
    let fmt = format.to_string();
    with_page(|page| async move {
        let bytes = if let Some(ref sel) = elem {
            onecrawl_cdp::screenshot::screenshot_element(&page, sel)
                .await
                .map_err(|e| e.to_string())?
        } else if fmt != "png" || quality.is_some() {
            let img_format = match fmt.as_str() {
                "jpeg" | "jpg" => onecrawl_cdp::ImageFormat::Jpeg,
                "webp" => onecrawl_cdp::ImageFormat::Webp,
                _ => onecrawl_cdp::ImageFormat::Png,
            };
            let opts = onecrawl_cdp::ScreenshotOptions {
                format: img_format,
                quality,
                full_page: full,
            };
            onecrawl_cdp::screenshot::screenshot_with_options(&page, &opts)
                .await
                .map_err(|e| e.to_string())?
        } else if full {
            onecrawl_cdp::screenshot::screenshot_full(&page)
                .await
                .map_err(|e| e.to_string())?
        } else {
            onecrawl_cdp::screenshot::screenshot_viewport(&page)
                .await
                .map_err(|e| e.to_string())?
        };
        std::fs::write(&out, &bytes).map_err(|e| format!("write failed: {e}"))?;
        println!(
            "{} Screenshot saved to {} ({} bytes)",
            "✓".green(),
            out.cyan(),
            bytes.len()
        );
        Ok(())
    })
    .await;
}

pub async fn pdf(output: &str, landscape: bool, scale: f64) {
    let out = output.to_string();
    with_page(|page| async move {
        let bytes = if landscape || (scale - 1.0).abs() > f64::EPSILON {
            let opts = onecrawl_cdp::PdfOptions {
                landscape,
                scale,
                ..Default::default()
            };
            onecrawl_cdp::screenshot::pdf_with_options(&page, &opts)
                .await
                .map_err(|e| e.to_string())?
        } else {
            onecrawl_cdp::screenshot::pdf(&page)
                .await
                .map_err(|e| e.to_string())?
        };
        std::fs::write(&out, &bytes).map_err(|e| format!("write failed: {e}"))?;
        println!(
            "{} PDF saved to {} ({} bytes)",
            "✓".green(),
            out.cyan(),
            bytes.len()
        );
        Ok(())
    })
    .await;
}

// ---------------------------------------------------------------------------
// Cookies
// ---------------------------------------------------------------------------

pub async fn cookie_get(name: Option<&str>, json: bool) {
    let name = name.map(String::from);
    with_page(|page| async move {
        let cookies = onecrawl_cdp::cookie::get_all_cookies(&page)
            .await
            .map_err(|e| e.to_string())?;
        let filtered: Vec<_> = if let Some(ref n) = name {
            cookies.into_iter().filter(|c| c.name == *n).collect()
        } else {
            cookies
        };
        if json {
            println!(
                "{}",
                serde_json::to_string_pretty(&filtered).unwrap_or_default()
            );
        } else {
            for c in &filtered {
                println!(
                    "{}={} (domain={}, path={}, secure={}, httpOnly={})",
                    c.name.green(),
                    c.value,
                    c.domain,
                    c.path,
                    c.secure,
                    c.http_only
                );
            }
            if filtered.is_empty() {
                println!("{}", "No cookies found".dimmed());
            }
        }
        Ok(())
    })
    .await;
}

pub async fn cookie_set(name: &str, value: &str, domain: Option<&str>, path: Option<&str>) {
    let params = onecrawl_cdp::SetCookieParams {
        name: name.to_string(),
        value: value.to_string(),
        domain: domain.map(String::from),
        path: path.map(String::from),
        expires: None,
        http_only: None,
        secure: None,
        same_site: None,
        url: None,
    };
    with_page(|page| async move {
        onecrawl_cdp::cookie::set_cookie(&page, &params)
            .await
            .map_err(|e| e.to_string())?;
        println!("{} Cookie '{}' set", "✓".green(), params.name);
        Ok(())
    })
    .await;
}

pub async fn cookie_delete(name: &str, domain: &str) {
    let n = name.to_string();
    let d = domain.to_string();
    with_page(|page| async move {
        onecrawl_cdp::cookie::delete_cookies(&page, &n, Some(&d), None)
            .await
            .map_err(|e| e.to_string())?;
        println!("{} Cookie '{}' deleted", "✓".green(), n);
        Ok(())
    })
    .await;
}

pub async fn cookie_clear() {
    with_page(|page| async move {
        onecrawl_cdp::cookie::clear_cookies(&page)
            .await
            .map_err(|e| e.to_string())?;
        println!("{} All cookies cleared", "✓".green());
        Ok(())
    })
    .await;
}

// ---------------------------------------------------------------------------
// Emulation
// ---------------------------------------------------------------------------

pub async fn emulate_viewport(width: u32, height: u32, scale: f64) {
    with_page(|page| async move {
        let vp = onecrawl_cdp::Viewport {
            width,
            height,
            device_scale_factor: scale,
            is_mobile: false,
            has_touch: false,
        };
        onecrawl_cdp::emulation::set_viewport(&page, &vp)
            .await
            .map_err(|e| e.to_string())?;
        println!(
            "{} Viewport set to {}×{} @{:.1}x",
            "✓".green(),
            width,
            height,
            scale
        );
        Ok(())
    })
    .await;
}

pub async fn emulate_device(name: &str) {
    let n = name.to_string();
    with_page(|page| async move {
        let vp = match n.as_str() {
            "iphone_14" | "iphone14" | "iphone" => onecrawl_cdp::Viewport::iphone_14(),
            "ipad" => onecrawl_cdp::Viewport::ipad(),
            "pixel_7" | "pixel7" | "pixel" => onecrawl_cdp::Viewport::pixel_7(),
            "desktop" => onecrawl_cdp::Viewport::desktop(),
            _ => {
                return Err(format!(
                    "Unknown device: {n}. Available: iphone_14, ipad, pixel_7, desktop"
                ))
            }
        };
        onecrawl_cdp::emulation::set_viewport(&page, &vp)
            .await
            .map_err(|e| e.to_string())?;
        println!(
            "{} Emulating {} ({}×{} @{:.1}x, mobile={}, touch={})",
            "✓".green(),
            n.cyan(),
            vp.width,
            vp.height,
            vp.device_scale_factor,
            vp.is_mobile,
            vp.has_touch
        );
        Ok(())
    })
    .await;
}

pub async fn emulate_user_agent(ua: &str) {
    let ua = ua.to_string();
    with_page(|page| async move {
        onecrawl_cdp::emulation::set_user_agent(&page, &ua)
            .await
            .map_err(|e| e.to_string())?;
        println!("{} User-Agent set", "✓".green());
        Ok(())
    })
    .await;
}

pub async fn emulate_geolocation(lat: f64, lon: f64, accuracy: f64) {
    with_page(|page| async move {
        onecrawl_cdp::emulation::set_geolocation(&page, lat, lon, accuracy)
            .await
            .map_err(|e| e.to_string())?;
        println!(
            "{} Geolocation set to ({}, {}) accuracy={}",
            "✓".green(),
            lat,
            lon,
            accuracy
        );
        Ok(())
    })
    .await;
}

pub async fn emulate_color_scheme(scheme: &str) {
    let s = scheme.to_string();
    with_page(|page| async move {
        onecrawl_cdp::emulation::set_color_scheme(&page, &s)
            .await
            .map_err(|e| e.to_string())?;
        println!("{} Color scheme set to {}", "✓".green(), s.cyan());
        Ok(())
    })
    .await;
}

pub async fn emulate_clear() {
    with_page(|page| async move {
        onecrawl_cdp::emulation::clear_viewport(&page)
            .await
            .map_err(|e| e.to_string())?;
        println!("{} Emulation cleared", "✓".green());
        Ok(())
    })
    .await;
}

// ---------------------------------------------------------------------------
// Network
// ---------------------------------------------------------------------------

pub async fn network_block(types: &str) {
    let types = types.to_string();
    with_page(|page| async move {
        let resource_types: Vec<onecrawl_cdp::ResourceType> = types
            .split(',')
            .filter_map(|t| match t.trim().to_lowercase().as_str() {
                "image" | "images" => Some(onecrawl_cdp::ResourceType::Image),
                "stylesheet" | "css" => Some(onecrawl_cdp::ResourceType::Stylesheet),
                "font" | "fonts" => Some(onecrawl_cdp::ResourceType::Font),
                "script" | "js" => Some(onecrawl_cdp::ResourceType::Script),
                "media" => Some(onecrawl_cdp::ResourceType::Media),
                "xhr" => Some(onecrawl_cdp::ResourceType::Xhr),
                "fetch" => Some(onecrawl_cdp::ResourceType::Fetch),
                "websocket" | "ws" => Some(onecrawl_cdp::ResourceType::WebSocket),
                "document" => Some(onecrawl_cdp::ResourceType::Document),
                _ => None,
            })
            .collect();
        if resource_types.is_empty() {
            return Err("No valid resource types. Use: image,stylesheet,font,script,media,xhr,fetch,websocket".into());
        }
        onecrawl_cdp::network::block_resources(&page, &resource_types)
            .await
            .map_err(|e| e.to_string())?;
        println!(
            "{} Blocking {} resource type(s)",
            "✓".green(),
            resource_types.len()
        );
        Ok(())
    })
    .await;
}

// ---------------------------------------------------------------------------
// HAR
// ---------------------------------------------------------------------------

pub async fn har_start() {
    with_page(|page| async move {
        let recorder = onecrawl_cdp::HarRecorder::new();
        onecrawl_cdp::har::start_har_recording(&page, &recorder)
            .await
            .map_err(|e| e.to_string())?;
        println!("{} HAR recording started", "✓".green());
        Ok(())
    })
    .await;
}

pub async fn har_drain() {
    with_page(|page| async move {
        let recorder = onecrawl_cdp::HarRecorder::new();
        let count = onecrawl_cdp::har::drain_har_entries(&page, &recorder)
            .await
            .map_err(|e| e.to_string())?;
        println!("{} Drained {} HAR entries", "✓".green(), count);
        Ok(())
    })
    .await;
}

pub async fn har_export(output: &str) {
    let out = output.to_string();
    with_page(|page| async move {
        let recorder = onecrawl_cdp::HarRecorder::new();
        // Start + drain to capture current entries
        let _ = onecrawl_cdp::har::start_har_recording(&page, &recorder).await;
        tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
        let _ = onecrawl_cdp::har::drain_har_entries(&page, &recorder).await;
        let url = onecrawl_cdp::navigation::get_url(&page)
            .await
            .unwrap_or_default();
        let har = onecrawl_cdp::har::export_har(&recorder, &url)
            .await
            .map_err(|e| e.to_string())?;
        let json = serde_json::to_string_pretty(&har).unwrap_or_default();
        std::fs::write(&out, &json).map_err(|e| format!("write failed: {e}"))?;
        println!("{} HAR exported to {}", "✓".green(), out.cyan());
        Ok(())
    })
    .await;
}

// ---------------------------------------------------------------------------
// WebSocket
// ---------------------------------------------------------------------------

pub async fn ws_start() {
    with_page(|page| async move {
        let recorder = onecrawl_cdp::WsRecorder::new();
        onecrawl_cdp::websocket::start_ws_recording(&page, &recorder)
            .await
            .map_err(|e| e.to_string())?;
        println!("{} WebSocket recording started", "✓".green());
        Ok(())
    })
    .await;
}

pub async fn ws_drain() {
    with_page(|page| async move {
        let recorder = onecrawl_cdp::WsRecorder::new();
        let count = onecrawl_cdp::websocket::drain_ws_frames(&page, &recorder)
            .await
            .map_err(|e| e.to_string())?;
        println!("{} Drained {} WebSocket frames", "✓".green(), count);
        Ok(())
    })
    .await;
}

pub async fn ws_export(output: &str) {
    let out = output.to_string();
    with_page(|page| async move {
        let recorder = onecrawl_cdp::WsRecorder::new();
        let _ = onecrawl_cdp::websocket::start_ws_recording(&page, &recorder).await;
        tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
        let _ = onecrawl_cdp::websocket::drain_ws_frames(&page, &recorder).await;
        let frames = onecrawl_cdp::websocket::export_ws_frames(&recorder)
            .await
            .map_err(|e| e.to_string())?;
        let json = serde_json::to_string_pretty(&frames).unwrap_or_default();
        std::fs::write(&out, &json).map_err(|e| format!("write failed: {e}"))?;
        println!("{} WebSocket frames exported to {}", "✓".green(), out.cyan());
        Ok(())
    })
    .await;
}

pub async fn ws_connections() {
    with_page(|page| async move {
        let count = onecrawl_cdp::websocket::active_ws_connections(&page)
            .await
            .map_err(|e| e.to_string())?;
        println!("{count}");
        Ok(())
    })
    .await;
}

// ---------------------------------------------------------------------------
// Coverage
// ---------------------------------------------------------------------------

pub async fn coverage_js_start() {
    with_page(|page| async move {
        onecrawl_cdp::coverage::start_js_coverage(&page)
            .await
            .map_err(|e| e.to_string())?;
        println!("{} JS coverage started", "✓".green());
        Ok(())
    })
    .await;
}

pub async fn coverage_js_stop() {
    with_page(|page| async move {
        let report = onecrawl_cdp::coverage::stop_js_coverage(&page)
            .await
            .map_err(|e| e.to_string())?;
        println!(
            "{}",
            serde_json::to_string_pretty(&report).unwrap_or_default()
        );
        Ok(())
    })
    .await;
}

pub async fn coverage_css_start() {
    with_page(|page| async move {
        onecrawl_cdp::coverage::start_css_coverage(&page)
            .await
            .map_err(|e| e.to_string())?;
        println!("{} CSS coverage started", "✓".green());
        Ok(())
    })
    .await;
}

pub async fn coverage_css_report() {
    with_page(|page| async move {
        let report = onecrawl_cdp::coverage::get_css_coverage(&page)
            .await
            .map_err(|e| e.to_string())?;
        println!(
            "{}",
            serde_json::to_string_pretty(&report).unwrap_or_default()
        );
        Ok(())
    })
    .await;
}

// ---------------------------------------------------------------------------
// Stealth
// ---------------------------------------------------------------------------

pub async fn stealth_inject() {
    with_page(|page| async move {
        let fp = onecrawl_cdp::generate_fingerprint();
        let script = onecrawl_cdp::get_stealth_init_script(&fp);
        onecrawl_cdp::page::evaluate_js(&page, &script)
            .await
            .map_err(|e| e.to_string())?;
        // Also override UA
        onecrawl_cdp::emulation::set_user_agent(&page, &fp.user_agent)
            .await
            .map_err(|e| e.to_string())?;
        println!("{} Stealth patches injected", "✓".green());
        println!("  UA: {}", fp.user_agent.dimmed());
        println!(
            "  Viewport: {}×{}",
            fp.viewport_width, fp.viewport_height
        );
        Ok(())
    })
    .await;
}

// ---------------------------------------------------------------------------
// Wait
// ---------------------------------------------------------------------------

pub async fn wait_ms(ms: u64) {
    onecrawl_cdp::navigation::wait_ms(ms).await;
    println!("{} Waited {}ms", "✓".green(), ms);
}

pub async fn wait_for_selector(selector: &str, timeout: u64) {
    let sel = selector.to_string();
    with_page(|page| async move {
        onecrawl_cdp::navigation::wait_for_selector(&page, &sel, timeout)
            .await
            .map_err(|e| e.to_string())?;
        println!("{} Selector '{}' found", "✓".green(), sel.dimmed());
        Ok(())
    })
    .await;
}

pub async fn wait_for_url(pattern: &str, timeout: u64) {
    let pat = pattern.to_string();
    with_page(|page| async move {
        onecrawl_cdp::navigation::wait_for_url(&page, &pat, timeout)
            .await
            .map_err(|e| e.to_string())?;
        println!("{} URL matched '{}'", "✓".green(), pat.dimmed());
        Ok(())
    })
    .await;
}

// ---------------------------------------------------------------------------
// Pages
// ---------------------------------------------------------------------------

pub async fn new_page(url: Option<&str>) {
    let url = url.unwrap_or("about:blank").to_string();
    let info = match super::session::load_session() {
        Some(i) => i,
        None => {
            eprintln!(
                "{} No active session. Run {} first.",
                "✗".red(),
                "onecrawl session start".yellow()
            );
            std::process::exit(1);
        }
    };
    match onecrawl_cdp::BrowserSession::connect(&info.ws_url).await {
        Ok(session) => match session.new_page(&url).await {
            Ok(_) => println!("{} New page opened: {}", "✓".green(), url.cyan()),
            Err(e) => {
                eprintln!("{} {e}", "✗".red());
                std::process::exit(1);
            }
        },
        Err(e) => {
            eprintln!("{} {e}", "✗".red());
            std::process::exit(1);
        }
    }
}
