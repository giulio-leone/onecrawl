use base64::{Engine as _, engine::general_purpose::STANDARD as B64};
use rmcp::{
    ErrorData as McpError, ServerHandler,
    handler::server::{router::tool::ToolRouter, tool::Parameters},
    model::*,
    schemars, tool, tool_router,
};
use std::sync::Arc;

// ──────────────────────────── Parameter types ────────────────────────────

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct EncryptRequest {
    #[schemars(description = "Plaintext string to encrypt")]
    pub plaintext: String,
    #[schemars(description = "Password for key derivation")]
    pub password: String,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct DecryptRequest {
    #[schemars(description = "Base64-encoded ciphertext (salt + nonce + ciphertext)")]
    pub ciphertext: String,
    #[schemars(description = "Password for key derivation")]
    pub password: String,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct TotpRequest {
    #[schemars(description = "Base32-encoded TOTP secret")]
    pub secret: String,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct HtmlRequest {
    #[schemars(description = "Raw HTML string")]
    pub html: String,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct SelectorRequest {
    #[schemars(description = "Raw HTML string")]
    pub html: String,
    #[schemars(description = "CSS selector to query")]
    pub selector: String,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct StoreSetRequest {
    #[schemars(description = "Storage key")]
    pub key: String,
    #[schemars(description = "Value to store")]
    pub value: String,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct StoreGetRequest {
    #[schemars(description = "Storage key to retrieve")]
    pub key: String,
}

// ──────────────────────────── Server ────────────────────────────

#[derive(Clone)]
#[allow(dead_code)]
pub struct OneCrawlMcp {
    tool_router: ToolRouter<Self>,
    store_path: Arc<String>,
    store_password: Arc<String>,
}

fn mcp_err(msg: impl Into<String>) -> McpError {
    McpError::internal_error(msg.into(), None)
}

#[tool_router]
impl OneCrawlMcp {
    pub fn new(store_path: String, store_password: String) -> Self {
        Self {
            tool_router: Self::tool_router(),
            store_path: Arc::new(store_path),
            store_password: Arc::new(store_password),
        }
    }

    fn open_store(&self) -> Result<onecrawl_storage::EncryptedStore, McpError> {
        onecrawl_storage::EncryptedStore::open(
            std::path::Path::new(self.store_path.as_ref()),
            &self.store_password,
        )
        .map_err(|e| mcp_err(e.to_string()))
    }

    // ── Crypto tools ──

    #[tool(description = "Encrypt text with AES-256-GCM. Returns base64-encoded ciphertext (salt+nonce+ct).")]
    fn encrypt(
        &self,
        Parameters(req): Parameters<EncryptRequest>,
    ) -> Result<CallToolResult, McpError> {
        let payload = onecrawl_crypto::encrypt(req.plaintext.as_bytes(), &req.password)
            .map_err(|e| mcp_err(e.to_string()))?;

        let salt = B64.decode(&payload.salt).map_err(|e| mcp_err(e.to_string()))?;
        let nonce = B64.decode(&payload.nonce).map_err(|e| mcp_err(e.to_string()))?;
        let ct = B64
            .decode(&payload.ciphertext)
            .map_err(|e| mcp_err(e.to_string()))?;

        let mut packed = Vec::with_capacity(salt.len() + nonce.len() + ct.len());
        packed.extend_from_slice(&salt);
        packed.extend_from_slice(&nonce);
        packed.extend_from_slice(&ct);

        Ok(CallToolResult::success(vec![Content::text(B64.encode(&packed))]))
    }

    #[tool(description = "Decrypt base64-encoded AES-256-GCM ciphertext (salt+nonce+ct).")]
    fn decrypt(
        &self,
        Parameters(req): Parameters<DecryptRequest>,
    ) -> Result<CallToolResult, McpError> {
        let raw = B64
            .decode(&req.ciphertext)
            .map_err(|e| mcp_err(format!("invalid base64: {e}")))?;

        if raw.len() < 29 {
            return Err(mcp_err(
                "ciphertext too short: need at least 29 bytes (16 salt + 12 nonce + 1 ct)",
            ));
        }

        let payload = onecrawl_core::EncryptedPayload {
            salt: B64.encode(&raw[..16]),
            nonce: B64.encode(&raw[16..28]),
            ciphertext: B64.encode(&raw[28..]),
        };

        let plaintext =
            onecrawl_crypto::decrypt(&payload, &req.password).map_err(|e| mcp_err(e.to_string()))?;

        let text = String::from_utf8(plaintext)
            .unwrap_or_else(|e| B64.encode(e.into_bytes()));

        Ok(CallToolResult::success(vec![Content::text(text)]))
    }

    #[tool(description = "Generate a PKCE S256 challenge pair (code_verifier + code_challenge).")]
    fn generate_pkce(&self) -> Result<CallToolResult, McpError> {
        let challenge =
            onecrawl_crypto::generate_pkce_challenge().map_err(|e| mcp_err(e.to_string()))?;
        let json = serde_json::json!({
            "code_verifier": challenge.code_verifier,
            "code_challenge": challenge.code_challenge,
        });
        Ok(CallToolResult::success(vec![Content::text(json.to_string())]))
    }

    #[tool(description = "Generate a 6-digit TOTP code from a base32 secret.")]
    fn generate_totp(
        &self,
        Parameters(req): Parameters<TotpRequest>,
    ) -> Result<CallToolResult, McpError> {
        let config = onecrawl_core::TotpConfig {
            secret: req.secret,
            ..Default::default()
        };
        let code =
            onecrawl_crypto::totp::generate_totp(&config).map_err(|e| mcp_err(e.to_string()))?;
        Ok(CallToolResult::success(vec![Content::text(code)]))
    }

    // ── Parser tools ──

    #[tool(description = "Parse HTML into an accessibility tree (text representation).")]
    fn parse_accessibility_tree(
        &self,
        Parameters(req): Parameters<HtmlRequest>,
    ) -> Result<CallToolResult, McpError> {
        let tree =
            onecrawl_parser::get_accessibility_tree(&req.html).map_err(|e| mcp_err(e.to_string()))?;
        let rendered = onecrawl_parser::accessibility::render_tree(&tree, 0, false);
        Ok(CallToolResult::success(vec![Content::text(rendered)]))
    }

    #[tool(description = "Query HTML with a CSS selector. Returns JSON array of matching elements.")]
    fn query_selector(
        &self,
        Parameters(req): Parameters<SelectorRequest>,
    ) -> Result<CallToolResult, McpError> {
        let elements = onecrawl_parser::query_selector(&req.html, &req.selector)
            .map_err(|e| mcp_err(e.to_string()))?;
        let json = serde_json::to_string(&elements).map_err(|e| mcp_err(e.to_string()))?;
        Ok(CallToolResult::success(vec![Content::text(json)]))
    }

    #[tool(description = "Extract visible text from HTML.")]
    fn extract_text(
        &self,
        Parameters(req): Parameters<HtmlRequest>,
    ) -> Result<CallToolResult, McpError> {
        let texts =
            onecrawl_parser::extract_text(&req.html, "body").map_err(|e| mcp_err(e.to_string()))?;
        Ok(CallToolResult::success(vec![Content::text(texts.join("\n"))]))
    }

    #[tool(description = "Extract all links from HTML. Returns JSON array with href, text, is_external.")]
    fn extract_links(
        &self,
        Parameters(req): Parameters<HtmlRequest>,
    ) -> Result<CallToolResult, McpError> {
        let links =
            onecrawl_parser::extract::extract_links(&req.html).map_err(|e| mcp_err(e.to_string()))?;
        let result: Vec<serde_json::Value> = links
            .into_iter()
            .map(|(href, text)| {
                let is_external =
                    href.starts_with("http://") || href.starts_with("https://");
                serde_json::json!({ "href": href, "text": text, "is_external": is_external })
            })
            .collect();
        let json = serde_json::to_string(&result).map_err(|e| mcp_err(e.to_string()))?;
        Ok(CallToolResult::success(vec![Content::text(json)]))
    }

    // ── Storage tools ──

    #[tool(description = "Store a key-value pair in encrypted storage.")]
    fn store_set(
        &self,
        Parameters(req): Parameters<StoreSetRequest>,
    ) -> Result<CallToolResult, McpError> {
        let store = self.open_store()?;
        store
            .set(&req.key, req.value.as_bytes())
            .map_err(|e| mcp_err(e.to_string()))?;
        Ok(CallToolResult::success(vec![Content::text(format!(
            "stored key \"{}\"",
            req.key
        ))]))
    }

    #[tool(description = "Retrieve a value from encrypted storage by key.")]
    fn store_get(
        &self,
        Parameters(req): Parameters<StoreGetRequest>,
    ) -> Result<CallToolResult, McpError> {
        let store = self.open_store()?;
        let value = store.get(&req.key).map_err(|e| mcp_err(e.to_string()))?;
        match value {
            Some(v) => {
                let text = String::from_utf8(v)
                    .unwrap_or_else(|e| B64.encode(e.into_bytes()));
                Ok(CallToolResult::success(vec![Content::text(text)]))
            }
            None => Ok(CallToolResult::success(vec![Content::text(format!(
                "key \"{}\" not found",
                req.key
            ))])),
        }
    }

    #[tool(description = "List all keys in encrypted storage.")]
    fn store_list(&self) -> Result<CallToolResult, McpError> {
        let store = self.open_store()?;
        let keys = store.list("").map_err(|e| mcp_err(e.to_string()))?;
        let json = serde_json::to_string(&keys).map_err(|e| mcp_err(e.to_string()))?;
        Ok(CallToolResult::success(vec![Content::text(json)]))
    }
}

impl ServerHandler for OneCrawlMcp {
    fn get_info(&self) -> ServerInfo {
        ServerInfo {
            instructions: Some(
                "OneCrawl MCP server — crypto, parser, and encrypted storage tools".into(),
            ),
            capabilities: ServerCapabilities::builder().enable_tools().build(),
            ..Default::default()
        }
    }
}
