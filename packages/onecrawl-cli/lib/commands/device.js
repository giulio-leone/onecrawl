'use strict';

const { runSessionCommand, withErrorHandling } = require('../session-helper');

/**
 * device command — emulate a predefined device profile.
 *
 * Usage:
 *   onecrawl-cli device <name>
 *   onecrawl-cli device list
 *   onecrawl-cli device --name="iPhone 15 Pro"
 *
 * @module commands/device
 */

const DEVICES = {
  'iPhone 15 Pro': {
    width: 393, height: 852, scale: 3,
    ua: 'Mozilla/5.0 (iPhone; CPU iPhone OS 17_0 like Mac OS X) AppleWebKit/605.1.15 (KHTML, like Gecko) Version/17.0 Mobile/15E148 Safari/604.1',
    touch: true,
  },
  'iPhone 14': {
    width: 390, height: 844, scale: 3,
    ua: 'Mozilla/5.0 (iPhone; CPU iPhone OS 16_0 like Mac OS X) AppleWebKit/605.1.15 (KHTML, like Gecko) Version/16.0 Mobile/15E148 Safari/604.1',
    touch: true,
  },
  'iPad Pro': {
    width: 1024, height: 1366, scale: 2,
    ua: 'Mozilla/5.0 (iPad; CPU OS 17_0 like Mac OS X) AppleWebKit/605.1.15 (KHTML, like Gecko) Version/17.0 Mobile/15E148 Safari/604.1',
    touch: true,
  },
  'iPad Mini': {
    width: 768, height: 1024, scale: 2,
    ua: 'Mozilla/5.0 (iPad; CPU OS 17_0 like Mac OS X) AppleWebKit/605.1.15 (KHTML, like Gecko) Version/17.0 Mobile/15E148 Safari/604.1',
    touch: true,
  },
  'Pixel 8': {
    width: 412, height: 915, scale: 2.625,
    ua: 'Mozilla/5.0 (Linux; Android 14; Pixel 8) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Mobile Safari/537.36',
    touch: true,
  },
  'Galaxy S24': {
    width: 360, height: 780, scale: 3,
    ua: 'Mozilla/5.0 (Linux; Android 14; SM-S921B) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Mobile Safari/537.36',
    touch: true,
  },
  'Desktop 1080p': {
    width: 1920, height: 1080, scale: 1,
    ua: null,
    touch: false,
  },
  'Desktop 1440p': {
    width: 2560, height: 1440, scale: 1,
    ua: null,
    touch: false,
  },
  'Laptop': {
    width: 1366, height: 768, scale: 1,
    ua: null,
    touch: false,
  },
};

function register(registry) {
  registry.add({
    name: 'device',
    description: 'emulate a predefined device (viewport + user-agent + touch)',
    usage: '<name> | list | --name="iPhone 15 Pro"',
    action: deviceAction,
  });
}

async function deviceAction(args) {
  await withErrorHandling(async () => {
    const name = args.name || args._[1];

    if (!name) {
      console.error(
        'Usage: onecrawl-cli device <name>\n' +
        '       onecrawl-cli device list\n' +
        '       onecrawl-cli device --name="iPhone 15 Pro"\n' +
        'Run "onecrawl-cli device list" to see available devices.'
      );
      process.exit(1);
    }

    if (name === 'list') {
      const list = Object.entries(DEVICES).map(([n, d]) => ({
        name: n,
        width: d.width,
        height: d.height,
        scale: d.scale,
        touch: d.touch,
      }));
      console.log(JSON.stringify(list, null, 2));
      return;
    }

    const device = DEVICES[name];
    if (!device) {
      console.error(`Unknown device: "${name}". Available devices:`);
      Object.keys(DEVICES).forEach(d => console.error(`  - ${d}`));
      process.exit(1);
    }

    const js = `(() => {
      // Viewport meta
      document.querySelector('meta[name="viewport"]')?.remove();
      const meta = document.createElement('meta');
      meta.name = 'viewport';
      meta.content = 'width=${device.width}, initial-scale=' + (1 / ${device.scale});
      document.head.appendChild(meta);
      window.dispatchEvent(new Event('resize'));

      // User-agent override
      ${device.ua ? `Object.defineProperty(navigator, 'userAgent', {
        get: () => ${JSON.stringify(device.ua)},
        configurable: true,
      });` : '// Desktop device — keep default user-agent'}

      // Touch emulation
      ${device.touch ? `Object.defineProperty(navigator, 'maxTouchPoints', {
        get: () => 5,
        configurable: true,
      });
      if (!('ontouchstart' in window)) {
        window.ontouchstart = null;
      }` : `Object.defineProperty(navigator, 'maxTouchPoints', {
        get: () => 0,
        configurable: true,
      });`}

      window.__onecrawl_device = ${JSON.stringify(name)};
      return JSON.stringify({
        device: ${JSON.stringify(name)},
        viewport: { width: ${device.width}, height: ${device.height}, scale: ${device.scale} },
        userAgent: ${JSON.stringify(device.ua)},
        touch: ${device.touch},
      });
    })()`;

    const result = await runSessionCommand({
      _: ['evaluate', js],
      session: args.session,
    });

    try {
      console.log(JSON.stringify(JSON.parse(result.text)));
    } catch {
      console.log(JSON.stringify({
        device: name,
        viewport: { width: device.width, height: device.height, scale: device.scale },
        userAgent: device.ua,
        touch: device.touch,
      }));
    }
  });
}

module.exports = { register };
