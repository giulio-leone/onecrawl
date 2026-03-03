'use strict';

const { runSessionCommand, withErrorHandling } = require('../session-helper');

const USAGE = `Usage: onecrawl-cli tap <ref|selector>
       onecrawl-cli tap <x> <y>`;

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
    name: 'tap',
    description: 'simulate a touch/tap event on an element or coordinates',
    usage: '<ref|selector> | <x> <y>',
    action: tapAction,
  });
}

async function tapAction(args) {
  await withErrorHandling(async () => {
    const first = args._[1];
    const second = args._[2];

    if (first === undefined || first === null) {
      console.error(USAGE);
      process.exit(1);
    }

    // Determine if coordinate mode: tap <x> <y>
    const xNum = parseFloat(first);
    const yNum = parseFloat(second);
    const isCoordMode = !isNaN(xNum) && !isNaN(yNum) && second !== undefined;

    let js;

    if (isCoordMode) {
      js = `(() => {
        const x = ${xNum};
        const y = ${yNum};
        const el = document.elementFromPoint(x, y);
        if (!el) return JSON.stringify({tapped: false, error: 'No element at coordinates (' + x + ', ' + y + ')'});
        const touch = new Touch({identifier: 0, target: el, clientX: x, clientY: y});
        el.dispatchEvent(new TouchEvent('touchstart', {touches: [touch], targetTouches: [touch], changedTouches: [touch], bubbles: true}));
        el.dispatchEvent(new TouchEvent('touchend', {touches: [], targetTouches: [], changedTouches: [touch], bubbles: true}));
        el.click();
        return JSON.stringify({tapped: true, target: el.tagName.toLowerCase(), x: x, y: y});
      })()`;
    } else {
      const selector = resolveSelector(String(first));
      js = `(() => {
        const el = document.querySelector(${JSON.stringify(selector)});
        if (!el) return JSON.stringify({tapped: false, error: 'Element not found: ${selector.replace(/\\/g, '\\\\').replace(/'/g, "\\'")}'});
        const rect = el.getBoundingClientRect();
        const x = rect.left + rect.width / 2;
        const y = rect.top + rect.height / 2;
        const touch = new Touch({identifier: 0, target: el, clientX: x, clientY: y});
        el.dispatchEvent(new TouchEvent('touchstart', {touches: [touch], targetTouches: [touch], changedTouches: [touch], bubbles: true}));
        el.dispatchEvent(new TouchEvent('touchend', {touches: [], targetTouches: [], changedTouches: [touch], bubbles: true}));
        el.click();
        return JSON.stringify({tapped: true, target: ${JSON.stringify(selector)}, x: Math.round(x), y: Math.round(y)});
      })()`;
    }

    const result = await runSessionCommand({ _: ['evaluate', js], session: args.session });
    const parsed = JSON.parse(result.text || '{}');
    if (!parsed.tapped) {
      console.error(`onecrawl: ${parsed.error || 'Tap failed'}`);
      process.exit(1);
    }
    console.log(JSON.stringify(parsed));
  });
}

module.exports = { register };
