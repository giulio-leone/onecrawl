'use strict';

/**
 * har command — record and export network activity in HAR 1.2 format.
 *
 * Usage:
 *   onecrawl-cli har start [--file=<path>]
 *   onecrawl-cli har stop
 *
 * @module commands/har
 */

const fs = require('fs');
const path = require('path');
const { runSessionCommand, withErrorHandling } = require('../session-helper');

/**
 * @param {import('./index').CommandRegistry} registry
 */
function register(registry) {
  registry.add({
    name: 'har',
    description: 'record and export network activity as HAR',
    usage: 'start [--file=<path>] | stop',
    action: harAction,
  });
}

/**
 * @param {Object} args - Parsed minimist args
 */
async function harAction(args) {
  await withErrorHandling(async () => {
    const subcommand = args._[1];

    if (!subcommand || !['start', 'stop'].includes(subcommand)) {
      console.error('Usage: onecrawl-cli har start [--file=<path>]\n       onecrawl-cli har stop');
      process.exit(1);
    }

    if (subcommand === 'start') {
      const js = `(() => {
        window.__onecrawl_har = { entries: [], startTime: new Date().toISOString() };

        if (!window.__onecrawl_har_patched) {
          const originalFetch = window.fetch;
          window.fetch = async function(input, init) {
            const url = typeof input === 'string' ? input : input.url;
            const method = (init && init.method) || 'GET';
            const startedAt = Date.now();
            try {
              const resp = await originalFetch.call(window, input, init);
              const endedAt = Date.now();
              const entry = {
                startedDateTime: new Date(startedAt).toISOString(),
                time: endedAt - startedAt,
                request: { method: method, url: url, httpVersion: 'HTTP/1.1', headers: [], queryString: [], bodySize: -1 },
                response: { status: resp.status, statusText: resp.statusText, httpVersion: 'HTTP/1.1', headers: [], content: { size: -1, mimeType: resp.headers.get('content-type') || '' }, bodySize: -1 },
                cache: {},
                timings: { send: 0, wait: endedAt - startedAt, receive: 0 }
              };
              if (window.__onecrawl_har) window.__onecrawl_har.entries.push(entry);
              return resp;
            } catch (e) {
              const endedAt = Date.now();
              if (window.__onecrawl_har) window.__onecrawl_har.entries.push({
                startedDateTime: new Date(startedAt).toISOString(),
                time: endedAt - startedAt,
                request: { method: method, url: url, httpVersion: 'HTTP/1.1', headers: [], queryString: [], bodySize: -1 },
                response: { status: 0, statusText: e.message, httpVersion: 'HTTP/1.1', headers: [], content: { size: 0, mimeType: '' }, bodySize: 0 },
                cache: {},
                timings: { send: 0, wait: endedAt - startedAt, receive: 0 }
              });
              throw e;
            }
          };

          const XHROpen = XMLHttpRequest.prototype.open;
          const XHRSend = XMLHttpRequest.prototype.send;
          XMLHttpRequest.prototype.open = function(method, url) {
            this.__onecrawl_har_req = { method: method, url: url, startedAt: null };
            return XHROpen.apply(this, arguments);
          };
          XMLHttpRequest.prototype.send = function() {
            if (this.__onecrawl_har_req) {
              this.__onecrawl_har_req.startedAt = Date.now();
              this.addEventListener('loadend', function() {
                const req = this.__onecrawl_har_req;
                if (req && window.__onecrawl_har) {
                  const endedAt = Date.now();
                  window.__onecrawl_har.entries.push({
                    startedDateTime: new Date(req.startedAt).toISOString(),
                    time: endedAt - req.startedAt,
                    request: { method: req.method, url: req.url, httpVersion: 'HTTP/1.1', headers: [], queryString: [], bodySize: -1 },
                    response: { status: this.status, statusText: this.statusText || '', httpVersion: 'HTTP/1.1', headers: [], content: { size: (this.responseText || '').length, mimeType: this.getResponseHeader('content-type') || '' }, bodySize: (this.responseText || '').length },
                    cache: {},
                    timings: { send: 0, wait: endedAt - req.startedAt, receive: 0 }
                  });
                }
              });
            }
            return XHRSend.apply(this, arguments);
          };

          window.__onecrawl_har_patched = true;
        }

        return JSON.stringify({ recording: true });
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
    } else {
      // stop
      const js = `(() => {
        if (!window.__onecrawl_har) {
          return JSON.stringify({ error: 'HAR recording not started' });
        }
        const har = {
          log: {
            version: '1.2',
            creator: { name: 'onecrawl-cli', version: '1.0' },
            entries: window.__onecrawl_har.entries
          }
        };
        const count = window.__onecrawl_har.entries.length;
        window.__onecrawl_har = null;
        return JSON.stringify({ _har: har, entries: count });
      })()`;

      const result = await runSessionCommand({
        _: ['evaluate', js],
        session: args.session,
      });

      let parsed;
      try {
        parsed = JSON.parse(result.text);
      } catch {
        console.log(result.text);
        return;
      }

      if (parsed.error) {
        console.error(parsed.error);
        process.exit(1);
      }

      const filePath = args.file || path.resolve(process.cwd(), 'recording.har');
      const harData = parsed._har;
      fs.writeFileSync(filePath, JSON.stringify(harData, null, 2), 'utf8');
      console.log(JSON.stringify({ saved: filePath, entries: parsed.entries }));
    }
  });
}

module.exports = { register };
