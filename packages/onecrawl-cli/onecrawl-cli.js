#!/usr/bin/env node
/**
 * OneCrawl CLI — Token-efficient browser automation with stealth
 *
 * Wraps Microsoft's playwright-cli using the official config system:
 * - browser.initScript: injects stealth patches into every page
 * - browser.contextOptions: sets locale, viewport, userAgent
 * - network.blockedOrigins: blocks trackers/analytics for speed
 * - outputMode: saves snapshots to files (not stdout) for token efficiency
 *
 * Stealth patches (8 total):
 *  1. navigator.webdriver = false
 *  2. Chrome runtime + plugins spoofing
 *  3. Languages/locale (it-IT)
 *  4. WebGL vendor/renderer spoofing (Intel)
 *  5. Permissions API normalization
 *  6. Console.debug leak prevention
 *  7. HeadlessChrome UA fix
 *  8. Window outer dimensions fix
 */

'use strict';

const path = require('path');
const fs = require('fs');

// ── Runtime stealth patches (must be before any playwright require) ──────────
require('./stealth-loader');

// ── Generate config file with stealth + optimizations ────────────────────────

const stealthInitScript = path.join(__dirname, 'stealth-init.js');
const ghostClickScript = path.join(__dirname, 'ghost-click.js');
const { CHROME_UA } = require('./stealth');

const initScripts = [stealthInitScript];
if (process.env.GHOST_CURSOR_ENABLED === 'true') {
  initScripts.push(ghostClickScript);
}

const config = {
  browser: {
    browserName: 'chromium',
    // Inject stealth patches (+ ghost cursor if enabled) before any page JS runs
    initScript: initScripts,
    launchOptions: {
      channel: 'chrome',
      args: [
        '--disable-blink-features=AutomationControlled',
        '--no-first-run',
        '--no-default-browser-check',
      ],
    },
    contextOptions: {
      userAgent: CHROME_UA,
      locale: 'it-IT',
      viewport: {
        width: parseInt(process.env.BROWSER_VIEWPORT_WIDTH || '1024', 10),
        height: parseInt(process.env.BROWSER_VIEWPORT_HEIGHT || '768', 10),
      },
    },
  },
  // Block trackers/analytics — faster loads + fewer tokens wasted
  network: {
    blockedOrigins: [
      'https://www.google-analytics.com',
      'https://analytics.google.com',
      'https://www.googletagmanager.com',
      'https://px.ads.linkedin.com',
      'https://snap.licdn.com',
      'https://bat.bing.com',
      'https://www.facebook.com/tr',
      'https://connect.facebook.net',
      'https://sentry.io',
      'https://browser.sentry-cdn.com',
    ],
  },
  // Save snapshots to files by default (token-efficient)
  outputMode: 'stdout',
  timeouts: {
    action: 10000,
    navigation: 30000,
  },
};

// Write config to temp location (or use existing if user provides --config)
const hasUserConfig = process.argv.some(a => a.startsWith('--config'));
if (!hasUserConfig) {
  const configDir = path.join(__dirname, '.playwright');
  if (!fs.existsSync(configDir)) fs.mkdirSync(configDir, { recursive: true });
  const configPath = path.join(configDir, 'cli.config.json');
  fs.writeFileSync(configPath, JSON.stringify(config, null, 2));
  // Inject --config flag before any command
  const cmdIdx = process.argv.findIndex((a, i) => i >= 2 && !a.startsWith('-'));
  if (cmdIdx >= 0) {
    process.argv.splice(cmdIdx, 0, `--config=${configPath}`);
  } else {
    process.argv.push(`--config=${configPath}`);
  }
}

// ── Delegate to the standard Playwright CLI ──────────────────────────────────

const { program } = require('playwright/lib/cli/client/program');
program().catch((err) => {
  console.error(err.message);
  process.exitCode = 1;
});
