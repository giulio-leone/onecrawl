import { describe, it, after } from 'node:test';
import assert from 'node:assert/strict';
import { createRequire } from 'node:module';

const require = createRequire(import.meta.url);
const { NativeBrowser } = require('../index.js');

const eval_ = async (browser, expr) => JSON.parse(await browser.evaluate(expr));

// ── Proxy Pool (no browser needed) ──────────────────────────────

describe('NativeBrowser: Proxy Pool', () => {
  let browser;

  after(async () => {
    if (browser) await browser.close();
  });

  it('launch headless', async () => {
    browser = await NativeBrowser.launch(true);
  });

  it('createProxyPool round-robin', () => {
    const config = JSON.stringify({
      proxies: [
        { server: 'http://proxy1:8080', username: null, password: null, bypass: null },
        { server: 'http://proxy2:8080', username: 'user', password: 'pass', bypass: 'localhost' },
      ],
      strategy: 'RoundRobin',
      current_index: 0,
    });
    const result = browser.createProxyPool(config);
    const pool = JSON.parse(result);
    assert.equal(pool.proxies.length, 2);
    assert.equal(pool.strategy, 'RoundRobin');
  });

  it('getProxyChromeArgs returns proper args', () => {
    const pool = JSON.stringify({
      proxies: [{ server: 'http://proxy1:8080', username: null, password: null, bypass: 'localhost,127.0.0.1' }],
      strategy: 'Sticky',
      current_index: 0,
    });
    const args = browser.getProxyChromeArgs(pool);
    assert.ok(args.some(a => a.includes('--proxy-server=http://proxy1:8080')));
    assert.ok(args.some(a => a.includes('--proxy-bypass-list=localhost,127.0.0.1')));
  });

  it('getProxyChromeArgs empty pool returns empty', () => {
    const pool = JSON.stringify({ proxies: [], strategy: 'RoundRobin', current_index: 0 });
    const args = browser.getProxyChromeArgs(pool);
    assert.equal(args.length, 0);
  });

  it('nextProxy advances index', () => {
    const pool = JSON.stringify({
      proxies: [
        { server: 'http://p1:80', username: null, password: null, bypass: null },
        { server: 'http://p2:80', username: null, password: null, bypass: null },
      ],
      strategy: 'RoundRobin',
      current_index: 0,
    });
    const updated = JSON.parse(browser.nextProxy(pool));
    assert.equal(updated.current_index, 1);
  });

  it('createProxyPool rejects invalid JSON', () => {
    assert.throws(() => browser.createProxyPool('not json'), /expected/i);
  });
});

// ── Request Interception ────────────────────────────────────────

describe('NativeBrowser: Request Interception', () => {
  let browser;

  after(async () => {
    if (browser) await browser.close();
  });

  it('launch and navigate', async () => {
    browser = await NativeBrowser.launch(true);
    await browser.goto('https://example.com');
  });

  it('setInterceptRules accepts valid rules', async () => {
    const rules = JSON.stringify([
      { url_pattern: '*blocked*', resource_type: null, action: 'Block' },
    ]);
    await browser.setInterceptRules(rules);
  });

  it('getInterceptedRequests returns array', async () => {
    const log = JSON.parse(await browser.getInterceptedRequests());
    assert.ok(Array.isArray(log));
  });

  it('clearInterceptRules succeeds', async () => {
    await browser.clearInterceptRules();
    const log = JSON.parse(await browser.getInterceptedRequests());
    assert.equal(log.length, 0);
  });

  it('setInterceptRules rejects invalid JSON', async () => {
    await assert.rejects(() => browser.setInterceptRules('bad'), /expected/i);
  });
});

// ── Advanced Emulation ──────────────────────────────────────────

describe('NativeBrowser: Advanced Emulation', () => {
  let browser;

  after(async () => {
    if (browser) await browser.close();
  });

  it('launch and navigate', async () => {
    browser = await NativeBrowser.launch(true);
    await browser.goto('https://example.com');
  });

  it('setDeviceOrientation overrides orientation', async () => {
    await browser.setDeviceOrientation(45.0, 90.0, 0.0);
    assert.ok(true);
  });

  it('overridePermission sets geolocation to granted', async () => {
    await browser.overridePermission('geolocation', 'granted');
    const state = await eval_(browser, `
      navigator.permissions.query({ name: 'geolocation' }).then(r => r.state)
    `);
    assert.equal(state, 'granted');
  });

  it('setBatteryStatus overrides battery', async () => {
    await browser.setBatteryStatus(0.75, false);
    const level = await eval_(browser, 'navigator.getBattery().then(b => b.level)');
    assert.equal(level, 0.75);
  });

  it('setConnectionInfo overrides connection', async () => {
    await browser.setConnectionInfo('4g', 10.0, 50);
    const type_ = await eval_(browser, 'navigator.connection.effectiveType');
    assert.equal(type_, '4g');
  });

  it('setHardwareConcurrency overrides CPU cores', async () => {
    await browser.setHardwareConcurrency(16);
    const cores = await eval_(browser, 'navigator.hardwareConcurrency');
    assert.equal(cores, 16);
  });

  it('setDeviceMemory overrides memory', async () => {
    await browser.setDeviceMemory(32.0);
    const mem = await eval_(browser, 'navigator.deviceMemory');
    assert.equal(mem, 32);
  });

  it('getNavigatorInfo returns JSON with expected keys', async () => {
    const info = JSON.parse(await browser.getNavigatorInfo());
    assert.ok('userAgent' in info);
    assert.ok('platform' in info);
    assert.ok('hardwareConcurrency' in info);
    assert.ok('deviceMemory' in info);
  });
});
