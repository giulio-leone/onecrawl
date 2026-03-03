'use strict';

const { runSessionCommand, withErrorHandling } = require('../session-helper');

/**
 * Highlight command — visually highlight an element with a colored border + overlay.
 *
 * Usage:
 *   onecrawl-cli highlight <ref|selector> [--color=red] [--duration=2000]
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
    name: 'highlight',
    description: 'visually highlight an element with a colored border and overlay',
    usage: '<ref|selector> [--color=red] [--duration=2000]',
    action: highlightAction,
  });
}

async function highlightAction(args) {
  await withErrorHandling(async () => {
    const refOrSelector = args._[1];
    if (!refOrSelector && refOrSelector !== 0) {
      console.error('Usage: onecrawl-cli highlight <ref|selector> [--color=red] [--duration=2000]');
      process.exit(1);
    }

    const selector = resolveSelector(String(refOrSelector));
    const color = args.color || 'red';
    const duration = parseInt(args.duration || '2000', 10);

    const js = `(() => {
      const el = document.querySelector(${JSON.stringify(selector)});
      if (!el) throw new Error('Element not found: ${selector.replace(/'/g, "\\'")}');
      const prev = {
        outline: el.style.outline,
        outlineOffset: el.style.outlineOffset,
        boxShadow: el.style.boxShadow,
      };
      el.style.outline = '3px solid ${color.replace(/'/g, "\\'")}';
      el.style.outlineOffset = '2px';
      el.style.boxShadow = '0 0 12px 4px ${color.replace(/'/g, "\\'")}44';
      if (${duration} > 0) {
        setTimeout(() => {
          el.style.outline = prev.outline;
          el.style.outlineOffset = prev.outlineOffset;
          el.style.boxShadow = prev.boxShadow;
        }, ${duration});
      }
      return JSON.stringify({
        highlighted: true,
        target: el.tagName.toLowerCase() + (el.id ? '#' + el.id : ''),
        color: ${JSON.stringify(color)},
        duration: ${duration}
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
