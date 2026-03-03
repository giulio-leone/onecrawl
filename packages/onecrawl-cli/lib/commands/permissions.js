'use strict';

const { runSessionCommand, withErrorHandling } = require('../session-helper');

/**
 * permissions command — override navigator.permissions.query results.
 *
 * Usage:
 *   onecrawl-cli permissions grant <name>
 *   onecrawl-cli permissions deny <name>
 *   onecrawl-cli permissions list
 *
 * @module commands/permissions
 */

const KNOWN_PERMISSIONS = [
  'geolocation', 'notifications', 'camera', 'microphone',
  'clipboard-read', 'clipboard-write',
];

function register(registry) {
  registry.add({
    name: 'permissions',
    description: 'override navigator.permissions.query for a given permission',
    usage: 'grant|deny <name> | list',
    action: permissionsAction,
  });
}

async function permissionsAction(args) {
  await withErrorHandling(async () => {
    const action = args._[1];
    const name = args._[2];

    if (!action) {
      console.error(
        'Usage: onecrawl-cli permissions grant <name>\n' +
        '       onecrawl-cli permissions deny <name>\n' +
        '       onecrawl-cli permissions list\n' +
        `Supported: ${KNOWN_PERMISSIONS.join(', ')}`
      );
      process.exit(1);
    }

    if (action === 'list') {
      const listJs = `(() => {
        var overrides = window.__onecrawl_permissions || {};
        var known = ${JSON.stringify(KNOWN_PERMISSIONS)};
        var result = known.map(function(p) {
          return { name: p, state: overrides[p] || 'default' };
        });
        return JSON.stringify({ permissions: result });
      })()`;

      const listResult = await runSessionCommand({
        _: ['evaluate', listJs],
        session: args.session,
      });
      try {
        console.log(JSON.stringify(JSON.parse(listResult.text)));
      } catch {
        console.log(JSON.stringify({
          permissions: KNOWN_PERMISSIONS.map(p => ({ name: p, state: 'default' })),
        }));
      }
      return;
    }

    if (action !== 'grant' && action !== 'deny') {
      console.error(`Unknown action: "${action}". Use grant, deny, or list.`);
      process.exit(1);
    }

    if (!name) {
      console.error(`Usage: onecrawl-cli permissions ${action} <name>`);
      console.error(`Supported: ${KNOWN_PERMISSIONS.join(', ')}`);
      process.exit(1);
    }

    if (!KNOWN_PERMISSIONS.includes(name)) {
      console.error(`Unknown permission: "${name}".`);
      console.error(`Supported: ${KNOWN_PERMISSIONS.join(', ')}`);
      process.exit(1);
    }

    const state = action === 'grant' ? 'granted' : 'denied';

    const js = `(() => {
      var overrides = window.__onecrawl_permissions || {};
      overrides[${JSON.stringify(name)}] = ${JSON.stringify(state)};
      window.__onecrawl_permissions = overrides;

      if (!window.__onecrawl_orig_permissions_query) {
        window.__onecrawl_orig_permissions_query = navigator.permissions.query.bind(navigator.permissions);
      }
      var origQuery = window.__onecrawl_orig_permissions_query;

      navigator.permissions.query = async function(desc) {
        if (overrides[desc.name]) {
          return {
            state: overrides[desc.name],
            onchange: null,
            addEventListener: function() {},
            removeEventListener: function() {},
            dispatchEvent: function() { return true; },
          };
        }
        return origQuery(desc);
      };

      return JSON.stringify({ permission: ${JSON.stringify(name)}, state: ${JSON.stringify(state)} });
    })()`;

    const result = await runSessionCommand({
      _: ['evaluate', js],
      session: args.session,
    });

    try {
      console.log(JSON.stringify(JSON.parse(result.text)));
    } catch {
      console.log(JSON.stringify({ permission: name, state }));
    }
  });
}

module.exports = { register };
