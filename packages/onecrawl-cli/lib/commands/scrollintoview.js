'use strict';

const { runSessionCommand, withErrorHandling } = require('../session-helper');

/**
 * Scrollintoview command — scrolls an element into the visible viewport.
 *
 * Usage:
 *   onecrawl-cli scrollintoview <ref|selector>
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
    name: 'scrollintoview',
    description: 'scroll an element into the visible viewport',
    usage: '<ref|selector>',
    action: scrollIntoViewAction,
  });
}

async function scrollIntoViewAction(args) {
  await withErrorHandling(async () => {
    const refOrSelector = args._[1];
    if (!refOrSelector && refOrSelector !== 0) {
      console.error('Usage: onecrawl-cli scrollintoview <ref|selector>');
      process.exit(1);
    }

    const selector = resolveSelector(String(refOrSelector));

    const js = `(() => {
      const el = document.querySelector(${JSON.stringify(selector)});
      if (!el) throw new Error('Element not found: ${selector.replace(/'/g, "\\'")}');
      el.scrollIntoView({ behavior: 'smooth', block: 'center' });
      const rect = el.getBoundingClientRect();
      return JSON.stringify({
        scrolledIntoView: true,
        target: el.tagName.toLowerCase() + (el.id ? '#' + el.id : ''),
        position: { top: Math.round(rect.top), left: Math.round(rect.left) }
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
