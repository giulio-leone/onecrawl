'use strict';

const { runSessionCommand, withErrorHandling } = require('../session-helper');

/**
 * Frame command — selects an iframe context by selector, name, or URL pattern.
 *
 * Usage:
 *   onecrawl-cli frame <selector|name|url-pattern>
 */

/**
 * @param {import('./index').CommandRegistry} registry
 */
function register(registry) {
  registry.add({
    name: 'frame',
    description: 'select an iframe context by selector, name, or URL pattern',
    usage: '<selector|name|url-pattern>',
    action: frameAction,
  });
}

/**
 * @param {Object} args - Parsed minimist args
 */
async function frameAction(args) {
  await withErrorHandling(async () => {
    const target = args._[1];
    if (!target) {
      console.error('Usage: onecrawl-cli frame <selector|name|url-pattern>');
      process.exit(1);
    }

    const js = `
      (() => {
        const target = ${JSON.stringify(target)};
        const iframes = Array.from(document.querySelectorAll('iframe'));
        let match = null;

        // 1. Try as CSS selector
        try { match = document.querySelector(target); } catch (_) {}
        if (match && match.tagName === 'IFRAME') {
          window.__onecrawl_active_frame = match;
          return JSON.stringify({
            frame: 'selected',
            src: match.src || '',
            name: match.name || ''
          });
        }

        // 2. Try by name attribute
        match = iframes.find(f => f.name === target);
        if (match) {
          window.__onecrawl_active_frame = match;
          return JSON.stringify({
            frame: 'selected',
            src: match.src || '',
            name: match.name || ''
          });
        }

        // 3. Try by URL pattern (substring match)
        match = iframes.find(f => f.src && f.src.includes(target));
        if (match) {
          window.__onecrawl_active_frame = match;
          return JSON.stringify({
            frame: 'selected',
            src: match.src || '',
            name: match.name || ''
          });
        }

        throw new Error(
          'No iframe found matching: ' + target +
          ' (available: ' + iframes.map(f => f.name || f.src || '<anonymous>').join(', ') + ')'
        );
      })()
    `;

    const result = await runSessionCommand({
      _: ['evaluate', js],
      session: args.session,
    });

    console.log(result.text || JSON.stringify({ frame: 'selected' }));
  });
}

module.exports = { register };
