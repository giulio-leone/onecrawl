'use strict';

const test = require('node:test');
const assert = require('node:assert');
const path = require('node:path');
const fs = require('node:fs');

const LIB_DIR = path.resolve(__dirname, '..', '..', 'lib');
const COMMANDS_DIR = path.join(LIB_DIR, 'commands');

// All command files (everything in commands/ except index.js)
const COMMAND_FILES = fs.readdirSync(COMMANDS_DIR)
  .filter(f => f.endsWith('.js') && f !== 'index.js')
  .sort();

const EXPECTED_COMMANDS = [
  'add-init-script',
  'add-script',
  'add-style',
  'assert',
  'auth',
  'click',
  'clipboard',
  'console',
  'cookie',
  'device',
  'dialog',
  'diff-screenshot',
  'diff-snapshot',
  'diff-url',
  'drag',
  'emulate-media',
  'extract',
  'find',
  'forms',
  'frame',
  'geolocation',
  'get',
  'get-box',
  'get-styles',
  'har',
  'headers',
  'health-check',
  'hover',
  'http-credentials',
  'is',
  'js-errors',
  'keyboard',
  'links',
  'locale',
  'mainframe',
  'offline',
  'pdf',
  'permissions',
  'profiler',
  'ptc',
  'recording',
  'requests',
  'route',
  'screencast',
  'screenshot-annotate',
  'scroll',
  'select',
  'session',
  'session-info',
  'set-content',
  'storage',
  'storage-state',
  'tab',
  'table',
  'tap',
  'timezone',
  'trace',
  'type',
  'unroute',
  'user-agent',
  'viewport',
  'wait-for',
  'wait-for-function',
];

// ── Test 1: All command files load without error ────────────────────────────

test('all command files load without error', () => {
  for (const file of COMMAND_FILES) {
    const filePath = path.join(COMMANDS_DIR, file);
    assert.doesNotThrow(
      () => require(filePath),
      `Failed to load ${file}`
    );
  }
});

// ── Test 2: Each command file exports { register: Function } ────────────────

test('each command file exports register()', () => {
  for (const file of COMMAND_FILES) {
    const mod = require(path.join(COMMANDS_DIR, file));
    assert.ok(mod.register, `${file} does not export register`);
    assert.strictEqual(
      typeof mod.register,
      'function',
      `${file}.register is not a function`
    );
  }
});

// ── Test 3: Registry discovers all 63 commands ──────────────────────────────

test('registry discovers all 63 commands', () => {
  // Clear require cache for commands/index.js to get a fresh registry
  const indexPath = path.join(COMMANDS_DIR, 'index.js');
  delete require.cache[indexPath];

  const { loadAllCommands } = require(indexPath);
  const registry = loadAllCommands();

  assert.strictEqual(
    registry.all().length,
    63,
    `Expected 63 commands, got ${registry.all().length}: [${registry.all().map(c => c.name).join(', ')}]`
  );
});

// ── Test 4: Registry returns correct command names ──────────────────────────

test('registry returns correct command names', () => {
  const { loadAllCommands } = require(path.join(COMMANDS_DIR, 'index.js'));
  const registry = loadAllCommands();
  const names = registry.all().map(c => c.name).sort();

  assert.deepStrictEqual(names, EXPECTED_COMMANDS);
});

// ── Test 5: Each command has name, description, and action ──────────────────

test('each command has name, description, and action', () => {
  const { loadAllCommands } = require(path.join(COMMANDS_DIR, 'index.js'));
  const registry = loadAllCommands();

  for (const cmd of registry.all()) {
    assert.ok(cmd.name, 'command missing name');
    assert.ok(cmd.description, `${cmd.name} missing description`);
    assert.strictEqual(typeof cmd.action, 'function', `${cmd.name}.action is not a function`);
  }
});

// ── Test 6: Session helper exports required functions ───────────────────────

test('session-helper exports required functions', () => {
  const helper = require(path.join(LIB_DIR, 'session-helper'));
  const expectedExports = [
    'resolveSessionName',
    'getSession',
    'runSessionCommand',
    'withErrorHandling',
    'getSessionDir',
  ];

  for (const name of expectedExports) {
    assert.ok(helper[name], `session-helper missing export: ${name}`);
    assert.strictEqual(
      typeof helper[name],
      'function',
      `session-helper.${name} is not a function`
    );
  }
});

// ── Test 7: resolveSessionName defaults and overrides ───────────────────────

test('resolveSessionName returns "default" with no args', () => {
  const { resolveSessionName } = require(path.join(LIB_DIR, 'session-helper'));
  const saved = process.env.PLAYWRIGHT_CLI_SESSION;
  delete process.env.PLAYWRIGHT_CLI_SESSION;

  assert.strictEqual(resolveSessionName(), 'default');
  assert.strictEqual(resolveSessionName({}), 'default');

  if (saved !== undefined) process.env.PLAYWRIGHT_CLI_SESSION = saved;
});

test('resolveSessionName respects args.session', () => {
  const { resolveSessionName } = require(path.join(LIB_DIR, 'session-helper'));
  assert.strictEqual(resolveSessionName({ session: 'mysession' }), 'mysession');
});

test('resolveSessionName respects env var', () => {
  const { resolveSessionName } = require(path.join(LIB_DIR, 'session-helper'));
  const saved = process.env.PLAYWRIGHT_CLI_SESSION;
  process.env.PLAYWRIGHT_CLI_SESSION = 'envsession';

  assert.strictEqual(resolveSessionName(), 'envsession');
  assert.strictEqual(resolveSessionName({}), 'envsession');

  if (saved !== undefined) process.env.PLAYWRIGHT_CLI_SESSION = saved;
  else delete process.env.PLAYWRIGHT_CLI_SESSION;
});

// ── Test 8: CommandRegistry add/has/get/all ─────────────────────────────────

test('CommandRegistry add/has/get/all work correctly', () => {
  const { CommandRegistry } = require(path.join(COMMANDS_DIR, 'index.js'));
  const reg = new CommandRegistry();

  const def = { name: 'test-cmd', description: 'test', action: async () => {} };
  reg.add(def);

  assert.ok(reg.has('test-cmd'));
  assert.ok(!reg.has('nonexistent'));
  assert.strictEqual(reg.get('test-cmd'), def);
  assert.strictEqual(reg.get('nonexistent'), undefined);
  assert.deepStrictEqual(reg.all(), [def]);
});

test('CommandRegistry rejects invalid definitions', () => {
  const { CommandRegistry } = require(path.join(COMMANDS_DIR, 'index.js'));
  const reg = new CommandRegistry();

  assert.throws(() => reg.add({}), /name and action are required/);
  assert.throws(() => reg.add({ name: 'x' }), /name and action are required/);
});

test('CommandRegistry rejects duplicates', () => {
  const { CommandRegistry } = require(path.join(COMMANDS_DIR, 'index.js'));
  const reg = new CommandRegistry();
  const def = { name: 'dup', description: 'test', action: async () => {} };

  reg.add(def);
  assert.throws(() => reg.add(def), /Duplicate command/);
});

// ── Test 9: formatCustomHelp output ─────────────────────────────────────────

test('formatCustomHelp includes all command names', () => {
  const { loadAllCommands, formatCustomHelp } = require(path.join(COMMANDS_DIR, 'index.js'));
  const registry = loadAllCommands();
  const help = formatCustomHelp(registry);

  assert.ok(help.includes('OneCrawl commands:'));
  for (const name of EXPECTED_COMMANDS) {
    assert.ok(help.includes(name), `help text missing command: ${name}`);
  }
});

test('formatCustomHelp returns empty string for empty registry', () => {
  const { CommandRegistry, formatCustomHelp } = require(path.join(COMMANDS_DIR, 'index.js'));
  assert.strictEqual(formatCustomHelp(new CommandRegistry()), '');
});

// ── Test 10: getSessionDir returns a path ───────────────────────────────────

test('getSessionDir returns a .playwright path', () => {
  const { getSessionDir } = require(path.join(LIB_DIR, 'session-helper'));
  const dir = getSessionDir();
  assert.ok(dir.endsWith('.playwright'), `Expected .playwright dir, got: ${dir}`);
});
