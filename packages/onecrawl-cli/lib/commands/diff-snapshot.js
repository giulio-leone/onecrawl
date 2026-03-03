'use strict';

/**
 * diff-snapshot command — compare accessibility snapshots of the current page.
 *
 * Usage:
 *   onecrawl-cli diff-snapshot [--baseline=<file>]
 *
 * Takes a simplified accessibility tree of the current page and compares it
 * with a previously saved baseline. If no baseline exists yet, saves the
 * current snapshot as the new baseline.
 *
 * @module commands/diff-snapshot
 */

const fs = require('node:fs');
const path = require('node:path');
const { runSessionCommand, withErrorHandling } = require('../session-helper');

const DIFF_DIR = '.onecrawl-diff';
const DEFAULT_BASELINE = path.join(DIFF_DIR, 'snapshot-baseline.json');

/**
 * @param {import('./index').CommandRegistry} registry
 */
function register(registry) {
  registry.add({
    name: 'diff-snapshot',
    description: 'compare accessibility snapshots against a baseline',
    usage: '[--baseline=<file>]',
    action: diffSnapshotAction,
  });
}

/**
 * @param {Object} args - Parsed minimist args
 */
async function diffSnapshotAction(args) {
  await withErrorHandling(async () => {
    const baselinePath = args.baseline || DEFAULT_BASELINE;

    const js = `(() => {
      const tree = [];
      const walker = document.createTreeWalker(document.body, NodeFilter.SHOW_ELEMENT);
      while (walker.nextNode()) {
        const el = walker.currentNode;
        const role = el.getAttribute('role') || el.tagName.toLowerCase();
        const text = (el.textContent || '').trim().slice(0, 50);
        const interactive = ['a','button','input','select','textarea'].includes(el.tagName.toLowerCase());
        if (interactive || role !== el.tagName.toLowerCase() || text) {
          tree.push({role, tag: el.tagName.toLowerCase(), text, interactive});
        }
      }
      return JSON.stringify(tree);
    })()`;

    const result = await runSessionCommand({
      _: ['evaluate', js],
      session: args.session,
    });

    const current = JSON.parse(result.text);

    if (!fs.existsSync(DIFF_DIR)) {
      fs.mkdirSync(DIFF_DIR, { recursive: true });
    }

    if (!fs.existsSync(baselinePath)) {
      fs.writeFileSync(baselinePath, JSON.stringify(current, null, 2));
      console.log(JSON.stringify({
        saved: baselinePath,
        elements: current.length,
        message: 'Baseline saved. Run again to compare.',
      }));
      return;
    }

    const baseline = JSON.parse(fs.readFileSync(baselinePath, 'utf-8'));
    const diff = computeDiff(baseline, current);

    console.log(JSON.stringify(diff));
  });
}

/**
 * Compare two accessibility trees and return a summary diff.
 */
function computeDiff(baseline, current) {
  const baseKeys = new Set(baseline.map(nodeKey));
  const currKeys = new Set(current.map(nodeKey));

  const added = current.filter(n => !baseKeys.has(nodeKey(n)));
  const removed = baseline.filter(n => !currKeys.has(nodeKey(n)));

  // Detect changed text for nodes with same tag+role
  const baseMap = new Map(baseline.map(n => [structKey(n), n]));
  const changed = current.filter(n => {
    const prev = baseMap.get(structKey(n));
    return prev && prev.text !== n.text;
  });

  return {
    added: added.length,
    removed: removed.length,
    changed: changed.length,
    total: current.length,
    details: [
      ...added.map(n => ({ type: 'added', ...n })),
      ...removed.map(n => ({ type: 'removed', ...n })),
      ...changed.map(n => ({ type: 'changed', ...n })),
    ],
  };
}

function nodeKey(n) {
  return `${n.tag}|${n.role}|${n.text}|${n.interactive}`;
}

function structKey(n) {
  return `${n.tag}|${n.role}|${n.interactive}`;
}

module.exports = { register };
