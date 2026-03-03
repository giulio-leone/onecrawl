'use strict';

const { runSessionCommand, withErrorHandling } = require('../session-helper');

/**
 * Hover command — hovers over an element by ref number or CSS selector.
 *
 * Usage:
 *   onecrawl-cli hover <ref|selector>
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
    name: 'hover',
    description: 'hover over an element by ref or CSS selector',
    usage: '<ref|selector>',
    action: hoverAction,
  });
}

/**
 * @param {Object} args - Parsed minimist args
 */
async function hoverAction(args) {
  await withErrorHandling(async () => {
    const refOrSelector = args._[1];
    if (!refOrSelector && refOrSelector !== 0) {
      console.error('Usage: onecrawl-cli hover <ref|selector>');
      process.exit(1);
    }

    const selector = resolveSelector(String(refOrSelector));

    await runSessionCommand({
      _: ['evaluate', `
        const el = document.querySelector(${JSON.stringify(selector)});
        if (!el) throw new Error('Element not found: ${selector.replace(/'/g, "\\'")}');
        'ok';
      `],
      session: args.session,
    });

    await runSessionCommand({
      _: ['click', selector],
      session: args.session,
      trial: true,
    });

    console.log(JSON.stringify({ hovered: true, target: selector }));
  });
}

module.exports = { register };
