//! Accessibility auditing via CDP and JS injection.
//!
//! Provides accessibility tree retrieval and WCAG-style audits.

use chromiumoxide::Page;
use onecrawl_core::{Error, Result};
use serde::{Deserialize, Serialize};

/// Structured accessibility audit report.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AccessibilityAudit {
    pub issues: Vec<A11yIssue>,
    pub summary: A11ySummary,
}

/// A single accessibility issue found during audit.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct A11yIssue {
    pub rule: String,
    pub severity: String,
    pub element: String,
    pub message: String,
}

/// Summary statistics for an accessibility audit.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct A11ySummary {
    pub total_issues: usize,
    pub errors: usize,
    pub warnings: usize,
    pub passed_checks: usize,
}

/// Get the full accessibility tree as JSON.
///
/// Uses JS to walk the DOM and collect ARIA attributes since
/// `GetFullAXTreeParams` is not available in chromiumoxide 0.8.
pub async fn get_accessibility_tree(page: &Page) -> Result<serde_json::Value> {
    let js = r#"
        (() => {
            function getAxNode(el) {
                const role = el.getAttribute('role') || el.tagName.toLowerCase();
                const name = el.getAttribute('aria-label')
                    || el.getAttribute('alt')
                    || el.getAttribute('title')
                    || el.textContent?.trim().substring(0, 80)
                    || '';
                const node = {
                    tag: el.tagName.toLowerCase(),
                    role: role,
                    name: name,
                    ariaAttributes: {}
                };
                for (const attr of el.attributes) {
                    if (attr.name.startsWith('aria-')) {
                        node.ariaAttributes[attr.name] = attr.value;
                    }
                }
                if (el.id) node.id = el.id;
                if (el.className && typeof el.className === 'string') node.className = el.className;
                return node;
            }
            const nodes = [];
            document.querySelectorAll('*').forEach(el => {
                nodes.push(getAxNode(el));
            });
            return { nodeCount: nodes.length, nodes: nodes.slice(0, 5000) };
        })()
    "#;

    let result = page
        .evaluate(js)
        .await
        .map_err(|e| Error::Browser(format!("get_accessibility_tree failed: {e}")))?;

    match result.into_value::<serde_json::Value>() {
        Ok(v) => Ok(v),
        Err(_) => Ok(serde_json::json!({ "nodeCount": 0, "nodes": [] })),
    }
}

/// Get accessibility snapshot for a specific element.
pub async fn get_element_accessibility(
    page: &Page,
    selector: &str,
) -> Result<serde_json::Value> {
    let js = format!(
        r#"
        (() => {{
            const el = document.querySelector({sel});
            if (!el) return {{ error: "Element not found", selector: {sel} }};
            const role = el.getAttribute('role') || el.tagName.toLowerCase();
            const name = el.getAttribute('aria-label')
                || el.getAttribute('alt')
                || el.getAttribute('title')
                || el.textContent?.trim().substring(0, 80)
                || '';
            const ariaAttrs = {{}};
            for (const attr of el.attributes) {{
                if (attr.name.startsWith('aria-')) {{
                    ariaAttrs[attr.name] = attr.value;
                }}
            }}
            const rect = el.getBoundingClientRect();
            return {{
                tag: el.tagName.toLowerCase(),
                role: role,
                name: name,
                ariaAttributes: ariaAttrs,
                id: el.id || null,
                className: (typeof el.className === 'string') ? el.className : null,
                bounds: {{ x: rect.x, y: rect.y, width: rect.width, height: rect.height }},
                visible: rect.width > 0 && rect.height > 0,
                focusable: el.tabIndex >= 0
            }};
        }})()
        "#,
        sel = serde_json::to_string(selector)
            .unwrap_or_else(|_| format!("\"{}\"", selector))
    );

    let result = page
        .evaluate(js)
        .await
        .map_err(|e| Error::Browser(format!("get_element_accessibility failed: {e}")))?;

    match result.into_value::<serde_json::Value>() {
        Ok(v) => Ok(v),
        Err(_) => Ok(serde_json::json!({ "error": "Failed to parse result" })),
    }
}

/// Run WCAG-style audit: checks for missing alt text, missing labels,
/// low contrast indicators, heading hierarchy issues, and more.
pub async fn audit_accessibility(page: &Page) -> Result<AccessibilityAudit> {
    let js = r#"
        (() => {
            const issues = [];
            let passedChecks = 0;

            // 1. Images without alt text
            const images = document.querySelectorAll('img');
            let imgIssues = 0;
            images.forEach(img => {
                if (!img.hasAttribute('alt')) {
                    issues.push({
                        rule: 'img-alt',
                        severity: 'error',
                        element: img.outerHTML.substring(0, 200),
                        message: 'Image is missing alt attribute'
                    });
                    imgIssues++;
                } else if (img.alt.trim() === '') {
                    issues.push({
                        rule: 'img-alt-empty',
                        severity: 'warning',
                        element: img.outerHTML.substring(0, 200),
                        message: 'Image has empty alt attribute (decorative?)'
                    });
                }
            });
            if (images.length > 0 && imgIssues === 0) passedChecks++;

            // 2. Inputs without labels
            const inputs = document.querySelectorAll('input, textarea, select');
            let inputIssues = 0;
            inputs.forEach(input => {
                if (input.type === 'hidden' || input.type === 'submit' || input.type === 'button') return;
                const hasLabel = input.id && document.querySelector('label[for="' + input.id + '"]');
                const hasAriaLabel = input.getAttribute('aria-label') || input.getAttribute('aria-labelledby');
                const hasTitle = input.getAttribute('title');
                const wrappedInLabel = input.closest('label');
                if (!hasLabel && !hasAriaLabel && !hasTitle && !wrappedInLabel) {
                    issues.push({
                        rule: 'input-label',
                        severity: 'error',
                        element: input.outerHTML.substring(0, 200),
                        message: 'Form input is missing an associated label'
                    });
                    inputIssues++;
                }
            });
            if (inputs.length > 0 && inputIssues === 0) passedChecks++;

            // 3. Buttons without accessible name
            const buttons = document.querySelectorAll('button, [role="button"]');
            let btnIssues = 0;
            buttons.forEach(btn => {
                const text = btn.textContent?.trim();
                const ariaLabel = btn.getAttribute('aria-label') || btn.getAttribute('aria-labelledby');
                const title = btn.getAttribute('title');
                if (!text && !ariaLabel && !title) {
                    issues.push({
                        rule: 'button-name',
                        severity: 'error',
                        element: btn.outerHTML.substring(0, 200),
                        message: 'Button has no accessible name'
                    });
                    btnIssues++;
                }
            });
            if (buttons.length > 0 && btnIssues === 0) passedChecks++;

            // 4. Links without text
            const links = document.querySelectorAll('a[href]');
            let linkIssues = 0;
            links.forEach(link => {
                const text = link.textContent?.trim();
                const ariaLabel = link.getAttribute('aria-label') || link.getAttribute('aria-labelledby');
                const title = link.getAttribute('title');
                const hasImg = link.querySelector('img[alt]');
                if (!text && !ariaLabel && !title && !hasImg) {
                    issues.push({
                        rule: 'link-name',
                        severity: 'error',
                        element: link.outerHTML.substring(0, 200),
                        message: 'Link has no discernible text'
                    });
                    linkIssues++;
                }
            });
            if (links.length > 0 && linkIssues === 0) passedChecks++;

            // 5. Heading hierarchy
            const headings = document.querySelectorAll('h1, h2, h3, h4, h5, h6');
            let lastLevel = 0;
            let headingIssues = 0;
            headings.forEach(h => {
                const level = parseInt(h.tagName.substring(1));
                if (lastLevel > 0 && level > lastLevel + 1) {
                    issues.push({
                        rule: 'heading-order',
                        severity: 'warning',
                        element: '<' + h.tagName.toLowerCase() + '>' + h.textContent?.trim().substring(0, 80),
                        message: 'Heading level skipped from h' + lastLevel + ' to h' + level
                    });
                    headingIssues++;
                }
                lastLevel = level;
            });
            if (headings.length > 0 && headingIssues === 0) passedChecks++;

            // 6. Color contrast indicators (heuristic check via computed styles)
            const textElements = document.querySelectorAll('p, span, a, li, td, th, label, h1, h2, h3, h4, h5, h6');
            let contrastIssues = 0;
            const checked = Math.min(textElements.length, 200);
            for (let i = 0; i < checked; i++) {
                const el = textElements[i];
                const style = getComputedStyle(el);
                const color = style.color;
                const bg = style.backgroundColor;
                if (color && bg && color === bg && el.textContent?.trim()) {
                    issues.push({
                        rule: 'color-contrast',
                        severity: 'error',
                        element: '<' + el.tagName.toLowerCase() + '>' + el.textContent?.trim().substring(0, 60),
                        message: 'Text color is identical to background color'
                    });
                    contrastIssues++;
                }
            }
            if (checked > 0 && contrastIssues === 0) passedChecks++;

            // 7. Document language
            const htmlLang = document.documentElement.getAttribute('lang');
            if (!htmlLang) {
                issues.push({
                    rule: 'html-lang',
                    severity: 'warning',
                    element: '<html>',
                    message: 'Document is missing lang attribute'
                });
            } else {
                passedChecks++;
            }

            const errors = issues.filter(i => i.severity === 'error').length;
            const warnings = issues.filter(i => i.severity === 'warning').length;
            return {
                issues: issues,
                summary: {
                    total_issues: issues.length,
                    errors: errors,
                    warnings: warnings,
                    passed_checks: passedChecks
                }
            };
        })()
    "#;

    let result = page
        .evaluate(js)
        .await
        .map_err(|e| Error::Browser(format!("audit_accessibility failed: {e}")))?;

    match result.into_value::<AccessibilityAudit>() {
        Ok(audit) => Ok(audit),
        Err(_) => Ok(AccessibilityAudit {
            issues: Vec::new(),
            summary: A11ySummary {
                total_issues: 0,
                errors: 0,
                warnings: 0,
                passed_checks: 0,
            },
        }),
    }
}
