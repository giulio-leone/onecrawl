'use strict';

/**
 * stealth-loader.js — Runtime module interceptor for rebrowser patches.
 *
 * Hooks Node's Module._load to intercept playwright-core internal modules
 * and apply in-memory prototype patches. Never modifies files on disk.
 *
 * MUST be required before any playwright module is loaded.
 */

const Module = require('module');
const path = require('path');
const fs = require('fs');

// ── Resolve playwright-core server directory ─────────────────────────────────
let serverDir;
let pcVersion;

try {
  const pwPkg = require.resolve('playwright/package.json');
  const pcDir = path.join(path.dirname(pwPkg), '..', 'playwright-core');
  const pcPkg = require(path.join(pcDir, 'package.json'));
  pcVersion = pcPkg.version;
  // Use realpath to resolve pnpm/hoisted symlinks so paths match Module._resolveFilename
  serverDir = fs.realpathSync(path.join(pcDir, 'lib', 'server'));
} catch (e) {
  console.warn('[stealth-loader] playwright-core not found, runtime patches disabled');
}

// ── Version gate ─────────────────────────────────────────────────────────────
if (serverDir && pcVersion) {
  const [major, minor] = pcVersion.split('.').map(Number);
  if (major < 1 || (major === 1 && minor < 59)) {
    console.warn(
      `[stealth-loader] playwright-core ${pcVersion} < 1.59.0 — ` +
      'runtime patches may not work correctly'
    );
  }
}

// ── Build target map: absolute path → patch key ──────────────────────────────
const patches = serverDir ? require('./patches') : {};
const targetMap = new Map();
const applied = new Set();

if (serverDir) {
  for (const key of Object.keys(patches)) {
    const absPath = path.join(serverDir, key + '.js');
    targetMap.set(absPath, key);
  }
}

// Shared context passed between patches (e.g. CRSession reference)
const ctx = {};

// ── Hook Module._load ────────────────────────────────────────────────────────
if (serverDir && targetMap.size > 0) {
  const origResolve = Module._resolveFilename;
  const origLoad = Module._load;

  Module._load = function (request, parent, isMain) {
    const result = origLoad.apply(this, arguments);

    // All patches applied — nothing left to do
    if (applied.size >= targetMap.size) return result;

    // Quick filter: skip loads that can't be playwright-core targets
    if (!parent || !parent.filename) return result;
    if (!request.startsWith('.') && !request.includes('playwright')) return result;

    // Resolve the filename the same way Node did
    let resolved;
    try {
      resolved = origResolve(request, parent, isMain);
    } catch (_) {
      return result;
    }

    const patchKey = targetMap.get(resolved);
    if (patchKey && !applied.has(patchKey)) {
      applied.add(patchKey);
      try {
        patches[patchKey](result, ctx);
      } catch (err) {
        console.error(`[stealth-loader] Failed to apply patch "${patchKey}":`, err.message);
      }
    }

    return result;
  };
}

module.exports = { applied, ctx };
