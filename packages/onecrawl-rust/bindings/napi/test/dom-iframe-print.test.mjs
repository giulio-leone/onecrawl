import { describe, it, before, after } from 'node:test';
import assert from 'node:assert/strict';
import { NativeBrowser } from '../index.js';

describe('DOM Observer, Iframe, and Print', () => {
  let b;
  before(async () => { b = await NativeBrowser.launch(true); });
  after(async () => { if (b) await b.close(); });

  // ── DOM Observer ────────────────────────────────────────────

  it('startDomObserver does not throw', async () => {
    await b.goto('data:text/html,<h1>DOM</h1>');
    await assert.doesNotReject(() => b.startDomObserver());
  });

  it('drainDomMutations returns valid JSON array', async () => {
    await b.evaluate('document.body.innerHTML += "<p>new</p>"');
    const json = await b.drainDomMutations();
    const mutations = JSON.parse(json);
    assert.ok(Array.isArray(mutations));
  });

  it('drainDomMutations captures childList mutations', async () => {
    await b.evaluate('document.body.appendChild(document.createElement("span"))');
    const json = await b.drainDomMutations();
    const mutations = JSON.parse(json);
    const childList = mutations.find(m => m.mutation_type === 'childList');
    assert.ok(childList, 'should have a childList mutation');
  });

  it('stopDomObserver does not throw', async () => {
    await assert.doesNotReject(() => b.stopDomObserver());
  });

  it('getDomSnapshot returns HTML string', async () => {
    const html = await b.getDomSnapshot();
    assert.ok(html.includes('<'), 'should contain HTML tags');
    assert.ok(html.length > 0);
  });

  it('getDomSnapshot with selector returns element HTML', async () => {
    await b.goto('data:text/html,<div id="target">content</div>');
    const html = await b.getDomSnapshot('#target');
    assert.ok(html.includes('content'));
  });

  // ── Iframe ──────────────────────────────────────────────────

  it('listIframes returns valid JSON array', async () => {
    await b.goto('data:text/html,<iframe src="about:blank"></iframe>');
    const json = await b.listIframes();
    const iframes = JSON.parse(json);
    assert.ok(Array.isArray(iframes));
    assert.ok(iframes.length >= 1, 'should detect at least one iframe');
  });

  it('listIframes returns empty array when no iframes', async () => {
    await b.goto('data:text/html,<h1>No Frames</h1>');
    const json = await b.listIframes();
    const iframes = JSON.parse(json);
    assert.ok(Array.isArray(iframes));
    assert.equal(iframes.length, 0);
  });

  it('getIframeContent returns string', async () => {
    await b.goto('data:text/html,<iframe srcdoc="<p>hello</p>"></iframe>');
    // Small delay for iframe to load
    await new Promise(r => setTimeout(r, 500));
    const content = await b.getIframeContent(0);
    assert.ok(typeof content === 'string');
  });

  it('evalInIframe returns JSON string', async () => {
    await b.goto('data:text/html,<iframe srcdoc="<p>eval</p>"></iframe>');
    await new Promise(r => setTimeout(r, 500));
    const json = await b.evalInIframe(0, '1 + 1');
    assert.ok(typeof json === 'string');
  });

  // ── Print / PDF ─────────────────────────────────────────────

  it('printToPdf returns a Buffer', async () => {
    await b.goto('data:text/html,<h1>PDF Test</h1>');
    const buf = await b.printToPdf();
    assert.ok(buf instanceof Buffer || buf instanceof Uint8Array);
    assert.ok(buf.length > 0, 'PDF should not be empty');
  });

  it('printToPdf with options returns a Buffer', async () => {
    const opts = JSON.stringify({ landscape: true, print_background: true });
    const buf = await b.printToPdf(opts);
    assert.ok(buf.length > 0);
  });

  it('getPrintMetrics returns valid JSON', async () => {
    const json = await b.getPrintMetrics();
    const metrics = JSON.parse(json);
    assert.ok(typeof metrics === 'object');
    assert.ok('width' in metrics);
    assert.ok('height' in metrics);
  });
});
