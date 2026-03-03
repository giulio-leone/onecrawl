import { describe, it, before, after } from 'node:test';
import assert from 'node:assert/strict';
import { NativeBrowser } from '../index.js';
import fs from 'node:fs';

describe('TLS Fingerprint', () => {
  let b;
  before(async () => { b = await NativeBrowser.launch(true); });
  after(async () => { if (b) await b.close(); });

  it('fingerprintProfiles returns 6 profiles', () => {
    const json = b.fingerprintProfiles();
    const profiles = JSON.parse(json);
    assert.ok(Array.isArray(profiles));
    assert.equal(profiles.length, 6);
    const names = profiles.map(p => p.name);
    assert.ok(names.includes('chrome-win'));
    assert.ok(names.includes('safari-mac'));
    assert.ok(names.includes('edge-win'));
  });

  it('applyFingerprint applies chrome-win profile', async () => {
    await b.goto('data:text/html,<h1>FP Test</h1>');
    const json = await b.applyFingerprint('chrome-win');
    const overridden = JSON.parse(json);
    assert.ok(Array.isArray(overridden));
    assert.ok(overridden.length > 0);
    assert.ok(overridden.includes('userAgent'));
    assert.ok(overridden.includes('platform'));
  });

  it('applyFingerprint rejects unknown profile', async () => {
    await assert.rejects(() => b.applyFingerprint('nonexistent'));
  });

  it('applyRandomFingerprint returns a fingerprint', async () => {
    await b.goto('data:text/html,<h1>Random FP</h1>');
    const json = await b.applyRandomFingerprint();
    const fp = JSON.parse(json);
    assert.equal(fp.name, 'random');
    assert.ok(fp.user_agent.length > 0);
    assert.ok(fp.screen_width > 0);
  });

  it('detectFingerprint returns current browser fingerprint', async () => {
    await b.goto('data:text/html,<h1>Detect</h1>');
    const json = await b.detectFingerprint();
    const fp = JSON.parse(json);
    assert.equal(fp.name, 'detected');
    assert.ok(typeof fp.user_agent === 'string');
    assert.ok(typeof fp.hardware_concurrency === 'number');
    assert.ok(fp.screen_width > 0);
  });

  it('applyCustomFingerprint applies from JSON', async () => {
    await b.goto('data:text/html,<h1>Custom</h1>');
    const profiles = JSON.parse(b.fingerprintProfiles());
    const customFp = profiles[0];
    customFp.name = 'custom-test';
    const json = await b.applyCustomFingerprint(JSON.stringify(customFp));
    const overridden = JSON.parse(json);
    assert.ok(overridden.includes('userAgent'));
  });

  it('applyCustomFingerprint rejects invalid JSON', async () => {
    await assert.rejects(() => b.applyCustomFingerprint('{invalid'));
  });

  it('applyFingerprint changes navigator.platform', async () => {
    await b.goto('data:text/html,<h1>Platform</h1>');
    await b.applyFingerprint('firefox-mac');
    const platform = JSON.parse(await b.evaluate('navigator.platform'));
    assert.equal(platform, 'MacIntel');
  });

  it('fingerprintProfiles each have required fields', () => {
    const profiles = JSON.parse(b.fingerprintProfiles());
    for (const p of profiles) {
      assert.ok(p.name, 'name required');
      assert.ok(p.user_agent, 'user_agent required');
      assert.ok(p.platform, 'platform required');
      assert.ok(typeof p.hardware_concurrency === 'number');
      assert.ok(typeof p.screen_width === 'number');
      assert.ok(typeof p.screen_height === 'number');
      assert.ok(typeof p.pixel_ratio === 'number');
    }
  });
});

describe('Page Snapshot', () => {
  let b;
  before(async () => { b = await NativeBrowser.launch(true); });
  after(async () => { if (b) await b.close(); });

  it('takeSnapshot captures page state', async () => {
    await b.goto('data:text/html,<html><head><title>Snap</title></head><body><p>Hello</p><a href="https://example.com">Link</a></body></html>');
    const json = await b.takeSnapshot();
    const snap = JSON.parse(json);
    assert.equal(snap.title, 'Snap');
    assert.ok(snap.text.includes('Hello'));
    assert.ok(snap.links.length > 0);
    assert.ok(snap.element_count > 0);
    assert.ok(snap.word_count > 0);
  });

  it('compareSnapshots detects no changes on identical snapshots', async () => {
    await b.goto('data:text/html,<p>Same</p>');
    const snap = await b.takeSnapshot();
    const diff = JSON.parse(b.compareSnapshots(snap, snap));
    assert.equal(diff.title_changed, false);
    assert.equal(diff.html_changed, false);
    assert.equal(diff.text_changed, false);
    assert.equal(diff.similarity, 1.0);
    assert.equal(diff.links_added.length, 0);
  });

  it('compareSnapshots detects text changes', async () => {
    await b.goto('data:text/html,<p>Version 1</p>');
    const snap1 = await b.takeSnapshot();
    await b.goto('data:text/html,<p>Version 2</p>');
    const snap2 = await b.takeSnapshot();
    const diff = JSON.parse(b.compareSnapshots(snap1, snap2));
    assert.equal(diff.text_changed, true);
    assert.ok(diff.similarity < 1.0);
  });

  it('saveSnapshot and loadSnapshot roundtrip', async () => {
    await b.goto('data:text/html,<p>Save me</p>');
    const snapJson = await b.takeSnapshot();
    const tmpPath = '/tmp/onecrawl-snap-napi-' + Date.now() + '.json';
    b.saveSnapshot(snapJson, tmpPath);
    const loaded = JSON.parse(b.loadSnapshot(tmpPath));
    const original = JSON.parse(snapJson);
    assert.equal(loaded.title, original.title);
    assert.equal(loaded.text, original.text);
    fs.unlinkSync(tmpPath);
  });

  it('loadSnapshot rejects missing file', () => {
    assert.throws(() => b.loadSnapshot('/tmp/nonexistent-snap.json'));
  });

  it('compareSnapshots detects added links', async () => {
    await b.goto('data:text/html,<a href="https://a.com">A</a>');
    const snap1 = await b.takeSnapshot();
    await b.goto('data:text/html,<a href="https://a.com">A</a><a href="https://b.com">B</a>');
    const snap2 = await b.takeSnapshot();
    const diff = JSON.parse(b.compareSnapshots(snap1, snap2));
    assert.ok(diff.links_added.length > 0);
  });

  it('compareSnapshots reports element_count_delta', async () => {
    await b.goto('data:text/html,<div><p>One</p></div>');
    const snap1 = await b.takeSnapshot();
    await b.goto('data:text/html,<div><p>One</p><p>Two</p><p>Three</p></div>');
    const snap2 = await b.takeSnapshot();
    const diff = JSON.parse(b.compareSnapshots(snap1, snap2));
    assert.ok(diff.element_count_delta > 0);
  });

  it('takeSnapshot has meta field', async () => {
    await b.goto('data:text/html,<html><head><meta name="description" content="test desc"></head><body>Hi</body></html>');
    const snap = JSON.parse(await b.takeSnapshot());
    assert.ok(typeof snap.meta === 'object');
    assert.equal(snap.meta.description, 'test desc');
  });

  it('compareSnapshots rejects invalid JSON', () => {
    assert.throws(() => b.compareSnapshots('{bad', '{}'));
  });
});
