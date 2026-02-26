'use strict';

/**
 * Patches CRSession prototype with lazy execution context methods.
 * These allow on-demand context creation without Runtime.enable.
 *
 * @param {object} mod - crConnection module exports ({ CRSession, CRConnection, ... })
 * @param {object} ctx - shared context between patches
 */
module.exports = function patchCrConnection(mod, ctx) {
  const { CRSession } = mod;
  if (!CRSession || !CRSession.prototype) {
    console.warn('[stealth-loader] CRSession not found in crConnection exports, skipping patch');
    return;
  }

  // Store reference for other patches
  ctx.CRSession = CRSession;

  // ── __re__emitExecutionContext ──────────────────────────────────────────────
  CRSession.prototype.__re__emitExecutionContext = async function ({ world, targetId, frame = null, utilityWorldName: passedUtilityWorldName }) {
    const fixMode = process.env['REBROWSER_PATCHES_RUNTIME_FIX_MODE'] || 'addBinding';
    const cdpWorldName = process.env['REBROWSER_PATCHES_UTILITY_WORLD_NAME'] !== '0'
      ? (process.env['REBROWSER_PATCHES_UTILITY_WORLD_NAME'] || 'util')
      : '__playwright_utility_world__';
    const contextUtilityName = passedUtilityWorldName || '__playwright_utility_world__';

    let getWorldPromise;
    if (fixMode === 'addBinding') {
      if (world === 'utility') {
        getWorldPromise = this.__re__getIsolatedWorld({ client: this, frameId: targetId, worldName: cdpWorldName })
          .then((contextId) => ({
            id: contextId,
            name: contextUtilityName,
            auxData: { frameId: targetId, isDefault: false },
          }));
      } else if (world === 'main') {
        getWorldPromise = this.__re__getMainWorld({ client: this, frameId: targetId, isWorker: frame === null })
          .then((contextId) => ({
            id: contextId,
            name: '',
            auxData: { frameId: targetId, isDefault: true },
          }));
      }
    } else if (fixMode === 'alwaysIsolated') {
      getWorldPromise = this.__re__getIsolatedWorld({ client: this, frameId: targetId, worldName: contextUtilityName })
        .then((contextId) => ({
          id: contextId,
          name: '',
          auxData: { frameId: targetId, isDefault: true },
        }));
    }

    const contextPayload = await getWorldPromise;
    this.emit('Runtime.executionContextCreated', { context: contextPayload });
  };

  // ── __re__getMainWorld ─────────────────────────────────────────────────────
  CRSession.prototype.__re__getMainWorld = async function ({ client, frameId, isWorker = false }) {
    let contextId;
    const randomName = [...Array(Math.floor(Math.random() * 11) + 10)]
      .map(() => Math.random().toString(36)[2])
      .join('');

    await client.send('Runtime.addBinding', { name: randomName });

    const bindingCalledHandler = ({ name, payload, executionContextId }) => {
      if (contextId > 0 || name !== randomName || payload !== frameId) return;
      contextId = executionContextId;
      client.off('Runtime.bindingCalled', bindingCalledHandler);
    };
    client.on('Runtime.bindingCalled', bindingCalledHandler);

    if (isWorker) {
      await client.send('Runtime.evaluate', {
        expression: `this['${randomName}']('${frameId}')`,
      });
    } else {
      await client.send('Page.addScriptToEvaluateOnNewDocument', {
        source: `document.addEventListener('${randomName}', (e) => self['${randomName}'](e.detail.frameId))`,
        runImmediately: true,
      });
      const createIsolatedWorldResult = await client.send('Page.createIsolatedWorld', {
        frameId,
        worldName: randomName,
        grantUniveralAccess: true,
      });
      await client.send('Runtime.evaluate', {
        expression: `document.dispatchEvent(new CustomEvent('${randomName}', { detail: { frameId: '${frameId}' } }))`,
        contextId: createIsolatedWorldResult.executionContextId,
      });
    }

    return contextId;
  };

  // ── __re__getIsolatedWorld ─────────────────────────────────────────────────
  CRSession.prototype.__re__getIsolatedWorld = async function ({ client, frameId, worldName }) {
    const result = await client.send('Page.createIsolatedWorld', {
      frameId,
      worldName,
      grantUniveralAccess: true,
    });
    return result.executionContextId;
  };
};
