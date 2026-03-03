import { describe, it, after } from 'node:test';
import assert from 'node:assert/strict';
import { createRequire } from 'node:module';

const require = createRequire(import.meta.url);
const { NativeBrowser } = require('../index.js');

describe('NativeBrowser: Advanced operations', () => {
  let browser;

  after(async () => {
    if (browser) await browser.close();
  });

  it('launch and setup', async () => {
    browser = await NativeBrowser.launch(true);
    await browser.goto('https://example.com');
  });

  // ── Cookie Management ──

  it('getCookies returns JSON array', async () => {
    const cookiesJson = await browser.getCookies();
    const cookies = JSON.parse(cookiesJson);
    assert.ok(Array.isArray(cookies), 'should be an array');
  });

  it('setCookie + getCookies roundtrip', async () => {
    await browser.setCookie(JSON.stringify({
      name: 'test_cookie',
      value: 'hello123',
      domain: 'example.com',
      path: '/',
    }));
    const cookiesJson = await browser.getCookies();
    const cookies = JSON.parse(cookiesJson);
    const found = cookies.find(c => c.name === 'test_cookie');
    assert.ok(found, 'should find the set cookie');
    assert.equal(found.value, 'hello123');
  });

  it('deleteCookies removes specific cookie', async () => {
    await browser.deleteCookies('test_cookie', 'example.com');
    const cookiesJson = await browser.getCookies();
    const cookies = JSON.parse(cookiesJson);
    const found = cookies.find(c => c.name === 'test_cookie');
    assert.equal(found, undefined, 'cookie should be deleted');
  });

  it('clearCookies clears all', async () => {
    await browser.setCookie(JSON.stringify({
      name: 'a', value: '1', domain: 'example.com', path: '/',
    }));
    await browser.clearCookies();
    const cookiesJson = await browser.getCookies();
    const cookies = JSON.parse(cookiesJson);
    assert.equal(cookies.length, 0, 'should have no cookies');
  });

  // ── Keyboard ──

  it('pressKey does not throw', async () => {
    await browser.setContent('<input id="kb" type="text" autofocus />');
    await browser.click('#kb');
    await browser.pressKey('a');
  });

  it('keyboardShortcut does not throw', async () => {
    await browser.keyboardShortcut('Control+a');
  });

  it('keyDown + keyUp does not throw', async () => {
    await browser.keyDown('Shift');
    await browser.keyUp('Shift');
  });

  it('fill sets input value', async () => {
    await browser.setContent('<input id="f" type="text" />');
    await browser.fill('#f', 'filled_value');
    const val = await browser.evaluate("document.querySelector('#f').value");
    assert.ok(val.includes('filled_value'));
  });

  // ── Advanced Input ──

  it('boundingBox returns dimensions', async () => {
    await browser.setContent('<div id="box" style="width:200px;height:100px;">Box</div>');
    const json = await browser.boundingBox('#box');
    const box = JSON.parse(json);
    assert.ok(box.width > 0, 'should have positive width');
    assert.ok(box.height > 0, 'should have positive height');
    assert.ok('x' in box && 'y' in box, 'should have x,y coords');
  });

  it('tap does not throw on valid element', async () => {
    await browser.setContent('<button id="tapme">Tap</button>');
    await browser.tap('#tapme');
  });

  it('dragAndDrop does not throw', async () => {
    await browser.setContent(`
      <div id="src" draggable="true" style="width:50px;height:50px;background:red;">S</div>
      <div id="tgt" style="width:100px;height:100px;background:blue;">T</div>
    `);
    await browser.dragAndDrop('#src', '#tgt');
  });
});
