'use strict';

/**
 * is command — check the boolean state of a referenced element.
 *
 * Usage:
 *   onecrawl-cli is <state> <ref>
 *
 * States: visible, hidden, enabled, disabled, checked, editable
 * Returns "true" or "false" to stdout.
 * Exit code 0 if true, 1 if false.
 *
 * @module commands/is
 */

const { runSessionCommand, withErrorHandling } = require('../session-helper');

const VALID_STATES = ['visible', 'hidden', 'enabled', 'disabled', 'checked', 'editable'];

/**
 * @param {import('./index').CommandRegistry} registry
 */
function register(registry) {
  registry.add({
    name: 'is',
    description: 'check element state (visible/hidden/enabled/disabled/checked/editable)',
    usage: '<state> <ref>',
    action: isAction,
  });
}

/**
 * @param {Object} args - Parsed minimist args
 */
async function isAction(args) {
  await withErrorHandling(async () => {
    const state = args._[1];
    const ref = args._[2];

    if (!state || !VALID_STATES.includes(state) || !ref) {
      console.error(
        `Usage: onecrawl-cli is <state> <ref>\n` +
        `States: ${VALID_STATES.join(', ')}`
      );
      process.exit(1);
    }

    const refNum = JSON.stringify(String(ref));
    const stateStr = JSON.stringify(state);

    const js = `(() => {
      const el = document.querySelector('[data-oncrawl-ref=' + ${refNum} + ']');
      if (!el) return JSON.stringify({ error: 'element ref ' + ${refNum} + ' not found' });

      const rect = el.getBoundingClientRect();
      const style = getComputedStyle(el);
      const isVisible = rect.width > 0 && rect.height > 0 &&
                        style.visibility !== 'hidden' && style.display !== 'none';

      let result;
      switch (${stateStr}) {
        case 'visible':  result = isVisible; break;
        case 'hidden':   result = !isVisible; break;
        case 'enabled':  result = !el.disabled; break;
        case 'disabled': result = !!el.disabled; break;
        case 'checked':  result = !!el.checked; break;
        case 'editable':
          result = !el.disabled && !el.readOnly &&
                   (el.tagName === 'INPUT' || el.tagName === 'TEXTAREA' || el.isContentEditable);
          break;
        default: result = false;
      }
      return JSON.stringify({ value: result });
    })()`;

    const result = await runSessionCommand({
      _: ['evaluate', js],
      session: args.session,
    });

    let parsed;
    try {
      parsed = JSON.parse(result.text);
    } catch {
      console.error(`onecrawl: unexpected response: ${result.text}`);
      process.exit(1);
    }

    if (parsed.error) {
      console.error(`onecrawl: ${parsed.error}`);
      process.exit(1);
    }

    const isTrue = !!parsed.value;
    console.log(isTrue ? 'true' : 'false');
    process.exit(isTrue ? 0 : 1);
  });
}

module.exports = { register };
