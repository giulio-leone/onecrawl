'use strict';

const { runSessionCommand, withErrorHandling } = require('../session-helper');

/**
 * Snapshot command — get the page accessibility tree with element refs.
 * The key AI-agent command: compact, token-efficient output.
 *
 * Usage:
 *   onecrawl-cli snapshot [-i] [-c] [-d <depth>] [-s <selector>] [--json]
 *
 * Flags:
 *   -i           Interactive elements only (buttons, links, inputs, etc.)
 *   -c           Compact output (single-line per node, no blank lines)
 *   -d <depth>   Max tree depth (default: unlimited)
 *   -s <selector> Scope to a subtree rooted at this CSS selector
 *   --json       Output as JSON instead of indented text
 */

const INTERACTIVE_ROLES = new Set([
  'button', 'link', 'textbox', 'checkbox', 'radio', 'combobox',
  'listbox', 'menuitem', 'menuitemcheckbox', 'menuitemradio',
  'option', 'searchbox', 'slider', 'spinbutton', 'switch',
  'tab', 'treeitem',
]);

/**
 * @param {import('./index').CommandRegistry} registry
 */
function register(registry) {
  registry.add({
    name: 'snapshot',
    description: 'get page accessibility tree with element refs for AI agents',
    usage: '[-i] [-c] [-d <depth>] [-s <selector>] [--json]',
    action: snapshotAction,
  });
}

/**
 * Format a single AX node into a text line.
 */
function formatNode(node, depth, compact) {
  const indent = compact ? '  '.repeat(depth) : '  '.repeat(depth);
  const role = node.role || 'unknown';
  const name = node.name ? ` "${node.name}"` : '';
  const ref = node.ref != null ? ` [ref=${node.ref}]` : '';
  const value = node.value != null && node.value !== '' ? ` value="${node.value}"` : '';
  const checked = node.checked != null ? ` checked=${node.checked}` : '';
  const selected = node.selected ? ' selected' : '';
  const disabled = node.disabled ? ' disabled' : '';
  const expanded = node.expanded != null ? ` expanded=${node.expanded}` : '';
  const attrs = `${value}${checked}${selected}${disabled}${expanded}`;
  return `${indent}- ${role}${name}${attrs}${ref}`;
}

/**
 * Recursively render the tree as indented text lines.
 */
function renderTree(node, depth, maxDepth, interactiveOnly, compact, lines) {
  if (maxDepth != null && depth > maxDepth) return;

  const isInteractive = INTERACTIVE_ROLES.has(node.role);
  const hasChildren = node.children && node.children.length > 0;

  if (!interactiveOnly || isInteractive) {
    lines.push(formatNode(node, depth, compact));
  }

  if (hasChildren) {
    const childDepth = (!interactiveOnly || isInteractive) ? depth + 1 : depth;
    for (const child of node.children) {
      renderTree(child, childDepth, maxDepth, interactiveOnly, compact, lines);
    }
  }
}

async function snapshotAction(args) {
  await withErrorHandling(async () => {
    const interactiveOnly = !!args.i;
    const compact = !!args.c;
    const maxDepth = args.d != null ? parseInt(args.d, 10) : null;
    const scopeSelector = args.s || null;
    const jsonOutput = !!args.json;

    // Build the JS to run in the browser — captures AX tree + annotates refs
    const js = `(async () => {
      const INTERACTIVE_ROLES = new Set([
        'button','link','textbox','checkbox','radio','combobox',
        'listbox','menuitem','menuitemcheckbox','menuitemradio',
        'option','searchbox','slider','spinbutton','switch',
        'tab','treeitem'
      ]);

      // Get all elements and assign refs
      const scope = ${scopeSelector ? `document.querySelector(${JSON.stringify(scopeSelector)})` : 'document.body'};
      if (!scope) return JSON.stringify({ error: 'Scope element not found' });

      const allEls = scope.querySelectorAll('*');
      let refCounter = 1;
      const refMap = new Map();
      for (const el of allEls) {
        el.setAttribute('data-oncrawl-ref', String(refCounter));
        refMap.set(el, refCounter);
        refCounter++;
      }
      // Also ref the scope itself if it's not document.body or already has one
      if (!scope.hasAttribute('data-oncrawl-ref')) {
        scope.setAttribute('data-oncrawl-ref', '0');
      }

      // Walk the DOM and build an AX-like tree
      function getRole(el) {
        if (el.getAttribute('role')) return el.getAttribute('role');
        const tag = el.tagName.toLowerCase();
        const roleMap = {
          a: 'link', button: 'button', input: 'textbox',
          select: 'combobox', textarea: 'textbox', img: 'img',
          h1: 'heading', h2: 'heading', h3: 'heading',
          h4: 'heading', h5: 'heading', h6: 'heading',
          nav: 'navigation', main: 'main', aside: 'complementary',
          footer: 'contentinfo', header: 'banner', form: 'form',
          table: 'table', ul: 'list', ol: 'list', li: 'listitem',
          dialog: 'dialog', details: 'group', summary: 'button',
          option: 'option', progress: 'progressbar',
        };
        if (tag === 'input') {
          const type = (el.type || 'text').toLowerCase();
          if (type === 'checkbox') return 'checkbox';
          if (type === 'radio') return 'radio';
          if (type === 'range') return 'slider';
          if (type === 'search') return 'searchbox';
          if (type === 'submit' || type === 'button' || type === 'reset') return 'button';
          return 'textbox';
        }
        return roleMap[tag] || 'generic';
      }

      function getName(el) {
        return el.getAttribute('aria-label')
          || el.getAttribute('alt')
          || el.getAttribute('title')
          || el.getAttribute('placeholder')
          || (el.tagName === 'LABEL' ? el.textContent.trim().substring(0, 80) : '')
          || (el.tagName === 'A' || el.tagName === 'BUTTON' ? el.textContent.trim().substring(0, 80) : '')
          || (el.tagName.match(/^H[1-6]$/) ? el.textContent.trim().substring(0, 80) : '')
          || '';
      }

      function buildTree(el) {
        const role = getRole(el);
        const name = getName(el);
        const ref = refMap.get(el) ?? (el === scope ? 0 : undefined);
        const node = { role };
        if (name) node.name = name;
        if (ref != null) node.ref = ref;
        if (el.tagName === 'INPUT' || el.tagName === 'TEXTAREA' || el.tagName === 'SELECT') {
          if (el.value) node.value = el.value;
        }
        if (el.type === 'checkbox' || el.type === 'radio') {
          node.checked = el.checked;
        }
        if (el.disabled) node.disabled = true;
        if (el.selected) node.selected = true;
        if (el.hasAttribute('aria-expanded')) {
          node.expanded = el.getAttribute('aria-expanded') === 'true';
        }
        const children = [];
        for (const child of el.children) {
          children.push(buildTree(child));
        }
        if (children.length > 0) node.children = children;
        return node;
      }

      const tree = buildTree(scope);
      return JSON.stringify(tree);
    })()`;

    const result = await runSessionCommand({
      _: ['evaluate', js],
      session: args.session,
    });

    let tree;
    try {
      tree = JSON.parse(result.text || '{}');
    } catch {
      console.error('onecrawl: failed to parse accessibility tree');
      process.exit(1);
    }

    if (tree.error) {
      console.error(`onecrawl: ${tree.error}`);
      process.exit(1);
    }

    if (jsonOutput) {
      // Filter if interactive-only before JSON output
      if (interactiveOnly) {
        tree = filterInteractive(tree);
      }
      console.log(JSON.stringify(tree, null, 2));
    } else {
      const lines = [];
      renderTree(tree, 0, maxDepth, interactiveOnly, compact, lines);
      console.log(lines.join('\n'));
    }
  });
}

/**
 * Filter tree to keep only interactive nodes (and their ancestors for structure).
 */
function filterInteractive(node) {
  if (!node) return null;
  const isInteractive = INTERACTIVE_ROLES.has(node.role);
  const filteredChildren = [];
  if (node.children) {
    for (const child of node.children) {
      const filtered = filterInteractive(child);
      if (filtered) filteredChildren.push(filtered);
    }
  }
  if (isInteractive || filteredChildren.length > 0) {
    const out = { ...node };
    if (filteredChildren.length > 0) {
      out.children = filteredChildren;
    } else {
      delete out.children;
    }
    return out;
  }
  return null;
}

module.exports = { register };
