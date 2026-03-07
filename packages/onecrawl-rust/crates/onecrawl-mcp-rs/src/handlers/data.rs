//! Handler implementations for the `data` super-tool.

use rmcp::{ErrorData as McpError, model::*};
use crate::cdp_tools::*;
use crate::helpers::{mcp_err, ensure_page, json_ok, json_escape, parse_json_str, parse_opt_json_str, McpResult};
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
        let limiter = state.rate_limiter.as_ref().ok_or_else(|| mcp_err("rate limiter not initialized"))?;
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
        let queue = state.retry_queue.as_mut().ok_or_else(|| mcp_err("retry queue not initialized"))?;
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

        json_ok(&serde_json::to_value(&schema).mcp()?)
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
        json_ok(&serde_json::to_value(&config).mcp()?)
    }


    pub(crate) async fn net_replay(
        &self,
        p: NetIntelReplayParams,
    ) -> Result<CallToolResult, McpError> {
        let endpoints: Vec<onecrawl_cdp::network_intel::ApiEndpoint> = serde_json::from_str(&p.endpoints)
            .map_err(|e| mcp_err(format!("invalid endpoints: {e}")))?;

        let name = p.name.as_deref().unwrap_or("replay_sequence");
        let sequence = onecrawl_cdp::network_intel::generate_replay_sequence(name, &endpoints);
        json_ok(&serde_json::to_value(&sequence).mcp()?)
    }

    // ════════════════════════════════════════════════════════════════
    //  Visual Regression Testing tools
    // ════════════════════════════════════════════════════════════════

    // ════════════════════════════════════════════════════════════════
    //  Structured Data Pipeline
    // ════════════════════════════════════════════════════════════════

    pub(crate) async fn extract_schema(&self, p: ExtractSchemaParams) -> Result<CallToolResult, McpError> {
        let page = ensure_page(&self.browser).await?;
        let schema_type = p.schema_type.as_deref().unwrap_or("all");

        let js = format!(r#"(() => {{
            const result = {{}};

            if ("{schema_type}" === "all" || "{schema_type}" === "json_ld") {{
                const ld = [...document.querySelectorAll('script[type="application/ld+json"]')];
                result.json_ld = ld.map(s => {{ try {{ return JSON.parse(s.textContent); }} catch(_) {{ return null; }} }}).filter(Boolean);
            }}

            if ("{schema_type}" === "all" || "{schema_type}" === "open_graph") {{
                const og = [...document.querySelectorAll('meta[property^="og:"]')];
                result.open_graph = {{}};
                og.forEach(m => {{ result.open_graph[m.getAttribute('property')] = m.getAttribute('content'); }});
            }}

            if ("{schema_type}" === "all" || "{schema_type}" === "twitter_card") {{
                const tw = [...document.querySelectorAll('meta[name^="twitter:"]')];
                result.twitter_card = {{}};
                tw.forEach(m => {{ result.twitter_card[m.getAttribute('name')] = m.getAttribute('content'); }});
            }}

            if ("{schema_type}" === "all" || "{schema_type}" === "microdata") {{
                const items = [...document.querySelectorAll('[itemscope]')];
                result.microdata = items.map(el => ({{
                    type: el.getAttribute('itemtype'),
                    properties: [...el.querySelectorAll('[itemprop]')].map(p => ({{
                        name: p.getAttribute('itemprop'),
                        value: p.textContent.trim().substring(0, 200)
                    }}))
                }}));
            }}

            return result;
        }})()"#);

        let result = page.evaluate(js).await.mcp()?;
        let val: serde_json::Value = result.into_value().unwrap_or(serde_json::json!({}));
        json_ok(&serde_json::json!({ "action": "extract_schema", "schema_type": schema_type, "data": val }))
    }

    pub(crate) async fn extract_tables(&self, p: ExtractTablesParams) -> Result<CallToolResult, McpError> {
        let page = ensure_page(&self.browser).await?;
        let selector = json_escape(p.selector.as_deref().unwrap_or("table"));
        let use_headers = p.headers.unwrap_or(true);

        let js = format!(r#"(() => {{
            const tables = [...document.querySelectorAll({selector})];
            return tables.map((table, idx) => {{
                const rows = [...table.querySelectorAll('tr')];
                if (rows.length === 0) return {{ index: idx, rows: [] }};

                let headers = null;
                let dataRows = rows;
                if ({use_headers} && rows.length > 0) {{
                    const firstRow = rows[0];
                    const cells = [...firstRow.querySelectorAll('th, td')];
                    headers = cells.map(c => c.textContent.trim());
                    dataRows = rows.slice(1);
                }}

                const data = dataRows.map(row => {{
                    const cells = [...row.querySelectorAll('td, th')];
                    if (headers) {{
                        const obj = {{}};
                        cells.forEach((c, i) => {{ obj[headers[i] || 'col_' + i] = c.textContent.trim(); }});
                        return obj;
                    }}
                    return cells.map(c => c.textContent.trim());
                }});

                return {{ index: idx, headers, row_count: data.length, data }};
            }});
        }})()"#);

        let result = page.evaluate(js).await.mcp()?;
        let val: serde_json::Value = result.into_value().unwrap_or(serde_json::json!([]));
        let format = p.format.as_deref().unwrap_or("json");
        json_ok(&serde_json::json!({ "action": "extract_tables", "format": format, "tables": val }))
    }

    pub(crate) async fn extract_entities(&self, p: ExtractEntitiesParams) -> Result<CallToolResult, McpError> {
        let page = ensure_page(&self.browser).await?;
        let scope = p.selector.as_deref().unwrap_or("body");
        let scope_js = json_escape(scope);
        let types = p.types.as_ref().map(|t| t.join(",")).unwrap_or_else(|| "emails,phones,urls,dates,prices".to_string());

        let js = format!(r#"(() => {{
            const el = document.querySelector({scope_js}) || document.body;
            const text = el.innerText || el.textContent || '';
            const result = {{}};
            const types = '{types}'.split(',');

            if (types.includes('emails')) {{
                result.emails = [...new Set(text.match(/[a-zA-Z0-9._%+\-]+@[a-zA-Z0-9.\-]+\.[a-zA-Z]{{2,}}/g) || [])];
            }}
            if (types.includes('phones')) {{
                result.phones = [...new Set(text.match(/[\+]?[(]?[0-9]{{1,4}}[)]?[-\s\.]?[0-9]{{1,4}}[-\s\.]?[0-9]{{1,9}}/g) || [])];
            }}
            if (types.includes('urls')) {{
                result.urls = [...new Set(text.match(/https?:\/\/[^\s<>"']+/g) || [])];
            }}
            if (types.includes('dates')) {{
                result.dates = [...new Set(text.match(/\d{{1,2}}[\/\-\.]\d{{1,2}}[\/\-\.]\d{{2,4}}|\d{{4}}-\d{{2}}-\d{{2}}|(?:Jan|Feb|Mar|Apr|May|Jun|Jul|Aug|Sep|Oct|Nov|Dec)[a-z]* \d{{1,2}},? \d{{4}}/gi) || [])];
            }}
            if (types.includes('prices')) {{
                result.prices = [...new Set(text.match(/[\$\€\£\¥]\s?[\d,]+\.?\d*/g) || [])];
            }}

            return result;
        }})()"#);

        let result = page.evaluate(js).await.mcp()?;
        let val: serde_json::Value = result.into_value().unwrap_or(serde_json::json!({}));
        json_ok(&serde_json::json!({ "action": "extract_entities", "entities": val }))
    }

    pub(crate) async fn classify_content(&self, p: ClassifyContentParams) -> Result<CallToolResult, McpError> {
        let page = ensure_page(&self.browser).await?;
        let strategy = p.strategy.as_deref().unwrap_or("type");
        let scope = p.selector.as_deref().unwrap_or("body");
        let scope_js = json_escape(scope);

        let js = format!(r#"(() => {{
            const el = document.querySelector({scope_js}) || document.body;
            const text = (el.innerText || '').substring(0, 5000);
            const title = document.title || '';
            const url = location.href;
            const meta = document.querySelector('meta[name="description"]');
            const desc = meta ? meta.content : '';

            const h1Count = el.querySelectorAll('h1').length;
            const formCount = el.querySelectorAll('form').length;
            const imgCount = el.querySelectorAll('img').length;
            const linkCount = el.querySelectorAll('a').length;
            const wordCount = text.split(/\s+/).length;
            const lang = document.documentElement.lang || 'unknown';

            let pageType = 'unknown';
            if (formCount > 0 && text.match(/login|sign\s?in|password/i)) pageType = 'login';
            else if (formCount > 0 && text.match(/sign\s?up|register|create account/i)) pageType = 'registration';
            else if (formCount > 0 && text.match(/search/i)) pageType = 'search';
            else if (formCount > 0) pageType = 'form';
            else if (text.match(/cart|checkout|payment/i)) pageType = 'commerce';
            else if (wordCount > 500 && h1Count >= 1) pageType = 'article';
            else if (linkCount > 20 && wordCount < 300) pageType = 'listing';
            else if (imgCount > 5) pageType = 'gallery';
            else if (wordCount < 100) pageType = 'landing';
            else pageType = 'content';

            return {{
                page_type: pageType,
                language: lang,
                title, description: desc, url,
                stats: {{ word_count: wordCount, h1_count: h1Count, form_count: formCount, image_count: imgCount, link_count: linkCount }}
            }};
        }})()"#);

        let result = page.evaluate(js).await.mcp()?;
        let val: serde_json::Value = result.into_value().unwrap_or(serde_json::json!({}));
        json_ok(&serde_json::json!({ "action": "classify_content", "strategy": strategy, "classification": val }))
    }

    pub(crate) fn transform_json(&self, p: TransformJsonParams) -> Result<CallToolResult, McpError> {
        let transform = &p.transform;
        let data = &p.data;
        let output_format = p.output_format.as_deref().unwrap_or("json");

        // Simple transform operations: select, filter, map, flatten, sort, unique, count
        let result = match transform.as_str() {
            "flatten" => {
                if let Some(arr) = data.as_array() {
                    let flat: Vec<&serde_json::Value> = arr.iter()
                        .flat_map(|v| v.as_array().map(|a| a.iter().collect::<Vec<_>>()).unwrap_or_else(|| vec![v]))
                        .collect();
                    serde_json::json!(flat)
                } else { data.clone() }
            }
            "count" => serde_json::json!({ "count": data.as_array().map(|a| a.len()).unwrap_or(1) }),
            "keys" => {
                if let Some(obj) = data.as_object() {
                    serde_json::json!(obj.keys().collect::<Vec<_>>())
                } else { serde_json::json!([]) }
            }
            "values" => {
                if let Some(obj) = data.as_object() {
                    serde_json::json!(obj.values().collect::<Vec<_>>())
                } else { serde_json::json!([]) }
            }
            "unique" => {
                if let Some(arr) = data.as_array() {
                    let mut seen = std::collections::HashSet::new();
                    let unique: Vec<&serde_json::Value> = arr.iter().filter(|v| seen.insert(v.to_string())).collect();
                    serde_json::json!(unique)
                } else { data.clone() }
            }
            "reverse" => {
                if let Some(arr) = data.as_array() {
                    let mut rev = arr.clone();
                    rev.reverse();
                    serde_json::json!(rev)
                } else { data.clone() }
            }
            _ => {
                // Treat as JMESPath-like field access: "field.subfield"
                let parts: Vec<&str> = transform.split('.').collect();
                let mut current = data.clone();
                for part in parts {
                    current = current.get(part).cloned().unwrap_or(serde_json::Value::Null);
                }
                current
            }
        };

        json_ok(&serde_json::json!({ "action": "transform_json", "format": output_format, "result": result }))
    }

    pub(crate) fn export_csv(&self, p: ExportCsvParams) -> Result<CallToolResult, McpError> {
        let delimiter = p.delimiter.as_deref().unwrap_or(",");
        let del_char = delimiter.chars().next().unwrap_or(',');

        let items = p.data.as_array().ok_or_else(|| mcp_err("data must be a JSON array"))?;
        if items.is_empty() {
            return json_ok(&serde_json::json!({ "action": "export_csv", "csv": "", "row_count": 0 }));
        }

        let columns: Vec<String> = if let Some(cols) = &p.columns {
            cols.clone()
        } else {
            // Auto-detect from first item
            items[0].as_object().map(|o| o.keys().cloned().collect()).unwrap_or_default()
        };

        let mut csv = String::new();
        csv.push_str(&columns.join(&del_char.to_string()));
        csv.push('\n');

        for item in items {
            let row: Vec<String> = columns.iter().map(|col| {
                item.get(col).map(|v| match v {
                    serde_json::Value::String(s) => {
                        if s.contains(del_char) || s.contains('"') || s.contains('\n') {
                            format!("\"{}\"", s.replace('"', "\"\""))
                        } else { s.clone() }
                    }
                    other => other.to_string().trim_matches('"').to_string(),
                }).unwrap_or_default()
            }).collect();
            csv.push_str(&row.join(&del_char.to_string()));
            csv.push('\n');
        }

        json_ok(&serde_json::json!({ "action": "export_csv", "csv": csv, "row_count": items.len(), "column_count": columns.len() }))
    }

    pub(crate) async fn extract_metadata(&self, p: ExtractMetadataParams) -> Result<CallToolResult, McpError> {
        let page = ensure_page(&self.browser).await?;
        let inc_og = p.include_og.unwrap_or(true);
        let inc_tw = p.include_twitter.unwrap_or(true);
        let inc_all = p.include_all.unwrap_or(false);

        let js = format!(r#"(() => {{
            const result = {{
                title: document.title,
                canonical: (document.querySelector('link[rel="canonical"]') || {{}}).href || null,
                description: (document.querySelector('meta[name="description"]') || {{}}).content || null,
                author: (document.querySelector('meta[name="author"]') || {{}}).content || null,
                robots: (document.querySelector('meta[name="robots"]') || {{}}).content || null,
            }};

            if ({inc_og}) {{
                result.open_graph = {{}};
                document.querySelectorAll('meta[property^="og:"]').forEach(m => {{
                    result.open_graph[m.getAttribute('property')] = m.content;
                }});
            }}

            if ({inc_tw}) {{
                result.twitter_card = {{}};
                document.querySelectorAll('meta[name^="twitter:"]').forEach(m => {{
                    result.twitter_card[m.getAttribute('name')] = m.content;
                }});
            }}

            if ({inc_all}) {{
                result.all_meta = [];
                document.querySelectorAll('meta').forEach(m => {{
                    result.all_meta.push({{
                        name: m.getAttribute('name'),
                        property: m.getAttribute('property'),
                        content: m.content
                    }});
                }});
            }}

            return result;
        }})()"#);

        let result = page.evaluate(js).await.mcp()?;
        let val: serde_json::Value = result.into_value().unwrap_or(serde_json::json!({}));
        json_ok(&serde_json::json!({ "action": "extract_metadata", "metadata": val }))
    }

    pub(crate) async fn extract_feeds(&self, p: ExtractFeedsParams) -> Result<CallToolResult, McpError> {
        let page = ensure_page(&self.browser).await?;
        let feed_type = p.feed_type.as_deref().unwrap_or("all");

        let js = format!(r#"(() => {{
            const feeds = [];
            const feedType = '{feed_type}';

            if (feedType === 'all' || feedType === 'rss' || feedType === 'atom') {{
                document.querySelectorAll('link[type="application/rss+xml"], link[type="application/atom+xml"]').forEach(link => {{
                    feeds.push({{
                        type: link.type.includes('atom') ? 'atom' : 'rss',
                        title: link.title || null,
                        url: link.href
                    }});
                }});
            }}

            if (feedType === 'all' || feedType === 'json_feed') {{
                document.querySelectorAll('link[type="application/feed+json"], link[type="application/json"]').forEach(link => {{
                    if (link.href.match(/feed|json/i)) {{
                        feeds.push({{ type: 'json_feed', title: link.title || null, url: link.href }});
                    }}
                }});
            }}

            // Also check common feed URL patterns in links
            document.querySelectorAll('a[href*="feed"], a[href*="rss"], a[href*="atom"]').forEach(a => {{
                feeds.push({{ type: 'discovered', title: a.textContent.trim().substring(0, 100), url: a.href }});
            }});

            return feeds;
        }})()"#);

        let result = page.evaluate(js).await.mcp()?;
        let val: serde_json::Value = result.into_value().unwrap_or(serde_json::json!([]));
        json_ok(&serde_json::json!({ "action": "extract_feeds", "feed_type": feed_type, "feeds": val }))
    }

}

// ── WebSocket & Real-Time Protocol ──────────────────────────────

impl OneCrawlMcp {
    pub(crate) async fn ws_connect(&self, p: WsConnectParams) -> Result<CallToolResult, McpError> {
        let page = ensure_page(&self.browser).await?;
        let protocols = p.protocols.as_ref().map(|ps| ps.join(",")).unwrap_or_default();
        let js = format!(r#"(async () => {{
            const protocols = "{}".split(",").filter(Boolean);
            const ws = protocols.length
                ? new WebSocket("{}", protocols)
                : new WebSocket("{}");
            window._onecrawl_ws = window._onecrawl_ws || {{}};
            window._onecrawl_ws_msgs = window._onecrawl_ws_msgs || [];
            const id = "ws_" + Date.now();
            ws.onmessage = (e) => window._onecrawl_ws_msgs.push({{ id, url: "{}", data: typeof e.data === 'string' ? e.data.substring(0, 1000) : '[binary]', ts: Date.now() }});
            ws.onerror = (e) => window._onecrawl_ws_msgs.push({{ id, url: "{}", error: 'connection_error', ts: Date.now() }});
            window._onecrawl_ws[id] = ws;
            await new Promise(r => {{ ws.onopen = r; setTimeout(r, 5000); }});
            return {{ id, url: "{}", state: ws.readyState, protocol: ws.protocol }};
        }})()"#, json_escape(&protocols), json_escape(&p.url), json_escape(&p.url), json_escape(&p.url), json_escape(&p.url), json_escape(&p.url));
        let result = page.evaluate(js).await.mcp()?;
        let val: serde_json::Value = result.into_value().unwrap_or(serde_json::json!(null));
        json_ok(&serde_json::json!({ "action": "ws_connect", "connection": val }))
    }

    pub(crate) async fn ws_intercept(&self, p: WsInterceptParams) -> Result<CallToolResult, McpError> {
        let page = ensure_page(&self.browser).await?;
        let pattern = json_escape(p.url_pattern.as_deref().unwrap_or("*"));
        let capture_only = p.capture_only.unwrap_or(true);
        let js = format!(r#"(() => {{
            window._onecrawl_ws_msgs = window._onecrawl_ws_msgs || [];
            const origWS = window._origWebSocket || window.WebSocket;
            window._origWebSocket = origWS;
            const pattern = "{}";
            const captureOnly = {};
            window.WebSocket = function(url, protocols) {{
                const ws = new origWS(url, protocols);
                const matchesPattern = pattern === "*" || url.includes(pattern);
                if (matchesPattern) {{
                    const origOnMessage = null;
                    ws.addEventListener('message', (e) => {{
                        window._onecrawl_ws_msgs.push({{
                            direction: 'incoming', url, data: typeof e.data === 'string' ? e.data.substring(0, 1000) : '[binary]', ts: Date.now()
                        }});
                    }});
                    const origSend = ws.send.bind(ws);
                    ws.send = function(data) {{
                        window._onecrawl_ws_msgs.push({{
                            direction: 'outgoing', url, data: typeof data === 'string' ? data.substring(0, 1000) : '[binary]', ts: Date.now()
                        }});
                        return origSend(data);
                    }};
                }}
                return ws;
            }};
            window.WebSocket.prototype = origWS.prototype;
            return {{ intercepting: true, pattern, capture_only: captureOnly }};
        }})()"#, pattern, capture_only);
        let result = page.evaluate(js).await.mcp()?;
        let val: serde_json::Value = result.into_value().unwrap_or(serde_json::json!(null));
        json_ok(&serde_json::json!({ "action": "ws_intercept", "result": val }))
    }

    pub(crate) async fn ws_send(&self, p: WsSendParams) -> Result<CallToolResult, McpError> {
        let page = ensure_page(&self.browser).await?;
        let js = format!(r#"(() => {{
            const ws = (window._onecrawl_ws || {{}})["{target}"];
            if (!ws) return {{ error: "WebSocket connection not found", target: "{target}" }};
            if (ws.readyState !== 1) return {{ error: "WebSocket not open", state: ws.readyState }};
            ws.send("{}");
            return {{ sent: true, target: "{target}", message_length: {len} }};
        }})()"#, json_escape(&p.message), target = json_escape(&p.target), len = p.message.len());
        let result = page.evaluate(js).await.mcp()?;
        let val: serde_json::Value = result.into_value().unwrap_or(serde_json::json!(null));
        json_ok(&serde_json::json!({ "action": "ws_send", "result": val }))
    }

    pub(crate) async fn ws_messages(&self, p: WsMessagesParams) -> Result<CallToolResult, McpError> {
        let page = ensure_page(&self.browser).await?;
        let limit = p.limit.unwrap_or(100);
        let filter = json_escape(p.url_filter.as_deref().unwrap_or(""));
        let js = format!(r#"(() => {{
            const msgs = window._onecrawl_ws_msgs || [];
            const filter = "{}";
            const filtered = filter ? msgs.filter(m => (m.url || '').includes(filter)) : msgs;
            return {{ total: msgs.length, returned: Math.min(filtered.length, {}), messages: filtered.slice(-{}) }};
        }})()"#, filter, limit, limit);
        let result = page.evaluate(js).await.mcp()?;
        let val: serde_json::Value = result.into_value().unwrap_or(serde_json::json!(null));
        json_ok(&serde_json::json!({ "action": "ws_messages", "result": val }))
    }

    pub(crate) async fn ws_close(&self, p: WsCloseParams) -> Result<CallToolResult, McpError> {
        let page = ensure_page(&self.browser).await?;
        let target = json_escape(p.target.as_deref().unwrap_or(""));
        let js = format!(r#"(() => {{
            const wss = window._onecrawl_ws || {{}};
            const target = "{}";
            let closed = 0;
            if (target) {{
                const ws = wss[target];
                if (ws && ws.readyState <= 1) {{ ws.close(); closed++; delete wss[target]; }}
            }} else {{
                for (const [id, ws] of Object.entries(wss)) {{
                    if (ws.readyState <= 1) {{ ws.close(); closed++; }}
                    delete wss[id];
                }}
            }}
            return {{ closed, target: target || "all" }};
        }})()"#, target);
        let result = page.evaluate(js).await.mcp()?;
        let val: serde_json::Value = result.into_value().unwrap_or(serde_json::json!(null));
        json_ok(&serde_json::json!({ "action": "ws_close", "result": val }))
    }

    pub(crate) async fn sse_listen(&self, p: SseListenParams) -> Result<CallToolResult, McpError> {
        let page = ensure_page(&self.browser).await?;
        let duration = p.duration_ms.unwrap_or(5000);
        let js = format!(r#"(async () => {{
            const url = "{}";
            const messages = [];
            const es = new EventSource(url);
            await new Promise((resolve) => {{
                es.onmessage = (e) => messages.push({{ type: 'message', data: e.data.substring(0, 500), lastEventId: e.lastEventId, ts: Date.now() }});
                es.onerror = () => messages.push({{ type: 'error', ts: Date.now() }});
                es.onopen = () => messages.push({{ type: 'open', ts: Date.now() }});
                setTimeout(() => {{ es.close(); resolve(); }}, {});
            }});
            window._onecrawl_sse_msgs = (window._onecrawl_sse_msgs || []).concat(messages);
            return {{ url, duration_ms: {}, messages_received: messages.length, messages }};
        }})()"#, json_escape(&p.url), duration, duration);
        let result = page.evaluate(js).await.mcp()?;
        let val: serde_json::Value = result.into_value().unwrap_or(serde_json::json!(null));
        json_ok(&serde_json::json!({ "action": "sse_listen", "result": val }))
    }

    pub(crate) async fn sse_messages(&self, p: SseMessagesParams) -> Result<CallToolResult, McpError> {
        let page = ensure_page(&self.browser).await?;
        let limit = p.limit.unwrap_or(100);
        let filter = json_escape(p.url_filter.as_deref().unwrap_or(""));
        let js = format!(r#"(() => {{
            const msgs = window._onecrawl_sse_msgs || [];
            const filter = "{}";
            const filtered = filter ? msgs.filter(m => m.type === 'message') : msgs;
            return {{ total: msgs.length, returned: Math.min(filtered.length, {}), messages: filtered.slice(-{}) }};
        }})()"#, filter, limit, limit);
        let result = page.evaluate(js).await.mcp()?;
        let val: serde_json::Value = result.into_value().unwrap_or(serde_json::json!(null));
        json_ok(&serde_json::json!({ "action": "sse_messages", "result": val }))
    }

    pub(crate) async fn graphql_subscribe(&self, p: GraphqlSubscribeParams) -> Result<CallToolResult, McpError> {
        let page = ensure_page(&self.browser).await?;
        let duration = p.duration_ms.unwrap_or(5000);
        let variables = p.variables.as_ref().map(|v| serde_json::to_string(v).unwrap_or_default()).unwrap_or_else(|| "{}".to_string());
        let js = format!(r#"(async () => {{
            const url = "{}";
            const query = "{}";
            const variables = {};
            const messages = [];
            
            // Try WebSocket-based GraphQL subscription (graphql-ws protocol)
            const wsUrl = url.replace(/^http/, 'ws');
            const ws = new WebSocket(wsUrl, 'graphql-transport-ws');
            
            await new Promise((resolve) => {{
                ws.onopen = () => {{
                    ws.send(JSON.stringify({{ type: 'connection_init' }}));
                    ws.send(JSON.stringify({{ id: '1', type: 'subscribe', payload: {{ query, variables }} }}));
                }};
                ws.onmessage = (e) => {{
                    try {{
                        const msg = JSON.parse(e.data);
                        messages.push({{ type: msg.type, data: msg.payload, ts: Date.now() }});
                    }} catch(_) {{
                        messages.push({{ type: 'raw', data: e.data.substring(0, 500), ts: Date.now() }});
                    }}
                }};
                ws.onerror = () => messages.push({{ type: 'error', ts: Date.now() }});
                setTimeout(() => {{ ws.close(); resolve(); }}, {});
            }});
            return {{ url, query: query.substring(0, 200), duration_ms: {}, messages_received: messages.length, messages }};
        }})()"#, json_escape(&p.url), json_escape(&p.query), variables, duration, duration);
        let result = page.evaluate(js).await.mcp()?;
        let val: serde_json::Value = result.into_value().unwrap_or(serde_json::json!(null));
        json_ok(&serde_json::json!({ "action": "graphql_subscribe", "result": val }))
    }

    pub(crate) async fn extract_compact(
        &self,
        p: ExtractCompactParams,
    ) -> Result<CallToolResult, McpError> {
        let page = ensure_page(&self.browser).await?;
        let fmt = p.format.as_deref().unwrap_or("text");
        let extract_format = match fmt {
            "markdown" => onecrawl_cdp::ExtractFormat::Markdown,
            _ => onecrawl_cdp::ExtractFormat::Text,
        };
        let result = onecrawl_cdp::extract::extract(&page, None, extract_format)
            .await
            .mcp()?;
        let content = result.content;
        let max_chars = p.max_tokens.unwrap_or(8000) * 4;
        let truncated = if content.len() > max_chars {
            content.chars().take(max_chars).collect::<String>()
        } else {
            content.clone()
        };
        let chars = truncated.len();
        json_ok(&serde_json::json!({
            "content": truncated,
            "format": fmt,
            "tokens": chars / 4,
            "truncated": content.len() > max_chars
        }))
    }
}
