//! Link graph builder and analyser — extract, build, and query link graphs.

use onecrawl_browser::Page;
use onecrawl_core::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// A node in the link graph.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LinkNode {
    pub url: String,
    pub title: String,
    pub inbound: usize,
    pub outbound: usize,
    pub depth: usize,
}

/// A directed edge between two pages.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LinkEdge {
    pub source: String,
    pub target: String,
    pub anchor_text: String,
    pub is_internal: bool,
}

/// A complete link graph.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LinkGraph {
    pub nodes: Vec<LinkNode>,
    pub edges: Vec<LinkEdge>,
    pub total_internal: usize,
    pub total_external: usize,
}

/// Aggregate link-graph statistics.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LinkStats {
    pub total_nodes: usize,
    pub total_edges: usize,
    pub avg_inbound: f64,
    pub avg_outbound: f64,
    pub max_inbound_url: String,
    pub max_outbound_url: String,
    pub orphan_pages: Vec<String>,
    pub broken_links: Vec<String>,
}

// ── helpers ──────────────────────────────────────────────────────

fn extract_domain(url: &str) -> &str {
    url.split("://")
        .nth(1)
        .unwrap_or(url)
        .split('/')
        .next()
        .unwrap_or("")
        .split(':')
        .next()
        .unwrap_or("")
}

// ── public API ──────────────────────────────────────────────────

/// Extract all links from the current page via JavaScript.
pub async fn extract_links(page: &Page, base_url: &str) -> Result<Vec<LinkEdge>> {
    let js = r#"
        Array.from(document.querySelectorAll('a[href]')).map(a => ({
            href: a.href,
            text: (a.textContent || '').trim().substring(0, 200)
        })).filter(l => l.href.startsWith('http'))
    "#;
    let val = page
        .evaluate(js)
        .await
        .map_err(|e| onecrawl_core::Error::Cdp(format!("extract_links failed: {e}")))?;

    #[derive(Deserialize)]
    struct RawLink {
        href: String,
        text: String,
    }

    let raw: Vec<RawLink> = val.into_value().unwrap_or_default();
    let base_domain = extract_domain(base_url);
    let source = base_url.to_string();

    Ok(raw
        .into_iter()
        .map(|l| {
            let is_internal = extract_domain(&l.href) == base_domain;
            LinkEdge {
                source: source.clone(),
                target: l.href,
                anchor_text: l.text,
                is_internal,
            }
        })
        .collect())
}

/// Build a link graph from a collection of edges.
pub fn build_graph(edges: &[LinkEdge]) -> LinkGraph {
    let cap = edges.len();
    let mut inbound: HashMap<String, usize> = HashMap::with_capacity(cap);
    let mut outbound: HashMap<String, usize> = HashMap::with_capacity(cap);
    let mut total_internal: usize = 0;
    let mut total_external: usize = 0;

    for e in edges {
        *outbound.entry(e.source.clone()).or_default() += 1;
        *inbound.entry(e.target.clone()).or_default() += 1;
        inbound.entry(e.source.clone()).or_default();
        outbound.entry(e.target.clone()).or_default();
        if e.is_internal {
            total_internal += 1;
        } else {
            total_external += 1;
        }
    }

    // Use a HashSet for O(1) dedup instead of sort+dedup on Vec
    let all_urls: std::collections::HashSet<&str> = inbound
        .keys()
        .chain(outbound.keys())
        .map(String::as_str)
        .collect();

    let mut nodes: Vec<LinkNode> = Vec::with_capacity(all_urls.len());
    for url in all_urls {
        let ib = inbound.get(url).copied().unwrap_or(0);
        let ob = outbound.get(url).copied().unwrap_or(0);
        nodes.push(LinkNode {
            url: url.to_string(),
            title: String::new(),
            inbound: ib,
            outbound: ob,
            depth: 0,
        });
    }
    nodes.sort_by(|a, b| a.url.cmp(&b.url));

    LinkGraph {
        nodes,
        edges: edges.to_vec(),
        total_internal,
        total_external,
    }
}

/// Compute statistics for a link graph.
pub fn analyze_graph(graph: &LinkGraph) -> LinkStats {
    let total_nodes = graph.nodes.len();
    let total_edges = graph.edges.len();

    let (sum_in, sum_out) = graph.nodes.iter().fold((0usize, 0usize), |(si, so), n| {
        (si + n.inbound, so + n.outbound)
    });

    let avg_inbound = if total_nodes > 0 {
        sum_in as f64 / total_nodes as f64
    } else {
        0.0
    };
    let avg_outbound = if total_nodes > 0 {
        sum_out as f64 / total_nodes as f64
    } else {
        0.0
    };

    let max_inbound_url = graph
        .nodes
        .iter()
        .max_by_key(|n| n.inbound)
        .map(|n| n.url.clone())
        .unwrap_or_default();

    let max_outbound_url = graph
        .nodes
        .iter()
        .max_by_key(|n| n.outbound)
        .map(|n| n.url.clone())
        .unwrap_or_default();

    let orphan_pages = find_orphans(graph);

    // broken_links: targets that appear only as targets (never as sources)
    let source_set: std::collections::HashSet<&str> =
        graph.edges.iter().map(|e| e.source.as_str()).collect();
    let broken_links: Vec<String> = graph
        .nodes
        .iter()
        .filter(|n| n.inbound > 0 && n.outbound == 0 && !source_set.contains(n.url.as_str()))
        .map(|n| n.url.clone())
        .collect();

    LinkStats {
        total_nodes,
        total_edges,
        avg_inbound,
        avg_outbound,
        max_inbound_url,
        max_outbound_url,
        orphan_pages,
        broken_links,
    }
}

/// Find pages with no inbound links.
pub fn find_orphans(graph: &LinkGraph) -> Vec<String> {
    graph
        .nodes
        .iter()
        .filter(|n| n.inbound == 0)
        .map(|n| n.url.clone())
        .collect()
}

/// Find hub pages with outbound links >= `min_outbound`.
pub fn find_hubs(graph: &LinkGraph, min_outbound: usize) -> Vec<LinkNode> {
    graph
        .nodes
        .iter()
        .filter(|n| n.outbound >= min_outbound)
        .cloned()
        .collect()
}

/// Export the graph as pretty-printed JSON.
pub fn export_graph_json(graph: &LinkGraph, path: &std::path::Path) -> Result<()> {
    let json = serde_json::to_string_pretty(graph)?;
    std::fs::write(path, json)?;
    Ok(())
}

/// Build a link graph from spider crawl results.
pub fn from_crawl_results(results: &[crate::spider::CrawlResult]) -> LinkGraph {
    let mut edges: Vec<LinkEdge> = Vec::new();
    let success: Vec<&crate::spider::CrawlResult> =
        results.iter().filter(|r| r.status == "success").collect();

    // Build a set of all crawled URLs for internal detection
    let crawled_set: std::collections::HashSet<&str> =
        success.iter().map(|r| r.url.as_str()).collect();

    // We don't have per-page link targets in CrawlResult, so we model
    // the relationships based on what was discovered: each page links to
    // pages that are one depth level deeper and share the same domain prefix.
    let first_domain = success
        .first()
        .map(|r| extract_domain(&r.url))
        .unwrap_or_default();

    for r in &success {
        let source_domain = extract_domain(&r.url);
        // Create edges to all pages at depth + 1
        for target in &success {
            if target.depth == r.depth + 1 {
                let target_domain = extract_domain(&target.url);
                let is_internal =
                    target_domain == source_domain || crawled_set.contains(target.url.as_str());
                edges.push(LinkEdge {
                    source: r.url.clone(),
                    target: target.url.clone(),
                    anchor_text: target.title.clone(),
                    is_internal: is_internal && target_domain == first_domain,
                });
            }
        }
    }

    let mut graph = build_graph(&edges);

    // Enrich nodes with titles and depths from crawl results
    let title_map: HashMap<&str, (&str, usize)> = success
        .iter()
        .map(|r| (r.url.as_str(), (r.title.as_str(), r.depth)))
        .collect();

    for node in &mut graph.nodes {
        if let Some(&(title, depth)) = title_map.get(node.url.as_str()) {
            node.title = title.to_string();
            node.depth = depth;
        }
    }

    graph
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_edges() -> Vec<LinkEdge> {
        vec![
            LinkEdge {
                source: "https://a.com".into(),
                target: "https://a.com/page1".into(),
                anchor_text: "Page 1".into(),
                is_internal: true,
            },
            LinkEdge {
                source: "https://a.com".into(),
                target: "https://a.com/page2".into(),
                anchor_text: "Page 2".into(),
                is_internal: true,
            },
            LinkEdge {
                source: "https://a.com/page1".into(),
                target: "https://a.com/page2".into(),
                anchor_text: "Page 2 link".into(),
                is_internal: true,
            },
            LinkEdge {
                source: "https://a.com".into(),
                target: "https://external.com".into(),
                anchor_text: "External".into(),
                is_internal: false,
            },
        ]
    }

    #[test]
    fn test_build_graph() {
        let graph = build_graph(&sample_edges());
        assert_eq!(graph.edges.len(), 4);
        assert!(graph.nodes.len() >= 3);
        assert_eq!(graph.total_internal, 3);
        assert_eq!(graph.total_external, 1);
    }

    #[test]
    fn test_analyze_graph() {
        let graph = build_graph(&sample_edges());
        let stats = analyze_graph(&graph);
        assert_eq!(stats.total_edges, 4);
        assert!(stats.total_nodes >= 3);
        assert!(stats.avg_inbound > 0.0);
        assert!(stats.avg_outbound > 0.0);
        assert!(!stats.max_inbound_url.is_empty());
        assert!(!stats.max_outbound_url.is_empty());
    }

    #[test]
    fn test_find_orphans() {
        let graph = build_graph(&sample_edges());
        let orphans = find_orphans(&graph);
        // "https://a.com" is a source but never a target → orphan
        assert!(orphans.contains(&"https://a.com".to_string()));
    }

    #[test]
    fn test_find_hubs() {
        let graph = build_graph(&sample_edges());
        let hubs = find_hubs(&graph, 2);
        // "https://a.com" has 3 outbound links
        assert!(hubs.iter().any(|n| n.url == "https://a.com"));
    }

    #[test]
    fn test_find_hubs_high_threshold() {
        let graph = build_graph(&sample_edges());
        let hubs = find_hubs(&graph, 100);
        assert!(hubs.is_empty());
    }
}
