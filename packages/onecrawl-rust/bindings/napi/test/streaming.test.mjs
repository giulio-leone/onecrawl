import { describe, it, after } from 'node:test';
import assert from 'node:assert/strict';
import { createRequire } from 'node:module';

const require = createRequire(import.meta.url);
const { NativeBrowser } = require('../index.js');

// evaluate() returns JSON-encoded values
const eval_ = async (browser, expr) => JSON.parse(await browser.evaluate(expr));

describe('NativeBrowser: Event Streaming + Network + Screenshot options', () => {
  let browser;

  after(async () => {
    if (browser) await browser.close();
  });

  it('launch and navigate', async () => {
    browser = await NativeBrowser.launch(true);
    await browser.goto('https://example.com');
  });

  // ── Event Streaming ──

  it('startEventStream installs observers', async () => {
    await browser.startEventStream();
    assert.ok(true);
  });

  it('drainEvents returns event counts', async () => {
    // Trigger console output
    await browser.evaluate('(console.log("test message"), "ok")');
    await browser.evaluate('(console.warn("test warning"), "ok")');
    await new Promise(r => setTimeout(r, 300));

    const result = JSON.parse(await browser.drainEvents());
    assert.ok(result.console_messages >= 2, `expected >= 2, got ${result.console_messages}`);
    assert.equal(typeof result.page_errors, 'number');
    assert.equal(result.total, result.console_messages + result.page_errors);
  });

  it('emitEvent sends custom event', async () => {
    await browser.emitEvent('custom', JSON.stringify({ key: 'value' }));
    assert.ok(true);
  });

  it('drainEvents returns 0 when no new events', async () => {
    // Drain first to clear buffer
    await browser.drainEvents();
    await new Promise(r => setTimeout(r, 100));
    const result = JSON.parse(await browser.drainEvents());
    assert.equal(result.total, 0);
  });

  // ── Network blocking ──

  it('blockResources accepts valid resource types', async () => {
    await browser.blockResources(['Image', 'Font']);
    assert.ok(true);
  });

  // ── Screenshot with options ──

  it('screenshotWithOptions png returns bytes', async () => {
    const buf = await browser.screenshotWithOptions('png', null, false);
    assert.ok(buf.length > 100, 'screenshot should have substantial data');
  });

  it('screenshotWithOptions jpeg returns bytes', async () => {
    const buf = await browser.screenshotWithOptions('jpeg', 80, false);
    assert.ok(buf.length > 100, 'jpeg screenshot should have data');
  });

  it('screenshotWithOptions full page', async () => {
    const buf = await browser.screenshotWithOptions(null, null, true);
    assert.ok(buf.length > 100);
  });

  // ── PDF with options ──

  it('pdfWithOptions returns bytes', async () => {
    const buf = await browser.pdfWithOptions(true, 0.8, 8.5, 11.0);
    assert.ok(buf.length > 100, 'pdf should have substantial data');
  });

  it('pdfWithOptions landscape false', async () => {
    const buf = await browser.pdfWithOptions(false);
    assert.ok(buf.length > 100);
  });
});
