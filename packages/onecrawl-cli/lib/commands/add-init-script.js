'use strict';

/**
 * add-init-script command — register a script to run before every page load.
 *
 * Usage:
 *   onecrawl-cli add-init-script <code>
 *
 * The script is stored on window.__onecrawl_init_scripts and re-executed
 * automatically on navigation events (popstate, DOMContentLoaded).
 *
 * @module commands/add-init-script
 */

const { runSessionCommand, withErrorHandling } = require('../session-helper');

/**
 * @param {import('./index').CommandRegistry} registry
 */
function register(registry) {
  registry.add({
    name: 'add-init-script',
    description: 'register a script to run on every page load',
    usage: '<code>',
    action: addInitScriptAction,
  });
}

/**
 * @param {Object} args - Parsed minimist args
 */
async function addInitScriptAction(args) {
  await withErrorHandling(async () => {
    const code = args._[1];

    if (!code) {
      console.error('Usage: onecrawl-cli add-init-script <code>');
      process.exit(1);
    }

    const escaped = JSON.stringify(code);

    const js = `(() => {
      if (!window.__onecrawl_init_scripts) {
        window.__onecrawl_init_scripts = [];

        // Re-run all init scripts on navigation
        function runAll() {
          (window.__onecrawl_init_scripts || []).forEach(function(s) {
            try { new Function(s)(); } catch(e) { console.error('[onecrawl-init]', e); }
          });
        }

        window.addEventListener('popstate', runAll);
        window.addEventListener('DOMContentLoaded', runAll);

        // MutationObserver as fallback for SPA navigations
        const obs = new MutationObserver(function(muts) {
          for (const m of muts) {
            if (m.type === 'childList' && m.target === document.documentElement) {
              runAll();
              break;
            }
          }
        });
        obs.observe(document.documentElement, { childList: true });
      }

      window.__onecrawl_init_scripts.push(${escaped});

      // Execute immediately as well
      try { new Function(${escaped})(); } catch(e) { /* ignore first-run errors */ }

      return JSON.stringify({
        registered: true,
        scripts: window.__onecrawl_init_scripts.length,
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
