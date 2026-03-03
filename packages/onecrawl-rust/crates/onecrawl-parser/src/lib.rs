//! OneCrawl Parser — HTML parsing, accessibility tree, element extraction.
//!
//! Uses `lol_html` for streaming HTML rewriting and `scraper` for CSS selector queries.

pub mod accessibility;
pub mod extract;
pub mod selector;

pub use accessibility::get_accessibility_tree;
pub use extract::extract_text;
pub use selector::query_selector;
