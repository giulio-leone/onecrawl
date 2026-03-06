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
            const el = document.querySelector("{}");
            if (!el) return {{ error: "Element not found" }};
            el.focus();
            el.value = '';
            el.dispatchEvent(new Event('focus', {{ bubbles: true }}));
            
            const text = "{}";
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
            return {{ typed: text.length, typos_simulated: typoCount, selector: "{}" }};
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
}
