import { describe, it, before, after } from 'node:test';
import assert from 'node:assert/strict';
import { NativeBrowser } from '../index.js';

describe('Geofencing', () => {
  let b;
  before(async () => { b = await NativeBrowser.launch(true); });
  after(async () => { if (b) await b.close(); });

  it('listGeoPresets returns preset names', () => {
    const presets = b.listGeoPresets();
    assert.ok(Array.isArray(presets));
    assert.ok(presets.length >= 8);
    assert.ok(presets.includes('New York'));
    assert.ok(presets.includes('London'));
    assert.ok(presets.includes('Tokyo'));
  });

  it('getGeoPreset returns profile JSON for valid name', () => {
    const json = b.getGeoPreset('New York');
    assert.ok(json);
    const profile = JSON.parse(json);
    assert.equal(profile.name, 'New York');
    assert.equal(profile.latitude, 40.7128);
    assert.equal(profile.timezone, 'America/New_York');
  });

  it('getGeoPreset is case-insensitive', () => {
    const json = b.getGeoPreset('new york');
    assert.ok(json);
    const profile = JSON.parse(json);
    assert.equal(profile.name, 'New York');
  });

  it('getGeoPreset returns null for unknown name', () => {
    const result = b.getGeoPreset('Atlantis');
    assert.equal(result, null);
  });

  it('applyGeoProfile does not throw with preset JSON', async () => {
    await b.goto('data:text/html,<h1>Geo</h1>');
    const preset = b.getGeoPreset('Tokyo');
    await assert.doesNotReject(() => b.applyGeoProfile(preset));
  });
});

describe('Cookie Jar', () => {
  let b;
  before(async () => { b = await NativeBrowser.launch(true); });
  after(async () => { if (b) await b.close(); });

  it('exportCookies returns valid JSON', async () => {
    await b.goto('data:text/html,<h1>CookieJar</h1>');
    const json = await b.exportCookies();
    const jar = JSON.parse(json);
    assert.ok(jar.version);
    assert.ok(Array.isArray(jar.cookies));
    assert.ok(jar.exported_at);
  });

  it('importCookies returns count', async () => {
    const jar = JSON.stringify({
      cookies: [{
        name: 'test_ck', value: 'abc123', domain: 'localhost',
        path: '/', expires: 0, http_only: false, secure: false, same_site: null
      }],
      domain: null, exported_at: '0', version: '1.0'
    });
    const count = await b.importCookies(jar);
    assert.equal(count, 1);
  });

  it('clearAllCookies does not throw', async () => {
    await assert.doesNotReject(() => b.clearAllCookies());
  });

  it('saveCookiesToFile writes a file', async () => {
    const fs = await import('node:fs');
    const path = '/tmp/onecrawl-test-cookies.json';
    const count = await b.saveCookiesToFile(path);
    assert.ok(typeof count === 'number');
    assert.ok(fs.existsSync(path));
    fs.unlinkSync(path);
  });

  it('loadCookiesFromFile reads a file', async () => {
    const fs = await import('node:fs');
    const path = '/tmp/onecrawl-test-cookies-load.json';
    const jar = { cookies: [], domain: null, exported_at: '0', version: '1.0' };
    fs.writeFileSync(path, JSON.stringify(jar));
    const count = await b.loadCookiesFromFile(path);
    assert.equal(count, 0);
    fs.unlinkSync(path);
  });
});

describe('Request Queue', () => {
  let b;
  before(async () => { b = await NativeBrowser.launch(true); });
  after(async () => { if (b) await b.close(); });

  it('createGetRequest returns valid JSON', () => {
    const json = b.createGetRequest('r1', 'https://example.com');
    const req = JSON.parse(json);
    assert.equal(req.id, 'r1');
    assert.equal(req.method, 'GET');
    assert.equal(req.url, 'https://example.com');
    assert.equal(req.max_retries, 3);
  });

  it('createPostRequest returns valid JSON', () => {
    const json = b.createPostRequest('r2', 'https://example.com/api', '{"key":"val"}');
    const req = JSON.parse(json);
    assert.equal(req.id, 'r2');
    assert.equal(req.method, 'POST');
    assert.ok(req.headers['Content-Type']);
    assert.equal(req.body, '{"key":"val"}');
  });

  it('executeRequest returns a result', async () => {
    await b.goto('data:text/html,<h1>Queue</h1>');
    const req = b.createGetRequest('test-req', 'data:text/html,hello');
    const json = await b.executeRequest(req);
    const result = JSON.parse(json);
    assert.ok(result.id);
    assert.ok(typeof result.attempts === 'number');
    assert.ok(typeof result.duration_ms === 'number');
  });

  it('executeBatch returns array of results', async () => {
    const reqs = JSON.stringify([
      JSON.parse(b.createGetRequest('b1', 'data:text/html,one')),
      JSON.parse(b.createGetRequest('b2', 'data:text/html,two')),
    ]);
    const json = await b.executeBatch(reqs);
    const results = JSON.parse(json);
    assert.ok(Array.isArray(results));
    assert.equal(results.length, 2);
  });

  it('executeBatch accepts custom config', async () => {
    const reqs = JSON.stringify([
      JSON.parse(b.createGetRequest('c1', 'data:text/html,cfg')),
    ]);
    const config = JSON.stringify({ concurrency: 1, delay_between_ms: 0, default_timeout_ms: 5000, default_max_retries: 1, default_retry_delay_ms: 100 });
    const json = await b.executeBatch(reqs, config);
    const results = JSON.parse(json);
    assert.ok(Array.isArray(results));
  });
});
