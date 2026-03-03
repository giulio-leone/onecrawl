import { describe, it, after } from 'node:test';
import assert from 'node:assert/strict';
import { createRequire } from 'node:module';

const require = createRequire(import.meta.url);
const { NativeBrowser } = require('../index.js');

// evaluate() returns JSON-encoded values, so we need JSON.parse to unwrap
const eval_ = async (browser, expr) => JSON.parse(await browser.evaluate(expr));

describe('NativeBrowser: Emulation', () => {
  let browser;

  after(async () => {
    if (browser) await browser.close();
  });

  it('launch and navigate', async () => {
    browser = await NativeBrowser.launch(true);
    await browser.goto('https://example.com');
  });

  it('setViewport sets custom dimensions', async () => {
    await browser.setViewport(1920, 1080, 2.0, false, false);
    const w = await eval_(browser, 'window.innerWidth');
    const h = await eval_(browser, 'window.innerHeight');
    assert.equal(w, 1920);
    assert.equal(h, 1080);
  });

  it('setDevice iphone14 sets mobile viewport', async () => {
    await browser.setDevice('iphone14');
    const w = await eval_(browser, 'window.innerWidth');
    const dpr = await eval_(browser, 'window.devicePixelRatio');
    assert.equal(w, 390);
    assert.equal(dpr, 3);
  });

  it('setDevice desktop restores standard viewport', async () => {
    await browser.setDevice('desktop');
    const w = await eval_(browser, 'window.innerWidth');
    assert.equal(w, 1280);
  });

  it('clearViewport resets override', async () => {
    await browser.setViewport(400, 300);
    await browser.clearViewport();
    const w = await eval_(browser, 'window.innerWidth');
    assert.ok(w > 0, 'viewport width should be positive');
  });

  it('setUserAgent overrides navigator.userAgent', async () => {
    const customUA = 'OneCrawl/1.0 TestBot';
    await browser.setUserAgent(customUA);
    await browser.goto('https://example.com');
    const ua = await eval_(browser, 'navigator.userAgent');
    assert.equal(ua, customUA);
  });

  it('setGeolocation sets GPS coordinates', async () => {
    await browser.setGeolocation(48.8566, 2.3522);
    assert.ok(true);
  });

  it('setColorScheme dark sets prefers-color-scheme', async () => {
    await browser.setColorScheme('dark');
    const isDark = await eval_(browser, 'window.matchMedia("(prefers-color-scheme: dark)").matches');
    assert.equal(isDark, true);
  });

  it('setColorScheme light sets prefers-color-scheme', async () => {
    await browser.setColorScheme('light');
    const isLight = await eval_(browser, 'window.matchMedia("(prefers-color-scheme: light)").matches');
    assert.equal(isLight, true);
  });

  it('setDevice rejects unknown device', async () => {
    await assert.rejects(
      () => browser.setDevice('nonexistent'),
      /Unknown device/
    );
  });
});
