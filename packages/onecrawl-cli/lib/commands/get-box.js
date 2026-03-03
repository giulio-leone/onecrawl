'use strict';

const { runSessionCommand, withErrorHandling } = require('../session-helper');

const USAGE = `Usage: onecrawl-cli get-box <ref|selector>`;

function resolveSelector(refOrSelector) {
  const num = parseInt(refOrSelector, 10);
  if (!isNaN(num) && String(num) === String(refOrSelector)) {
    return `[data-oncrawl-ref="${num}"]`;
  }
  return refOrSelector;
}

/**
 * @param {import('./index').CommandRegistry} registry
 */
function register(registry) {
  registry.add({
    name: 'get-box',
    description: 'get the bounding box of an element',
    usage: '<ref|selector>',
    action: getBoxAction,
  });
}

async function getBoxAction(args) {
  await withErrorHandling(async () => {
    const refOrSelector = args._[1];

    if (!refOrSelector && refOrSelector !== 0) {
      console.error(USAGE);
      process.exit(1);
    }

    const selector = resolveSelector(String(refOrSelector));

    const js = `(() => {
      const el = document.querySelector(${JSON.stringify(selector)});
      if (!el) return JSON.stringify({error: 'Element not found: ${selector.replace(/\\/g, '\\\\').replace(/'/g, "\\'")}'});
      const rect = el.getBoundingClientRect();
      return JSON.stringify({
        x: rect.x,
        y: rect.y,
        width: rect.width,
        height: rect.height,
        top: rect.top,
        right: rect.right,
        bottom: rect.bottom,
        left: rect.left,
        visible: rect.width > 0 && rect.height > 0
      });
    })()`;

    const result = await runSessionCommand({ _: ['evaluate', js], session: args.session });
    const parsed = JSON.parse(result.text || '{}');
    if (parsed.error) {
      console.error(`onecrawl: ${parsed.error}`);
      process.exit(1);
    }
    console.log(JSON.stringify(parsed, null, 2));
  });
}

module.exports = { register };
