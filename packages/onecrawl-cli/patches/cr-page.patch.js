'use strict';

/**
 * Intercepts Runtime.enable at the CRSession.send level.
 * This single patch covers crPage, crServiceWorker, and any other
 * code path that calls session.send('Runtime.enable').
 *
 * @param {object} _mod - crPage module exports (unused — we patch CRSession)
 * @param {object} ctx  - shared context (must contain ctx.CRSession)
 */
module.exports = function patchCrPage(_mod, ctx) {
  const CRSession = ctx.CRSession;
  if (!CRSession || !CRSession.prototype) {
    console.warn('[stealth-loader] CRSession not available in context, Runtime.enable bypass skipped');
    return;
  }

  // Guard against double-wrapping
  if (CRSession.prototype.__re__sendPatched) return;

  const origSend = CRSession.prototype.send;
  CRSession.prototype.send = function (method, params) {
    if (method === 'Runtime.enable' && process.env['REBROWSER_PATCHES_RUNTIME_FIX_MODE'] !== '0') {
      return Promise.resolve();
    }
    return origSend.call(this, method, params);
  };

  CRSession.prototype.__re__sendPatched = true;
};
