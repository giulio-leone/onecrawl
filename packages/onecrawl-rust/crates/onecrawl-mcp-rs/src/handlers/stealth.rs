//! Handler implementations for the `stealth` super-tool.

use rmcp::{ErrorData as McpError, model::*};
use crate::cdp_tools::*;
use crate::helpers::{mcp_err, ensure_page, json_ok, text_ok, McpResult};
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
