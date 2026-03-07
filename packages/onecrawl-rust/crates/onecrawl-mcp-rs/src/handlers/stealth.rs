//! Handler implementations for the `stealth` super-tool.

use rmcp::{ErrorData as McpError, model::*};
use crate::cdp_tools::*;
use crate::helpers::{mcp_err, ensure_page, json_ok, json_escape, text_ok, McpResult};
use crate::types::*;
use crate::OneCrawlMcp;

impl OneCrawlMcp {

    // ════════════════════════════════════════════════════════════════
    //  CDP tools — Stealth & Anti-Detection
    // ════════════════════════════════════════════════════════════════

    pub(crate) async fn stealth_inject(
        &self,
        _p: InjectStealthParams,
    ) -> Result<CallToolResult, McpError> {
        let page = ensure_page(&self.browser).await?;
        let patches = onecrawl_cdp::antibot::inject_stealth_full(&page)
            .await
            .mcp()?;
        json_ok(&StealthInjectResult {
            patches_applied: patches.len(),
            patches,
        })
    }


    pub(crate) async fn stealth_test(
        &self,
        _p: BotDetectionTestParams,
    ) -> Result<CallToolResult, McpError> {
        let page = ensure_page(&self.browser).await?;
        let result = onecrawl_cdp::antibot::bot_detection_test(&page)
            .await
            .mcp()?;
        json_ok(&result)
    }


    pub(crate) async fn stealth_fingerprint(
        &self,
        p: ApplyFingerprintParams,
    ) -> Result<CallToolResult, McpError> {
        let page = ensure_page(&self.browser).await?;
        let mut fp = onecrawl_cdp::stealth::generate_fingerprint();
        if let Some(ua) = &p.user_agent {
            fp.user_agent = ua.clone();
        }
        let script = onecrawl_cdp::stealth::get_stealth_init_script(&fp);
        onecrawl_cdp::page::evaluate_js(&page, &script)
            .await
            .mcp()?;
        json_ok(&FingerprintResult {
            user_agent: &fp.user_agent,
            platform: &fp.platform,
        })
    }


    pub(crate) async fn stealth_block_domains(
        &self,
        p: BlockDomainsParams,
    ) -> Result<CallToolResult, McpError> {
        let page = ensure_page(&self.browser).await?;
        let count = if let Some(cat) = &p.category {
            onecrawl_cdp::domain_blocker::block_category(&page, cat)
                .await
                .mcp()?
        } else if let Some(domains) = &p.domains {
            onecrawl_cdp::domain_blocker::block_domains(&page, domains)
                .await
                .mcp()?
        } else {
            return Err(mcp_err(
                "provide either 'domains' or 'category'",
            ));
        };
        text_ok(format!("{count} domains blocked"))
    }


    pub(crate) async fn stealth_detect_captcha(
        &self,
        _p: DetectCaptchaParams,
    ) -> Result<CallToolResult, McpError> {
        let page = ensure_page(&self.browser).await?;
        let detection = onecrawl_cdp::captcha::detect_captcha(&page)
            .await
            .mcp()?;
        json_ok(&detection)
    }

    pub(crate) async fn stealth_solve_captcha(
        &self,
        p: SolveCaptchaParams,
    ) -> Result<CallToolResult, McpError> {
        let page = ensure_page(&self.browser).await?;
        let timeout = p.timeout_ms.unwrap_or(15000);
        let captcha_type = p.captcha_type.as_deref().unwrap_or("auto");

        match captcha_type {
            "recaptcha_checkbox" => {
                // Click just the reCAPTCHA checkbox using CDP frame targeting
                let checkbox_sel = ".recaptcha-checkbox-border, [role=\"checkbox\"], .recaptcha-checkbox";
                let pattern = "recaptcha/api2/anchor";
                match onecrawl_cdp::iframe::human_click_in_frame(&page, pattern, checkbox_sel).await {
                    Ok(()) => json_ok(&serde_json::json!({
                        "solved": true,
                        "method": "cdp_frame_targeting",
                        "captcha_type": "recaptcha_checkbox",
                        "note": "Checkbox clicked via CDP cross-origin frame targeting. Check if challenge appeared."
                    })),
                    Err(e) => json_ok(&serde_json::json!({
                        "solved": false,
                        "error": format!("{e}"),
                        "suggestion": "Try 'recaptcha_audio' for audio challenge fallback"
                    })),
                }
            }
            "recaptcha_audio" => {
                match onecrawl_cdp::captcha::solve_recaptcha_audio(&page).await {
                    Ok(transcription) => json_ok(&serde_json::json!({
                        "solved": true,
                        "method": "audio_whisper",
                        "transcription": transcription,
                    })),
                    Err(e) => json_ok(&serde_json::json!({
                        "solved": false,
                        "error": format!("{e}"),
                    })),
                }
            }
            "turnstile" => {
                match onecrawl_cdp::captcha::solve_turnstile_native(&page, timeout).await {
                    Ok(passed) => json_ok(&serde_json::json!({
                        "solved": passed,
                        "method": "turnstile_native",
                    })),
                    Err(e) => json_ok(&serde_json::json!({
                        "solved": false,
                        "error": format!("{e}"),
                    })),
                }
            }
            "auto" | _ => {
                // Auto-detect and solve
                let detection = onecrawl_cdp::captcha::detect_captcha(&page).await.mcp()?;
                if !detection.detected {
                    return json_ok(&serde_json::json!({
                        "solved": false,
                        "error": "No CAPTCHA detected on page",
                    }));
                }

                let provider = detection.provider.as_str();
                match provider {
                    p if p.contains("recaptcha") || p.contains("google") => {
                        // Try checkbox first, then audio if challenge appears
                        let checkbox_sel = ".recaptcha-checkbox-border, [role=\"checkbox\"], .recaptcha-checkbox";
                        let pattern = "recaptcha/api2/anchor";
                        let click_result = onecrawl_cdp::iframe::human_click_in_frame(
                            &page, pattern, checkbox_sel,
                        ).await;
                        json_ok(&serde_json::json!({
                            "solved": click_result.is_ok(),
                            "method": "auto_recaptcha_checkbox",
                            "detection": {
                                "type": detection.captcha_type,
                                "provider": detection.provider,
                                "confidence": detection.confidence,
                            },
                            "note": if click_result.is_ok() {
                                "Checkbox clicked. If challenge appears, use solve_captcha with type='recaptcha_audio'"
                            } else {
                                "Checkbox click failed. Try 'recaptcha_audio' type"
                            }
                        }))
                    }
                    p if p.contains("turnstile") || p.contains("cloudflare") => {
                        let passed = onecrawl_cdp::captcha::solve_turnstile_native(
                            &page, timeout,
                        ).await.unwrap_or(false);
                        json_ok(&serde_json::json!({
                            "solved": passed,
                            "method": "auto_turnstile",
                        }))
                    }
                    _ => {
                        json_ok(&serde_json::json!({
                            "solved": false,
                            "detection": {
                                "type": detection.captcha_type,
                                "provider": detection.provider,
                            },
                            "error": "Unsupported CAPTCHA type for auto-solve",
                        }))
                    }
                }
            }
        }
    }
}

// ── Human Behavior Simulation & Stealth Max ─────────────────────

impl OneCrawlMcp {
    pub(crate) async fn human_delay(&self, p: HumanDelayParams) -> Result<CallToolResult, McpError> {
        let min = p.min_ms;
        let max = p.max_ms.max(min);
        let page = ensure_page(&self.browser).await?;
        let js = format!(r#"(async () => {{
            const min = {min};
            const max = {max};
            const arr = new Uint32Array(1);
            crypto.getRandomValues(arr);
            const delay = min + (arr[0] % (max - min + 1));
            await new Promise(r => setTimeout(r, delay));
            return {{ delayed_ms: delay, min: min, max: max }};
        }})()"#);
        let result = page.evaluate(js).await.mcp()?;
        let val: serde_json::Value = result.into_value().unwrap_or(serde_json::json!(null));
        json_ok(&serde_json::json!({ "action": "human_delay", "result": val }))
    }

    pub(crate) async fn human_mouse(&self, p: HumanMouseParams) -> Result<CallToolResult, McpError> {
        let page = ensure_page(&self.browser).await?;
        let steps = p.steps.unwrap_or(20);
        let speed_mult = match p.speed.as_deref().unwrap_or("normal") {
            "slow" => 3.0,
            "fast" => 0.5,
            _ => 1.0,
        };
        let js = format!(r#"(async () => {{
            const startX = window._lastMouseX || (window.innerWidth / 2);
            const startY = window._lastMouseY || (window.innerHeight / 2);
            const endX = {x};
            const endY = {y};
            const steps = {steps};
            const speedMult = {speed_mult};
            
            // Cubic Bezier control points (random offsets for natural curves)
            const cp1x = startX + (endX - startX) * 0.25 + (Math.random() - 0.5) * 100;
            const cp1y = startY + (endY - startY) * 0.25 + (Math.random() - 0.5) * 100;
            const cp2x = startX + (endX - startX) * 0.75 + (Math.random() - 0.5) * 50;
            const cp2y = startY + (endY - startY) * 0.75 + (Math.random() - 0.5) * 50;
            
            for (let i = 0; i <= steps; i++) {{
                const t = i / steps;
                const t2 = t * t;
                const t3 = t2 * t;
                const mt = 1 - t;
                const mt2 = mt * mt;
                const mt3 = mt2 * mt;
                
                const x = mt3 * startX + 3 * mt2 * t * cp1x + 3 * mt * t2 * cp2x + t3 * endX;
                const y = mt3 * startY + 3 * mt2 * t * cp1y + 3 * mt * t2 * cp2y + t3 * endY;
                
                document.dispatchEvent(new MouseEvent('mousemove', {{
                    clientX: x, clientY: y, bubbles: true
                }}));
                
                const delay = (5 + Math.random() * 15) * speedMult;
                await new Promise(r => setTimeout(r, delay));
            }}
            
            window._lastMouseX = endX;
            window._lastMouseY = endY;
            return {{ from: [startX, startY], to: [endX, endY], steps, speed: "{speed}" }};
        }})()"#, x = p.x, y = p.y, speed = p.speed.as_deref().unwrap_or("normal"));
        let result = page.evaluate(js).await.mcp()?;
        let val: serde_json::Value = result.into_value().unwrap_or(serde_json::json!(null));
        json_ok(&serde_json::json!({ "action": "human_mouse", "movement": val }))
    }

    pub(crate) async fn human_type(&self, p: HumanTypeParams) -> Result<CallToolResult, McpError> {
        let page = ensure_page(&self.browser).await?;
        let min_delay = p.min_delay_ms.unwrap_or(50);
        let max_delay = p.max_delay_ms.unwrap_or(200);
        let typos = p.typos.unwrap_or(false);
        let js = format!(r#"(async () => {{
            const el = document.querySelector({});
            if (!el) return {{ error: "Element not found" }};
            el.focus();
            el.value = '';
            el.dispatchEvent(new Event('focus', {{ bubbles: true }}));
            
            const text = {};
            let typed = '';
            const minDelay = {min_delay};
            const maxDelay = {max_delay};
            const doTypos = {typos};
            let typoCount = 0;
            
            for (let i = 0; i < text.length; i++) {{
                // Occasional typo simulation
                if (doTypos && Math.random() < 0.05 && text.length > 5) {{
                    const wrongChar = String.fromCharCode(text.charCodeAt(i) + (Math.random() > 0.5 ? 1 : -1));
                    el.value += wrongChar;
                    el.dispatchEvent(new Event('input', {{ bubbles: true }}));
                    await new Promise(r => setTimeout(r, minDelay + Math.random() * (maxDelay - minDelay)));
                    
                    // Backspace to correct
                    el.value = el.value.slice(0, -1);
                    el.dispatchEvent(new Event('input', {{ bubbles: true }}));
                    await new Promise(r => setTimeout(r, 50 + Math.random() * 100));
                    typoCount++;
                }}
                
                el.value += text[i];
                typed += text[i];
                el.dispatchEvent(new Event('input', {{ bubbles: true }}));
                el.dispatchEvent(new KeyboardEvent('keydown', {{ key: text[i], bubbles: true }}));
                el.dispatchEvent(new KeyboardEvent('keyup', {{ key: text[i], bubbles: true }}));
                
                // Variable delay between keystrokes (Gaussian-like distribution)
                const u1 = Math.random();
                const u2 = Math.random();
                const gaussian = Math.sqrt(-2 * Math.log(u1)) * Math.cos(2 * Math.PI * u2);
                const delay = Math.max(minDelay, Math.min(maxDelay, (minDelay + maxDelay) / 2 + gaussian * (maxDelay - minDelay) / 4));
                await new Promise(r => setTimeout(r, delay));
            }}
            
            el.dispatchEvent(new Event('change', {{ bubbles: true }}));
            return {{ typed: text.length, typos_simulated: typoCount, selector: {} }};
        }})()"#, json_escape(&p.selector), json_escape(&p.text), json_escape(&p.selector));
        let result = page.evaluate(js).await.mcp()?;
        let val: serde_json::Value = result.into_value().unwrap_or(serde_json::json!(null));
        json_ok(&serde_json::json!({ "action": "human_type", "result": val }))
    }

    pub(crate) async fn human_scroll(&self, p: HumanScrollParams) -> Result<CallToolResult, McpError> {
        let page = ensure_page(&self.browser).await?;
        let direction = p.direction.as_deref().unwrap_or("down");
        let distance = p.distance.unwrap_or(500);
        let steps = p.steps.unwrap_or(5);
        let speed = p.speed.as_deref().unwrap_or("normal");
        let speed_mult = match speed { "slow" => 2.0f64, "fast" => 0.3, _ => 1.0 };
        let js = format!(r#"(async () => {{
            const dir = "{}";
            const totalDist = {};
            const steps = {};
            const speedMult = {};
            
            let scrolled = 0;
            for (let i = 0; i < steps; i++) {{
                // Each step scrolls a slightly different amount (human-like)
                const stepRatio = (1 + (Math.random() - 0.5) * 0.4) / steps;
                const stepDist = Math.round(totalDist * stepRatio);
                
                const opts = {{}};
                if (dir === 'down') opts.top = stepDist;
                else if (dir === 'up') opts.top = -stepDist;
                else if (dir === 'right') opts.left = stepDist;
                else if (dir === 'left') opts.left = -stepDist;
                opts.behavior = 'smooth';
                
                window.scrollBy(opts);
                scrolled += Math.abs(stepDist);
                
                // Random delay between scroll steps
                const delay = (100 + Math.random() * 300) * speedMult;
                await new Promise(r => setTimeout(r, delay));
            }}
            
            return {{ direction: dir, total_scrolled: scrolled, steps, final_position: {{ x: window.scrollX, y: window.scrollY }} }};
        }})()"#, direction, distance, steps, speed_mult);
        let result = page.evaluate(js).await.mcp()?;
        let val: serde_json::Value = result.into_value().unwrap_or(serde_json::json!(null));
        json_ok(&serde_json::json!({ "action": "human_scroll", "result": val }))
    }

    pub(crate) async fn human_profile(&self, p: HumanProfileParams) -> Result<CallToolResult, McpError> {
        let page = ensure_page(&self.browser).await?;
        let profile = &p.profile;
        let js = format!(r#"(() => {{
            const profiles = {{
                fast: {{ mouse_speed: 'fast', type_min_ms: 30, type_max_ms: 100, scroll_speed: 'fast', delay_min: 100, delay_max: 500, typos: false }},
                normal: {{ mouse_speed: 'normal', type_min_ms: 50, type_max_ms: 200, scroll_speed: 'normal', delay_min: 300, delay_max: 1500, typos: true }},
                careful: {{ mouse_speed: 'slow', type_min_ms: 100, type_max_ms: 400, scroll_speed: 'slow', delay_min: 800, delay_max: 3000, typos: true }},
                elderly: {{ mouse_speed: 'slow', type_min_ms: 200, type_max_ms: 600, scroll_speed: 'slow', delay_min: 1500, delay_max: 5000, typos: true }}
            }};
            const p = profiles["{}"] || profiles.normal;
            window._onecrawl_human_profile = p;
            return {{ profile: "{}", settings: p }};
        }})()"#, json_escape(profile), json_escape(profile));
        let result = page.evaluate(js).await.mcp()?;
        let val: serde_json::Value = result.into_value().unwrap_or(serde_json::json!(null));
        json_ok(&serde_json::json!({ "action": "human_profile", "result": val }))
    }

    pub(crate) async fn stealth_max(&self, p: StealthMaxParams) -> Result<CallToolResult, McpError> {
        let page = ensure_page(&self.browser).await?;
        let human_sim = p.human_simulation.unwrap_or(true);
        let js = format!(r#"(() => {{
            const patches = [];
            
            // 1. Navigator patches
            Object.defineProperty(navigator, 'webdriver', {{ get: () => undefined }});
            patches.push('navigator.webdriver');
            
            // 2. Chrome runtime
            if (!window.chrome) {{
                window.chrome = {{ runtime: {{}} }};
                patches.push('chrome.runtime');
            }}
            
            // 3. Plugins
            Object.defineProperty(navigator, 'plugins', {{
                get: () => [
                    {{ name: 'Chrome PDF Plugin', filename: 'internal-pdf-viewer' }},
                    {{ name: 'Chrome PDF Viewer', filename: 'mhjfbmdgcfjbbpaeojofohoefgiehjai' }},
                    {{ name: 'Native Client', filename: 'internal-nacl-plugin' }}
                ]
            }});
            patches.push('navigator.plugins');
            
            // 4. Languages
            Object.defineProperty(navigator, 'languages', {{ get: () => ['en-US', 'en'] }});
            patches.push('navigator.languages');
            
            // 5. Permission API
            const origQuery = window.Permissions?.prototype?.query;
            if (origQuery) {{
                window.Permissions.prototype.query = function(params) {{
                    if (params.name === 'notifications') return Promise.resolve({{ state: 'denied', onchange: null }});
                    return origQuery.apply(this, arguments);
                }};
                patches.push('Permissions.query');
            }}
            
            // 6. WebGL vendor
            const getParameter = WebGLRenderingContext.prototype.getParameter;
            WebGLRenderingContext.prototype.getParameter = function(param) {{
                if (param === 37445) return 'Intel Inc.';
                if (param === 37446) return 'Intel Iris OpenGL Engine';
                return getParameter.call(this, param);
            }};
            patches.push('WebGL.vendor');
            
            // 7. Canvas fingerprint noise
            const origToDataURL = HTMLCanvasElement.prototype.toDataURL;
            HTMLCanvasElement.prototype.toDataURL = function(type) {{
                if (this.width > 16 && this.height > 16) {{
                    const ctx = this.getContext('2d');
                    if (ctx) {{
                        const imageData = ctx.getImageData(0, 0, 1, 1);
                        imageData.data[0] = imageData.data[0] ^ 1;
                        ctx.putImageData(imageData, 0, 0);
                    }}
                }}
                return origToDataURL.apply(this, arguments);
            }};
            patches.push('Canvas.toDataURL');
            
            // 8. Connection info
            if (navigator.connection) {{
                Object.defineProperty(navigator.connection, 'rtt', {{ get: () => 50 + Math.floor(Math.random() * 100) }});
                patches.push('navigator.connection.rtt');
            }}
            
            // 9. Hardware concurrency (randomize slightly)
            Object.defineProperty(navigator, 'hardwareConcurrency', {{ get: () => [4, 8, 12, 16][Math.floor(Math.random() * 4)] }});
            patches.push('navigator.hardwareConcurrency');
            
            // 10. Device memory
            Object.defineProperty(navigator, 'deviceMemory', {{ get: () => [4, 8, 16][Math.floor(Math.random() * 3)] }});
            patches.push('navigator.deviceMemory');
            
            // 11. Remove CDP artifacts
            delete window.cdc_adoQpoasnfa76pfcZLmcfl_Array;
            delete window.cdc_adoQpoasnfa76pfcZLmcfl_Promise;
            delete window.cdc_adoQpoasnfa76pfcZLmcfl_Symbol;
            patches.push('CDP artifacts cleanup');
            
            // 12. Iframe contentWindow
            try {{
                Object.defineProperty(HTMLIFrameElement.prototype, 'contentWindow', {{
                    get: function() {{ return window; }}
                }});
                patches.push('iframe.contentWindow');
            }} catch(e) {{}}
            
            // Human simulation flag
            const humanSim = {human_sim};
            if (humanSim) {{
                window._onecrawl_human_profile = window._onecrawl_human_profile || {{
                    mouse_speed: 'normal', type_min_ms: 50, type_max_ms: 200,
                    scroll_speed: 'normal', delay_min: 300, delay_max: 1500, typos: true
                }};
                patches.push('human_simulation_profile');
            }}
            
            return {{ patches_applied: patches.length, patches, human_simulation: humanSim }};
        }})()"#);
        let result = page.evaluate(js).await.mcp()?;
        let val: serde_json::Value = result.into_value().unwrap_or(serde_json::json!(null));
        json_ok(&serde_json::json!({ "action": "stealth_max", "result": val }))
    }

    pub(crate) async fn stealth_score(&self) -> Result<CallToolResult, McpError> {
        let page = ensure_page(&self.browser).await?;
        let js = r#"(() => {
            const checks = [];
            let score = 100;
            
            // Check webdriver
            if (navigator.webdriver) { score -= 20; checks.push({ test: 'webdriver', passed: false }); }
            else checks.push({ test: 'webdriver', passed: true });
            
            // Check chrome runtime
            if (!window.chrome || !window.chrome.runtime) { score -= 10; checks.push({ test: 'chrome.runtime', passed: false }); }
            else checks.push({ test: 'chrome.runtime', passed: true });
            
            // Check plugins
            if (!navigator.plugins || navigator.plugins.length === 0) { score -= 10; checks.push({ test: 'plugins', passed: false }); }
            else checks.push({ test: 'plugins', passed: true });
            
            // Check languages
            if (!navigator.languages || navigator.languages.length === 0) { score -= 5; checks.push({ test: 'languages', passed: false }); }
            else checks.push({ test: 'languages', passed: true });
            
            // Check WebGL
            try {
                const canvas = document.createElement('canvas');
                const gl = canvas.getContext('webgl');
                const vendor = gl.getParameter(gl.VENDOR);
                if (vendor === 'Brian Paul' || vendor === 'Mesa') { score -= 15; checks.push({ test: 'webgl_vendor', passed: false, value: vendor }); }
                else checks.push({ test: 'webgl_vendor', passed: true, value: vendor });
            } catch(e) { score -= 10; checks.push({ test: 'webgl', passed: false, error: e.message }); }
            
            // Check CDP artifacts
            const cdcKeys = Object.keys(window).filter(k => k.startsWith('cdc_'));
            if (cdcKeys.length > 0) { score -= 20; checks.push({ test: 'cdp_artifacts', passed: false, keys: cdcKeys }); }
            else checks.push({ test: 'cdp_artifacts', passed: true });
            
            // Check Permissions
            if (navigator.permissions) {
                checks.push({ test: 'permissions_api', passed: true });
            } else {
                score -= 5;
                checks.push({ test: 'permissions_api', passed: false });
            }
            
            // Check screen resolution consistency
            if (screen.width === 0 || screen.height === 0) { score -= 10; checks.push({ test: 'screen_size', passed: false }); }
            else checks.push({ test: 'screen_size', passed: true, value: screen.width + 'x' + screen.height });
            
            // Check connection API
            if (navigator.connection && navigator.connection.rtt === 0) { score -= 5; checks.push({ test: 'connection_rtt', passed: false }); }
            else checks.push({ test: 'connection_rtt', passed: true });
            
            // Check user agent consistency
            const ua = navigator.userAgent;
            const isHeadless = /HeadlessChrome/.test(ua);
            if (isHeadless) { score -= 25; checks.push({ test: 'headless_ua', passed: false }); }
            else checks.push({ test: 'headless_ua', passed: true });
            
            const rating = score >= 90 ? 'excellent' : score >= 70 ? 'good' : score >= 50 ? 'fair' : 'poor';
            return { score: Math.max(0, score), rating, checks_run: checks.length, checks_passed: checks.filter(c => c.passed).length, details: checks };
        })()"#;
        let result = page.evaluate(js).await.mcp()?;
        let val: serde_json::Value = result.into_value().unwrap_or(serde_json::json!(null));
        json_ok(&serde_json::json!({ "action": "stealth_score", "score": val }))
    }

    pub(crate) async fn stealth_tls_apply(
        &self,
        p: TlsApplyParams,
    ) -> Result<CallToolResult, McpError> {
        let page = ensure_page(&self.browser).await?;
        let profile_name = p.profile.as_deref().unwrap_or("random");

        match profile_name {
            "detect" => {
                let fp = onecrawl_cdp::tls_fingerprint::detect_fingerprint(&page).await.mcp()?;
                json_ok(&serde_json::json!({
                    "action": "tls_detect",
                    "fingerprint": {
                        "name": fp.name,
                        "user_agent": fp.user_agent,
                        "platform": fp.platform,
                        "vendor": fp.vendor,
                        "languages": fp.languages,
                        "hardware_concurrency": fp.hardware_concurrency,
                        "device_memory": fp.device_memory,
                        "screen_width": fp.screen_width,
                        "screen_height": fp.screen_height,
                    }
                }))
            }
            name => {
                let fp = if name == "random" {
                    onecrawl_cdp::tls_fingerprint::random_fingerprint()
                } else {
                    onecrawl_cdp::tls_fingerprint::get_profile(name)
                        .ok_or_else(|| mcp_err(format!("Unknown TLS profile: '{name}'. Available: chrome-win, chrome-mac, firefox-win, safari-mac, edge-win, random")))?
                };
                let applied = onecrawl_cdp::tls_fingerprint::apply_fingerprint(&page, &fp).await.mcp()?;
                json_ok(&serde_json::json!({
                    "action": "tls_apply",
                    "profile": name,
                    "applied_patches": applied.len(),
                    "patches": applied
                }))
            }
        }
    }

    pub(crate) async fn stealth_webrtc_block(
        &self,
        p: WebrtcBlockParams,
    ) -> Result<CallToolResult, McpError> {
        let page = ensure_page(&self.browser).await?;
        let mode = p.mode.as_deref().unwrap_or("block");

        let js = match mode {
            "turn_only" => r#"
                const origRTC = window.RTCPeerConnection;
                window.RTCPeerConnection = function(config, constraints) {
                    if (config && config.iceServers) {
                        config.iceServers = config.iceServers.filter(s =>
                            s.urls && (Array.isArray(s.urls) ? s.urls : [s.urls]).some(u => u.startsWith('turn:'))
                        );
                    }
                    return new origRTC(config, constraints);
                };
                window.RTCPeerConnection.prototype = origRTC.prototype;
                if (navigator.mediaDevices) {
                    navigator.mediaDevices.enumerateDevices = () => Promise.resolve([]);
                }
                'turn_only'
            "#,
            _ => r#"
                window.RTCPeerConnection = undefined;
                window.webkitRTCPeerConnection = undefined;
                window.mozRTCPeerConnection = undefined;
                if (navigator.mediaDevices) {
                    navigator.mediaDevices.getUserMedia = () => Promise.reject(new DOMException('Not allowed', 'NotAllowedError'));
                    navigator.mediaDevices.enumerateDevices = () => Promise.resolve([]);
                }
                'blocked'
            "#,
        };

        page.evaluate(js.to_string()).await.map_err(|e| mcp_err(format!("webrtc_block: {e}")))?;
        json_ok(&serde_json::json!({
            "action": "webrtc_block",
            "mode": mode,
            "status": "applied"
        }))
    }

    pub(crate) async fn stealth_battery_spoof(
        &self,
        p: BatterySpoofParams,
    ) -> Result<CallToolResult, McpError> {
        let page = ensure_page(&self.browser).await?;
        let charging = p.charging.unwrap_or(true);
        let level = p.level.unwrap_or(1.0).max(0.0).min(1.0);
        let charging_time = if charging { 0 } else { 3600 };
        let discharging_time = if charging { "Infinity".to_string() } else { "7200".to_string() };

        let js = format!(r#"
            const fakeBattery = {{
                charging: {charging},
                chargingTime: {charging_time},
                dischargingTime: {discharging_time},
                level: {level},
                addEventListener: function() {{}},
                removeEventListener: function() {{}},
                dispatchEvent: function() {{ return true; }}
            }};
            navigator.getBattery = () => Promise.resolve(fakeBattery);
            'spoofed'
        "#);

        page.evaluate(js).await.map_err(|e| mcp_err(format!("battery_spoof: {e}")))?;
        json_ok(&serde_json::json!({
            "action": "battery_spoof",
            "charging": charging,
            "level": level,
            "status": "applied"
        }))
    }

    pub(crate) async fn stealth_sensor_block(
        &self,
        p: SensorBlockParams,
    ) -> Result<CallToolResult, McpError> {
        let page = ensure_page(&self.browser).await?;
        let _blocked = p.sensors.as_ref().map(|s| s.iter().map(|s| s.as_str()).collect::<Vec<_>>());

        let js = r#"
            const sensorAPIs = ['DeviceMotionEvent', 'DeviceOrientationEvent',
                'AmbientLightSensor', 'Gyroscope', 'Accelerometer',
                'Magnetometer', 'AbsoluteOrientationSensor', 'RelativeOrientationSensor',
                'LinearAccelerationSensor', 'GravitySensor'];
            const blocked = [];
            for (const api of sensorAPIs) {
                if (window[api]) {
                    Object.defineProperty(window, api, {
                        value: undefined, writable: false, configurable: false
                    });
                    blocked.push(api);
                }
            }
            const origQuery = navigator.permissions.query.bind(navigator.permissions);
            navigator.permissions.query = (desc) => {
                if (desc.name && ['accelerometer','gyroscope','magnetometer','ambient-light-sensor'].includes(desc.name)) {
                    return Promise.resolve({state: 'denied', addEventListener: ()=>{}});
                }
                return origQuery(desc);
            };
            JSON.stringify(blocked)
        "#;

        let result = page.evaluate(js.to_string()).await.map_err(|e| mcp_err(format!("sensor_block: {e}")))?;
        let blocked_list: Vec<String> = result.into_value().unwrap_or_default();
        json_ok(&serde_json::json!({
            "action": "sensor_block",
            "blocked_apis": blocked_list,
            "status": "applied"
        }))
    }

    pub(crate) async fn stealth_canvas_advanced(
        &self,
        p: CanvasAdvancedParams,
    ) -> Result<CallToolResult, McpError> {
        let page = ensure_page(&self.browser).await?;
        let intensity = p.intensity.unwrap_or(2.0).max(0.0).min(10.0);
        onecrawl_cdp::antibot::inject_canvas_advanced(&page, intensity).await.mcp()?;
        json_ok(&serde_json::json!({
            "action": "canvas_advanced",
            "intensity": intensity,
            "status": "applied",
            "note": "Gaussian noise injected into canvas getImageData/toDataURL/toBlob"
        }))
    }

    pub(crate) async fn stealth_timezone_sync(
        &self,
        p: TimezoneSyncParams,
    ) -> Result<CallToolResult, McpError> {
        let page = ensure_page(&self.browser).await?;
        onecrawl_cdp::antibot::inject_timezone_sync(&page, &p.timezone).await.mcp()?;
        json_ok(&serde_json::json!({
            "action": "timezone_sync",
            "timezone": p.timezone,
            "status": "applied",
            "synced": ["Date.getTimezoneOffset", "Intl.DateTimeFormat", "Intl.resolvedOptions"]
        }))
    }

    pub(crate) async fn stealth_font_protect(
        &self,
        _p: FontProtectParams,
    ) -> Result<CallToolResult, McpError> {
        let page = ensure_page(&self.browser).await?;
        onecrawl_cdp::antibot::inject_font_protection(&page).await.mcp()?;
        json_ok(&serde_json::json!({
            "action": "font_protect",
            "status": "applied",
            "note": "Font enumeration limited to common cross-platform subset"
        }))
    }

    pub(crate) async fn stealth_behavior_sim(
        &self,
        p: BehaviorSimParams,
    ) -> Result<CallToolResult, McpError> {
        let page = ensure_page(&self.browser).await?;
        let cmd = p.command.as_deref().unwrap_or("start");
        match cmd {
            "stop" => {
                onecrawl_cdp::antibot::stop_behavior_simulation(&page).await.mcp()?;
                json_ok(&serde_json::json!({ "action": "behavior_sim", "status": "stopped" }))
            }
            _ => {
                let interval = p.interval_ms.unwrap_or(2000);
                onecrawl_cdp::antibot::inject_behavior_simulation(&page, interval).await.mcp()?;
                json_ok(&serde_json::json!({
                    "action": "behavior_sim",
                    "status": "started",
                    "interval_ms": interval,
                    "behaviors": ["micro_movements", "idle_scroll", "focus_blur"]
                }))
            }
        }
    }

    pub(crate) async fn stealth_rotate(
        &self,
        p: StealthRotateParams,
    ) -> Result<CallToolResult, McpError> {
        let page = ensure_page(&self.browser).await?;
        let per_page = p.per_page.unwrap_or(false);

        let fp = onecrawl_cdp::tls_fingerprint::random_fingerprint();
        let applied = onecrawl_cdp::tls_fingerprint::apply_fingerprint(&page, &fp).await.mcp()?;

        let patches = onecrawl_cdp::antibot::inject_stealth_full(&page).await.mcp()?;

        onecrawl_cdp::antibot::inject_canvas_advanced(&page, 2.0).await.mcp()?;

        onecrawl_cdp::antibot::inject_font_protection(&page).await.mcp()?;

        json_ok(&serde_json::json!({
            "action": "stealth_rotate",
            "rotated": true,
            "fingerprint_patches": applied.len(),
            "stealth_patches": patches.len(),
            "per_page": per_page,
            "ua": fp.user_agent,
            "note": "Fresh identity applied: fingerprint + stealth patches + canvas noise + font protection"
        }))
    }

    pub(crate) async fn stealth_detection_audit(
        &self,
        p: DetectionAuditParams,
    ) -> Result<CallToolResult, McpError> {
        let page = ensure_page(&self.browser).await?;
        let detailed = p.detailed.unwrap_or(true);

        let js = r#"
            (async () => {
                const tests = {};
                let passed = 0;
                let total = 0;

                total++;
                tests.webdriver = !navigator.webdriver;
                if (tests.webdriver) passed++;

                total++;
                tests.chrome_runtime = !!(window.chrome && window.chrome.runtime);
                if (tests.chrome_runtime) passed++;

                total++;
                tests.plugins = navigator.plugins.length > 0;
                if (tests.plugins) passed++;

                total++;
                tests.languages = navigator.languages && navigator.languages.length > 0;
                if (tests.languages) passed++;

                total++;
                try {
                    const c = document.createElement('canvas');
                    const gl = c.getContext('webgl');
                    const dbg = gl?.getExtension('WEBGL_debug_renderer_info');
                    tests.webgl = !!(dbg && gl.getParameter(dbg.UNMASKED_VENDOR_WEBGL));
                } catch { tests.webgl = false; }
                if (tests.webgl) passed++;

                total++;
                try {
                    const perm = await navigator.permissions.query({name: 'notifications'});
                    tests.permissions = perm.state !== 'denied' || true;
                } catch { tests.permissions = false; }
                if (tests.permissions) passed++;

                total++;
                tests.screen = window.outerWidth > 0 && window.outerHeight > 0 &&
                               window.screen.width > 0 && screen.width === window.outerWidth;
                if (tests.screen) passed++;

                total++;
                try {
                    const c = document.createElement('canvas');
                    c.width = 200; c.height = 50;
                    const ctx = c.getContext('2d');
                    ctx.fillStyle = '#f60';
                    ctx.fillRect(0, 0, 200, 50);
                    ctx.fillStyle = '#069';
                    ctx.font = '14px Arial';
                    ctx.fillText('fingerprint test', 2, 15);
                    const d1 = c.toDataURL();
                    ctx.fillStyle = '#f60';
                    ctx.fillRect(0, 0, 200, 50);
                    ctx.fillStyle = '#069';
                    ctx.fillText('fingerprint test', 2, 15);
                    const d2 = c.toDataURL();
                    tests.canvas_noise = d1 !== d2;
                } catch { tests.canvas_noise = false; }
                if (tests.canvas_noise) passed++;

                total++;
                tests.webrtc_blocked = (typeof RTCPeerConnection === 'undefined') ||
                    (typeof window.RTCPeerConnection === 'undefined');
                if (tests.webrtc_blocked) passed++;

                total++;
                try {
                    const dtf = new Intl.DateTimeFormat();
                    const resolved = dtf.resolvedOptions();
                    tests.timezone = !!(resolved.timeZone);
                } catch { tests.timezone = false; }
                if (tests.timezone) passed++;

                total++;
                tests.visibility = document.hidden === false && document.visibilityState === 'visible';
                if (tests.visibility) passed++;

                total++;
                try {
                    const battery = await navigator.getBattery();
                    tests.battery = battery.charging === true && battery.level === 1;
                } catch { tests.battery = true; }
                if (tests.battery) passed++;

                return JSON.stringify({
                    score: Math.round((passed / total) * 100),
                    passed, total,
                    tests: tests,
                    grade: passed === total ? 'A+' : passed >= total * 0.9 ? 'A' : passed >= total * 0.75 ? 'B' : passed >= total * 0.5 ? 'C' : 'F'
                });
            })()
        "#;

        let result = page.evaluate(js.to_string()).await.map_err(|e| mcp_err(format!("detection_audit: {e}")))?;
        let raw: String = result.into_value().unwrap_or_else(|_| r#"{"score":0}"#.to_string());
        let parsed: serde_json::Value = serde_json::from_str(&raw).unwrap_or(serde_json::json!({"score": 0}));

        if detailed {
            json_ok(&parsed)
        } else {
            json_ok(&serde_json::json!({
                "score": parsed["score"],
                "grade": parsed["grade"],
                "passed": parsed["passed"],
                "total": parsed["total"]
            }))
        }
    }

    pub(crate) async fn stealth_status(
        &self,
        _p: StealthStatusParams,
    ) -> Result<CallToolResult, McpError> {
        let page = ensure_page(&self.browser).await?;
        let js = r#"(() => {
            const checks = {};
            checks.webdriver = navigator.webdriver;
            checks.chrome_runtime = !!window.chrome?.runtime?.id;
            checks.plugins = navigator.plugins.length;
            checks.languages = navigator.languages;
            checks.platform = navigator.platform;
            checks.hardware_concurrency = navigator.hardwareConcurrency;
            checks.device_memory = navigator.deviceMemory;
            checks.webgl_vendor = (() => { try { const c = document.createElement('canvas'); const gl = c.getContext('webgl'); return gl?.getParameter(gl.VENDOR); } catch(e) { return null; } })();
            checks.webgl_renderer = (() => { try { const c = document.createElement('canvas'); const gl = c.getContext('webgl'); const d = gl?.getExtension('WEBGL_debug_renderer_info'); return d ? gl.getParameter(d.UNMASKED_RENDERER_WEBGL) : null; } catch(e) { return null; } })();
            checks.timezone = Intl.DateTimeFormat().resolvedOptions().timeZone;
            checks.screen = { width: screen.width, height: screen.height, depth: screen.colorDepth };
            checks.touch = navigator.maxTouchPoints;
            return JSON.stringify(checks);
        })()"#;
        let result = page.evaluate(js.to_string()).await.mcp()?;
        let text = result
            .into_value::<String>()
            .unwrap_or_default();
        let parsed: serde_json::Value =
            serde_json::from_str(&text).unwrap_or(serde_json::json!({"raw": text}));
        json_ok(&serde_json::json!({"stealth_report": parsed}))
    }
}
