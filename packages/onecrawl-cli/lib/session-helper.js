'use strict';

const path = require('path');

/**
 * Shared utility for custom commands that need to interact with a running
 * Playwright CLI browser session.
 *
 * Uses Playwright's internal Registry + Session to locate and communicate
 * with the daemon process over its Unix/named-pipe socket.
 */

// Lazy-loaded Playwright internals (resolved once, cached)
let _Registry, _createClientInfo, _Session;

function _loadPlaywrightInternals() {
  if (_Registry) return;
  const clientDir = path.dirname(
    require.resolve('playwright/lib/cli/client/program')
  );
  const reg = require(path.join(clientDir, 'registry'));
  const sess = require(path.join(clientDir, 'session'));
  _Registry = reg.Registry;
  _createClientInfo = reg.createClientInfo;
  _Session = sess.Session;
}

/**
 * Resolve the session name from args or environment.
 * Mirrors Playwright's own resolveSessionName logic.
 *
 * @param {Object} [args] - Parsed minimist args (may contain .session)
 * @returns {string}
 */
function resolveSessionName(args) {
  if (args && args.session) return args.session;
  if (process.env.PLAYWRIGHT_CLI_SESSION) return process.env.PLAYWRIGHT_CLI_SESSION;
  return 'default';
}

/**
 * Get a connected Session object for the given (or default) session name.
 * Throws with a helpful message if the session is not running.
 *
 * @param {Object} [args] - Parsed minimist args
 * @returns {Promise<{session: Object, clientInfo: Object}>}
 */
async function getSession(args) {
  _loadPlaywrightInternals();

  const clientInfo = _createClientInfo();
  const registry = await _Registry.load();
  const sessionName = resolveSessionName(args);
  const entry = registry.entry(clientInfo, sessionName);

  if (!entry) {
    throw new Error(
      `Browser '${sessionName}' is not open. Run:\n\n` +
      `  onecrawl-cli${sessionName !== 'default' ? ` -s=${sessionName}` : ''} open\n\n` +
      `to start the browser session.`
    );
  }

  const session = new _Session(entry);
  const canConnect = await session.canConnect();
  if (!canConnect) {
    throw new Error(
      `Browser '${sessionName}' is not responding. Try:\n\n` +
      `  onecrawl-cli${sessionName !== 'default' ? ` -s=${sessionName}` : ''} open\n\n` +
      `to restart the browser session.`
    );
  }

  return { session, clientInfo };
}

/**
 * Send a command to the running Playwright session and return the result.
 * This is the primary way custom commands interact with the browser.
 *
 * @param {Object} args - Parsed minimist args (command args forwarded to session)
 * @returns {Promise<{text: string}>}
 */
async function runSessionCommand(args) {
  const { session, clientInfo } = await getSession(args);
  return await session.run(clientInfo, args);
}

/**
 * Execute an async function with proper error handling and process exit.
 * Wraps the common pattern: try action, catch → stderr + exit(1).
 *
 * @param {function(): Promise<void>} fn
 */
async function withErrorHandling(fn) {
  try {
    await fn();
  } catch (err) {
    console.error(`onecrawl: ${err.message}`);
    process.exit(1);
  }
}

/**
 * Get the path to the .playwright session directory for the current workspace.
 * @returns {string}
 */
function getSessionDir() {
  return path.resolve('.playwright');
}

module.exports = {
  resolveSessionName,
  getSession,
  runSessionCommand,
  withErrorHandling,
  getSessionDir,
};
