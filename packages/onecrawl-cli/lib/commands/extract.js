'use strict';

/**
 * extract command — extract structured data from the page.
 *
 * Usage:
 *   onecrawl-cli extract [--selector=<css>] [--fields=name,price,url]
 *
 * Options:
 *   --selector  CSS selector to scope extraction to repeated container elements
 *   --fields    Comma-separated list of field names to extract
 *
 * Without --fields, auto-detects text, links, and images from each match.
 * Output: JSON array of objects.
 *
 * @module commands/extract
 */

const { runSessionCommand, withErrorHandling } = require('../session-helper');

/**
 * @param {import('./index').CommandRegistry} registry
 */
function register(registry) {
  registry.add({
    name: 'extract',
    description: 'extract structured data from the page',
    usage: '[--selector=<css>] [--fields=name,price,url]',
    action: extractAction,
  });
}

/**
 * @param {Object} args - Parsed minimist args
 */
async function extractAction(args) {
  await withErrorHandling(async () => {
    const selector = args.selector || null;
    const fields = args.fields ? String(args.fields).split(',').map(f => f.trim()).filter(Boolean) : [];

    const selectorLiteral = selector ? JSON.stringify(selector) : 'null';
    const fieldsLiteral = JSON.stringify(fields);

    const js = `(() => {
  const selector = ${selectorLiteral};
  const fields = ${fieldsLiteral};
  const containers = selector ? document.querySelectorAll(selector) : [document.body];
  const results = [];
  for (const container of containers) {
    const item = {};
    if (fields && fields.length) {
      for (const field of fields) {
        const el = container.querySelector(
          '[aria-label*="' + field + '" i], [data-' + field + '], .' + field + ', [class*="' + field + '" i]'
        );
        item[field] = el ? (el.textContent || '').trim() : null;
      }
    } else {
      item.text = container.textContent.trim().slice(0, 500);
      item.links = Array.from(container.querySelectorAll('a[href]')).map(function(a) {
        return { text: a.textContent.trim(), href: a.href };
      });
      item.images = Array.from(container.querySelectorAll('img')).map(function(img) {
        return { alt: img.alt, src: img.src };
      });
    }
    results.push(item);
  }
  return JSON.stringify(results);
})()`;

    const result = await runSessionCommand({
      _: ['evaluate', js],
      session: args.session,
    });

    try {
      console.log(JSON.stringify(JSON.parse(result.text)));
    } catch {
      console.log(result.text);
    }
  });
}

module.exports = { register };
