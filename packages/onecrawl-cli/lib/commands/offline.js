'use strict';

const { runSessionCommand, withErrorHandling } = require('../session-helper');

/**
 * offline command — simulate offline mode by intercepting fetch/XHR.
 *
 * Usage:
 *   onecrawl-cli offline [--enable|--disable]
 *   onecrawl-cli offline status
 *
 * @module commands/offline
 */

function register(registry) {
  registry.add({
    name: 'offline',
    description: 'simulate offline mode by intercepting fetch and XHR',
    usage: '[--enable|--disable] | status',
    action: offlineAction,
  });
}

async function offlineAction(args) {
  await withErrorHandling(async () => {
    const subcommand = args._[1];
    const enableFlag = args.enable === true;
    const disableFlag = args.disable === true;

    if (subcommand === 'status') {
      const statusJs = `(() => {
        return JSON.stringify({ offline: !!window.__onecrawl_offline });
      })()`;
      const statusResult = await runSessionCommand({
        _: ['evaluate', statusJs],
        session: args.session,
      });
      try {
        console.log(JSON.stringify(JSON.parse(statusResult.text)));
      } catch {
        console.log(JSON.stringify({ offline: false }));
      }
      return;
    }

    if (!enableFlag && !disableFlag && !subcommand) {
      console.error(
        'Usage: onecrawl-cli offline [--enable|--disable]\n' +
        '       onecrawl-cli offline status'
      );
      process.exit(1);
    }

    if (enableFlag && disableFlag) {
      console.error('Cannot use --enable and --disable together.');
      process.exit(1);
    }

    // Also support positional: offline enable / offline disable
    let goOffline;
    if (enableFlag || subcommand === 'enable') {
      goOffline = true;
    } else if (disableFlag || subcommand === 'disable') {
      goOffline = false;
    } else if (subcommand) {
      console.error(`Unknown subcommand: "${subcommand}". Use --enable, --disable, or status.`);
      process.exit(1);
    }

    if (goOffline) {
      const enableJs = `(() => {
        window.__onecrawl_offline = true;

        // Intercept fetch
        if (!window.__onecrawl_orig_fetch) {
          window.__onecrawl_orig_fetch = window.fetch;
        }
        var origFetch = window.__onecrawl_orig_fetch;
        window.fetch = async function() {
          if (window.__onecrawl_offline) throw new TypeError('Failed to fetch');
          return origFetch.apply(this, arguments);
        };

        // Intercept XHR
        if (!window.__onecrawl_orig_xhr_send) {
          window.__onecrawl_orig_xhr_send = XMLHttpRequest.prototype.send;
        }
        var origSend = window.__onecrawl_orig_xhr_send;
        XMLHttpRequest.prototype.send = function() {
          if (window.__onecrawl_offline) {
            var xhr = this;
            setTimeout(function() { xhr.dispatchEvent(new Event('error')); }, 0);
            return;
          }
          return origSend.apply(this, arguments);
        };

        // Override navigator.onLine
        Object.defineProperty(navigator, 'onLine', {
          get: () => false,
          configurable: true,
        });
        window.dispatchEvent(new Event('offline'));
        return JSON.stringify({ offline: true });
      })()`;

      const result = await runSessionCommand({
        _: ['evaluate', enableJs],
        session: args.session,
      });
      try {
        console.log(JSON.stringify(JSON.parse(result.text)));
      } catch {
        console.log(JSON.stringify({ offline: true }));
      }
    } else {
      const disableJs = `(() => {
        window.__onecrawl_offline = false;

        // Restore fetch
        if (window.__onecrawl_orig_fetch) {
          window.fetch = window.__onecrawl_orig_fetch;
        }

        // Restore XHR
        if (window.__onecrawl_orig_xhr_send) {
          XMLHttpRequest.prototype.send = window.__onecrawl_orig_xhr_send;
        }

        // Restore navigator.onLine
        Object.defineProperty(navigator, 'onLine', {
          get: () => true,
          configurable: true,
        });
        window.dispatchEvent(new Event('online'));
        return JSON.stringify({ offline: false });
      })()`;

      const result = await runSessionCommand({
        _: ['evaluate', disableJs],
        session: args.session,
      });
      try {
        console.log(JSON.stringify(JSON.parse(result.text)));
      } catch {
        console.log(JSON.stringify({ offline: false }));
      }
    }
  });
}

module.exports = { register };
