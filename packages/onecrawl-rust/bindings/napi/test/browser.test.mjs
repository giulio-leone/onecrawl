import { describe, it, after } from 'node:test';
import assert from 'node:assert/strict';
import { createRequire } from 'node:module';

const require = createRequire(import.meta.url);
const { NativeBrowser } = require('../index.js');

describe('NativeBrowser: CDP operations', () => {
  let browser;

  after(async () => {
    if (browser) await browser.close();
  });

  it('launch + goto + getTitle + getUrl', async () => {
    browser = await NativeBrowser.launch(true);
    await browser.goto('https://example.com');
    const title = await browser.getTitle();
    assert.equal(title, 'Example Domain');
    const url = await browser.getUrl();
    assert.ok(url.includes('example.com'));
  });

  it('content returns HTML', async () => {
    const html = await browser.content();
    assert.ok(html.includes('Example Domain'));
    assert.ok(html.includes('<h1>'));
  });

  it('screenshot returns PNG bytes', async () => {
    const png = await browser.screenshot();
    assert.ok(png.length > 1000, 'screenshot should be > 1KB');
    // PNG magic bytes
    assert.equal(png[0], 0x89);
    assert.equal(png[1], 0x50); // P
    assert.equal(png[2], 0x4E); // N
    assert.equal(png[3], 0x47); // G
  });

  it('screenshotFull returns PNG bytes', async () => {
    const png = await browser.screenshotFull();
    assert.ok(png.length > 1000);
  });

  it('evaluate returns JS result', async () => {
    const result = await browser.evaluate('document.title');
    assert.ok(result.includes('Example Domain'));
  });

  it('getText retrieves element text', async () => {
    const text = await browser.getText('h1');
    assert.equal(text, 'Example Domain');
  });

  it('getAttribute reads href', async () => {
    const href = await browser.getAttribute('a', 'href');
    assert.ok(href.includes('iana.org'));
  });

  it('click does not throw on valid selector', async () => {
    await browser.goto('https://example.com');
    await browser.click('h1');
  });

  it('reload reloads page', async () => {
    await browser.goto('https://example.com');
    await browser.reload();
    await browser.wait(500);
    const title = await browser.getTitle();
    assert.equal(title, 'Example Domain');
  });

  it('waitForSelector finds h1', async () => {
    await browser.waitForSelector('h1', 5000);
  });

  it('injectStealth patches fingerprint', async () => {
    const fp = await browser.injectStealth();
    assert.ok(fp.platform, 'should have platform');
    assert.ok(fp.hardwareConcurrency > 0, 'should have hw concurrency');
    assert.ok(fp.deviceMemory > 0, 'should have device memory');

    // Verify stealth worked
    const webdriver = await browser.evaluate('String(navigator.webdriver)');
    assert.ok(webdriver.includes('false'), 'webdriver should be false');
  });

  it('setContent sets custom HTML', async () => {
    await browser.setContent('<html><body><h1 id="test">Custom</h1></body></html>');
    const text = await browser.getText('#test');
    assert.equal(text, 'Custom');
  });

  it('newPage creates a new tab', async () => {
    await browser.newPage('https://example.com');
    await browser.wait(500);
    const title = await browser.getTitle();
    assert.equal(title, 'Example Domain');
  });
});
