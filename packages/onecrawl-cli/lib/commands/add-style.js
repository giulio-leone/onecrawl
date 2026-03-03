'use strict';

/**
 * add-style command — inject a <link> or <style> tag into the current page.
 *
 * Usage:
 *   onecrawl-cli add-style <url-or-css>
 *
 * If the argument starts with "http" a <link rel="stylesheet"> is added,
 * otherwise a <style> block with the provided CSS text.
 *
 * @module commands/add-style
 */

const { runSessionCommand, withErrorHandling } = require('../session-helper');

/**
 * @param {import('./index').CommandRegistry} registry
 */
function register(registry) {
  registry.add({
    name: 'add-style',
    description: 'inject a stylesheet or inline CSS into the page',
    usage: '<url-or-css>',
    action: addStyleAction,
  });
}

/**
 * @param {Object} args - Parsed minimist args
 */
async function addStyleAction(args) {
  await withErrorHandling(async () => {
    const input = args._[1];

    if (!input) {
      console.error('Usage: onecrawl-cli add-style <url-or-css>');
      process.exit(1);
    }

    const isUrl = /^https?:\/\//.test(input);
    const escaped = JSON.stringify(input);

    const js = `(() => {
      if (${isUrl}) {
        const link = document.createElement('link');
        link.rel = 'stylesheet';
        link.href = ${escaped};
        document.head.appendChild(link);
        return JSON.stringify({ injected: true, type: 'style', method: 'link', href: ${escaped} });
      } else {
        const style = document.createElement('style');
        style.textContent = ${escaped};
        document.head.appendChild(style);
        return JSON.stringify({ injected: true, type: 'style', method: 'inline', length: ${escaped}.length });
      }
    })()`;

    const result = await runSessionCommand({
      _: ['evaluate', js],
      session: args.session,
    });

    console.log(result.text);
  });
}

module.exports = { register };
