//! DOM query and content methods for Page.

use std::sync::Arc;

use onecrawl_protocol::cdp::browser_protocol::dom::*;
use onecrawl_protocol::cdp::js_protocol::runtime::CallArgument;
use onecrawl_protocol::cdp::js_protocol::runtime::CallFunctionOnParams;

use crate::element::Element;
use crate::error::Result;
use crate::handler::domworld::DOMWorldKind;

use super::Page;

impl Page {
    /// Returns the root DOM node (and optionally the subtree) of the page.
    ///
    /// # Note: This does not return the actual HTML document of the page. To
    /// retrieve the HTML content of the page see `Page::content`.
    pub async fn get_document(&self) -> Result<Node> {
        let resp = self.execute(GetDocumentParams::default()).await?;
        Ok(resp.result.root)
    }

    /// Returns the first element in the document which matches the given CSS
    /// selector.
    ///
    /// Execute a query selector on the document's node.
    pub async fn find_element(&self, selector: impl Into<String>) -> Result<Element> {
        let root = self.get_document().await?.node_id;
        let node_id = self.inner.find_element(selector, root).await?;
        Element::new(Arc::clone(&self.inner), node_id).await
    }

    /// Return all `Element`s in the document that match the given selector
    pub async fn find_elements(&self, selector: impl Into<String>) -> Result<Vec<Element>> {
        let root = self.get_document().await?.node_id;
        let node_ids = self.inner.find_elements(selector, root).await?;
        Element::from_nodes(&self.inner, &node_ids).await
    }

    /// Returns the first element in the document which matches the given xpath
    /// selector.
    ///
    /// Execute a xpath selector on the document's node.
    pub async fn find_xpath(&self, selector: impl Into<String>) -> Result<Element> {
        self.get_document().await?;
        let node_id = self.inner.find_xpaths(selector).await?[0];
        Element::new(Arc::clone(&self.inner), node_id).await
    }

    /// Return all `Element`s in the document that match the given xpath selector
    pub async fn find_xpaths(&self, selector: impl Into<String>) -> Result<Vec<Element>> {
        self.get_document().await?;
        let node_ids = self.inner.find_xpaths(selector).await?;
        Element::from_nodes(&self.inner, &node_ids).await
    }

    /// Describes node given its id
    pub async fn describe_node(&self, node_id: NodeId) -> Result<Node> {
        let resp = self
            .execute(
                DescribeNodeParams::builder()
                    .node_id(node_id)
                    .depth(100)
                    .build(),
            )
            .await?;
        Ok(resp.result.node)
    }

    /// Returns the HTML content of the page
    pub async fn content(&self) -> Result<String> {
        Ok(self
            .evaluate(
                "{
          let retVal = '';
          if (document.doctype) {
            retVal = new XMLSerializer().serializeToString(document.doctype);
          }
          if (document.documentElement) {
            retVal += document.documentElement.outerHTML;
          }
          retVal
      }
      ",
            )
            .await?
            .into_value()?)
    }

    #[cfg(feature = "bytes")]
    /// Returns the HTML content of the page
    pub async fn content_bytes(&self) -> Result<bytes::Bytes> {
        Ok(self
            .evaluate(
                "{
            let retVal = '';
            if (document.doctype) {
            retVal = new XMLSerializer().serializeToString(document.doctype);
            }
            if (document.documentElement) {
            retVal += document.documentElement.outerHTML;
            }
            retVal
        }
        ",
            )
            .await?
            .into_value()?)
    }

    /// Set the content of the frame.
    ///
    /// # Example
    /// ```no_run
    /// # use onecrawl_browser::page::Page;
    /// # use onecrawl_browser::error::Result;
    /// # async fn demo(page: Page) -> Result<()> {
    ///     page.set_content("<body>
    ///  <h1>This was set via onecrawl_browser</h1>
    ///  </body>").await?;
    ///     # Ok(())
    /// # }
    /// ```
    pub async fn set_content(&self, html: impl AsRef<str>) -> Result<&Self> {
        let mut call = CallFunctionOnParams::builder()
            .function_declaration(
                "(html) => {
            document.open();
            document.write(html);
            document.close();
        }",
            )
            .argument(
                CallArgument::builder()
                    .value(serde_json::json!(html.as_ref()))
                    .build(),
            )
            .build()
            .unwrap();

        call.execution_context_id = self
            .inner
            .execution_context_for_world(None, DOMWorldKind::Secondary)
            .await?;

        self.evaluate_function(call).await?;
        // relying that document.open() will reset frame lifecycle with "init"
        // lifecycle event. @see https://crrev.com/608658
        self.wait_for_navigation().await
    }
}
