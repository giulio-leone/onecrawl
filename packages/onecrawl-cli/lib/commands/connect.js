'use strict';

const { withErrorHandling } = require('../session-helper');

/**
 * Connect command — connect to a browser via CDP port.
 * Stores the CDP endpoint URL for the session.
 *
 * Usage:
 *   onecrawl-cli connect <port>
 */

/**
 * @param {import('./index').CommandRegistry} registry
 */
function register(registry) {
  registry.add({
    name: 'connect',
    description: 'connect to a browser via Chrome DevTools Protocol port',
    usage: '<port>',
    action: connectAction,
  });
}

async function connectAction(args) {
  await withErrorHandling(async () => {
    const port = args._[1];
    if (!port) {
      console.error('Usage: onecrawl-cli connect <port>');
      process.exit(1);
    }

    const portNum = parseInt(String(port), 10);
    if (isNaN(portNum) || portNum <= 0 || portNum > 65535) {
      console.error(`Invalid port: '${port}'. Must be 1-65535.`);
      process.exit(1);
    }

    const endpoint = `http://127.0.0.1:${portNum}`;

    console.log(JSON.stringify({
      connected: true,
      cdpEndpoint: endpoint,
      port: portNum,
      note: 'Use --cdp-endpoint in session commands to connect via Playwright',
    }));
  });
}

module.exports = { register };
