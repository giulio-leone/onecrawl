use colored::Colorize;
use super::super::helpers::{with_page};

// Print (Enhanced)
// ---------------------------------------------------------------------------
// Page Snapshot
// ---------------------------------------------------------------------------

pub async fn pdf(output: &str, landscape: bool, scale: f64) {
    let out = output.to_string();
    with_page(|page| async move {
        let bytes = if landscape || (scale - 1.0).abs() > f64::EPSILON {
            let opts = onecrawl_cdp::PdfOptions {
                landscape,
                scale,
                ..Default::default()
            };
            onecrawl_cdp::screenshot::pdf_with_options(&page, &opts)
                .await
                .map_err(|e| e.to_string())?
        } else {
            onecrawl_cdp::screenshot::pdf(&page)
                .await
                .map_err(|e| e.to_string())?
        };
        std::fs::write(&out, &bytes).map_err(|e| format!("write failed: {e}"))?;
        println!(
            "{} PDF saved to {} ({} bytes)",
            "✓".green(),
            out.cyan(),
            bytes.len()
        );
        Ok(())
    })
    .await;
}

#[allow(clippy::too_many_arguments)]
pub async fn print_pdf(
    output: &str,
    landscape: bool,
    background: bool,
    scale: Option<f64>,
    paper_width: Option<f64>,
    paper_height: Option<f64>,
    margins: Option<&str>,
    page_ranges: Option<String>,
    header: Option<String>,
    footer: Option<String>,
) {
    let out = output.to_string();
    let (mt, mb, ml, mr) = if let Some(m) = margins {
        let parts: Vec<f64> = m.split(',').filter_map(|s| s.trim().parse().ok()).collect();
        (
            parts.first().copied(),
            parts.get(1).copied(),
            parts.get(2).copied(),
            parts.get(3).copied(),
        )
    } else {
        (None, None, None, None)
    };
    with_page(|page| async move {
        let opts = onecrawl_cdp::DetailedPdfOptions {
            landscape: if landscape { Some(true) } else { None },
            print_background: if background { Some(true) } else { None },
            scale,
            paper_width,
            paper_height,
            margin_top: mt,
            margin_bottom: mb,
            margin_left: ml,
            margin_right: mr,
            page_ranges,
            header_template: header,
            footer_template: footer,
            display_header_footer: None,
            prefer_css_page_size: None,
        };
        let bytes = onecrawl_cdp::print::print_to_pdf(&page, &opts)
            .await
            .map_err(|e| e.to_string())?;
        std::fs::write(&out, &bytes).map_err(|e| format!("write failed: {e}"))?;
        println!(
            "{} PDF saved to {} ({} bytes)",
            "✓".green(),
            out.cyan(),
            bytes.len()
        );
        Ok(())
    })
    .await;
}

pub async fn print_metrics() {
    with_page(|page| async move {
        let val = onecrawl_cdp::print::get_print_metrics(&page)
            .await
            .map_err(|e| e.to_string())?;
        println!("{}", serde_json::to_string_pretty(&val).unwrap_or_default());
        Ok(())
    })
    .await;
}

