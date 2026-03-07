//! Adaptive Element Tracker — smart element relocation using similarity algorithms.
//!
//! Fingerprints DOM elements and relocates them after page structure changes
//! using structural, content, and visual similarity scoring.

use onecrawl_browser::Page;
use onecrawl_core::{Error, Result};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ElementFingerprint {
    pub selector: String,
    pub tag: String,
    pub classes: Vec<String>,
    pub attributes: std::collections::HashMap<String, String>,
    pub text_preview: String,
    pub parent_tag: String,
    pub sibling_count: usize,
    pub child_count: usize,
    pub depth: usize,
    pub position: Option<(f64, f64, f64, f64)>,
    pub computed_styles: std::collections::HashMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrackedElement {
    pub id: String,
    pub original: ElementFingerprint,
    pub current_selector: String,
    pub confidence: f64,
    pub last_found: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ElementMatch {
    pub selector: String,
    pub score: f64,
    /// `"exact"`, `"structural"`, `"content"`, or `"visual"`
    pub match_type: String,
}

/// Capture a fingerprint of an element for later relocation.
pub async fn fingerprint_element(page: &Page, selector: &str) -> Result<ElementFingerprint> {
    let escaped = selector.replace('\\', "\\\\").replace('\'', "\\'");
    let js = format!(
        r#"
        (() => {{
            const el = document.querySelector('{escaped}');
            if (!el) return null;

            const rect = el.getBoundingClientRect();
            const style = window.getComputedStyle(el);

            let depth = 0;
            let p = el;
            while (p.parentElement) {{ depth++; p = p.parentElement; }}

            const styleProps = ['display', 'position', 'font-size', 'color', 'background-color', 'margin', 'padding'];
            const computedStyles = {{}};
            styleProps.forEach(prop => {{
                computedStyles[prop] = style.getPropertyValue(prop);
            }});

            return {{
                selector: '{escaped}',
                tag: el.tagName.toLowerCase(),
                classes: Array.from(el.classList),
                attributes: Object.fromEntries(Array.from(el.attributes).map(a => [a.name, a.value])),
                text_preview: (el.textContent || '').substring(0, 200).trim(),
                parent_tag: el.parentElement?.tagName?.toLowerCase() || '',
                sibling_count: el.parentElement?.children?.length || 0,
                child_count: el.children.length,
                depth: depth,
                position: [rect.x, rect.y, rect.width, rect.height],
                computed_styles: computedStyles
            }};
        }})()
    "#
    );

    let val = page
        .evaluate(js)
        .await
        .map_err(|e| Error::Cdp(e.to_string()))?;
    let v = val.into_value().unwrap_or(serde_json::json!(null));
    if v.is_null() {
        return Err(Error::NotFound(format!("Element not found: {selector}")));
    }
    serde_json::from_value(v).map_err(|e| Error::Cdp(e.to_string()))
}

/// Relocate an element after the page structure has changed.
/// Uses multiple strategies: exact match, structural similarity, content similarity, visual similarity.
pub async fn relocate_element(
    page: &Page,
    fingerprint: &ElementFingerprint,
) -> Result<Vec<ElementMatch>> {
    let fp_json = serde_json::to_string(fingerprint)?;

    let js = format!(
        r#"
        (() => {{
            const fp = {fp_json};
            const matches = [];

            // Strategy 1: Exact selector match
            const exactEl = document.querySelector(fp.selector);
            if (exactEl) {{
                matches.push({{
                    selector: fp.selector,
                    score: 100,
                    match_type: 'exact'
                }});
            }}

            // Strategy 2: Tag + class match
            const candidates = document.querySelectorAll(fp.tag);
            for (const el of candidates) {{
                let score = 0;
                const elClasses = Array.from(el.classList);

                // Class overlap
                const classOverlap = fp.classes.filter(c => elClasses.includes(c)).length;
                if (fp.classes.length > 0) {{
                    score += (classOverlap / fp.classes.length) * 30;
                }}

                // Attribute match
                let attrMatch = 0;
                let attrTotal = 0;
                for (const [key, val] of Object.entries(fp.attributes)) {{
                    if (key === 'class' || key === 'style') continue;
                    attrTotal++;
                    if (el.getAttribute(key) === val) attrMatch++;
                }}
                if (attrTotal > 0) score += (attrMatch / attrTotal) * 20;

                // Text similarity (Jaccard on words)
                const elText = (el.textContent || '').substring(0, 200).trim();
                if (fp.text_preview && elText) {{
                    const words1 = new Set(fp.text_preview.toLowerCase().split(/\s+/));
                    const words2 = new Set(elText.toLowerCase().split(/\s+/));
                    const intersection = [...words1].filter(w => words2.has(w)).length;
                    const union = new Set([...words1, ...words2]).size;
                    if (union > 0) score += (intersection / union) * 25;
                }}

                // Structural: same parent tag, similar depth
                if (el.parentElement?.tagName?.toLowerCase() === fp.parent_tag) score += 10;
                if (el.children.length === fp.child_count) score += 5;

                // Position similarity
                if (fp.position) {{
                    const rect = el.getBoundingClientRect();
                    const dx = Math.abs(rect.x - fp.position[0]);
                    const dy = Math.abs(rect.y - fp.position[1]);
                    const dw = Math.abs(rect.width - fp.position[2]);
                    const dh = Math.abs(rect.height - fp.position[3]);
                    const posDist = Math.sqrt(dx*dx + dy*dy + dw*dw + dh*dh);
                    if (posDist < 50) score += 10;
                    else if (posDist < 200) score += 5;
                }}

                if (score >= 30) {{
                    let sel = fp.tag;
                    if (el.id) sel = '#' + el.id;
                    else if (elClasses.length > 0) sel = fp.tag + '.' + elClasses.join('.');

                    matches.push({{
                        selector: sel,
                        score: Math.min(score, 99),
                        match_type: score >= 70 ? 'structural' : score >= 50 ? 'content' : 'visual'
                    }});
                }}
            }}

            // Sort by score descending, deduplicate
            matches.sort((a, b) => b.score - a.score);
            const seen = new Set();
            return matches.filter(m => {{
                if (seen.has(m.selector)) return false;
                seen.add(m.selector);
                return true;
            }}).slice(0, 10);
        }})()
    "#
    );

    let val = page
        .evaluate(js)
        .await
        .map_err(|e| Error::Cdp(e.to_string()))?;
    let matches: Vec<ElementMatch> =
        serde_json::from_value(val.into_value().unwrap_or(serde_json::json!([])))?;
    Ok(matches)
}

/// Save element fingerprints to file for persistence across sessions.
pub fn save_fingerprints(
    fingerprints: &[ElementFingerprint],
    path: &std::path::Path,
) -> Result<()> {
    let json = serde_json::to_string_pretty(fingerprints)?;
    std::fs::write(path, json)?;
    Ok(())
}

/// Load element fingerprints from file.
pub fn load_fingerprints(path: &std::path::Path) -> Result<Vec<ElementFingerprint>> {
    let json = std::fs::read_to_string(path)?;
    Ok(serde_json::from_str(&json)?)
}

/// Track multiple elements: fingerprint them and save for later relocation.
pub async fn track_elements(
    page: &Page,
    selectors: &[&str],
    save_path: Option<&std::path::Path>,
) -> Result<Vec<ElementFingerprint>> {
    let mut fingerprints = Vec::new();
    for selector in selectors {
        match fingerprint_element(page, selector).await {
            Ok(fp) => fingerprints.push(fp),
            Err(e) => eprintln!("Failed to fingerprint {}: {}", selector, e),
        }
    }
    if let Some(path) = save_path {
        save_fingerprints(&fingerprints, path)?;
    }
    Ok(fingerprints)
}

/// Relocate all tracked elements and return best matches.
pub async fn relocate_all(
    page: &Page,
    fingerprints: &[ElementFingerprint],
) -> Result<Vec<(String, Vec<ElementMatch>)>> {
    let mut results = Vec::new();
    for fp in fingerprints {
        let matches = relocate_element(page, fp).await?;
        results.push((fp.selector.clone(), matches));
    }
    Ok(results)
}
