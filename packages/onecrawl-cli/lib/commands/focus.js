'use strict';

const { runSessionCommand, withErrorHandling } = require('../session-helper');

/**
 * Focus command — focuses an element by ref number or CSS selector.
 *
 * Usage:
 *   onecrawl-cli focus <ref|selector>
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
    name: 'focus',
    description: 'focus an element by ref number or CSS selector',
    usage: '<ref|selector>',
    action: focusAction,
  });
}

async function focusAction(args) {
  await withErrorHandling(async () => {
    const refOrSelector = args._[1];
    if (!refOrSelector && refOrSelector !== 0) {
      console.error('Usage: onecrawl-cli focus <ref|selector>');
      process.exit(1);
    }

    const selector = resolveSelector(String(refOrSelector));

    const js = `(() => {
      const el = document.querySelector(${JSON.stringify(selector)});
      if (!el) throw new Error('Element not found: ${selector.replace(/'/g, "\\'")}');
      el.focus();
      return JSON.stringify({
        focused: true,
        target: el.tagName.toLowerCase() + (el.id ? '#' + el.id : '')
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
