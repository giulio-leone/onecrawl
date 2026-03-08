use onecrawl_core::Result;
use serde::{Deserialize, Serialize};

/// A node in the accessibility tree.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct A11yNode {
    pub role: String,
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub value: Option<String>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub children: Vec<A11yNode>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ref_id: Option<String>,
}

/// Build a simplified accessibility tree from HTML.
///
/// Maps HTML elements to ARIA roles:
/// - headings → heading with level
/// - links → link
/// - buttons → button
/// - inputs → textbox/checkbox/radio
/// - images → img
/// - nav/main/footer/header → navigation/main/contentinfo/banner
pub fn get_accessibility_tree(html: &str) -> Result<A11yNode> {
    let document = scraper::Html::parse_document(html);
    let mut counter = 0u32;
    let root = build_tree(document.root_element(), &mut counter, true);
    Ok(root)
}

fn build_tree(element: scraper::ElementRef, counter: &mut u32, is_root: bool) -> A11yNode {
    let tag = element.value().name();
    let (role, name) = infer_role_and_name(element, tag);

    *counter += 1;
    let ref_id = format!("e{counter}");

    let children: Vec<A11yNode> = element
        .children()
        .filter_map(|child| {
            child
                .value()
                .as_element()
                .map(|_| scraper::ElementRef::wrap(child).unwrap())
        })
        .map(|child| build_tree(child, counter, false))
        .collect();

    A11yNode {
        role,
        name,
        value: element.value().attr("value").map(|s| s.to_string()),
        children,
        ref_id: if is_root { None } else { Some(ref_id) },
    }
}

fn infer_role_and_name(el: scraper::ElementRef, tag: &str) -> (String, String) {
    let explicit_role = el.value().attr("role").unwrap_or("");
    let aria_label = el.value().attr("aria-label").unwrap_or("");
    let alt = el.value().attr("alt").unwrap_or("");
    let title = el.value().attr("title").unwrap_or("");

    let role = if !explicit_role.is_empty() {
        explicit_role.to_string()
    } else {
        let role_str = match tag {
            "h1" | "h2" | "h3" | "h4" | "h5" | "h6" => "heading",
            "a" => "link",
            "button" => "button",
            "input" => {
                let input_type = el.value().attr("type").unwrap_or("text");
                match input_type {
                    "checkbox" => "checkbox",
                    "radio" => "radio",
                    "submit" | "button" => "button",
                    _ => "textbox",
                }
            }
            "textarea" => "textbox",
            "select" => "combobox",
            "img" => "img",
            "nav" => "navigation",
            "main" => "main",
            "footer" => "contentinfo",
            "header" => "banner",
            "aside" => "complementary",
            "form" => "form",
            "table" => "table",
            "ul" | "ol" => "list",
            "li" => "listitem",
            _ => "generic",
        };
        role_str.to_string()
    };

    let name = if !aria_label.is_empty() {
        aria_label.to_string()
    } else if !alt.is_empty() {
        alt.to_string()
    } else if !title.is_empty() {
        title.to_string()
    } else {
        // Use direct text content for interactive elements
        let text: String = el.text().collect::<Vec<_>>().join(" ").trim().to_string();
        if text.len() > 100 {
            let truncated: String = text.chars().take(97).collect();
            format!("{truncated}…")
        } else {
            text
        }
    };

    (role, name)
}

/// Render the accessibility tree as a compact text representation.
pub fn render_tree(node: &A11yNode, depth: usize, interactive_only: bool) -> String {
    let mut output = String::new();
    let is_interactive = matches!(
        node.role.as_str(),
        "link" | "button" | "textbox" | "checkbox" | "radio" | "combobox"
    );

    if !interactive_only || is_interactive || depth == 0 {
        let indent = "  ".repeat(depth);
        let ref_str = node
            .ref_id
            .as_ref()
            .map(|r| format!(" [ref={r}]"))
            .unwrap_or_default();
        let name_str = if node.name.is_empty() {
            String::new()
        } else {
            format!(" \"{}\"", node.name)
        };

        output.push_str(&format!("{indent}- {}{name_str}{ref_str}\n", node.role));
    }

    for child in &node.children {
        output.push_str(&render_tree(child, depth + 1, interactive_only));
    }

    output
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_simple_html() {
        let html = r#"<html><body><h1>Title</h1><a href="/link">Click me</a></body></html>"#;
        let tree = get_accessibility_tree(html).unwrap();
        assert_eq!(tree.role, "generic"); // html tag
        assert!(!tree.children.is_empty());
    }

    #[test]
    fn interactive_elements_have_refs() {
        let html = r#"<html><body><button>OK</button><input type="text" /></body></html>"#;
        let tree = get_accessibility_tree(html).unwrap();
        let rendered = render_tree(&tree, 0, true);
        assert!(rendered.contains("button"));
        assert!(rendered.contains("textbox"));
        assert!(rendered.contains("[ref="));
    }

    #[test]
    fn heading_role_detected() {
        let html = r#"<html><body><h2>Section</h2></body></html>"#;
        let tree = get_accessibility_tree(html).unwrap();
        let rendered = render_tree(&tree, 0, false);
        assert!(rendered.contains("heading"));
        assert!(rendered.contains("Section"));
    }

    #[test]
    fn aria_label_used_as_name() {
        let html = r#"<html><body><button aria-label="Close dialog">X</button></body></html>"#;
        let tree = get_accessibility_tree(html).unwrap();
        let rendered = render_tree(&tree, 0, false);
        assert!(rendered.contains("Close dialog"));
    }
}
