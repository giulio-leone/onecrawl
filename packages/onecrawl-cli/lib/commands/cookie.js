'use strict';

/**
 * cookie command — manage browser cookies.
 *
 * Usage:
 *   onecrawl-cli cookie list [--domain=<filter>]
 *   onecrawl-cli cookie export <file> [--domain=<filter>]
 *   onecrawl-cli cookie import <file>
 *   onecrawl-cli cookie clear [--domain=<filter>]
 *
 * @module commands/cookie
 */

const { runSessionCommand, withErrorHandling } = require('../session-helper');
const fs = require('node:fs');
const path = require('node:path');

/**
 * @param {import('./index').CommandRegistry} registry
 */
function register(registry) {
  registry.add({
    name: 'cookie',
    description: 'manage browser cookies (list|export|import|clear)',
    usage: '<sub-command> [file] [--domain=<filter>]',
    action: cookieAction,
  });
}

/**
 * @param {Object} args - Parsed minimist args
 */
async function cookieAction(args) {
  await withErrorHandling(async () => {
    const subCommand = args._[1];

    if (!subCommand) {
      console.error(
        'Usage:\n' +
        '  onecrawl-cli cookie list [--domain=<filter>]\n' +
        '  onecrawl-cli cookie export <file> [--domain=<filter>]\n' +
        '  onecrawl-cli cookie import <file>\n' +
        '  onecrawl-cli cookie clear [--domain=<filter>]'
      );
      process.exit(1);
    }

    switch (subCommand) {
      case 'list':
        return await listCookies(args);
      case 'export':
        return await exportCookies(args);
      case 'import':
        return await importCookies(args);
      case 'clear':
        return await clearCookies(args);
      default:
        throw new Error(`Unknown cookie sub-command: '${subCommand}'. Use list, export, import, or clear.`);
    }
  });
}

/**
 * Retrieve cookies from the browser and optionally filter by domain.
 */
async function getCookiesFromBrowser(args) {
  const js = `(() => {
    const cookies = document.cookie.split(';').map(c => {
      const [k, ...v] = c.split('=');
      return { name: k.trim(), value: v.join('=').trim(), domain: window.location.hostname, path: '/' };
    }).filter(c => c.name);
    return JSON.stringify(cookies);
  })()`;

  const result = await runSessionCommand({
    _: ['evaluate', js],
    session: args.session,
  });

  let cookies = [];
  try {
    cookies = JSON.parse(result.text);
  } catch {
    cookies = [];
  }

  const domainFilter = args.domain;
  if (domainFilter) {
    cookies = cookies.filter(c =>
      c.domain && c.domain.includes(domainFilter)
    );
  }

  return cookies;
}

async function listCookies(args) {
  const cookies = await getCookiesFromBrowser(args);
  console.log(JSON.stringify(cookies, null, 2));
}

async function exportCookies(args) {
  const file = args._[2];
  if (!file) {
    throw new Error('File path is required. Usage: onecrawl-cli cookie export <file> [--domain=<filter>]');
  }

  const cookies = await getCookiesFromBrowser(args);
  const outputPath = path.resolve(file);

  // Ensure parent directory exists
  const dir = path.dirname(outputPath);
  fs.mkdirSync(dir, { recursive: true });

  fs.writeFileSync(outputPath, JSON.stringify(cookies, null, 2), 'utf8');
  console.log(JSON.stringify({ exported: true, path: outputPath, count: cookies.length }));
}

async function importCookies(args) {
  const file = args._[2];
  if (!file) {
    throw new Error('File path is required. Usage: onecrawl-cli cookie import <file>');
  }

  const filePath = path.resolve(file);
  if (!fs.existsSync(filePath)) {
    throw new Error(`Cookie file not found: ${filePath}`);
  }

  let cookies;
  try {
    cookies = JSON.parse(fs.readFileSync(filePath, 'utf8'));
  } catch (err) {
    throw new Error(`Failed to parse cookie file: ${err.message}`);
  }

  if (!Array.isArray(cookies)) {
    throw new Error('Cookie file must contain a JSON array of cookie objects.');
  }

  let imported = 0;
  for (const cookie of cookies) {
    if (!cookie.name) continue;

    // Prefer the Playwright CLI cookie-set command when domain/path are available
    if (cookie.domain && cookie.path) {
      try {
        await runSessionCommand({
          _: ['cookie-set', cookie.name, cookie.value || '',
               `--domain=${cookie.domain}`, `--path=${cookie.path}`],
          session: args.session,
        });
        imported++;
        continue;
      } catch {
        // Fall through to document.cookie injection
      }
    }

    // Fallback: inject via document.cookie
    const cookieStr = `${cookie.name}=${cookie.value || ''}` +
      (cookie.path ? `; path=${cookie.path}` : '') +
      (cookie.domain ? `; domain=${cookie.domain}` : '');

    await runSessionCommand({
      _: ['evaluate', `document.cookie = ${JSON.stringify(cookieStr)}`],
      session: args.session,
    });
    imported++;
  }

  console.log(JSON.stringify({ imported: true, count: imported }));
}

async function clearCookies(args) {
  const domainFilter = args.domain;

  // Get current cookies to clear them by setting expiry in the past
  const js = `(() => {
    const cookies = document.cookie.split(';').map(c => c.trim().split('=')[0]).filter(Boolean);
    cookies.forEach(name => {
      document.cookie = name + '=; expires=Thu, 01 Jan 1970 00:00:00 GMT; path=/';
      document.cookie = name + '=; expires=Thu, 01 Jan 1970 00:00:00 GMT; path=/; domain=' + window.location.hostname;
    });
    return JSON.stringify({ cleared: cookies.length });
  })()`;

  if (domainFilter) {
    // With domain filter: get cookies first, then selectively clear
    const cookies = await getCookiesFromBrowser(args);
    let cleared = 0;
    for (const cookie of cookies) {
      const clearJs = `(() => {
        document.cookie = ${JSON.stringify(cookie.name)} + '=; expires=Thu, 01 Jan 1970 00:00:00 GMT; path=/';
        document.cookie = ${JSON.stringify(cookie.name)} + '=; expires=Thu, 01 Jan 1970 00:00:00 GMT; path=/; domain=${domainFilter}';
        return 'ok';
      })()`;
      await runSessionCommand({
        _: ['evaluate', clearJs],
        session: args.session,
      });
      cleared++;
    }
    console.log(JSON.stringify({ cleared: true, count: cleared, domain: domainFilter }));
  } else {
    // Clear all cookies
    const result = await runSessionCommand({
      _: ['evaluate', js],
      session: args.session,
    });

    let info = { cleared: 0 };
    try { info = JSON.parse(result.text); } catch { /* keep default */ }
    console.log(JSON.stringify({ cleared: true, count: info.cleared }));
  }
}

module.exports = { register };
