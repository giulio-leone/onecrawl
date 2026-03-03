'use strict';

/**
 * add-script command — inject a <script> tag into the current page.
 *
 * Usage:
 *   onecrawl-cli add-script <url-or-code> [--type=module|text/javascript]
 *
 * If the argument starts with "http" it is treated as a remote src,
 * otherwise as inline script content.
 *
 * @module commands/add-script
 */

const { runSessionCommand, withErrorHandling } = require('../session-helper');

/**
 * @param {import('./index').CommandRegistry} registry
 */
function register(registry) {
  registry.add({
    name: 'add-script',
    description: 'inject a script tag into the page',
    usage: '<url-or-code> [--type=module|text/javascript]',
    action: addScriptAction,
  });
}

/**
 * @param {Object} args - Parsed minimist args
 */
async function addScriptAction(args) {
  await withErrorHandling(async () => {
    const input = args._[1];

    if (!input) {
      console.error('Usage: onecrawl-cli add-script <url-or-code> [--type=module|text/javascript]');
      process.exit(1);
    }

    const type = args.type || 'text/javascript';
    const isUrl = /^https?:\/\//.test(input);
    const escaped = JSON.stringify(input);
    const typeStr = JSON.stringify(type);

    const js = `(() => {
      const s = document.createElement('script');
      s.type = ${typeStr};
      if (${isUrl}) {
        s.src = ${escaped};
      } else {
        s.textContent = ${escaped};
      }
      document.head.appendChild(s);
      return JSON.stringify({
        injected: true,
        type: 'script',
        scriptType: ${typeStr},
        src: ${isUrl} ? ${escaped} : null,
        inline: ${!isUrl},
      });
    })()`;

    const result = await runSessionCommand({
      _: ['evaluate', js],
      session: args.session,
    });

    console.log(result.text);
  });
}

module.exports = { register };
