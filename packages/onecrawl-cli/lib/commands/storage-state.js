'use strict';

/**
 * storage-state command — save / load full browser storage state.
 *
 * Usage:
 *   onecrawl-cli storage-state save <file>
 *   onecrawl-cli storage-state load <file>
 *
 * Exports cookies, localStorage and sessionStorage to a JSON file and
 * can re-import them later to restore session state.
 *
 * @module commands/storage-state
 */

const fs = require('node:fs');
const path = require('node:path');
const { runSessionCommand, withErrorHandling } = require('../session-helper');

/**
 * @param {import('./index').CommandRegistry} registry
 */
function register(registry) {
  registry.add({
    name: 'storage-state',
    description: 'save or load browser storage state (cookies + storage)',
    usage: 'save|load <file>',
    action: storageStateAction,
  });
}

/**
 * @param {Object} args - Parsed minimist args
 */
async function storageStateAction(args) {
  await withErrorHandling(async () => {
    const sub = args._[1];
    const file = args._[2];

    if (!sub || !['save', 'load'].includes(sub) || !file) {
      console.error('Usage: onecrawl-cli storage-state save|load <file>');
      process.exit(1);
    }

    const filePath = path.resolve(file);

    if (sub === 'save') {
      const js = `(() => {
        const state = {
          cookies: document.cookie.split(';').filter(Boolean).map(function(c) {
            const parts = c.split('=');
            const name = (parts.shift() || '').trim();
            const value = parts.join('=').trim();
            return { name: name, value: value };
          }),
          localStorage: Object.fromEntries(
            Object.keys(localStorage).map(function(k) { return [k, localStorage.getItem(k)]; })
          ),
          sessionStorage: Object.fromEntries(
            Object.keys(sessionStorage).map(function(k) { return [k, sessionStorage.getItem(k)]; })
          ),
          url: window.location.href,
          timestamp: new Date().toISOString(),
        };
        return JSON.stringify(state);
      })()`;

      const result = await runSessionCommand({
        _: ['evaluate', js],
        session: args.session,
      });

      const state = JSON.parse(result.text);
      fs.writeFileSync(filePath, JSON.stringify(state, null, 2));

      console.log(JSON.stringify({
        saved: filePath,
        cookies: state.cookies.length,
        localStorage: Object.keys(state.localStorage).length,
        sessionStorage: Object.keys(state.sessionStorage).length,
      }));
      return;
    }

    if (sub === 'load') {
      if (!fs.existsSync(filePath)) {
        console.error(`File not found: ${filePath}`);
        process.exit(1);
      }

      const state = JSON.parse(fs.readFileSync(filePath, 'utf-8'));
      const escaped = JSON.stringify(state);

      const js = `(() => {
        const state = ${escaped};

        // Restore cookies
        (state.cookies || []).forEach(function(c) {
          document.cookie = c.name + '=' + c.value;
        });

        // Restore localStorage
        const ls = state.localStorage || {};
        Object.keys(ls).forEach(function(k) { localStorage.setItem(k, ls[k]); });

        // Restore sessionStorage
        const ss = state.sessionStorage || {};
        Object.keys(ss).forEach(function(k) { sessionStorage.setItem(k, ss[k]); });

        return JSON.stringify({
          loaded: true,
          cookies: (state.cookies || []).length,
          localStorage: Object.keys(ls).length,
          sessionStorage: Object.keys(ss).length,
          fromUrl: state.url || null,
          timestamp: state.timestamp || null,
        });
      })()`;

      const result = await runSessionCommand({
        _: ['evaluate', js],
        session: args.session,
      });

      console.log(result.text);
    }
  });
}

module.exports = { register };
