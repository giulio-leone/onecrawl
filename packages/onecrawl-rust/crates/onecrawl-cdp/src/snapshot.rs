//! Page snapshot and diff for DOM change detection.
//!
//! Captures structured snapshots of page state and computes
//! diffs between snapshots using Jaccard word similarity.

use chromiumoxide::Page;
use onecrawl_core::{Error, Result};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::path::Path;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DomSnapshot {
    pub url: String,
    pub title: String,
    pub timestamp: f64,
    pub html: String,
    pub text: String,
    pub links: Vec<String>,
    pub images: Vec<String>,
    pub meta: HashMap<String, String>,
    pub element_count: usize,
    pub word_count: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SnapshotDiff {
    pub url: String,
    pub timestamp_before: f64,
    pub timestamp_after: f64,
    pub title_changed: bool,
    pub html_changed: bool,
    pub text_changed: bool,
    pub links_added: Vec<String>,
    pub links_removed: Vec<String>,
    pub images_added: Vec<String>,
    pub images_removed: Vec<String>,
    pub meta_changes: Vec<(String, String, String)>,
    pub element_count_delta: i64,
    pub word_count_delta: i64,
    pub similarity: f64,
}

/// Take a structured snapshot of the current page state.
pub async fn take_snapshot(page: &Page) -> Result<DomSnapshot> {
    let js = r#"(() => {
const metas = {};
document.querySelectorAll('meta[name],meta[property]').forEach(m => {
    const key = m.getAttribute('name') || m.getAttribute('property') || '';
    const val = m.getAttribute('content') || '';
    if (key) metas[key] = val;
});
const links = [];
document.querySelectorAll('a[href]').forEach(a => {
    const h = a.href;
    if (h && !links.includes(h)) links.push(h);
});
const images = [];
document.querySelectorAll('img[src]').forEach(img => {
    const s = img.src;
    if (s && !images.includes(s)) images.push(s);
});
const text = document.body ? document.body.innerText || '' : '';
return JSON.stringify({
    url: location.href,
    title: document.title || '',
    timestamp: Date.now() / 1000.0,
    html: document.documentElement.outerHTML,
    text: text,
    links: links,
    images: images,
    meta: metas,
    element_count: document.querySelectorAll('*').length,
    word_count: text.split(/\s+/).filter(w => w.length > 0).length,
});
})()"#;

    let val = page
        .evaluate(js)
        .await
        .map_err(|e| Error::Cdp(format!("take_snapshot failed: {e}")))?;

    let json_str: String =
        serde_json::from_value(val.into_value().unwrap_or(serde_json::json!("")))
            .unwrap_or_default();

    serde_json::from_str(&json_str).map_err(|e| Error::Cdp(format!("parse snapshot: {e}")))
}

/// Compute Jaccard word similarity between two text strings.
fn jaccard_similarity(a: &str, b: &str) -> f64 {
    let words_a: Vec<&str> = a.split_whitespace().collect();
    let words_b: Vec<&str> = b.split_whitespace().collect();
    if words_a.is_empty() && words_b.is_empty() {
        return 1.0;
    }
    let set_a: HashSet<&str> = HashSet::from_iter(words_a);
    let set_b: HashSet<&str> = HashSet::from_iter(words_b);
    let intersection = set_a.intersection(&set_b).count();
    let union = set_a.union(&set_b).count();
    if union == 0 {
        1.0
    } else {
        intersection as f64 / union as f64
    }
}

/// Compare two snapshots and return the diff.
pub fn compare_snapshots(before: &DomSnapshot, after: &DomSnapshot) -> SnapshotDiff {
    let before_links: HashSet<&str> =
        HashSet::from_iter(before.links.iter().map(String::as_str));
    let after_links: HashSet<&str> =
        HashSet::from_iter(after.links.iter().map(String::as_str));
    let links_added: Vec<String> = after_links
        .difference(&before_links)
        .map(|s| (*s).to_string())
        .collect();
    let links_removed: Vec<String> = before_links
        .difference(&after_links)
        .map(|s| (*s).to_string())
        .collect();

    let before_imgs: HashSet<&str> =
        HashSet::from_iter(before.images.iter().map(String::as_str));
    let after_imgs: HashSet<&str> =
        HashSet::from_iter(after.images.iter().map(String::as_str));
    let images_added: Vec<String> = after_imgs
        .difference(&before_imgs)
        .map(|s| (*s).to_string())
        .collect();
    let images_removed: Vec<String> = before_imgs
        .difference(&after_imgs)
        .map(|s| (*s).to_string())
        .collect();

    // Meta diff: iterate the smaller map, check against the larger
    let mut meta_changes = Vec::new();
    for (key, new_val) in &after.meta {
        let old_val = before.meta.get(key).map(String::as_str).unwrap_or("");
        if old_val != new_val {
            meta_changes.push((key.clone(), old_val.to_string(), new_val.clone()));
        }
    }
    // Keys removed in after
    for (key, old_val) in &before.meta {
        if !after.meta.contains_key(key) {
            meta_changes.push((key.clone(), old_val.clone(), String::new()));
        }
    }

    let similarity = jaccard_similarity(&before.text, &after.text);

    SnapshotDiff {
        url: after.url.clone(),
        timestamp_before: before.timestamp,
        timestamp_after: after.timestamp,
        title_changed: before.title != after.title,
        html_changed: before.html != after.html,
        text_changed: before.text != after.text,
        links_added,
        links_removed,
        images_added,
        images_removed,
        meta_changes,
        element_count_delta: after.element_count as i64 - before.element_count as i64,
        word_count_delta: after.word_count as i64 - before.word_count as i64,
        similarity,
    }
}

/// Save a snapshot to a JSON file.
pub fn save_snapshot(snapshot: &DomSnapshot, path: &Path) -> Result<()> {
    let json = serde_json::to_string_pretty(snapshot)
        .map_err(|e| Error::Cdp(format!("serialize snapshot: {e}")))?;
    std::fs::write(path, json)
        .map_err(|e| Error::Cdp(format!("write snapshot to {}: {e}", path.display())))
}

/// Load a snapshot from a JSON file.
pub fn load_snapshot(path: &Path) -> Result<DomSnapshot> {
    let data = std::fs::read_to_string(path)
        .map_err(|e| Error::Cdp(format!("read snapshot from {}: {e}", path.display())))?;
    serde_json::from_str(&data)
        .map_err(|e| Error::Cdp(format!("parse snapshot from {}: {e}", path.display())))
}

/// Take snapshots at regular intervals and return the diffs.
/// Limited to at most `count` iterations (max 10).
pub async fn watch_for_changes(
    page: &Page,
    interval_ms: u64,
    selector: Option<&str>,
    count: usize,
) -> Result<Vec<SnapshotDiff>> {
    let max_iters = count.min(10);
    if max_iters == 0 {
        return Ok(Vec::new());
    }

    // If a selector is provided, wait for it first
    if let Some(sel) = selector {
        let wait_js = format!(
            "document.querySelector({}) !== null",
            serde_json::to_string(sel).unwrap_or_default()
        );
        page.evaluate(wait_js)
            .await
            .map_err(|e| Error::Cdp(format!("selector wait failed: {e}")))?;
    }

    let mut prev = take_snapshot(page).await?;
    let mut diffs = Vec::with_capacity(max_iters);

    for _ in 0..max_iters {
        tokio::time::sleep(tokio::time::Duration::from_millis(interval_ms)).await;
        let current = take_snapshot(page).await?;
        diffs.push(compare_snapshots(&prev, &current));
        prev = current;
    }

    Ok(diffs)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_snapshot(text: &str, links: Vec<&str>, images: Vec<&str>) -> DomSnapshot {
        DomSnapshot {
            url: "https://example.com".into(),
            title: "Test".into(),
            timestamp: 1000.0,
            html: format!("<html><body>{text}</body></html>"),
            text: text.into(),
            links: links.into_iter().map(|s| s.to_string()).collect(),
            images: images.into_iter().map(|s| s.to_string()).collect(),
            meta: HashMap::new(),
            element_count: 5,
            word_count: text.split_whitespace().count(),
        }
    }

    #[test]
    fn test_identical_snapshots() {
        let a = make_snapshot("hello world", vec!["https://a.com"], vec![]);
        let b = a.clone();
        let diff = compare_snapshots(&a, &b);
        assert!(!diff.title_changed);
        assert!(!diff.html_changed);
        assert!(!diff.text_changed);
        assert!(diff.links_added.is_empty());
        assert!(diff.links_removed.is_empty());
        assert!((diff.similarity - 1.0).abs() < f64::EPSILON);
        assert_eq!(diff.element_count_delta, 0);
    }

    #[test]
    fn test_different_text() {
        let a = make_snapshot("the quick brown fox", vec![], vec![]);
        let mut b = make_snapshot("the slow brown fox", vec![], vec![]);
        b.timestamp = 2000.0;
        let diff = compare_snapshots(&a, &b);
        assert!(diff.text_changed);
        assert!(diff.similarity > 0.0);
        assert!(diff.similarity < 1.0);
    }

    #[test]
    fn test_links_added_removed() {
        let a = make_snapshot("text", vec!["https://a.com", "https://b.com"], vec![]);
        let b = make_snapshot("text", vec!["https://b.com", "https://c.com"], vec![]);
        let diff = compare_snapshots(&a, &b);
        assert_eq!(diff.links_added, vec!["https://c.com"]);
        assert_eq!(diff.links_removed, vec!["https://a.com"]);
    }

    #[test]
    fn test_images_diff() {
        let a = make_snapshot("text", vec![], vec!["img1.png"]);
        let b = make_snapshot("text", vec![], vec!["img2.png"]);
        let diff = compare_snapshots(&a, &b);
        assert_eq!(diff.images_added, vec!["img2.png"]);
        assert_eq!(diff.images_removed, vec!["img1.png"]);
    }

    #[test]
    fn test_meta_changes() {
        let mut a = make_snapshot("text", vec![], vec![]);
        a.meta.insert("description".into(), "old desc".into());
        let mut b = make_snapshot("text", vec![], vec![]);
        b.meta.insert("description".into(), "new desc".into());
        let diff = compare_snapshots(&a, &b);
        assert_eq!(diff.meta_changes.len(), 1);
        assert_eq!(diff.meta_changes[0].0, "description");
        assert_eq!(diff.meta_changes[0].1, "old desc");
        assert_eq!(diff.meta_changes[0].2, "new desc");
    }

    #[test]
    fn test_jaccard_identical() {
        assert!((jaccard_similarity("a b c", "a b c") - 1.0).abs() < f64::EPSILON);
    }

    #[test]
    fn test_jaccard_disjoint() {
        assert!(jaccard_similarity("a b", "c d").abs() < f64::EPSILON);
    }

    #[test]
    fn test_jaccard_empty() {
        assert!((jaccard_similarity("", "") - 1.0).abs() < f64::EPSILON);
    }

    #[test]
    fn test_save_load_roundtrip() {
        let snap = make_snapshot("roundtrip test", vec!["https://x.com"], vec!["logo.png"]);
        let tmp = std::env::temp_dir().join("onecrawl-snap-test.json");
        save_snapshot(&snap, &tmp).unwrap();
        let loaded = load_snapshot(&tmp).unwrap();
        assert_eq!(loaded.url, snap.url);
        assert_eq!(loaded.text, snap.text);
        assert_eq!(loaded.links, snap.links);
        assert_eq!(loaded.images, snap.images);
        std::fs::remove_file(&tmp).ok();
    }

    #[test]
    fn test_element_count_delta() {
        let mut a = make_snapshot("text", vec![], vec![]);
        a.element_count = 10;
        let mut b = make_snapshot("text", vec![], vec![]);
        b.element_count = 15;
        let diff = compare_snapshots(&a, &b);
        assert_eq!(diff.element_count_delta, 5);
    }

    #[test]
    fn test_word_count_delta() {
        let a = make_snapshot("one two three", vec![], vec![]);
        let b = make_snapshot("one two three four five", vec![], vec![]);
        let diff = compare_snapshots(&a, &b);
        assert_eq!(diff.word_count_delta, 2);
    }
}
