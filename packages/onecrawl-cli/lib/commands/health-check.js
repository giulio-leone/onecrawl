'use strict';

/**
 * health-check command — diagnostic probe for browser, page, cookies,
 * passkey and stealth status.
 *
 * Usage:
 *   onecrawl-cli health-check
 *
 * Output: pretty JSON to stdout.
 * Exit 0 if healthy, 1 if any critical check fails.
 *
 * @module commands/health-check
 */

const fs = require('fs');
const path = require('path');
const os = require('os');
const { getSession, withErrorHandling } = require('../session-helper');

const ONECRAWL_DIR = path.join(os.homedir(), '.onecrawl', 'linkedin');

/**
 * @param {import('./index').CommandRegistry} registry
 */
function register(registry) {
  registry.add({
    name: 'health-check',
    description: 'run diagnostic checks on browser, page, cookies, passkey and stealth',
    usage: '',
    action: healthCheckAction,
  });
}

/**
 * @param {Object} args - Parsed minimist args
 */
async function healthCheckAction(args) {
  await withErrorHandling(async () => {
    const report = {
      browser: { alive: false, version: null },
      page: { responsive: false, url: null, title: null },
      cookies: { count: 0, hasLinkedInSession: false, liAtExpiry: null },
      passkey: { available: false, rpId: null },
      stealth: { active: false },
      timestamp: new Date().toISOString(),
    };

    let critical = false;
    let session, clientInfo;

    // ── Browser check ───────────────────────────────────────────────────
    try {
      ({ session, clientInfo } = await getSession(args));
      report.browser.alive = true;

      const uaResult = await session.run(clientInfo, {
        _: ['evaluate', 'navigator.userAgent'],
        session: args.session,
      });
      report.browser.version = uaResult.text.trim() || null;
    } catch {
      critical = true;
    }

    // All subsequent checks require a live session
    if (report.browser.alive) {
      // ── Page responsiveness ─────────────────────────────────────────
      try {
        const pingResult = await session.run(clientInfo, {
          _: ['evaluate', '1+1'],
          session: args.session,
        });
        report.page.responsive = pingResult.text.trim() === '2';
      } catch {
        critical = true;
      }

      // ── Page URL & title ────────────────────────────────────────────
      try {
        const pageJs = `JSON.stringify({ url: window.location.href, title: document.title })`;
        const pageResult = await session.run(clientInfo, {
          _: ['evaluate', pageJs],
          session: args.session,
        });
        const pageInfo = JSON.parse(pageResult.text);
        report.page.url = pageInfo.url;
        report.page.title = pageInfo.title;
      } catch {
        // non-critical — page info is informational
      }

      // ── Cookies ─────────────────────────────────────────────────────
      try {
        const cookieJs = `document.cookie`;
        const cookieResult = await session.run(clientInfo, {
          _: ['evaluate', cookieJs],
          session: args.session,
        });
        const raw = cookieResult.text.trim();
        const pairs = raw.split(';').filter(c => c.trim());
        report.cookies.count = pairs.length;

        for (const pair of pairs) {
          const [name] = pair.trim().split('=');
          if (name === 'li_at') report.cookies.hasLinkedInSession = true;
          if (name === 'JSESSIONID') report.cookies.hasLinkedInSession = true;
        }
      } catch {
        // non-critical
      }

      // ── Cookie file expiry (li_at from stored cookies.json) ─────────
      try {
        const cookiePath = path.join(ONECRAWL_DIR, 'cookies.json');
        if (fs.existsSync(cookiePath)) {
          const cookies = JSON.parse(fs.readFileSync(cookiePath, 'utf8'));
          const liAt = (Array.isArray(cookies) ? cookies : []).find(c => c.name === 'li_at');
          if (liAt && liAt.expires) {
            report.cookies.liAtExpiry = new Date(liAt.expires * 1000).toISOString();
          }
        }
      } catch {
        // non-critical
      }

      // ── Stealth ─────────────────────────────────────────────────────
      try {
        const stealthResult = await session.run(clientInfo, {
          _: ['evaluate', 'navigator.webdriver'],
          session: args.session,
        });
        const val = stealthResult.text.trim();
        // Stealth is active when webdriver is false or undefined
        report.stealth.active = val === 'false' || val === 'undefined' || val === '';
      } catch {
        // non-critical
      }
    } else {
      critical = true;
    }

    // ── Passkey ───────────────────────────────────────────────────────
    try {
      const passkeyPath = path.join(ONECRAWL_DIR, 'passkey.json');
      if (fs.existsSync(passkeyPath)) {
        report.passkey.available = true;
        const pk = JSON.parse(fs.readFileSync(passkeyPath, 'utf8'));
        report.passkey.rpId = pk.rpId || null;
      }
    } catch {
      // non-critical
    }

    console.log(JSON.stringify(report, null, 2));

    if (critical) {
      process.exit(1);
    }
  });
}

module.exports = { register };
