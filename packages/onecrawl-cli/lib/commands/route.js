'use strict';

/**
 * route command — intercept network requests by URL pattern.
 *
 * Usage:
 *   onecrawl-cli route <url-pattern> --action=abort|mock [--status=200] [--body=<json>] [--content-type=application/json]
 *
 * Installs fetch and XMLHttpRequest interceptors that match outgoing
 * requests against the given regex pattern and either abort them or
 * return a mocked response.
 *
 * @module commands/route
 */

const { runSessionCommand, withErrorHandling } = require('../session-helper');

/**
 * @param {import('./index').CommandRegistry} registry
 */
function register(registry) {
  registry.add({
    name: 'route',
    description: 'intercept network requests matching a URL pattern',
    usage: '<url-pattern> --action=abort|mock [--status=200] [--body=<json>] [--content-type=application/json]',
    action: routeAction,
  });
}

/**
 * @param {Object} args - Parsed minimist args
 */
async function routeAction(args) {
  await withErrorHandling(async () => {
    const pattern = args._[1];
    const routeAction = args.action;

    if (!pattern || !routeAction || !['abort', 'mock'].includes(routeAction)) {
      console.error(
        'Usage: onecrawl-cli route <url-pattern> --action=abort|mock [--status=200] [--body=<json>] [--content-type=application/json]'
      );
      process.exit(1);
    }

    const status = parseInt(args.status, 10) || 200;
    const body = args.body || '{}';
    const contentType = args['content-type'] || 'application/json';

    const patternStr = JSON.stringify(pattern);
    const actionStr = JSON.stringify(routeAction);
    const statusNum = JSON.stringify(status);
    const bodyStr = JSON.stringify(body);
    const ctStr = JSON.stringify(contentType);

    const js = `(() => {
      window.__onecrawl_routes = window.__onecrawl_routes || {};
      window.__onecrawl_routes[${patternStr}] = {
        action: ${actionStr},
        status: ${statusNum},
        body: ${bodyStr},
        contentType: ${ctStr}
      };

      if (!window.__onecrawl_fetch_patched) {
        const originalFetch = window.fetch;
        window.fetch = async function(input, init) {
          const url = typeof input === 'string' ? input : input.url;
          for (const [pat, cfg] of Object.entries(window.__onecrawl_routes || {})) {
            if (url.match(new RegExp(pat))) {
              if (cfg.action === 'abort') throw new TypeError('Network request aborted');
              if (cfg.action === 'mock') return new Response(cfg.body, {
                status: cfg.status,
                headers: { 'Content-Type': cfg.contentType }
              });
            }
          }
          return originalFetch.call(window, input, init);
        };

        const XHROpen = XMLHttpRequest.prototype.open;
        const XHRSend = XMLHttpRequest.prototype.send;
        XMLHttpRequest.prototype.open = function(method, url) {
          this.__onecrawl_url = url;
          this.__onecrawl_method = method;
          return XHROpen.apply(this, arguments);
        };
        XMLHttpRequest.prototype.send = function() {
          const url = this.__onecrawl_url || '';
          for (const [pat, cfg] of Object.entries(window.__onecrawl_routes || {})) {
            if (url.match(new RegExp(pat))) {
              if (cfg.action === 'abort') {
                this.dispatchEvent(new Event('error'));
                return;
              }
              if (cfg.action === 'mock') {
                Object.defineProperty(this, 'status', { get: () => cfg.status });
                Object.defineProperty(this, 'responseText', { get: () => cfg.body });
                Object.defineProperty(this, 'readyState', { get: () => 4 });
                this.dispatchEvent(new Event('readystatechange'));
                this.dispatchEvent(new Event('load'));
                return;
              }
            }
          }
          return XHRSend.apply(this, arguments);
        };

        window.__onecrawl_fetch_patched = true;
      }

      return JSON.stringify({ routed: true, pattern: ${patternStr}, action: ${actionStr} });
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
