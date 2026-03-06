//! Network Intelligence — auto-detect APIs from page traffic, generate SDK stubs,
//! mock server definitions, and request replay sequences.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// A captured API endpoint from network traffic.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiEndpoint {
    pub method: String,
    pub url: String,
    pub path: String,
    pub base_url: String,
    pub query_params: HashMap<String, String>,
    pub request_headers: HashMap<String, String>,
    pub response_headers: HashMap<String, String>,
    pub request_body: Option<serde_json::Value>,
    pub response_body: Option<serde_json::Value>,
    pub status_code: u16,
    pub content_type: Option<String>,
    pub timing_ms: Option<f64>,
    pub category: ApiCategory,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum ApiCategory {
    Rest,
    GraphQL,
    WebSocket,
    Sse,
    Rpc,
    Static,
    Unknown,
}

/// Discovered API schema from traffic analysis.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiSchema {
    pub base_url: String,
    pub endpoints: Vec<EndpointSchema>,
    pub auth_pattern: Option<AuthPattern>,
    pub total_requests: usize,
    pub unique_endpoints: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EndpointSchema {
    pub method: String,
    pub path: String,
    pub path_params: Vec<String>,
    pub query_params: Vec<ParamSchema>,
    pub request_body_schema: Option<serde_json::Value>,
    pub response_body_schema: Option<serde_json::Value>,
    pub status_codes: Vec<u16>,
    pub content_types: Vec<String>,
    pub call_count: usize,
    pub avg_latency_ms: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParamSchema {
    pub name: String,
    pub example_values: Vec<String>,
    pub required: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum AuthPattern {
    Bearer { header: String },
    ApiKey { header: String, prefix: Option<String> },
    Cookie { name: String },
    Basic,
    None,
}

/// Generated SDK stub.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SdkStub {
    pub language: String,
    pub code: String,
    pub endpoints_covered: usize,
}

/// Mock server endpoint definition.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MockEndpoint {
    pub method: String,
    pub path: String,
    pub response_status: u16,
    pub response_body: serde_json::Value,
    pub response_headers: HashMap<String, String>,
    pub delay_ms: Option<u64>,
}

/// Mock server configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MockServerConfig {
    pub port: u16,
    pub endpoints: Vec<MockEndpoint>,
    pub default_response: Option<MockEndpoint>,
}

/// Replay sequence for recorded requests.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReplaySequence {
    pub name: String,
    pub steps: Vec<ReplayStep>,
    pub total_duration_ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReplayStep {
    pub method: String,
    pub url: String,
    pub headers: HashMap<String, String>,
    pub body: Option<String>,
    pub expected_status: u16,
    pub delay_before_ms: u64,
}

/// Classify a network request by its characteristics.
pub fn classify_request(url: &str, content_type: Option<&str>, method: &str) -> ApiCategory {
    if url.contains("/graphql") || url.contains("/gql") {
        return ApiCategory::GraphQL;
    }
    if url.ends_with(".ws") || url.starts_with("wss://") || url.starts_with("ws://") {
        return ApiCategory::WebSocket;
    }
    if content_type.map_or(false, |ct| ct.contains("text/event-stream")) {
        return ApiCategory::Sse;
    }
    if url.contains("/rpc") || url.contains("/jsonrpc") {
        return ApiCategory::Rpc;
    }

    let static_exts = [".js", ".css", ".png", ".jpg", ".jpeg", ".gif", ".svg", ".woff", ".woff2", ".ttf", ".ico"];
    if static_exts.iter().any(|ext| url.contains(ext)) {
        return ApiCategory::Static;
    }

    if content_type.map_or(false, |ct| ct.contains("application/json") || ct.contains("text/plain")) {
        return ApiCategory::Rest;
    }

    match method.to_uppercase().as_str() {
        "GET" | "POST" | "PUT" | "PATCH" | "DELETE" => ApiCategory::Rest,
        _ => ApiCategory::Unknown,
    }
}

/// Extract path parameters from URL patterns.
/// E.g., `/api/users/123/posts/456` → params: ["123", "456"]
/// with template: `/api/users/{id}/posts/{post_id}`
pub fn extract_path_params(url: &str) -> (String, Vec<String>) {
    let path = url.split('?').next().unwrap_or(url);
    let segments: Vec<&str> = path.split('/').collect();
    let mut template_segments = Vec::new();
    let mut params = Vec::new();
    let mut param_count = 0;

    for segment in &segments {
        if segment.is_empty() {
            template_segments.push(String::new());
            continue;
        }
        // Heuristic: if segment is numeric or UUID-like, it's a path param
        if is_likely_param(segment) {
            param_count += 1;
            let name = match param_count {
                1 => "id".to_string(),
                _ => format!("param_{}", param_count),
            };
            params.push(name.clone());
            template_segments.push(format!("{{{}}}", name));
        } else {
            template_segments.push(segment.to_string());
        }
    }

    (template_segments.join("/"), params)
}

fn is_likely_param(segment: &str) -> bool {
    // Numeric
    if segment.chars().all(|c| c.is_ascii_digit()) {
        return true;
    }
    // UUID-like
    if segment.len() >= 20 && segment.chars().all(|c| c.is_ascii_hexdigit() || c == '-') {
        return true;
    }
    // Base64-ish (long alphanumeric)
    if segment.len() > 16 && segment.chars().all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-') {
        return true;
    }
    false
}

/// Detect authentication pattern from request headers.
pub fn detect_auth_pattern(headers: &HashMap<String, String>) -> AuthPattern {
    if let Some(auth) = headers.get("authorization").or_else(|| headers.get("Authorization")) {
        if auth.starts_with("Bearer ") {
            return AuthPattern::Bearer { header: "Authorization".into() };
        }
        if auth.starts_with("Basic ") {
            return AuthPattern::Basic;
        }
        return AuthPattern::ApiKey { header: "Authorization".into(), prefix: None };
    }

    let api_key_headers = ["x-api-key", "api-key", "apikey", "x-auth-token"];
    for h in &api_key_headers {
        let lower_headers: HashMap<String, &String> = headers.iter()
            .map(|(k, v)| (k.to_lowercase(), v))
            .collect();
        if lower_headers.contains_key(*h) {
            return AuthPattern::ApiKey { header: h.to_string(), prefix: None };
        }
    }

    if let Some(cookie) = headers.get("cookie").or_else(|| headers.get("Cookie")) {
        let session_cookies = ["session", "sid", "token", "auth", "jwt"];
        for sc in &session_cookies {
            if cookie.to_lowercase().contains(sc) {
                return AuthPattern::Cookie { name: sc.to_string() };
            }
        }
    }

    AuthPattern::None
}

/// Infer a JSON schema from a sample value.
pub fn infer_json_schema(value: &serde_json::Value) -> serde_json::Value {
    match value {
        serde_json::Value::Null => serde_json::json!({ "type": "null" }),
        serde_json::Value::Bool(_) => serde_json::json!({ "type": "boolean" }),
        serde_json::Value::Number(n) => {
            if n.is_f64() {
                serde_json::json!({ "type": "number" })
            } else {
                serde_json::json!({ "type": "integer" })
            }
        }
        serde_json::Value::String(_) => serde_json::json!({ "type": "string" }),
        serde_json::Value::Array(arr) => {
            let items = arr.first()
                .map(|v| infer_json_schema(v))
                .unwrap_or(serde_json::json!({ "type": "any" }));
            serde_json::json!({ "type": "array", "items": items })
        }
        serde_json::Value::Object(obj) => {
            let mut properties = serde_json::Map::new();
            let required: Vec<String> = obj.keys().cloned().collect();
            for (key, val) in obj {
                properties.insert(key.clone(), infer_json_schema(val));
            }
            serde_json::json!({
                "type": "object",
                "properties": properties,
                "required": required,
            })
        }
    }
}

/// Generate a TypeScript SDK stub from discovered endpoints.
pub fn generate_typescript_sdk(schema: &ApiSchema) -> SdkStub {
    let mut code = String::new();
    code.push_str("// Auto-generated by OneCrawl Network Intelligence\n");
    code.push_str(&format!("// Base URL: {}\n\n", schema.base_url));
    code.push_str("export class ApiClient {\n");
    code.push_str("  private baseUrl: string;\n");
    code.push_str("  private headers: Record<string, string>;\n\n");
    code.push_str("  constructor(baseUrl: string, headers: Record<string, string> = {}) {\n");
    code.push_str("    this.baseUrl = baseUrl;\n");
    code.push_str("    this.headers = { 'Content-Type': 'application/json', ...headers };\n");
    code.push_str("  }\n\n");

    for ep in &schema.endpoints {
        let fn_name = endpoint_to_function_name(&ep.method, &ep.path);
        let params: Vec<String> = ep.path_params.iter()
            .map(|p| format!("{}: string", p))
            .chain(
                ep.query_params.iter().map(|p| format!("{}: string", p.name))
            )
            .collect();

        let has_body = matches!(ep.method.as_str(), "POST" | "PUT" | "PATCH");
        let all_params = if has_body {
            let mut p = params.clone();
            p.push("body?: any".into());
            p
        } else {
            params.clone()
        };

        code.push_str(&format!("  async {}({}) {{\n", fn_name, all_params.join(", ")));

        let mut url_expr = format!("${{this.baseUrl}}{}", ep.path);
        for param in &ep.path_params {
            url_expr = url_expr.replace(&format!("{{{}}}", param), &format!("${{{}}}", param));
        }

        if !ep.query_params.is_empty() {
            let qs: Vec<String> = ep.query_params.iter()
                .map(|p| format!("{}=${{encodeURIComponent({})}}", p.name, p.name))
                .collect();
            url_expr = format!("{}?{}", url_expr, qs.join("&"));
        }

        code.push_str(&format!("    const url = `{}`;\n", url_expr));
        code.push_str("    const resp = await fetch(url, {\n");
        code.push_str(&format!("      method: '{}',\n", ep.method));
        code.push_str("      headers: this.headers,\n");
        if has_body {
            code.push_str("      body: body ? JSON.stringify(body) : undefined,\n");
        }
        code.push_str("    });\n");
        code.push_str("    return resp.json();\n");
        code.push_str("  }\n\n");
    }

    code.push_str("}\n");

    SdkStub {
        language: "typescript".into(),
        code,
        endpoints_covered: schema.endpoints.len(),
    }
}

/// Generate a Python SDK stub.
pub fn generate_python_sdk(schema: &ApiSchema) -> SdkStub {
    let mut code = String::new();
    code.push_str("# Auto-generated by OneCrawl Network Intelligence\n");
    code.push_str("import requests\n\n\n");
    code.push_str("class ApiClient:\n");
    code.push_str(&format!("    \"\"\"Client for {}\"\"\"\n\n", schema.base_url));
    code.push_str("    def __init__(self, base_url: str, headers: dict = None):\n");
    code.push_str("        self.base_url = base_url\n");
    code.push_str("        self.headers = headers or {'Content-Type': 'application/json'}\n");
    code.push_str("        self.session = requests.Session()\n");
    code.push_str("        self.session.headers.update(self.headers)\n\n");

    for ep in &schema.endpoints {
        let fn_name = endpoint_to_python_name(&ep.method, &ep.path);
        let mut params = Vec::new();
        params.push("self".into());
        for p in &ep.path_params {
            params.push(format!("{}: str", p));
        }
        for p in &ep.query_params {
            params.push(format!("{}: str = None", p.name));
        }
        let has_body = matches!(ep.method.as_str(), "POST" | "PUT" | "PATCH");
        if has_body {
            params.push("body: dict = None".into());
        }

        code.push_str(&format!("    def {}({}):\n", fn_name, params.join(", ")));
        let url_template = ep.path.replace('{', "{{").replace('}', "}}");
        let url_template = ep.path_params.iter().fold(url_template, |acc, p| {
            acc.replace(&format!("{{{{{}}}}}", p), &format!("{{{}}}", p))
        });
        code.push_str(&format!("        url = f\"{{self.base_url}}{}\"\n", url_template));

        if !ep.query_params.is_empty() {
            code.push_str("        params = {");
            let qs: Vec<String> = ep.query_params.iter()
                .map(|p| format!("'{}': {}", p.name, p.name))
                .collect();
            code.push_str(&qs.join(", "));
            code.push_str("}\n");
            code.push_str("        params = {k: v for k, v in params.items() if v is not None}\n");
        }

        let method_lower = ep.method.to_lowercase();
        let mut call_args = vec!["url".to_string()];
        if !ep.query_params.is_empty() {
            call_args.push("params=params".into());
        }
        if has_body {
            call_args.push("json=body".into());
        }
        code.push_str(&format!("        resp = self.session.{}({})\n", method_lower, call_args.join(", ")));
        code.push_str("        resp.raise_for_status()\n");
        code.push_str("        return resp.json()\n\n");
    }

    SdkStub {
        language: "python".into(),
        code,
        endpoints_covered: schema.endpoints.len(),
    }
}

/// Generate mock server config from captured traffic.
pub fn generate_mock_config(endpoints: &[ApiEndpoint], port: u16) -> MockServerConfig {
    let mock_endpoints: Vec<MockEndpoint> = endpoints.iter()
        .filter(|e| e.category != ApiCategory::Static)
        .map(|e| MockEndpoint {
            method: e.method.clone(),
            path: e.path.clone(),
            response_status: e.status_code,
            response_body: e.response_body.clone().unwrap_or(serde_json::Value::Null),
            response_headers: {
                let mut h = HashMap::new();
                h.insert("Content-Type".into(), e.content_type.clone().unwrap_or("application/json".into()));
                h
            },
            delay_ms: e.timing_ms.map(|t| t as u64),
        })
        .collect();

    MockServerConfig {
        port,
        endpoints: mock_endpoints,
        default_response: Some(MockEndpoint {
            method: "GET".into(),
            path: "*".into(),
            response_status: 404,
            response_body: serde_json::json!({ "error": "not found" }),
            response_headers: {
                let mut h = HashMap::new();
                h.insert("Content-Type".into(), "application/json".into());
                h
            },
            delay_ms: None,
        }),
    }
}

/// Generate replay sequence from captured endpoints.
pub fn generate_replay_sequence(name: &str, endpoints: &[ApiEndpoint]) -> ReplaySequence {
    let steps: Vec<ReplayStep> = endpoints.iter()
        .filter(|e| e.category != ApiCategory::Static)
        .map(|e| ReplayStep {
            method: e.method.clone(),
            url: e.url.clone(),
            headers: e.request_headers.clone(),
            body: e.request_body.as_ref().map(|b| b.to_string()),
            expected_status: e.status_code,
            delay_before_ms: e.timing_ms.map(|t| (t / 2.0) as u64).unwrap_or(100),
        })
        .collect();

    let total = steps.iter().map(|s| s.delay_before_ms).sum();

    ReplaySequence {
        name: name.to_string(),
        steps,
        total_duration_ms: total,
    }
}

fn endpoint_to_function_name(method: &str, path: &str) -> String {
    let clean = path
        .trim_matches('/')
        .replace('/', "_")
        .replace('{', "")
        .replace('}', "")
        .replace('-', "_");
    format!("{}_{}", method.to_lowercase(), if clean.is_empty() { "root".to_string() } else { clean })
}

fn endpoint_to_python_name(method: &str, path: &str) -> String {
    endpoint_to_function_name(method, path)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn classify_graphql() {
        assert_eq!(classify_request("https://api.example.com/graphql", Some("application/json"), "POST"), ApiCategory::GraphQL);
    }

    #[test]
    fn classify_rest_json() {
        assert_eq!(classify_request("https://api.example.com/users", Some("application/json"), "GET"), ApiCategory::Rest);
    }

    #[test]
    fn classify_static() {
        assert_eq!(classify_request("https://cdn.example.com/style.css", Some("text/css"), "GET"), ApiCategory::Static);
    }

    #[test]
    fn classify_sse() {
        assert_eq!(classify_request("https://api.example.com/events", Some("text/event-stream"), "GET"), ApiCategory::Sse);
    }

    #[test]
    fn classify_rpc() {
        assert_eq!(classify_request("https://api.example.com/rpc", Some("application/json"), "POST"), ApiCategory::Rpc);
    }

    #[test]
    fn extract_params_numeric() {
        let (template, params) = extract_path_params("/api/users/123/posts/456");
        assert_eq!(template, "/api/users/{id}/posts/{param_2}");
        assert_eq!(params.len(), 2);
    }

    #[test]
    fn extract_params_uuid() {
        let (template, params) = extract_path_params("/api/items/550e8400-e29b-41d4-a716-446655440000");
        assert!(template.contains("{id}"));
        assert_eq!(params.len(), 1);
    }

    #[test]
    fn extract_params_no_params() {
        let (template, params) = extract_path_params("/api/users");
        assert_eq!(template, "/api/users");
        assert!(params.is_empty());
    }

    #[test]
    fn detect_bearer_auth() {
        let mut headers = HashMap::new();
        headers.insert("Authorization".into(), "Bearer eyJhbGciOiJIUzI1NiJ9".into());
        match detect_auth_pattern(&headers) {
            AuthPattern::Bearer { .. } => {}
            other => panic!("expected Bearer, got {:?}", other),
        }
    }

    #[test]
    fn detect_api_key_auth() {
        let mut headers = HashMap::new();
        headers.insert("X-API-Key".into(), "sk-1234567890".into());
        match detect_auth_pattern(&headers) {
            AuthPattern::ApiKey { .. } => {}
            other => panic!("expected ApiKey, got {:?}", other),
        }
    }

    #[test]
    fn detect_no_auth() {
        let headers = HashMap::new();
        match detect_auth_pattern(&headers) {
            AuthPattern::None => {}
            other => panic!("expected None, got {:?}", other),
        }
    }

    #[test]
    fn infer_schema_object() {
        let val = serde_json::json!({
            "name": "John",
            "age": 30,
            "active": true,
            "tags": ["a", "b"]
        });
        let schema = infer_json_schema(&val);
        assert_eq!(schema["type"], "object");
        assert_eq!(schema["properties"]["name"]["type"], "string");
        assert_eq!(schema["properties"]["age"]["type"], "integer");
        assert_eq!(schema["properties"]["active"]["type"], "boolean");
        assert_eq!(schema["properties"]["tags"]["type"], "array");
    }

    #[test]
    fn infer_schema_array() {
        let val = serde_json::json!([1, 2, 3]);
        let schema = infer_json_schema(&val);
        assert_eq!(schema["type"], "array");
        assert_eq!(schema["items"]["type"], "integer");
    }

    #[test]
    fn generate_ts_sdk() {
        let schema = ApiSchema {
            base_url: "https://api.example.com".into(),
            endpoints: vec![EndpointSchema {
                method: "GET".into(),
                path: "/users/{id}".into(),
                path_params: vec!["id".into()],
                query_params: vec![],
                request_body_schema: None,
                response_body_schema: None,
                status_codes: vec![200],
                content_types: vec!["application/json".into()],
                call_count: 5,
                avg_latency_ms: 150.0,
            }],
            auth_pattern: None,
            total_requests: 5,
            unique_endpoints: 1,
        };
        let sdk = generate_typescript_sdk(&schema);
        assert!(sdk.code.contains("class ApiClient"));
        assert!(sdk.code.contains("get_users_id"));
        assert_eq!(sdk.endpoints_covered, 1);
    }

    #[test]
    fn generate_py_sdk() {
        let schema = ApiSchema {
            base_url: "https://api.example.com".into(),
            endpoints: vec![EndpointSchema {
                method: "POST".into(),
                path: "/users".into(),
                path_params: vec![],
                query_params: vec![],
                request_body_schema: None,
                response_body_schema: None,
                status_codes: vec![201],
                content_types: vec!["application/json".into()],
                call_count: 3,
                avg_latency_ms: 200.0,
            }],
            auth_pattern: None,
            total_requests: 3,
            unique_endpoints: 1,
        };
        let sdk = generate_python_sdk(&schema);
        assert!(sdk.code.contains("class ApiClient"));
        assert!(sdk.code.contains("post_users"));
        assert_eq!(sdk.endpoints_covered, 1);
    }

    #[test]
    fn generate_mock() {
        let endpoints = vec![ApiEndpoint {
            method: "GET".into(),
            url: "https://api.example.com/users".into(),
            path: "/users".into(),
            base_url: "https://api.example.com".into(),
            query_params: HashMap::new(),
            request_headers: HashMap::new(),
            response_headers: HashMap::new(),
            request_body: None,
            response_body: Some(serde_json::json!([{"id": 1}])),
            status_code: 200,
            content_type: Some("application/json".into()),
            timing_ms: Some(100.0),
            category: ApiCategory::Rest,
        }];
        let config = generate_mock_config(&endpoints, 3001);
        assert_eq!(config.port, 3001);
        assert_eq!(config.endpoints.len(), 1);
        assert!(config.default_response.is_some());
    }

    #[test]
    fn generate_replay() {
        let endpoints = vec![
            ApiEndpoint {
                method: "GET".into(), url: "https://api.example.com/users".into(),
                path: "/users".into(), base_url: "https://api.example.com".into(),
                query_params: HashMap::new(), request_headers: HashMap::new(),
                response_headers: HashMap::new(), request_body: None,
                response_body: None, status_code: 200,
                content_type: None, timing_ms: Some(50.0),
                category: ApiCategory::Rest,
            },
            ApiEndpoint {
                method: "POST".into(), url: "https://api.example.com/users".into(),
                path: "/users".into(), base_url: "https://api.example.com".into(),
                query_params: HashMap::new(), request_headers: HashMap::new(),
                response_headers: HashMap::new(), request_body: Some(serde_json::json!({"name": "test"})),
                response_body: None, status_code: 201,
                content_type: None, timing_ms: Some(100.0),
                category: ApiCategory::Rest,
            },
        ];
        let seq = generate_replay_sequence("test_flow", &endpoints);
        assert_eq!(seq.steps.len(), 2);
        assert_eq!(seq.name, "test_flow");
    }

    #[test]
    fn function_name_generation() {
        assert_eq!(endpoint_to_function_name("GET", "/api/users"), "get_api_users");
        assert_eq!(endpoint_to_function_name("POST", "/api/users/{id}"), "post_api_users_id");
        assert_eq!(endpoint_to_function_name("GET", "/"), "get_root");
    }
}
