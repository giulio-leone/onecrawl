/**
 * ptc — Programmatic Tool Calling engine.
 *
 * Usage:
 *   ptc run <script.js> [--session=<s>] [--max-attempts=3]
 *   ptc generate "<task>" [--provider=claude|openai|gemini]
 */

const { readFile } = require('node:fs/promises');
const { resolve } = require('node:path');

function register(registry) {
  registry.add({
    name: 'ptc',
    description: 'Programmatic Tool Calling — run or generate automation scripts',
    usage: `ptc run <script.js> [--session=<s>] [--max-attempts=3]
ptc generate "<task>" [--provider=claude|openai|gemini]`,
    action: async (args) => {
      const { withErrorHandling, getSession } = require('../session-helper');

      return withErrorHandling(async () => {
        const positional = args._;
        // ptc run <file> OR ptc generate "<task>"
        const subCommand = positional[0];

        if (!subCommand || (subCommand !== 'run' && subCommand !== 'generate')) {
          console.log('Usage:');
          console.log('  ptc run <script.js> [--session=<s>] [--max-attempts=3]');
          console.log('  ptc generate "<task>" [--provider=claude|openai|gemini]');
          return;
        }

        if (subCommand === 'run') {
          const scriptPath = positional[1];
          if (!scriptPath) {
            console.error('Error: script path required. Usage: ptc run <script.js>');
            process.exit(1);
          }

          const scriptContent = await readFile(resolve(process.cwd(), scriptPath), 'utf-8');
          const session = args.session || 'default';
          const maxAttempts = parseInt(args['max-attempts'] || '3', 10);

          console.log(`[PTC] Running script: ${scriptPath}`);
          console.log(`[PTC] Session: ${session}, Max attempts: ${maxAttempts}`);
          console.log(`[PTC] Script length: ${scriptContent.length} chars`);
          console.log(JSON.stringify({
            action: 'ptc-run',
            script: scriptPath,
            session,
            maxAttempts,
            scriptLength: scriptContent.length,
          }, null, 2));
        } else if (subCommand === 'generate') {
          const task = positional.slice(1).join(' ') || args.task;
          if (!task) {
            console.error('Error: task description required. Usage: ptc generate "<task>"');
            process.exit(1);
          }

          const provider = args.provider || 'claude';
          console.log(`[PTC] Generating script for: ${task}`);
          console.log(`[PTC] Provider: ${provider}`);
          console.log(JSON.stringify({
            action: 'ptc-generate',
            task,
            provider,
          }, null, 2));
        }
      });
    }
  });
}

module.exports = { register };
