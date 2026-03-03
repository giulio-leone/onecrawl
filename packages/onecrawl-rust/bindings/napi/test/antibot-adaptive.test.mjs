import { describe, it, before, after } from 'node:test';
import assert from 'node:assert/strict';
import { NativeBrowser } from '../index.js';

describe('Anti-Bot Bypass', () => {
  let b;
  before(async () => { b = await NativeBrowser.launch(true); });
  after(async () => { if (b) await b.close(); });

  it('injectStealthFull returns applied patches array', async () => {
    await b.goto('data:text/html,<h1>Antibot</h1>');
    const json = await b.injectStealthFull();
    const patches = JSON.parse(json);
    assert.ok(Array.isArray(patches));
    assert.ok(patches.length > 0);
    assert.ok(patches.includes('webdriver'));
    assert.ok(patches.includes('chrome_runtime'));
  });

  it('botDetectionTest returns score object', async () => {
    const json = await b.botDetectionTest();
    const result = JSON.parse(json);
    assert.ok(typeof result === 'object');
    assert.ok(typeof result.score === 'number');
    assert.ok(result.score >= 0 && result.score <= 100);
  });

  it('botDetectionTest has expected fields', async () => {
    const json = await b.botDetectionTest();
    const result = JSON.parse(json);
    assert.ok('chrome' in result);
    assert.ok('plugins_length' in result);
    assert.ok('screen' in result);
    assert.ok('visibility_state' in result);
    assert.ok('hardware_concurrency' in result);
  });

  it('stealthProfiles returns valid profiles', () => {
    const json = b.stealthProfiles();
    const profiles = JSON.parse(json);
    assert.ok(Array.isArray(profiles));
    assert.equal(profiles.length, 3);
    const names = profiles.map(p => p.name);
    assert.ok(names.includes('basic'));
    assert.ok(names.includes('standard'));
    assert.ok(names.includes('aggressive'));
  });

  it('stealthProfiles levels are correct', () => {
    const profiles = JSON.parse(b.stealthProfiles());
    const aggressive = profiles.find(p => p.name === 'aggressive');
    assert.ok(aggressive.patches.length > 10);
    assert.ok(aggressive.patches.includes('canvas'));
    assert.ok(aggressive.patches.includes('audio'));
  });

  it('injectStealthFull can be called multiple times', async () => {
    const json1 = await b.injectStealthFull();
    const json2 = await b.injectStealthFull();
    const p1 = JSON.parse(json1);
    const p2 = JSON.parse(json2);
    assert.deepEqual(p1, p2);
  });
});

describe('Adaptive Element Tracker', () => {
  let b;
  before(async () => { b = await NativeBrowser.launch(true); });
  after(async () => { if (b) await b.close(); });

  it('fingerprintElement captures element data', async () => {
    await b.goto('data:text/html,<div id="target" class="box big">Hello World</div>');
    const json = await b.fingerprintElement('#target');
    const fp = JSON.parse(json);
    assert.equal(fp.tag, 'div');
    assert.ok(fp.classes.includes('box'));
    assert.ok(fp.classes.includes('big'));
    assert.ok(fp.text_preview.includes('Hello World'));
  });

  it('fingerprintElement rejects missing selector', async () => {
    await assert.rejects(() => b.fingerprintElement('#nonexistent'));
  });

  it('relocateElement finds exact match', async () => {
    await b.goto('data:text/html,<div id="target" class="box">Content</div>');
    const fpJson = await b.fingerprintElement('#target');
    const matches = JSON.parse(await b.relocateElement(fpJson));
    assert.ok(matches.length > 0);
    assert.equal(matches[0].score, 100);
    assert.equal(matches[0].match_type, 'exact');
  });

  it('trackElements returns fingerprints array', async () => {
    await b.goto('data:text/html,<h1>Title</h1><p id="para">Text</p>');
    const selectors = JSON.stringify(['h1', '#para']);
    const json = await b.trackElements(selectors);
    const fps = JSON.parse(json);
    assert.equal(fps.length, 2);
    assert.equal(fps[0].tag, 'h1');
    assert.equal(fps[1].tag, 'p');
  });

  it('relocateAll finds matches for multiple elements', async () => {
    await b.goto('data:text/html,<h1>Title</h1><p id="para">Text</p>');
    const fpJson = await b.trackElements(JSON.stringify(['h1', '#para']));
    const results = JSON.parse(await b.relocateAll(fpJson));
    assert.equal(results.length, 2);
    assert.ok(results[0][1].length > 0);
    assert.ok(results[1][1].length > 0);
  });

  it('saveFingerprints and loadFingerprints roundtrip', async () => {
    await b.goto('data:text/html,<div id="rt" class="test">Roundtrip</div>');
    const fpJson = await b.fingerprintElement('#rt');
    const tmpPath = '/tmp/onecrawl-test-fp-' + Date.now() + '.json';
    b.saveFingerprints('[' + fpJson + ']', tmpPath);
    const loaded = JSON.parse(b.loadFingerprints(tmpPath));
    assert.equal(loaded.length, 1);
    assert.equal(loaded[0].tag, 'div');
    assert.ok(loaded[0].classes.includes('test'));
    // cleanup
    const fs = await import('node:fs');
    fs.unlinkSync(tmpPath);
  });
});
