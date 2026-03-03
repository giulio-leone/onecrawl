'use strict';

/**
 * http-credentials command — set or clear HTTP Basic Auth credentials.
 *
 * Usage:
 *   onecrawl-cli http-credentials <username> <password>
 *   onecrawl-cli http-credentials --clear
 *
 * Injects an Authorization header into all outgoing fetch and XHR requests.
 *
 * @module commands/http-credentials
 */

const { runSessionCommand, withErrorHandling } = require('../session-helper');

/**
 * @param {import('./index').CommandRegistry} registry
 */
function register(registry) {
  registry.add({
    name: 'http-credentials',
    description: 'set or clear HTTP Basic Auth credentials for outgoing requests',
    usage: '<username> <password> | --clear',
    action: httpCredentialsAction,
  });
}

/**
 * @param {Object} args - Parsed minimist args
 */
async function httpCredentialsAction(args) {
  await withErrorHandling(async () => {
    const clear = args.clear || false;
    const username = args._[1];
    const password = args._[2];

    if (!clear && (!username || password === undefined)) {
      console.error(
        'Usage: onecrawl-cli http-credentials <username> <password>\n' +
        '       onecrawl-cli http-credentials --clear'
      );
      process.exit(1);
    }

    let js;

    if (clear) {
      js = `(() => {
        window.__onecrawl_credentials = null;
        return JSON.stringify({ credentials: null, cleared: true });
      })()`;
    } else {
      const userStr = JSON.stringify(String(username));
      const passStr = JSON.stringify(String(password));
      js = `(() => {
        const user = ${userStr};
        const pass = ${passStr};
        const token = btoa(user + ':' + pass);
        window.__onecrawl_credentials = { username: user, token: token };

        if (!window.__onecrawl_credentials_patched) {
          const originalFetch = window.fetch;
          window.fetch = async function(input, init) {
            if (window.__onecrawl_credentials) {
              init = init || {};
              init.headers = init.headers || {};
              if (init.headers instanceof Headers) {
                init.headers.set('Authorization', 'Basic ' + window.__onecrawl_credentials.token);
              } else {
                init.headers['Authorization'] = 'Basic ' + window.__onecrawl_credentials.token;
              }
            }
            return originalFetch.call(window, input, init);
          };

          const XHRSend = XMLHttpRequest.prototype.send;
          XMLHttpRequest.prototype.send = function() {
            if (window.__onecrawl_credentials) {
              try {
                this.setRequestHeader('Authorization', 'Basic ' + window.__onecrawl_credentials.token);
              } catch (e) { /* ignore */ }
            }
            return XHRSend.apply(this, arguments);
          };

          window.__onecrawl_credentials_patched = true;
        }

        return JSON.stringify({ credentials: { username: user } });
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
