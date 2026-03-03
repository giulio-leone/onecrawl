'use strict';

/**
 * console command — capture and retrieve browser console output.
 *
 * Usage:
 *   onecrawl-cli console [--level=all|log|warn|error|info|debug] [--clear] [--follow]
 *
 * Installs interceptors on console.log / warn / error / info / debug
 * that record all output to window.__onecrawl_console_log.
 *
 * @module commands/console-log
 */

const { runSessionCommand, withErrorHandling } = require('../session-helper');

/**
 * @param {import('./index').CommandRegistry} registry
 */
function register(registry) {
  registry.add({
    name: 'console',
    description: 'capture and retrieve browser console output',
    usage: '[--level=all|log|warn|error|info|debug] [--clear] [--follow]',
    action: consoleLogAction,
  });
}

/**
 * @param {Object} args - Parsed minimist args
 */
async function consoleLogAction(args) {
  await withErrorHandling(async () => {
    const level = args.level || 'all';
    const clear = args.clear || false;
    const follow = args.follow || false;

    const validLevels = ['all', 'log', 'warn', 'error', 'info', 'debug'];
    if (!validLevels.includes(level)) {
      console.error(
        'Usage: onecrawl-cli console [--level=all|log|warn|error|info|debug] [--clear] [--follow]'
      );
      process.exit(1);
    }

    const levelStr = JSON.stringify(level);
    const clearBool = JSON.stringify(!!clear);

    const js = `(() => {
      if (!window.__onecrawl_console_log) {
        window.__onecrawl_console_log = [];
        ['log', 'warn', 'error', 'info', 'debug'].forEach(function(m) {
          var orig = console[m];
          console[m] = function() {
            var args = Array.prototype.slice.call(arguments);
            window.__onecrawl_console_log.push({
              level: m,
              message: args.map(String).join(' '),
              timestamp: Date.now()
            });
            orig.apply(console, args);
          };
        });
      }

      if (${clearBool}) {
        var count = window.__onecrawl_console_log.length;
        window.__onecrawl_console_log = [];
        return JSON.stringify({ cleared: true, count: count });
      }

      var entries = window.__onecrawl_console_log.slice();
      var lvl = ${levelStr};
      if (lvl !== 'all') {
        entries = entries.filter(function(e) { return e.level === lvl; });
      }

      return JSON.stringify(entries);
    })()`;

    if (follow) {
      // Poll in a loop
      let lastLength = 0;
      const poll = async () => {
        const pollJs = `(() => {
          var entries = (window.__onecrawl_console_log || []).slice();
          var lvl = ${levelStr};
          if (lvl !== 'all') {
            entries = entries.filter(function(e) { return e.level === lvl; });
          }
          return JSON.stringify(entries);
        })()`;

        const result = await runSessionCommand({
          _: ['evaluate', pollJs],
          session: args.session,
        });

        let parsed;
        try {
          parsed = JSON.parse(result.text);
        } catch {
          return true; // stop
        }

        if (parsed.length > lastLength) {
          const newEntries = parsed.slice(lastLength);
          for (const entry of newEntries) {
            const ts = new Date(entry.timestamp).toISOString();
            console.log(`[${ts}] [${entry.level.toUpperCase()}] ${entry.message}`);
          }
          lastLength = parsed.length;
        }
        return false;
      };

      // Initial install
      await runSessionCommand({ _: ['evaluate', js], session: args.session });

      console.error('Following console output (Ctrl+C to stop)...');
      // eslint-disable-next-line no-constant-condition
      while (true) {
        const done = await poll();
        if (done) break;
        await new Promise(resolve => setTimeout(resolve, 1000));
      }
    } else {
      const result = await runSessionCommand({
        _: ['evaluate', js],
        session: args.session,
      });

      try {
        console.log(JSON.stringify(JSON.parse(result.text)));
      } catch {
        console.log(result.text);
      }
    }
  });
}

module.exports = { register };
