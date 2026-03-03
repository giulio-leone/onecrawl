'use strict';

/**
 * table command — extract table data from the page.
 *
 * Usage:
 *   onecrawl-cli table [selector] [--format=json|csv]
 *
 * Options:
 *   selector   CSS selector targeting a specific <table> (default: first table)
 *   --format   Output format: json (default) or csv
 *
 * Extracts headers from <th> and rows from <td>.
 * Output: JSON array of objects keyed by headers, or CSV string.
 *
 * @module commands/table
 */

const { runSessionCommand, withErrorHandling } = require('../session-helper');

/**
 * @param {import('./index').CommandRegistry} registry
 */
function register(registry) {
  registry.add({
    name: 'table',
    description: 'extract table data from the page',
    usage: '[selector] [--format=json|csv]',
    action: tableAction,
  });
}

/**
 * @param {Object} args - Parsed minimist args
 */
async function tableAction(args) {
  await withErrorHandling(async () => {
    const selector = args._[1] || null;
    const format = args.format || 'json';

    const selectorLiteral = selector ? JSON.stringify(selector) : 'null';
    const formatLiteral = JSON.stringify(format);

    const js = `(() => {
  var selector = ${selectorLiteral};
  var format = ${formatLiteral};
  var table = selector ? document.querySelector(selector) : document.querySelector('table');
  if (!table) return JSON.stringify({ error: 'No table found on page' });
  var headers = [];
  var thEls = table.querySelectorAll('thead th, tr:first-child th');
  for (var i = 0; i < thEls.length; i++) {
    headers.push(thEls[i].textContent.trim());
  }
  if (!headers.length) {
    var firstRow = table.querySelector('tr');
    if (firstRow) {
      var cells = firstRow.querySelectorAll('td, th');
      for (var j = 0; j < cells.length; j++) {
        headers.push(cells[j].textContent.trim());
      }
    }
  }
  var rows = [];
  var trEls = table.querySelectorAll('tbody tr');
  if (!trEls.length) {
    var allTr = table.querySelectorAll('tr');
    trEls = Array.prototype.slice.call(allTr, headers.length ? 1 : 0);
  }
  for (var r = 0; r < trEls.length; r++) {
    var tds = trEls[r].querySelectorAll('td');
    if (!tds.length) continue;
    var row = {};
    for (var c = 0; c < tds.length; c++) {
      var key = c < headers.length ? headers[c] : 'col_' + c;
      row[key] = tds[c].textContent.trim();
    }
    rows.push(row);
  }
  if (format === 'csv') {
    var lines = [];
    if (headers.length) lines.push(headers.join(','));
    for (var ri = 0; ri < rows.length; ri++) {
      var vals = [];
      for (var ci = 0; ci < headers.length; ci++) {
        var v = rows[ri][headers[ci]] || '';
        vals.push(v.indexOf(',') !== -1 ? '"' + v.replace(/"/g, '""') + '"' : v);
      }
      lines.push(vals.join(','));
    }
    return JSON.stringify(lines.join('\\n'));
  }
  return JSON.stringify(rows);
})()`;

    const result = await runSessionCommand({
      _: ['evaluate', js],
      session: args.session,
    });

    try {
      const parsed = JSON.parse(result.text);
      if (format === 'csv' && typeof parsed === 'string') {
        console.log(parsed);
      } else {
        console.log(JSON.stringify(parsed));
      }
    } catch {
      console.log(result.text);
    }
  });
}

module.exports = { register };
