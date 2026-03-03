'use strict';

const { runSessionCommand, withErrorHandling } = require('../session-helper');

const USAGE = `Usage: onecrawl-cli wait-for-function "<js-expression>" [timeout]

Examples:
  onecrawl-cli wait-for-function "document.querySelector('.loaded')"
  onecrawl-cli wait-for-function "window.appReady === true" 10000`;

/**
 * @param {import('./index').CommandRegistry} registry
 */
function register(registry) {
  registry.add({
    name: 'wait-for-function',
    description: 'wait until a JS expression returns truthy',
    usage: '"<js-expression>" [timeout]',
    action: waitForFunctionAction,
  });
}

async function waitForFunctionAction(args) {
  await withErrorHandling(async () => {
    const expr = args._[1];
    const timeoutArg = args._[2];

    if (!expr) {
      console.error(USAGE);
      process.exit(1);
    }

    const timeout = parseInt(timeoutArg || '30000', 10);
    if (isNaN(timeout) || timeout <= 0) {
      console.error('Error: timeout must be a positive number in milliseconds.');
      console.error(USAGE);
      process.exit(1);
    }

    const js = `(() => {
      return new Promise((resolve) => {
        const start = Date.now();
        const timeout = ${timeout};
        function check() {
          try {
            const result = eval(${JSON.stringify(expr)});
            if (result) return resolve(JSON.stringify({waited: Date.now() - start, result: String(result)}));
          } catch(e) {}
          if (Date.now() - start > timeout) return resolve(JSON.stringify({timeout: true, waited: timeout}));
          setTimeout(check, 100);
        }
        check();
      });
    })()`;

    const result = await runSessionCommand({ _: ['evaluate', js], session: args.session });
    const parsed = JSON.parse(result.text || '{}');
    if (parsed.timeout) {
      console.error(`onecrawl: timed out after ${timeout}ms waiting for: ${expr}`);
    }
    console.log(JSON.stringify(parsed));
  });
}

module.exports = { register };
