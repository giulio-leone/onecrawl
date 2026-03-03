'use strict';

/**
 * js-errors command — capture and retrieve JavaScript errors.
 *
 * Usage:
 *   onecrawl-cli js-errors [--clear]
 *
 * Installs window error and unhandledrejection listeners that record
 * all uncaught errors to window.__onecrawl_errors.
 *
 * @module commands/js-errors
 */

const { runSessionCommand, withErrorHandling } = require('../session-helper');

/**
 * @param {import('./index').CommandRegistry} registry
 */
function register(registry) {
  registry.add({
    name: 'js-errors',
    description: 'capture and retrieve JavaScript errors',
    usage: '[--clear]',
    action: jsErrorsAction,
  });
}

/**
 * @param {Object} args - Parsed minimist args
 */
async function jsErrorsAction(args) {
  await withErrorHandling(async () => {
    const clear = args.clear || false;
    const clearBool = JSON.stringify(!!clear);

    const js = `(() => {
      if (!window.__onecrawl_errors) {
        window.__onecrawl_errors = [];
        window.addEventListener('error', function(e) {
          window.__onecrawl_errors.push({
            type: 'error',
            message: e.message || String(e),
            source: e.filename || null,
            line: e.lineno || null,
            col: e.colno || null,
            timestamp: Date.now()
          });
        });
        window.addEventListener('unhandledrejection', function(e) {
          window.__onecrawl_errors.push({
            type: 'unhandledrejection',
            message: e.reason ? String(e.reason) : 'Unknown rejection',
            source: null,
            line: null,
            col: null,
            timestamp: Date.now()
          });
        });
      }

      if (${clearBool}) {
        var count = window.__onecrawl_errors.length;
        window.__onecrawl_errors = [];
        return JSON.stringify({ cleared: true, count: count });
      }

      return JSON.stringify(window.__onecrawl_errors.slice());
    })()`;

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
