//! E2E tests for the parser pipeline.
//! Tests accessibility tree, text extraction, link extraction, and CSS selectors.

use onecrawl_parser::{accessibility, extract, selector};

const COMPLEX_HTML: &str = r#"<!DOCTYPE html>
<html lang="en">
<head><title>E2E Test Page</title></head>
<body>
  <header>
    <nav aria-label="Main">
      <a href="/">Home</a>
      <a href="/about">About</a>
      <a href="https://external.com">External</a>
    </nav>
  </header>
  <main>
    <h1>Welcome</h1>
    <article>
      <h2>Article Title</h2>
      <p>First paragraph with <strong>bold</strong> text.</p>
      <p>Second paragraph with <a href="/link">internal link</a>.</p>
    </article>
    <form>
      <input type="text" placeholder="Name" aria-label="Your name">
      <button type="submit">Submit</button>
    </form>
  </main>
  <footer><p>&copy; 2026</p></footer>
</body>
</html>"#;

// ────────────────────── Full Page Analysis ──────────────────────

#[test]
fn e2e_full_page_analysis() {
    // 1. Build accessibility tree
    let tree = accessibility::get_accessibility_tree(COMPLEX_HTML).unwrap();
    let rendered = accessibility::render_tree(&tree, 0, false);
    assert!(
        rendered.contains("Welcome"),
        "tree should contain heading text"
    );
    assert!(
        rendered.contains("Article Title"),
        "tree should contain h2 text"
    );
    assert!(
        rendered.contains("Submit"),
        "tree should contain button text"
    );

    // 2. Extract heading text
    let headings = extract::extract_text(COMPLEX_HTML, "h1, h2").unwrap();
    assert!(headings.contains(&"Welcome".to_string()));
    assert!(headings.contains(&"Article Title".to_string()));

    // 3. Extract paragraph text
    let paragraphs = extract::extract_text(COMPLEX_HTML, "p").unwrap();
    assert!(paragraphs.iter().any(|p| p.contains("First paragraph")));
    assert!(paragraphs.iter().any(|p| p.contains("Second paragraph")));

    // 4. Extract links
    let links = extract::extract_links(COMPLEX_HTML).unwrap();
    assert!(links.len() >= 4, "should have at least 4 links");

    // Find external link
    let external = links.iter().find(|(href, _)| href.contains("external.com"));
    assert!(external.is_some(), "should have external link");

    // 5. Query specific elements with CSS selector
    let articles = selector::query_selector(COMPLEX_HTML, "article").unwrap();
    assert_eq!(articles.len(), 1);
    assert!(articles[0].text.contains("Article Title"));
}

// ────────────────────── Accessibility Tree ──────────────────────

#[test]
fn e2e_accessibility_tree_roles() {
    let tree = accessibility::get_accessibility_tree(COMPLEX_HTML).unwrap();
    let rendered = accessibility::render_tree(&tree, 0, false);

    // Landmark roles
    assert!(rendered.contains("navigation"), "should detect nav role");
    assert!(rendered.contains("main"), "should detect main role");
    assert!(
        rendered.contains("contentinfo"),
        "should detect footer role"
    );
    assert!(rendered.contains("banner"), "should detect header role");

    // Heading roles
    assert!(rendered.contains("heading"), "should detect heading role");

    // Form roles
    assert!(rendered.contains("form"), "should detect form role");
}

#[test]
fn e2e_accessibility_tree_interactive_elements() {
    let tree = accessibility::get_accessibility_tree(COMPLEX_HTML).unwrap();
    let interactive = accessibility::render_tree(&tree, 0, true);

    // Interactive-only mode should include links, buttons, inputs
    assert!(
        interactive.contains("link"),
        "should include links in interactive mode"
    );
    assert!(
        interactive.contains("button"),
        "should include buttons in interactive mode"
    );
    assert!(
        interactive.contains("textbox"),
        "should include textbox in interactive mode"
    );
    assert!(
        interactive.contains("[ref="),
        "interactive elements should have refs"
    );
}

#[test]
fn e2e_accessibility_tree_aria_labels() {
    let tree = accessibility::get_accessibility_tree(COMPLEX_HTML).unwrap();
    let rendered = accessibility::render_tree(&tree, 0, false);

    // aria-label should be used as the accessible name
    assert!(
        rendered.contains("Main"),
        "should use aria-label 'Main' from nav"
    );
    assert!(
        rendered.contains("Your name"),
        "should use aria-label 'Your name' from input"
    );
}

// ────────────────────── Text Extraction ──────────────────────

#[test]
fn e2e_extract_text_by_selector() {
    // Extract only from <strong> elements
    let bold = extract::extract_text(COMPLEX_HTML, "strong").unwrap();
    assert_eq!(bold, vec!["bold"]);

    // Extract from buttons
    let buttons = extract::extract_text(COMPLEX_HTML, "button").unwrap();
    assert_eq!(buttons, vec!["Submit"]);
}

#[test]
fn e2e_extract_text_body() {
    let body_text = extract::extract_text(COMPLEX_HTML, "body").unwrap();
    assert!(!body_text.is_empty());
    let joined = body_text.join(" ");
    assert!(joined.contains("Welcome"));
    assert!(joined.contains("First paragraph"));
}

#[test]
fn e2e_extract_text_invalid_selector_errors() {
    let result = extract::extract_text(COMPLEX_HTML, ":::");
    assert!(result.is_err());
}

// ────────────────────── Link Extraction ──────────────────────

#[test]
fn e2e_link_classification() {
    let links = extract::extract_links(COMPLEX_HTML).unwrap();

    // Internal links (relative paths)
    let internal: Vec<_> = links
        .iter()
        .filter(|(href, _)| !href.starts_with("http"))
        .collect();
    assert!(
        !internal.is_empty(),
        "should have internal (relative) links"
    );

    // External links (absolute URLs)
    let external: Vec<_> = links
        .iter()
        .filter(|(href, _)| href.starts_with("https://"))
        .collect();
    assert!(!external.is_empty(), "should have external links");

    // Verify specific links
    assert!(
        links
            .iter()
            .any(|(href, text)| href == "/" && text == "Home")
    );
    assert!(
        links
            .iter()
            .any(|(href, text)| href == "/about" && text == "About")
    );
    assert!(links.iter().any(|(href, _)| href == "https://external.com"));
}

#[test]
fn e2e_links_preserve_text() {
    let links = extract::extract_links(COMPLEX_HTML).unwrap();
    let internal_link = links.iter().find(|(href, _)| href == "/link");
    assert!(internal_link.is_some());
    assert_eq!(internal_link.unwrap().1, "internal link");
}

// ────────────────────── CSS Selector Queries ──────────────────────

#[test]
fn e2e_query_selector_elements() {
    // Query paragraphs
    let paragraphs = selector::query_selector(COMPLEX_HTML, "p").unwrap();
    assert!(paragraphs.len() >= 3, "should find at least 3 <p> elements");
    assert!(paragraphs.iter().all(|e| e.tag == "p"));

    // Query links
    let links = selector::query_selector(COMPLEX_HTML, "a").unwrap();
    assert!(links.len() >= 4);
    assert!(links.iter().all(|e| e.tag == "a"));
}

#[test]
fn e2e_query_selector_attributes() {
    let inputs = selector::query_selector(COMPLEX_HTML, "input[type='text']").unwrap();
    assert_eq!(inputs.len(), 1);
    assert_eq!(inputs[0].tag, "input");

    let attrs: Vec<(&str, &str)> = inputs[0]
        .attributes
        .iter()
        .map(|(k, v)| (k.as_str(), v.as_str()))
        .collect();
    assert!(attrs.contains(&("type", "text")));
    assert!(attrs.contains(&("placeholder", "Name")));
    assert!(attrs.contains(&("aria-label", "Your name")));
}

#[test]
fn e2e_query_count() {
    let count = selector::query_count(COMPLEX_HTML, "a").unwrap();
    assert!(count >= 4, "should have at least 4 <a> elements");

    let h1_count = selector::query_count(COMPLEX_HTML, "h1").unwrap();
    assert_eq!(h1_count, 1);

    let h2_count = selector::query_count(COMPLEX_HTML, "h2").unwrap();
    assert_eq!(h2_count, 1);
}

#[test]
fn e2e_query_selector_inner_html() {
    let articles = selector::query_selector(COMPLEX_HTML, "article").unwrap();
    assert_eq!(articles.len(), 1);
    // inner_html should contain child elements
    assert!(articles[0].inner_html.contains("<h2>"));
    assert!(articles[0].inner_html.contains("<p>"));
}

#[test]
fn e2e_query_selector_no_matches() {
    let results = selector::query_selector(COMPLEX_HTML, ".nonexistent-class").unwrap();
    assert!(results.is_empty());

    let count = selector::query_count(COMPLEX_HTML, ".nonexistent-class").unwrap();
    assert_eq!(count, 0);
}

// ────────────────────── Job Listing Parse Scenario ──────────────────────

const JOB_HTML: &str = r#"<!DOCTYPE html>
<html lang="en">
<head><title>Job Listing</title></head>
<body>
  <h1>Senior AI Engineer</h1>
  <div class="company">Scale AI</div>
  <div class="location">San Francisco, CA</div>
  <div class="description">
    <p>We are looking for an experienced AI engineer to join our team.</p>
    <p>Requirements: Python, Rust, ML systems</p>
  </div>
  <a href="https://scale.com/apply" class="apply-btn">Apply Now</a>
  <a href="/save" class="save-btn">Save Job</a>
</body>
</html>"#;

#[test]
fn e2e_job_listing_parse_pipeline() {
    // 1. Extract job title
    let title = extract::extract_text(JOB_HTML, "h1").unwrap();
    assert_eq!(title, vec!["Senior AI Engineer"]);

    // 2. Extract company & location
    let company = extract::extract_text(JOB_HTML, ".company").unwrap();
    assert_eq!(company, vec!["Scale AI"]);

    let location = extract::extract_text(JOB_HTML, ".location").unwrap();
    assert_eq!(location, vec!["San Francisco, CA"]);

    // 3. Extract description paragraphs
    let desc = extract::extract_text(JOB_HTML, ".description p").unwrap();
    assert_eq!(desc.len(), 2);
    assert!(desc[0].contains("experienced AI engineer"));
    assert!(desc[1].contains("Python, Rust"));

    // 4. Find apply button link
    let links = extract::extract_links(JOB_HTML).unwrap();
    let apply_link = links
        .iter()
        .find(|(href, _)| href.contains("scale.com"))
        .unwrap();
    assert_eq!(apply_link.1, "Apply Now");

    // 5. Accessibility tree should represent the full page
    let tree = accessibility::get_accessibility_tree(JOB_HTML).unwrap();
    let rendered = accessibility::render_tree(&tree, 0, false);
    assert!(rendered.contains("Senior AI Engineer"));
    assert!(rendered.contains("Apply Now"));
}
