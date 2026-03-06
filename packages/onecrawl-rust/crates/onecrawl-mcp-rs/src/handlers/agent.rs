//! Handler implementations for the `agent` super-tool.

use futures::StreamExt;
use rmcp::{ErrorData as McpError, model::*};
use crate::cdp_tools::*;
use crate::helpers::{mcp_err, ensure_page, json_ok, McpResult};
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

}
