'use strict';

const { runSessionCommand, withErrorHandling } = require('../session-helper');

/**
 * Mainframe command — clears iframe selection, returns focus to main document.
 *
 * Usage:
 *   onecrawl-cli mainframe
 */

/**
 * @param {import('./index').CommandRegistry} registry
 */
function register(registry) {
  registry.add({
    name: 'mainframe',
    description: 'clear iframe selection, return to main document',
    usage: '',
    action: mainframeAction,
  });
}

/**
 * @param {Object} args - Parsed minimist args
 */
async function mainframeAction(args) {
  await withErrorHandling(async () => {
    const js = `
      (() => {
        window.__onecrawl_active_frame = null;
        return JSON.stringify({ frame: 'main' });
      })()
    `;

    const result = await runSessionCommand({
      _: ['evaluate', js],
      session: args.session,
    });

    console.log(result.text || JSON.stringify({ frame: 'main' }));
  });
}

module.exports = { register };
