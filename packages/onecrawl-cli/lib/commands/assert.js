'use strict';

/**
 * assert command — assert conditions on the active page or referenced elements.
 *
 * Usage:
 *   onecrawl-cli assert <condition> [args...]
 *
 * Conditions:
 *   visible <ref>           — element is visible
 *   hidden <ref>            — element is hidden
 *   text <ref> <expected>   — element text matches expected string
 *   url <pattern>           — page URL matches regex pattern
 *   title <expected>        — page title matches expected string
 *   count <selector> <n>    — element count equals n
 *
 * Exit code 0 if assertion passes, 1 if it fails.
 * On failure: prints descriptive message to stderr.
 *
 * @module commands/assert
 */

const { runSessionCommand, withErrorHandling } = require('../session-helper');

const VALID_CONDITIONS = ['visible', 'hidden', 'text', 'url', 'title', 'count'];

/**
 * @param {import('./index').CommandRegistry} registry
 */
function register(registry) {
  registry.add({
    name: 'assert',
    description: 'assert a condition on the page or element',
    usage: '<condition> [args...]',
    action: assertAction,
  });
}

/**
 * @param {Object} args - Parsed minimist args
 */
async function assertAction(args) {
  await withErrorHandling(async () => {
    const condition = args._[1];

    if (!condition || !VALID_CONDITIONS.includes(condition)) {
      console.error(
        `Usage: onecrawl-cli assert <condition> [args...]\n` +
        `Conditions: ${VALID_CONDITIONS.join(', ')}`
      );
      process.exit(1);
    }

    let js;

    switch (condition) {
      case 'visible': {
        const ref = args._[2];
        if (!ref) { console.error('Usage: assert visible <ref>'); process.exit(1); }
        const refNum = JSON.stringify(String(ref));
        js = `(() => {
          const el = document.querySelector('[data-oncrawl-ref=' + ${refNum} + ']');
          if (!el) return JSON.stringify({ pass: false, msg: 'element ref ' + ${refNum} + ' not found' });
          const r = el.getBoundingClientRect();
          const s = getComputedStyle(el);
          const vis = r.width > 0 && r.height > 0 && s.visibility !== 'hidden' && s.display !== 'none';
          return JSON.stringify({ pass: vis, msg: vis ? 'ok' : 'element ref ' + ${refNum} + ' is not visible' });
        })()`;
        break;
      }

      case 'hidden': {
        const ref = args._[2];
        if (!ref) { console.error('Usage: assert hidden <ref>'); process.exit(1); }
        const refNum = JSON.stringify(String(ref));
        js = `(() => {
          const el = document.querySelector('[data-oncrawl-ref=' + ${refNum} + ']');
          if (!el) return JSON.stringify({ pass: true, msg: 'ok' });
          const r = el.getBoundingClientRect();
          const s = getComputedStyle(el);
          const hid = r.width === 0 || r.height === 0 || s.visibility === 'hidden' || s.display === 'none';
          return JSON.stringify({ pass: hid, msg: hid ? 'ok' : 'element ref ' + ${refNum} + ' is visible, expected hidden' });
        })()`;
        break;
      }

      case 'text': {
        const ref = args._[2];
        const expected = args._[3];
        if (!ref || expected === undefined) {
          console.error('Usage: assert text <ref> <expected>');
          process.exit(1);
        }
        const refNum = JSON.stringify(String(ref));
        const expectedStr = JSON.stringify(expected);
        js = `(() => {
          const el = document.querySelector('[data-oncrawl-ref=' + ${refNum} + ']');
          if (!el) return JSON.stringify({ pass: false, msg: 'element ref ' + ${refNum} + ' not found' });
          const actual = el.textContent.trim();
          const expected = ${expectedStr};
          const pass = actual === expected;
          return JSON.stringify({
            pass: pass,
            msg: pass ? 'ok' : 'expected text ' + JSON.stringify(expected) + ' but got ' + JSON.stringify(actual)
          });
        })()`;
        break;
      }

      case 'url': {
        const pattern = args._[2];
        if (!pattern) { console.error('Usage: assert url <pattern>'); process.exit(1); }
        const patternStr = JSON.stringify(pattern);
        js = `(() => {
          const re = new RegExp(${patternStr});
          const href = window.location.href;
          const pass = re.test(href);
          return JSON.stringify({
            pass: pass,
            msg: pass ? 'ok' : 'URL ' + JSON.stringify(href) + ' does not match pattern ' + ${patternStr}
          });
        })()`;
        break;
      }

      case 'title': {
        const expected = args._[2];
        if (expected === undefined) { console.error('Usage: assert title <expected>'); process.exit(1); }
        const expectedStr = JSON.stringify(expected);
        js = `(() => {
          const actual = document.title;
          const expected = ${expectedStr};
          const pass = actual === expected;
          return JSON.stringify({
            pass: pass,
            msg: pass ? 'ok' : 'expected title ' + JSON.stringify(expected) + ' but got ' + JSON.stringify(actual)
          });
        })()`;
        break;
      }

      case 'count': {
        const selector = args._[2];
        const n = parseInt(args._[3], 10);
        if (!selector || isNaN(n)) {
          console.error('Usage: assert count <selector> <n>');
          process.exit(1);
        }
        const selectorStr = JSON.stringify(selector);
        js = `(() => {
          const actual = document.querySelectorAll(${selectorStr}).length;
          const expected = ${n};
          const pass = actual === expected;
          return JSON.stringify({
            pass: pass,
            msg: pass ? 'ok' : 'expected ' + expected + ' elements matching ' + ${selectorStr} + ' but found ' + actual
          });
        })()`;
        break;
      }
    }

    const result = await runSessionCommand({
      _: ['evaluate', js],
      session: args.session,
    });

    let parsed;
    try {
      parsed = JSON.parse(result.text);
    } catch {
      console.error(`onecrawl: unexpected assertion result: ${result.text}`);
      process.exit(1);
    }

    if (parsed.pass) {
      process.exit(0);
    } else {
      console.error(`Assertion failed: ${parsed.msg}`);
      process.exit(1);
    }
  });
}

module.exports = { register };
