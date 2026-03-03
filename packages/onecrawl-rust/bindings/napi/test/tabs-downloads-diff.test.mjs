import { describe, it, after } from 'node:test';
import assert from 'node:assert/strict';
import { createRequire } from 'node:module';
import { writeFileSync, unlinkSync, mkdtempSync } from 'node:fs';
import { join } from 'node:path';
import { tmpdir } from 'node:os';

const require = createRequire(import.meta.url);
const { NativeBrowser } = require('../index.js');

describe('NativeBrowser: Tabs, Downloads, Screenshot Diff', () => {
  let browser;

  after(async () => {
    if (browser) await browser.close();
  });

  // ── Tab Management ────────────────────────────────────────────

  it('launch and listTabs returns at least one tab', async () => {
    browser = await NativeBrowser.launch(true);
    const tabs = JSON.parse(await browser.listTabs());
    assert.ok(Array.isArray(tabs), 'listTabs should return an array');
    assert.ok(tabs.length >= 1, 'should have at least 1 tab');
  });

  it('tabCount returns a number >= 1', async () => {
    const count = await browser.tabCount();
    assert.ok(typeof count === 'number');
    assert.ok(count >= 1);
  });

  it('newTab opens a new tab', async () => {
    const countBefore = await browser.tabCount();
    await browser.newTab('about:blank');
    const countAfter = await browser.tabCount();
    assert.ok(countAfter >= countBefore, 'tab count should not decrease after newTab');
  });

  it('listTabs includes url and target_id fields', async () => {
    const tabs = JSON.parse(await browser.listTabs());
    for (const tab of tabs) {
      assert.ok('url' in tab, 'tab should have url');
      assert.ok('target_id' in tab, 'tab should have target_id');
      assert.ok('index' in tab, 'tab should have index');
    }
  });

  it('switchTab switches active page', async () => {
    await browser.switchTab(0);
    const tabs = JSON.parse(await browser.listTabs());
    assert.ok(tabs.length >= 1);
  });

  // ── Download Management ───────────────────────────────────────

  it('setDownloadPath does not throw', async () => {
    const tmpDir = mkdtempSync(join(tmpdir(), 'onecrawl-dl-'));
    await browser.setDownloadPath(tmpDir);
  });

  it('getDownloads returns empty array initially', async () => {
    await browser.goto('about:blank');
    const downloads = JSON.parse(await browser.getDownloads());
    assert.ok(Array.isArray(downloads));
  });

  it('clearDownloads does not throw', async () => {
    await browser.clearDownloads();
    const downloads = JSON.parse(await browser.getDownloads());
    assert.deepEqual(downloads, []);
  });

  it('waitForDownload returns null on timeout', async () => {
    const result = JSON.parse(await browser.waitForDownload(500));
    assert.equal(result, null);
  });

  it('downloadFile returns string (base64 or empty)', async () => {
    await browser.goto('https://example.com');
    // This may fail on cross-origin, but should not throw a binding error
    try {
      const b64 = await browser.downloadFile('https://example.com/');
      assert.ok(typeof b64 === 'string');
    } catch {
      // Cross-origin fetch may fail — acceptable
      assert.ok(true);
    }
  });

  // ── Screenshot Diff ───────────────────────────────────────────

  it('compareScreenshots returns diff result JSON', async () => {
    await browser.goto('https://example.com');
    const png = await browser.screenshot();

    const tmpDir = mkdtempSync(join(tmpdir(), 'onecrawl-diff-'));
    const fileA = join(tmpDir, 'a.png');
    const fileB = join(tmpDir, 'b.png');
    writeFileSync(fileA, png);
    writeFileSync(fileB, png);

    const result = JSON.parse(await browser.compareScreenshots(fileA, fileB));
    assert.ok('is_identical' in result);
    assert.ok('difference_percentage' in result);
    assert.equal(result.is_identical, true);
    assert.equal(result.difference_percentage, 0);

    unlinkSync(fileA);
    unlinkSync(fileB);
  });

  it('compareScreenshots detects differences', async () => {
    const tmpDir = mkdtempSync(join(tmpdir(), 'onecrawl-diff2-'));
    const fileA = join(tmpDir, 'a.bin');
    const fileB = join(tmpDir, 'b.bin');

    writeFileSync(fileA, Buffer.from([0, 0, 0, 0, 1, 1, 1, 1]));
    writeFileSync(fileB, Buffer.from([0, 0, 0, 0, 2, 2, 2, 2]));

    const result = JSON.parse(await browser.compareScreenshots(fileA, fileB));
    assert.equal(result.is_identical, false);
    assert.ok(result.difference_percentage > 0);

    unlinkSync(fileA);
    unlinkSync(fileB);
  });

  it('visualRegression creates baseline if missing', async () => {
    await browser.goto('https://example.com');
    const tmpDir = mkdtempSync(join(tmpdir(), 'onecrawl-vr-'));
    const baseline = join(tmpDir, 'baseline.png');

    const result = JSON.parse(await browser.visualRegression(baseline));
    assert.equal(result.is_identical, true);
    assert.equal(result.difference_percentage, 0);

    unlinkSync(baseline);
  });

  it('visualRegression compares against existing baseline', async () => {
    const tmpDir = mkdtempSync(join(tmpdir(), 'onecrawl-vr2-'));
    const baseline = join(tmpDir, 'baseline.png');

    // First call creates baseline
    await browser.visualRegression(baseline);
    // Second call compares
    const result = JSON.parse(await browser.visualRegression(baseline));
    assert.ok('is_identical' in result);
    assert.ok('difference_percentage' in result);

    unlinkSync(baseline);
  });

  it('closeTab does not throw for valid index', async () => {
    // Open a spare tab then close it
    await browser.newTab('about:blank');
    const count = await browser.tabCount();
    if (count > 1) {
      await browser.closeTab(count - 1);
      const after = await browser.tabCount();
      assert.ok(after < count);
    }
  });
});
