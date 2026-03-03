import { describe, it, before, after } from 'node:test';
import assert from 'node:assert/strict';
import { NativeBrowser } from '../index.js';

describe('Network Log and Page Watcher', () => {
  let b;
  before(async () => { b = await NativeBrowser.launch(true); });
  after(async () => { if (b) await b.close(); });

  // ── Network Log ──────────────────────────────────────────────

  it('startNetworkLog does not throw', async () => {
    await b.goto('data:text/html,<h1>NetLog</h1>');
    await assert.doesNotReject(() => b.startNetworkLog());
  });

  it('drainNetworkLog returns valid JSON array', async () => {
    const json = await b.drainNetworkLog();
    const entries = JSON.parse(json);
    assert.ok(Array.isArray(entries));
  });

  it('drainNetworkLog captures fetch requests', async () => {
    await b.evaluate('fetch("data:text/plain,hello")');
    await new Promise(r => setTimeout(r, 500));
    const json = await b.drainNetworkLog();
    const entries = JSON.parse(json);
    assert.ok(Array.isArray(entries));
  });

  it('getNetworkSummary returns valid summary object', async () => {
    const json = await b.getNetworkSummary();
    const summary = JSON.parse(json);
    assert.equal(typeof summary.total_requests, 'number');
    assert.equal(typeof summary.total_size_bytes, 'number');
    assert.ok(typeof summary.by_type === 'object');
    assert.ok(typeof summary.by_status === 'object');
    assert.ok(Array.isArray(summary.errors));
    assert.ok(Array.isArray(summary.slowest));
  });

  it('exportNetworkLog writes a file', async () => {
    const fs = await import('node:fs');
    const path = '/tmp/onecrawl-netlog-test.json';
    await b.exportNetworkLog(path);
    assert.ok(fs.existsSync(path));
    const content = JSON.parse(fs.readFileSync(path, 'utf8'));
    assert.ok(Array.isArray(content));
    fs.unlinkSync(path);
  });

  it('stopNetworkLog does not throw', async () => {
    await assert.doesNotReject(() => b.stopNetworkLog());
  });

  it('drainNetworkLog returns empty after stop + drain cycle', async () => {
    await b.startNetworkLog();
    const json = await b.drainNetworkLog();
    const entries = JSON.parse(json);
    assert.ok(Array.isArray(entries));
    await b.stopNetworkLog();
  });

  // ── Page Watcher ─────────────────────────────────────────────

  it('startPageWatcher does not throw', async () => {
    await b.goto('data:text/html,<html><head><title>Watcher</title></head><body><h1>PW</h1></body></html>');
    await assert.doesNotReject(() => b.startPageWatcher());
  });

  it('drainPageChanges returns valid JSON array', async () => {
    const json = await b.drainPageChanges();
    const changes = JSON.parse(json);
    assert.ok(Array.isArray(changes));
  });

  it('drainPageChanges captures title change', async () => {
    await b.evaluate('document.title = "New Title"');
    await new Promise(r => setTimeout(r, 300));
    const json = await b.drainPageChanges();
    const changes = JSON.parse(json);
    const titleChange = changes.find(c => c.change_type === 'title');
    if (titleChange) {
      assert.ok(titleChange.new_value.includes('New Title'));
    }
  });

  it('getPageState returns valid state object', async () => {
    const json = await b.getPageState();
    const state = JSON.parse(json);
    assert.equal(typeof state.url, 'string');
    assert.equal(typeof state.title, 'string');
    assert.equal(typeof state.ready_state, 'string');
    assert.equal(typeof state.viewport_width, 'number');
    assert.equal(typeof state.element_count, 'number');
  });

  it('stopPageWatcher does not throw', async () => {
    await assert.doesNotReject(() => b.stopPageWatcher());
  });
});
