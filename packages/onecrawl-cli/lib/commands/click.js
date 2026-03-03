'use strict';

const { runSessionCommand, withErrorHandling } = require('../session-helper');

/**
 * Click command — clicks an element by ref number or CSS selector.
 *
 * Usage:
 *   onecrawl-cli click <ref|selector> [--right] [--double] [--force]
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
    name: 'click',
    description: 'click an element by ref number or CSS selector',
    usage: '<ref|selector> [--right] [--double] [--force]',
    action: clickAction,
  });
}

/**
 * @param {Object} args - Parsed minimist args
 */
async function clickAction(args) {
  await withErrorHandling(async () => {
    const refOrSelector = args._[1];
    if (!refOrSelector && refOrSelector !== 0) {
      console.error(
        'Usage: onecrawl-cli click <ref|selector> [--right] [--double] [--force]'
      );
      process.exit(1);
    }

    const selector = resolveSelector(String(refOrSelector));

    // Build evaluate JS for click with options
    const opts = [];
    if (args.force) opts.push('force: true');
    if (args.right) opts.push("button: 'right'");
    if (args.double) opts.push('clickCount: 2');
    const optsStr = opts.length ? `, { ${opts.join(', ')} }` : '';

    const js = `
      const el = document.querySelector(${JSON.stringify(selector)});
      if (!el) throw new Error('Element not found: ${selector.replace(/'/g, "\\'")}');
      el.tagName + (el.id ? '#' + el.id : '') + (el.className ? '.' + el.className.split(' ').join('.') : '');
    `;

    // First resolve the element description via evaluate
    const descResult = await runSessionCommand({
      _: ['evaluate', js],
      session: args.session,
    });

    // Now perform the actual click via Playwright's click command
    const clickArgs = { _: ['click', selector], session: args.session };
    if (args.force) clickArgs.force = true;
    if (args.right) clickArgs.button = 'right';
    if (args.double) clickArgs.clickCount = 2;

    await runSessionCommand(clickArgs);

    const desc = descResult.text || selector;
    console.log(JSON.stringify({ clicked: true, target: desc }));
  });
}

module.exports = { register };
