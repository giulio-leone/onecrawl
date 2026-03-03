import { describe, it, before, after } from 'node:test';
import assert from 'node:assert/strict';
import { NativeBrowser } from '../index.js';

describe('Accessibility, Throttling, and Performance', () => {
  let b;
  before(async () => { b = await NativeBrowser.launch(true); });
  after(async () => { if (b) await b.close(); });

  // ── Accessibility ────────────────────────────────────────────

  it('getAccessibilityTree returns JSON', async () => {
    await b.goto('data:text/html,<h1>Title</h1><p>Text</p><button>Click</button>');
    const json = await b.getAccessibilityTree();
    const tree = JSON.parse(json);
    assert.ok(tree !== null && typeof tree === 'object');
  });

  it('getElementAccessibility returns element info', async () => {
    const json = await b.getElementAccessibility('h1');
    const info = JSON.parse(json);
    assert.ok(info !== null && typeof info === 'object');
  });

  it('auditAccessibility returns audit report', async () => {
    await b.goto('data:text/html,<img src="x.png"><input type="text"><a href="#">Link</a>');
    const json = await b.auditAccessibility();
    const audit = JSON.parse(json);
    assert.ok('issues' in audit);
    assert.ok('summary' in audit);
    assert.ok(audit.summary.total_issues >= 0);
  });

  // ── Network Throttling ───────────────────────────────────────

  it('setNetworkThrottle slow3g does not throw', async () => {
    await assert.doesNotReject(() => b.setNetworkThrottle('slow3g'));
  });

  it('setNetworkThrottleCustom does not throw', async () => {
    await assert.doesNotReject(() => b.setNetworkThrottleCustom(1000, 500, 100));
  });

  it('clearNetworkThrottle does not throw', async () => {
    await assert.doesNotReject(() => b.clearNetworkThrottle());
  });

  it('setNetworkThrottle rejects unknown profile', async () => {
    await assert.rejects(() => b.setNetworkThrottle('unknown_profile'));
  });

  // ── Performance Tracing ──────────────────────────────────────

  it('getPerformanceMetrics returns JSON', async () => {
    await b.goto('data:text/html,<h1>Perf</h1>');
    const json = await b.getPerformanceMetrics();
    const metrics = JSON.parse(json);
    assert.ok(Array.isArray(metrics));
  });

  it('getNavigationTiming returns timing data', async () => {
    const json = await b.getNavigationTiming();
    const timing = JSON.parse(json);
    assert.ok(timing !== null && typeof timing === 'object');
  });

  it('getResourceTiming returns array', async () => {
    const json = await b.getResourceTiming();
    const resources = JSON.parse(json);
    assert.ok(Array.isArray(resources));
  });

  it('startTracing and stopTracing work', async () => {
    await b.startTracing();
    await b.goto('data:text/html,<h1>Trace</h1>');
    const json = await b.stopTracing();
    const trace = JSON.parse(json);
    assert.ok(trace !== null && typeof trace === 'object');
  });
});
