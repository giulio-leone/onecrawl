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
        .map_err(|e| Error::Cdp(format!("get_accessibility_tree failed: {e}")))?;

    match result.into_value::<serde_json::Value>() {
        Ok(v) => Ok(v),
        Err(_) => Ok(serde_json::json!({ "nodeCount": 0, "nodes": [] })),
    }
}

/// Get accessibility snapshot for a specific element.
pub async fn get_element_accessibility(page: &Page, selector: &str) -> Result<serde_json::Value> {
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
        sel = serde_json::to_string(selector).unwrap_or_else(|_| format!("\"{}\"", selector))
    );

    let result = page
        .evaluate(js)
        .await
        .map_err(|e| Error::Cdp(format!("get_element_accessibility failed: {e}")))?;

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
        .map_err(|e| Error::Cdp(format!("audit_accessibility failed: {e}")))?;

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

/// Options for `agent_snapshot()`.
#[derive(Debug, Clone, Default)]
pub struct AgentSnapshotOptions {
    /// Only include interactive elements (buttons, links, inputs).
    pub interactive_only: bool,
    /// Include cursor-interactive elements (cursor:pointer, onclick, tabindex)
    /// that are not already interactive by ARIA role or tag.
    pub cursor: bool,
    /// Remove empty structural elements for minimal output.
    pub compact: bool,
    /// Maximum DOM depth to include.
    pub depth: Option<usize>,
    /// CSS selector to scope snapshot to a subtree.
    pub selector: Option<String>,
}

/// Agent-mode snapshot: tags interactive elements with `data-onecrawl-ref` attributes,
/// returns a compact text snapshot + refs map for AI-driven automation.
///
/// After calling this, `@e1` resolves to `[data-onecrawl-ref="e1"]` in any selector.
pub async fn agent_snapshot(page: &Page, opts: &AgentSnapshotOptions) -> Result<AgentSnapshot> {
    let interactive_only_js = if opts.interactive_only { "true" } else { "false" };
    let cursor_js = if opts.cursor { "true" } else { "false" };
    let compact_js = if opts.compact { "true" } else { "false" };
    let depth_js = match opts.depth {
        Some(d) => d.to_string(),
        None => "null".to_string(),
    };
    let scope_js = match &opts.selector {
        Some(s) => serde_json::to_string(s).unwrap_or_else(|_| "null".to_string()),
        None => "null".to_string(),
    };

    let js = format!(
        r#"
        (() => {{
            const INTERACTIVE_TAGS = new Set(['a','button','input','textarea','select','label','details','summary']);
            const INTERACTIVE_ROLES = new Set(['button','link','checkbox','radio','textbox','combobox',
                'listbox','option','menuitem','menuitemcheckbox','menuitemradio','tab','treeitem',
                'slider','spinbutton','switch','searchbox','gridcell',
                'alertdialog','dialog','progressbar','tooltip','scrollbar','tree','menu']);
            const CONTENT_TAGS = new Set(['h1','h2','h3','h4','h5','h6','p','li','td','th','span','div','section','article','main','nav','header','footer']);
            const STRUCTURAL_ROLES = new Set(['generic','group','list','table','row','rowgroup',
                'columnheader','rowheader','cell','grid','treegrid','toolbar','separator',
                'presentation','none','directory','document','feed','figure','region',
                'banner','contentinfo','complementary','form','search']);

            const interactiveOnly = {interactive_only_js};
            const cursorMode = {cursor_js};
            const compactMode = {compact_js};
            const maxDepth = {depth_js};
            const scopeSelector = {scope_js};

            function isVisible(el) {{
                const r = el.getBoundingClientRect();
                if (r.width === 0 || r.height === 0) return false;
                const s = window.getComputedStyle(el);
                return s.display !== 'none' && s.visibility !== 'hidden' && s.opacity !== '0';
            }}

            function isInteractive(el) {{
                const tag = el.tagName.toLowerCase();
                const role = el.getAttribute('role') || '';
                return INTERACTIVE_TAGS.has(tag) || INTERACTIVE_ROLES.has(role)
                    || el.hasAttribute('onclick') || el.hasAttribute('tabindex')
                    || (el.getAttribute('tabindex') !== null && parseInt(el.getAttribute('tabindex')) >= 0);
            }}

            function isCursorInteractive(el) {{
                if (!cursorMode) return false;
                if (isInteractive(el)) return false;
                const s = window.getComputedStyle(el);
                const hasCursorPointer = s.cursor === 'pointer';
                const hasOnclick = el.hasAttribute('onclick');
                const hasTabindex = el.hasAttribute('tabindex') && parseInt(el.getAttribute('tabindex')) >= 0;
                if (!hasCursorPointer && !hasOnclick && !hasTabindex) return false;
                // Skip elements that inherit cursor:pointer from a parent
                if (hasCursorPointer && !hasOnclick && !hasTabindex) {{
                    const parent = el.parentElement;
                    if (parent) {{
                        const ps = window.getComputedStyle(parent);
                        if (ps.cursor === 'pointer') return false;
                    }}
                }}
                return true;
            }}

            function getCursorRole(el) {{
                const hasTabindex = el.hasAttribute('tabindex') && parseInt(el.getAttribute('tabindex')) >= 0;
                const s = window.getComputedStyle(el);
                if (el.hasAttribute('onclick') || s.cursor === 'pointer') return 'clickable';
                if (hasTabindex) return 'focusable';
                return 'clickable';
            }}

            function getName(el) {{
                return (el.getAttribute('aria-label')
                    || el.getAttribute('placeholder')
                    || el.getAttribute('alt')
                    || el.getAttribute('title')
                    || el.textContent?.replace(/\s+/g, ' ').trim().substring(0, 60)
                    || el.getAttribute('name')
                    || el.tagName.toLowerCase()).trim();
            }}

            function getRole(el) {{
                if (el.getAttribute('role')) return el.getAttribute('role');
                const tag = el.tagName.toLowerCase();
                const type = el.getAttribute('type') || '';
                if (tag === 'a') return 'link';
                if (tag === 'button') return 'button';
                if (tag === 'input') {{
                    if (type === 'checkbox') return 'checkbox';
                    if (type === 'radio') return 'radio';
                    if (type === 'submit' || type === 'button') return 'button';
                    return 'textbox';
                }}
                if (tag === 'textarea') return 'textbox';
                if (tag === 'select') return 'combobox';
                if (tag.match(/^h[1-6]$/)) return 'heading';
                if (tag === 'img') return 'img';
                if (tag === 'nav') return 'navigation';
                if (tag === 'main') return 'main';
                return tag;
            }}

            function getSelector(el, role, name) {{
                const tag = el.tagName.toLowerCase();
                const escapedName = name.replace(/"/g, '\\"');
                if (INTERACTIVE_ROLES.has(role) || ['button','link','checkbox','radio','textbox','combobox','heading','img','navigation','main'].includes(role)) {{
                    if (name && name !== tag) {{
                        return `getByRole('${{role}}', {{ name: "${{escapedName}}", exact: true }})`;
                    }}
                    return `getByRole('${{role}}')`;
                }}
                if (el.getAttribute('aria-label')) {{
                    return `getByLabel("${{escapedName}}", {{ exact: true }})`;
                }}
                if (el.getAttribute('placeholder')) {{
                    return `getByPlaceholder("${{el.getAttribute('placeholder').replace(/"/g, '\\"')}}", {{ exact: true }})`;
                }}
                if (name && name.length <= 40 && name !== tag) {{
                    return `getByText("${{escapedName}}", {{ exact: true }})`;
                }}
                return `locator('${{tag}}')`;
            }}

            function getExtra(el) {{
                const tag = el.tagName.toLowerCase();
                if (tag === 'input') {{
                    const t = el.getAttribute('type') || 'text';
                    const v = el.value || '';
                    return v ? ` value="${{v.substring(0,30)}}"` : ` type="${{t}}"`;
                }}
                if (tag === 'select') {{
                    const sel = el.options[el.selectedIndex];
                    return sel ? ` selected="${{sel.text.substring(0,30)}}"` : '';
                }}
                if (tag === 'a') {{
                    const h = el.getAttribute('href') || '';
                    return h ? ` href="${{h.substring(0,50)}}"` : '';
                }}
                return '';
            }}

            function getDepth(el, root) {{
                if (depthCache.has(el)) return depthCache.get(el);
                let depth = 0;
                let node = el;
                while (node && node !== root) {{
                    depth++;
                    node = node.parentElement;
                }}
                depthCache.set(el, depth);
                return depth;
            }}

            // Clear previous refs
            document.querySelectorAll('[data-onecrawl-ref]').forEach(el => {{
                el.removeAttribute('data-onecrawl-ref');
            }});

            const root = scopeSelector ? document.querySelector(scopeSelector) : document.body;
            if (!root) return {{ snapshot: '', refs: {{}}, total: 0 }};

            const depthCache = new WeakMap();

            const allEls = Array.from(root.querySelectorAll('*'));
            const refs = {{}};
            let counter = 0;
            let interactiveCount = 0;
            const lines = [];

            allEls.forEach(el => {{
                if (!isVisible(el)) return;

                if (maxDepth !== null && getDepth(el, root) > maxDepth) return;

                const isInt = isInteractive(el);
                const isCursorInt = isCursorInteractive(el);
                const tag = el.tagName.toLowerCase();
                const isContent = CONTENT_TAGS.has(tag) && el.textContent?.trim().length > 2;

                if (interactiveOnly && !isInt && !isCursorInt) return;
                if (!isInt && !isCursorInt && !isContent) return;
                // Skip if children already cover this (avoid redundant wrappers)
                if (!isInt && !isCursorInt && el.querySelector('button,a,input,textarea,select')) return;

                const role = isCursorInt ? getCursorRole(el) : getRole(el);
                const name = getName(el);

                // Compact mode: skip structural elements with no name and no ref'd children
                if (compactMode && !isInt && !isCursorInt) {{
                    const mappedRole = getRole(el);
                    if (STRUCTURAL_ROLES.has(mappedRole) && (!name || name === tag)) {{
                        return;
                    }}
                }}

                counter++;
                const refId = 'e' + counter;
                el.setAttribute('data-onecrawl-ref', refId);
                const selector = getSelector(el, role, name);
                const extra = getExtra(el);
                refs[refId] = {{ role, name, tag, selector }};
                if (isInt || isCursorInt) interactiveCount++;
                lines.push(`[${{refId}}] ${{role}} "${{name}}"${{extra}}`);
            }});

            return {{ snapshot: lines.join('\n'), refs, total: counter, interactiveCount }};
        }})()
        "#,
        interactive_only_js = interactive_only_js,
        cursor_js = cursor_js,
        compact_js = compact_js,
        depth_js = depth_js,
        scope_js = scope_js
    );

    let result = page
        .evaluate(js)
        .await
        .map_err(|e| Error::Cdp(format!("agent_snapshot failed: {e}")))?;

    match result.into_value::<AgentSnapshotRaw>() {
        Ok(raw) => Ok(AgentSnapshot {
            snapshot: raw.snapshot,
            refs: raw.refs,
            total: raw.total,
            interactive_count: raw.interactive_count.unwrap_or(0),
        }),
        Err(e) => Err(Error::Cdp(format!("agent_snapshot parse error: {e}"))),
    }
}

/// Internal deserialization target for JS result (camelCase from JS).
#[derive(Debug, Deserialize)]
struct AgentSnapshotRaw {
    snapshot: String,
    refs: std::collections::HashMap<String, AgentRef>,
    total: usize,
    #[serde(rename = "interactiveCount")]
    interactive_count: Option<usize>,
}

/// Result of an agent-mode snapshot.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentSnapshot {
    /// Compact text representation: `[e1] button "Submit"\n[e2] link "Home"...`
    pub snapshot: String,
    /// Map of ref_id → node info. `@e1` resolves to `[data-onecrawl-ref="e1"]`.
    pub refs: std::collections::HashMap<String, AgentRef>,
    /// Total number of elements tagged.
    pub total: usize,
    /// Number of interactive (actionable) elements in the snapshot.
    #[serde(default)]
    pub interactive_count: usize,
}

/// Metadata for a single agent ref.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentRef {
    pub role: String,
    pub name: String,
    pub tag: String,
    /// Playwright-style selector, e.g. `getByRole('button', { name: "Submit", exact: true })`.
    #[serde(default)]
    pub selector: String,
}

/// Statistics for an agent snapshot.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SnapshotStats {
    pub lines: usize,
    pub chars: usize,
    /// Approximate token count (chars / 4).
    pub estimated_tokens: usize,
    pub total_refs: usize,
    pub interactive_refs: usize,
}

impl AgentSnapshot {
    /// Compute statistics for this snapshot.
    pub fn stats(&self) -> SnapshotStats {
        let chars = self.snapshot.len();
        SnapshotStats {
            lines: if self.snapshot.is_empty() {
                0
            } else {
                self.snapshot.lines().count()
            },
            chars,
            estimated_tokens: chars / 4,
            total_refs: self.total,
            interactive_refs: self.interactive_count,
        }
    }
}

/// Resolve an agent ref selector (`@e1`) to a CSS selector.
///
/// - `@e1` → `[data-onecrawl-ref="e1"]`
/// - anything else → pass-through unchanged
pub fn resolve_ref(selector: &str) -> String {
    if let Some(ref_id) = selector.strip_prefix('@') {
        format!(r#"[data-onecrawl-ref="{}"]"#, ref_id)
    } else {
        selector.to_string()
    }
}
