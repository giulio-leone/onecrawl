//! Enhanced PDF generation with detailed options.
//!
//! Extends the basic `screenshot::pdf()` with full control over margins,
//! headers/footers, page ranges, and CSS page-size preferences.

use onecrawl_browser::Page;
use onecrawl_core::Result;
use serde::{Deserialize, Serialize};

/// Detailed options for PDF generation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DetailedPdfOptions {
    pub landscape: Option<bool>,
    pub display_header_footer: Option<bool>,
    pub print_background: Option<bool>,
    pub scale: Option<f64>,
    pub paper_width: Option<f64>,
    pub paper_height: Option<f64>,
    pub margin_top: Option<f64>,
    pub margin_bottom: Option<f64>,
    pub margin_left: Option<f64>,
    pub margin_right: Option<f64>,
    pub page_ranges: Option<String>,
    pub header_template: Option<String>,
    pub footer_template: Option<String>,
    pub prefer_css_page_size: Option<bool>,
}

impl Default for DetailedPdfOptions {
    fn default() -> Self {
        Self {
            landscape: None,
            display_header_footer: None,
            print_background: Some(true),
            scale: None,
            paper_width: None,
            paper_height: None,
            margin_top: None,
            margin_bottom: None,
            margin_left: None,
            margin_right: None,
            page_ranges: None,
            header_template: None,
            footer_template: None,
            prefer_css_page_size: None,
        }
    }
}

/// Generate PDF with detailed options.
pub async fn print_to_pdf(page: &Page, options: &DetailedPdfOptions) -> Result<Vec<u8>> {
    let mut builder = onecrawl_browser::cdp::browser_protocol::page::PrintToPdfParams::builder();

    if let Some(v) = options.landscape {
        builder = builder.landscape(v);
    }
    if let Some(v) = options.display_header_footer {
        builder = builder.display_header_footer(v);
    }
    if let Some(v) = options.print_background {
        builder = builder.print_background(v);
    }
    if let Some(v) = options.scale {
        builder = builder.scale(v);
    }
    if let Some(v) = options.paper_width {
        builder = builder.paper_width(v);
    }
    if let Some(v) = options.paper_height {
        builder = builder.paper_height(v);
    }
    if let Some(v) = options.margin_top {
        builder = builder.margin_top(v);
    }
    if let Some(v) = options.margin_bottom {
        builder = builder.margin_bottom(v);
    }
    if let Some(v) = options.margin_left {
        builder = builder.margin_left(v);
    }
    if let Some(v) = options.margin_right {
        builder = builder.margin_right(v);
    }
    if let Some(ref v) = options.page_ranges {
        builder = builder.page_ranges(v.clone());
    }
    if let Some(ref v) = options.header_template {
        builder = builder.header_template(v.clone());
    }
    if let Some(ref v) = options.footer_template {
        builder = builder.footer_template(v.clone());
    }
    if let Some(v) = options.prefer_css_page_size {
        builder = builder.prefer_css_page_size(v);
    }

    let params = builder.build();

    let bytes = page
        .pdf(params)
        .await
        .map_err(|e| onecrawl_core::Error::Cdp(format!("print_to_pdf failed: {e}")))?;

    Ok(bytes)
}

/// Get page print preview metrics.
pub async fn get_print_metrics(page: &Page) -> Result<serde_json::Value> {
    let result = page
        .evaluate(
            r#"({
                width: document.documentElement.scrollWidth,
                height: document.documentElement.scrollHeight,
                viewportWidth: window.innerWidth,
                viewportHeight: window.innerHeight,
                devicePixelRatio: window.devicePixelRatio,
                mediaQueries: {
                    print: window.matchMedia('print').matches,
                    screen: window.matchMedia('screen').matches
                }
            })"#,
        )
        .await
        .map_err(|e| onecrawl_core::Error::Cdp(format!("get_print_metrics failed: {e}")))?;

    let val: serde_json::Value = result.into_value().unwrap_or(serde_json::Value::Null);
    Ok(val)
}
