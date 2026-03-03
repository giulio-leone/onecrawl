'use strict';

/**
 * session-info command — display session and browser information.
 *
 * Usage:
 *   onecrawl-cli session-info
 *
 * Shows JSON with: browserVersion, viewport, currentUrl, currentTitle,
 * cookiesCount, stealthStatus, sessionAge.
 * Output: pretty JSON to stdout.
 *
 * @module commands/session-info
 */

const fs = require('fs');
const { runSessionCommand, withErrorHandling, getSessionDir } = require('../session-helper');

/**
 * @param {import('./index').CommandRegistry} registry
 */
function register(registry) {
  registry.add({
    name: 'session-info',
    description: 'show session and browser information as JSON',
    usage: '',
    action: sessionInfoAction,
  });
}

/**
 * @param {Object} args - Parsed minimist args
 */
async function sessionInfoAction(args) {
  await withErrorHandling(async () => {
    // Gather browser-side information via evaluate
    const js = `(() => {
      return JSON.stringify({
        browserVersion: navigator.userAgent,
        viewport: { width: window.innerWidth, height: window.innerHeight },
        currentUrl: window.location.href,
        currentTitle: document.title,
        cookiesCount: document.cookie.split(';').filter(c => c.trim()).length,
      });
    })()`;

    const result = await runSessionCommand({
      _: ['evaluate', js],
      session: args.session,
    });

    let info;
    try {
      info = JSON.parse(result.text);
    } catch {
      info = {
        browserVersion: 'unknown',
        viewport: {},
        currentUrl: '',
        currentTitle: '',
        cookiesCount: 0,
      };
    }

    // Stealth status: active when onecrawl-cli is the launcher
    info.stealthStatus = 'active';
    try {
      const ua = (info.browserVersion || '').toLowerCase();
      if (ua.includes('headlesschrome') || ua.includes('puppeteer')) {
        info.stealthStatus = 'inactive';
      }
    } catch {
      // keep as active
    }

    // Session age based on .playwright directory timestamps
    try {
      const sessionDir = getSessionDir();
      const stat = fs.statSync(sessionDir);
      const ageMs = Date.now() - stat.birthtimeMs;
      const ageSec = Math.floor(ageMs / 1000);
      const mins = Math.floor(ageSec / 60);
      const hrs = Math.floor(mins / 60);
      if (hrs > 0) {
        info.sessionAge = `${hrs}h ${mins % 60}m`;
      } else if (mins > 0) {
        info.sessionAge = `${mins}m ${ageSec % 60}s`;
      } else {
        info.sessionAge = `${ageSec}s`;
      }
    } catch {
      info.sessionAge = 'unknown';
    }

    console.log(JSON.stringify(info, null, 2));
  });
}

module.exports = { register };
