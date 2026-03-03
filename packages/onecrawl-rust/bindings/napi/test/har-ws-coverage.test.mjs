import { describe, it, before, after } from 'node:test';
import assert from 'node:assert/strict';
import { NativeBrowser } from '../index.js';

describe('HAR, WebSocket, and Coverage', () => {
  let b;
  before(async () => { b = await NativeBrowser.launch(true); });
  after(async () => { if (b) await b.close(); });

  // ── HAR Recording ────────────────────────────────────────────

  it('startHarRecording does not throw', async () => {
    await b.goto('data:text/html,<h1>HAR</h1>');
    await assert.doesNotReject(() => b.startHarRecording());
  });

  it('drainHarEntries returns a number', async () => {
    const count = await b.drainHarEntries();
    assert.equal(typeof count, 'number');
  });

  it('exportHar returns valid HAR 1.2 JSON', async () => {
    const json = await b.exportHar();
    const har = JSON.parse(json);
    assert.equal(har.log.version, '1.2');
    assert.equal(har.log.creator.name, 'OneCrawl');
    assert.ok(Array.isArray(har.log.entries));
  });

  // ── WebSocket Recording ──────────────────────────────────────

  it('startWsRecording does not throw', async () => {
    await b.goto('data:text/html,<h1>WS</h1>');
    await assert.doesNotReject(() => b.startWsRecording());
  });

  it('drainWsFrames returns 0 when no WS traffic', async () => {
    const count = await b.drainWsFrames();
    assert.equal(count, 0);
  });

  it('exportWsFrames returns valid JSON array', async () => {
    const json = await b.exportWsFrames();
    const frames = JSON.parse(json);
    assert.ok(Array.isArray(frames));
  });

  it('activeWsConnections returns 0 when no connections', async () => {
    const count = await b.activeWsConnections();
    assert.equal(count, 0);
  });

  // ── JS Coverage ──────────────────────────────────────────────

  it('startJsCoverage does not throw', async () => {
    await b.goto('data:text/html,<script>function foo(){return 1;} foo();</script>');
    await assert.doesNotReject(() => b.startJsCoverage());
  });

  it('stopJsCoverage returns a coverage report', async () => {
    await b.evaluate('(() => { let x = 1; return x + 1; })()');
    const json = await b.stopJsCoverage();
    const report = JSON.parse(json);
    assert.ok('scripts' in report);
    assert.ok('total_bytes' in report);
    assert.ok('used_bytes' in report);
    assert.ok('overall_percent' in report);
  });

  // ── CSS Coverage ─────────────────────────────────────────────

  it('startCssCoverage does not throw', async () => {
    await b.goto('data:text/html,<style>body{color:red;}</style><p>Hello</p>');
    await assert.doesNotReject(() => b.startCssCoverage());
  });

  it('getCssCoverage returns coverage data', async () => {
    const json = await b.getCssCoverage();
    const report = JSON.parse(json);
    assert.ok('used_properties' in report);
    assert.ok('total_stylesheets' in report);
  });
});
