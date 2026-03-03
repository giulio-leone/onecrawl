'use strict';

/**
 * screenshot-annotate command — take an annotated screenshot with numbered
 * interactive element overlays.
 *
 * Usage:
 *   onecrawl-cli screenshot-annotate [output]
 *
 * Finds all interactive elements (buttons, links, inputs, selects, textareas),
 * numbers them with red overlay labels, takes a screenshot, removes overlays,
 * and outputs a JSON mapping of numbers to element metadata.
 *
 * Default output: annotated-<timestamp>.png
 *
 * @module commands/screenshot-annotate
 */

const { runSessionCommand, withErrorHandling } = require('../session-helper');

/**
 * @param {import('./index').CommandRegistry} registry
 */
function register(registry) {
  registry.add({
    name: 'screenshot-annotate',
    description: 'take a screenshot with numbered interactive element overlays',
    usage: '[output]',
    action: screenshotAnnotateAction,
  });
}

/**
 * @param {Object} args - Parsed minimist args
 */
async function screenshotAnnotateAction(args) {
  await withErrorHandling(async () => {
    const output = args._[1] || `annotated-${Date.now()}.png`;

    // Step 1: Inject numbered overlays and collect element mapping
    const injectJs = `(() => {
      document.querySelectorAll('[data-oncrawl-ref]').forEach(el => el.removeAttribute('data-oncrawl-ref'));
      document.querySelectorAll('.oncrawl-annotation').forEach(el => el.remove());

      const selectors = [
        'a[href]', 'button', 'input', 'select', 'textarea',
        '[role=button]', '[role=link]', '[role=checkbox]', '[role=radio]', '[role=tab]',
        '[onclick]', '[tabindex]:not([tabindex="-1"])'
      ].join(',');

      const elements = Array.from(document.querySelectorAll(selectors)).filter(el => {
        const r = el.getBoundingClientRect();
        const s = getComputedStyle(el);
        return r.width > 0 && r.height > 0 &&
               s.visibility !== 'hidden' && s.display !== 'none';
      });

      const mapping = {};
      elements.forEach((el, i) => {
        const num = i + 1;
        el.setAttribute('data-oncrawl-ref', String(num));

        var label = document.createElement('div');
        label.className = 'oncrawl-annotation';
        label.setAttribute('data-oncrawl-label', String(num));
        label.textContent = String(num);
        label.style.cssText = [
          'position:absolute',
          'z-index:2147483647',
          'background:#e63946',
          'color:#fff',
          'font-size:11px',
          'font-weight:bold',
          'font-family:monospace',
          'padding:1px 4px',
          'border-radius:3px',
          'pointer-events:none',
          'line-height:16px',
          'min-width:16px',
          'text-align:center',
          'box-shadow:0 1px 3px rgba(0,0,0,0.3)',
        ].join(';');

        var rect = el.getBoundingClientRect();
        label.style.left = (rect.left + window.scrollX) + 'px';
        label.style.top = Math.max(0, rect.top + window.scrollY - 18) + 'px';
        document.body.appendChild(label);

        mapping[num] = {
          tag: el.tagName.toLowerCase(),
          text: (el.textContent || el.value || '').trim().slice(0, 80),
          role: el.getAttribute('role') || '',
          ref: num,
        };
      });

      return JSON.stringify(mapping);
    })()`;

    const mapResult = await runSessionCommand({
      _: ['evaluate', injectJs],
      session: args.session,
    });

    // Step 2: Take the annotated screenshot
    await runSessionCommand({
      _: ['screenshot', output],
      session: args.session,
    });

    // Step 3: Remove overlay labels (leave data-oncrawl-ref for other commands)
    const cleanupJs = `(() => {
      document.querySelectorAll('.oncrawl-annotation').forEach(el => el.remove());
      return 'cleaned';
    })()`;

    await runSessionCommand({
      _: ['evaluate', cleanupJs],
      session: args.session,
    });

    // Step 4: Output mapping JSON
    try {
      const mapping = JSON.parse(mapResult.text);
      console.log(JSON.stringify(mapping, null, 2));
    } catch {
      console.log(mapResult.text);
    }

    console.error(`Screenshot saved to: ${output}`);
  });
}

module.exports = { register };
