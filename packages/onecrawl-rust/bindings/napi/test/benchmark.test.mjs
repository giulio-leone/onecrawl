import { describe, it } from 'node:test';
import assert from 'node:assert/strict';
import { createRequire } from 'node:module';

const require = createRequire(import.meta.url);
const { NativeBrowser } = require('../index.js');

describe('Benchmark', () => {
  let browser;

  it('runBenchmark returns valid JSON with default iterations', async () => {
    browser = await NativeBrowser.launch(true);
    await browser.goto('data:text/html,<h1>bench</h1>');
    const json = await browser.runBenchmark(3);
    assert.ok(typeof json === 'string', 'result should be a string');
    const suite = JSON.parse(json);
    assert.ok(Array.isArray(suite.results), 'results should be an array');
    assert.ok(suite.results.length > 0, 'should have at least one result');
    assert.ok(typeof suite.total_duration_ms === 'number', 'total_duration_ms should be number');
    assert.ok(typeof suite.timestamp === 'string', 'timestamp should be string');
  });

  it('each result has expected fields', async () => {
    const json = await browser.runBenchmark(2);
    const suite = JSON.parse(json);
    const first = suite.results[0];
    for (const field of ['name', 'iterations', 'avg_ms', 'min_ms', 'max_ms', 'p50_ms', 'p95_ms', 'p99_ms', 'ops_per_sec']) {
      assert.ok(field in first, `result missing field: ${field}`);
    }
    assert.ok(typeof first.name === 'string');
    assert.ok(typeof first.avg_ms === 'number');
    assert.ok(first.iterations >= 2);
  });

  it('result JSON is re-parseable (round-trip)', async () => {
    const json = await browser.runBenchmark(2);
    const parsed = JSON.parse(json);
    const reparsed = JSON.parse(JSON.stringify(parsed));
    assert.deepEqual(reparsed.results.length, parsed.results.length);
    await browser.close();
  });
});
