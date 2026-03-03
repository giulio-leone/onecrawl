import { describe, it, before, after } from 'node:test';
import assert from 'node:assert/strict';
import { NativeBrowser } from '../index.js';

describe('Console, Dialog, Workers, and Web Storage', () => {
  let b;
  before(async () => { b = await NativeBrowser.launch(true); });
  after(async () => { if (b) await b.close(); });

  // ── Console Interception ─────────────────────────────────────

  it('startConsoleCapture does not throw', async () => {
    await b.goto('data:text/html,<h1>Console</h1>');
    await assert.doesNotReject(() => b.startConsoleCapture());
  });

  it('drainConsoleEntries returns valid JSON array', async () => {
    await b.evaluate('console.log("hello from test")');
    const json = await b.drainConsoleEntries();
    const entries = JSON.parse(json);
    assert.ok(Array.isArray(entries));
  });

  it('drainConsoleEntries captures log messages', async () => {
    await b.evaluate('console.warn("warn msg")');
    const json = await b.drainConsoleEntries();
    const entries = JSON.parse(json);
    const warn = entries.find(e => e.level === 'warn');
    assert.ok(warn, 'should have a warn entry');
    assert.ok(warn.text.includes('warn msg'));
  });

  it('clearConsole does not throw', async () => {
    await assert.doesNotReject(() => b.clearConsole());
  });

  it('drainConsoleEntries returns empty after clear', async () => {
    const json = await b.drainConsoleEntries();
    const entries = JSON.parse(json);
    assert.equal(entries.length, 0);
  });

  // ── Dialog Handling ──────────────────────────────────────────

  it('setDialogHandler does not throw', async () => {
    await b.goto('data:text/html,<h1>Dialog</h1>');
    await assert.doesNotReject(() => b.setDialogHandler(true));
  });

  it('getDialogHistory returns valid JSON array', async () => {
    const json = await b.getDialogHistory();
    const events = JSON.parse(json);
    assert.ok(Array.isArray(events));
  });

  it('dialog handler records alert calls', async () => {
    await b.evaluate('alert("test alert")');
    const json = await b.getDialogHistory();
    const events = JSON.parse(json);
    const alertEvt = events.find(e => e.dialog_type === 'alert');
    assert.ok(alertEvt, 'should have an alert event');
    assert.ok(alertEvt.message.includes('test alert'));
  });

  it('clearDialogHistory does not throw', async () => {
    await assert.doesNotReject(() => b.clearDialogHistory());
  });

  // ── Service Workers ──────────────────────────────────────────

  it('getServiceWorkers returns valid JSON array', async () => {
    await b.goto('data:text/html,<h1>Workers</h1>');
    const json = await b.getServiceWorkers();
    const workers = JSON.parse(json);
    assert.ok(Array.isArray(workers));
  });

  it('unregisterServiceWorkers returns 0 when none registered', async () => {
    const count = await b.unregisterServiceWorkers();
    assert.equal(count, 0);
  });

  it('getWorkerInfo returns valid JSON', async () => {
    const json = await b.getWorkerInfo();
    const info = JSON.parse(json);
    assert.ok(typeof info === 'object');
  });

  // ── Web Storage ──────────────────────────────────────────────

  it('setLocalStorage and getLocalStorage round-trip', async () => {
    await b.goto('data:text/html,<h1>Storage</h1>');
    await b.setLocalStorage('testKey', 'testValue');
    const json = await b.getLocalStorage();
    const data = JSON.parse(json);
    assert.equal(data.testKey, 'testValue');
  });

  it('clearLocalStorage empties localStorage', async () => {
    await b.clearLocalStorage();
    const json = await b.getLocalStorage();
    const data = JSON.parse(json);
    assert.equal(Object.keys(data).length, 0);
  });

  it('setSessionStorage and getSessionStorage round-trip', async () => {
    await b.setSessionStorage('sessKey', 'sessVal');
    const json = await b.getSessionStorage();
    const data = JSON.parse(json);
    assert.equal(data.sessKey, 'sessVal');
  });

  it('clearSessionStorage empties sessionStorage', async () => {
    await b.clearSessionStorage();
    const json = await b.getSessionStorage();
    const data = JSON.parse(json);
    assert.equal(Object.keys(data).length, 0);
  });

  it('getIndexeddbDatabases returns valid JSON array', async () => {
    const json = await b.getIndexeddbDatabases();
    const names = JSON.parse(json);
    assert.ok(Array.isArray(names));
  });

  it('clearSiteData does not throw', async () => {
    await assert.doesNotReject(() => b.clearSiteData());
  });
});
