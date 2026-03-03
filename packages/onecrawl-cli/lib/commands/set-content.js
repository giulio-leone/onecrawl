'use strict';

/**
 * set-content command — replace the page HTML with provided content.
 *
 * Usage:
 *   onecrawl-cli set-content <html> [--url=<base-url>]
 *   echo '<html>...' | onecrawl-cli set-content --stdin
 *
 * @module commands/set-content
 */

const { runSessionCommand, withErrorHandling } = require('../session-helper');

/**
 * @param {import('./index').CommandRegistry} registry
 */
function register(registry) {
  registry.add({
    name: 'set-content',
    description: 'replace page HTML with provided content',
    usage: '<html> [--url=<base-url>] [--stdin]',
    action: setContentAction,
  });
}

/**
 * @param {Object} args - Parsed minimist args
 */
async function setContentAction(args) {
  await withErrorHandling(async () => {
    let html;

    if (args.stdin) {
      html = await readStdin();
    } else {
      html = args._[1];
    }

    if (!html) {
      console.error(
        'Usage: onecrawl-cli set-content <html> [--url=<base-url>]\n' +
        '       echo \'<html>...\' | onecrawl-cli set-content --stdin'
      );
      process.exit(1);
    }

    if (args.url) {
      await runSessionCommand({
        _: ['navigate', args.url],
        session: args.session,
      });
    }

    const escaped = JSON.stringify(html);
    const js = `(() => {
      document.open();
      document.write(${escaped});
      document.close();
      return JSON.stringify({ set: true, length: ${escaped}.length });
    })()`;

    const result = await runSessionCommand({
      _: ['evaluate', js],
      session: args.session,
    });

    console.log(result.text);
  });
}

function readStdin() {
  return new Promise((resolve, reject) => {
    let data = '';
    process.stdin.setEncoding('utf-8');
    process.stdin.on('data', chunk => { data += chunk; });
    process.stdin.on('end', () => resolve(data));
    process.stdin.on('error', reject);
    // If stdin is a TTY, resolve immediately (no piped data)
    if (process.stdin.isTTY) resolve('');
  });
}

module.exports = { register };
