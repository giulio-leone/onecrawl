'use strict';

const { runSessionCommand, withErrorHandling } = require('../session-helper');

/**
 * Check command — checks (or unchecks) a checkbox/radio by ref or CSS selector.
 *
 * Usage:
 *   onecrawl-cli check <ref|selector> [--uncheck]
 */

function resolveSelector(refOrSelector) {
  const num = parseInt(refOrSelector, 10);
  if (!isNaN(num) && String(num) === refOrSelector) {
    return `[data-oncrawl-ref="${num}"]`;
  }
  return refOrSelector;
}

/**
 * @param {import('./index').CommandRegistry} registry
 */
function register(registry) {
  registry.add({
    name: 'check',
    description: 'check or uncheck a checkbox/radio by ref or CSS selector',
    usage: '<ref|selector> [--uncheck]',
    action: checkAction,
  });
}

async function checkAction(args) {
  await withErrorHandling(async () => {
    const refOrSelector = args._[1];
    if (!refOrSelector && refOrSelector !== 0) {
      console.error('Usage: onecrawl-cli check <ref|selector> [--uncheck]');
      process.exit(1);
    }

    const selector = resolveSelector(String(refOrSelector));
    const uncheck = !!args.uncheck;

    const js = `(() => {
      const el = document.querySelector(${JSON.stringify(selector)});
      if (!el) throw new Error('Element not found: ${selector.replace(/'/g, "\\'")}');
      if (el.type !== 'checkbox' && el.type !== 'radio') {
        throw new Error('Element is not a checkbox or radio: ' + el.tagName + '[type=' + (el.type || 'none') + ']');
      }
      el.checked = ${uncheck ? 'false' : 'true'};
      el.dispatchEvent(new Event('change', { bubbles: true }));
      el.dispatchEvent(new Event('input', { bubbles: true }));
      return JSON.stringify({
        checked: el.checked,
        target: el.tagName.toLowerCase() + (el.id ? '#' + el.id : ''),
        type: el.type
      });
    })()`;

    const result = await runSessionCommand({
      _: ['evaluate', js],
      session: args.session,
    });

    console.log(result.text);
  });
}

module.exports = { register };
