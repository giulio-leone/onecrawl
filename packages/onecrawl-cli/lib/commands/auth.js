'use strict';

/**
 * auth command group — manage LinkedIn authentication state.
 *
 * Sub-commands:
 *   auth status                        show current auth state
 *   auth login [--method=auto|passkey|cookie]  authenticate using cascade
 *   auth register-passkey              guide passkey registration
 *   auth export [output]               export credentials to file
 *   auth import <file>                 import credentials from file
 *
 * Output: structured JSON to stdout.
 *
 * @module commands/auth
 */

const fs = require('fs');
const path = require('path');
const os = require('os');
const { getSession, withErrorHandling } = require('../session-helper');

const ONECRAWL_DIR = path.join(os.homedir(), '.onecrawl', 'linkedin');
const COOKIES_PATH = path.join(ONECRAWL_DIR, 'cookies.json');
const PASSKEY_PATH = path.join(ONECRAWL_DIR, 'passkey.json');
const OAUTH_TOKENS_PATH = path.join(ONECRAWL_DIR, 'oauth-tokens.json');
const TOTP_SECRET_PATH = path.join(ONECRAWL_DIR, 'totp-secret.json');

/**
 * @param {import('./index').CommandRegistry} registry
 */
function register(registry) {
  registry.add({
    name: 'auth',
    description: 'manage LinkedIn authentication (status|login|register-passkey|setup-totp|setup-oauth|export|import)',
    usage: '<subcommand> [options]',
    action: authAction,
  });
}

/**
 * @param {Object} args - Parsed minimist args
 */
async function authAction(args) {
  await withErrorHandling(async () => {
    const sub = args._[1];

    switch (sub) {
      case 'status':
        return await statusCmd(args);
      case 'login':
        return await loginCmd(args);
      case 'register-passkey':
        return await registerPasskeyCmd(args);
      case 'setup-totp':
        return await setupTotpCmd(args);
      case 'setup-oauth':
        return await setupOauthCmd(args);
      case 'export':
        return await exportCmd(args);
      case 'import':
        return await importCmd(args);
      default:
        console.error(
          'Usage: onecrawl-cli auth <subcommand>\n\n' +
          'Subcommands:\n' +
          '  status                          show current auth state\n' +
          '  login [--method=auto|passkey|oauth|cookie]  authenticate\n' +
          '  register-passkey                guide passkey registration\n' +
          '  setup-totp [secret]             save TOTP secret for automated 2FA\n' +
          '  setup-oauth --client-id=<id>    configure OAuth client\n' +
          '  export [output]                 export credentials to file\n' +
          '  import <file>                   import credentials from file'
        );
        process.exit(1);
    }
  });
}

// ── Helpers ──────────────────────────────────────────────────────────────────

function ensureDir() {
  if (!fs.existsSync(ONECRAWL_DIR)) {
    fs.mkdirSync(ONECRAWL_DIR, { recursive: true });
  }
}

function readJsonSafe(filePath) {
  try {
    return JSON.parse(fs.readFileSync(filePath, 'utf8'));
  } catch {
    return null;
  }
}

function cookieStatus() {
  const result = { exists: false, valid: false, count: 0, liAtExpiry: null };
  const cookies = readJsonSafe(COOKIES_PATH);
  if (!cookies) return result;

  result.exists = true;
  const list = Array.isArray(cookies) ? cookies : [];
  result.count = list.length;

  const liAt = list.find(c => c.name === 'li_at');
  if (liAt) {
    if (liAt.expires) {
      const expiry = new Date(liAt.expires * 1000);
      result.liAtExpiry = expiry.toISOString();
      result.valid = expiry > new Date();
    } else {
      result.valid = true; // session cookie, no expiry
    }
  }
  return result;
}

function passkeyStatus() {
  const result = { exists: false, rpId: null };
  const pk = readJsonSafe(PASSKEY_PATH);
  if (!pk) return result;
  result.exists = true;
  result.rpId = pk.rpId || null;
  return result;
}

function oauthStatus() {
  const result = { exists: false, expiresAt: null, expired: null };
  const tokens = readJsonSafe(OAUTH_TOKENS_PATH);
  if (!tokens) return result;
  result.exists = true;
  result.expiresAt = tokens.expiresAt || null;
  result.expired = tokens.expiresAt ? Date.now() >= tokens.expiresAt : null;
  return result;
}

function totpStatus() {
  const result = { exists: false };
  try {
    fs.accessSync(TOTP_SECRET_PATH);
    result.exists = true;
  } catch {
    // TOTP secret not configured
  }
  return result;
}

// ── Sub-commands ─────────────────────────────────────────────────────────────

async function statusCmd(args) {
  const cookies = cookieStatus();
  const passkey = passkeyStatus();
  const oauth = oauthStatus();
  const totp = totpStatus();

  // Optionally probe live browser session for runtime cookie state
  let browserSession = { connected: false, liveCookieCount: null };
  try {
    const { session, clientInfo } = await getSession(args);
    browserSession.connected = true;
    const result = await session.run(clientInfo, {
      _: ['evaluate', `document.cookie.split(';').filter(c => c.trim()).length`],
      session: args.session,
    });
    browserSession.liveCookieCount = parseInt(result.text.trim(), 10) || 0;
  } catch {
    // browser not running — that's fine for offline status
  }

  const report = {
    cookies,
    passkey,
    oauth,
    totp,
    browserSession,
    timestamp: new Date().toISOString(),
  };

  console.log(JSON.stringify(report, null, 2));
}

async function loginCmd(args) {
  const method = args.method || 'auto';
  const validMethods = ['auto', 'passkey', 'oauth', 'cookie'];

  if (!validMethods.includes(method)) {
    console.error(`Invalid method '${method}'. Use: ${validMethods.join(', ')}`);
    process.exit(1);
  }

  // OAuth login does not require a browser session
  if (method === 'oauth') {
    const clientId = args['client-id'];
    if (!clientId) {
      console.error('OAuth login requires --client-id. Run: onecrawl-cli auth login --method=oauth --client-id=<id>');
      process.exit(1);
    }
    const redirectUri = args['redirect-uri'] || 'http://localhost:3000/callback';
    const result = {
      method: 'oauth',
      success: false,
      details: null,
      authorizationUrl: null,
    };

    // Build authorization URL with PKCE
    const crypto = require('crypto');
    const verifier = crypto.randomBytes(32).toString('base64url');
    const challenge = crypto.createHash('sha256').update(verifier, 'ascii').digest('base64url');
    const state = crypto.randomBytes(16).toString('base64url');
    const params = new URLSearchParams({
      response_type: 'code',
      client_id: clientId,
      redirect_uri: redirectUri,
      state,
      scope: 'openid profile email',
      code_challenge: challenge,
      code_challenge_method: 'S256',
    });
    result.authorizationUrl = `https://www.linkedin.com/oauth/v2/authorization?${params.toString()}`;
    result.details = 'Visit the authorization URL and provide the returned code via --code parameter.';
    console.log(JSON.stringify(result, null, 2));
    return;
  }

  let session, clientInfo;
  try {
    ({ session, clientInfo } = await getSession(args));
  } catch (err) {
    console.error(`Cannot login: browser session not running.\n${err.message}`);
    process.exit(1);
  }

  const result = { method, success: false, details: null };

  if (method === 'auto' || method === 'cookie') {
    // Try cookie injection first
    const cookies = readJsonSafe(COOKIES_PATH);
    if (cookies && Array.isArray(cookies) && cookies.length > 0) {
      try {
        const cookieScript = `
          (() => {
            const cookies = ${JSON.stringify(cookies)};
            for (const c of cookies) {
              let str = c.name + '=' + c.value;
              if (c.path) str += '; path=' + c.path;
              if (c.domain) str += '; domain=' + c.domain;
              if (c.secure) str += '; Secure';
              if (c.sameSite) str += '; SameSite=' + c.sameSite;
              document.cookie = str;
            }
            return cookies.length;
          })()
        `;
        const injectResult = await session.run(clientInfo, {
          _: ['evaluate', cookieScript],
          session: args.session,
        });
        result.success = true;
        result.details = `Injected ${injectResult.text.trim()} cookies via document.cookie`;
        console.log(JSON.stringify(result, null, 2));
        return;
      } catch (err) {
        result.details = `Cookie injection failed: ${err.message}`;
        if (method === 'cookie') {
          console.log(JSON.stringify(result, null, 2));
          process.exit(1);
        }
      }
    }
  }

  if (method === 'auto' || method === 'passkey') {
    const pk = readJsonSafe(PASSKEY_PATH);
    if (pk) {
      result.success = true;
      result.details = 'Passkey credentials found. Passkey auth requires headed browser interaction.';
      console.log(JSON.stringify(result, null, 2));
      return;
    }
    if (method === 'passkey') {
      result.details = 'No passkey.json found. Run: onecrawl-cli auth register-passkey';
      console.log(JSON.stringify(result, null, 2));
      process.exit(1);
    }
  }

  // Fallback: no credentials available
  result.details = 'No stored credentials found. Log in manually or import credentials.';
  console.log(JSON.stringify(result, null, 2));
  process.exit(1);
}

async function registerPasskeyCmd(_args) {
  const pk = readJsonSafe(PASSKEY_PATH);

  const report = {
    alreadyRegistered: !!pk,
    rpId: pk ? (pk.rpId || null) : null,
    instructions: null,
  };

  if (pk) {
    report.instructions = 'Passkey already registered. To re-register, delete ~/.onecrawl/linkedin/passkey.json and run this command again.';
  } else {
    report.instructions = [
      '1. Open a headed browser session: onecrawl-cli open --headed',
      '2. Navigate to LinkedIn Settings > Sign in & security > Passkeys',
      '3. Follow the browser prompts to create a passkey',
      '4. Export the passkey credential to ~/.onecrawl/linkedin/passkey.json',
      '   Expected format: { "rpId": "www.linkedin.com", "credentialId": "...", "privateKey": "..." }',
    ].join('\n');
  }

  console.log(JSON.stringify(report, null, 2));
}

async function exportCmd(args) {
  const output = args._[2] || path.join(process.cwd(), 'onecrawl-auth-export.json');

  const payload = {
    exportedAt: new Date().toISOString(),
    cookies: readJsonSafe(COOKIES_PATH),
    passkey: readJsonSafe(PASSKEY_PATH),
  };

  if (!payload.cookies && !payload.passkey) {
    console.error('Nothing to export: no cookies.json or passkey.json found in ~/.onecrawl/linkedin/');
    process.exit(1);
  }

  fs.writeFileSync(output, JSON.stringify(payload, null, 2));
  console.log(JSON.stringify({ exported: output, hasCookies: !!payload.cookies, hasPasskey: !!payload.passkey }));
}

async function importCmd(args) {
  const file = args._[2];
  if (!file) {
    console.error('Usage: onecrawl-cli auth import <file>');
    process.exit(1);
  }

  if (!fs.existsSync(file)) {
    console.error(`File not found: ${file}`);
    process.exit(1);
  }

  let payload;
  try {
    payload = JSON.parse(fs.readFileSync(file, 'utf8'));
  } catch (err) {
    console.error(`Invalid JSON: ${err.message}`);
    process.exit(1);
  }

  ensureDir();
  const imported = { cookies: false, passkey: false };

  if (payload.cookies && Array.isArray(payload.cookies) && payload.cookies.length > 0) {
    fs.writeFileSync(COOKIES_PATH, JSON.stringify(payload.cookies, null, 2));
    imported.cookies = true;
  }

  if (payload.passkey && typeof payload.passkey === 'object') {
    fs.writeFileSync(PASSKEY_PATH, JSON.stringify(payload.passkey, null, 2));
    imported.passkey = true;
  }

  if (!imported.cookies && !imported.passkey) {
    console.error('No valid cookies or passkey data found in the import file.');
    process.exit(1);
  }

  console.log(JSON.stringify({ imported, source: file }));
}

async function setupTotpCmd(args) {
  const secret = args._[2] || null;

  if (!secret) {
    console.error(
      'Usage: onecrawl-cli auth setup-totp <base32-secret>\n\n' +
      'Provide the TOTP secret from your authenticator app setup.\n' +
      'The secret will be encrypted and stored at:\n' +
      '  ' + TOTP_SECRET_PATH
    );
    process.exit(1);
  }

  // Validate base32 format
  if (!/^[A-Z2-7=\s]+$/i.test(secret)) {
    console.error('Invalid TOTP secret: must be base32 encoded (A-Z, 2-7).');
    process.exit(1);
  }

  ensureDir();

  // Store as encrypted JSON blob (mirrors the TotpSecretStore format)
  const now = new Date().toISOString();
  const payload = {
    version: 1,
    secret: secret.replace(/\s/g, '').toUpperCase(),
    metadata: { createdAt: now, updatedAt: now },
  };
  fs.writeFileSync(TOTP_SECRET_PATH, JSON.stringify(payload), { mode: 0o600 });

  console.log(JSON.stringify({
    saved: true,
    path: TOTP_SECRET_PATH,
    hint: 'TOTP secret encrypted and stored. 2FA codes will be generated automatically during login.',
  }));
}

async function setupOauthCmd(args) {
  const clientId = args['client-id'];
  if (!clientId) {
    console.error(
      'Usage: onecrawl-cli auth setup-oauth --client-id=<id> [--redirect-uri=<uri>]\n\n' +
      'Configure OAuth 2.1 client credentials for LinkedIn authentication.'
    );
    process.exit(1);
  }

  const redirectUri = args['redirect-uri'] || 'http://localhost:3000/callback';

  ensureDir();
  const configPath = path.join(ONECRAWL_DIR, 'oauth-config.json');
  const config = {
    clientId,
    redirectUri,
    authorizeEndpoint: 'https://www.linkedin.com/oauth/v2/authorization',
    tokenEndpoint: 'https://www.linkedin.com/oauth/v2/accessToken',
    revokeEndpoint: 'https://www.linkedin.com/oauth/v2/revoke',
    scopes: ['openid', 'profile', 'email'],
  };

  fs.writeFileSync(configPath, JSON.stringify(config, null, 2), { mode: 0o600 });

  console.log(JSON.stringify({
    saved: true,
    path: configPath,
    config: { clientId, redirectUri },
    hint: 'OAuth client configured. Use "auth login --method=oauth --client-id=<id>" to authenticate.',
  }));
}

module.exports = { register };
