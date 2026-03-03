'use strict';

/**
 * diff-url command — compare two URLs by snapshot and screenshot.
 *
 * Usage:
 *   onecrawl-cli diff-url <url1> <url2>
 *
 * Navigates to each URL in turn, captures an accessibility snapshot and
 * screenshot, then diffs both artefacts.
 *
 * @module commands/diff-url
 */

const fs = require('node:fs');
const path = require('node:path');
const { runSessionCommand, withErrorHandling } = require('../session-helper');

const DIFF_DIR = '.onecrawl-diff';

/**
 * @param {import('./index').CommandRegistry} registry
 */
function register(registry) {
  registry.add({
    name: 'diff-url',
    description: 'compare two URLs by snapshot and screenshot',
    usage: '<url1> <url2>',
    action: diffUrlAction,
  });
}

/**
 * @param {Object} args - Parsed minimist args
 */
async function diffUrlAction(args) {
  await withErrorHandling(async () => {
    const url1 = args._[1];
    const url2 = args._[2];

    if (!url1 || !url2) {
      console.error('Usage: onecrawl-cli diff-url <url1> <url2>');
      process.exit(1);
    }

    if (!fs.existsSync(DIFF_DIR)) {
      fs.mkdirSync(DIFF_DIR, { recursive: true });
    }

    const snapshotJs = `(() => {
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

    // Capture URL 1
    await runSessionCommand({ _: ['navigate', url1], session: args.session });
    const snap1Res = await runSessionCommand({ _: ['evaluate', snapshotJs], session: args.session });
    const snap1 = JSON.parse(snap1Res.text);
    const shot1Path = path.join(DIFF_DIR, 'diff-url-1.png');
    await runSessionCommand({ _: ['screenshot', shot1Path], session: args.session });
    const shot1Size = fs.statSync(shot1Path).size;

    // Capture URL 2
    await runSessionCommand({ _: ['navigate', url2], session: args.session });
    const snap2Res = await runSessionCommand({ _: ['evaluate', snapshotJs], session: args.session });
    const snap2 = JSON.parse(snap2Res.text);
    const shot2Path = path.join(DIFF_DIR, 'diff-url-2.png');
    await runSessionCommand({ _: ['screenshot', shot2Path], session: args.session });
    const shot2Size = fs.statSync(shot2Path).size;

    // Diff snapshots
    const snapshotDiff = computeSnapshotDiff(snap1, snap2);

    // Diff screenshots (simplified — dimension + size comparison)
    const screenshotDiff = {
      url1Size: shot1Size,
      url2Size: shot2Size,
      sizeDelta: Math.abs(shot1Size - shot2Size),
      match: shot1Size === shot2Size,
    };

    const output = {
      url1,
      url2,
      snapshotDiff,
      screenshotDiff,
    };

    console.log(JSON.stringify(output));
  });
}

function computeSnapshotDiff(a, b) {
  const aKeys = new Set(a.map(nodeKey));
  const bKeys = new Set(b.map(nodeKey));

  const added = b.filter(n => !aKeys.has(nodeKey(n)));
  const removed = a.filter(n => !bKeys.has(nodeKey(n)));

  const aMap = new Map(a.map(n => [structKey(n), n]));
  const changed = b.filter(n => {
    const prev = aMap.get(structKey(n));
    return prev && prev.text !== n.text;
  });

  return {
    added: added.length,
    removed: removed.length,
    changed: changed.length,
    totalA: a.length,
    totalB: b.length,
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
