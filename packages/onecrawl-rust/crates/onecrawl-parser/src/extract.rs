use onecrawl_core::Result;
use scraper::{Html, Selector};

/// Extract text content from HTML matching a CSS selector.
pub fn extract_text(html: &str, css_selector: &str) -> Result<Vec<String>> {
    let document = Html::parse_document(html);
    let selector = Selector::parse(css_selector)
        .map_err(|e| onecrawl_core::Error::Parser(format!("invalid selector: {e:?}")))?;

    let texts: Vec<String> = document
        .select(&selector)
        .map(|el| {
            el.text().fold(String::new(), |mut acc, t| {
                if !acc.is_empty() { acc.push(' '); }
                acc.push_str(t);
                acc
            }).trim().to_string()
        })
        .filter(|t| !t.is_empty())
        .collect();

    Ok(texts)
}

/// Extract all links (href + text) from HTML.
pub fn extract_links(html: &str) -> Result<Vec<(String, String)>> {
    let document = Html::parse_document(html);
    let selector = Selector::parse("a[href]")
        .map_err(|e| onecrawl_core::Error::Parser(format!("selector error: {e:?}")))?;

    let links: Vec<(String, String)> = document
        .select(&selector)
        .filter_map(|el| {
            let href = el.value().attr("href")?.to_string();
            let text = el.text().fold(String::new(), |mut acc, t| {
                if !acc.is_empty() { acc.push(' '); }
                acc.push_str(t);
                acc
            }).trim().to_string();
            Some((href, text))
        })
        .collect();

    Ok(links)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extract_heading_text() {
        let html = "<html><body><h1>Hello</h1><h1>World</h1></body></html>";
        let texts = extract_text(html, "h1").unwrap();
        assert_eq!(texts, vec!["Hello", "World"]);
    }

    #[test]
    fn extract_links_from_html() {
        let html = r#"<html><body><a href="/a">Link A</a><a href="/b">Link B</a></body></html>"#;
        let links = extract_links(html).unwrap();
        assert_eq!(links.len(), 2);
        assert_eq!(links[0], ("/a".to_string(), "Link A".to_string()));
    }

    #[test]
    fn invalid_selector() {
        let result = extract_text("<html></html>", ":::");
        assert!(result.is_err());
    }
}
