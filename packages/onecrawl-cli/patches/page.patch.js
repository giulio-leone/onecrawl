'use strict';

/**
 * Patches Worker and PageBinding from the page module:
 *  - Worker: adds _targetId / _session props and getExecutionContext()
 *  - PageBinding.dispatch: guards against non-JSON payloads
 *
 * @param {object} mod - page module exports ({ Page, Worker, PageBinding, ... })
 * @param {object} _ctx - shared context (unused)
 */
module.exports = function patchPage(mod, _ctx) {
  const { Worker, PageBinding } = mod;

  // ── Worker patches ─────────────────────────────────────────────────────────
  if (Worker && Worker.prototype) {
    // Wrap constructor to inject _targetId and _session
    const OrigWorker = Worker;
    const origConstruct = OrigWorker.prototype.constructor;

    // We can't easily replace the constructor in CJS modules with esbuild output,
    // so instead we wrap createExecutionContext which is always called after construction.
    // But _targetId/_session are set externally by crPage (e.g. worker._targetId = targetId).
    // We only need to ensure the properties exist with defaults.
    if (!('_targetId' in OrigWorker.prototype)) {
      Object.defineProperty(OrigWorker.prototype, '_targetId', {
        value: null,
        writable: true,
        configurable: true,
        enumerable: false,
      });
    }
    if (!('_session' in OrigWorker.prototype)) {
      Object.defineProperty(OrigWorker.prototype, '_session', {
        value: null,
        writable: true,
        configurable: true,
        enumerable: false,
      });
    }

    // Add getExecutionContext() for lazy context creation in workers
    if (!OrigWorker.prototype.getExecutionContext) {
      OrigWorker.prototype.getExecutionContext = async function () {
        if (process.env['REBROWSER_PATCHES_RUNTIME_FIX_MODE'] !== '0' && !this.existingExecutionContext) {
          if (this._session && this._session.__re__emitExecutionContext) {
            await this._session.__re__emitExecutionContext({
              world: 'main',
              targetId: this._targetId,
            });
          }
        }
        return this._executionContextPromise;
      };
    }
  } else {
    console.warn('[stealth-loader] Worker not found in page exports, skipping patch');
  }

  // ── PageBinding.dispatch guard ─────────────────────────────────────────────
  if (PageBinding && typeof PageBinding.dispatch === 'function') {
    const origDispatch = PageBinding.dispatch;
    PageBinding.dispatch = async function (page, payload, context) {
      if (process.env['REBROWSER_PATCHES_RUNTIME_FIX_MODE'] !== '0' && !payload.includes('{')) {
        return;
      }
      return origDispatch.call(this, page, payload, context);
    };
  } else {
    console.warn('[stealth-loader] PageBinding.dispatch not found, skipping patch');
  }
};
