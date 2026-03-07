//! DOM mutation observation via JS MutationObserver API.
//!
//! Injects a MutationObserver that records childList, attribute, and
//! characterData changes into `window.__onecrawl_dom_mutations`.

use chromiumoxide::Page;
use onecrawl_core::Result;
use serde::{Deserialize, Serialize};

/// A captured DOM mutation entry.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DomMutation {
    pub mutation_type: String,
    pub target: String,
    pub added_nodes: Vec<String>,
    pub removed_nodes: Vec<String>,
    pub attribute_name: Option<String>,
    pub old_value: Option<String>,
    pub new_value: Option<String>,
    pub timestamp: f64,
}

/// Start observing DOM mutations on the given selector (or `document.body` by default).
pub async fn start_dom_observer(page: &Page, target_selector: Option<&str>) -> Result<()> {
    let target_js = match target_selector {
        Some(sel) if sel.starts_with("document.") => sel.to_string(),
        Some(sel) => format!("document.querySelector('{}')", sel.replace('\\', "\\\\").replace('\'', "\\'")),
        None => "document.body".to_string(),
    };

    let js = format!(
        r#"
        (() => {{
            window.__onecrawl_dom_mutations = window.__onecrawl_dom_mutations || [];
            if (window.__onecrawl_dom_observer) {{
                window.__onecrawl_dom_observer.disconnect();
            }}
            window.__onecrawl_dom_observer = new MutationObserver((mutations) => {{
                for (const m of mutations) {{
                    const entry = {{
                        mutation_type: m.type,
                        target: m.target.tagName
                            ? m.target.tagName.toLowerCase() + (m.target.id ? '#' + m.target.id : '')
                            : '#text',
                        added_nodes: Array.from(m.addedNodes).map(n => n.tagName ? n.tagName.toLowerCase() : '#text'),
                        removed_nodes: Array.from(m.removedNodes).map(n => n.tagName ? n.tagName.toLowerCase() : '#text'),
                        attribute_name: m.attributeName || null,
                        old_value: m.oldValue || null,
                        new_value: m.attributeName && m.target.getAttribute
                            ? m.target.getAttribute(m.attributeName)
                            : null,
                        timestamp: Date.now()
                    }};
                    window.__onecrawl_dom_mutations.push(entry);
                }}
            }});
            const target = {target} || document.body;
            window.__onecrawl_dom_observer.observe(target, {{
                childList: true,
                attributes: true,
                characterData: true,
                subtree: true,
                attributeOldValue: true,
                characterDataOldValue: true
            }});
            return true;
        }})()
        "#,
        target = target_js,
    );

    page.evaluate(js.as_str())
        .await
        .map_err(|e| onecrawl_core::Error::Cdp(format!("start_dom_observer failed: {e}")))?;

    Ok(())
}

/// Drain accumulated DOM mutations.
pub async fn drain_dom_mutations(page: &Page) -> Result<Vec<DomMutation>> {
    let result = page
        .evaluate(
            r#"
            (() => {
                const m = window.__onecrawl_dom_mutations || [];
                window.__onecrawl_dom_mutations = [];
                return m;
            })()
            "#,
        )
        .await
        .map_err(|e| onecrawl_core::Error::Cdp(format!("drain_dom_mutations failed: {e}")))?;

    let mutations: Vec<DomMutation> = result.into_value().unwrap_or_default();

    Ok(mutations)
}

/// Stop the DOM observer.
pub async fn stop_dom_observer(page: &Page) -> Result<()> {
    page.evaluate(
        "if (window.__onecrawl_dom_observer) { window.__onecrawl_dom_observer.disconnect(); window.__onecrawl_dom_observer = null; }",
    )
    .await
    .map_err(|e| onecrawl_core::Error::Cdp(format!("stop_dom_observer failed: {e}")))?;

    Ok(())
}

/// Get a snapshot of the current DOM as HTML.
pub async fn get_dom_snapshot(page: &Page, selector: Option<&str>) -> Result<String> {
    let js = match selector {
        Some(sel) => format!(
            "document.querySelector('{}')?.outerHTML || ''",
            sel.replace('\\', "\\\\").replace('\'', "\\'")
        ),
        None => "document.documentElement.outerHTML".to_string(),
    };

    let result = page
        .evaluate(js.as_str())
        .await
        .map_err(|e| onecrawl_core::Error::Cdp(format!("get_dom_snapshot failed: {e}")))?;

    let html: String = result.into_value().unwrap_or_default();
    Ok(html)
}
