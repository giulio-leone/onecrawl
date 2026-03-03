import { describe, it, before, after } from 'node:test';
import assert from 'node:assert/strict';
import { NativeBrowser } from '../index.js';
import { writeFileSync, unlinkSync, existsSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';

describe('Task Scheduler', () => {
  let b;
  before(async () => { b = await NativeBrowser.launch(true); });
  after(async () => { if (b) await b.close(); });

  it('schedulerAddTask returns an id starting with task-', async () => {
    const schedule = JSON.stringify({ interval_ms: 1000, delay_ms: 0, max_runs: null });
    const id = await b.schedulerAddTask('test-nav', 'navigate', '{}', schedule);
    assert.ok(id.startsWith('task-'));
  });

  it('schedulerListTasks returns non-empty array after add', async () => {
    const raw = await b.schedulerListTasks();
    const tasks = JSON.parse(raw);
    assert.ok(Array.isArray(tasks));
    assert.ok(tasks.length >= 1);
  });

  it('schedulerGetStats returns stats with active count', async () => {
    const raw = await b.schedulerGetStats();
    const stats = JSON.parse(raw);
    assert.ok(typeof stats.active === 'number');
    assert.ok(typeof stats.total === 'number');
  });

  it('schedulerPauseTask pauses an existing task', async () => {
    const schedule = JSON.stringify({ interval_ms: 5000, delay_ms: 0, max_runs: null });
    const id = await b.schedulerAddTask('pause-me', 'extract', '{}', schedule);
    const ok = await b.schedulerPauseTask(id);
    assert.strictEqual(ok, true);
  });

  it('schedulerResumeTask resumes a paused task', async () => {
    const schedule = JSON.stringify({ interval_ms: 5000, delay_ms: 0, max_runs: null });
    const id = await b.schedulerAddTask('resume-me', 'crawl', '{}', schedule);
    await b.schedulerPauseTask(id);
    const ok = await b.schedulerResumeTask(id);
    assert.strictEqual(ok, true);
  });

  it('schedulerRemoveTask removes an existing task', async () => {
    const schedule = JSON.stringify({ interval_ms: 0, delay_ms: 0, max_runs: 1 });
    const id = await b.schedulerAddTask('remove-me', 'screenshot', '{}', schedule);
    const ok = await b.schedulerRemoveTask(id);
    assert.strictEqual(ok, true);
    const gone = await b.schedulerRemoveTask(id);
    assert.strictEqual(gone, false);
  });

  it('schedulerGetDueTasks returns due tasks', async () => {
    const schedule = JSON.stringify({ interval_ms: 1000, delay_ms: 0, max_runs: null });
    await b.schedulerAddTask('due-task', 'custom', '{}', schedule);
    const raw = await b.schedulerGetDueTasks();
    const due = JSON.parse(raw);
    assert.ok(Array.isArray(due));
  });

  it('schedulerSave and schedulerLoad round-trip', async () => {
    const tmp = join(tmpdir(), `sched-test-${Date.now()}.json`);
    try {
      await b.schedulerSave(tmp);
      assert.ok(existsSync(tmp));
      await b.schedulerLoad(tmp);
    } finally {
      if (existsSync(tmp)) unlinkSync(tmp);
    }
  });
});

describe('Session Pool', () => {
  let b;
  before(async () => { b = await NativeBrowser.launch(true); });
  after(async () => { if (b) await b.close(); });

  it('poolAddSession returns an id starting with sess-', async () => {
    const id = await b.poolAddSession('worker-1', null);
    assert.ok(id.startsWith('sess-'));
  });

  it('poolGetNext returns a session when idle exists', async () => {
    await b.poolAddSession('worker-2', null);
    const raw = await b.poolGetNext();
    assert.ok(raw !== null);
    const session = JSON.parse(raw);
    assert.ok(typeof session.id === 'string');
    assert.ok(typeof session.status === 'string');
  });

  it('poolGetStats returns valid stats', async () => {
    const raw = await b.poolGetStats();
    const stats = JSON.parse(raw);
    assert.ok(typeof stats.total === 'number');
    assert.ok(typeof stats.idle === 'number');
    assert.ok(typeof stats.busy === 'number');
  });

  it('poolMarkBusy changes session status', async () => {
    const id = await b.poolAddSession('busy-test', null);
    await b.poolMarkBusy(id);
    const raw = await b.poolGetStats();
    const stats = JSON.parse(raw);
    assert.ok(stats.busy >= 1);
  });

  it('poolMarkIdle reverts a busy session', async () => {
    const id = await b.poolAddSession('idle-test', null);
    await b.poolMarkBusy(id);
    await b.poolMarkIdle(id);
    // session should be idle again
    const raw = await b.poolGetStats();
    const stats = JSON.parse(raw);
    assert.ok(stats.idle >= 1);
  });

  it('poolCloseSession closes a session', async () => {
    const id = await b.poolAddSession('close-test', null);
    await b.poolCloseSession(id);
    const raw = await b.poolGetStats();
    const stats = JSON.parse(raw);
    assert.ok(stats.closed >= 1);
  });

  it('poolCleanupIdle returns count of cleaned sessions', async () => {
    const count = await b.poolCleanupIdle();
    assert.ok(typeof count === 'number');
  });

  it('poolSave and poolLoad round-trip', async () => {
    const tmp = join(tmpdir(), `pool-test-${Date.now()}.json`);
    try {
      await b.poolSave(tmp);
      assert.ok(existsSync(tmp));
      await b.poolLoad(tmp);
    } finally {
      if (existsSync(tmp)) unlinkSync(tmp);
    }
  });
});
