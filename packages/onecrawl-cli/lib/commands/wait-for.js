'use strict';

/**
 * wait-for command — wait for a condition on the active page.
 *
 * Usage:
 *   onecrawl-cli wait-for <target> [timeout]
 *
 * Targets:
 *   selector:<css>   — wait for a CSS selector to match an element
 *   text:<string>    — wait for text to appear on the page
 *   url:<pattern>    — wait for the URL to match a regex pattern
 *   load             — wait for the page load event
 *   networkidle      — wait for network activity to settle
 *
 * Default timeout: 30000ms
 * Exit code 0 on success, 1 on timeout.
 *
 * @module commands/wait-for
 */

const { runSessionCommand, withErrorHandling } = require('../session-helper');

const DEFAULT_TIMEOUT = 30000;

/**
 * @param {import('./index').CommandRegistry} registry
 */
function register(registry) {
  registry.add({
    name: 'wait-for',
    description: 'wait for a condition (selector/text/url/load/networkidle)',
    usage: '<target> [timeout]',
    action: waitForAction,
  });
}

/**
 * @param {Object} args - Parsed minimist args
 */
async function waitForAction(args) {
  await withErrorHandling(async () => {
    const target = args._[1];
    const timeout = parseInt(args._[2] || DEFAULT_TIMEOUT, 10);

    if (!target) {
      console.error(
        `Usage: onecrawl-cli wait-for <target> [timeout]\n` +
        `Targets: selector:<css>, text:<string>, url:<pattern>, load, networkidle\n` +
        `Default timeout: ${DEFAULT_TIMEOUT}ms`
      );
      process.exit(1);
    }

    if (isNaN(timeout) || timeout <= 0) {
      console.error(`Invalid timeout: '${args._[2]}'. Must be a positive integer (ms).`);
      process.exit(1);
    }

    let js;

    if (target === 'load') {
      js = `new Promise(resolve => {
        if (document.readyState === 'complete') return resolve('ready');
        const t = setTimeout(() => resolve('timeout'), ${timeout});
        window.addEventListener('load', () => { clearTimeout(t); resolve('ready'); });
      })`;

    } else if (target === 'networkidle') {
      js = `new Promise(resolve => {
        let pending = 0;
        let idleTimer = null;
        const deadline = setTimeout(() => resolve('timeout'), ${timeout});
        const check = () => {
          if (pending <= 0) {
            clearTimeout(idleTimer);
            idleTimer = setTimeout(() => { clearTimeout(deadline); resolve('ready'); }, 500);
          }
        };
        const origFetch = window.fetch;
        window.fetch = function() {
          pending++;
          return origFetch.apply(this, arguments).finally(() => { pending--; check(); });
        };
        const origXhrSend = XMLHttpRequest.prototype.send;
        XMLHttpRequest.prototype.send = function() {
          pending++;
          this.addEventListener('loadend', () => { pending--; check(); });
          return origXhrSend.apply(this, arguments);
        };
        check();
      })`;

    } else if (target.startsWith('selector:')) {
      const css = target.slice(9);
      js = `new Promise(resolve => {
        const sel = ${JSON.stringify(css)};
        if (document.querySelector(sel)) return resolve('ready');
        const deadline = setTimeout(() => { obs.disconnect(); resolve('timeout'); }, ${timeout});
        const obs = new MutationObserver(() => {
          if (document.querySelector(sel)) {
            obs.disconnect(); clearTimeout(deadline); resolve('ready');
          }
        });
        obs.observe(document.documentElement, { childList: true, subtree: true, attributes: true });
      })`;

    } else if (target.startsWith('text:')) {
      const text = target.slice(5);
      js = `new Promise(resolve => {
        const needle = ${JSON.stringify(text)};
        const has = () => document.body && document.body.innerText.includes(needle);
        if (has()) return resolve('ready');
        const deadline = setTimeout(() => { obs.disconnect(); resolve('timeout'); }, ${timeout});
        const obs = new MutationObserver(() => {
          if (has()) { obs.disconnect(); clearTimeout(deadline); resolve('ready'); }
        });
        obs.observe(document.documentElement, { childList: true, subtree: true, characterData: true });
      })`;

    } else if (target.startsWith('url:')) {
      const pattern = target.slice(4);
      js = `new Promise(resolve => {
        const re = new RegExp(${JSON.stringify(pattern)});
        if (re.test(window.location.href)) return resolve('ready');
        const deadline = setTimeout(() => { clearInterval(id); resolve('timeout'); }, ${timeout});
        const id = setInterval(() => {
          if (re.test(window.location.href)) {
            clearInterval(id); clearTimeout(deadline); resolve('ready');
          }
        }, 100);
      })`;

    } else {
      console.error(
        `Unknown target: '${target}'\n` +
        `Targets: selector:<css>, text:<string>, url:<pattern>, load, networkidle`
      );
      process.exit(1);
    }

    const result = await runSessionCommand({
      _: ['evaluate', js],
      session: args.session,
    });

    const outcome = (result.text || '').trim();
    if (outcome === 'timeout') {
      console.error(`onecrawl: wait-for '${target}' timed out after ${timeout}ms`);
      process.exit(1);
    }

    process.exit(0);
  });
}

module.exports = { register };
