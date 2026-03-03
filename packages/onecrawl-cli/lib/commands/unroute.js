'use strict';

/**
 * unroute command — remove network request interception for a URL pattern.
 *
 * Usage:
 *   onecrawl-cli unroute <url-pattern>
 *   onecrawl-cli unroute --all
 *
 * @module commands/unroute
 */

const { runSessionCommand, withErrorHandling } = require('../session-helper');

/**
 * @param {import('./index').CommandRegistry} registry
 */
function register(registry) {
  registry.add({
    name: 'unroute',
    description: 'remove network request interception for a URL pattern',
    usage: '<url-pattern> | --all',
    action: unrouteAction,
  });
}

/**
 * @param {Object} args - Parsed minimist args
 */
async function unrouteAction(args) {
  await withErrorHandling(async () => {
    const pattern = args._[1];
    const all = args.all;

    if (!pattern && !all) {
      console.error('Usage: onecrawl-cli unroute <url-pattern>\n       onecrawl-cli unroute --all');
      process.exit(1);
    }

    let js;

    if (all) {
      js = `(() => {
        const count = Object.keys(window.__onecrawl_routes || {}).length;
        window.__onecrawl_routes = {};
        return JSON.stringify({ unrouted: true, pattern: '*', removed: count });
      })()`;
    } else {
      const patternStr = JSON.stringify(pattern);
      js = `(() => {
        const routes = window.__onecrawl_routes || {};
        const existed = ${patternStr} in routes;
        delete routes[${patternStr}];
        return JSON.stringify({ unrouted: existed, pattern: ${patternStr} });
      })()`;
    }

    const result = await runSessionCommand({
      _: ['evaluate', js],
      session: args.session,
    });

    try {
      console.log(JSON.stringify(JSON.parse(result.text)));
    } catch {
      console.log(result.text);
    }
  });
}

module.exports = { register };
