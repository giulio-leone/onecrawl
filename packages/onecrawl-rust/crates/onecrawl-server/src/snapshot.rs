use serde::{Deserialize, Serialize};

/// A single element from an accessibility snapshot with a stable ref ID.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SnapshotElement {
    pub ref_id: String,
    pub role: String,
    pub name: String,
    pub tag: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub value: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    pub interactive: bool,
    pub backend_node_id: i64,
}

/// Full page snapshot with element refs.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PageSnapshot {
    pub elements: Vec<SnapshotElement>,
    pub url: String,
    pub title: String,
}

/// Query parameters for the snapshot endpoint.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SnapshotQuery {
    pub filter: Option<String>,
    pub format: Option<String>,
    pub max_tokens: Option<usize>,
    pub depth: Option<usize>,
}

/// JS injected into the page to build an accessibility snapshot with element refs.
pub const SNAPSHOT_JS: &str = r#"(() => {
    const INTERACTIVE_ROLES = new Set([
        'button', 'link', 'textbox', 'combobox', 'checkbox', 'radio',
        'slider', 'spinbutton', 'switch', 'tab', 'menuitem', 'option',
        'searchbox', 'listbox', 'menu', 'menubar', 'tree', 'treeitem',
        'grid', 'gridcell', 'tablist'
    ]);
    const INTERACTIVE_TAGS = new Set([
        'a', 'button', 'input', 'select', 'textarea', 'details', 'summary'
    ]);
    const elements = [];
    let idx = 0;
    const walker = document.createTreeWalker(
        document.body || document.documentElement,
        NodeFilter.SHOW_ELEMENT,
        null
    );
    let node = walker.currentNode;
    while (node) {
        const tag = node.tagName ? node.tagName.toLowerCase() : '';
        const role = node.getAttribute && (node.getAttribute('role') || '');
        const effectiveRole = role || tag;
        const isInteractive = INTERACTIVE_ROLES.has(effectiveRole) || INTERACTIVE_TAGS.has(tag);
        const name = (node.getAttribute && (
            node.getAttribute('aria-label') ||
            node.getAttribute('alt') ||
            node.getAttribute('title') ||
            node.getAttribute('placeholder')
        )) || (node.textContent || '').trim().substring(0, 80) || '';
        const value = node.value !== undefined && node.value !== '' ? String(node.value) : null;
        const description = node.getAttribute ? node.getAttribute('aria-description') : null;

        // Compute a pseudo backendNodeId from document order
        const backendNodeId = idx;

        elements.push({
            ref_id: 'e' + idx,
            role: effectiveRole,
            name: name,
            tag: tag,
            value: value,
            description: description,
            interactive: isInteractive,
            backend_node_id: backendNodeId
        });
        idx++;
        node = walker.nextNode();
    }
    return JSON.stringify({
        elements: elements,
        url: location.href,
        title: document.title || ''
    });
})()"#;

/// JS to extract clean text content (readability mode).
pub const TEXT_EXTRACT_JS: &str = r#"(() => {
    function isVisible(el) {
        const s = getComputedStyle(el);
        return s.display !== 'none' && s.visibility !== 'hidden' && s.opacity !== '0';
    }
    const SKIP = new Set(['SCRIPT','STYLE','NOSCRIPT','SVG','PATH','META','LINK','HEAD']);
    const blocks = [];
    const walk = document.createTreeWalker(
        document.body || document.documentElement,
        NodeFilter.SHOW_ELEMENT,
        { acceptNode: n => SKIP.has(n.tagName) ? NodeFilter.FILTER_REJECT : NodeFilter.FILTER_ACCEPT }
    );
    let node = walk.currentNode;
    while (node) {
        if (node.tagName && isVisible(node)) {
            const text = node.innerText || node.textContent || '';
            const trimmed = text.trim();
            if (trimmed.length > 0) {
                const tag = node.tagName.toLowerCase();
                if (['h1','h2','h3','h4','h5','h6','p','li','td','th','blockquote','pre','figcaption'].includes(tag)) {
                    blocks.push(trimmed);
                }
            }
        }
        node = walk.nextNode();
    }
    // Deduplicate consecutive identical blocks
    const deduped = [];
    for (const b of blocks) {
        if (deduped.length === 0 || deduped[deduped.length-1] !== b) {
            deduped.push(b);
        }
    }
    return deduped.join('\n\n');
})()"#;

/// JS to click an element by its document-order index (backendNodeId from our snapshot).
pub fn click_by_index_js(index: i64) -> String {
    format!(
        r#"(() => {{
    const walker = document.createTreeWalker(
        document.body || document.documentElement,
        NodeFilter.SHOW_ELEMENT, null
    );
    let node = walker.currentNode;
    let i = 0;
    while (node) {{
        if (i === {index}) {{
            node.scrollIntoView({{ block: 'center' }});
            node.focus();
            node.click();
            return 'clicked';
        }}
        i++;
        node = walker.nextNode();
    }}
    throw new Error('element index {index} not found');
}})()"#,
        index = index
    )
}

/// JS to focus an element by index.
pub fn focus_by_index_js(index: i64) -> String {
    format!(
        r#"(() => {{
    const walker = document.createTreeWalker(
        document.body || document.documentElement,
        NodeFilter.SHOW_ELEMENT, null
    );
    let node = walker.currentNode;
    let i = 0;
    while (node) {{
        if (i === {index}) {{
            node.scrollIntoView({{ block: 'center' }});
            node.focus();
            return 'focused';
        }}
        i++;
        node = walker.nextNode();
    }}
    throw new Error('element index {index} not found');
}})()"#,
        index = index
    )
}

/// JS to fill (clear + set value) an element by index.
pub fn fill_by_index_js(index: i64, text: &str) -> String {
    let escaped = text.replace('\\', "\\\\").replace('\'', "\\'").replace('\n', "\\n");
    format!(
        r#"(() => {{
    const walker = document.createTreeWalker(
        document.body || document.documentElement,
        NodeFilter.SHOW_ELEMENT, null
    );
    let node = walker.currentNode;
    let i = 0;
    while (node) {{
        if (i === {index}) {{
            node.scrollIntoView({{ block: 'center' }});
            node.focus();
            node.value = '';
            const nativeInputValueSetter = Object.getOwnPropertyDescriptor(
                window.HTMLInputElement.prototype, 'value'
            )?.set || Object.getOwnPropertyDescriptor(
                window.HTMLTextAreaElement.prototype, 'value'
            )?.set;
            if (nativeInputValueSetter) {{
                nativeInputValueSetter.call(node, '{text}');
            }} else {{
                node.value = '{text}';
            }}
            node.dispatchEvent(new Event('input', {{ bubbles: true }}));
            node.dispatchEvent(new Event('change', {{ bubbles: true }}));
            return 'filled';
        }}
        i++;
        node = walker.nextNode();
    }}
    throw new Error('element index {index} not found');
}})()"#,
        index = index,
        text = escaped
    )
}

/// JS to type text character-by-character into an element by index.
pub fn type_by_index_js(index: i64, text: &str) -> String {
    let escaped = text.replace('\\', "\\\\").replace('\'', "\\'").replace('\n', "\\n");
    format!(
        r#"(() => {{
    const walker = document.createTreeWalker(
        document.body || document.documentElement,
        NodeFilter.SHOW_ELEMENT, null
    );
    let node = walker.currentNode;
    let i = 0;
    while (node) {{
        if (i === {index}) {{
            node.scrollIntoView({{ block: 'center' }});
            node.focus();
            const text = '{text}';
            for (const ch of text) {{
                node.dispatchEvent(new KeyboardEvent('keydown', {{ key: ch, bubbles: true }}));
                node.dispatchEvent(new KeyboardEvent('keypress', {{ key: ch, bubbles: true }}));
                node.value = (node.value || '') + ch;
                node.dispatchEvent(new Event('input', {{ bubbles: true }}));
                node.dispatchEvent(new KeyboardEvent('keyup', {{ key: ch, bubbles: true }}));
            }}
            node.dispatchEvent(new Event('change', {{ bubbles: true }}));
            return 'typed';
        }}
        i++;
        node = walker.nextNode();
    }}
    throw new Error('element index {index} not found');
}})()"#,
        index = index,
        text = escaped
    )
}

/// JS to hover an element by index.
pub fn hover_by_index_js(index: i64) -> String {
    format!(
        r#"(() => {{
    const walker = document.createTreeWalker(
        document.body || document.documentElement,
        NodeFilter.SHOW_ELEMENT, null
    );
    let node = walker.currentNode;
    let i = 0;
    while (node) {{
        if (i === {index}) {{
            node.scrollIntoView({{ block: 'center' }});
            const rect = node.getBoundingClientRect();
            node.dispatchEvent(new MouseEvent('mouseover', {{
                bubbles: true, clientX: rect.x + rect.width/2, clientY: rect.y + rect.height/2
            }}));
            node.dispatchEvent(new MouseEvent('mouseenter', {{
                bubbles: false, clientX: rect.x + rect.width/2, clientY: rect.y + rect.height/2
            }}));
            return 'hovered';
        }}
        i++;
        node = walker.nextNode();
    }}
    throw new Error('element index {index} not found');
}})()"#,
        index = index
    )
}

/// JS to select an option value on a `<select>` element by index.
pub fn select_by_index_js(index: i64, value: &str) -> String {
    let escaped = value.replace('\\', "\\\\").replace('\'', "\\'");
    format!(
        r#"(() => {{
    const walker = document.createTreeWalker(
        document.body || document.documentElement,
        NodeFilter.SHOW_ELEMENT, null
    );
    let node = walker.currentNode;
    let i = 0;
    while (node) {{
        if (i === {index}) {{
            node.value = '{value}';
            node.dispatchEvent(new Event('input', {{ bubbles: true }}));
            node.dispatchEvent(new Event('change', {{ bubbles: true }}));
            return 'selected';
        }}
        i++;
        node = walker.nextNode();
    }}
    throw new Error('element index {index} not found');
}})()"#,
        index = index,
        value = escaped
    )
}
