//! Handler implementations for the `secure` super-tool.

use rmcp::{ErrorData as McpError, model::*};
use crate::cdp_tools::*;
use crate::helpers::{mcp_err, ensure_page, json_ok, json_escape, text_ok, McpResult};
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

    // ════════════════════════════════════════════════════════════════
    //  Authentication Flows
    // ════════════════════════════════════════════════════════════════

    pub(crate) async fn auth_oauth2(&self, p: AuthOauth2Params) -> Result<CallToolResult, McpError> {
        let use_pkce = p.use_pkce.unwrap_or(true);
        let scopes = p.scopes.as_ref().map(|s| s.join(" ")).unwrap_or_else(|| "openid profile email".to_string());

        // Generate PKCE pair if requested
        let pkce = if use_pkce {
            let pair = onecrawl_crypto::pkce::generate_pkce_challenge().map_err(|e| mcp_err(&format!("PKCE generation failed: {e}")))?;
            Some(serde_json::json!({
                "code_verifier": pair.code_verifier,
                "code_challenge": pair.code_challenge,
                "method": "S256"
            }))
        } else { None };

        let redirect_uri = p.redirect_uri.as_deref().unwrap_or("http://localhost:3000/callback");

        // Build authorization URL
        let mut auth_url = format!("{}?response_type=code&client_id={}&redirect_uri={}&scope={}",
            p.auth_url, p.client_id, redirect_uri, scopes);
        if let Some(ref pkce_data) = pkce {
            auth_url.push_str(&format!("&code_challenge={}&code_challenge_method=S256",
                pkce_data["code_challenge"].as_str().unwrap_or("")));
        }

        // Store session info
        let mut state = self.browser.lock().await;
        state.auth_sessions.insert("oauth2".to_string(), serde_json::json!({
            "auth_url": p.auth_url,
            "token_url": p.token_url,
            "client_id": p.client_id,
            "redirect_uri": redirect_uri,
            "scopes": scopes,
            "pkce": pkce,
        }));
        state.auth_status = Some("oauth2_initiated".to_string());

        json_ok(&serde_json::json!({
            "action": "auth_oauth2",
            "authorization_url": auth_url,
            "token_url": p.token_url,
            "use_pkce": use_pkce,
            "scopes": scopes,
            "status": "authorization_url_generated",
            "next_step": "Navigate to authorization_url, complete login, capture redirect code"
        }))
    }

    pub(crate) async fn auth_session(&self, p: AuthSessionParams) -> Result<CallToolResult, McpError> {
        let page = ensure_page(&self.browser).await?;

        if p.export.unwrap_or(false) {
            // Export current cookies and storage
            let js = r#"(() => {
                const storage = {};
                try {
                    for (let i = 0; i < localStorage.length; i++) {
                        const key = localStorage.key(i);
                        storage[key] = localStorage.getItem(key);
                    }
                } catch(_) {}
                return {
                    url: location.href,
                    cookies: document.cookie,
                    localStorage: storage
                };
            })()"#;
            let result = page.evaluate(js).await.mcp()?;
            let session_data: serde_json::Value = result.into_value().unwrap_or(serde_json::json!({}));

            let mut state = self.browser.lock().await;
            state.auth_sessions.insert(p.name.clone(), session_data.clone());

            json_ok(&serde_json::json!({
                "action": "auth_session",
                "name": p.name,
                "operation": "export",
                "session_data": session_data,
                "stored": true
            }))
        } else if let Some(import_data) = p.import_data {
            // Import session data
            let data: serde_json::Value = serde_json::from_str(&import_data)
                .map_err(|e| mcp_err(format!("invalid session data: {e}")))?;

            if let Some(url) = data.get("url").and_then(|v| v.as_str()) {
                page.goto(url).await.mcp()?;
            }

            if let Some(storage) = data.get("localStorage").and_then(|v| v.as_object()) {
                for (key, value) in storage {
                    let val_str = value.as_str().unwrap_or("");
                    let js = format!("localStorage.setItem({}, {})",
                        json_escape(key), json_escape(val_str));
                    page.evaluate(js).await.mcp()?;
                }
            }

            let mut state = self.browser.lock().await;
            state.auth_sessions.insert(p.name.clone(), data);

            json_ok(&serde_json::json!({
                "action": "auth_session",
                "name": p.name,
                "operation": "import",
                "restored": true
            }))
        } else {
            // Just check session status
            let state = self.browser.lock().await;
            let session = state.auth_sessions.get(&p.name);
            json_ok(&serde_json::json!({
                "action": "auth_session",
                "name": p.name,
                "exists": session.is_some(),
                "session": session
            }))
        }
    }

    pub(crate) async fn auth_form_login(&self, p: AuthFormLoginParams) -> Result<CallToolResult, McpError> {
        let page = ensure_page(&self.browser).await?;

        // Navigate to login page
        page.goto(&p.url).await.mcp()?;
        tokio::time::sleep(tokio::time::Duration::from_millis(1000)).await;

        // Auto-detect or use provided selectors
        let user_sel = p.username_selector.as_deref().unwrap_or("input[type='email'], input[name='email'], input[name='username'], input[type='text']:first-of-type");
        let pass_sel = p.password_selector.as_deref().unwrap_or("input[type='password']");
        let submit_sel = p.submit_selector.as_deref().unwrap_or("button[type='submit'], input[type='submit'], button:has-text('Login'), button:has-text('Sign in')");

        // Fill credentials via JS
        let js = format!(r#"(() => {{
            const userField = document.querySelector({user_sel_js});
            const passField = document.querySelector({pass_sel_js});

            if (!userField) return {{ error: 'username field not found', selector: {user_sel_js} }};
            if (!passField) return {{ error: 'password field not found', selector: {pass_sel_js} }};

            userField.value = {username_js};
            userField.dispatchEvent(new Event('input', {{ bubbles: true }}));
            userField.dispatchEvent(new Event('change', {{ bubbles: true }}));

            passField.value = {password_js};
            passField.dispatchEvent(new Event('input', {{ bubbles: true }}));
            passField.dispatchEvent(new Event('change', {{ bubbles: true }}));

            const submitBtn = document.querySelector({submit_sel_js});
            if (submitBtn) submitBtn.click();

            return {{ filled: true, submitted: !!submitBtn }};
        }})()"#,
            user_sel_js = json_escape(user_sel),
            pass_sel_js = json_escape(pass_sel),
            submit_sel_js = json_escape(submit_sel),
            username_js = json_escape(&p.username),
            password_js = json_escape(&p.password),
        );

        let result = page.evaluate(js).await.mcp()?;
        let val: serde_json::Value = result.into_value().unwrap_or(serde_json::json!({}));

        // Wait for navigation
        tokio::time::sleep(tokio::time::Duration::from_millis(2000)).await;

        let mut state = self.browser.lock().await;
        state.auth_status = Some("form_login_attempted".to_string());

        json_ok(&serde_json::json!({
            "action": "auth_form_login",
            "url": p.url,
            "result": val,
            "status": "login_attempted"
        }))
    }

    pub(crate) async fn auth_mfa(&self, p: AuthMfaParams) -> Result<CallToolResult, McpError> {
        let page = ensure_page(&self.browser).await?;

        let code = if let Some(manual_code) = &p.code {
            manual_code.clone()
        } else if p.mfa_type == "totp" {
            if let Some(secret) = &p.totp_secret {
                let config = onecrawl_core::TotpConfig {
                    secret: secret.clone(),
                    digits: 6,
                    period: 30,
                    algorithm: onecrawl_core::TotpAlgorithm::Sha1,
                };
                onecrawl_crypto::totp::generate_totp(&config)
                    .map_err(|e| mcp_err(&format!("TOTP generation failed: {e}")))?
            } else {
                return json_ok(&serde_json::json!({ "error": "totp_secret required for TOTP MFA" }));
            }
        } else {
            return json_ok(&serde_json::json!({
                "action": "auth_mfa",
                "mfa_type": p.mfa_type,
                "status": "awaiting_code",
                "message": "Provide code parameter or totp_secret for auto-generation"
            }));
        };

        let code_sel = p.code_selector.as_deref().unwrap_or("input[type='text'], input[name='code'], input[name='otp'], input[autocomplete='one-time-code']");
        let submit_sel = p.submit_selector.as_deref().unwrap_or("button[type='submit'], input[type='submit']");

        let js = format!(r#"(() => {{
            const codeField = document.querySelector({code_sel_js});
            if (!codeField) return {{ error: 'MFA code field not found' }};

            codeField.value = {code_js};
            codeField.dispatchEvent(new Event('input', {{ bubbles: true }}));
            codeField.dispatchEvent(new Event('change', {{ bubbles: true }}));

            const submitBtn = document.querySelector({submit_sel_js});
            if (submitBtn) submitBtn.click();

            return {{ filled: true, submitted: !!submitBtn }};
        }})()"#,
            code_sel_js = json_escape(code_sel),
            code_js = json_escape(&code),
            submit_sel_js = json_escape(submit_sel),
        );

        let result = page.evaluate(js).await.mcp()?;
        let val: serde_json::Value = result.into_value().unwrap_or(serde_json::json!({}));

        tokio::time::sleep(tokio::time::Duration::from_millis(2000)).await;

        let mut state = self.browser.lock().await;
        state.auth_status = Some("mfa_completed".to_string());

        json_ok(&serde_json::json!({
            "action": "auth_mfa",
            "mfa_type": p.mfa_type,
            "result": val,
            "status": "mfa_submitted"
        }))
    }

    pub(crate) async fn auth_status_check(&self) -> Result<CallToolResult, McpError> {
        let state = self.browser.lock().await;
        let sessions: Vec<&String> = state.auth_sessions.keys().collect();

        json_ok(&serde_json::json!({
            "action": "auth_status",
            "current_status": state.auth_status,
            "active_sessions": sessions,
            "session_count": sessions.len()
        }))
    }

    pub(crate) async fn auth_logout(&self) -> Result<CallToolResult, McpError> {
        let page = ensure_page(&self.browser).await?;

        // Clear all auth state
        let js = r#"(() => {
            localStorage.clear();
            sessionStorage.clear();
            document.cookie.split(';').forEach(c => {
                document.cookie = c.replace(/^ +/, '').replace(/=.*/, '=;expires=' + new Date().toUTCString() + ';path=/');
            });
            return { cleared: true };
        })()"#;

        page.evaluate(js).await.mcp()?;

        let mut state = self.browser.lock().await;
        state.auth_sessions.clear();
        state.auth_status = Some("logged_out".to_string());

        json_ok(&serde_json::json!({
            "action": "auth_logout",
            "status": "logged_out",
            "sessions_cleared": true,
            "cookies_cleared": true,
            "storage_cleared": true
        }))
    }

    pub(crate) fn credential_store(&self, p: CredentialStoreParams) -> Result<CallToolResult, McpError> {
        // Store credentials in encrypted KV store
        let store = self.open_store()?;
        let cred_value = serde_json::json!({
            "username": p.username,
            "password": p.password,
            "domain": p.domain,
            "metadata": p.metadata,
        });
        let json_str = serde_json::to_string(&cred_value).mcp()?;
        store.set(&format!("cred:{}", p.label), json_str.as_bytes()).mcp()?;

        json_ok(&serde_json::json!({
            "action": "credential_store",
            "label": p.label,
            "domain": p.domain,
            "stored": true
        }))
    }

    pub(crate) fn credential_get(&self, p: CredentialGetParams) -> Result<CallToolResult, McpError> {
        let store = self.open_store()?;
        let key = format!("cred:{}", p.label);
        match store.get(&key) {
            Ok(Some(val)) => {
                let val_str = std::str::from_utf8(&val).mcp()?;
                let cred: serde_json::Value = serde_json::from_str(val_str).mcp()?;
                json_ok(&serde_json::json!({
                    "action": "credential_get",
                    "label": p.label,
                    "found": true,
                    "credential": cred
                }))
            }
            Ok(None) => json_ok(&serde_json::json!({
                "action": "credential_get",
                "label": p.label,
                "found": false
            })),
            Err(e) => Err(mcp_err(format!("credential retrieval failed: {e}"))),
        }
    }

}
