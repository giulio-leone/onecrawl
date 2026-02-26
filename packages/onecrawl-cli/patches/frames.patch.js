'use strict';

/**
 * Replaces Frame.prototype._context with a lazy variant that creates
 * execution contexts on-demand via CRSession.__re__emitExecutionContext
 * instead of relying on the Runtime.enable event stream.
 *
 * @param {object} mod - frames module exports ({ Frame, FrameManager, ... })
 * @param {object} _ctx - shared context (unused)
 */
module.exports = function patchFrames(mod, _ctx) {
  const { Frame } = mod;
  if (!Frame || !Frame.prototype) {
    console.warn('[stealth-loader] Frame not found in frames exports, skipping patch');
    return;
  }

  // Capture the js.ExecutionContext class for instanceof checks.
  // We resolve the javascript module from the same directory as frames.js.
  let ExecutionContext;
  try {
    const path = require('path');
    const fs = require('fs');
    const pwPkg = require.resolve('playwright/package.json');
    const pcDir = fs.realpathSync(path.join(path.dirname(pwPkg), '..', 'playwright-core'));
    const jsModule = require(path.join(pcDir, 'lib', 'server', 'javascript.js'));
    ExecutionContext = jsModule.ExecutionContext;
  } catch (e) {
    // Fallback: duck-type detection (see below)
  }

  Frame.prototype._context = function _context(world, useContextPromise = false) {
    // Fast path: disabled, context already exists, or explicitly requested
    if (
      process.env['REBROWSER_PATCHES_RUNTIME_FIX_MODE'] === '0' ||
      this._contextData.get(world).context ||
      useContextPromise
    ) {
      return this._contextData.get(world).contextPromise.then((contextOrDestroyedReason) => {
        if (ExecutionContext && contextOrDestroyedReason instanceof ExecutionContext)
          return contextOrDestroyedReason;
        // Fallback: duck-type check
        if (!ExecutionContext && contextOrDestroyedReason && typeof contextOrDestroyedReason.evaluate === 'function')
          return contextOrDestroyedReason;
        throw new Error(contextOrDestroyedReason.destroyedReason);
      });
    }

    // Lazy path: request context creation from CRSession
    const sessions = this._page.delegate._sessions || (this._page._delegate && this._page._delegate._sessions);
    const frameSession = (sessions && (sessions.get(this._id) || Array.from(sessions.values())[0])) ||
      this._page.delegate._mainFrameSession ||
      (this._page._delegate && this._page._delegate._mainFrameSession);
    const crSession = frameSession._client;

    // utilityWorldName is dynamic in 1.59+ (includes page GUID)
    const utilityWorldName = (this._page.delegate && this._page.delegate.utilityWorldName) || '__playwright_utility_world__';

    return crSession.__re__emitExecutionContext({ world, targetId: this._id, frame: this, utilityWorldName })
      .then(() => this._context(world, true))
      .catch((error) => {
        if (error && error.message && error.message.includes('No frame for given id found')) {
          return { destroyedReason: 'Frame was detached' };
        }
        throw error;
      });
  };
};
