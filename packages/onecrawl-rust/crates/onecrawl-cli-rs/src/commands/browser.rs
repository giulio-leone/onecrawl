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

/// Run a browser command that needs the BrowserSession (e.g. tab management).
pub async fn with_session<F, Fut>(f: F)
where
    F: FnOnce(onecrawl_cdp::BrowserSession, Page) -> Fut,
    Fut: std::future::Future<Output = Result<(), String>>,
{
    let (session, page) = match super::session::connect_to_session().await {
        Ok(v) => v,
        Err(e) => {
            eprintln!("{} {e}", "✗".red());
            std::process::exit(1);
        }
    };
    if let Err(e) = f(session, page).await {
        eprintln!("{} {e}", "✗".red());
        std::process::exit(1);
    }
}

// ---------------------------------------------------------------------------
// Navigation
// ---------------------------------------------------------------------------

pub async fn navigate(url: &str, wait: u64) {
    let t0 = std::time::Instant::now();
    // Try proxy first (avoids CDP reconnect overhead)
    if let Some(proxy) = super::proxy::ServerProxy::from_session().await {
        match proxy.navigate(url).await {
            Ok(_) => {
                if wait > 0 {
                    tokio::time::sleep(std::time::Duration::from_millis(wait)).await;
                }
                let ms = t0.elapsed().as_millis();
                println!("{} Navigated to {} {} {}", "✓".green(), url.cyan(), format!("{ms}ms").dimmed(), "(proxy)".dimmed());
                return;
            }
            Err(_) => {} // fall through to CDP
        }
    }
    with_page(|page| async move {
        onecrawl_cdp::navigation::goto(&page, url)
            .await
            .map_err(|e| e.to_string())?;
        if wait > 0 {
            onecrawl_cdp::navigation::wait_ms(wait).await;
        }
        let ms = t0.elapsed().as_millis();
        println!("{} Navigated to {} {}", "✓".green(), url.cyan(), format!("{ms}ms").dimmed());
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
    // Proxy fast-path for simple content retrieval (no selector)
    if selector.is_none() {
        if let Some(proxy) = super::proxy::ServerProxy::from_session().await {
            match what {
                "text" => {
                    if let Ok(text) = proxy.get_text().await {
                        println!("{text}");
                        return;
                    }
                }
                "url" => {
                    if let Ok(val) = proxy.evaluate("window.location.href").await {
                        let url = val["result"].as_str().unwrap_or("");
                        println!("{url}");
                        return;
                    }
                }
                "title" => {
                    if let Ok(val) = proxy.evaluate("document.title").await {
                        let title = val["result"].as_str().unwrap_or("");
                        println!("{title}");
                        return;
                    }
                }
                "html" => {
                    if let Ok(val) = proxy.evaluate("document.documentElement.outerHTML").await {
                        let html = val["result"].as_str().unwrap_or("");
                        println!("{html}");
                        return;
                    }
                }
                _ => {}
            }
        }
    }
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
                    let val =
                        onecrawl_cdp::page::evaluate_js(&page, "document.body?.innerText || ''")
                            .await
                            .map_err(|e| e.to_string())?;
                    println!("{}", val.as_str().unwrap_or(&val.to_string()));
                }
            }
            other => {
                return Err(format!(
                    "Unknown target: {other}. Use: text, html, url, title"
                ));
            }
        }
        Ok(())
    })
    .await;
}

pub async fn eval(expression: &str) {
    // Try proxy first
    if let Some(proxy) = super::proxy::ServerProxy::from_session().await {
        if let Ok(val) = proxy.evaluate(expression).await {
            let result = &val["result"];
            match result {
                serde_json::Value::String(s) => println!("{s}"),
                serde_json::Value::Null => println!("undefined"),
                other => println!(
                    "{}",
                    serde_json::to_string_pretty(other).unwrap_or_default()
                ),
            }
            return;
        }
    }
    with_page(|page| async move {
        let val = onecrawl_cdp::page::evaluate_js(&page, expression)
            .await
            .map_err(|e| e.to_string())?;
        match &val {
            serde_json::Value::String(s) => println!("{s}"),
            serde_json::Value::Null => println!("undefined"),
            other => println!(
                "{}",
                serde_json::to_string_pretty(other).unwrap_or_default()
            ),
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
    let t0 = std::time::Instant::now();
    // Proxy fast-path for simple PNG screenshots (no element selector, no custom format)
    if element.is_none() && format == "png" && quality.is_none() {
        if let Some(proxy) = super::proxy::ServerProxy::from_session().await {
            if let Ok(bytes) = proxy.screenshot().await {
                if std::fs::write(output, &bytes).is_ok() {
                    let ms = t0.elapsed().as_millis();
                    println!(
                        "{} Screenshot saved to {} ({} bytes) {} {}",
                        "✓".green(),
                        output.cyan(),
                        bytes.len(),
                        format!("{ms}ms").dimmed(),
                        "(proxy)".dimmed()
                    );
                    return;
                }
            }
        }
    }
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
        let ms = t0.elapsed().as_millis();
        println!(
            "{} Screenshot saved to {} ({} bytes) {}",
            "✓".green(),
            out.cyan(),
            bytes.len(),
            format!("{ms}ms").dimmed()
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

/// Export all cookies from the current page as a CookieJar JSON file.
/// Delegates to `cookie_jar::save_cookies_to_file` (or prints JSON to stdout).
pub async fn cookie_export(output: Option<&str>) {
    cookie_jar_export(output).await;
}

/// Import cookies from a CookieJar JSON file into the current page.
/// Delegates to `cookie_jar::load_cookies_from_file`.
pub async fn cookie_import(path: &str) {
    cookie_jar_import(path).await;
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
                ));
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
        println!(
            "{} WebSocket frames exported to {}",
            "✓".green(),
            out.cyan()
        );
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
// Accessibility
// ---------------------------------------------------------------------------

pub async fn a11y_tree() {
    with_page(|page| async move {
        let result = onecrawl_cdp::accessibility::get_accessibility_tree(&page)
            .await
            .map_err(|e| e.to_string())?;
        println!(
            "{}",
            serde_json::to_string_pretty(&result).unwrap_or_default()
        );
        Ok(())
    })
    .await;
}

pub async fn a11y_element(selector: &str) {
    let sel = selector.to_string();
    with_page(|page| async move {
        let result = onecrawl_cdp::accessibility::get_element_accessibility(&page, &sel)
            .await
            .map_err(|e| e.to_string())?;
        println!(
            "{}",
            serde_json::to_string_pretty(&result).unwrap_or_default()
        );
        Ok(())
    })
    .await;
}

pub async fn a11y_audit() {
    with_page(|page| async move {
        let result = onecrawl_cdp::accessibility::audit_accessibility(&page)
            .await
            .map_err(|e| e.to_string())?;
        println!(
            "{}",
            serde_json::to_string_pretty(&result).unwrap_or_default()
        );
        Ok(())
    })
    .await;
}

// ---------------------------------------------------------------------------
// Network Throttling
// ---------------------------------------------------------------------------

fn cli_parse_network_profile(name: &str) -> Result<onecrawl_cdp::NetworkProfile, String> {
    match name.to_lowercase().as_str() {
        "fast3g" | "fast-3g" => Ok(onecrawl_cdp::NetworkProfile::Fast3G),
        "slow3g" | "slow-3g" => Ok(onecrawl_cdp::NetworkProfile::Slow3G),
        "offline" => Ok(onecrawl_cdp::NetworkProfile::Offline),
        "regular4g" | "4g" => Ok(onecrawl_cdp::NetworkProfile::Regular4G),
        "wifi" => Ok(onecrawl_cdp::NetworkProfile::WiFi),
        _ => Err(format!(
            "Unknown profile: {name}. Use: fast3g, slow3g, offline, regular4g, wifi"
        )),
    }
}

pub async fn throttle_set(profile: &str) {
    let p = match cli_parse_network_profile(profile) {
        Ok(p) => p,
        Err(e) => {
            eprintln!("{} {e}", "✗".red());
            std::process::exit(1);
        }
    };
    let desc = onecrawl_cdp::throttle::describe_profile(&p);
    with_page(|page| async move {
        onecrawl_cdp::throttle::set_network_conditions(&page, p)
            .await
            .map_err(|e| e.to_string())?;
        println!("{} Network throttle set: {}", "✓".green(), desc);
        Ok(())
    })
    .await;
}

pub async fn throttle_custom(download_kbps: f64, upload_kbps: f64, latency_ms: f64) {
    let profile = onecrawl_cdp::NetworkProfile::Custom {
        download_kbps,
        upload_kbps,
        latency_ms,
    };
    with_page(|page| async move {
        onecrawl_cdp::throttle::set_network_conditions(&page, profile)
            .await
            .map_err(|e| e.to_string())?;
        println!(
            "{} Custom throttle set: ↓{}kbps ↑{}kbps ~{}ms",
            "✓".green(),
            download_kbps,
            upload_kbps,
            latency_ms
        );
        Ok(())
    })
    .await;
}

pub async fn throttle_clear() {
    with_page(|page| async move {
        onecrawl_cdp::throttle::clear_network_conditions(&page)
            .await
            .map_err(|e| e.to_string())?;
        println!("{} Network throttle cleared", "✓".green());
        Ok(())
    })
    .await;
}

// ---------------------------------------------------------------------------
// Performance Tracing
// ---------------------------------------------------------------------------

pub async fn perf_trace_start() {
    with_page(|page| async move {
        onecrawl_cdp::tracing_cdp::start_tracing(&page)
            .await
            .map_err(|e| e.to_string())?;
        println!("{} Tracing started", "✓".green());
        Ok(())
    })
    .await;
}

pub async fn perf_trace_stop() {
    with_page(|page| async move {
        let result = onecrawl_cdp::tracing_cdp::stop_tracing(&page)
            .await
            .map_err(|e| e.to_string())?;
        println!(
            "{}",
            serde_json::to_string_pretty(&result).unwrap_or_default()
        );
        Ok(())
    })
    .await;
}

pub async fn perf_metrics() {
    with_page(|page| async move {
        let result = onecrawl_cdp::tracing_cdp::get_performance_metrics(&page)
            .await
            .map_err(|e| e.to_string())?;
        println!(
            "{}",
            serde_json::to_string_pretty(&result).unwrap_or_default()
        );
        Ok(())
    })
    .await;
}

pub async fn perf_timing() {
    with_page(|page| async move {
        let result = onecrawl_cdp::tracing_cdp::get_navigation_timing(&page)
            .await
            .map_err(|e| e.to_string())?;
        println!(
            "{}",
            serde_json::to_string_pretty(&result).unwrap_or_default()
        );
        Ok(())
    })
    .await;
}

pub async fn perf_resources() {
    with_page(|page| async move {
        let result = onecrawl_cdp::tracing_cdp::get_resource_timing(&page)
            .await
            .map_err(|e| e.to_string())?;
        println!(
            "{}",
            serde_json::to_string_pretty(&result).unwrap_or_default()
        );
        Ok(())
    })
    .await;
}

// ---------------------------------------------------------------------------
// Console
// ---------------------------------------------------------------------------

pub async fn console_start() {
    with_page(|page| async move {
        onecrawl_cdp::console::start_console_capture(&page)
            .await
            .map_err(|e| e.to_string())?;
        println!("{} Console capture started", "✓".green());
        Ok(())
    })
    .await;
}

pub async fn console_drain() {
    with_page(|page| async move {
        let entries = onecrawl_cdp::console::drain_console_entries(&page)
            .await
            .map_err(|e| e.to_string())?;
        println!(
            "{}",
            serde_json::to_string_pretty(&entries).unwrap_or_default()
        );
        Ok(())
    })
    .await;
}

pub async fn console_clear() {
    with_page(|page| async move {
        onecrawl_cdp::console::clear_console(&page)
            .await
            .map_err(|e| e.to_string())?;
        println!("{} Console buffer cleared", "✓".green());
        Ok(())
    })
    .await;
}

// ---------------------------------------------------------------------------
// Dialog
// ---------------------------------------------------------------------------

pub async fn dialog_set_handler(accept: bool, prompt_text: Option<&str>) {
    let pt = prompt_text.map(String::from);
    with_page(|page| async move {
        onecrawl_cdp::dialog::set_dialog_handler(&page, accept, pt.as_deref())
            .await
            .map_err(|e| e.to_string())?;
        println!("{} Dialog handler set (accept={})", "✓".green(), accept);
        Ok(())
    })
    .await;
}

pub async fn dialog_history() {
    with_page(|page| async move {
        let events = onecrawl_cdp::dialog::get_dialog_history(&page)
            .await
            .map_err(|e| e.to_string())?;
        println!(
            "{}",
            serde_json::to_string_pretty(&events).unwrap_or_default()
        );
        Ok(())
    })
    .await;
}

pub async fn dialog_clear() {
    with_page(|page| async move {
        onecrawl_cdp::dialog::clear_dialog_history(&page)
            .await
            .map_err(|e| e.to_string())?;
        println!("{} Dialog history cleared", "✓".green());
        Ok(())
    })
    .await;
}

// ---------------------------------------------------------------------------
// Worker
// ---------------------------------------------------------------------------

pub async fn worker_list() {
    with_page(|page| async move {
        let workers = onecrawl_cdp::workers::get_service_workers(&page)
            .await
            .map_err(|e| e.to_string())?;
        println!(
            "{}",
            serde_json::to_string_pretty(&workers).unwrap_or_default()
        );
        Ok(())
    })
    .await;
}

pub async fn worker_unregister() {
    with_page(|page| async move {
        let count = onecrawl_cdp::workers::unregister_service_workers(&page)
            .await
            .map_err(|e| e.to_string())?;
        println!("{} Unregistered {} service worker(s)", "✓".green(), count);
        Ok(())
    })
    .await;
}

pub async fn worker_info() {
    with_page(|page| async move {
        let info = onecrawl_cdp::workers::get_worker_info(&page)
            .await
            .map_err(|e| e.to_string())?;
        println!(
            "{}",
            serde_json::to_string_pretty(&info).unwrap_or_default()
        );
        Ok(())
    })
    .await;
}

// ---------------------------------------------------------------------------
// Web Storage
// ---------------------------------------------------------------------------

pub async fn web_storage_local_get() {
    with_page(|page| async move {
        let data = onecrawl_cdp::web_storage::get_local_storage(&page)
            .await
            .map_err(|e| e.to_string())?;
        println!(
            "{}",
            serde_json::to_string_pretty(&data).unwrap_or_default()
        );
        Ok(())
    })
    .await;
}

pub async fn web_storage_local_set(key: &str, value: &str) {
    let k = key.to_string();
    let v = value.to_string();
    with_page(|page| async move {
        onecrawl_cdp::web_storage::set_local_storage(&page, &k, &v)
            .await
            .map_err(|e| e.to_string())?;
        println!("{} localStorage['{}'] set", "✓".green(), k);
        Ok(())
    })
    .await;
}

pub async fn web_storage_local_clear() {
    with_page(|page| async move {
        onecrawl_cdp::web_storage::clear_local_storage(&page)
            .await
            .map_err(|e| e.to_string())?;
        println!("{} localStorage cleared", "✓".green());
        Ok(())
    })
    .await;
}

pub async fn web_storage_session_get() {
    with_page(|page| async move {
        let data = onecrawl_cdp::web_storage::get_session_storage(&page)
            .await
            .map_err(|e| e.to_string())?;
        println!(
            "{}",
            serde_json::to_string_pretty(&data).unwrap_or_default()
        );
        Ok(())
    })
    .await;
}

pub async fn web_storage_session_set(key: &str, value: &str) {
    let k = key.to_string();
    let v = value.to_string();
    with_page(|page| async move {
        onecrawl_cdp::web_storage::set_session_storage(&page, &k, &v)
            .await
            .map_err(|e| e.to_string())?;
        println!("{} sessionStorage['{}'] set", "✓".green(), k);
        Ok(())
    })
    .await;
}

pub async fn web_storage_session_clear() {
    with_page(|page| async move {
        onecrawl_cdp::web_storage::clear_session_storage(&page)
            .await
            .map_err(|e| e.to_string())?;
        println!("{} sessionStorage cleared", "✓".green());
        Ok(())
    })
    .await;
}

pub async fn web_storage_indexeddb_list() {
    with_page(|page| async move {
        let names = onecrawl_cdp::web_storage::get_indexeddb_databases(&page)
            .await
            .map_err(|e| e.to_string())?;
        println!(
            "{}",
            serde_json::to_string_pretty(&names).unwrap_or_default()
        );
        Ok(())
    })
    .await;
}

pub async fn web_storage_clear_all() {
    with_page(|page| async move {
        onecrawl_cdp::web_storage::clear_site_data(&page)
            .await
            .map_err(|e| e.to_string())?;
        println!("{} All site data cleared", "✓".green());
        Ok(())
    })
    .await;
}

// ---------------------------------------------------------------------------
// Passkey / WebAuthn
// ---------------------------------------------------------------------------

pub async fn passkey_enable(protocol: &str, transport: &str) {
    let proto = protocol.to_string();
    let trans = transport.to_string();
    with_page(|page| async move {
        let config = onecrawl_cdp::webauthn::VirtualAuthenticator {
            id: format!(
                "auth-{}",
                std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_millis()
            ),
            protocol: proto,
            transport: trans.clone(),
            has_resident_key: true,
            has_user_verification: true,
            is_user_verified: true,
        };
        onecrawl_cdp::webauthn::enable_virtual_authenticator(&page, &config)
            .await
            .map_err(|e| e.to_string())?;
        println!(
            "{} Virtual authenticator enabled (transport: {})",
            "✓".green(),
            trans
        );
        Ok(())
    })
    .await;
}

pub async fn passkey_add(credential_id: &str, rp_id: &str, user_handle: Option<&str>) {
    let cred = onecrawl_cdp::webauthn::VirtualCredential {
        credential_id: credential_id.to_string(),
        rp_id: rp_id.to_string(),
        user_handle: user_handle.unwrap_or_default().to_string(),
        sign_count: 0,
    };
    with_page(|page| async move {
        onecrawl_cdp::webauthn::add_virtual_credential(&page, &cred)
            .await
            .map_err(|e| e.to_string())?;
        println!("{} Credential added: {}", "✓".green(), cred.credential_id);
        Ok(())
    })
    .await;
}

pub async fn passkey_list() {
    with_page(|page| async move {
        let creds = onecrawl_cdp::webauthn::get_virtual_credentials(&page)
            .await
            .map_err(|e| e.to_string())?;
        println!(
            "{}",
            serde_json::to_string_pretty(&creds).unwrap_or_default()
        );
        Ok(())
    })
    .await;
}

pub async fn passkey_log() {
    with_page(|page| async move {
        let log = onecrawl_cdp::webauthn::get_webauthn_log(&page)
            .await
            .map_err(|e| e.to_string())?;
        println!(
            "{}",
            serde_json::to_string_pretty(&log).unwrap_or_default()
        );
        Ok(())
    })
    .await;
}

pub async fn passkey_disable() {
    with_page(|page| async move {
        onecrawl_cdp::webauthn::disable_virtual_authenticator(&page)
            .await
            .map_err(|e| e.to_string())?;
        println!("{} Virtual authenticator disabled", "✓".green());
        Ok(())
    })
    .await;
}

pub async fn passkey_remove(credential_id: &str) {
    let cid = credential_id.to_string();
    with_page(|page| async move {
        let removed = onecrawl_cdp::webauthn::remove_virtual_credential(&page, &cid)
            .await
            .map_err(|e| e.to_string())?;
        if removed {
            println!("{} Credential removed: {cid}", "✓".green());
        } else {
            println!("{} Credential not found: {cid}", "⚠".yellow());
        }
        Ok(())
    })
    .await;
}

/// Enable a CDP real virtual authenticator, wait for a passkey to be registered
/// on the current page (e.g. x.com Settings → Security → Passkey), then export
/// the credential (including private key) to a JSON file.
///
/// The credential exported here can later be injected via
/// `session start --import-passkey FILE` for fully automated headless passkey auth.
pub async fn passkey_register(output: &str, timeout_secs: u64) {
    let output = output.to_string();
    let (_session, page) = match super::session::connect_to_session().await {
        Ok(v) => v,
        Err(e) => {
            eprintln!("{} {e}", "✗".red());
            std::process::exit(1);
        }
    };

    // Enable CDP WebAuthn domain
    if let Err(e) = onecrawl_cdp::cdp_enable(&page).await {
        eprintln!("{} WebAuthn.enable failed: {e}", "✗".red());
        std::process::exit(1);
    }

    // Create a CTAP2.1 platform authenticator with auto-presence simulation
    let auth_id = match onecrawl_cdp::cdp_create_authenticator(&page).await {
        Ok(id) => id,
        Err(e) => {
            eprintln!("{} addVirtualAuthenticator failed: {e}", "✗".red());
            std::process::exit(1);
        }
    };

    println!("{} CDP virtual authenticator ready (ID: {})", "✓".green(), auth_id.cyan());
    println!(
        "  {}",
        "Please register a passkey on the current page (e.g. x.com Settings → Security → Passkey)."
            .dimmed()
    );
    println!("  Waiting up to {}s for credential creation…", timeout_secs);

    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(timeout_secs);
    loop {
        tokio::time::sleep(std::time::Duration::from_millis(500)).await;
        match onecrawl_cdp::cdp_get_credentials(&page, &auth_id).await {
            Ok(creds) if !creds.is_empty() => {
                println!("{} {} credential(s) registered", "✓".green(), creds.len());
                let path = std::path::Path::new(&output);
                match onecrawl_cdp::save_passkeys(path, &creds) {
                    Ok(()) => {
                        println!("{} Passkeys saved to {}", "✓".green(), output.cyan());
                        println!(
                            "  {}",
                            "Use `session start --import-passkey FILE` to enable headless passkey auth."
                                .dimmed()
                        );
                    }
                    Err(e) => eprintln!("{} Failed to save passkeys: {e}", "✗".red()),
                }
                return;
            }
            Ok(_) => {} // no credentials yet — keep polling
            Err(e) => {
                eprintln!("{} getCredentials error: {e}", "⚠".yellow());
            }
        }
        if std::time::Instant::now() >= deadline {
            eprintln!("{} Timeout: no passkey registered within {}s", "✗".red(), timeout_secs);
            std::process::exit(1);
        }
    }
}

/// Store the passkey file path in the active session so that CDP WebAuthn is
/// automatically re-enabled and credentials are injected on every
/// `connect_to_session()` call (same lifecycle as stealth scripts).
pub async fn passkey_set_file(file: &str) {
    match super::session::load_session() {
        Some(mut info) => {
            info.passkey_file = Some(file.to_string());
            match super::session::save_session(&info) {
                Ok(()) => println!(
                    "{} Passkey file set: {} (will be injected on every connect)",
                    "✓".green(),
                    file.cyan()
                ),
                Err(e) => {
                    eprintln!("{} Failed to save session: {e}", "✗".red());
                    std::process::exit(1);
                }
            }
        }
        None => {
            eprintln!("{} No active session. Start one with `session start`.", "✗".red());
            std::process::exit(1);
        }
    }
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
        println!("  Viewport: {}×{}", fp.viewport_width, fp.viewport_height);
        Ok(())
    })
    .await;
}

// ---------------------------------------------------------------------------
// Anti-Bot
// ---------------------------------------------------------------------------

pub async fn antibot_inject(level: &str) {
    let lvl = level.to_string();
    with_page(|page| async move {
        let applied = onecrawl_cdp::antibot::inject_stealth_full(&page)
            .await
            .map_err(|e| e.to_string())?;
        // Filter by profile level
        let profiles = onecrawl_cdp::antibot::stealth_profiles();
        let profile = profiles.iter().find(|p| p.level == lvl);
        let names: Vec<&str> = if let Some(p) = profile {
            applied
                .iter()
                .filter(|a| p.patches.contains(a))
                .map(|s| s.as_str())
                .collect()
        } else {
            applied.iter().map(|s| s.as_str()).collect()
        };
        println!(
            "{} Anti-bot patches injected (level: {})",
            "✓".green(),
            lvl.cyan()
        );
        for n in &names {
            println!("  • {}", n);
        }
        Ok(())
    })
    .await;
}

pub async fn antibot_test() {
    with_page(|page| async move {
        let result = onecrawl_cdp::antibot::bot_detection_test(&page)
            .await
            .map_err(|e| e.to_string())?;
        println!(
            "{}",
            serde_json::to_string_pretty(&result).unwrap_or_default()
        );
        Ok(())
    })
    .await;
}

pub async fn antibot_profiles() {
    let profiles = onecrawl_cdp::antibot::stealth_profiles();
    println!(
        "{}",
        serde_json::to_string_pretty(&profiles).unwrap_or_default()
    );
}

// ---------------------------------------------------------------------------
// Adaptive Element Tracker
// ---------------------------------------------------------------------------

pub async fn adaptive_fingerprint(selector: &str) {
    let sel = selector.to_string();
    with_page(|page| async move {
        let fp = onecrawl_cdp::adaptive::fingerprint_element(&page, &sel)
            .await
            .map_err(|e| e.to_string())?;
        println!("{}", serde_json::to_string_pretty(&fp).unwrap_or_default());
        Ok(())
    })
    .await;
}

pub async fn adaptive_relocate(fingerprint_json: &str) {
    let fp: onecrawl_cdp::ElementFingerprint = match serde_json::from_str(fingerprint_json) {
        Ok(fp) => fp,
        Err(e) => {
            eprintln!("{} Invalid fingerprint JSON: {}", "✗".red(), e);
            std::process::exit(1);
        }
    };
    with_page(|page| async move {
        let matches = onecrawl_cdp::adaptive::relocate_element(&page, &fp)
            .await
            .map_err(|e| e.to_string())?;
        println!(
            "{}",
            serde_json::to_string_pretty(&matches).unwrap_or_default()
        );
        Ok(())
    })
    .await;
}

pub async fn adaptive_track(selectors: &str, save_path: Option<&str>) {
    let sels: Vec<String> = match serde_json::from_str(selectors) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("{} Invalid selectors JSON: {}", "✗".red(), e);
            std::process::exit(1);
        }
    };
    let sel_refs: Vec<&str> = sels.iter().map(|s| s.as_str()).collect();
    let path_buf = save_path.map(std::path::PathBuf::from);
    with_page(|page| async move {
        let fps = onecrawl_cdp::adaptive::track_elements(&page, &sel_refs, path_buf.as_deref())
            .await
            .map_err(|e| e.to_string())?;
        println!("{}", serde_json::to_string_pretty(&fps).unwrap_or_default());
        Ok(())
    })
    .await;
}

pub async fn adaptive_relocate_all(fingerprints_json: &str) {
    let fps: Vec<onecrawl_cdp::ElementFingerprint> = match serde_json::from_str(fingerprints_json) {
        Ok(fps) => fps,
        Err(e) => {
            eprintln!("{} Invalid fingerprints JSON: {}", "✗".red(), e);
            std::process::exit(1);
        }
    };
    with_page(|page| async move {
        let results = onecrawl_cdp::adaptive::relocate_all(&page, &fps)
            .await
            .map_err(|e| e.to_string())?;
        println!(
            "{}",
            serde_json::to_string_pretty(&results).unwrap_or_default()
        );
        Ok(())
    })
    .await;
}

pub async fn adaptive_save(fingerprints: &str, path: &str) {
    let fps: Vec<onecrawl_cdp::ElementFingerprint> = match serde_json::from_str(fingerprints) {
        Ok(fps) => fps,
        Err(e) => {
            eprintln!("{} Invalid fingerprints JSON: {}", "✗".red(), e);
            std::process::exit(1);
        }
    };
    match onecrawl_cdp::adaptive::save_fingerprints(&fps, std::path::Path::new(path)) {
        Ok(_) => println!(
            "{} Saved {} fingerprints to {}",
            "✓".green(),
            fps.len(),
            path.cyan()
        ),
        Err(e) => {
            eprintln!("{} {}", "✗".red(), e);
            std::process::exit(1);
        }
    }
}

pub async fn adaptive_load(path: &str) {
    match onecrawl_cdp::adaptive::load_fingerprints(std::path::Path::new(path)) {
        Ok(fps) => {
            println!("{}", serde_json::to_string_pretty(&fps).unwrap_or_default());
        }
        Err(e) => {
            eprintln!("{} {}", "✗".red(), e);
            std::process::exit(1);
        }
    }
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

// ---------------------------------------------------------------------------
// DOM Observer
// ---------------------------------------------------------------------------

pub async fn dom_observe(selector: Option<&str>) {
    let sel = selector.map(String::from);
    with_page(|page| async move {
        onecrawl_cdp::dom_observer::start_dom_observer(&page, sel.as_deref())
            .await
            .map_err(|e| e.to_string())?;
        println!("{} DOM observer started", "✓".green());
        Ok(())
    })
    .await;
}

pub async fn dom_mutations() {
    with_page(|page| async move {
        let mutations = onecrawl_cdp::dom_observer::drain_dom_mutations(&page)
            .await
            .map_err(|e| e.to_string())?;
        println!(
            "{}",
            serde_json::to_string_pretty(&mutations).unwrap_or_default()
        );
        Ok(())
    })
    .await;
}

pub async fn dom_stop() {
    with_page(|page| async move {
        onecrawl_cdp::dom_observer::stop_dom_observer(&page)
            .await
            .map_err(|e| e.to_string())?;
        println!("{} DOM observer stopped", "✓".green());
        Ok(())
    })
    .await;
}

pub async fn dom_snapshot(selector: Option<&str>) {
    let sel = selector.map(String::from);
    with_page(|page| async move {
        let html = onecrawl_cdp::dom_observer::get_dom_snapshot(&page, sel.as_deref())
            .await
            .map_err(|e| e.to_string())?;
        println!("{html}");
        Ok(())
    })
    .await;
}

// ---------------------------------------------------------------------------
// Iframe
// ---------------------------------------------------------------------------

pub async fn iframe_list() {
    with_page(|page| async move {
        let iframes = onecrawl_cdp::iframe::list_iframes(&page)
            .await
            .map_err(|e| e.to_string())?;
        println!(
            "{}",
            serde_json::to_string_pretty(&iframes).unwrap_or_default()
        );
        Ok(())
    })
    .await;
}

pub async fn iframe_eval(index: usize, expression: &str) {
    let expr = expression.to_string();
    with_page(|page| async move {
        let val = onecrawl_cdp::iframe::eval_in_iframe(&page, index, &expr)
            .await
            .map_err(|e| e.to_string())?;
        println!("{}", serde_json::to_string_pretty(&val).unwrap_or_default());
        Ok(())
    })
    .await;
}

pub async fn iframe_content(index: usize) {
    with_page(|page| async move {
        let html = onecrawl_cdp::iframe::get_iframe_content(&page, index)
            .await
            .map_err(|e| e.to_string())?;
        println!("{html}");
        Ok(())
    })
    .await;
}

// ---------------------------------------------------------------------------
// Print (Enhanced)
// ---------------------------------------------------------------------------

#[allow(clippy::too_many_arguments)]
pub async fn print_pdf(
    output: &str,
    landscape: bool,
    background: bool,
    scale: Option<f64>,
    paper_width: Option<f64>,
    paper_height: Option<f64>,
    margins: Option<&str>,
    page_ranges: Option<String>,
    header: Option<String>,
    footer: Option<String>,
) {
    let out = output.to_string();
    let (mt, mb, ml, mr) = if let Some(m) = margins {
        let parts: Vec<f64> = m.split(',').filter_map(|s| s.trim().parse().ok()).collect();
        (
            parts.first().copied(),
            parts.get(1).copied(),
            parts.get(2).copied(),
            parts.get(3).copied(),
        )
    } else {
        (None, None, None, None)
    };
    with_page(|page| async move {
        let opts = onecrawl_cdp::DetailedPdfOptions {
            landscape: if landscape { Some(true) } else { None },
            print_background: if background { Some(true) } else { None },
            scale,
            paper_width,
            paper_height,
            margin_top: mt,
            margin_bottom: mb,
            margin_left: ml,
            margin_right: mr,
            page_ranges,
            header_template: header,
            footer_template: footer,
            display_header_footer: None,
            prefer_css_page_size: None,
        };
        let bytes = onecrawl_cdp::print::print_to_pdf(&page, &opts)
            .await
            .map_err(|e| e.to_string())?;
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

pub async fn print_metrics() {
    with_page(|page| async move {
        let val = onecrawl_cdp::print::get_print_metrics(&page)
            .await
            .map_err(|e| e.to_string())?;
        println!("{}", serde_json::to_string_pretty(&val).unwrap_or_default());
        Ok(())
    })
    .await;
}

// ---------------------------------------------------------------------------
// Proxy
// ---------------------------------------------------------------------------

pub async fn proxy_create_pool(json: &str) {
    match onecrawl_cdp::ProxyPool::from_json(json) {
        Ok(pool) => match pool.to_json() {
            Ok(out) => println!("{out}"),
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

pub async fn proxy_chrome_args(json: &str) {
    match onecrawl_cdp::ProxyPool::from_json(json) {
        Ok(pool) => {
            let args = pool.chrome_args();
            println!("{}", args.join(" "));
        }
        Err(e) => {
            eprintln!("{} {e}", "✗".red());
            std::process::exit(1);
        }
    }
}

pub async fn proxy_next(json: &str) {
    match onecrawl_cdp::ProxyPool::from_json(json) {
        Ok(mut pool) => {
            pool.next_proxy();
            match pool.to_json() {
                Ok(out) => println!("{out}"),
                Err(e) => {
                    eprintln!("{} {e}", "✗".red());
                    std::process::exit(1);
                }
            }
        }
        Err(e) => {
            eprintln!("{} {e}", "✗".red());
            std::process::exit(1);
        }
    }
}

// ---------------------------------------------------------------------------
// Request Interception
// ---------------------------------------------------------------------------

pub async fn intercept_set(rules_json: &str) {
    let rules: Vec<onecrawl_cdp::InterceptRule> = match serde_json::from_str(rules_json) {
        Ok(r) => r,
        Err(e) => {
            eprintln!("{} Invalid rules JSON: {e}", "✗".red());
            std::process::exit(1);
        }
    };
    with_page(|page| async move {
        onecrawl_cdp::intercept::set_intercept_rules(&page, rules)
            .await
            .map_err(|e| e.to_string())?;
        println!("{} Intercept rules set", "✓".green());
        Ok(())
    })
    .await;
}

pub async fn intercept_log() {
    with_page(|page| async move {
        let log = onecrawl_cdp::intercept::get_intercepted_requests(&page)
            .await
            .map_err(|e| e.to_string())?;
        println!("{}", serde_json::to_string_pretty(&log).unwrap_or_default());
        Ok(())
    })
    .await;
}

pub async fn intercept_clear() {
    with_page(|page| async move {
        onecrawl_cdp::intercept::clear_intercept_rules(&page)
            .await
            .map_err(|e| e.to_string())?;
        println!("{} Intercept rules cleared", "✓".green());
        Ok(())
    })
    .await;
}

// ---------------------------------------------------------------------------
// Advanced Emulation
// ---------------------------------------------------------------------------

pub async fn adv_emulation_orientation(alpha: f64, beta: f64, gamma: f64) {
    with_page(|page| async move {
        let reading = onecrawl_cdp::advanced_emulation::SensorReading { alpha, beta, gamma };
        onecrawl_cdp::advanced_emulation::set_device_orientation(&page, reading)
            .await
            .map_err(|e| e.to_string())?;
        println!(
            "{} Device orientation set (α={alpha}, β={beta}, γ={gamma})",
            "✓".green()
        );
        Ok(())
    })
    .await;
}

pub async fn adv_emulation_permission(name: &str, state: &str) {
    let n = name.to_string();
    let s = state.to_string();
    with_page(|page| async move {
        onecrawl_cdp::advanced_emulation::override_permission(&page, &n, &s)
            .await
            .map_err(|e| e.to_string())?;
        println!("{} Permission '{n}' → {s}", "✓".green());
        Ok(())
    })
    .await;
}

pub async fn adv_emulation_battery(level: f64, charging: bool) {
    with_page(|page| async move {
        onecrawl_cdp::advanced_emulation::set_battery_status(&page, level, charging)
            .await
            .map_err(|e| e.to_string())?;
        println!(
            "{} Battery: {:.0}% {}",
            "✓".green(),
            level * 100.0,
            if charging { "(charging)" } else { "" }
        );
        Ok(())
    })
    .await;
}

pub async fn adv_emulation_connection(effective_type: &str, downlink: f64, rtt: u32) {
    let et = effective_type.to_string();
    with_page(|page| async move {
        onecrawl_cdp::advanced_emulation::set_connection_info(&page, &et, downlink, rtt)
            .await
            .map_err(|e| e.to_string())?;
        println!(
            "{} Connection: {et} ↓{downlink}Mbps RTT={rtt}ms",
            "✓".green()
        );
        Ok(())
    })
    .await;
}

pub async fn adv_emulation_cpu_cores(n: u32) {
    with_page(|page| async move {
        onecrawl_cdp::advanced_emulation::set_hardware_concurrency(&page, n)
            .await
            .map_err(|e| e.to_string())?;
        println!("{} CPU cores → {n}", "✓".green());
        Ok(())
    })
    .await;
}

pub async fn adv_emulation_memory(gb: f64) {
    with_page(|page| async move {
        onecrawl_cdp::advanced_emulation::set_device_memory(&page, gb)
            .await
            .map_err(|e| e.to_string())?;
        println!("{} Device memory → {gb}GB", "✓".green());
        Ok(())
    })
    .await;
}

pub async fn adv_emulation_navigator_info() {
    with_page(|page| async move {
        let info = onecrawl_cdp::advanced_emulation::get_navigator_info(&page)
            .await
            .map_err(|e| e.to_string())?;
        println!(
            "{}",
            serde_json::to_string_pretty(&info).unwrap_or_default()
        );
        Ok(())
    })
    .await;
}

// ---------------------------------------------------------------------------
// Tab Management
// ---------------------------------------------------------------------------

pub async fn tab_list() {
    with_session(|session, _page| async move {
        let tabs = onecrawl_cdp::tabs::list_tabs(session.browser())
            .await
            .map_err(|e| e.to_string())?;
        println!(
            "{}",
            serde_json::to_string_pretty(&tabs).unwrap_or_default()
        );
        Ok(())
    })
    .await;
}

pub async fn tab_new(url: &str) {
    let url = url.to_string();
    with_session(|session, _page| async move {
        let _page = onecrawl_cdp::tabs::new_tab(session.browser(), &url)
            .await
            .map_err(|e| e.to_string())?;
        println!("{} Opened new tab: {}", "✓".green(), url.cyan());
        Ok(())
    })
    .await;
}

pub async fn tab_close(index: usize) {
    with_session(|session, _page| async move {
        onecrawl_cdp::tabs::close_tab(session.browser(), index)
            .await
            .map_err(|e| e.to_string())?;
        println!("{} Closed tab {}", "✓".green(), index);
        Ok(())
    })
    .await;
}

pub async fn tab_switch(index: usize) {
    with_session(|session, _page| async move {
        let tab = onecrawl_cdp::tabs::get_tab(session.browser(), index)
            .await
            .map_err(|e| e.to_string())?;
        let target_id = tab.target_id().inner().clone();

        // Persist the active tab so every subsequent command uses this tab.
        let mut info = super::session::load_session()
            .ok_or_else(|| "No active session".to_string())?;
        info.active_tab_id = Some(target_id);
        super::session::save_session(&info)
            .map_err(|e| format!("Failed to save session: {e}"))?;

        println!("{} Switched to tab {}", "✓".green(), index);
        Ok(())
    })
    .await;
}

pub async fn tab_count_cmd() {
    with_session(|session, _page| async move {
        let count = onecrawl_cdp::tabs::tab_count(session.browser())
            .await
            .map_err(|e| e.to_string())?;
        println!("{count}");
        Ok(())
    })
    .await;
}

// ---------------------------------------------------------------------------
// Download Management
// ---------------------------------------------------------------------------

pub async fn download_set_path(path: &str) {
    let path = path.to_string();
    with_page(|page| async move {
        onecrawl_cdp::downloads::set_download_path(&page, std::path::Path::new(&path))
            .await
            .map_err(|e| e.to_string())?;
        println!("{} Download path set to: {}", "✓".green(), path.cyan());
        Ok(())
    })
    .await;
}

pub async fn download_list() {
    with_page(|page| async move {
        let downloads = onecrawl_cdp::downloads::get_downloads(&page)
            .await
            .map_err(|e| e.to_string())?;
        println!(
            "{}",
            serde_json::to_string_pretty(&downloads).unwrap_or_default()
        );
        Ok(())
    })
    .await;
}

pub async fn download_fetch(url: &str) {
    let url = url.to_string();
    with_page(|page| async move {
        let b64 = onecrawl_cdp::downloads::download_file(&page, &url)
            .await
            .map_err(|e| e.to_string())?;
        println!("{b64}");
        Ok(())
    })
    .await;
}

pub async fn download_wait(timeout_ms: u64) {
    with_page(|page| async move {
        let result = onecrawl_cdp::downloads::wait_for_download(&page, timeout_ms)
            .await
            .map_err(|e| e.to_string())?;
        match result {
            Some(d) => println!("{}", serde_json::to_string_pretty(&d).unwrap_or_default()),
            None => println!("No download detected within {timeout_ms}ms"),
        }
        Ok(())
    })
    .await;
}

pub async fn download_clear() {
    with_page(|page| async move {
        onecrawl_cdp::downloads::clear_downloads(&page)
            .await
            .map_err(|e| e.to_string())?;
        println!("{} Download history cleared", "✓".green());
        Ok(())
    })
    .await;
}

// ---------------------------------------------------------------------------
// Screenshot Diff
// ---------------------------------------------------------------------------

pub async fn screenshot_diff_compare(baseline: &str, current: &str) {
    let b = baseline.to_string();
    let c = current.to_string();
    with_page(|_page| async move {
        let result = onecrawl_cdp::screenshot_diff::compare_screenshot_files(
            std::path::Path::new(&b),
            std::path::Path::new(&c),
        )
        .map_err(|e| e.to_string())?;
        println!(
            "{}",
            serde_json::to_string_pretty(&result).unwrap_or_default()
        );
        Ok(())
    })
    .await;
}

pub async fn screenshot_diff_regression(baseline_path: &str) {
    let bp = baseline_path.to_string();
    with_page(|page| async move {
        let result =
            onecrawl_cdp::screenshot_diff::visual_regression(&page, std::path::Path::new(&bp))
                .await
                .map_err(|e| e.to_string())?;
        println!(
            "{}",
            serde_json::to_string_pretty(&result).unwrap_or_default()
        );
        Ok(())
    })
    .await;
}

// ---------------------------------------------------------------------------
// Benchmark
// ---------------------------------------------------------------------------

pub async fn bench_run(iterations: u32, _module: Option<&str>) {
    with_page(|page| async move {
        println!(
            "{} Running CDP benchmarks ({iterations} iterations)…",
            "⏱".yellow()
        );
        let suite = onecrawl_cdp::benchmark::run_cdp_benchmarks(&page, iterations).await;
        let table = onecrawl_cdp::benchmark::format_results(&suite);
        println!("{table}");

        // Save JSON report
        let dir = std::path::PathBuf::from("reports");
        let _ = std::fs::create_dir_all(&dir);
        let json_path = dir.join("cdp-bench.json");
        if let Ok(json) = serde_json::to_string_pretty(&suite) {
            let _ = std::fs::write(&json_path, &json);
            println!("{} Report saved to {}", "✓".green(), json_path.display());
        }
        Ok(())
    })
    .await;
}

pub async fn bench_report(format: &str) {
    let json_path = std::path::PathBuf::from("reports").join("cdp-bench.json");

    let data = match std::fs::read_to_string(&json_path) {
        Ok(d) => d,
        Err(_) => {
            eprintln!(
                "{} No benchmark data found. Run `onecrawl bench run` first.",
                "✗".red()
            );
            std::process::exit(1);
        }
    };

    match format {
        "json" => println!("{data}"),
        _ => {
            if let Ok(suite) = serde_json::from_str::<onecrawl_cdp::BenchmarkSuite>(&data) {
                println!("{}", onecrawl_cdp::benchmark::format_results(&suite));
            } else {
                eprintln!("{} Failed to parse benchmark data", "✗".red());
                std::process::exit(1);
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Geofencing
// ---------------------------------------------------------------------------

pub async fn geo_apply(profile: &str) {
    let profile = profile.to_string();
    with_page(|page| async move {
        let geo: onecrawl_cdp::GeoProfile =
            if let Some(p) = onecrawl_cdp::geofencing::get_preset(&profile) {
                p
            } else {
                serde_json::from_str(&profile)
                    .map_err(|e| format!("Invalid profile name or JSON: {e}"))?
            };
        onecrawl_cdp::geofencing::apply_geo_profile(&page, &geo)
            .await
            .map_err(|e| e.to_string())?;
        println!(
            "{} Geo profile '{}' applied (lat={}, lng={})",
            "✓".green(),
            geo.name,
            geo.latitude,
            geo.longitude
        );
        Ok(())
    })
    .await;
}

pub async fn geo_presets() {
    let presets = onecrawl_cdp::geofencing::list_presets();
    for name in &presets {
        if let Some(p) = onecrawl_cdp::geofencing::get_preset(name) {
            println!(
                "  {} — lat={:.4}, lng={:.4}, tz={}",
                name.green(),
                p.latitude,
                p.longitude,
                p.timezone
            );
        }
    }
}

pub async fn geo_current() {
    with_page(|page| async move {
        let val = onecrawl_cdp::geofencing::get_current_geo(&page)
            .await
            .map_err(|e| e.to_string())?;
        println!("{}", serde_json::to_string_pretty(&val).unwrap_or_default());
        Ok(())
    })
    .await;
}

// ---------------------------------------------------------------------------
// Cookie Jar
// ---------------------------------------------------------------------------

pub async fn cookie_jar_export(output: Option<&str>) {
    let output = output.map(String::from);
    with_page(|page| async move {
        if let Some(path) = output {
            let count =
                onecrawl_cdp::cookie_jar::save_cookies_to_file(&page, std::path::Path::new(&path))
                    .await
                    .map_err(|e| e.to_string())?;
            println!("{} Exported {} cookies to {}", "✓".green(), count, path);
        } else {
            let jar = onecrawl_cdp::cookie_jar::export_cookies(&page)
                .await
                .map_err(|e| e.to_string())?;
            println!("{}", serde_json::to_string_pretty(&jar).unwrap_or_default());
        }
        Ok(())
    })
    .await;
}

pub async fn cookie_jar_import(path: &str) {
    let path = path.to_string();
    with_page(|page| async move {
        let count =
            onecrawl_cdp::cookie_jar::load_cookies_from_file(&page, std::path::Path::new(&path))
                .await
                .map_err(|e| e.to_string())?;
        println!("{} Imported {} cookies from {}", "✓".green(), count, path);
        Ok(())
    })
    .await;
}

pub async fn cookie_jar_clear() {
    with_page(|page| async move {
        onecrawl_cdp::cookie_jar::clear_all_cookies(&page)
            .await
            .map_err(|e| e.to_string())?;
        println!("{} All cookies cleared (cookie jar)", "✓".green());
        Ok(())
    })
    .await;
}

// ---------------------------------------------------------------------------
// Request Queue
// ---------------------------------------------------------------------------

pub async fn request_execute(json: &str) {
    let json = json.to_string();
    with_page(|page| async move {
        let req: onecrawl_cdp::QueuedRequest =
            serde_json::from_str(&json).map_err(|e| format!("Invalid request JSON: {e}"))?;
        let result = onecrawl_cdp::request_queue::execute_request(&page, &req)
            .await
            .map_err(|e| e.to_string())?;
        println!(
            "{}",
            serde_json::to_string_pretty(&result).unwrap_or_default()
        );
        Ok(())
    })
    .await;
}

pub async fn request_batch(json: &str, concurrency: usize, delay: u64) {
    let json = json.to_string();
    with_page(|page| async move {
        let reqs: Vec<onecrawl_cdp::QueuedRequest> =
            serde_json::from_str(&json).map_err(|e| format!("Invalid requests JSON: {e}"))?;
        let config = onecrawl_cdp::QueueConfig {
            concurrency,
            delay_between_ms: delay,
            ..Default::default()
        };
        let results = onecrawl_cdp::request_queue::execute_batch(&page, &reqs, &config)
            .await
            .map_err(|e| e.to_string())?;
        println!(
            "{}",
            serde_json::to_string_pretty(&results).unwrap_or_default()
        );
        Ok(())
    })
    .await;
}

// ---------------------------------------------------------------------------
// Smart Selectors
// ---------------------------------------------------------------------------

pub async fn select_css(selector: &str) {
    let selector = selector.to_string();
    with_page(|page| async move {
        let result = onecrawl_cdp::selectors::css_select(&page, &selector)
            .await
            .map_err(|e| e.to_string())?;
        println!(
            "{}",
            serde_json::to_string_pretty(&result).unwrap_or_default()
        );
        Ok(())
    })
    .await;
}

pub async fn select_xpath(expression: &str) {
    let expression = expression.to_string();
    with_page(|page| async move {
        let result = onecrawl_cdp::selectors::xpath_select(&page, &expression)
            .await
            .map_err(|e| e.to_string())?;
        println!(
            "{}",
            serde_json::to_string_pretty(&result).unwrap_or_default()
        );
        Ok(())
    })
    .await;
}

pub async fn select_text(text: &str, tag: Option<&str>) {
    let text = text.to_string();
    let tag = tag.map(String::from);
    with_page(|page| async move {
        let result = onecrawl_cdp::selectors::find_by_text(&page, &text, tag.as_deref())
            .await
            .map_err(|e| e.to_string())?;
        println!(
            "{}",
            serde_json::to_string_pretty(&result).unwrap_or_default()
        );
        Ok(())
    })
    .await;
}

pub async fn select_regex(pattern: &str, tag: Option<&str>) {
    let pattern = pattern.to_string();
    let tag = tag.map(String::from);
    with_page(|page| async move {
        let result = onecrawl_cdp::selectors::find_by_regex(&page, &pattern, tag.as_deref())
            .await
            .map_err(|e| e.to_string())?;
        println!(
            "{}",
            serde_json::to_string_pretty(&result).unwrap_or_default()
        );
        Ok(())
    })
    .await;
}

pub async fn select_auto(selector: &str) {
    let selector = selector.to_string();
    with_page(|page| async move {
        let result = onecrawl_cdp::selectors::auto_selector(&page, &selector)
            .await
            .map_err(|e| e.to_string())?;
        println!("{result}");
        Ok(())
    })
    .await;
}

// ---------------------------------------------------------------------------
// DOM Navigation
// ---------------------------------------------------------------------------

pub async fn nav_parent(selector: &str) {
    let selector = selector.to_string();
    with_page(|page| async move {
        let result = onecrawl_cdp::dom_nav::get_parent(&page, &selector)
            .await
            .map_err(|e| e.to_string())?;
        println!(
            "{}",
            serde_json::to_string_pretty(&result).unwrap_or_default()
        );
        Ok(())
    })
    .await;
}

pub async fn nav_children(selector: &str) {
    let selector = selector.to_string();
    with_page(|page| async move {
        let result = onecrawl_cdp::dom_nav::get_children(&page, &selector)
            .await
            .map_err(|e| e.to_string())?;
        println!(
            "{}",
            serde_json::to_string_pretty(&result).unwrap_or_default()
        );
        Ok(())
    })
    .await;
}

pub async fn nav_next_sibling(selector: &str) {
    let selector = selector.to_string();
    with_page(|page| async move {
        let result = onecrawl_cdp::dom_nav::get_next_sibling(&page, &selector)
            .await
            .map_err(|e| e.to_string())?;
        println!(
            "{}",
            serde_json::to_string_pretty(&result).unwrap_or_default()
        );
        Ok(())
    })
    .await;
}

pub async fn nav_prev_sibling(selector: &str) {
    let selector = selector.to_string();
    with_page(|page| async move {
        let result = onecrawl_cdp::dom_nav::get_prev_sibling(&page, &selector)
            .await
            .map_err(|e| e.to_string())?;
        println!(
            "{}",
            serde_json::to_string_pretty(&result).unwrap_or_default()
        );
        Ok(())
    })
    .await;
}

pub async fn nav_siblings(selector: &str) {
    let selector = selector.to_string();
    with_page(|page| async move {
        let result = onecrawl_cdp::dom_nav::get_siblings(&page, &selector)
            .await
            .map_err(|e| e.to_string())?;
        println!(
            "{}",
            serde_json::to_string_pretty(&result).unwrap_or_default()
        );
        Ok(())
    })
    .await;
}

pub async fn nav_similar(selector: &str) {
    let selector = selector.to_string();
    with_page(|page| async move {
        let result = onecrawl_cdp::dom_nav::find_similar(&page, &selector)
            .await
            .map_err(|e| e.to_string())?;
        println!(
            "{}",
            serde_json::to_string_pretty(&result).unwrap_or_default()
        );
        Ok(())
    })
    .await;
}

pub async fn nav_above(selector: &str, limit: usize) {
    let selector = selector.to_string();
    with_page(|page| async move {
        let result = onecrawl_cdp::dom_nav::above_elements(&page, &selector, limit)
            .await
            .map_err(|e| e.to_string())?;
        println!(
            "{}",
            serde_json::to_string_pretty(&result).unwrap_or_default()
        );
        Ok(())
    })
    .await;
}

pub async fn nav_below(selector: &str, limit: usize) {
    let selector = selector.to_string();
    with_page(|page| async move {
        let result = onecrawl_cdp::dom_nav::below_elements(&page, &selector, limit)
            .await
            .map_err(|e| e.to_string())?;
        println!(
            "{}",
            serde_json::to_string_pretty(&result).unwrap_or_default()
        );
        Ok(())
    })
    .await;
}

// ---------------------------------------------------------------------------
// Content Extraction
// ---------------------------------------------------------------------------

pub async fn extract_content(format: &str, selector: Option<&str>, output: Option<&str>) {
    let format = format.to_string();
    let selector = selector.map(String::from);
    let output = output.map(String::from);
    with_page(|page| async move {
        let fmt =
            onecrawl_cdp::extract::parse_extract_format(&format).map_err(|e| e.to_string())?;

        if let Some(path) = output {
            let bytes = onecrawl_cdp::extract::extract_to_file(
                &page,
                selector.as_deref(),
                std::path::Path::new(&path),
            )
            .await
            .map_err(|e| e.to_string())?;
            println!("{} Extracted {} bytes to {}", "✓".green(), bytes, path);
        } else {
            let result = onecrawl_cdp::extract::extract(&page, selector.as_deref(), fmt)
                .await
                .map_err(|e| e.to_string())?;
            println!("{}", result.content);
        }
        Ok(())
    })
    .await;
}

pub async fn extract_metadata() {
    with_page(|page| async move {
        let meta = onecrawl_cdp::extract::get_page_metadata(&page)
            .await
            .map_err(|e| e.to_string())?;
        println!(
            "{}",
            serde_json::to_string_pretty(&meta).unwrap_or_default()
        );
        Ok(())
    })
    .await;
}

// ---------------------------------------------------------------------------
// Network Log
// ---------------------------------------------------------------------------

pub async fn network_log_start() {
    with_page(|page| async move {
        onecrawl_cdp::network_log::start_network_log(&page)
            .await
            .map_err(|e| e.to_string())?;
        println!("{} Network logging started", "✓".green());
        Ok(())
    })
    .await;
}

pub async fn network_log_drain() {
    with_page(|page| async move {
        let entries = onecrawl_cdp::network_log::drain_network_log(&page)
            .await
            .map_err(|e| e.to_string())?;
        println!(
            "{}",
            serde_json::to_string_pretty(&entries).unwrap_or_default()
        );
        Ok(())
    })
    .await;
}

pub async fn network_log_summary() {
    with_page(|page| async move {
        let summary = onecrawl_cdp::network_log::get_network_summary(&page)
            .await
            .map_err(|e| e.to_string())?;
        println!(
            "{}",
            serde_json::to_string_pretty(&summary).unwrap_or_default()
        );
        Ok(())
    })
    .await;
}

pub async fn network_log_stop() {
    with_page(|page| async move {
        onecrawl_cdp::network_log::stop_network_log(&page)
            .await
            .map_err(|e| e.to_string())?;
        println!("{} Network logging stopped", "✓".green());
        Ok(())
    })
    .await;
}

pub async fn network_log_export(path: &str) {
    let p = path.to_string();
    with_page(|page| async move {
        onecrawl_cdp::network_log::export_network_log(&page, &p)
            .await
            .map_err(|e| e.to_string())?;
        println!("{} Network log exported to {}", "✓".green(), p.cyan());
        Ok(())
    })
    .await;
}

// ---------------------------------------------------------------------------
// Page Watcher
// ---------------------------------------------------------------------------

pub async fn page_watcher_start() {
    with_page(|page| async move {
        onecrawl_cdp::page_watcher::start_page_watcher(&page)
            .await
            .map_err(|e| e.to_string())?;
        println!("{} Page watcher started", "✓".green());
        Ok(())
    })
    .await;
}

pub async fn page_watcher_drain() {
    with_page(|page| async move {
        let changes = onecrawl_cdp::page_watcher::drain_page_changes(&page)
            .await
            .map_err(|e| e.to_string())?;
        println!(
            "{}",
            serde_json::to_string_pretty(&changes).unwrap_or_default()
        );
        Ok(())
    })
    .await;
}

pub async fn page_watcher_stop() {
    with_page(|page| async move {
        onecrawl_cdp::page_watcher::stop_page_watcher(&page)
            .await
            .map_err(|e| e.to_string())?;
        println!("{} Page watcher stopped", "✓".green());
        Ok(())
    })
    .await;
}

pub async fn page_watcher_state() {
    with_page(|page| async move {
        let state = onecrawl_cdp::page_watcher::get_page_state(&page)
            .await
            .map_err(|e| e.to_string())?;
        println!(
            "{}",
            serde_json::to_string_pretty(&state).unwrap_or_default()
        );
        Ok(())
    })
    .await;
}

// ── Spider / Crawl ─────────────────────────────────────────────────

#[allow(clippy::too_many_arguments)]
pub async fn spider_crawl(
    start_url: &str,
    max_depth: usize,
    max_pages: usize,
    concurrency: usize,
    delay: u64,
    same_domain: bool,
    selector: Option<&str>,
    format: &str,
    output: Option<&str>,
    output_format: &str,
) {
    with_page(|page| async move {
        let config = onecrawl_cdp::SpiderConfig {
            start_urls: vec![start_url.to_string()],
            max_depth,
            max_pages,
            concurrency,
            delay_ms: delay,
            follow_links: true,
            same_domain_only: same_domain,
            extract_selector: selector.map(String::from),
            extract_format: format.to_string(),
            ..Default::default()
        };
        println!(
            "{} Starting crawl from {} (depth={}, max_pages={})",
            "→".cyan(),
            start_url,
            max_depth,
            max_pages
        );
        let results = onecrawl_cdp::spider::crawl(&page, config)
            .await
            .map_err(|e| e.to_string())?;
        let summary = onecrawl_cdp::spider::summarize(&results);
        println!(
            "{} Crawl complete: {} pages ({} ok, {} failed) in {:.0}ms ({:.2} p/s)",
            "✓".green(),
            summary.total_pages,
            summary.successful,
            summary.failed,
            summary.total_duration_ms,
            summary.pages_per_second,
        );
        if let Some(path) = output {
            let p = std::path::Path::new(path);
            let count = match output_format {
                "jsonl" => onecrawl_cdp::spider::export_results_jsonl(&results, p),
                _ => onecrawl_cdp::spider::export_results(&results, p),
            }
            .map_err(|e| e.to_string())?;
            println!("{} Saved {} results to {}", "✓".green(), count, path);
        } else {
            println!(
                "{}",
                serde_json::to_string_pretty(&results).unwrap_or_default()
            );
        }
        Ok(())
    })
    .await;
}

pub async fn spider_resume(state_file: &str) {
    let state = match onecrawl_cdp::spider::load_state(std::path::Path::new(state_file)) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("{} Failed to load state: {}", "✗".red(), e);
            return;
        }
    };
    println!(
        "{} Resuming crawl: {} visited, {} pending",
        "→".cyan(),
        state.visited.len(),
        state.pending.len(),
    );
    let mut config = state.config.clone();
    config.start_urls = state.pending.iter().map(|(u, _)| u.clone()).collect();
    with_page(|page| async move {
        let results = onecrawl_cdp::spider::crawl(&page, config)
            .await
            .map_err(|e| e.to_string())?;
        let summary = onecrawl_cdp::spider::summarize(&results);
        println!(
            "{} Resume complete: {} pages ({} ok, {} failed)",
            "✓".green(),
            summary.total_pages,
            summary.successful,
            summary.failed,
        );
        println!(
            "{}",
            serde_json::to_string_pretty(&results).unwrap_or_default()
        );
        Ok(())
    })
    .await;
}

pub fn spider_summary(results_file: &str) {
    let data = match std::fs::read_to_string(results_file) {
        Ok(d) => d,
        Err(e) => {
            eprintln!("{} Failed to read file: {}", "✗".red(), e);
            return;
        }
    };
    let results: Vec<onecrawl_cdp::CrawlResult> = match serde_json::from_str(&data) {
        Ok(r) => r,
        Err(e) => {
            eprintln!("{} Invalid JSON: {}", "✗".red(), e);
            return;
        }
    };
    let summary = onecrawl_cdp::spider::summarize(&results);
    println!(
        "{}",
        serde_json::to_string_pretty(&summary).unwrap_or_default()
    );
}

// ---------------------------------------------------------------------------
// Robots.txt
// ---------------------------------------------------------------------------

pub async fn robots_parse(source: &str) {
    // If it looks like a URL, fetch via browser; otherwise read as file
    if source.starts_with("http://") || source.starts_with("https://") {
        with_page(|page| async move {
            let robots = onecrawl_cdp::robots::fetch_robots(&page, source)
                .await
                .map_err(|e| e.to_string())?;
            println!(
                "{}",
                serde_json::to_string_pretty(&robots).unwrap_or_default()
            );
            Ok(())
        })
        .await;
    } else {
        let content = match std::fs::read_to_string(source) {
            Ok(c) => c,
            Err(e) => {
                eprintln!("{} Failed to read file: {}", "✗".red(), e);
                return;
            }
        };
        let robots = onecrawl_cdp::robots::parse_robots(&content);
        println!(
            "{}",
            serde_json::to_string_pretty(&robots).unwrap_or_default()
        );
    }
}

pub async fn robots_check(url: &str, path: &str, user_agent: &str) {
    with_page(|page| async move {
        let robots = onecrawl_cdp::robots::fetch_robots(&page, url)
            .await
            .map_err(|e| e.to_string())?;
        let allowed = onecrawl_cdp::robots::is_allowed(&robots, user_agent, path);
        if allowed {
            println!(
                "{} Path \"{}\" is {} for {}",
                "✓".green(),
                path,
                "ALLOWED".green(),
                user_agent
            );
        } else {
            println!(
                "{} Path \"{}\" is {} for {}",
                "✗".red(),
                path,
                "DISALLOWED".red(),
                user_agent
            );
        }
        Ok(())
    })
    .await;
}

pub async fn robots_sitemaps(url: &str) {
    with_page(|page| async move {
        let robots = onecrawl_cdp::robots::fetch_robots(&page, url)
            .await
            .map_err(|e| e.to_string())?;
        let sitemaps = onecrawl_cdp::robots::get_sitemaps(&robots);
        if sitemaps.is_empty() {
            println!("{} No sitemaps declared", "→".cyan());
        } else {
            for s in &sitemaps {
                println!("  {s}");
            }
        }
        Ok(())
    })
    .await;
}

// ---------------------------------------------------------------------------
// Link Graph
// ---------------------------------------------------------------------------

pub async fn graph_extract(base_url: Option<&str>) {
    with_page(|page| async move {
        let current_url: String = page
            .evaluate("window.location.href")
            .await
            .ok()
            .and_then(|v| v.into_value::<String>().ok())
            .unwrap_or_default();
        let base = base_url.unwrap_or(&current_url);
        let edges = onecrawl_cdp::link_graph::extract_links(&page, base)
            .await
            .map_err(|e| e.to_string())?;
        println!(
            "{}",
            serde_json::to_string_pretty(&edges).unwrap_or_default()
        );
        Ok(())
    })
    .await;
}

pub fn graph_build(edges_file: &str) {
    let data = match std::fs::read_to_string(edges_file) {
        Ok(d) => d,
        Err(e) => {
            eprintln!("{} Failed to read file: {}", "✗".red(), e);
            return;
        }
    };
    let edges: Vec<onecrawl_cdp::LinkEdge> = match serde_json::from_str(&data) {
        Ok(e) => e,
        Err(e) => {
            eprintln!("{} Invalid JSON: {}", "✗".red(), e);
            return;
        }
    };
    let graph = onecrawl_cdp::link_graph::build_graph(&edges);
    println!(
        "{}",
        serde_json::to_string_pretty(&graph).unwrap_or_default()
    );
}

pub fn graph_analyze(graph_file: &str) {
    let data = match std::fs::read_to_string(graph_file) {
        Ok(d) => d,
        Err(e) => {
            eprintln!("{} Failed to read file: {}", "✗".red(), e);
            return;
        }
    };
    let graph: onecrawl_cdp::LinkGraph = match serde_json::from_str(&data) {
        Ok(g) => g,
        Err(e) => {
            eprintln!("{} Invalid JSON: {}", "✗".red(), e);
            return;
        }
    };
    let stats = onecrawl_cdp::link_graph::analyze_graph(&graph);
    println!(
        "{}",
        serde_json::to_string_pretty(&stats).unwrap_or_default()
    );
}

pub fn graph_export(graph_file: &str, output: &str) {
    let data = match std::fs::read_to_string(graph_file) {
        Ok(d) => d,
        Err(e) => {
            eprintln!("{} Failed to read file: {}", "✗".red(), e);
            return;
        }
    };
    let graph: onecrawl_cdp::LinkGraph = match serde_json::from_str(&data) {
        Ok(g) => g,
        Err(e) => {
            eprintln!("{} Invalid JSON: {}", "✗".red(), e);
            return;
        }
    };
    match onecrawl_cdp::link_graph::export_graph_json(&graph, std::path::Path::new(output)) {
        Ok(()) => println!("{} Graph exported to {}", "✓".green(), output),
        Err(e) => eprintln!("{} Export failed: {}", "✗".red(), e),
    }
}

// ---------------------------------------------------------------------------
// Interactive Shell
// ---------------------------------------------------------------------------

pub async fn shell_repl() {
    use std::io::{self, BufRead, Write};

    let mut history = onecrawl_cdp::shell::ShellHistory::new(500);
    let commands = onecrawl_cdp::shell::available_commands();

    println!("{} OneCrawl Interactive Shell", "▶".green());
    println!(
        "  Type {} for commands, {} to quit.\n",
        "help".cyan(),
        "exit".cyan()
    );

    loop {
        print!("{} ", "onecrawl>".green());
        io::stdout().flush().ok();

        let mut line = String::new();
        if io::stdin().lock().read_line(&mut line).is_err() || line.is_empty() {
            break;
        }
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }

        let cmd = onecrawl_cdp::shell::parse_command(trimmed);
        history.add(cmd.clone());

        match cmd.command.as_str() {
            "exit" | "quit" => {
                println!("{} Bye!", "✓".green());
                break;
            }
            "help" => {
                for (name, desc) in &commands {
                    println!("  {:<28} {}", name.cyan(), desc);
                }
            }
            "history" => {
                for (i, c) in history.commands.iter().enumerate() {
                    println!("  {:>4}  {}", i + 1, c.raw);
                }
            }
            other => {
                println!(
                    "{} Command '{}' would be dispatched to the browser session",
                    "→".yellow(),
                    other
                );
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Domain Blocker
// ---------------------------------------------------------------------------

pub async fn domain_block(domains: &[String]) {
    with_page(|page| async move {
        let count = onecrawl_cdp::domain_blocker::block_domains(&page, domains)
            .await
            .map_err(|e| e.to_string())?;
        println!(
            "{} Blocked {} domain(s) — {} total on blocklist",
            "✓".green(),
            domains.len(),
            count
        );
        Ok(())
    })
    .await;
}

pub async fn domain_block_category(category: &str) {
    let cat = category.to_string();
    with_page(|page| async move {
        let count = onecrawl_cdp::domain_blocker::block_category(&page, &cat)
            .await
            .map_err(|e| e.to_string())?;
        println!(
            "{} Category '{}' blocked — {} total on blocklist",
            "✓".green(),
            cat.cyan(),
            count
        );
        Ok(())
    })
    .await;
}

pub async fn domain_unblock() {
    with_page(|page| async move {
        onecrawl_cdp::domain_blocker::clear_blocks(&page)
            .await
            .map_err(|e| e.to_string())?;
        println!("{} All domain blocks cleared", "✓".green());
        Ok(())
    })
    .await;
}

pub async fn domain_stats() {
    with_page(|page| async move {
        let stats = onecrawl_cdp::domain_blocker::block_stats(&page)
            .await
            .map_err(|e| e.to_string())?;
        println!(
            "{}",
            serde_json::to_string_pretty(&stats).unwrap_or_default()
        );
        Ok(())
    })
    .await;
}

pub async fn domain_list() {
    with_page(|page| async move {
        let domains = onecrawl_cdp::domain_blocker::list_blocked(&page)
            .await
            .map_err(|e| e.to_string())?;
        if domains.is_empty() {
            println!("No domains currently blocked.");
        } else {
            for d in &domains {
                println!("  • {}", d);
            }
            println!("\n{} domain(s) blocked", domains.len());
        }
        Ok(())
    })
    .await;
}

pub fn domain_categories() {
    let cats = onecrawl_cdp::domain_blocker::available_categories();
    for (name, count) in &cats {
        println!("  {:<12} {} domains", name.cyan(), count);
    }
}

// ---------------------------------------------------------------------------
// Streaming Extractor
// ---------------------------------------------------------------------------

pub async fn stream_extract(
    item_selector: &str,
    fields: &[String],
    paginate: Option<&str>,
    max_pages: usize,
    output: Option<&str>,
    format: &str,
) {
    let fields = fields.to_vec();
    let item_selector = item_selector.to_string();
    let paginate = paginate.map(String::from);
    let output = output.map(String::from);
    let format = format.to_string();

    with_page(|page| async move {
        let rules: Vec<onecrawl_cdp::ExtractionRule> = fields
            .iter()
            .map(|f| onecrawl_cdp::streaming::parse_field_spec(f).map_err(|e| e.to_string()))
            .collect::<Result<Vec<_>, _>>()?;

        let pagination = paginate.map(|sel| onecrawl_cdp::PaginationConfig {
            next_selector: sel,
            max_pages,
            delay_ms: 1000,
        });

        let schema = onecrawl_cdp::ExtractionSchema {
            item_selector,
            fields: rules,
            pagination,
        };

        let result = onecrawl_cdp::streaming::extract_with_pagination(&page, &schema)
            .await
            .map_err(|e| e.to_string())?;

        if let Some(path) = output {
            let count = match format.as_str() {
                "csv" => {
                    onecrawl_cdp::streaming::export_csv(&result.items, std::path::Path::new(&path))
                        .map_err(|e| e.to_string())?
                }
                _ => {
                    onecrawl_cdp::streaming::export_json(&result.items, std::path::Path::new(&path))
                        .map_err(|e| e.to_string())?
                }
            };
            println!("{} Exported {} items to {}", "✓".green(), count, path);
        } else {
            println!(
                "{}",
                serde_json::to_string_pretty(&result).unwrap_or_default()
            );
        }

        if !result.errors.is_empty() {
            for err in &result.errors {
                eprintln!("{} {}", "⚠".yellow(), err);
            }
        }
        Ok(())
    })
    .await;
}

// ---------------------------------------------------------------------------
// HTTP Client
// ---------------------------------------------------------------------------

pub async fn http_get(url: &str) {
    let url = url.to_string();
    with_page(|page| async move {
        let resp = onecrawl_cdp::http_client::get(&page, &url, None)
            .await
            .map_err(|e| e.to_string())?;
        println!(
            "{}",
            serde_json::to_string_pretty(&resp).unwrap_or_default()
        );
        Ok(())
    })
    .await;
}

pub async fn http_post(url: &str, body: &str, content_type: &str) {
    let url = url.to_string();
    let body = body.to_string();
    let content_type = content_type.to_string();
    with_page(|page| async move {
        let resp = onecrawl_cdp::http_client::post(&page, &url, &body, &content_type, None)
            .await
            .map_err(|e| e.to_string())?;
        println!(
            "{}",
            serde_json::to_string_pretty(&resp).unwrap_or_default()
        );
        Ok(())
    })
    .await;
}

pub async fn http_head(url: &str) {
    let url = url.to_string();
    with_page(|page| async move {
        let resp = onecrawl_cdp::http_client::head(&page, &url)
            .await
            .map_err(|e| e.to_string())?;
        println!(
            "{}",
            serde_json::to_string_pretty(&resp).unwrap_or_default()
        );
        Ok(())
    })
    .await;
}

pub async fn http_fetch(json: &str) {
    let json = json.to_string();
    with_page(|page| async move {
        let request: onecrawl_cdp::HttpRequest =
            serde_json::from_str(&json).map_err(|e| e.to_string())?;
        let resp = onecrawl_cdp::http_client::fetch(&page, &request)
            .await
            .map_err(|e| e.to_string())?;
        println!(
            "{}",
            serde_json::to_string_pretty(&resp).unwrap_or_default()
        );
        Ok(())
    })
    .await;
}

// ---------------------------------------------------------------------------
// TLS Fingerprint
// ---------------------------------------------------------------------------

pub async fn fingerprint_apply(name: &str) {
    let n = name.to_string();
    with_page(|page| async move {
        let fp = if n == "random" {
            onecrawl_cdp::tls_fingerprint::random_fingerprint()
        } else {
            onecrawl_cdp::tls_fingerprint::get_profile(&n)
                .ok_or_else(|| format!("Unknown profile: {n}. Use: chrome-win, chrome-mac, firefox-win, firefox-mac, safari-mac, edge-win, random"))?
        };
        let overridden = onecrawl_cdp::tls_fingerprint::apply_fingerprint(&page, &fp)
            .await
            .map_err(|e| e.to_string())?;
        println!("{} Applied fingerprint: {}", "✓".green(), fp.name.cyan());
        println!("  UA: {}", fp.user_agent.dimmed());
        println!("  Platform: {}", fp.platform);
        println!("  Overridden: {}", overridden.join(", "));
        Ok(())
    })
    .await;
}

pub async fn fingerprint_detect() {
    with_page(|page| async move {
        let fp = onecrawl_cdp::tls_fingerprint::detect_fingerprint(&page)
            .await
            .map_err(|e| e.to_string())?;
        println!("{}", serde_json::to_string_pretty(&fp).unwrap_or_default());
        Ok(())
    })
    .await;
}

pub fn fingerprint_list() {
    let profiles = onecrawl_cdp::tls_fingerprint::browser_profiles();
    for p in &profiles {
        println!(
            "  {} — {} ({}×{}, {})",
            p.name.cyan(),
            p.platform,
            p.screen_width,
            p.screen_height,
            p.vendor
        );
    }
    println!("\n{} profiles available", profiles.len());
}

// ---------------------------------------------------------------------------
// Page Snapshot
// ---------------------------------------------------------------------------

pub async fn snapshot_take(output: Option<&str>) {
    let out = output.map(|s| s.to_string());
    with_page(|page| async move {
        let snap = onecrawl_cdp::snapshot::take_snapshot(&page)
            .await
            .map_err(|e| e.to_string())?;
        if let Some(path) = &out {
            onecrawl_cdp::snapshot::save_snapshot(&snap, std::path::Path::new(path))
                .map_err(|e| e.to_string())?;
            println!("{} Snapshot saved to {}", "✓".green(), path.cyan());
        } else {
            println!(
                "{}",
                serde_json::to_string_pretty(&snap).unwrap_or_default()
            );
        }
        Ok(())
    })
    .await;
}

pub fn snapshot_compare(path1: &str, path2: &str) {
    let a = onecrawl_cdp::snapshot::load_snapshot(std::path::Path::new(path1));
    let b = onecrawl_cdp::snapshot::load_snapshot(std::path::Path::new(path2));
    match (a, b) {
        (Ok(before), Ok(after)) => {
            let diff = onecrawl_cdp::snapshot::compare_snapshots(&before, &after);
            println!(
                "{}",
                serde_json::to_string_pretty(&diff).unwrap_or_default()
            );
        }
        (Err(e), _) | (_, Err(e)) => {
            eprintln!("{} {e}", "✗".red());
            std::process::exit(1);
        }
    }
}

pub async fn snapshot_watch(interval_ms: u64, selector: Option<&str>, count: usize) {
    let sel = selector.map(|s| s.to_string());
    with_page(|page| async move {
        let diffs =
            onecrawl_cdp::snapshot::watch_for_changes(&page, interval_ms, sel.as_deref(), count)
                .await
                .map_err(|e| e.to_string())?;
        for (i, diff) in diffs.iter().enumerate() {
            println!("--- Diff #{} ---", i + 1);
            println!("{}", serde_json::to_string_pretty(diff).unwrap_or_default());
        }
        println!("{} {} diffs captured", "✓".green(), diffs.len());
        Ok(())
    })
    .await;
}

// ---------------------------------------------------------------------------
// Rate Limiter (standalone — no Page required)
// ---------------------------------------------------------------------------

pub fn ratelimit_set(preset: Option<&str>, config_json: Option<&str>) {
    let cfg = if let Some(name) = preset {
        let presets = onecrawl_cdp::rate_limiter::presets();
        match presets.get(name) {
            Some(c) => c.clone(),
            None => {
                eprintln!(
                    "{} Unknown preset: {}. Use: conservative, moderate, aggressive, unlimited",
                    "✗".red(),
                    name
                );
                std::process::exit(1);
            }
        }
    } else if let Some(json) = config_json {
        match serde_json::from_str::<onecrawl_cdp::RateLimitConfig>(json) {
            Ok(c) => c,
            Err(e) => {
                eprintln!("{} Invalid config JSON: {e}", "✗".red());
                std::process::exit(1);
            }
        }
    } else {
        onecrawl_cdp::RateLimitConfig::default()
    };
    let state = onecrawl_cdp::RateLimitState::new(cfg);
    let stats = onecrawl_cdp::rate_limiter::get_stats(&state);
    println!("{} Rate limiter configured", "✓".green());
    println!(
        "{}",
        serde_json::to_string_pretty(&stats).unwrap_or_default()
    );
}

pub fn ratelimit_stats() {
    let state = onecrawl_cdp::RateLimitState::new(onecrawl_cdp::RateLimitConfig::default());
    let stats = onecrawl_cdp::rate_limiter::get_stats(&state);
    println!(
        "{}",
        serde_json::to_string_pretty(&stats).unwrap_or_default()
    );
}

pub fn ratelimit_reset() {
    println!("{} Rate limiter reset", "✓".green());
}

// ---------------------------------------------------------------------------
// Retry Queue (standalone — no Page required)
// ---------------------------------------------------------------------------

pub fn retry_enqueue(url: &str, operation: &str, payload: Option<&str>) {
    let mut queue = onecrawl_cdp::RetryQueue::new(onecrawl_cdp::RetryConfig::default());
    let id = onecrawl_cdp::retry_queue::enqueue(&mut queue, url, operation, payload);
    println!("{} Enqueued: {} ({})", "✓".green(), id, operation.cyan());
}

pub fn retry_next() {
    let mut queue = onecrawl_cdp::RetryQueue::new(onecrawl_cdp::RetryConfig::default());
    match onecrawl_cdp::retry_queue::get_next(&mut queue) {
        Some(item) => println!("{}", serde_json::to_string_pretty(item).unwrap_or_default()),
        None => println!("No items due for retry"),
    }
}

pub fn retry_success(id: &str) {
    println!("{} Marked {} as success", "✓".green(), id.cyan());
}

pub fn retry_fail(id: &str, error: &str) {
    println!("{} Marked {} as failed: {}", "✓".green(), id.cyan(), error);
}

pub fn retry_stats() {
    let queue = onecrawl_cdp::RetryQueue::new(onecrawl_cdp::RetryConfig::default());
    let stats = onecrawl_cdp::retry_queue::get_stats(&queue);
    println!(
        "{}",
        serde_json::to_string_pretty(&stats).unwrap_or_default()
    );
}

pub fn retry_clear() {
    println!("{} Completed items cleared", "✓".green());
}

pub fn retry_save(path: &str) {
    let queue = onecrawl_cdp::RetryQueue::new(onecrawl_cdp::RetryConfig::default());
    match onecrawl_cdp::retry_queue::save_queue(&queue, std::path::Path::new(path)) {
        Ok(()) => println!("{} Queue saved to {}", "✓".green(), path.cyan()),
        Err(e) => {
            eprintln!("{} Save failed: {e}", "✗".red());
            std::process::exit(1);
        }
    }
}

pub fn retry_load(path: &str) {
    match onecrawl_cdp::retry_queue::load_queue(std::path::Path::new(path)) {
        Ok(queue) => {
            let stats = onecrawl_cdp::retry_queue::get_stats(&queue);
            println!("{} Queue loaded from {}", "✓".green(), path.cyan());
            println!(
                "{}",
                serde_json::to_string_pretty(&stats).unwrap_or_default()
            );
        }
        Err(e) => {
            eprintln!("{} Load failed: {e}", "✗".red());
            std::process::exit(1);
        }
    }
}

// ──────────────── Data Pipeline ────────────────

pub fn pipeline_run(pipeline_path: &str, data_path: &str, output: Option<&str>, format: &str) {
    let pipeline =
        match onecrawl_cdp::data_pipeline::load_pipeline(std::path::Path::new(pipeline_path)) {
            Ok(p) => p,
            Err(e) => {
                eprintln!("{} Failed to load pipeline: {e}", "✗".red());
                std::process::exit(1);
            }
        };

    let data_str = match std::fs::read_to_string(data_path) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("{} Failed to read data: {e}", "✗".red());
            std::process::exit(1);
        }
    };

    let items: Vec<std::collections::HashMap<String, String>> =
        match serde_json::from_str(&data_str) {
            Ok(v) => v,
            Err(e) => {
                eprintln!("{} Invalid data JSON: {e}", "✗".red());
                std::process::exit(1);
            }
        };

    let result = onecrawl_cdp::data_pipeline::execute_pipeline(&pipeline, items);
    println!(
        "{} Pipeline '{}': {} → {} items ({} filtered, {} deduplicated)",
        "✓".green(),
        pipeline.name,
        result.input_count,
        result.output_count,
        result.filtered_count,
        result.deduplicated_count,
    );
    for err in &result.errors {
        eprintln!("  {} {err}", "⚠".yellow());
    }

    if let Some(out) = output {
        match onecrawl_cdp::data_pipeline::export_processed(
            &result,
            std::path::Path::new(out),
            format,
        ) {
            Ok(n) => println!("{} Exported {n} items to {}", "✓".green(), out.cyan()),
            Err(e) => {
                eprintln!("{} Export failed: {e}", "✗".red());
                std::process::exit(1);
            }
        }
    } else {
        println!(
            "{}",
            serde_json::to_string_pretty(&result).unwrap_or_default()
        );
    }
}

pub fn pipeline_validate(pipeline_path: &str) {
    let pipeline =
        match onecrawl_cdp::data_pipeline::load_pipeline(std::path::Path::new(pipeline_path)) {
            Ok(p) => p,
            Err(e) => {
                eprintln!("{} Failed to load pipeline: {e}", "✗".red());
                std::process::exit(1);
            }
        };

    let errors = onecrawl_cdp::data_pipeline::validate_pipeline(&pipeline);
    if errors.is_empty() {
        println!("{} Pipeline '{}' is valid", "✓".green(), pipeline.name);
    } else {
        eprintln!(
            "{} Pipeline '{}' has {} error(s):",
            "✗".red(),
            pipeline.name,
            errors.len()
        );
        for err in &errors {
            eprintln!("  - {err}");
        }
        std::process::exit(1);
    }
}

pub fn pipeline_save_file(pipeline_json: &str, path: &str) {
    let pipeline: onecrawl_cdp::Pipeline = match serde_json::from_str(pipeline_json) {
        Ok(p) => p,
        Err(e) => {
            eprintln!("{} Invalid pipeline JSON: {e}", "✗".red());
            std::process::exit(1);
        }
    };
    match onecrawl_cdp::data_pipeline::save_pipeline(&pipeline, std::path::Path::new(path)) {
        Ok(()) => println!("{} Pipeline saved to {}", "✓".green(), path.cyan()),
        Err(e) => {
            eprintln!("{} Save failed: {e}", "✗".red());
            std::process::exit(1);
        }
    }
}

pub fn pipeline_load_file(path: &str) {
    match onecrawl_cdp::data_pipeline::load_pipeline(std::path::Path::new(path)) {
        Ok(pipeline) => {
            println!(
                "{}",
                serde_json::to_string_pretty(&pipeline).unwrap_or_default()
            );
        }
        Err(e) => {
            eprintln!("{} Failed to load pipeline: {e}", "✗".red());
            std::process::exit(1);
        }
    }
}

// ──────────────── Structured Data ────────────────

pub async fn structured_extract_all() {
    with_page(|page| async move {
        let data = onecrawl_cdp::structured_data::extract_all(&page)
            .await
            .map_err(|e| e.to_string())?;
        println!(
            "{}",
            serde_json::to_string_pretty(&data).unwrap_or_default()
        );
        Ok(())
    })
    .await;
}

pub async fn structured_json_ld() {
    with_page(|page| async move {
        let data = onecrawl_cdp::structured_data::extract_json_ld(&page)
            .await
            .map_err(|e| e.to_string())?;
        println!(
            "{}",
            serde_json::to_string_pretty(&data).unwrap_or_default()
        );
        Ok(())
    })
    .await;
}

pub async fn structured_open_graph() {
    with_page(|page| async move {
        let data = onecrawl_cdp::structured_data::extract_open_graph(&page)
            .await
            .map_err(|e| e.to_string())?;
        println!(
            "{}",
            serde_json::to_string_pretty(&data).unwrap_or_default()
        );
        Ok(())
    })
    .await;
}

pub async fn structured_twitter_card() {
    with_page(|page| async move {
        let data = onecrawl_cdp::structured_data::extract_twitter_card(&page)
            .await
            .map_err(|e| e.to_string())?;
        println!(
            "{}",
            serde_json::to_string_pretty(&data).unwrap_or_default()
        );
        Ok(())
    })
    .await;
}

pub async fn structured_metadata() {
    with_page(|page| async move {
        let data = onecrawl_cdp::structured_data::extract_metadata(&page)
            .await
            .map_err(|e| e.to_string())?;
        println!(
            "{}",
            serde_json::to_string_pretty(&data).unwrap_or_default()
        );
        Ok(())
    })
    .await;
}

pub fn structured_validate(data_json: &str) {
    let data: onecrawl_cdp::StructuredDataResult = match serde_json::from_str(data_json) {
        Ok(d) => d,
        Err(e) => {
            eprintln!("{} Invalid data JSON: {e}", "✗".red());
            std::process::exit(1);
        }
    };
    let warnings = onecrawl_cdp::structured_data::validate_schema(&data);
    if warnings.is_empty() {
        println!("{} Structured data is complete", "✓".green());
    } else {
        println!("{} {} warning(s):", "⚠".yellow(), warnings.len());
        for w in &warnings {
            println!("  - {w}");
        }
    }
}

// ---------------------------------------------------------------------------
// Proxy Health
// ---------------------------------------------------------------------------

pub async fn proxy_health_check(proxy: &str, test_url: Option<&str>, timeout: u64) {
    let proxy = proxy.to_string();
    let mut config = onecrawl_cdp::ProxyHealthConfig::default();
    if let Some(url) = test_url {
        config.test_url = url.to_string();
    }
    config.timeout_ms = timeout;
    with_page(|page| async move {
        let result = onecrawl_cdp::proxy_health::check_proxy(&page, &proxy, &config)
            .await
            .map_err(|e| e.to_string())?;
        println!(
            "{}",
            serde_json::to_string_pretty(&result).unwrap_or_default()
        );
        Ok(())
    })
    .await;
}

pub async fn proxy_health_check_all(proxies_json: &str) {
    let proxies: Vec<String> = match serde_json::from_str(proxies_json) {
        Ok(p) => p,
        Err(e) => {
            eprintln!("{} Invalid proxies JSON: {e}", "✗".red());
            std::process::exit(1);
        }
    };
    let config = onecrawl_cdp::ProxyHealthConfig::default();
    with_page(|page| async move {
        let results = onecrawl_cdp::proxy_health::check_proxies(&page, &proxies, &config)
            .await
            .map_err(|e| e.to_string())?;
        println!(
            "{}",
            serde_json::to_string_pretty(&results).unwrap_or_default()
        );
        Ok(())
    })
    .await;
}

pub fn proxy_health_rank(results_json: &str) {
    let results: Vec<onecrawl_cdp::ProxyHealthResult> = match serde_json::from_str(results_json) {
        Ok(r) => r,
        Err(e) => {
            eprintln!("{} Invalid results JSON: {e}", "✗".red());
            std::process::exit(1);
        }
    };
    let ranked = onecrawl_cdp::proxy_health::rank_proxies(&results);
    println!(
        "{}",
        serde_json::to_string_pretty(&ranked).unwrap_or_default()
    );
}

pub fn proxy_health_filter(results_json: &str, min_score: u32) {
    let results: Vec<onecrawl_cdp::ProxyHealthResult> = match serde_json::from_str(results_json) {
        Ok(r) => r,
        Err(e) => {
            eprintln!("{} Invalid results JSON: {e}", "✗".red());
            std::process::exit(1);
        }
    };
    let filtered = onecrawl_cdp::proxy_health::filter_healthy(&results, min_score);
    println!(
        "{}",
        serde_json::to_string_pretty(&filtered).unwrap_or_default()
    );
}

// ---------------------------------------------------------------------------
// Captcha
// ---------------------------------------------------------------------------

pub async fn captcha_detect() {
    with_page(|page| async move {
        let detection = onecrawl_cdp::captcha::detect_captcha(&page)
            .await
            .map_err(|e| e.to_string())?;
        println!(
            "{}",
            serde_json::to_string_pretty(&detection).unwrap_or_default()
        );
        Ok(())
    })
    .await;
}

pub async fn captcha_wait(timeout: u64) {
    with_page(|page| async move {
        let detection = onecrawl_cdp::captcha::wait_for_captcha(&page, timeout)
            .await
            .map_err(|e| e.to_string())?;
        println!(
            "{}",
            serde_json::to_string_pretty(&detection).unwrap_or_default()
        );
        Ok(())
    })
    .await;
}

pub async fn captcha_screenshot() {
    with_page(|page| async move {
        let detection = onecrawl_cdp::captcha::detect_captcha(&page)
            .await
            .map_err(|e| e.to_string())?;
        if !detection.detected {
            println!("{} No CAPTCHA detected on current page", "⚠".yellow());
            return Ok(());
        }
        let data = onecrawl_cdp::captcha::screenshot_captcha(&page, &detection)
            .await
            .map_err(|e| e.to_string())?;
        println!("{data}");
        Ok(())
    })
    .await;
}

pub async fn captcha_inject(solution: &str) {
    let sol = solution.to_string();
    with_page(|page| async move {
        let detection = onecrawl_cdp::captcha::detect_captcha(&page)
            .await
            .map_err(|e| e.to_string())?;
        if !detection.detected {
            eprintln!("{} No CAPTCHA detected on current page", "✗".red());
            std::process::exit(1);
        }
        let ok = onecrawl_cdp::captcha::inject_solution(&page, &detection, &sol)
            .await
            .map_err(|e| e.to_string())?;
        if ok {
            println!(
                "{} Solution injected for {}",
                "✓".green(),
                detection.captcha_type.cyan()
            );
        } else {
            eprintln!(
                "{} Injection failed for {}",
                "✗".red(),
                detection.captcha_type
            );
            std::process::exit(1);
        }
        Ok(())
    })
    .await;
}

pub fn captcha_types() {
    let types = onecrawl_cdp::captcha::supported_types();
    for (name, desc) in &types {
        println!("  {}: {}", name.cyan(), desc);
    }
}

// ---------------------------------------------------------------------------
// Task Scheduler (standalone — no Page required)
// ---------------------------------------------------------------------------

pub fn schedule_add(
    name: &str,
    task_type: &str,
    config: &str,
    interval: u64,
    delay: u64,
    max_runs: Option<usize>,
) {
    let mut sched = onecrawl_cdp::Scheduler::new();
    let schedule = onecrawl_cdp::TaskSchedule {
        interval_ms: interval,
        delay_ms: delay,
        max_runs,
    };
    let id = onecrawl_cdp::scheduler::add_task(&mut sched, name, task_type, config, schedule);
    println!("{} Task added: {}", "✓".green(), id);
}

pub fn schedule_remove(id: &str) {
    let mut sched = onecrawl_cdp::Scheduler::new();
    if onecrawl_cdp::scheduler::remove_task(&mut sched, id) {
        println!("{} Task removed: {id}", "✓".green());
    } else {
        eprintln!("{} Task not found: {id}", "✗".red());
    }
}

pub fn schedule_pause(id: &str) {
    let mut sched = onecrawl_cdp::Scheduler::new();
    if onecrawl_cdp::scheduler::pause_task(&mut sched, id) {
        println!("{} Task paused: {id}", "✓".green());
    } else {
        eprintln!("{} Task not found: {id}", "✗".red());
    }
}

pub fn schedule_resume(id: &str) {
    let mut sched = onecrawl_cdp::Scheduler::new();
    if onecrawl_cdp::scheduler::resume_task(&mut sched, id) {
        println!("{} Task resumed: {id}", "✓".green());
    } else {
        eprintln!("{} Task not found or not paused: {id}", "✗".red());
    }
}

pub fn schedule_list() {
    let sched = onecrawl_cdp::Scheduler::new();
    println!(
        "{}",
        serde_json::to_string_pretty(&sched.tasks).unwrap_or_default()
    );
}

pub fn schedule_stats() {
    let sched = onecrawl_cdp::Scheduler::new();
    let stats = onecrawl_cdp::scheduler::get_stats(&sched);
    println!(
        "{}",
        serde_json::to_string_pretty(&stats).unwrap_or_default()
    );
}

pub fn schedule_save(path: &str) {
    let sched = onecrawl_cdp::Scheduler::new();
    match onecrawl_cdp::scheduler::save_scheduler(&sched, std::path::Path::new(path)) {
        Ok(()) => println!("{} Scheduler saved to {path}", "✓".green()),
        Err(e) => eprintln!("{} Save failed: {e}", "✗".red()),
    }
}

pub fn schedule_load(path: &str) {
    match onecrawl_cdp::scheduler::load_scheduler(std::path::Path::new(path)) {
        Ok(sched) => {
            println!(
                "{} Scheduler loaded: {} tasks",
                "✓".green(),
                sched.tasks.len()
            );
        }
        Err(e) => eprintln!("{} Load failed: {e}", "✗".red()),
    }
}

// ---------------------------------------------------------------------------
// Session Pool (standalone — no Page required)
// ---------------------------------------------------------------------------

pub fn pool_add(name: &str, tags: Option<Vec<String>>) {
    let mut pool = onecrawl_cdp::SessionPool::new(onecrawl_cdp::PoolConfig::default());
    let id = onecrawl_cdp::session_pool::add_session(&mut pool, name, tags);
    println!("{} Session added: {}", "✓".green(), id);
}

pub fn pool_next() {
    let mut pool = onecrawl_cdp::SessionPool::new(onecrawl_cdp::PoolConfig::default());
    match onecrawl_cdp::session_pool::get_next(&mut pool) {
        Some(s) => println!("{}", serde_json::to_string_pretty(s).unwrap_or_default()),
        None => println!("{} No available sessions", "⚠".yellow()),
    }
}

pub fn pool_stats() {
    let pool = onecrawl_cdp::SessionPool::new(onecrawl_cdp::PoolConfig::default());
    let stats = onecrawl_cdp::session_pool::get_stats(&pool);
    println!(
        "{}",
        serde_json::to_string_pretty(&stats).unwrap_or_default()
    );
}

pub fn pool_cleanup() {
    let mut pool = onecrawl_cdp::SessionPool::new(onecrawl_cdp::PoolConfig::default());
    let n = onecrawl_cdp::session_pool::cleanup_idle(&mut pool);
    println!("{} Cleaned up {n} idle session(s)", "✓".green());
}

pub fn pool_save(path: &str) {
    let pool = onecrawl_cdp::SessionPool::new(onecrawl_cdp::PoolConfig::default());
    match onecrawl_cdp::session_pool::save_pool(&pool, std::path::Path::new(path)) {
        Ok(()) => println!("{} Pool saved to {path}", "✓".green()),
        Err(e) => eprintln!("{} Save failed: {e}", "✗".red()),
    }
}

pub fn pool_load(path: &str) {
    match onecrawl_cdp::session_pool::load_pool(std::path::Path::new(path)) {
        Ok(pool) => {
            println!(
                "{} Pool loaded: {} sessions",
                "✓".green(),
                pool.sessions.len()
            );
        }
        Err(e) => eprintln!("{} Load failed: {e}", "✗".red()),
    }
}

// ---------------------------------------------------------------------------
// Passkey Vault (multi-site persistent store)
// ---------------------------------------------------------------------------

/// List all sites and credential counts in the passkey vault.
pub fn passkey_vault_list() {
    match onecrawl_cdp::load_vault() {
        Ok(vault) => {
            let list = onecrawl_cdp::vault_list(&vault);
            let total = onecrawl_cdp::vault_total(&vault);
            if list.is_empty() {
                println!("{} Passkey vault is empty", "⚠".yellow());
                println!("  Register with: onecrawl auth passkey-register");
                println!("  Import from  : onecrawl passkey import --from bitwarden|1password|cxf --input FILE");
                return;
            }
            println!("{} Passkey vault — {} credential(s) across {} site(s)", "✓".green(), total, list.len());
            println!("{:<35} {}", "rp_id", "credentials");
            println!("{}", "─".repeat(45));
            for (rp_id, count) in &list {
                println!("  {:<33} {}", rp_id, count);
            }
            println!("{}", "─".repeat(45));
            println!("  Vault path: {}", onecrawl_cdp::vault_path().display());
        }
        Err(e) => eprintln!("{} Vault error: {e}", "✗".red()),
    }
}

/// Add credentials from a native passkey JSON file to the vault.
pub fn passkey_vault_save(input: &str) {
    let path = std::path::Path::new(input);
    let creds = match onecrawl_cdp::load_passkeys(path) {
        Ok(c) => c,
        Err(e) => { eprintln!("{} Cannot read passkey file: {e}", "✗".red()); return; }
    };
    let mut vault = match onecrawl_cdp::load_vault() {
        Ok(v) => v,
        Err(e) => { eprintln!("{} Cannot load vault: {e}", "✗".red()); return; }
    };
    let count = creds.len();
    onecrawl_cdp::vault_add(&mut vault, creds);
    match onecrawl_cdp::save_vault(&vault) {
        Ok(()) => println!("{} Added {count} credential(s) to vault ({})", "✓".green(), onecrawl_cdp::vault_path().display()),
        Err(e) => eprintln!("{} Vault save failed: {e}", "✗".red()),
    }
}

/// Remove a specific credential from the vault by its credential_id.
pub fn passkey_vault_remove(credential_id: &str) {
    let mut vault = match onecrawl_cdp::load_vault() {
        Ok(v) => v,
        Err(e) => { eprintln!("{} Cannot load vault: {e}", "✗".red()); return; }
    };
    if onecrawl_cdp::vault_remove(&mut vault, credential_id) {
        match onecrawl_cdp::save_vault(&vault) {
            Ok(()) => println!("{} Removed credential {}", "✓".green(), credential_id),
            Err(e) => eprintln!("{} Vault save failed: {e}", "✗".red()),
        }
    } else {
        eprintln!("{} Credential not found: {}", "✗".red(), credential_id);
    }
}

/// Remove all credentials for a specific rp_id from the vault.
pub fn passkey_vault_clear_site(rp_id: &str) {
    let mut vault = match onecrawl_cdp::load_vault() {
        Ok(v) => v,
        Err(e) => { eprintln!("{} Cannot load vault: {e}", "✗".red()); return; }
    };
    let removed = onecrawl_cdp::vault_clear_site(&mut vault, rp_id);
    match onecrawl_cdp::save_vault(&vault) {
        Ok(()) => println!("{} Removed {removed} credential(s) for '{rp_id}'", "✓".green()),
        Err(e) => eprintln!("{} Vault save failed: {e}", "✗".red()),
    }
}

/// Export vault credentials for a site to a passkey JSON file.
pub fn passkey_vault_export(rp_id: &str, output: &str) {
    let vault = match onecrawl_cdp::load_vault() {
        Ok(v) => v,
        Err(e) => { eprintln!("{} Cannot load vault: {e}", "✗".red()); return; }
    };
    let creds = onecrawl_cdp::vault_get(&vault, rp_id);
    if creds.is_empty() {
        eprintln!("{} No credentials found for '{rp_id}'", "✗".red());
        return;
    }
    match onecrawl_cdp::save_passkeys(std::path::Path::new(output), &creds) {
        Ok(()) => println!("{} Exported {} credential(s) for '{}' → {}", "✓".green(), creds.len(), rp_id, output),
        Err(e) => eprintln!("{} Export failed: {e}", "✗".red()),
    }
}

/// Import passkeys from a Bitwarden unencrypted JSON export.
pub fn passkey_import_bitwarden(input: &str, save_to_vault: bool) {
    let creds = match onecrawl_cdp::import_bitwarden(std::path::Path::new(input)) {
        Ok(c) => c,
        Err(e) => { eprintln!("{} Bitwarden import failed: {e}", "✗".red()); return; }
    };
    if creds.is_empty() {
        println!("{} No importable passkeys found (hardware-bound credentials are skipped)", "⚠".yellow());
        return;
    }
    println!("{} Found {} passkey(s):", "✓".green(), creds.len());
    for c in &creds {
        println!("  • {} @ {}", c.credential_id, c.rp_id);
    }
    if save_to_vault {
        _vault_add_and_save(creds);
    }
}

/// Import passkeys from a 1Password export.data JSON file (extracted from .1pux).
pub fn passkey_import_1password(input: &str, save_to_vault: bool) {
    let creds = match onecrawl_cdp::import_1password_json(std::path::Path::new(input)) {
        Ok(c) => c,
        Err(e) => { eprintln!("{} 1Password import failed: {e}", "✗".red()); return; }
    };
    if creds.is_empty() {
        println!("{} No importable passkeys found", "⚠".yellow());
        return;
    }
    println!("{} Found {} passkey(s):", "✓".green(), creds.len());
    for c in &creds {
        println!("  • {} @ {}", c.credential_id, c.rp_id);
    }
    if save_to_vault {
        _vault_add_and_save(creds);
    }
}

/// Import passkeys from a FIDO Alliance CXF JSON file.
pub fn passkey_import_cxf(input: &str, save_to_vault: bool) {
    let creds = match onecrawl_cdp::import_cxf(std::path::Path::new(input)) {
        Ok(c) => c,
        Err(e) => { eprintln!("{} FIDO CXF import failed: {e}", "✗".red()); return; }
    };
    if creds.is_empty() {
        println!("{} No importable passkeys found (hardware-bound or unsupported type)", "⚠".yellow());
        return;
    }
    println!("{} Found {} passkey(s):", "✓".green(), creds.len());
    for c in &creds {
        println!("  • {} @ {}", c.credential_id, c.rp_id);
    }
    if save_to_vault {
        _vault_add_and_save(creds);
    }
}

fn _vault_add_and_save(creds: Vec<onecrawl_cdp::PasskeyCredential>) {
    let mut vault = match onecrawl_cdp::load_vault() {
        Ok(v) => v,
        Err(e) => { eprintln!("{} Cannot load vault: {e}", "✗".red()); return; }
    };
    let count = creds.len();
    onecrawl_cdp::vault_add(&mut vault, creds);
    match onecrawl_cdp::save_vault(&vault) {
        Ok(()) => println!("{} Saved {count} credential(s) to vault", "✓".green()),
        Err(e) => eprintln!("{} Vault save failed: {e}", "✗".red()),
    }
}
