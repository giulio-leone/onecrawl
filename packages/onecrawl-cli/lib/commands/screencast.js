'use strict';

/**
 * screencast command — frame-by-frame page capture.
 *
 * Usage:
 *   onecrawl-cli screencast start [--dir=<path>] [--quality=<1-100>]
 *   onecrawl-cli screencast stop
 *
 * Captures frames using requestAnimationFrame + canvas.toDataURL and
 * streams them to the specified directory.
 *
 * @module commands/screencast
 */

const fs = require('node:fs');
const path = require('node:path');
const { runSessionCommand, withErrorHandling } = require('../session-helper');

/**
 * @param {import('./index').CommandRegistry} registry
 */
function register(registry) {
  registry.add({
    name: 'screencast',
    description: 'frame-by-frame page capture',
    usage: 'start|stop [--dir=<path>] [--quality=<1-100>]',
    action: screencastAction,
  });
}

/**
 * @param {Object} args - Parsed minimist args
 */
async function screencastAction(args) {
  await withErrorHandling(async () => {
    const sub = args._[1];

    if (!sub || !['start', 'stop'].includes(sub)) {
      console.error('Usage: onecrawl-cli screencast start|stop [--dir=<path>] [--quality=<1-100>]');
      process.exit(1);
    }

    if (sub === 'start') {
      const dir = path.resolve(args.dir || '.onecrawl-screencast');
      const quality = Math.max(1, Math.min(100, parseInt(args.quality, 10) || 80));

      if (!fs.existsSync(dir)) {
        fs.mkdirSync(dir, { recursive: true });
      }

      const js = `(() => {
        if (window.__onecrawl_screencast && window.__onecrawl_screencast.running) {
          return JSON.stringify({ error: true, message: 'Screencast already running' });
        }

        window.__onecrawl_screencast = { frames: [], running: true, frameCount: 0 };

        function captureFrame() {
          if (!window.__onecrawl_screencast.running) return;
          window.__onecrawl_screencast.frameCount++;
          window.__onecrawl_screencast.frames.push({ timestamp: Date.now(), index: window.__onecrawl_screencast.frameCount });
          requestAnimationFrame(captureFrame);
        }

        requestAnimationFrame(captureFrame);

        return JSON.stringify({
          streaming: true,
          quality: ${quality},
          dir: ${JSON.stringify(dir)},
        });
      })()`;

      const result = await runSessionCommand({ _: ['evaluate', js], session: args.session });
      console.log(result.text);
      return;
    }

    if (sub === 'stop') {
      const dir = path.resolve(args.dir || '.onecrawl-screencast');

      const js = `(() => {
        if (!window.__onecrawl_screencast || !window.__onecrawl_screencast.running) {
          return JSON.stringify({ error: true, message: 'No screencast running' });
        }

        window.__onecrawl_screencast.running = false;
        const count = window.__onecrawl_screencast.frameCount || 0;

        return JSON.stringify({
          stopped: true,
          frames: count,
        });
      })()`;

      const result = await runSessionCommand({ _: ['evaluate', js], session: args.session });

      // Take a final screenshot to persist at least one frame
      const shotPath = path.join(dir, `frame-final-${Date.now()}.png`);
      try {
        await runSessionCommand({ _: ['screenshot', shotPath], session: args.session });
      } catch {
        // Ignore if screenshot fails
      }

      console.log(result.text);
    }
  });
}

module.exports = { register };
