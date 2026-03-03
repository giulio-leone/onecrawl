'use strict';

const { runSessionCommand, withErrorHandling } = require('../session-helper');

/**
 * Type command — types text into an element by ref number or CSS selector.
 *
 * Usage:
 *   onecrawl-cli type <ref|selector> <text> [--clear] [--delay=ms]
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
    name: 'type',
    description: 'type text into an element by ref or CSS selector',
    usage: '<ref|selector> <text> [--clear] [--delay=ms]',
    action: typeAction,
  });
}

/**
 * @param {Object} args - Parsed minimist args
 */
async function typeAction(args) {
  await withErrorHandling(async () => {
    const refOrSelector = args._[1];
    const text = args._[2];

    if (!refOrSelector || text === undefined || text === null) {
      console.error(
        'Usage: onecrawl-cli type <ref|selector> <text> [--clear] [--delay=ms]'
      );
      process.exit(1);
    }

    const selector = resolveSelector(String(refOrSelector));

    // If --clear, triple-click to select all then delete before typing
    if (args.clear) {
      await runSessionCommand({
        _: ['click', selector],
        session: args.session,
        clickCount: 3,
      });
      await runSessionCommand({
        _: ['evaluate', `
          const el = document.querySelector(${JSON.stringify(selector)});
          if (el) { el.value = ''; el.dispatchEvent(new Event('input', { bubbles: true })); }
        `],
        session: args.session,
      });
    }

    // Use fill for instant input, or evaluate with delay for human-like typing
    const delay = parseInt(args.delay, 10);
    if (delay > 0) {
      const js = `
        const el = document.querySelector(${JSON.stringify(selector)});
        if (!el) throw new Error('Element not found: ${selector.replace(/'/g, "\\'")}');
        el.focus();
        for (const ch of ${JSON.stringify(String(text))}) {
          el.value += ch;
          el.dispatchEvent(new Event('input', { bubbles: true }));
          await new Promise(r => setTimeout(r, ${delay}));
        }
        el.dispatchEvent(new Event('change', { bubbles: true }));
        'ok';
      `;
      await runSessionCommand({
        _: ['evaluate', js],
        session: args.session,
      });
    } else {
      await runSessionCommand({
        _: ['fill', selector, String(text)],
        session: args.session,
      });
    }

    console.log(JSON.stringify({ typed: true, text: String(text), target: selector }));
  });
}

module.exports = { register };
