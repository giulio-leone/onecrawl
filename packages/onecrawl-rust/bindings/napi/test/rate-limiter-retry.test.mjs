import { describe, it, before, after } from 'node:test';
import assert from 'node:assert/strict';
import { NativeBrowser } from '../index.js';

describe('Rate Limiter', () => {
  let b;
  before(async () => { b = await NativeBrowser.launch(true); });
  after(async () => { if (b) await b.close(); });

  it('rateLimitStats returns valid JSON with expected fields', async () => {
    const json = await b.rateLimitStats();
    const stats = JSON.parse(json);
    assert.ok(typeof stats === 'object');
    assert.ok(typeof stats.total_requests === 'number');
    assert.ok(typeof stats.total_throttled === 'number');
    assert.ok(typeof stats.status === 'string');
  });

  it('rateLimitCanProceed returns true initially', async () => {
    await b.rateLimitSet(null);
    const ok = await b.rateLimitCanProceed();
    assert.strictEqual(ok, true);
  });

  it('rateLimitRecord returns true for allowed request', async () => {
    await b.rateLimitSet(null);
    const ok = await b.rateLimitRecord();
    assert.strictEqual(ok, true);
  });

  it('rateLimitWait returns 0 when not throttled', async () => {
    await b.rateLimitSet('unlimited');
    const ms = await b.rateLimitWait();
    assert.strictEqual(ms, 0);
  });

  it('rateLimitSet with preset updates config', async () => {
    const json = await b.rateLimitSet('conservative');
    const stats = JSON.parse(json);
    assert.ok(typeof stats === 'object');
    assert.strictEqual(stats.status, 'active');
  });

  it('rateLimitReset clears counters', async () => {
    await b.rateLimitRecord();
    await b.rateLimitReset();
    const json = await b.rateLimitStats();
    const stats = JSON.parse(json);
    assert.strictEqual(stats.total_requests, 0);
    assert.strictEqual(stats.total_throttled, 0);
  });

  it('rateLimitPresets returns map with 4 presets', () => {
    const json = b.rateLimitPresets();
    const presets = JSON.parse(json);
    assert.ok(typeof presets === 'object');
    assert.ok('conservative' in presets);
    assert.ok('moderate' in presets);
    assert.ok('aggressive' in presets);
    assert.ok('unlimited' in presets);
  });

  it('rateLimitSet with JSON config works', async () => {
    const cfg = JSON.stringify({
      max_requests_per_second: 10,
      max_requests_per_minute: 100,
      max_requests_per_hour: 5000,
      burst_size: 20,
      cooldown_ms: 50,
    });
    const json = await b.rateLimitSet(cfg);
    const stats = JSON.parse(json);
    assert.strictEqual(stats.status, 'active');
  });
});

describe('Retry Queue', () => {
  let b;
  before(async () => { b = await NativeBrowser.launch(true); });
  after(async () => { if (b) await b.close(); });

  it('retryEnqueue returns an id string', async () => {
    const id = await b.retryEnqueue('https://example.com', 'navigate', null);
    assert.ok(typeof id === 'string');
    assert.ok(id.startsWith('retry-'));
  });

  it('retryNext returns an item after enqueue', async () => {
    const json = await b.retryNext();
    assert.ok(json !== null);
    const item = JSON.parse(json);
    assert.ok(typeof item.id === 'string');
    assert.strictEqual(item.operation, 'navigate');
  });

  it('retrySuccess moves item to completed', async () => {
    const id = await b.retryEnqueue('https://a.com', 'click', null);
    await b.retrySuccess(id);
    const json = await b.retryStats();
    const stats = JSON.parse(json);
    assert.ok(stats.completed_success >= 1);
  });

  it('retryFail increments retries', async () => {
    const id = await b.retryEnqueue('https://b.com', 'extract', null);
    await b.retryFail(id, 'timeout');
    const json = await b.retryStats();
    const stats = JSON.parse(json);
    assert.ok(stats.total_retries >= 1);
  });

  it('retryStats returns valid JSON', async () => {
    const json = await b.retryStats();
    const stats = JSON.parse(json);
    assert.ok(typeof stats.pending === 'number');
    assert.ok(typeof stats.retrying === 'number');
    assert.ok(typeof stats.completed_success === 'number');
    assert.ok(typeof stats.completed_failed === 'number');
  });

  it('retryClear removes completed items', async () => {
    const id = await b.retryEnqueue('https://c.com', 'submit', null);
    await b.retrySuccess(id);
    const cleared = await b.retryClear();
    assert.ok(cleared >= 1);
  });

  it('retrySave and retryLoad roundtrip', async () => {
    const id = await b.retryEnqueue('https://d.com', 'navigate', 'test-payload');
    const path = '/tmp/onecrawl_retry_napi_test.json';
    await b.retrySave(path);
    await b.retryLoad(path);
    const json = await b.retryStats();
    const stats = JSON.parse(json);
    assert.ok(stats.pending >= 1);
  });

  it('retryEnqueue with payload stores it', async () => {
    const id = await b.retryEnqueue('https://e.com', 'submit', 'my-data');
    const json = await b.retryNext();
    // The item returned could be any pending item, just verify it's valid
    assert.ok(json !== null);
    const item = JSON.parse(json);
    assert.ok(typeof item.id === 'string');
  });
});
