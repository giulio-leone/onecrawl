//! Vision stream MCP handler — streaming AI vision for continuous page understanding.

use crate::cdp_tools::*;
use crate::helpers::{ensure_page, json_ok, mcp_err};
use rmcp::{model::*, ErrorData as McpError};
use std::sync::Arc;

use crate::server::OneCrawlMcp;

impl OneCrawlMcp {
    // ════════════════════════════════════════════════════════════════
    //  Streaming AI Vision Handlers
    // ════════════════════════════════════════════════════════════════

    pub(crate) async fn vision_stream_start(
        &self,
        p: VisionStreamStartParams,
    ) -> Result<CallToolResult, McpError> {
        let page = ensure_page(&self.browser).await?;

        let screenshot_format = match p.format.as_deref() {
            Some("png") => onecrawl_cdp::ScreenshotFormat::Png,
            _ => onecrawl_cdp::ScreenshotFormat::Jpeg {
                quality: p.quality.unwrap_or(70),
            },
        };

        let resolution = match (p.width, p.height) {
            (Some(w), Some(h)) => Some((w, h)),
            _ => None,
        };

        let config = onecrawl_cdp::VisionConfig {
            model: p.model.unwrap_or_else(|| "gpt-4o".to_string()),
            api_key: None,
            api_url: None,
            fps: p.fps.unwrap_or(0.5),
            max_fps: p.max_fps.unwrap_or(2.0),
            describe: p.describe.unwrap_or(false),
            react_to: p.react_to.unwrap_or_default(),
            prompt: p.prompt,
            max_tokens: p.max_tokens,
            max_cost_cents: p.max_cost_cents,
            screenshot_format,
            resolution,
            output_log: p.output,
        };

        let stream = Arc::new(onecrawl_cdp::VisionStream::new(config));
        stream.start(&page).await.map_err(|e| mcp_err(e))?;

        {
            let mut state = self.browser.lock().await;
            state.vision_stream = Some(Arc::clone(&stream));
        }

        let status = stream.status().await;
        json_ok(&serde_json::json!({
            "started": true,
            "fps": status.fps,
            "running": status.running
        }))
    }

    pub(crate) async fn vision_stream_stop(
        &self,
        _p: VisionStreamStopParams,
    ) -> Result<CallToolResult, McpError> {
        let stream = {
            let state = self.browser.lock().await;
            state
                .vision_stream
                .clone()
                .ok_or_else(|| mcp_err("no vision stream is running"))?
        };

        let status = stream.stop().await.map_err(|e| mcp_err(e))?;
        json_ok(&status)
    }

    pub(crate) async fn vision_stream_status(
        &self,
        _p: VisionStreamStatusParams,
    ) -> Result<CallToolResult, McpError> {
        let stream = {
            let state = self.browser.lock().await;
            state
                .vision_stream
                .clone()
                .ok_or_else(|| mcp_err("no vision stream has been started"))?
        };

        let status = stream.status().await;
        json_ok(&status)
    }

    pub(crate) async fn vision_stream_describe(
        &self,
        p: VisionStreamDescribeParams,
    ) -> Result<CallToolResult, McpError> {
        let page = ensure_page(&self.browser).await?;

        let stream = {
            let state = self.browser.lock().await;
            match state.vision_stream.clone() {
                Some(s) => s,
                None => {
                    // Create a one-shot stream with defaults
                    let s = Arc::new(onecrawl_cdp::VisionStream::new(
                        onecrawl_cdp::VisionConfig::default(),
                    ));
                    s
                }
            }
        };

        let obs = stream
            .describe_once(&page)
            .await
            .map_err(|e| mcp_err(e))?;

        // Also prepare the request payload so the caller can send it
        let request = stream
            .prepare_vision_request_async(
                "",  // The actual frame is inside the observation description
                p.prompt.as_deref(),
            )
            .await;

        json_ok(&serde_json::json!({
            "observation": obs,
            "request_payload": request
        }))
    }

    pub(crate) async fn vision_stream_observations(
        &self,
        p: VisionStreamObservationsParams,
    ) -> Result<CallToolResult, McpError> {
        let stream = {
            let state = self.browser.lock().await;
            state
                .vision_stream
                .clone()
                .ok_or_else(|| mcp_err("no vision stream has been started"))?
        };

        let limit = p.limit.unwrap_or(10);
        let obs = stream.observations(limit).await;
        json_ok(&serde_json::json!({
            "count": obs.len(),
            "observations": obs
        }))
    }

    pub(crate) async fn vision_stream_set_fps(
        &self,
        p: VisionStreamSetFpsParams,
    ) -> Result<CallToolResult, McpError> {
        let stream = {
            let state = self.browser.lock().await;
            state
                .vision_stream
                .clone()
                .ok_or_else(|| mcp_err("no vision stream has been started"))?
        };

        stream.set_fps(p.fps).await.map_err(|e| mcp_err(e))?;
        json_ok(&serde_json::json!({
            "fps": p.fps,
            "updated": true
        }))
    }

    pub(crate) async fn vision_stream_react(
        &self,
        p: VisionStreamReactParams,
    ) -> Result<CallToolResult, McpError> {
        let stream = {
            let state = self.browser.lock().await;
            state
                .vision_stream
                .clone()
                .ok_or_else(|| mcp_err("no vision stream has been started"))?
        };

        let obs = stream
            .parse_vision_response(&p.response_text, p.frame_index.unwrap_or(0))
            .map_err(|e| mcp_err(e))?;

        stream.record_observation(obs.clone()).await;

        json_ok(&serde_json::json!({
            "observation": obs,
            "recorded": true
        }))
    }
}
