use async_trait::async_trait;
use onecrawl_protocol::cdp::browser_protocol::page::CaptureScreenshotFormat;

use crate::element::Element;
use crate::error::Result;
use super::{ElementPort, ElementRect};

#[async_trait]
impl ElementPort for Element {
    async fn click_element(&self) -> Result<()> {
        self.click().await?;
        Ok(())
    }

    async fn hover_element(&self) -> Result<()> {
        self.hover().await?;
        Ok(())
    }

    async fn focus_element(&self) -> Result<()> {
        self.focus().await?;
        Ok(())
    }

    async fn type_text(&self, text: &str) -> Result<()> {
        self.type_str(text).await?;
        Ok(())
    }

    async fn press_key(&self, key: &str) -> Result<()> {
        Element::press_key(self, key).await?;
        Ok(())
    }

    async fn scroll_into_view(&self) -> Result<()> {
        Element::scroll_into_view(self).await?;
        Ok(())
    }

    async fn inner_text(&self) -> Result<Option<String>> {
        Element::inner_text(self).await
    }

    async fn inner_html(&self) -> Result<Option<String>> {
        Element::inner_html(self).await
    }

    async fn outer_html(&self) -> Result<Option<String>> {
        Element::outer_html(self).await
    }

    async fn get_attribute(&self, name: &str) -> Result<Option<String>> {
        self.attribute(name).await
    }

    async fn get_property(&self, name: &str) -> Result<Option<serde_json::Value>> {
        self.property(name).await
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

    async fn bounding_box(&self) -> Result<ElementRect> {
        let bb = Element::bounding_box(self).await?;
        Ok(ElementRect {
            x: bb.x,
            y: bb.y,
            width: bb.width,
            height: bb.height,
        })
    }

    async fn capture_screenshot(&self) -> Result<Vec<u8>> {
        self.screenshot(CaptureScreenshotFormat::Png).await
    }
}
