//! Handler implementations for the `agent` super-tool.

use futures::StreamExt;
use rmcp::{ErrorData as McpError, model::*};
use crate::cdp_tools::*;
use crate::helpers::{mcp_err, ensure_page, json_ok, json_escape, McpResult};
use crate::types::*;
use crate::OneCrawlMcp;
use base64::{Engine as _, engine::general_purpose::STANDARD as B64};

impl OneCrawlMcp {

    // ════════════════════════════════════════════════════════════════
    //  Agent tools — Enhanced Agentic API Layer
    // ════════════════════════════════════════════════════════════════

    pub(crate) async fn agent_execute_chain(
        &self,
        p: ExecuteChainParams,
    ) -> Result<CallToolResult, McpError> {
        let stop_on_error = p.stop_on_error.unwrap_or(true);
        let total = p.commands.len();
        let mut results: Vec<serde_json::Value> = Vec::with_capacity(total);
        let mut completed = 0usize;

        for cmd in &p.commands {
            let outcome = self.dispatch_chain_command(cmd).await;
            completed += 1;
            match outcome {
                Ok(data) => {
                    results.push(serde_json::json!({
                        "tool": cmd.tool,
                        "success": true,
                        "data": data
                    }));
                }
                Err(err_msg) => {
                    results.push(serde_json::json!({
                        "tool": cmd.tool,
                        "success": false,
                        "error": {
                            "message": err_msg,
                            "code": "CHAIN_STEP_FAILED"
                        }
                    }));
                    if stop_on_error {
                        break;
                    }
                }
            }
        }

        json_ok(&serde_json::json!({
            "results": results,
            "completed": completed,
            "total": total
        }))
    }


    pub(crate) async fn agent_element_screenshot(
        &self,
        p: ElementScreenshotParams,
    ) -> Result<CallToolResult, McpError> {
        if p.selector.is_empty() {
            return Err(mcp_err("selector must not be empty"));
        }
        let page = ensure_page(&self.browser).await?;
        let selector = onecrawl_cdp::accessibility::resolve_ref(&p.selector);

        // Get element bounds via JS
        let bounds_js = format!(
            r#"(() => {{
                const el = document.querySelector({sel});
                if (!el) return null;
                const r = el.getBoundingClientRect();
                return {{ x: r.x, y: r.y, width: r.width, height: r.height }};
            }})()"#,
            sel = serde_json::to_string(&selector).unwrap_or_else(|_| "null".into())
        );
        let bounds_val = onecrawl_cdp::page::evaluate_js(&page, &bounds_js)
            .await
            .mcp()?;

        if bounds_val.is_null() {
            return Err(crate::helpers::agent_err(
                crate::agent_error::element_not_found(&p.selector),
            ));
        }

        let bytes = onecrawl_cdp::screenshot::screenshot_element(&page, &selector)
            .await
            .mcp()?;
        let b64 = B64.encode(&bytes);

        json_ok(&serde_json::json!({
            "image": b64,
            "bounds": bounds_val
        }))
    }


    pub(crate) async fn agent_api_capture_start(
        &self,
        _p: ApiCaptureStartParams,
    ) -> Result<CallToolResult, McpError> {
        let page = ensure_page(&self.browser).await?;
        let js = r#"
        (() => {
            if (window.__onecrawl_api_log) return { already_active: true, entries: window.__onecrawl_api_log.length };
            window.__onecrawl_api_log = [];

            // Patch fetch
            const origFetch = window.fetch;
            window.fetch = async function(...args) {
                const url = typeof args[0] === 'string' ? args[0] : (args[0]?.url || '');
                const method = (args[1]?.method || 'GET').toUpperCase();
                const entry = { type: 'fetch', method, url, status: null, contentType: null, timestamp: Date.now() };
                try {
                    const resp = await origFetch.apply(this, args);
                    entry.status = resp.status;
                    entry.contentType = resp.headers.get('content-type');
                    window.__onecrawl_api_log.push(entry);
                    return resp;
                } catch(e) {
                    entry.error = e.message;
                    window.__onecrawl_api_log.push(entry);
                    throw e;
                }
            };

            // Patch XMLHttpRequest
            const origOpen = XMLHttpRequest.prototype.open;
            const origSend = XMLHttpRequest.prototype.send;
            XMLHttpRequest.prototype.open = function(method, url, ...rest) {
                this.__onecrawl_entry = { type: 'xhr', method: (method || 'GET').toUpperCase(), url: url || '', status: null, contentType: null, timestamp: Date.now() };
                return origOpen.call(this, method, url, ...rest);
            };
            XMLHttpRequest.prototype.send = function(...args) {
                const entry = this.__onecrawl_entry;
                if (entry) {
                    this.addEventListener('load', function() {
                        entry.status = this.status;
                        entry.contentType = this.getResponseHeader('content-type');
                        window.__onecrawl_api_log.push(entry);
                    });
                    this.addEventListener('error', function() {
                        entry.error = 'network error';
                        window.__onecrawl_api_log.push(entry);
                    });
                }
                return origSend.apply(this, args);
            };

            return { active: true, entries: 0 };
        })()
        "#;
        let result = onecrawl_cdp::page::evaluate_js(&page, js)
            .await
            .mcp()?;
        json_ok(&result)
    }


    pub(crate) async fn agent_api_capture_summary(
        &self,
        p: ApiCaptureSummaryParams,
    ) -> Result<CallToolResult, McpError> {
        let page = ensure_page(&self.browser).await?;
        let clear = p.clear.unwrap_or(false);
        let js = format!(
            r#"(() => {{
                const log = window.__onecrawl_api_log || [];
                const result = {{ total: log.length, requests: log.slice() }};
                if ({clear}) {{ window.__onecrawl_api_log = []; }}
                return result;
            }})()"#,
            clear = if clear { "true" } else { "false" }
        );
        let result = onecrawl_cdp::page::evaluate_js(&page, &js)
            .await
            .mcp()?;
        json_ok(&result)
    }


    pub(crate) async fn agent_iframe_list(
        &self,
        _p: IframeListParams,
    ) -> Result<CallToolResult, McpError> {
        let page = ensure_page(&self.browser).await?;
        let iframes = onecrawl_cdp::iframe::list_iframes(&page)
            .await
            .mcp()?;
        json_ok(&serde_json::json!({
            "total": iframes.len(),
            "iframes": iframes
        }))
    }


    pub(crate) async fn agent_iframe_snapshot(
        &self,
        p: IframeSnapshotParams,
    ) -> Result<CallToolResult, McpError> {
        let page = ensure_page(&self.browser).await?;
        let interactive_only = if p.interactive_only.unwrap_or(false) { "true" } else { "false" };
        let compact = if p.compact.unwrap_or(false) { "true" } else { "false" };

        // Inject a lightweight snapshot script into the iframe
        let snap_js = format!(
            r#"(() => {{
                const interactiveOnly = {interactive_only};
                const compactMode = {compact};
                const INTERACTIVE_TAGS = new Set(['A','BUTTON','INPUT','SELECT','TEXTAREA','DETAILS','SUMMARY']);
                const INTERACTIVE_ROLES = new Set(['button','link','textbox','checkbox','radio','combobox','listbox','menuitem','tab','switch','searchbox','slider','spinbutton']);
                let refCounter = 0;
                const refs = {{}};
                function walk(node, depth) {{
                    if (!node || node.nodeType !== 1) return '';
                    const tag = node.tagName.toLowerCase();
                    if (tag === 'script' || tag === 'style' || tag === 'noscript') return '';
                    const role = node.getAttribute('role') || '';
                    const isInteractive = INTERACTIVE_TAGS.has(node.tagName) || INTERACTIVE_ROLES.has(role);
                    if (interactiveOnly && !isInteractive && node.children.length === 0) return '';
                    const refId = 'f{idx}e' + (refCounter++);
                    node.setAttribute('data-onecrawl-ref', refId);
                    const label = node.getAttribute('aria-label') || node.getAttribute('alt') || node.getAttribute('placeholder') || '';
                    const text = node.childNodes.length === 1 && node.childNodes[0].nodeType === 3 ? node.childNodes[0].textContent.trim().substring(0, 80) : '';
                    let line = '  '.repeat(depth) + tag;
                    if (role) line += '[role=' + role + ']';
                    line += ' @' + refId;
                    if (label) line += ' "' + label + '"';
                    else if (text) line += ' "' + text + '"';
                    let children = '';
                    for (const c of node.children) {{ children += walk(c, depth + 1); }}
                    if (compactMode && !isInteractive && !children && !text && !label) return '';
                    refs[refId] = tag + (node.id ? '#' + node.id : '') + (node.className && typeof node.className === 'string' ? '.' + node.className.trim().split(/\\s+/).join('.') : '');
                    return line + '\\n' + children;
                }}
                const snapshot = walk(document.body || document.documentElement, 0);
                return {{ snapshot, refs, total: refCounter, iframe_index: {idx} }};
            }})()"#,
            interactive_only = interactive_only,
            compact = compact,
            idx = p.index
        );

        let result = onecrawl_cdp::iframe::eval_in_iframe(&page, p.index, &snap_js)
            .await
            .mcp()?;

        if let Some(err) = result.get("error") {
            return Err(mcp_err(format!("iframe snapshot failed: {}", err)));
        }

        json_ok(&result)
    }


    pub(crate) async fn agent_connect_remote(
        &self,
        p: RemoteCdpParams,
    ) -> Result<CallToolResult, McpError> {
        let mut state = self.browser.lock().await;

        // Validate the WebSocket URL format
        if !p.ws_url.starts_with("ws://") && !p.ws_url.starts_with("wss://") {
            return Err(mcp_err("ws_url must start with ws:// or wss://"));
        }

        // Connect to remote browser via chromiumoxide (with timeout)
        let (browser, mut handler) =
            tokio::time::timeout(
                std::time::Duration::from_secs(15),
                chromiumoxide::Browser::connect(&p.ws_url),
            )
            .await
            .map_err(|_| mcp_err("remote CDP connect timed out after 15s"))?
            .map_err(|e| mcp_err(format!("remote CDP connect failed: {e}")))?;

        // Spawn the handler loop
        tokio::spawn(async move {
            while let Some(_event) = handler.next().await {}
        });

        let page = browser
            .new_page("about:blank")
            .await
            .map_err(|e| mcp_err(format!("remote new_page failed: {e}")))?;

        // Store in shared state (replace any existing session)
        state.session = None; // drop local session
        state.page = Some(page);

        let _ = &p.headers; // reserved for future handshake header support

        json_ok(&serde_json::json!({
            "connected": true,
            "ws_url": p.ws_url,
            "info": "Remote browser connected. Subsequent tools will use this session."
        }))
    }

    // ════════════════════════════════════════════════════════════════
    //  Safety Policy tools
    // ════════════════════════════════════════════════════════════════


    // ════════════════════════════════════════════════════════════════
    //  Safety Policy tools
    // ════════════════════════════════════════════════════════════════

    pub(crate) async fn agent_safety_policy_set(
        &self,
        p: SafetyPolicySetParams,
    ) -> Result<CallToolResult, McpError> {
        let policy = if let Some(ref path) = p.policy_file {
            onecrawl_cdp::SafetyState::load_from_file(std::path::Path::new(path))
                .mcp()?
        } else {
            onecrawl_cdp::SafetyPolicy {
                allowed_domains: p.allowed_domains.unwrap_or_default(),
                blocked_domains: p.blocked_domains.unwrap_or_default(),
                blocked_url_patterns: p.blocked_url_patterns.unwrap_or_default(),
                max_actions: p.max_actions.unwrap_or(0),
                confirm_form_submit: p.confirm_form_submit.unwrap_or(false),
                confirm_file_upload: p.confirm_file_upload.unwrap_or(false),
                blocked_commands: p.blocked_commands.unwrap_or_default(),
                allowed_commands: p.allowed_commands.unwrap_or_default(),
                rate_limit_per_minute: p.rate_limit_per_minute.unwrap_or(0),
            }
        };

        let mut state = self.browser.lock().await;
        match state.safety.as_mut() {
            Some(existing) => existing.set_policy(policy.clone()),
            None => state.safety = Some(onecrawl_cdp::SafetyState::new(policy.clone())),
        }

        json_ok(&serde_json::json!({
            "status": "policy_set",
            "policy": policy
        }))
    }


    pub(crate) async fn agent_safety_status(
        &self,
        _p: SafetyStatusParams,
    ) -> Result<CallToolResult, McpError> {
        let state = self.browser.lock().await;
        match &state.safety {
            Some(safety) => json_ok(&safety.stats()),
            None => json_ok(&serde_json::json!({
                "status": "no_policy",
                "info": "No safety policy is active. Use agent.safety_policy_set to configure one."
            })),
        }
    }


    pub(crate) fn agent_skills_list(
        &self,
        _p: SkillsListParams,
    ) -> Result<CallToolResult, McpError> {
        let builtins = onecrawl_cdp::skills::SkillRegistry::builtins();
        let skills: Vec<serde_json::Value> = builtins
            .iter()
            .map(|s| {
                serde_json::json!({
                    "name": s.name,
                    "version": s.version,
                    "description": s.description,
                    "tools": s.tools.iter().map(|t| serde_json::json!({
                        "name": t.name,
                        "description": t.description,
                    })).collect::<Vec<_>>(),
                    "requires": s.requires,
                    "author": s.author,
                    "source": "built-in",
                })
            })
            .collect();
        json_ok(&skills)
    }


    pub(crate) async fn agent_screencast_start(
        &self,
        p: ScreencastStartParams,
    ) -> Result<CallToolResult, McpError> {
        let page = ensure_page(&self.browser).await?;
        let opts = onecrawl_cdp::screencast::ScreencastOptions {
            format: p.format.unwrap_or_else(|| "jpeg".into()),
            quality: p.quality.map(|q| q.min(100)).or(Some(60)),
            max_width: p.max_width.or(Some(1280)),
            max_height: p.max_height.or(Some(720)),
            every_nth_frame: p.every_nth_frame.or(Some(1)),
        };
        onecrawl_cdp::screencast::start_screencast(&page, &opts)
            .await
            .mcp()?;
        json_ok(&serde_json::json!({
            "status": "started",
            "format": opts.format,
            "quality": opts.quality,
            "max_width": opts.max_width,
            "max_height": opts.max_height,
            "every_nth_frame": opts.every_nth_frame
        }))
    }


    pub(crate) async fn agent_screencast_stop(
        &self,
        _p: ScreencastStopParams,
    ) -> Result<CallToolResult, McpError> {
        let page = ensure_page(&self.browser).await?;
        onecrawl_cdp::screencast::stop_screencast(&page)
            .await
            .mcp()?;
        json_ok(&serde_json::json!({ "status": "stopped" }))
    }


    pub(crate) async fn agent_screencast_frame(
        &self,
        p: ScreencastFrameParams,
    ) -> Result<CallToolResult, McpError> {
        let page = ensure_page(&self.browser).await?;
        let opts = onecrawl_cdp::screencast::ScreencastOptions {
            format: p.format.unwrap_or_else(|| "jpeg".into()),
            quality: p.quality.or(Some(80)),
            ..Default::default()
        };
        let bytes = onecrawl_cdp::screencast::capture_frame(&page, &opts)
            .await
            .mcp()?;
        let b64 = B64.encode(&bytes);
        json_ok(&serde_json::json!({
            "image": b64,
            "format": opts.format,
            "size": bytes.len()
        }))
    }


    pub(crate) async fn agent_recording_start(
        &self,
        p: RecordingStartParams,
    ) -> Result<CallToolResult, McpError> {
        let page = ensure_page(&self.browser).await?;
        let output = p.output.unwrap_or_else(|| "recording.webm".into());
        let fps = p.fps.unwrap_or(5);

        {
            let mut state = self.browser.lock().await;
            if state.recording.as_ref().is_some_and(|r| r.is_recording()) {
                return Err(mcp_err("recording already in progress"));
            }
            let mut rec = onecrawl_cdp::RecordingState::new(
                std::path::PathBuf::from(&output),
                fps,
            );
            rec.start();
            state.recording = Some(rec);
        }

        let opts = onecrawl_cdp::screencast::ScreencastOptions {
            format: "jpeg".into(),
            quality: Some(60),
            max_width: Some(1280),
            max_height: Some(720),
            every_nth_frame: Some(1),
        };
        onecrawl_cdp::screencast::start_screencast(&page, &opts)
            .await
            .mcp()?;

        json_ok(&serde_json::json!({
            "status": "recording",
            "output": output,
            "fps": fps
        }))
    }


    pub(crate) async fn agent_recording_stop(
        &self,
        _p: RecordingStopParams,
    ) -> Result<CallToolResult, McpError> {
        let page = ensure_page(&self.browser).await?;
        let _ = onecrawl_cdp::screencast::stop_screencast(&page).await;

        let mut state = self.browser.lock().await;
        let rec = state.recording.as_mut()
            .ok_or_else(|| mcp_err("no recording in progress"))?;

        // If no frames were captured via events, grab one snapshot
        if rec.is_recording() && rec.frame_count() == 0 {
            drop(state);
            let opts = onecrawl_cdp::screencast::ScreencastOptions::default();
            if let Ok(bytes) = onecrawl_cdp::screencast::capture_frame(&page, &opts).await {
                let mut state = self.browser.lock().await;
                if let Some(rec) = state.recording.as_mut() {
                    rec.add_frame(bytes);
                }
            }
            let mut state = self.browser.lock().await;
            let rec = state.recording.as_mut()
                .ok_or_else(|| mcp_err("no recording in progress"))?;
            rec.stop();
            let frame_count = rec.frame_count();
            let result = rec.save_frames().mcp()?;
            state.recording = None;
            return json_ok(&serde_json::json!({
                "status": "saved",
                "frames": frame_count,
                "path": result.display().to_string()
            }));
        }

        rec.stop();
        let frame_count = rec.frame_count();
        let result = rec.save_frames().mcp()?;
        state.recording = None;
        json_ok(&serde_json::json!({
            "status": "saved",
            "frames": frame_count,
            "path": result.display().to_string()
        }))
    }


    pub(crate) async fn agent_recording_status(
        &self,
        _p: RecordingStatusParams,
    ) -> Result<CallToolResult, McpError> {
        let state = self.browser.lock().await;
        match state.recording.as_ref() {
            Some(rec) => {
                let status = if rec.is_recording() { "recording" } else { "stopped" };
                json_ok(&serde_json::json!({
                    "status": status,
                    "frames": rec.frame_count(),
                    "fps": rec.fps(),
                    "output": rec.output_path().display().to_string()
                }))
            }
            None => json_ok(&serde_json::json!({
                "status": "idle",
                "frames": 0
            })),
        }
    }

    // ════════════════════════════════════════════════════════════════
    //  iOS / Mobile Safari tools
    // ════════════════════════════════════════════════════════════════


    // ════════════════════════════════════════════════════════════════
    //  iOS / Mobile Safari tools
    // ════════════════════════════════════════════════════════════════

    pub(crate) async fn agent_ios_devices(
        &self,
        _p: IosDevicesParams,
    ) -> Result<CallToolResult, McpError> {
        let devices = onecrawl_cdp::ios::IosClient::list_devices()
            .map_err(|e| mcp_err(format!("iOS list devices failed: {e}")))?;
        json_ok(&serde_json::json!({
            "devices": devices,
            "count": devices.len()
        }))
    }


    pub(crate) async fn agent_ios_connect(
        &self,
        p: IosConnectParams,
    ) -> Result<CallToolResult, McpError> {
        let config = onecrawl_cdp::ios::IosSessionConfig {
            wda_url: p.wda_url.unwrap_or_else(|| "http://localhost:8100".to_string()),
            device_udid: p.udid,
            bundle_id: p.bundle_id.unwrap_or_else(|| "com.apple.mobilesafari".to_string()),
        };
        let mut client = onecrawl_cdp::ios::IosClient::new(config);
        let session_id = client.create_session().await
            .map_err(|e| mcp_err(format!("iOS connect failed: {e}")))?;

        let mut state = self.browser.lock().await;
        state.ios_client = Some(client);

        json_ok(&serde_json::json!({
            "connected": true,
            "session_id": session_id
        }))
    }


    pub(crate) async fn agent_ios_navigate(
        &self,
        p: IosNavigateParams,
    ) -> Result<CallToolResult, McpError> {
        let state = self.browser.lock().await;
        let client = state.ios_client.as_ref()
            .ok_or_else(|| mcp_err("no active iOS session — call agent.ios_connect first"))?;
        client.navigate(&p.url).await
            .map_err(|e| mcp_err(format!("iOS navigate failed: {e}")))?;
        json_ok(&serde_json::json!({
            "navigated": true,
            "url": p.url
        }))
    }


    pub(crate) async fn agent_ios_tap(
        &self,
        p: IosTapParams,
    ) -> Result<CallToolResult, McpError> {
        let state = self.browser.lock().await;
        let client = state.ios_client.as_ref()
            .ok_or_else(|| mcp_err("no active iOS session — call agent.ios_connect first"))?;
        client.tap(p.x, p.y).await
            .map_err(|e| mcp_err(format!("iOS tap failed: {e}")))?;
        json_ok(&serde_json::json!({
            "tapped": true,
            "x": p.x,
            "y": p.y
        }))
    }


    pub(crate) async fn agent_ios_screenshot(
        &self,
        _p: IosScreenshotParams,
    ) -> Result<CallToolResult, McpError> {
        let state = self.browser.lock().await;
        let client = state.ios_client.as_ref()
            .ok_or_else(|| mcp_err("no active iOS session — call agent.ios_connect first"))?;
        let bytes = client.screenshot().await
            .map_err(|e| mcp_err(format!("iOS screenshot failed: {e}")))?;
        let b64 = B64.encode(&bytes);
        json_ok(&serde_json::json!({
            "format": "png",
            "size": bytes.len(),
            "data": b64
        }))
    }

    // ──────────────── Computer Use Protocol ─────────────────

    // ════════════════════════════════════════════════════════════════
    //  Task Decomposition Engine
    // ════════════════════════════════════════════════════════════════

    pub(crate) async fn task_decompose(
        &self,
        p: TaskDecomposeParams,
    ) -> Result<CallToolResult, McpError> {
        let goal = &p.goal;
        let context = p.context.as_deref().unwrap_or("");
        let max_depth = p.max_depth.unwrap_or(3);

        // Analyze goal and decompose into atomic subtasks based on common patterns
        let goal_lower = goal.to_lowercase();
        let mut subtasks: Vec<serde_json::Value> = Vec::new();
        let mut id = 1u32;

        let patterns: &[(&str, &[&str])] = &[
            ("navigate", &["navigate to target page"]),
            ("login", &["navigate to login page", "find username field", "fill username", "find password field", "fill password", "click login button", "verify login success"]),
            ("search", &["find search input", "type search query", "submit search", "wait for results", "extract results"]),
            ("fill", &["find form fields", "fill form data", "validate form", "submit form"]),
            ("click", &["find target element", "click element", "verify action result"]),
            ("extract", &["navigate to page", "wait for content", "extract data", "format output"]),
            ("scrape", &["navigate to page", "wait for content", "extract elements", "paginate if needed", "collect results"]),
            ("test", &["navigate to page", "verify page loaded", "check elements", "validate behavior", "report results"]),
            ("submit", &["find form", "fill required fields", "validate inputs", "click submit", "verify submission"]),
            ("download", &["navigate to resource", "find download link", "initiate download", "wait for completion"]),
        ];

        let mut matched = false;
        for (keyword, steps) in patterns {
            if goal_lower.contains(keyword) {
                for step in *steps {
                    subtasks.push(serde_json::json!({
                        "id": format!("task_{id}"),
                        "description": step,
                        "complexity": if step.contains("navigate") { "low" }
                            else if step.contains("extract") || step.contains("verify") { "medium" }
                            else { "low" },
                        "depth": 1
                    }));
                    id += 1;
                }
                matched = true;
                break;
            }
        }

        if !matched {
            subtasks.push(serde_json::json!({ "id": "task_1", "description": format!("analyze: {goal}"), "complexity": "medium", "depth": 1 }));
            subtasks.push(serde_json::json!({ "id": "task_2", "description": "execute primary action", "complexity": "medium", "depth": 1 }));
            subtasks.push(serde_json::json!({ "id": "task_3", "description": "verify result", "complexity": "low", "depth": 1 }));
        }

        json_ok(&serde_json::json!({
            "goal": goal,
            "context": context,
            "max_depth": max_depth,
            "subtasks": subtasks,
            "total": subtasks.len()
        }))
    }

    pub(crate) async fn task_plan(
        &self,
        p: TaskPlanParams,
    ) -> Result<CallToolResult, McpError> {
        let strategy = p.strategy.as_deref().unwrap_or("sequential");
        let plan_id = format!("plan_{}", std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap_or_default().as_millis());

        let steps: Vec<serde_json::Value> = p.tasks.iter().enumerate().map(|(i, task)| {
            let deps = match strategy {
                "sequential" if i > 0 => vec![format!("step_{}", i)],
                "dependency" if i > 0 => vec![format!("step_{}", i)],
                _ => vec![],
            };
            serde_json::json!({
                "id": format!("step_{}", i + 1),
                "task": task,
                "dependencies": deps,
                "status": "pending",
                "order": i + 1
            })
        }).collect();

        let plan = serde_json::json!({
            "plan_id": plan_id,
            "strategy": strategy,
            "steps": steps,
            "total_steps": steps.len(),
            "status": "created"
        });

        let mut state = self.browser.lock().await;
        state.task_plans.push(plan.clone());

        json_ok(&plan)
    }

    pub(crate) async fn task_status(
        &self,
        _v: serde_json::Value,
    ) -> Result<CallToolResult, McpError> {
        let state = self.browser.lock().await;
        let plans = &state.task_plans;
        json_ok(&serde_json::json!({
            "plans": plans,
            "total": plans.len()
        }))
    }

    // ════════════════════════════════════════════════════════════════
    //  Vision/LLM Observation Layer
    // ════════════════════════════════════════════════════════════════

    pub(crate) async fn vision_describe(
        &self,
        p: VisionDescribeParams,
    ) -> Result<CallToolResult, McpError> {
        let page = ensure_page(&self.browser).await?;
        let format = p.format.as_deref().unwrap_or("structured");
        let selector_js = json_escape(p.selector.as_deref().unwrap_or(""));
        let js = format!(r#"(() => {{
            const sel = {selector_js};
            const root = sel ? document.querySelector(sel) : document.body;
            if (!root) return JSON.stringify({{ error: 'element not found' }});
            const title = document.title;
            const url = location.href;
            const elements = [];
            const interactive = root.querySelectorAll('button, a, input, select, textarea, [role="button"], [role="link"], [tabindex]');
            interactive.forEach((el, i) => {{
                if (i >= 50) return;
                const rect = el.getBoundingClientRect();
                elements.push({{
                    role: el.getAttribute('role') || el.tagName.toLowerCase(),
                    name: el.getAttribute('aria-label') || el.textContent?.trim().slice(0, 60) || '',
                    bounds: {{ x: Math.round(rect.x), y: Math.round(rect.y), w: Math.round(rect.width), h: Math.round(rect.height) }},
                    text: (el.textContent || '').trim().slice(0, 80)
                }});
            }});
            const headings = [];
            root.querySelectorAll('h1,h2,h3,h4,h5,h6').forEach(h => {{
                headings.push({{ level: parseInt(h.tagName[1]), text: h.textContent.trim().slice(0, 100) }});
            }});
            return JSON.stringify({{
                page_title: title,
                url: url,
                visible_elements: elements,
                headings: headings,
                interactive_elements_count: interactive.length,
                layout_summary: root.children.length + ' top-level children'
            }});
        }})()"#);
        let result = page.evaluate(js).await.mcp()?;
        let raw = result.into_value::<String>().unwrap_or_else(|_| "{}".into());
        let mut val: serde_json::Value = serde_json::from_str(&raw).unwrap_or_default();
        if let Some(obj) = val.as_object_mut() {
            obj.insert("format".into(), serde_json::json!(format));
        }
        json_ok(&val)
    }

    pub(crate) async fn vision_locate(
        &self,
        p: VisionLocateParams,
    ) -> Result<CallToolResult, McpError> {
        let page = ensure_page(&self.browser).await?;
        let desc_js = json_escape(&p.description);
        let strategy = p.strategy.as_deref().unwrap_or("semantic");
        let js = format!(r#"(() => {{
            const desc = {desc_js}.toLowerCase();
            const matches = [];
            const candidates = document.querySelectorAll('button, a, input, select, textarea, [role], [aria-label], label, h1, h2, h3, h4, h5, h6');
            candidates.forEach(el => {{
                let confidence = 0;
                const role = el.getAttribute('role') || el.tagName.toLowerCase();
                const name = (el.getAttribute('aria-label') || el.textContent || '').trim().toLowerCase();
                const placeholder = (el.getAttribute('placeholder') || '').toLowerCase();
                const title = (el.getAttribute('title') || '').toLowerCase();
                // Text matching
                if (name.includes(desc)) confidence += 0.5;
                if (placeholder.includes(desc)) confidence += 0.4;
                if (title.includes(desc)) confidence += 0.3;
                // Role matching
                const words = desc.split(' ');
                if (words.some(w => role.includes(w))) confidence += 0.3;
                if (words.some(w => name.includes(w))) confidence += 0.2;
                if (confidence > 0.2) {{
                    const tag = el.tagName.toLowerCase();
                    const id = el.id ? '#' + el.id : '';
                    const cls = el.className ? '.' + String(el.className).split(' ').filter(Boolean).slice(0, 2).join('.') : '';
                    matches.push({{
                        selector: tag + id + cls,
                        role,
                        name: (el.getAttribute('aria-label') || el.textContent || '').trim().slice(0, 60),
                        confidence: Math.min(confidence, 1.0)
                    }});
                }}
            }});
            matches.sort((a, b) => b.confidence - a.confidence);
            return JSON.stringify({{
                found: matches.length > 0,
                matches: matches.slice(0, 5)
            }});
        }})()"#);
        let result = page.evaluate(js).await.mcp()?;
        let raw = result.into_value::<String>().unwrap_or_else(|_| "{}".into());
        let mut val: serde_json::Value = serde_json::from_str(&raw).unwrap_or_default();
        if let Some(obj) = val.as_object_mut() {
            obj.insert("strategy".into(), serde_json::json!(strategy));
        }
        json_ok(&val)
    }

    pub(crate) async fn vision_compare(
        &self,
        p: VisionCompareParams,
    ) -> Result<CallToolResult, McpError> {
        let page = ensure_page(&self.browser).await?;
        let threshold = p.threshold.unwrap_or(0.9);

        // Get current page state
        let current_js = r#"(() => {
            const elements = [];
            document.querySelectorAll('*').forEach(el => {
                if (el.children.length === 0 || el.tagName === 'A' || el.tagName === 'BUTTON') {
                    const text = (el.textContent || '').trim();
                    if (text.length > 0 && text.length < 200) {
                        elements.push({
                            tag: el.tagName.toLowerCase(),
                            role: el.getAttribute('role') || '',
                            text: text.slice(0, 100),
                            id: el.id || ''
                        });
                    }
                }
            });
            return JSON.stringify({ title: document.title, url: location.href, elements: elements.slice(0, 100) });
        })()"#;
        let result = page.evaluate(current_js).await.mcp()?;
        let current_raw = result.into_value::<String>().unwrap_or_else(|_| "{}".into());
        let current_state: serde_json::Value = serde_json::from_str(&current_raw).unwrap_or_default();

        let current_text = p.current.as_deref()
            .map(|s| serde_json::from_str(s).unwrap_or(serde_json::json!({"raw": s})))
            .unwrap_or(current_state);
        let baseline: serde_json::Value = serde_json::from_str(&p.baseline)
            .unwrap_or(serde_json::json!({"raw": p.baseline}));

        // Compare structures
        let baseline_str = serde_json::to_string(&baseline).unwrap_or_default();
        let current_str = serde_json::to_string(&current_text).unwrap_or_default();

        // Simple similarity: count matching character sequences
        let max_len = baseline_str.len().max(current_str.len()).max(1);
        let common = baseline_str.chars().zip(current_str.chars())
            .filter(|(a, b)| a == b)
            .count();
        let similarity = common as f64 / max_len as f64;

        let mut changes = Vec::new();
        if let (Some(b_els), Some(c_els)) = (
            baseline.get("elements").and_then(|e| e.as_array()),
            current_text.get("elements").and_then(|e| e.as_array()),
        ) {
            if b_els.len() != c_els.len() {
                changes.push(serde_json::json!({
                    "type": "count_change",
                    "element": "total_elements",
                    "before": b_els.len(),
                    "after": c_els.len()
                }));
            }
        }

        json_ok(&serde_json::json!({
            "visual_similarity": (similarity * 100.0).round() / 100.0,
            "threshold": threshold,
            "passed": similarity >= threshold,
            "structural_changes": changes,
            "summary": if similarity >= threshold { "Pages are similar" } else { "Significant differences detected" }
        }))
    }
}
