'use strict';

const { runSessionCommand, withErrorHandling } = require('../session-helper');

/**
 * Dialog command — handle browser dialogs (alert, confirm, prompt).
 *
 * Usage:
 *   onecrawl-cli dialog [--action=accept|dismiss] [--text=<response>]
 *   onecrawl-cli dialog status
 */

const VALID_ACTIONS = ['accept', 'dismiss'];

/**
 * @param {import('./index').CommandRegistry} registry
 */
function register(registry) {
  registry.add({
    name: 'dialog',
    description: 'handle browser dialogs (accept/dismiss)',
    usage: '[--action=accept|dismiss] [--text=<response>] | status',
    action: dialogAction,
  });
}

/**
 * @param {Object} args - Parsed minimist args
 */
async function dialogAction(args) {
  await withErrorHandling(async () => {
    const sub = args._[1];
    const dialogActionType = args.action || (sub === 'status' ? null : 'accept');

    // Sub-command: status — check pending dialog state
    if (sub === 'status') {
      const js = `
        (() => {
          const pending = window.__onecrawl_dialog_pending || null;
          return JSON.stringify({ dialog: 'status', pending: pending });
        })()
      `;
      const result = await runSessionCommand({
        _: ['evaluate', js],
        session: args.session,
      });
      console.log(result.text || JSON.stringify({ dialog: 'status', pending: null }));
      return;
    }

    // Validate action
    if (dialogActionType && !VALID_ACTIONS.includes(dialogActionType)) {
      console.error(
        'Usage: onecrawl-cli dialog [--action=accept|dismiss] [--text=<response>]\n' +
        '       onecrawl-cli dialog status\n\n' +
        `Invalid action: '${dialogActionType}'. Must be 'accept' or 'dismiss'.`
      );
      process.exit(1);
    }

    let result;

    if (dialogActionType === 'accept') {
      const cmdArgs = ['dialog-accept'];
      if (args.text) cmdArgs.push(args.text);
      result = await runSessionCommand({
        _: cmdArgs,
        session: args.session,
      });
      const text = args.text || '';
      console.log(JSON.stringify({ dialog: 'accepted', text }));
    } else if (dialogActionType === 'dismiss') {
      result = await runSessionCommand({
        _: ['dialog-dismiss'],
        session: args.session,
      });
      console.log(JSON.stringify({ dialog: 'dismissed' }));
    } else {
      // No action specified and not status — install dialog listener
      const responseText = args.text || '';
      const js = `
        (() => {
          window.__onecrawl_dialog_pending = null;
          window.__onecrawl_dialog_action = 'accept';
          window.__onecrawl_dialog_text = ${JSON.stringify(responseText)};

          if (!window.__onecrawl_dialog_installed) {
            const origAlert = window.alert;
            const origConfirm = window.confirm;
            const origPrompt = window.prompt;

            window.alert = function(msg) {
              window.__onecrawl_dialog_pending = { type: 'alert', message: msg };
            };
            window.confirm = function(msg) {
              window.__onecrawl_dialog_pending = { type: 'confirm', message: msg };
              return window.__onecrawl_dialog_action === 'accept';
            };
            window.prompt = function(msg, def) {
              window.__onecrawl_dialog_pending = { type: 'prompt', message: msg, default: def };
              return window.__onecrawl_dialog_action === 'accept'
                ? (window.__onecrawl_dialog_text || def || '')
                : null;
            };
            window.__onecrawl_dialog_installed = true;
          }

          return JSON.stringify({ dialog: 'listener_installed' });
        })()
      `;
      result = await runSessionCommand({
        _: ['evaluate', js],
        session: args.session,
      });
      console.log(result.text || JSON.stringify({ dialog: 'listener_installed' }));
    }
  });
}

module.exports = { register };
