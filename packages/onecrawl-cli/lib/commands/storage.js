'use strict';

/**
 * storage command — interact with localStorage / sessionStorage.
 *
 * Usage:
 *   onecrawl-cli storage get <key> [--type=local|session]
 *   onecrawl-cli storage set <key> <value> [--type=local|session]
 *   onecrawl-cli storage list [--type=local|session]
 *   onecrawl-cli storage clear [--type=local|session]
 *   onecrawl-cli storage remove <key> [--type=local|session]
 *
 * @module commands/storage
 */

const { runSessionCommand, withErrorHandling } = require('../session-helper');

const VALID_OPS = ['get', 'set', 'list', 'clear', 'remove'];

/**
 * @param {import('./index').CommandRegistry} registry
 */
function register(registry) {
  registry.add({
    name: 'storage',
    description: 'interact with localStorage or sessionStorage',
    usage: 'get|set|list|clear|remove <key> [<value>] [--type=local|session]',
    action: storageAction,
  });
}

/**
 * @param {Object} args - Parsed minimist args
 */
async function storageAction(args) {
  await withErrorHandling(async () => {
    const op = args._[1];

    if (!op || !VALID_OPS.includes(op)) {
      console.error(
        'Usage: onecrawl-cli storage get|set|list|clear|remove <key> [<value>] [--type=local|session]'
      );
      process.exit(1);
    }

    const storeType = args.type || 'local';
    if (!['local', 'session'].includes(storeType)) {
      console.error('--type must be "local" or "session"');
      process.exit(1);
    }

    const storeName = storeType === 'session' ? 'sessionStorage' : 'localStorage';
    let js;

    switch (op) {
      case 'get': {
        const key = args._[2];
        if (key === undefined) { console.error('Usage: storage get <key>'); process.exit(1); }
        const k = JSON.stringify(String(key));
        js = `(() => {
          const v = ${storeName}.getItem(${k});
          return JSON.stringify({ key: ${k}, value: v, type: '${storeType}' });
        })()`;
        break;
      }

      case 'set': {
        const key = args._[2];
        const value = args._[3];
        if (key === undefined || value === undefined) {
          console.error('Usage: storage set <key> <value>');
          process.exit(1);
        }
        const k = JSON.stringify(String(key));
        const v = JSON.stringify(String(value));
        js = `(() => {
          ${storeName}.setItem(${k}, ${v});
          return JSON.stringify({ set: true, key: ${k}, value: ${v}, type: '${storeType}' });
        })()`;
        break;
      }

      case 'list': {
        js = `(() => {
          const items = [];
          for (let i = 0; i < ${storeName}.length; i++) {
            const k = ${storeName}.key(i);
            items.push({ key: k, value: ${storeName}.getItem(k) });
          }
          return JSON.stringify({ items: items, count: items.length, type: '${storeType}' });
        })()`;
        break;
      }

      case 'clear': {
        js = `(() => {
          const count = ${storeName}.length;
          ${storeName}.clear();
          return JSON.stringify({ cleared: true, removed: count, type: '${storeType}' });
        })()`;
        break;
      }

      case 'remove': {
        const key = args._[2];
        if (key === undefined) { console.error('Usage: storage remove <key>'); process.exit(1); }
        const k = JSON.stringify(String(key));
        js = `(() => {
          const existed = ${storeName}.getItem(${k}) !== null;
          ${storeName}.removeItem(${k});
          return JSON.stringify({ removed: existed, key: ${k}, type: '${storeType}' });
        })()`;
        break;
      }
    }

    const result = await runSessionCommand({
      _: ['evaluate', js],
      session: args.session,
    });

    console.log(result.text);
  });
}

module.exports = { register };
