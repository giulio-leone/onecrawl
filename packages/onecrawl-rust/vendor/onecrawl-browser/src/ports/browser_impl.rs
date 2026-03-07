use async_trait::async_trait;
use onecrawl_protocol::cdp::browser_protocol::target::CreateTargetParams;

use crate::browser::Browser;
use crate::error::Result;
use super::{BrowserPort, PagePort};

#[async_trait]
impl BrowserPort for Browser {
    async fn new_page(&self, url: &str) -> Result<Box<dyn PagePort>> {
        let page = self.new_page(CreateTargetParams::new(url)).await?;
        Ok(Box::new(page) as Box<dyn PagePort>)
    }

    async fn close_browser(&mut self) -> Result<()> {
        self.close().await?;
        Ok(())
    }

    fn websocket_address(&self) -> &str {
        self.websocket_address()
    }

    async fn version(&self) -> Result<String> {
        let v = self.version().await?;
        Ok(v.product)
    }

    async fn user_agent(&self) -> Result<String> {
        Browser::user_agent(self).await
    }

    async fn clear_all_cookies(&self) -> Result<()> {
        self.clear_cookies().await
    }
}
