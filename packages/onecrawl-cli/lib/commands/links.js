'use strict';

/**
 * links command — extract all links from the page.
 *
 * Usage:
 *   onecrawl-cli links [--filter=<pattern>] [--external] [--internal]
 *
 * Options:
 *   --filter    Regex pattern to filter hrefs
 *   --external  Only external links (different origin)
 *   --internal  Only same-origin links
 *
 * Output: JSON array of {href, text, rel, target, external}.
 *
 * @module commands/links
 */

const { runSessionCommand, withErrorHandling } = require('../session-helper');

/**
 * @param {import('./index').CommandRegistry} registry
 */
function register(registry) {
  registry.add({
    name: 'links',
    description: 'extract all links from the page',
    usage: '[--filter=<pattern>] [--external] [--internal]',
    action: linksAction,
  });
}

/**
 * @param {Object} args - Parsed minimist args
 */
async function linksAction(args) {
  await withErrorHandling(async () => {
    const filter = args.filter || null;
    const externalOnly = !!args.external;
    const internalOnly = !!args.internal;

    const filterLiteral = filter ? JSON.stringify(filter) : 'null';
    const externalLiteral = externalOnly ? 'true' : 'false';
    const internalLiteral = internalOnly ? 'true' : 'false';

    const js = `(() => {
  var filterPattern = ${filterLiteral};
  var externalOnly = ${externalLiteral};
  var internalOnly = ${internalLiteral};
  var origin = window.location.origin;
  var anchors = document.querySelectorAll('a[href]');
  var regex = filterPattern ? new RegExp(filterPattern) : null;
  var results = [];
  for (var i = 0; i < anchors.length; i++) {
    var a = anchors[i];
    var href = a.href;
    var isExternal = href.indexOf(origin) !== 0;
    if (externalOnly && !isExternal) continue;
    if (internalOnly && isExternal) continue;
    if (regex && !regex.test(href)) continue;
    results.push({
      href: href,
      text: a.textContent.trim(),
      rel: a.getAttribute('rel') || '',
      target: a.getAttribute('target') || '',
      external: isExternal
    });
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
