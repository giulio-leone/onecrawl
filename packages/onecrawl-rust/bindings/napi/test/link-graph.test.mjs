import { describe, it, before, after } from 'node:test';
import assert from 'node:assert/strict';
import { NativeBrowser } from '../index.js';
import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';

describe('Link Graph', () => {
  let b;
  before(async () => { b = await NativeBrowser.launch(true); });
  after(async () => { if (b) await b.close(); });

  const EDGES = [
    { source: 'https://a.com/', target: 'https://a.com/about', anchor_text: 'About', is_internal: true },
    { source: 'https://a.com/', target: 'https://a.com/blog', anchor_text: 'Blog', is_internal: true },
    { source: 'https://a.com/', target: 'https://external.com', anchor_text: 'Ext', is_internal: false },
    { source: 'https://a.com/about', target: 'https://a.com/', anchor_text: 'Home', is_internal: true },
    { source: 'https://a.com/blog', target: 'https://a.com/', anchor_text: 'Home', is_internal: true },
  ];

  it('graphBuild creates nodes and edges', () => {
    const raw = b.graphBuild(JSON.stringify(EDGES));
    const graph = JSON.parse(raw);
    assert.ok(Array.isArray(graph.nodes));
    assert.ok(graph.nodes.length >= 3);
    assert.equal(graph.edges.length, 5);
    assert.equal(graph.total_internal, 4);
    assert.equal(graph.total_external, 1);
  });

  it('graphAnalyze computes statistics', () => {
    const graph = b.graphBuild(JSON.stringify(EDGES));
    const raw = b.graphAnalyze(graph);
    const stats = JSON.parse(raw);
    assert.equal(stats.total_edges, 5);
    assert.ok(stats.total_nodes >= 3);
    assert.ok(stats.avg_outbound > 0);
    assert.ok(stats.max_inbound_url.length > 0);
  });

  it('graphFindOrphans detects orphan pages', () => {
    const graph = b.graphBuild(JSON.stringify(EDGES));
    const orphans = JSON.parse(b.graphFindOrphans(graph));
    assert.ok(Array.isArray(orphans));
    // https://a.com/ has inbound from about and blog, so not orphan
    // https://external.com has no outbound links out but has inbound
    assert.ok(!orphans.includes('https://a.com/about'));
  });

  it('graphFindHubs finds pages with many outbound links', () => {
    const graph = b.graphBuild(JSON.stringify(EDGES));
    const hubs = JSON.parse(b.graphFindHubs(graph, 2));
    assert.ok(Array.isArray(hubs));
    assert.ok(hubs.some(h => h.url === 'https://a.com/'));
  });

  it('graphExport writes graph to file', () => {
    const graph = b.graphBuild(JSON.stringify(EDGES));
    const tmp = path.join(os.tmpdir(), `graph-${Date.now()}.json`);
    b.graphExport(graph, tmp);
    const data = JSON.parse(fs.readFileSync(tmp, 'utf-8'));
    assert.ok(data.nodes.length >= 3);
    fs.unlinkSync(tmp);
  });

  it('graphFromCrawlResults builds graph from crawl data', () => {
    const results = [
      { url: 'https://a.com/', status: 'success', title: 'Home', depth: 0, links_found: 2, content: null, error: null, duration_ms: 10, timestamp: 0 },
      { url: 'https://a.com/about', status: 'success', title: 'About', depth: 1, links_found: 1, content: null, error: null, duration_ms: 10, timestamp: 0 },
      { url: 'https://a.com/blog', status: 'success', title: 'Blog', depth: 1, links_found: 0, content: null, error: null, duration_ms: 10, timestamp: 0 },
    ];
    const raw = b.graphFromCrawlResults(JSON.stringify(results));
    const graph = JSON.parse(raw);
    assert.ok(graph.nodes.length >= 2);
    assert.ok(graph.edges.length >= 2);
  });

  it('graphBuild handles empty edges', () => {
    const raw = b.graphBuild(JSON.stringify([]));
    const graph = JSON.parse(raw);
    assert.deepEqual(graph.nodes, []);
    assert.deepEqual(graph.edges, []);
    assert.equal(graph.total_internal, 0);
    assert.equal(graph.total_external, 0);
  });

  it('graphAnalyze handles empty graph', () => {
    const graph = b.graphBuild(JSON.stringify([]));
    const stats = JSON.parse(b.graphAnalyze(graph));
    assert.equal(stats.total_nodes, 0);
    assert.equal(stats.total_edges, 0);
    assert.equal(stats.avg_inbound, 0);
    assert.equal(stats.avg_outbound, 0);
  });
});
