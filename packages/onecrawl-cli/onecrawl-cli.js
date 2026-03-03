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

// ── Load custom OneCrawl commands ─────────────────────────────────────────────

const { loadAllCommands, formatCustomHelp } = require('./lib/commands');
const customCommands = loadAllCommands();

// ── Route: custom command or Playwright CLI ──────────────────────────────────

// Extract the command name from argv, skipping flag values.
// Global flags that consume the next arg: -s/--session, --browser, --config, --profile, --extension
const _valueFlagSet = new Set(['-s', '--session', '--browser', '--config', '--profile', '--extension']);
let _cmdName = null;
for (let _i = 2; _i < process.argv.length; _i++) {
  const _a = process.argv[_i];
  if (_a.startsWith('-')) {
    // Skip the next arg if this flag consumes a value (and isn't --flag=val form)
    if (_valueFlagSet.has(_a) && !_a.includes('=')) _i++;
    continue;
  }
  _cmdName = _a;
  break;
}

if (_cmdName && customCommands.has(_cmdName)) {
  // Lightweight argv parser (avoids external minimist dependency)
  const _argv = process.argv.slice(2);
  const args = { _: [] };
  for (let i = 0; i < _argv.length; i++) {
    const a = _argv[i];
    if (a.startsWith('--') && a.includes('=')) {
      const [k, ...v] = a.slice(2).split('=');
      args[k] = v.join('=');
    } else if (a.startsWith('--no-')) {
      args[a.slice(5)] = false;
    } else if (a.startsWith('--')) {
      const next = _argv[i + 1];
      if (next && !next.startsWith('-')) { args[a.slice(2)] = next; i++; }
      else args[a.slice(2)] = true;
    } else if (a.startsWith('-') && a.length === 2) {
      const next = _argv[i + 1];
      if (next && !next.startsWith('-')) { args[a.slice(1)] = next; i++; }
      else args[a.slice(1)] = true;
    } else {
      args._.push(a);
    }
  }
  if (args.s) { args.session = args.s; delete args.s; }

  const cmd = customCommands.get(_cmdName);
  cmd.action(args).catch((err) => {
    console.error(`onecrawl: ${err.message}`);
    process.exitCode = 1;
  });
} else {
  // Inject custom help lines when --help is used without a specific command
  if (process.argv.includes('--help') || process.argv.includes('-h')) {
    const helpBlock = formatCustomHelp(customCommands);
    if (helpBlock) {
      // Monkey-patch console.log once to append custom commands to global help
      const _origLog = console.log;
      let _patched = false;
      console.log = function (...logArgs) {
        _origLog.apply(console, logArgs);
        if (!_patched && typeof logArgs[0] === 'string' && logArgs[0].includes('Global options:')) {
          _patched = true;
          _origLog(helpBlock);
          console.log = _origLog;
        }
      };
    }
  }

  const { program } = require('playwright/lib/cli/client/program');
  program().catch((err) => {
    console.error(err.message);
    process.exitCode = 1;
  });
}
