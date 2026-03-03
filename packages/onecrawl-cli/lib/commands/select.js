'use strict';

const { runSessionCommand, withErrorHandling } = require('../session-helper');

/**
 * Select command — selects a dropdown option by ref number or CSS selector.
 *
 * Usage:
 *   onecrawl-cli select <ref|selector> <value> [--by=value|label|index]
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
    name: 'select',
    description: 'select a dropdown option by ref or CSS selector',
    usage: '<ref|selector> <value> [--by=value|label|index]',
    action: selectAction,
  });
}

/**
 * @param {Object} args - Parsed minimist args
 */
async function selectAction(args) {
  await withErrorHandling(async () => {
    const refOrSelector = args._[1];
    const value = args._[2];

    if (!refOrSelector || value === undefined || value === null) {
      console.error(
        'Usage: onecrawl-cli select <ref|selector> <value> [--by=value|label|index]'
      );
      process.exit(1);
    }

    const selector = resolveSelector(String(refOrSelector));
    const by = args.by || 'value';
    const val = String(value);

    if (by === 'label') {
      // Select by visible label text via evaluate
      const js = `
        const sel = document.querySelector(${JSON.stringify(selector)});
        if (!sel) throw new Error('Element not found: ${selector.replace(/'/g, "\\'")}');
        const opt = Array.from(sel.options).find(o => o.textContent.trim() === ${JSON.stringify(val)});
        if (!opt) throw new Error('Option with label "${val.replace(/"/g, '\\"')}" not found');
        sel.value = opt.value;
        sel.dispatchEvent(new Event('change', { bubbles: true }));
        opt.value;
      `;
      await runSessionCommand({ _: ['evaluate', js], session: args.session });
    } else if (by === 'index') {
      const idx = parseInt(val, 10);
      const js = `
        const sel = document.querySelector(${JSON.stringify(selector)});
        if (!sel) throw new Error('Element not found: ${selector.replace(/'/g, "\\'")}');
        if (${idx} < 0 || ${idx} >= sel.options.length) throw new Error('Index ${idx} out of range');
        sel.selectedIndex = ${idx};
        sel.dispatchEvent(new Event('change', { bubbles: true }));
        sel.options[${idx}].value;
      `;
      await runSessionCommand({ _: ['evaluate', js], session: args.session });
    } else {
      // Default: select by value via Playwright's selectOption
      await runSessionCommand({
        _: ['selectOption', selector, val],
        session: args.session,
      });
    }

    console.log(JSON.stringify({ selected: true, value: val, target: selector }));
  });
}

module.exports = { register };
