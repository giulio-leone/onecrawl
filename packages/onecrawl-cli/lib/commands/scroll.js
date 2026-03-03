'use strict';

const { runSessionCommand, withErrorHandling } = require('../session-helper');

/**
 * Scroll command — scrolls the active page via Playwright's mousewheel.
 *
 * Usage:
 *   onecrawl-cli scroll <direction> [pixels]
 *
 * Directions: up, down, left, right
 * Default pixels: 300
 */

const VALID_DIRECTIONS = ['up', 'down', 'left', 'right'];
const DEFAULT_PIXELS = 300;

/**
 * @param {import('./index').CommandRegistry} registry
 */
function register(registry) {
  registry.add({
    name: 'scroll',
    description: 'scroll the page in a direction (up/down/left/right)',
    usage: '<direction> [pixels]',
    action: scrollAction,
  });
}

/**
 * @param {Object} args - Parsed minimist args
 */
async function scrollAction(args) {
  await withErrorHandling(async () => {
    const direction = args._[1];
    const pixels = parseInt(args._[2] || DEFAULT_PIXELS, 10);

    if (!direction || !VALID_DIRECTIONS.includes(direction)) {
      console.error(
        `Usage: onecrawl-cli scroll <direction> [pixels]\n` +
        `Directions: ${VALID_DIRECTIONS.join(', ')}\n` +
        `Default pixels: ${DEFAULT_PIXELS}`
      );
      process.exit(1);
    }

    if (isNaN(pixels) || pixels <= 0) {
      console.error(`Invalid pixel value: '${args._[2]}'. Must be a positive integer.`);
      process.exit(1);
    }

    // Map direction to mousewheel dx/dy
    let dx = 0;
    let dy = 0;
    switch (direction) {
      case 'down':  dy = pixels;  break;
      case 'up':    dy = -pixels; break;
      case 'right': dx = pixels;  break;
      case 'left':  dx = -pixels; break;
    }

    // Delegate to Playwright's mousewheel command via the session
    const wheelArgs = {
      _: ['mousewheel', String(dx), String(dy)],
      session: args.session,
    };

    const result = await runSessionCommand(wheelArgs);
    console.log(result.text);
  });
}

module.exports = { register };
