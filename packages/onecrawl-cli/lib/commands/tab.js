'use strict';

const { runSessionCommand, withErrorHandling } = require('../session-helper');

/**
 * Tab command — manage browser tabs (list, new, switch, close).
 *
 * Usage:
 *   onecrawl-cli tab list
 *   onecrawl-cli tab new [url]
 *   onecrawl-cli tab switch <index>
 *   onecrawl-cli tab close [index]
 */

const SUB_COMMANDS = ['list', 'new', 'switch', 'close'];

/**
 * @param {import('./index').CommandRegistry} registry
 */
function register(registry) {
  registry.add({
    name: 'tab',
    description: 'manage browser tabs (list, new, switch, close)',
    usage: '<list|new|switch|close> [args]',
    action: tabAction,
  });
}

/**
 * @param {Object} args - Parsed minimist args
 */
async function tabAction(args) {
  await withErrorHandling(async () => {
    const sub = args._[1];

    if (!sub || !SUB_COMMANDS.includes(sub)) {
      console.error(
        'Usage: onecrawl-cli tab <list|new|switch|close> [args]\n\n' +
        '  tab list              list open tabs\n' +
        '  tab new [url]         open a new tab\n' +
        '  tab switch <index>    switch to tab by index\n' +
        '  tab close [index]     close tab (current or by index)'
      );
      process.exit(1);
    }

    let result;

    switch (sub) {
      case 'list': {
        result = await runSessionCommand({
          _: ['tab-list'],
          session: args.session,
        });
        break;
      }

      case 'new': {
        const url = args._[2];
        const cmdArgs = ['tab-new'];
        if (url) cmdArgs.push(url);
        result = await runSessionCommand({
          _: cmdArgs,
          session: args.session,
        });
        break;
      }

      case 'switch': {
        const idx = args._[2];
        if (idx === undefined || idx === null) {
          console.error('Usage: onecrawl-cli tab switch <index>');
          process.exit(1);
        }
        result = await runSessionCommand({
          _: ['tab-select', String(idx)],
          session: args.session,
        });
        break;
      }

      case 'close': {
        const closeIdx = args._[2];
        const closeArgs = ['tab-close'];
        if (closeIdx !== undefined && closeIdx !== null) {
          closeArgs.push(String(closeIdx));
        }
        result = await runSessionCommand({
          _: closeArgs,
          session: args.session,
        });
        break;
      }
    }

    const text = result && result.text ? result.text : '';
    // Wrap in JSON if not already JSON
    try {
      JSON.parse(text);
      console.log(text);
    } catch (_) {
      console.log(JSON.stringify({ tab: sub, result: text }));
    }
  });
}

module.exports = { register };
