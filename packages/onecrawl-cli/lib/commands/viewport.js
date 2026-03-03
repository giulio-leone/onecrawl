'use strict';

const { runSessionCommand, withErrorHandling } = require('../session-helper');

/**
 * viewport command — resize the browser viewport.
 *
 * Usage:
 *   onecrawl-cli viewport <width> <height> [--scale=<deviceScaleFactor>]
 *   onecrawl-cli viewport --preset=mobile|tablet|desktop
 *
 * @module commands/viewport
 */

const PRESETS = {
  mobile:  { width: 375, height: 812, scale: 3 },
  tablet:  { width: 768, height: 1024, scale: 2 },
  desktop: { width: 1920, height: 1080, scale: 1 },
};

function register(registry) {
  registry.add({
    name: 'viewport',
    description: 'resize the browser viewport to specified dimensions or a preset',
    usage: '<width> <height> [--scale=<deviceScaleFactor>] | --preset=mobile|tablet|desktop',
    action: viewportAction,
  });
}

async function viewportAction(args) {
  await withErrorHandling(async () => {
    let w, h, scale;

    if (args.preset) {
      const preset = PRESETS[args.preset];
      if (!preset) {
        console.error(`Unknown preset: "${args.preset}". Available: ${Object.keys(PRESETS).join(', ')}`);
        process.exit(1);
      }
      w = preset.width;
      h = preset.height;
      scale = preset.scale;
    } else {
      w = parseInt(args._[1], 10);
      h = parseInt(args._[2], 10);
      scale = parseFloat(args.scale) || 1;

      if (!w || !h || w <= 0 || h <= 0) {
        console.error(
          'Usage: onecrawl-cli viewport <width> <height> [--scale=<deviceScaleFactor>]\n' +
          '       onecrawl-cli viewport --preset=mobile|tablet|desktop\n' +
          'Presets: mobile=375x812@3, tablet=768x1024@2, desktop=1920x1080@1'
        );
        process.exit(1);
      }
      if (scale <= 0) {
        console.error('Scale must be a positive number.');
        process.exit(1);
      }
    }

    const js = `(() => {
      const width = ${w};
      const height = ${h};
      const scale = ${scale};
      document.querySelector('meta[name="viewport"]')?.remove();
      const meta = document.createElement('meta');
      meta.name = 'viewport';
      meta.content = 'width=' + width + ', initial-scale=' + (1 / scale);
      document.head.appendChild(meta);
      window.dispatchEvent(new Event('resize'));
      window.__onecrawl_viewport = { width: width, height: height, scale: scale };
      return JSON.stringify({ viewport: { width: width, height: height, scale: scale } });
    })()`;

    const result = await runSessionCommand({
      _: ['evaluate', js],
      session: args.session,
    });

    try {
      console.log(JSON.stringify(JSON.parse(result.text)));
    } catch {
      console.log(JSON.stringify({ viewport: { width: w, height: h, scale } }));
    }
  });
}

module.exports = { register };
