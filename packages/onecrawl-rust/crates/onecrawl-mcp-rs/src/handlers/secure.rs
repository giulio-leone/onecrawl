//! Handler implementations for the `secure` super-tool.

use rmcp::{ErrorData as McpError, model::*};
use crate::cdp_tools::*;
use crate::helpers::{mcp_err, ensure_page, json_ok, text_ok, McpResult};
use crate::types::*;
use crate::OneCrawlMcp;
use base64::{Engine as _, engine::general_purpose::STANDARD as B64};

impl OneCrawlMcp {

    // ── Crypto tools ──

    pub(crate) fn encrypt(
        &self,
        req: EncryptRequest,
    ) -> Result<CallToolResult, McpError> {
        let payload = onecrawl_crypto::encrypt(req.plaintext.as_bytes(), &req.password)
            .mcp()?;

        let salt = B64
            .decode(&payload.salt)
            .mcp()?;
        let nonce = B64
            .decode(&payload.nonce)
            .mcp()?;
        let ct = B64
            .decode(&payload.ciphertext)
            .mcp()?;

        let mut packed = Vec::with_capacity(salt.len() + nonce.len() + ct.len());
        packed.extend_from_slice(&salt);
        packed.extend_from_slice(&nonce);
        packed.extend_from_slice(&ct);

        Ok(CallToolResult::success(vec![Content::text(
            B64.encode(&packed),
        )]))
    }


    pub(crate) fn decrypt(
        &self,
        req: DecryptRequest,
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

        let plaintext = onecrawl_crypto::decrypt(&payload, &req.password)
            .mcp()?;

        let text = String::from_utf8(plaintext).unwrap_or_else(|e| B64.encode(e.into_bytes()));

        Ok(CallToolResult::success(vec![Content::text(text)]))
    }


    pub(crate) fn generate_pkce(&self) -> Result<CallToolResult, McpError> {
        let challenge =
            onecrawl_crypto::generate_pkce_challenge().mcp()?;
        json_ok(&PkceResponse {
            code_verifier: &challenge.code_verifier,
            code_challenge: &challenge.code_challenge,
        })
    }


    pub(crate) fn generate_totp(
        &self,
        req: TotpRequest,
    ) -> Result<CallToolResult, McpError> {
        let config = onecrawl_core::TotpConfig {
            secret: req.secret,
            ..Default::default()
        };
        let code =
            onecrawl_crypto::totp::generate_totp(&config).mcp()?;
        Ok(CallToolResult::success(vec![Content::text(code)]))
    }

    // ── Parser tools ──


    // ── Parser tools ──

    pub(crate) fn parse_accessibility_tree(
        &self,
        req: HtmlRequest,
    ) -> Result<CallToolResult, McpError> {
        let tree = onecrawl_parser::get_accessibility_tree(&req.html)
            .mcp()?;
        let rendered = onecrawl_parser::accessibility::render_tree(&tree, 0, false);
        Ok(CallToolResult::success(vec![Content::text(rendered)]))
    }


    pub(crate) fn query_selector(
        &self,
        req: SelectorRequest,
    ) -> Result<CallToolResult, McpError> {
        let elements = onecrawl_parser::query_selector(&req.html, &req.selector)
            .mcp()?;
        let json = serde_json::to_string(&elements).mcp()?;
        Ok(CallToolResult::success(vec![Content::text(json)]))
    }


    pub(crate) fn html_extract_text(
        &self,
        req: HtmlRequest,
    ) -> Result<CallToolResult, McpError> {
        let texts =
            onecrawl_parser::extract_text(&req.html, "body").mcp()?;
        Ok(CallToolResult::success(vec![Content::text(
            texts.join("\n"),
        )]))
    }


    pub(crate) fn html_extract_links(
        &self,
        req: HtmlRequest,
    ) -> Result<CallToolResult, McpError> {
        let links = onecrawl_parser::extract::extract_links(&req.html)
            .mcp()?;
        let result: Vec<LinkInfo> = links
            .into_iter()
            .map(|(href, text)| {
                let is_external = href.starts_with("http://") || href.starts_with("https://");
                LinkInfo { href, text, is_external }
            })
            .collect();
        let json = serde_json::to_string(&result).mcp()?;
        Ok(CallToolResult::success(vec![Content::text(json)]))
    }

    // ── Storage tools ──


    // ── Storage tools ──

    pub(crate) fn store_set(
        &self,
        req: StoreSetRequest,
    ) -> Result<CallToolResult, McpError> {
        let store = self.open_store()?;
        store
            .set(&req.key, req.value.as_bytes())
            .mcp()?;
        Ok(CallToolResult::success(vec![Content::text(format!(
            "stored key \"{}\"",
            req.key
        ))]))
    }


    pub(crate) fn store_get(
        &self,
        req: StoreGetRequest,
    ) -> Result<CallToolResult, McpError> {
        let store = self.open_store()?;
        let value = store.get(&req.key).mcp()?;
        match value {
            Some(v) => {
                let text = String::from_utf8(v).unwrap_or_else(|e| B64.encode(e.into_bytes()));
                Ok(CallToolResult::success(vec![Content::text(text)]))
            }
            None => Ok(CallToolResult::success(vec![Content::text(format!(
                "key \"{}\" not found",
                req.key
            ))])),
        }
    }


    pub(crate) fn store_list(&self) -> Result<CallToolResult, McpError> {
        let store = self.open_store()?;
        let keys = store.list("").mcp()?;
        let json = serde_json::to_string(&keys).mcp()?;
        Ok(CallToolResult::success(vec![Content::text(json)]))
    }

    // ════════════════════════════════════════════════════════════════
    //  CDP tools — Navigation & Page Control
    // ════════════════════════════════════════════════════════════════


    //  Passkey / WebAuthn tools

    pub(crate) async fn auth_passkey_enable(
        &self,
        p: PasskeyEnableParams,
    ) -> Result<CallToolResult, McpError> {
        let page = ensure_page(&self.browser).await?;
        let config = onecrawl_cdp::webauthn::VirtualAuthenticator {
            id: format!(
                "auth-{}",
                std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_millis()
            ),
            protocol: p.protocol.unwrap_or_else(|| "ctap2".into()),
            transport: p.transport.unwrap_or_else(|| "internal".into()),
            has_resident_key: true,
            has_user_verification: true,
            is_user_verified: true,
        };
        onecrawl_cdp::webauthn::enable_virtual_authenticator(&page, &config)
            .await
            .mcp()?;
        text_ok("Virtual authenticator enabled")
    }


    pub(crate) async fn auth_passkey_add(
        &self,
        p: PasskeyAddParams,
    ) -> Result<CallToolResult, McpError> {
        let page = ensure_page(&self.browser).await?;
        let cred = onecrawl_cdp::webauthn::VirtualCredential {
            credential_id: p.credential_id,
            rp_id: p.rp_id,
            user_handle: p.user_handle.unwrap_or_default(),
            sign_count: 0,
        };
        onecrawl_cdp::webauthn::add_virtual_credential(&page, &cred)
            .await
            .mcp()?;
        text_ok("Credential added")
    }


    pub(crate) async fn auth_passkey_list(
        &self,
        _p: PasskeyListParams,
    ) -> Result<CallToolResult, McpError> {
        let page = ensure_page(&self.browser).await?;
        let creds = onecrawl_cdp::webauthn::get_virtual_credentials(&page)
            .await
            .mcp()?;
        json_ok(&creds)
    }


    pub(crate) async fn auth_passkey_log(
        &self,
        _p: PasskeyLogParams,
    ) -> Result<CallToolResult, McpError> {
        let page = ensure_page(&self.browser).await?;
        let log = onecrawl_cdp::webauthn::get_webauthn_log(&page)
            .await
            .mcp()?;
        json_ok(&log)
    }


    pub(crate) async fn auth_passkey_disable(
        &self,
        _p: PasskeyDisableParams,
    ) -> Result<CallToolResult, McpError> {
        let page = ensure_page(&self.browser).await?;
        onecrawl_cdp::webauthn::disable_virtual_authenticator(&page)
            .await
            .mcp()?;
        text_ok("Virtual authenticator disabled")
    }


    pub(crate) async fn auth_passkey_remove(
        &self,
        p: PasskeyRemoveParams,
    ) -> Result<CallToolResult, McpError> {
        let page = ensure_page(&self.browser).await?;
        let removed = onecrawl_cdp::webauthn::remove_virtual_credential(&page, &p.credential_id)
            .await
            .mcp()?;
        json_ok(&RemovedResult { removed })
    }

    // ════════════════════════════════════════════════════════════════
    //  Agent tools — Enhanced Agentic API Layer
    // ════════════════════════════════════════════════════════════════

}
