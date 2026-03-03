'use strict';

const { runSessionCommand, withErrorHandling } = require('../session-helper');

const USAGE = `Usage: onecrawl-cli clipboard read
       onecrawl-cli clipboard write <text>
       onecrawl-cli clipboard clear`;

const ACTIONS = ['read', 'write', 'clear'];

/**
 * @param {import('./index').CommandRegistry} registry
 */
function register(registry) {
  registry.add({
    name: 'clipboard',
    description: 'read, write, or clear the browser clipboard',
    usage: 'read | write <text> | clear',
    action: clipboardAction,
  });
}

async function clipboardAction(args) {
  await withErrorHandling(async () => {
    const action = args._[1];

    if (!action || !ACTIONS.includes(action)) {
      console.error(USAGE);
      console.error(`\nActions: ${ACTIONS.join(', ')}`);
      process.exit(1);
    }

    switch (action) {
      case 'read': {
        const js = `(async () => {
          try {
            const text = await navigator.clipboard.readText();
            return JSON.stringify({clipboard: 'read', text: text});
          } catch (e) {
            return JSON.stringify({clipboard: 'read', error: e.message});
          }
        })()`;
        const result = await runSessionCommand({ _: ['evaluate', js], session: args.session });
        const parsed = JSON.parse(result.text || '{}');
        if (parsed.error) {
          console.error(`onecrawl: clipboard read failed — ${parsed.error}`);
          process.exit(1);
        }
        console.log(JSON.stringify(parsed));
        break;
      }

      case 'write': {
        const text = args._[2];
        if (text === undefined || text === null) {
          console.error('Error: missing text argument for "write".');
          console.error(USAGE);
          process.exit(1);
        }
        const writeText = String(text);
        const js = `(async () => {
          try {
            await navigator.clipboard.writeText(${JSON.stringify(writeText)});
            return JSON.stringify({clipboard: 'write', text: ${JSON.stringify(writeText)}});
          } catch (e) {
            return JSON.stringify({clipboard: 'write', error: e.message});
          }
        })()`;
        const result = await runSessionCommand({ _: ['evaluate', js], session: args.session });
        const parsed = JSON.parse(result.text || '{}');
        if (parsed.error) {
          console.error(`onecrawl: clipboard write failed — ${parsed.error}`);
          process.exit(1);
        }
        console.log(JSON.stringify(parsed));
        break;
      }

      case 'clear': {
        const js = `(async () => {
          try {
            await navigator.clipboard.writeText('');
            return JSON.stringify({clipboard: 'clear', text: ''});
          } catch (e) {
            return JSON.stringify({clipboard: 'clear', error: e.message});
          }
        })()`;
        const result = await runSessionCommand({ _: ['evaluate', js], session: args.session });
        const parsed = JSON.parse(result.text || '{}');
        if (parsed.error) {
          console.error(`onecrawl: clipboard clear failed — ${parsed.error}`);
          process.exit(1);
        }
        console.log(JSON.stringify(parsed));
        break;
      }
    }
  });
}

module.exports = { register };
