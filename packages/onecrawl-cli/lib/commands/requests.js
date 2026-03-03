'use strict';

/**
 * requests command — capture and inspect network requests.
 *
 * Usage:
 *   onecrawl-cli requests [--filter=<pattern>] [--type=xhr|fetch|all] [--clear]
 *
 * On first invocation, installs fetch and XMLHttpRequest interceptors
 * that log all requests to window.__onecrawl_requests.
 *
 * @module commands/requests
 */

const { runSessionCommand, withErrorHandling } = require('../session-helper');

/**
 * @param {import('./index').CommandRegistry} registry
 */
function register(registry) {
  registry.add({
    name: 'requests',
    description: 'capture and list network requests',
    usage: '[--filter=<pattern>] [--type=xhr|fetch|all] [--clear]',
    action: requestsAction,
  });
}

/**
 * @param {Object} args - Parsed minimist args
 */
async function requestsAction(args) {
  await withErrorHandling(async () => {
    const filter = args.filter || null;
    const type = args.type || 'all';
    const clear = args.clear || false;

    if (type && !['xhr', 'fetch', 'all'].includes(type)) {
      console.error('Usage: onecrawl-cli requests [--filter=<pattern>] [--type=xhr|fetch|all] [--clear]');
      process.exit(1);
    }

    const filterStr = JSON.stringify(filter);
    const typeStr = JSON.stringify(type);
    const clearBool = JSON.stringify(!!clear);

    const js = `(() => {
      if (!window.__onecrawl_requests) {
        window.__onecrawl_requests = [];

        const originalFetch = window.fetch;
        window.fetch = async function(input, init) {
          const url = typeof input === 'string' ? input : input.url;
          const method = (init && init.method) || 'GET';
          const entry = { url: url, method: method, type: 'fetch', timestamp: Date.now(), status: null };
          try {
            const resp = await originalFetch.call(window, input, init);
            entry.status = resp.status;
            window.__onecrawl_requests.push(entry);
            return resp;
          } catch (e) {
            entry.status = 0;
            entry.error = e.message;
            window.__onecrawl_requests.push(entry);
            throw e;
          }
        };

        const XHROpen = XMLHttpRequest.prototype.open;
        const XHRSend = XMLHttpRequest.prototype.send;
        XMLHttpRequest.prototype.open = function(method, url) {
          this.__onecrawl_req = { url: url, method: method, type: 'xhr', timestamp: Date.now(), status: null };
          return XHROpen.apply(this, arguments);
        };
        XMLHttpRequest.prototype.send = function() {
          const req = this.__onecrawl_req;
          if (req) {
            this.addEventListener('loadend', function() {
              req.status = this.status;
              window.__onecrawl_requests.push(req);
            });
          }
          return XHRSend.apply(this, arguments);
        };
      }

      if (${clearBool}) {
        const count = window.__onecrawl_requests.length;
        window.__onecrawl_requests = [];
        return JSON.stringify({ cleared: true, count: count });
      }

      let reqs = window.__onecrawl_requests.slice();
      const filterVal = ${filterStr};
      const typeVal = ${typeStr};

      if (filterVal) {
        const re = new RegExp(filterVal);
        reqs = reqs.filter(r => re.test(r.url));
      }
      if (typeVal && typeVal !== 'all') {
        reqs = reqs.filter(r => r.type === typeVal);
      }

      return JSON.stringify(reqs);
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
