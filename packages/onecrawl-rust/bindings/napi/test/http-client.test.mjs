import { describe, it, before, after } from 'node:test';
import assert from 'node:assert/strict';
import { NativeBrowser } from '../index.js';

const JSON_HTML = `data:text/html,
<html><body>
  <script>
    window.addEventListener('load', () => {
      window.__testData = { greeting: "hello", count: 42 };
    });
  </script>
  <p>Page with JS data</p>
</body></html>`;

describe('HTTP Client (browser fetch)', () => {
  let b;
  before(async () => { b = await NativeBrowser.launch(true); });
  after(async () => { if (b) await b.close(); });

  it('httpGet returns response with status', async () => {
    await b.goto('https://httpbin.org/get');
    const resp = JSON.parse(await b.httpGet('https://httpbin.org/get'));
    assert.equal(resp.status, 200);
    assert.equal(typeof resp.body, 'string');
    assert.ok(resp.body.length > 0);
    assert.equal(resp.redirected, false);
    assert.ok(resp.duration_ms >= 0);
  });

  it('httpPost sends body and returns response', async () => {
    await b.goto('https://httpbin.org/post');
    const resp = JSON.parse(
      await b.httpPost('https://httpbin.org/post', '{"key":"value"}', 'application/json')
    );
    assert.equal(resp.status, 200);
    const body = JSON.parse(resp.body);
    assert.equal(body.json.key, 'value');
  });

  it('httpHead returns headers without body', async () => {
    await b.goto('https://httpbin.org/get');
    const resp = JSON.parse(await b.httpHead('https://httpbin.org/get'));
    assert.equal(resp.status, 200);
    assert.equal(resp.body, '');
  });

  it('httpFetch with custom request', async () => {
    await b.goto('https://httpbin.org/get');
    const req = JSON.stringify({
      url: 'https://httpbin.org/headers',
      method: 'GET',
      headers: { 'X-Custom': 'test-value' },
      body: null,
      timeout_ms: 10000,
    });
    const resp = JSON.parse(await b.httpFetch(req));
    assert.equal(resp.status, 200);
    const body = JSON.parse(resp.body);
    assert.equal(body.headers['X-Custom'], 'test-value');
  });

  it('httpFetchJson parses JSON response', async () => {
    await b.goto('https://httpbin.org/get');
    const json = JSON.parse(await b.httpFetchJson('https://httpbin.org/get'));
    assert.ok(json.url);
    assert.ok(json.headers);
  });

  it('httpGet with custom headers', async () => {
    await b.goto('https://httpbin.org/get');
    const headers = JSON.stringify({ 'Accept-Language': 'it-IT' });
    const resp = JSON.parse(await b.httpGet('https://httpbin.org/headers', headers));
    assert.equal(resp.status, 200);
    const body = JSON.parse(resp.body);
    assert.equal(body.headers['Accept-Language'], 'it-IT');
  });

  it('httpGet returns correct url field', async () => {
    await b.goto('https://httpbin.org/get');
    const resp = JSON.parse(await b.httpGet('https://httpbin.org/get'));
    assert.ok(resp.url.includes('httpbin.org'));
  });

  it('httpGet has status_text', async () => {
    await b.goto('https://httpbin.org/get');
    const resp = JSON.parse(await b.httpGet('https://httpbin.org/get'));
    assert.equal(typeof resp.status_text, 'string');
  });
});
