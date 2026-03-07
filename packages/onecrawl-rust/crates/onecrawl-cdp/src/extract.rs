//! Content extraction — text, HTML, Markdown, JSON (Scrapling-like).

use chromiumoxide::Page;
use onecrawl_core::{Error, Result};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ExtractFormat {
    Text,
    Html,
    Markdown,
    Json,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtractResult {
    pub format: String,
    pub content: String,
    pub url: String,
    pub title: String,
    pub word_count: usize,
    pub links: Vec<LinkInfo>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LinkInfo {
    pub href: String,
    pub text: String,
    pub rel: String,
}

/// Extract page content in specified format.
pub async fn extract(
    page: &Page,
    selector: Option<&str>,
    format: ExtractFormat,
) -> Result<ExtractResult> {
    let sel = selector.unwrap_or("body");

    let js = format!(
        r#"
        (() => {{
            const root = document.querySelector('{}') || document.body;
            const url = window.location.href;
            const title = document.title;

            const links = Array.from(root.querySelectorAll('a[href]')).map(a => ({{
                href: a.href,
                text: a.textContent.trim(),
                rel: a.rel || ''
            }}));

            function extractText(el) {{
                const clone = el.cloneNode(true);
                clone.querySelectorAll('script, style, noscript').forEach(e => e.remove());
                return clone.textContent.replace(/\s+/g, ' ').trim();
            }}

            function extractMarkdown(el) {{
                const clone = el.cloneNode(true);
                clone.querySelectorAll('script, style, noscript').forEach(e => e.remove());

                let md = '';
                function walk(node, depth) {{
                    if (node.nodeType === Node.TEXT_NODE) {{
                        md += node.textContent;
                        return;
                    }}
                    if (node.nodeType !== Node.ELEMENT_NODE) return;

                    const tag = node.tagName?.toLowerCase();
                    switch(tag) {{
                        case 'h1': md += '\n# '; break;
                        case 'h2': md += '\n## '; break;
                        case 'h3': md += '\n### '; break;
                        case 'h4': md += '\n#### '; break;
                        case 'h5': md += '\n##### '; break;
                        case 'h6': md += '\n###### '; break;
                        case 'p': md += '\n\n'; break;
                        case 'br': md += '\n'; break;
                        case 'li': md += '\n- '; break;
                        case 'strong': case 'b': md += '**'; break;
                        case 'em': case 'i': md += '_'; break;
                        case 'code': md += '`'; break;
                        case 'pre': md += '\n```\n'; break;
                        case 'a': md += '['; break;
                        case 'img': md += '![' + (node.alt || '') + '](' + (node.src || '') + ')'; return;
                        case 'blockquote': md += '\n> '; break;
                        case 'hr': md += '\n---\n'; return;
                    }}

                    for (const child of node.childNodes) walk(child, depth + 1);

                    switch(tag) {{
                        case 'strong': case 'b': md += '**'; break;
                        case 'em': case 'i': md += '_'; break;
                        case 'code': md += '`'; break;
                        case 'pre': md += '\n```\n'; break;
                        case 'a': md += '](' + (node.href || '') + ')'; break;
                        case 'h1': case 'h2': case 'h3': case 'h4': case 'h5': case 'h6': md += '\n'; break;
                    }}
                }}
                walk(clone, 0);
                return md.replace(/\n{{3,}}/g, '\n\n').trim();
            }}

            function extractHtml(el) {{
                const clone = el.cloneNode(true);
                clone.querySelectorAll('script, style, noscript').forEach(e => e.remove());
                return clone.innerHTML;
            }}

            function extractJson(el) {{
                const data = {{
                    title: title,
                    url: url,
                    headings: Array.from(el.querySelectorAll('h1,h2,h3,h4,h5,h6')).map(h => ({{
                        level: parseInt(h.tagName[1]),
                        text: h.textContent.trim()
                    }})),
                    paragraphs: Array.from(el.querySelectorAll('p')).map(p => p.textContent.trim()).filter(Boolean),
                    links: links,
                    images: Array.from(el.querySelectorAll('img')).map(img => ({{
                        src: img.src,
                        alt: img.alt || '',
                        width: img.width,
                        height: img.height
                    }})),
                    tables: Array.from(el.querySelectorAll('table')).map(table => {{
                        const rows = Array.from(table.querySelectorAll('tr'));
                        return rows.map(row => Array.from(row.querySelectorAll('td,th')).map(cell => cell.textContent.trim()));
                    }}),
                    meta: Object.fromEntries(Array.from(document.querySelectorAll('meta[name], meta[property]')).map(m => [m.name || m.getAttribute('property'), m.content]))
                }};
                return JSON.stringify(data, null, 2);
            }}

            const text = extractText(root);
            const wordCount = text.split(/\s+/).filter(Boolean).length;

            return {{
                text: text,
                html: extractHtml(root),
                markdown: extractMarkdown(root),
                json: extractJson(root),
                url: url,
                title: title,
                word_count: wordCount,
                links: links
            }};
        }})()
    "#,
        sel.replace('\\', "\\\\").replace('\'', "\\'")
    );

    let val = page
        .evaluate(js)
        .await
        .map_err(|e| Error::Cdp(e.to_string()))?;
    let raw = val.into_value().unwrap_or(serde_json::json!({}));

    let format_str = match format {
        ExtractFormat::Text => "text",
        ExtractFormat::Html => "html",
        ExtractFormat::Markdown => "markdown",
        ExtractFormat::Json => "json",
    };

    let content = raw
        .get(format_str)
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();

    let links: Vec<LinkInfo> =
        serde_json::from_value(raw.get("links").cloned().unwrap_or(serde_json::json!([])))?;

    Ok(ExtractResult {
        format: format_str.to_string(),
        content,
        url: raw
            .get("url")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string(),
        title: raw
            .get("title")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string(),
        word_count: raw.get("word_count").and_then(|v| v.as_u64()).unwrap_or(0) as usize,
        links,
    })
}

/// Extract and save to file (like Scrapling's `scrapling extract` CLI).
pub async fn extract_to_file(
    page: &Page,
    selector: Option<&str>,
    output_path: &std::path::Path,
) -> Result<usize> {
    let extension = output_path
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("txt");

    let format = match extension {
        "md" | "markdown" => ExtractFormat::Markdown,
        "html" | "htm" => ExtractFormat::Html,
        "json" => ExtractFormat::Json,
        _ => ExtractFormat::Text,
    };

    let result = extract(page, selector, format).await?;
    let bytes = result.content.len();
    std::fs::write(output_path, &result.content)?;
    Ok(bytes)
}

/// Get structured page metadata.
pub async fn get_page_metadata(page: &Page) -> Result<serde_json::Value> {
    let js = r#"
        (() => ({
            title: document.title,
            url: window.location.href,
            description: document.querySelector('meta[name="description"]')?.content || '',
            keywords: document.querySelector('meta[name="keywords"]')?.content || '',
            author: document.querySelector('meta[name="author"]')?.content || '',
            canonical: document.querySelector('link[rel="canonical"]')?.href || '',
            ogTitle: document.querySelector('meta[property="og:title"]')?.content || '',
            ogDescription: document.querySelector('meta[property="og:description"]')?.content || '',
            ogImage: document.querySelector('meta[property="og:image"]')?.content || '',
            ogType: document.querySelector('meta[property="og:type"]')?.content || '',
            twitterCard: document.querySelector('meta[name="twitter:card"]')?.content || '',
            language: document.documentElement.lang || '',
            charset: document.characterSet || '',
            viewport: document.querySelector('meta[name="viewport"]')?.content || '',
            robots: document.querySelector('meta[name="robots"]')?.content || '',
            favicon: document.querySelector('link[rel="icon"], link[rel="shortcut icon"]')?.href || '',
            stylesheets: Array.from(document.querySelectorAll('link[rel="stylesheet"]')).map(l => l.href),
            scripts: Array.from(document.querySelectorAll('script[src]')).map(s => s.src),
            wordCount: document.body?.textContent?.split(/\s+/).filter(Boolean).length || 0,
            elementCount: document.querySelectorAll('*').length
        }))
    "#;
    let val = page
        .evaluate(js)
        .await
        .map_err(|e| Error::Cdp(e.to_string()))?;
    Ok(val.into_value().unwrap_or(serde_json::json!({})))
}

/// Parse a format string into ExtractFormat.
pub fn parse_extract_format(s: &str) -> Result<ExtractFormat> {
    match s.to_lowercase().as_str() {
        "text" | "txt" => Ok(ExtractFormat::Text),
        "html" | "htm" => Ok(ExtractFormat::Html),
        "markdown" | "md" => Ok(ExtractFormat::Markdown),
        "json" => Ok(ExtractFormat::Json),
        _ => Err(Error::Config(format!(
            "Unknown format: {s}. Use: text, html, markdown, json"
        ))),
    }
}
