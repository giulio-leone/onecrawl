'use strict';

const { runSessionCommand, withErrorHandling } = require('../session-helper');

const USAGE = `Usage: onecrawl-cli keyboard <action> [text-or-key]
       onecrawl-cli keyboard type "Hello"        # real keystroke simulation
       onecrawl-cli keyboard inserttext "Hello"   # raw text insertion (no key events)
       onecrawl-cli keyboard press Enter
       onecrawl-cli keyboard down Shift
       onecrawl-cli keyboard up Shift
       onecrawl-cli keyboard combo Ctrl+A         # key combination`;

const ACTIONS = ['type', 'inserttext', 'press', 'down', 'up', 'combo'];

/**
 * @param {import('./index').CommandRegistry} registry
 */
function register(registry) {
  registry.add({
    name: 'keyboard',
    description: 'simulate keyboard input (type, press, combo, etc.)',
    usage: '<action> [text-or-key]',
    action: keyboardAction,
  });
}

async function keyboardAction(args) {
  await withErrorHandling(async () => {
    const action = args._[1];
    const value = args._[2];

    if (!action || !ACTIONS.includes(action)) {
      console.error(USAGE);
      console.error(`\nActions: ${ACTIONS.join(', ')}`);
      process.exit(1);
    }

    if (!value && value !== 0) {
      console.error(`Error: missing text or key argument for "${action}".`);
      console.error(USAGE);
      process.exit(1);
    }

    const text = String(value);

    switch (action) {
      case 'type': {
        await runSessionCommand({ _: ['type', text], session: args.session });
        console.log(JSON.stringify({ keyboard: 'type', text }));
        break;
      }

      case 'inserttext': {
        const js = `
          document.execCommand('insertText', false, ${JSON.stringify(text)});
          ${JSON.stringify(text)};
        `;
        await runSessionCommand({ _: ['evaluate', js], session: args.session });
        console.log(JSON.stringify({ keyboard: 'inserttext', text }));
        break;
      }

      case 'press': {
        await runSessionCommand({ _: ['press', text], session: args.session });
        console.log(JSON.stringify({ keyboard: 'press', key: text }));
        break;
      }

      case 'down': {
        await runSessionCommand({ _: ['keydown', text], session: args.session });
        console.log(JSON.stringify({ keyboard: 'down', key: text }));
        break;
      }

      case 'up': {
        await runSessionCommand({ _: ['keyup', text], session: args.session });
        console.log(JSON.stringify({ keyboard: 'up', key: text }));
        break;
      }

      case 'combo': {
        const parts = text.split('+');
        if (parts.length < 2) {
          console.error('Error: combo requires format like "Ctrl+A" or "Ctrl+Shift+Enter".');
          process.exit(1);
        }
        const modifiers = parts.slice(0, -1);
        const key = parts[parts.length - 1];

        // Press modifiers down
        for (const mod of modifiers) {
          await runSessionCommand({ _: ['keydown', mod], session: args.session });
        }
        // Press the key
        await runSessionCommand({ _: ['press', key], session: args.session });
        // Release modifiers in reverse order
        for (const mod of modifiers.reverse()) {
          await runSessionCommand({ _: ['keyup', mod], session: args.session });
        }

        console.log(JSON.stringify({ keyboard: 'combo', combo: text, modifiers: parts.slice(0, -1), key }));
        break;
      }
    }
  });
}

module.exports = { register };
