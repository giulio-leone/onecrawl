//! Handler implementations for the `data` super-tool.

use rmcp::{ErrorData as McpError, model::*};
use crate::cdp_tools::*;
use crate::helpers::{mcp_err, ensure_page, json_ok, parse_json_str, parse_opt_json_str, McpResult};
use crate::types::*;
use crate::OneCrawlMcp;
use std::collections::HashMap;

impl OneCrawlMcp {

    // ════════════════════════════════════════════════════════════════
    //  CDP tools — Data Processing
    // ════════════════════════════════════════════════════════════════

    pub(crate) fn data_pipeline(
        &self,
        p: PipelineExecuteParams,
    ) -> Result<CallToolResult, McpError> {
        let steps: Vec<onecrawl_cdp::PipelineStep> = parse_json_str(&p.steps, "steps")?;
        let pipeline = onecrawl_cdp::Pipeline {
            name: p.name,
            steps,
        };
        let items: Vec<HashMap<String, String>> = parse_json_str(&p.input, "input")?;
        let result = onecrawl_cdp::data_pipeline::execute_pipeline(&pipeline, items);
        json_ok(&result)
    }


    pub(crate) async fn data_http_get(
        &self,
        p: HttpGetParams,
    ) -> Result<CallToolResult, McpError> {
        let page = ensure_page(&self.browser).await?;
        let headers: Option<HashMap<String, String>> = parse_opt_json_str(p.headers.as_deref(), "headers")?;
        let resp = onecrawl_cdp::http_client::get(&page, &p.url, headers)
            .await
            .mcp()?;
        json_ok(&resp)
    }


    pub(crate) async fn data_http_post(
        &self,
        p: HttpPostParams,
    ) -> Result<CallToolResult, McpError> {
        let page = ensure_page(&self.browser).await?;
        let headers: Option<HashMap<String, String>> = parse_opt_json_str(p.headers.as_deref(), "headers")?;
        let resp =
            onecrawl_cdp::http_client::post(&page, &p.url, &p.body, "application/json", headers)
                .await
                .mcp()?;
        json_ok(&resp)
    }


    pub(crate) async fn data_links(
        &self,
        p: ExtractLinksParams,
    ) -> Result<CallToolResult, McpError> {
        let page = ensure_page(&self.browser).await?;
        let edges = onecrawl_cdp::link_graph::extract_links(&page, &p.base_url)
            .await
            .mcp()?;
        json_ok(&edges)
    }


    pub(crate) fn data_graph(
        &self,
        p: AnalyzeGraphParams,
    ) -> Result<CallToolResult, McpError> {
        let edges: Vec<onecrawl_cdp::LinkEdge> = parse_json_str(&p.edges, "edges")?;
        let graph = onecrawl_cdp::link_graph::build_graph(&edges);
        let stats = onecrawl_cdp::link_graph::analyze_graph(&graph);
        json_ok(&stats)
    }

    // ════════════════════════════════════════════════════════════════
    //  CDP tools — Automation
    // ════════════════════════════════════════════════════════════════


    // ════════════════════════════════════════════════════════════════
    //  CDP tools — Automation
    // ════════════════════════════════════════════════════════════════

    pub(crate) async fn automation_rate_limit(
        &self,
        p: RateLimitCheckParams,
    ) -> Result<CallToolResult, McpError> {
        let mut state = self.browser.lock().await;
        if state.rate_limiter.is_none() {
            let config = onecrawl_cdp::RateLimitConfig {
                max_requests_per_second: p.max_per_second.unwrap_or(2.0),
                max_requests_per_minute: p.max_per_minute.unwrap_or(60.0),
                max_requests_per_hour: 3600.0,
                burst_size: 5,
                cooldown_ms: 500,
            };
            state.rate_limiter = Some(onecrawl_cdp::RateLimitState::new(config));
        }
        let limiter = state.rate_limiter.as_ref().unwrap();
        let can = onecrawl_cdp::rate_limiter::can_proceed(limiter);
        let stats = onecrawl_cdp::rate_limiter::get_stats(limiter);
        json_ok(&RateLimitResult {
            can_proceed: can,
            stats,
        })
    }


    pub(crate) async fn automation_retry(
        &self,
        p: RetryEnqueueParams,
    ) -> Result<CallToolResult, McpError> {
        let mut state = self.browser.lock().await;
        if state.retry_queue.is_none() {
            state.retry_queue = Some(onecrawl_cdp::RetryQueue::new(onecrawl_cdp::RetryConfig {
                max_retries: 3,
                initial_delay_ms: 1000,
                max_delay_ms: 30_000,
                backoff_factor: 2.0,
                jitter: true,
            }));
        }
        let queue = state.retry_queue.as_mut().unwrap();
        let id = onecrawl_cdp::retry_queue::enqueue(
            queue,
            &p.url,
            &p.operation,
            p.payload.as_deref(),
        );
        let stats = onecrawl_cdp::retry_queue::get_stats(queue);
        json_ok(&RetryResult {
            id,
            queue_stats: stats,
        })
    }

    //  Passkey / WebAuthn tools


    // ════════════════════════════════════════════════════════════════
    //  Network Intelligence tools
    // ════════════════════════════════════════════════════════════════

    pub(crate) async fn net_capture(
        &self,
        p: NetIntelCaptureParams,
    ) -> Result<CallToolResult, McpError> {
        let page = ensure_page(&self.browser).await?;
        let duration = p.duration_seconds.unwrap_or(10);
        let api_only = p.api_only.unwrap_or(true);

        // Inject network interceptor
        let js = r#"
        (() => {
            if (!window.__onecrawl_net_capture) {
                window.__onecrawl_net_capture = [];
                const origFetch = window.fetch;
                window.fetch = async function(...args) {
                    const start = Date.now();
                    const req = new Request(...args);
                    try {
                        const resp = await origFetch.apply(this, args);
                        const clone = resp.clone();
                        let body = null;
                        try { body = await clone.json(); } catch(_) {
                            try { body = await clone.text(); } catch(_) {}
                        }
                        let reqBody = null;
                        try { if (req.body) { reqBody = await new Request(...args).json(); } } catch(_) {}
                        window.__onecrawl_net_capture.push({
                            method: req.method,
                            url: req.url,
                            status: resp.status,
                            contentType: resp.headers.get('content-type'),
                            requestHeaders: Object.fromEntries(req.headers.entries()),
                            responseHeaders: Object.fromEntries(resp.headers.entries()),
                            requestBody: reqBody,
                            responseBody: body,
                            timing: Date.now() - start,
                        });
                        return resp;
                    } catch(e) {
                        window.__onecrawl_net_capture.push({
                            method: req.method,
                            url: req.url,
                            status: 0,
                            error: e.message,
                            timing: Date.now() - start,
                        });
                        throw e;
                    }
                };

                const origXHR = XMLHttpRequest.prototype.open;
                XMLHttpRequest.prototype.open = function(method, url, ...rest) {
                    this.__onecrawl_method = method;
                    this.__onecrawl_url = url;
                    this.__onecrawl_start = Date.now();
                    return origXHR.call(this, method, url, ...rest);
                };
                const origSend = XMLHttpRequest.prototype.send;
                XMLHttpRequest.prototype.send = function(body) {
                    this.addEventListener('load', function() {
                        let respBody = null;
                        try { respBody = JSON.parse(this.responseText); } catch(_) { respBody = this.responseText; }
                        window.__onecrawl_net_capture.push({
                            method: this.__onecrawl_method,
                            url: this.__onecrawl_url,
                            status: this.status,
                            contentType: this.getResponseHeader('content-type'),
                            responseBody: respBody,
                            timing: Date.now() - this.__onecrawl_start,
                        });
                    });
                    return origSend.call(this, body);
                };
            }
            return 'capture_started';
        })()
        "#;

        page.evaluate(js).await.mcp()?;

        // Wait for capture duration
        tokio::time::sleep(tokio::time::Duration::from_secs(duration)).await;

        // Collect results
        let collect_js = r#"
        (() => {
            const raw = window.__onecrawl_net_capture || [];
            window.__onecrawl_net_capture = [];
            return raw;
        })()
        "#;

        let result = page.evaluate(collect_js).await.mcp()?;
        let raw: Vec<serde_json::Value> = result.into_value().unwrap_or_default();

        let endpoints: Vec<onecrawl_cdp::network_intel::ApiEndpoint> = raw.iter().filter_map(|r| {
            let url = r.get("url")?.as_str()?;
            let method = r.get("method")?.as_str().unwrap_or("GET");
            let status = r.get("status")?.as_u64().unwrap_or(0) as u16;
            let content_type = r.get("contentType").and_then(|v| v.as_str()).map(String::from);
            let category = onecrawl_cdp::network_intel::classify_request(url, content_type.as_deref(), method);

            if api_only && category == onecrawl_cdp::network_intel::ApiCategory::Static {
                return None;
            }

            let (parsed_path, parsed_base) = url.split_once("://")
                .and_then(|(scheme, rest)| rest.split_once('/').map(|(host, path)| {
                    let p = format!("/{}", path).split('?').next().unwrap_or("/").to_string();
                    let b = format!("{}://{}", scheme, host);
                    (p, b)
                }))
                .unwrap_or(("/".into(), url.to_string()));

            Some(onecrawl_cdp::network_intel::ApiEndpoint {
                method: method.to_string(),
                url: url.to_string(),
                path: parsed_path,
                base_url: parsed_base,
                query_params: std::collections::HashMap::new(),
                request_headers: r.get("requestHeaders").and_then(|v| serde_json::from_value(v.clone()).ok()).unwrap_or_default(),
                response_headers: r.get("responseHeaders").and_then(|v| serde_json::from_value(v.clone()).ok()).unwrap_or_default(),
                request_body: r.get("requestBody").cloned().filter(|v| !v.is_null()),
                response_body: r.get("responseBody").cloned().filter(|v| !v.is_null()),
                status_code: status,
                content_type,
                timing_ms: r.get("timing").and_then(|v| v.as_f64()),
                category,
            })
        }).collect();

        json_ok(&serde_json::json!({
            "endpoints": endpoints,
            "count": endpoints.len(),
            "duration_seconds": duration,
        }))
    }


    pub(crate) async fn net_analyze(
        &self,
        p: NetIntelAnalyzeParams,
    ) -> Result<CallToolResult, McpError> {
        let endpoints: Vec<onecrawl_cdp::network_intel::ApiEndpoint> = serde_json::from_str(&p.capture)
            .map_err(|e| mcp_err(format!("invalid capture data: {e}")))?;

        if endpoints.is_empty() {
            return json_ok(&serde_json::json!({ "error": "no endpoints to analyze" }));
        }

        let base_url = endpoints.first().map(|e| e.base_url.clone()).unwrap_or_default();
        let total_requests = endpoints.len();

        // Group by method+path template
        let mut endpoint_map: std::collections::HashMap<String, Vec<&onecrawl_cdp::network_intel::ApiEndpoint>> = std::collections::HashMap::new();
        for ep in &endpoints {
            let (template, _) = onecrawl_cdp::network_intel::extract_path_params(&ep.path);
            let key = format!("{} {}", ep.method, template);
            endpoint_map.entry(key).or_default().push(ep);
        }

        let schemas: Vec<onecrawl_cdp::network_intel::EndpointSchema> = endpoint_map.iter().map(|(key, eps)| {
            let parts: Vec<&str> = key.splitn(2, ' ').collect();
            let method = parts.first().unwrap_or(&"GET");
            let path = parts.get(1).unwrap_or(&"/");
            let (template, params) = onecrawl_cdp::network_intel::extract_path_params(path);

            let status_codes: Vec<u16> = eps.iter().map(|e| e.status_code).collect::<std::collections::HashSet<_>>().into_iter().collect();
            let content_types: Vec<String> = eps.iter().filter_map(|e| e.content_type.clone()).collect::<std::collections::HashSet<_>>().into_iter().collect();
            let avg_latency = eps.iter().filter_map(|e| e.timing_ms).sum::<f64>() / eps.len().max(1) as f64;

            let response_schema = eps.iter().find_map(|e| e.response_body.as_ref())
                .map(|b| onecrawl_cdp::network_intel::infer_json_schema(b));
            let request_schema = eps.iter().find_map(|e| e.request_body.as_ref())
                .map(|b| onecrawl_cdp::network_intel::infer_json_schema(b));

            onecrawl_cdp::network_intel::EndpointSchema {
                method: method.to_string(),
                path: template,
                path_params: params,
                query_params: vec![],
                request_body_schema: request_schema,
                response_body_schema: response_schema,
                status_codes,
                content_types,
                call_count: eps.len(),
                avg_latency_ms: avg_latency,
            }
        }).collect();

        let auth_pattern = endpoints.iter()
            .find_map(|e| {
                let auth = onecrawl_cdp::network_intel::detect_auth_pattern(&e.request_headers);
                match auth {
                    onecrawl_cdp::network_intel::AuthPattern::None => None,
                    other => Some(other),
                }
            });

        let schema = onecrawl_cdp::network_intel::ApiSchema {
            base_url,
            endpoints: schemas,
            auth_pattern,
            total_requests,
            unique_endpoints: endpoint_map.len(),
        };

        json_ok(&serde_json::to_value(&schema).unwrap())
    }


    pub(crate) async fn net_sdk(
        &self,
        p: NetIntelSdkParams,
    ) -> Result<CallToolResult, McpError> {
        let schema: onecrawl_cdp::network_intel::ApiSchema = serde_json::from_str(&p.schema)
            .map_err(|e| mcp_err(format!("invalid schema: {e}")))?;

        let sdk = match p.language.as_deref().unwrap_or("typescript") {
            "python" | "py" => onecrawl_cdp::network_intel::generate_python_sdk(&schema),
            _ => onecrawl_cdp::network_intel::generate_typescript_sdk(&schema),
        };

        json_ok(&serde_json::json!({
            "language": sdk.language,
            "code": sdk.code,
            "endpoints_covered": sdk.endpoints_covered,
        }))
    }


    pub(crate) async fn net_mock(
        &self,
        p: NetIntelMockParams,
    ) -> Result<CallToolResult, McpError> {
        let endpoints: Vec<onecrawl_cdp::network_intel::ApiEndpoint> = serde_json::from_str(&p.endpoints)
            .map_err(|e| mcp_err(format!("invalid endpoints: {e}")))?;

        let config = onecrawl_cdp::network_intel::generate_mock_config(&endpoints, p.port.unwrap_or(3001));
        json_ok(&serde_json::to_value(&config).unwrap())
    }


    pub(crate) async fn net_replay(
        &self,
        p: NetIntelReplayParams,
    ) -> Result<CallToolResult, McpError> {
        let endpoints: Vec<onecrawl_cdp::network_intel::ApiEndpoint> = serde_json::from_str(&p.endpoints)
            .map_err(|e| mcp_err(format!("invalid endpoints: {e}")))?;

        let name = p.name.as_deref().unwrap_or("replay_sequence");
        let sequence = onecrawl_cdp::network_intel::generate_replay_sequence(name, &endpoints);
        json_ok(&serde_json::to_value(&sequence).unwrap())
    }

    // ════════════════════════════════════════════════════════════════
    //  Visual Regression Testing tools
    // ════════════════════════════════════════════════════════════════

}
