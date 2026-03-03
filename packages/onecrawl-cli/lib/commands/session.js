'use strict';

/**
 * session command — manage saved browser sessions.
 *
 * Usage:
 *   onecrawl-cli session list
 *   onecrawl-cli session save <name>
 *   onecrawl-cli session restore <name>
 *   onecrawl-cli session delete <name>
 *   onecrawl-cli session clone <new-name>
 *
 * Saved sessions are stored as JSON files in ~/.onecrawl/sessions/.
 *
 * @module commands/session
 */

const { runSessionCommand, withErrorHandling } = require('../session-helper');
const fs = require('node:fs');
const path = require('node:path');
const os = require('node:os');

const SESSIONS_DIR = path.join(os.homedir(), '.onecrawl', 'sessions');

function ensureSessionsDir() {
  fs.mkdirSync(SESSIONS_DIR, { recursive: true });
}

function sessionFilePath(name) {
  return path.join(SESSIONS_DIR, `${name}.json`);
}

/**
 * @param {import('./index').CommandRegistry} registry
 */
function register(registry) {
  registry.add({
    name: 'session',
    description: 'manage saved browser sessions (list|save|restore|delete|clone)',
    usage: '<sub-command> [name]',
    action: sessionAction,
  });
}

/**
 * @param {Object} args - Parsed minimist args
 */
async function sessionAction(args) {
  await withErrorHandling(async () => {
    const subCommand = args._[1];

    if (!subCommand) {
      console.error(
        'Usage:\n' +
        '  onecrawl-cli session list\n' +
        '  onecrawl-cli session save <name>\n' +
        '  onecrawl-cli session restore <name>\n' +
        '  onecrawl-cli session delete <name>\n' +
        '  onecrawl-cli session clone <new-name>'
      );
      process.exit(1);
    }

    switch (subCommand) {
      case 'list':
        return await listSessions();
      case 'save':
        return await saveSession(args);
      case 'restore':
        return await restoreSession(args);
      case 'delete':
        return await deleteSession(args);
      case 'clone':
        return await saveSession(args); // clone is an alias for save
      default:
        throw new Error(`Unknown session sub-command: '${subCommand}'. Use list, save, restore, delete, or clone.`);
    }
  });
}

async function listSessions() {
  ensureSessionsDir();

  let files;
  try {
    files = fs.readdirSync(SESSIONS_DIR).filter(f => f.endsWith('.json'));
  } catch {
    files = [];
  }

  const sessions = files.map(file => {
    const filePath = path.join(SESSIONS_DIR, file);
    try {
      const data = JSON.parse(fs.readFileSync(filePath, 'utf8'));
      return {
        name: path.basename(file, '.json'),
        savedAt: data.savedAt || null,
        url: data.url || null,
        cookieCount: Array.isArray(data.cookies) ? data.cookies.length : 0,
      };
    } catch {
      return {
        name: path.basename(file, '.json'),
        savedAt: null,
        url: null,
        cookieCount: 0,
      };
    }
  });

  console.log(JSON.stringify(sessions, null, 2));
}

async function saveSession(args) {
  const name = args._[2];
  if (!name) {
    throw new Error('Session name is required. Usage: onecrawl-cli session save <name>');
  }

  ensureSessionsDir();

  // Gather current browser state
  const urlResult = await runSessionCommand({
    _: ['evaluate', 'window.location.href'],
    session: args.session,
  });

  const cookiesResult = await runSessionCommand({
    _: ['evaluate', `JSON.stringify(document.cookie.split(';').map(c => c.trim()).filter(Boolean))`],
    session: args.session,
  });

  const localStorageResult = await runSessionCommand({
    _: ['evaluate', `JSON.stringify(Object.entries(localStorage).reduce((o,[k,v])=>{o[k]=v;return o;}, {}))`],
    session: args.session,
  });

  const viewportResult = await runSessionCommand({
    _: ['evaluate', `JSON.stringify({width: window.innerWidth, height: window.innerHeight})`],
    session: args.session,
  });

  let cookies = [];
  try { cookies = JSON.parse(cookiesResult.text); } catch { /* keep empty */ }

  let localStorage = {};
  try { localStorage = JSON.parse(localStorageResult.text); } catch { /* keep empty */ }

  let viewport = {};
  try { viewport = JSON.parse(viewportResult.text); } catch { /* keep empty */ }

  const sessionData = {
    name,
    savedAt: new Date().toISOString(),
    url: urlResult.text,
    cookies,
    localStorage,
    viewport,
  };

  const filePath = sessionFilePath(name);
  fs.writeFileSync(filePath, JSON.stringify(sessionData, null, 2), 'utf8');

  console.log(JSON.stringify({ saved: true, name, path: filePath }));
}

async function restoreSession(args) {
  const name = args._[2];
  if (!name) {
    throw new Error('Session name is required. Usage: onecrawl-cli session restore <name>');
  }

  const filePath = sessionFilePath(name);
  if (!fs.existsSync(filePath)) {
    throw new Error(`Saved session '${name}' not found at ${filePath}`);
  }

  const data = JSON.parse(fs.readFileSync(filePath, 'utf8'));

  // Navigate to saved URL
  if (data.url) {
    await runSessionCommand({
      _: ['navigate', data.url],
      session: args.session,
    });
  }

  // Inject cookies
  if (Array.isArray(data.cookies)) {
    for (const cookie of data.cookies) {
      if (cookie) {
        await runSessionCommand({
          _: ['evaluate', `document.cookie = ${JSON.stringify(cookie)}`],
          session: args.session,
        });
      }
    }
  }

  // Restore localStorage
  if (data.localStorage && typeof data.localStorage === 'object') {
    const lsEntries = JSON.stringify(data.localStorage);
    await runSessionCommand({
      _: ['evaluate', `(() => { const e = ${lsEntries}; Object.entries(e).forEach(([k,v]) => localStorage.setItem(k,v)); return 'ok'; })()`],
      session: args.session,
    });
  }

  console.log(JSON.stringify({ restored: true, name, url: data.url || '' }));
}

async function deleteSession(args) {
  const name = args._[2];
  if (!name) {
    throw new Error('Session name is required. Usage: onecrawl-cli session delete <name>');
  }

  const filePath = sessionFilePath(name);
  if (!fs.existsSync(filePath)) {
    throw new Error(`Saved session '${name}' not found at ${filePath}`);
  }

  fs.unlinkSync(filePath);
  console.log(JSON.stringify({ deleted: true, name }));
}

module.exports = { register };
