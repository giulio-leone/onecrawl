'use strict';

/**
 * Patch registry — maps playwright-core server-relative paths
 * (without .js extension) to their patch functions.
 *
 * Used by stealth-loader.js to apply the right patch when a
 * matching module is loaded.
 */
module.exports = {
  'chromium/crConnection': require('./cr-connection.patch'),
  'chromium/crPage':       require('./cr-page.patch'),
  'frames':                require('./frames.patch'),
  'page':                  require('./page.patch'),
};
