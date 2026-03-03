'use strict';

/**
 * headers command — set, list, or clear custom headers for outgoing requests.
 *
 * Usage:
 *   onecrawl-cli headers <name> <value>
 *   onecrawl-cli headers --list
 *   onecrawl-cli headers --clear
 *
 * @module commands/headers
 */

const { runSessionCommand, withErrorHandling } = require('../session-helper');

/**
 * @param {import('./index').CommandRegistry} registry
 */
function register(registry) {
  registry.add({
    name: 'headers',
    description: 'set, list, or clear custom request headers',
    usage: '<name> <value> | --list | --clear',
    action: headersAction,
  });
}

/**
 * @param {Object} args - Parsed minimist args
 */
async function headersAction(args) {
  await withErrorHandling(async () => {
    const list = args.list || false;
    const clear = args.clear || false;
    const name = args._[1];
    const value = args._[2];

    if (!list && !clear && (!name || value === undefined)) {
      console.error(
        'Usage: onecrawl-cli headers <name> <value>\n' +
        '       onecrawl-cli headers --list\n' +
        '       onecrawl-cli headers --clear'
      );
      process.exit(1);
    }

    let js;

    if (clear) {
      js = `(() => {
        const count = Object.keys(window.__onecrawl_headers || {}).length;
        window.__onecrawl_headers = {};
        return JSON.stringify({ cleared: true, count: count });
      })()`;
    } else if (list) {
      js = `(() => {
        return JSON.stringify({ headers: window.__onecrawl_headers || {} });
      })()`;
    } else {
      const nameStr = JSON.stringify(String(name));
      const valueStr = JSON.stringify(String(value));
      js = `(() => {
        window.__onecrawl_headers = window.__onecrawl_headers || {};
        window.__onecrawl_headers[${nameStr}] = ${valueStr};

        if (!window.__onecrawl_headers_patched) {
          const originalFetch = window.fetch;
          window.fetch = async function(input, init) {
            init = init || {};
            init.headers = init.headers || {};
            if (init.headers instanceof Headers) {
              for (const [k, v] of Object.entries(window.__onecrawl_headers || {})) {
                init.headers.set(k, v);
              }
            } else {
              Object.assign(init.headers, window.__onecrawl_headers || {});
            }
            return originalFetch.call(window, input, init);
          };

          const XHROpen = XMLHttpRequest.prototype.open;
          const XHRSend = XMLHttpRequest.prototype.send;
          XMLHttpRequest.prototype.open = function() {
            this.__onecrawl_opened = true;
            return XHROpen.apply(this, arguments);
          };
          XMLHttpRequest.prototype.send = function() {
            if (this.__onecrawl_opened) {
              for (const [k, v] of Object.entries(window.__onecrawl_headers || {})) {
                try { this.setRequestHeader(k, v); } catch (e) { /* ignore */ }
              }
            }
            return XHRSend.apply(this, arguments);
          };

          window.__onecrawl_headers_patched = true;
        }

        return JSON.stringify({ header: ${nameStr}, value: ${valueStr} });
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
