use onecrawl_core::Result;
use scraper::{Html, Selector};
use serde::{Deserialize, Serialize};

/// Result of a CSS selector query.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ElementInfo {
    pub tag: String,
    pub text: String,
    pub attributes: Vec<(String, String)>,
    pub inner_html: String,
}

/// Query HTML document with a CSS selector and return matching elements.
pub fn query_selector(html: &str, css_selector: &str) -> Result<Vec<ElementInfo>> {
    let document = Html::parse_document(html);
    let selector = Selector::parse(css_selector)
        .map_err(|e| onecrawl_core::Error::Parser(format!("invalid selector: {e:?}")))?;

    let elements: Vec<ElementInfo> = document
        .select(&selector)
        .map(|el| ElementInfo {
            tag: el.value().name().to_string(),
            text: el.text().collect::<Vec<_>>().join(" ").trim().to_string(),
            attributes: el
                .value()
                .attrs()
                .map(|(k, v)| (k.to_string(), v.to_string()))
                .collect(),
            inner_html: el.inner_html(),
        })
        .collect();

    Ok(elements)
}

/// Count elements matching a CSS selector.
pub fn query_count(html: &str, css_selector: &str) -> Result<usize> {
    let document = Html::parse_document(html);
    let selector = Selector::parse(css_selector)
        .map_err(|e| onecrawl_core::Error::Parser(format!("invalid selector: {e:?}")))?;
    Ok(document.select(&selector).count())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn query_selector_basic() {
        let html = r#"<html><body><div class="test">Hello</div><div class="test">World</div></body></html>"#;
        let results = query_selector(html, ".test").unwrap();
        assert_eq!(results.len(), 2);
        assert_eq!(results[0].tag, "div");
        assert_eq!(results[0].text, "Hello");
    }

    #[test]
    fn query_count_basic() {
        let html = "<html><body><p>A</p><p>B</p><p>C</p></body></html>";
        assert_eq!(query_count(html, "p").unwrap(), 3);
    }

    #[test]
    fn query_attributes_extracted() {
        let html = r#"<html><body><a href="/page" title="Go">Link</a></body></html>"#;
        let results = query_selector(html, "a").unwrap();
        assert_eq!(results.len(), 1);
        let attrs: Vec<&str> = results[0]
            .attributes
            .iter()
            .map(|(k, _)| k.as_str())
            .collect();
        assert!(attrs.contains(&"href"));
        assert!(attrs.contains(&"title"));
    }
}
