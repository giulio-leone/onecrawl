'use strict';

const { runSessionCommand, withErrorHandling } = require('../session-helper');

/**
 * Drag command — drags one element to another by ref number or CSS selector.
 *
 * Usage:
 *   onecrawl-cli drag <from-ref|selector> <to-ref|selector>
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
    name: 'drag',
    description: 'drag an element to another element by ref or CSS selector',
    usage: '<from-ref|selector> <to-ref|selector>',
    action: dragAction,
  });
}

/**
 * @param {Object} args - Parsed minimist args
 */
async function dragAction(args) {
  await withErrorHandling(async () => {
    const fromRef = args._[1];
    const toRef = args._[2];

    if (!fromRef || !toRef) {
      console.error(
        'Usage: onecrawl-cli drag <from-ref|selector> <to-ref|selector>'
      );
      process.exit(1);
    }

    const fromSelector = resolveSelector(String(fromRef));
    const toSelector = resolveSelector(String(toRef));

    // Verify both elements exist
    const js = `
      const from = document.querySelector(${JSON.stringify(fromSelector)});
      const to = document.querySelector(${JSON.stringify(toSelector)});
      if (!from) throw new Error('Source element not found: ${fromSelector.replace(/'/g, "\\'")}');
      if (!to) throw new Error('Target element not found: ${toSelector.replace(/'/g, "\\'")}');
      'ok';
    `;
    await runSessionCommand({ _: ['evaluate', js], session: args.session });

    // Perform drag via evaluate using native drag events with element coordinates
    const dragJs = `
      const from = document.querySelector(${JSON.stringify(fromSelector)});
      const to = document.querySelector(${JSON.stringify(toSelector)});
      const fromRect = from.getBoundingClientRect();
      const toRect = to.getBoundingClientRect();
      const fx = fromRect.x + fromRect.width / 2;
      const fy = fromRect.y + fromRect.height / 2;
      const tx = toRect.x + toRect.width / 2;
      const ty = toRect.y + toRect.height / 2;

      const dataTransfer = new DataTransfer();
      from.dispatchEvent(new DragEvent('dragstart', { bubbles: true, clientX: fx, clientY: fy, dataTransfer }));
      to.dispatchEvent(new DragEvent('dragover', { bubbles: true, clientX: tx, clientY: ty, dataTransfer }));
      to.dispatchEvent(new DragEvent('drop', { bubbles: true, clientX: tx, clientY: ty, dataTransfer }));
      from.dispatchEvent(new DragEvent('dragend', { bubbles: true, clientX: tx, clientY: ty, dataTransfer }));
      'ok';
    `;
    await runSessionCommand({ _: ['evaluate', dragJs], session: args.session });

    console.log(JSON.stringify({ dragged: true, from: fromSelector, to: toSelector }));
  });
}

module.exports = { register };
