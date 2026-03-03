import { describe, it, before, after } from 'node:test';
import assert from 'node:assert/strict';
import { NativeBrowser } from '../index.js';
import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';

const HTML = `data:text/html,
<html>
<head><title>Spider Root</title></head>
<body>
  <h1>Home</h1>
  <a href="https://example.com/page1">Page 1</a>
  <a href="https://example.com/page2">Page 2</a>
  <a href="mailto:test@test.com">Mail</a>
  <a href="javascript:void(0)">JS</a>
  <a href="https://example.com/image.png">Image</a>
  <p class="content">Root content text</p>
</body>
</html>`;

describe('Spider / Crawl', () => {
  let b;
  before(async () => { b = await NativeBrowser.launch(true); });
  after(async () => { if (b) await b.close(); });

  it('crawl returns results array', async () => {
    const config = JSON.stringify({
      start_urls: ['data:text/html,<html><head><title>T</title></head><body>Hello</body></html>'],
      max_depth: 0,
      max_pages: 1,
      follow_links: false,
      same_domain_only: false,
    });
    const raw = await b.crawl(config);
    const results = JSON.parse(raw);
    assert.ok(Array.isArray(results));
    assert.equal(results.length, 1);
    assert.equal(results[0].status, 'success');
    assert.ok(results[0].timestamp > 0);
  });

  it('crawl respects max_pages', async () => {
    const config = JSON.stringify({
      start_urls: [
        'data:text/html,<html><head><title>A</title></head><body>A</body></html>',
        'data:text/html,<html><head><title>B</title></head><body>B</body></html>',
        'data:text/html,<html><head><title>C</title></head><body>C</body></html>',
      ],
      max_depth: 0,
      max_pages: 2,
      follow_links: false,
      same_domain_only: false,
    });
    const results = JSON.parse(await b.crawl(config));
    assert.ok(results.length <= 2);
  });

  it('crawl extracts content with selector', async () => {
    const config = JSON.stringify({
      start_urls: ['data:text/html,<html><body><p class="x">extracted</p></body></html>'],
      max_depth: 0,
      max_pages: 1,
      follow_links: false,
      same_domain_only: false,
      extract_selector: 'p.x',
      extract_format: 'text',
    });
    const results = JSON.parse(await b.crawl(config));
    assert.equal(results[0].content, 'extracted');
  });

  it('crawlSummary computes stats', async () => {
    const results = JSON.stringify([
      { url: 'https://a.com/', status: 'success', title: 'A', depth: 0, links_found: 2, content: null, error: null, duration_ms: 100, timestamp: 0 },
      { url: 'https://a.com/x', status: 'error', title: '', depth: 1, links_found: 0, content: null, error: 'fail', duration_ms: 50, timestamp: 0 },
    ]);
    const summary = JSON.parse(b.crawlSummary(results));
    assert.equal(summary.total_pages, 2);
    assert.equal(summary.successful, 1);
    assert.equal(summary.failed, 1);
    assert.equal(summary.total_links_found, 2);
  });

  it('saveCrawlState + loadCrawlState round-trip', () => {
    const state = {
      config: { start_urls: ['https://example.com'], max_depth: 3, max_pages: 100, concurrency: 3, delay_ms: 500, follow_links: true, same_domain_only: true, url_patterns: [], exclude_patterns: [], extract_selector: null, extract_format: 'text', timeout_ms: 30000, user_agent: null },
      visited: ['https://example.com'],
      pending: [['https://example.com/a', 1]],
      results: [],
      status: 'paused',
    };
    const tmp = path.join(os.tmpdir(), `spider-state-${Date.now()}.json`);
    b.saveCrawlState(JSON.stringify(state), tmp);
    const loaded = JSON.parse(b.loadCrawlState(tmp));
    assert.equal(loaded.status, 'paused');
    assert.deepEqual(loaded.visited, ['https://example.com']);
    fs.unlinkSync(tmp);
  });

  it('exportCrawlResults writes JSON file', () => {
    const results = [
      { url: 'https://a.com/', status: 'success', title: 'A', depth: 0, links_found: 0, content: null, error: null, duration_ms: 10, timestamp: 0 },
    ];
    const tmp = path.join(os.tmpdir(), `spider-results-${Date.now()}.json`);
    const count = b.exportCrawlResults(JSON.stringify(results), tmp);
    assert.equal(count, 1);
    const data = JSON.parse(fs.readFileSync(tmp, 'utf-8'));
    assert.equal(data.length, 1);
    fs.unlinkSync(tmp);
  });

  it('exportCrawlResults writes JSONL file', () => {
    const results = [
      { url: 'https://a.com/', status: 'success', title: 'A', depth: 0, links_found: 0, content: null, error: null, duration_ms: 10, timestamp: 0 },
      { url: 'https://b.com/', status: 'error', title: '', depth: 0, links_found: 0, content: null, error: 'e', duration_ms: 5, timestamp: 0 },
    ];
    const tmp = path.join(os.tmpdir(), `spider-results-${Date.now()}.jsonl`);
    const count = b.exportCrawlResults(JSON.stringify(results), tmp, 'jsonl');
    assert.equal(count, 2);
    const lines = fs.readFileSync(tmp, 'utf-8').trim().split('\n');
    assert.equal(lines.length, 2);
    assert.equal(JSON.parse(lines[0]).url, 'https://a.com/');
    fs.unlinkSync(tmp);
  });

  it('crawl with invalid config rejects', async () => {
    await assert.rejects(
      () => b.crawl('not-json'),
      /reason/i
    );
  });
});
