use async_trait::async_trait;
use onecrawl_protocol::cdp::browser_protocol::page::PrintToPdfParams;
use onecrawl_protocol::cdp::js_protocol::runtime::{CallArgument, CallFunctionOnParams};

use crate::error::Result;
use crate::page::{Page, ScreenshotParams};
use super::{ElementPort, PagePort};

#[async_trait]
impl PagePort for Page {
    async fn goto_url(&self, url: &str) -> Result<()> {
        self.goto(url).await?;
        Ok(())
    }

    async fn reload_page(&self) -> Result<()> {
        self.reload().await?;
        Ok(())
    }

    async fn wait_for_navigation(&self) -> Result<()> {
        Page::wait_for_navigation(self).await?;
        Ok(())
    }

    async fn current_url(&self) -> Result<Option<String>> {
        self.url().await
    }

    async fn page_title(&self) -> Result<Option<String>> {
        self.get_title().await
    }

    async fn page_content(&self) -> Result<String> {
        self.content().await
    }

    async fn set_page_content(&self, html: &str) -> Result<()> {
        self.set_content(html).await?;
        Ok(())
    }

    async fn query_selector(&self, selector: &str) -> Result<Box<dyn ElementPort>> {
        let el = self.find_element(selector).await?;
        Ok(Box::new(el) as Box<dyn ElementPort>)
    }

    async fn query_selector_all(&self, selector: &str) -> Result<Vec<Box<dyn ElementPort>>> {
        let elements = self.find_elements(selector).await?;
        Ok(elements
            .into_iter()
            .map(|el| Box::new(el) as Box<dyn ElementPort>)
            .collect())
    }

    async fn evaluate_expression(&self, expression: &str) -> Result<serde_json::Value> {
        let result = Page::evaluate_expression(self, expression).await?;
        Ok(result.value().cloned().unwrap_or(serde_json::Value::Null))
    }

    async fn evaluate_function(
        &self,
        function_declaration: &str,
        args: Vec<serde_json::Value>,
    ) -> Result<serde_json::Value> {
        let mut params = CallFunctionOnParams::new(function_declaration);
        if !args.is_empty() {
            params.arguments = Some(
                args.into_iter()
                    .map(|v| CallArgument::builder().value(v).build())
                    .collect(),
            );
        }
        let result = Page::evaluate_function(self, params).await?;
        Ok(result.value().cloned().unwrap_or(serde_json::Value::Null))
    }

    async fn capture_screenshot(&self) -> Result<Vec<u8>> {
        self.screenshot(ScreenshotParams::default()).await
    }

    async fn capture_pdf(&self) -> Result<Vec<u8>> {
        self.pdf(PrintToPdfParams::default()).await
    }

    async fn activate_page(&self) -> Result<()> {
        self.activate().await?;
        Ok(())
    }

    async fn close_page(&self) -> Result<()> {
        self.execute(
            onecrawl_protocol::cdp::browser_protocol::page::CloseParams::default(),
        )
        .await?;
        Ok(())
    }

    async fn page_metrics(&self) -> Result<Vec<(String, f64)>> {
        let metrics = self.metrics().await?;
        Ok(metrics.into_iter().map(|m| (m.name, m.value)).collect())
    }
}
