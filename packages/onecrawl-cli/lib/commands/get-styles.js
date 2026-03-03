'use strict';

const { runSessionCommand, withErrorHandling } = require('../session-helper');

const USAGE = `Usage: onecrawl-cli get-styles <ref|selector> [--properties=color,fontSize,padding]`;

const DEFAULT_PROPERTIES = [
  'color', 'backgroundColor', 'fontSize', 'fontFamily', 'fontWeight',
  'display', 'position', 'width', 'height', 'padding', 'margin',
  'border', 'visibility', 'opacity', 'zIndex',
];

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
    name: 'get-styles',
    description: 'get computed CSS styles of an element',
    usage: '<ref|selector> [--properties=color,fontSize,padding]',
    action: getStylesAction,
  });
}

async function getStylesAction(args) {
  await withErrorHandling(async () => {
    const refOrSelector = args._[1];

    if (!refOrSelector && refOrSelector !== 0) {
      console.error(USAGE);
      process.exit(1);
    }

    const selector = resolveSelector(String(refOrSelector));
    const properties = args.properties
      ? String(args.properties).split(',').map(p => p.trim()).filter(Boolean)
      : DEFAULT_PROPERTIES;

    const js = `(() => {
      const el = document.querySelector(${JSON.stringify(selector)});
      if (!el) return JSON.stringify({error: 'Element not found: ${selector.replace(/\\/g, '\\\\').replace(/'/g, "\\'")}'});
      const computed = getComputedStyle(el);
      const props = ${JSON.stringify(properties)};
      const result = {};
      for (const prop of props) {
        result[prop] = computed.getPropertyValue(prop) || computed[prop] || '';
      }
      return JSON.stringify(result);
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
