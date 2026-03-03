'use strict';

/**
 * get command — retrieve a property from the page or a referenced element.
 *
 * Usage:
 *   onecrawl-cli get <property> [ref]
 *
 * Properties: text, html, url, title, attr:<name>, value
 * Without ref: operates on page (url, title)
 * With ref: operates on element identified by data-oncrawl-ref
 * Output: plain text value to stdout
 *
 * @module commands/get
 */

const { runSessionCommand, withErrorHandling } = require('../session-helper');

/**
 * @param {import('./index').CommandRegistry} registry
 */
function register(registry) {
  registry.add({
    name: 'get',
    description: 'get a property from the page or an element',
    usage: '<property> [ref]',
    action: getAction,
  });
}

/**
 * @param {Object} args - Parsed minimist args
 */
async function getAction(args) {
  await withErrorHandling(async () => {
    const property = args._[1];
    const ref = args._[2];

    if (!property) {
      console.error(
        `Usage: onecrawl-cli get <property> [ref]\n` +
        `Properties: text, html, url, title, attr:<name>, value\n` +
        `Without ref: url, title (page-level)\n` +
        `With ref: text, html, value, attr:<name> (element-level)`
      );
      process.exit(1);
    }

    let js;

    if (!ref) {
      // Page-level properties
      switch (property) {
        case 'url':
          js = 'window.location.href';
          break;
        case 'title':
          js = 'document.title';
          break;
        default:
          console.error(
            `Property '${property}' requires a ref number.\n` +
            `Page-level properties (no ref): url, title`
          );
          process.exit(1);
      }
    } else {
      // Element-level properties via data-oncrawl-ref
      const refNum = JSON.stringify(String(ref));
      const notFoundMsg = JSON.stringify(`element ref ${ref} not found`);

      if (property === 'text') {
        js = `(() => {
          const el = document.querySelector('[data-oncrawl-ref=' + ${refNum} + ']');
          if (!el) throw new Error(${notFoundMsg});
          return el.textContent.trim();
        })()`;
      } else if (property === 'html') {
        js = `(() => {
          const el = document.querySelector('[data-oncrawl-ref=' + ${refNum} + ']');
          if (!el) throw new Error(${notFoundMsg});
          return el.innerHTML;
        })()`;
      } else if (property === 'value') {
        js = `(() => {
          const el = document.querySelector('[data-oncrawl-ref=' + ${refNum} + ']');
          if (!el) throw new Error(${notFoundMsg});
          return typeof el.value !== 'undefined' ? String(el.value) : '';
        })()`;
      } else if (property.startsWith('attr:')) {
        const attrName = JSON.stringify(property.slice(5));
        js = `(() => {
          const el = document.querySelector('[data-oncrawl-ref=' + ${refNum} + ']');
          if (!el) throw new Error(${notFoundMsg});
          return el.getAttribute(${attrName}) || '';
        })()`;
      } else {
        console.error(
          `Unknown property: '${property}'\n` +
          `Valid properties: text, html, url, title, attr:<name>, value`
        );
        process.exit(1);
      }
    }

    const result = await runSessionCommand({
      _: ['evaluate', js],
      session: args.session,
    });

    console.log(result.text);
  });
}

module.exports = { register };
