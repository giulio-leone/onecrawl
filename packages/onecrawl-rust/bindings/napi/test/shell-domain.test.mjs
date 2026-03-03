import { describe, it, before, after } from 'node:test';
import assert from 'node:assert/strict';
import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import { NativeBrowser } from '../index.js';

describe('Shell', () => {
  let b;
  before(async () => { b = await NativeBrowser.launch(true); });
  after(async () => { if (b) await b.close(); });

  it('shellParse returns valid JSON with command and args', () => {
    const json = b.shellParse('goto https://example.com');
    const cmd = JSON.parse(json);
    assert.equal(cmd.command, 'goto');
    assert.deepEqual(cmd.args, ['https://example.com']);
    assert.equal(typeof cmd.raw, 'string');
    assert.equal(typeof cmd.timestamp, 'number');
  });

  it('shellParse handles empty input', () => {
    const json = b.shellParse('');
    const cmd = JSON.parse(json);
    assert.equal(cmd.command, '');
    assert.deepEqual(cmd.args, []);
  });

  it('shellParse handles multiple arguments', () => {
    const json = b.shellParse('type #input hello world');
    const cmd = JSON.parse(json);
    assert.equal(cmd.command, 'type');
    assert.deepEqual(cmd.args, ['#input', 'hello', 'world']);
  });

  it('shellCommands returns non-empty array', () => {
    const json = b.shellCommands();
    const cmds = JSON.parse(json);
    assert.ok(Array.isArray(cmds));
    assert.ok(cmds.length > 10);
    const names = cmds.map(c => c[0]);
    assert.ok(names.some(n => n.includes('goto')));
    assert.ok(names.some(n => n.includes('exit')));
  });

  it('shellSaveHistory + shellLoadHistory round-trips', () => {
    const history = {
      commands: [
        { raw: 'goto https://example.com', command: 'goto', args: ['https://example.com'], timestamp: 1000 },
      ],
      max_size: 100,
    };
    const tmp = path.join(os.tmpdir(), `onecrawl-shell-test-${Date.now()}.json`);
    try {
      b.shellSaveHistory(JSON.stringify(history), tmp);
      assert.ok(fs.existsSync(tmp));
      const loaded = JSON.parse(b.shellLoadHistory(tmp));
      assert.equal(loaded.commands.length, 1);
      assert.equal(loaded.commands[0].command, 'goto');
      assert.equal(loaded.max_size, 100);
    } finally {
      if (fs.existsSync(tmp)) fs.unlinkSync(tmp);
    }
  });
});

describe('Domain Blocker', () => {
  let b;
  before(async () => { b = await NativeBrowser.launch(true); });
  after(async () => { if (b) await b.close(); });

  it('blockDomains installs blocker and returns count', async () => {
    await b.goto('data:text/html,<h1>Block</h1>');
    const count = await b.blockDomains(JSON.stringify(['evil.com', 'tracker.io']));
    assert.ok(count >= 2);
  });

  it('listBlocked returns the blocked domains', async () => {
    const json = await b.listBlocked();
    const domains = JSON.parse(json);
    assert.ok(Array.isArray(domains));
    assert.ok(domains.includes('evil.com'));
    assert.ok(domains.includes('tracker.io'));
  });

  it('blockCategory adds category domains', async () => {
    const count = await b.blockCategory('ads');
    assert.ok(count > 10);
  });

  it('blockStats returns valid stats object', async () => {
    const json = await b.blockStats();
    const stats = JSON.parse(json);
    assert.equal(typeof stats.total_blocked, 'number');
    assert.ok(Array.isArray(stats.domains));
  });

  it('availableBlockCategories returns categories with counts', () => {
    const json = b.availableBlockCategories();
    const cats = JSON.parse(json);
    assert.ok(Array.isArray(cats));
    assert.ok(cats.length >= 5);
    const names = cats.map(c => c[0]);
    assert.ok(names.includes('ads'));
    assert.ok(names.includes('trackers'));
  });

  it('clearBlocks removes all blocks', async () => {
    await b.clearBlocks();
    const json = await b.listBlocked();
    const domains = JSON.parse(json);
    assert.equal(domains.length, 0);
  });
});
