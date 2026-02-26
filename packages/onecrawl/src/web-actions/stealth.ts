/**
 * Stealth module — anti-detection patches for Playwright browser contexts.
 *
 * Applies runtime JS patches via `context.addInitScript()` to make automated
 * Chrome indistinguishable from a regular user session.
 *
 * Patches:
 *  1. navigator.webdriver = false
 *  2. Chrome runtime + plugins spoofing
 *  3. Languages & platform consistency
 *  4. WebGL renderer/vendor spoofing
 *  5. Permissions API normalization
 *  6. Console.debug leak prevention
 *  7. User-Agent HeadlessChrome→Chrome fix
 *  8. Window outer dimensions (headless gives 0)
 *
 * Additionally applies UA override at CDP network level when available.
 */

import type { BrowserContext, CDPSession } from "rebrowser-playwright";

// ── Constants ────────────────────────────────────────────────────────────────

/** Realistic Chrome UA matching the installed Chrome Canary version. */
export const CHROME_UA =
  "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/147.0.7697.0 Safari/537.36";

/** Accept-Language header matching Italian locale with English fallback. */
const ACCEPT_LANGUAGE = "it-IT,it;q=0.9,en-US;q=0.8,en;q=0.7";

// ── Stealth Script ───────────────────────────────────────────────────────────

/**
 * Combined init script injected into every page BEFORE any page JS runs.
 * Single script avoids multiple addInitScript calls overhead.
 */
const STEALTH_INIT_SCRIPT = `
// ─── 1. navigator.webdriver ─────────────────────────────────────
Object.defineProperty(navigator, 'webdriver', {
  get: () => false,
  configurable: true,
});

// ─── 2. Chrome runtime + plugins ────────────────────────────────
if (!window.chrome) window.chrome = {};
if (!window.chrome.runtime) {
  window.chrome.runtime = {
    connect: function() {},
    sendMessage: function() {},
    onMessage: { addListener: function() {} },
  };
}
Object.defineProperty(navigator, 'plugins', {
  get: () => {
    const p = [
      { name: 'Chrome PDF Plugin', filename: 'internal-pdf-viewer', description: 'Portable Document Format' },
      { name: 'Chrome PDF Viewer', filename: 'mhjfbmdgcfjbbpaeojofohoefgiehjai', description: '' },
      { name: 'Native Client', filename: 'internal-nacl-plugin', description: '' },
    ];
    p.refresh = () => {};
    return p;
  },
  configurable: true,
});

// ─── 3. Languages ───────────────────────────────────────────────
Object.defineProperty(navigator, 'languages', {
  get: () => ['it-IT', 'it', 'en-US', 'en'],
  configurable: true,
});
Object.defineProperty(navigator, 'language', {
  get: () => 'it-IT',
  configurable: true,
});

// ─── 4. WebGL vendor/renderer ───────────────────────────────────
(function() {
  const proto = WebGLRenderingContext.prototype.getParameter;
  WebGLRenderingContext.prototype.getParameter = function(p) {
    if (p === 37445) return 'Intel Inc.';
    if (p === 37446) return 'Intel Iris OpenGL Engine';
    return proto.call(this, p);
  };
  if (typeof WebGL2RenderingContext !== 'undefined') {
    const proto2 = WebGL2RenderingContext.prototype.getParameter;
    WebGL2RenderingContext.prototype.getParameter = function(p) {
      if (p === 37445) return 'Intel Inc.';
      if (p === 37446) return 'Intel Iris OpenGL Engine';
      return proto2.call(this, p);
    };
  }
})();

// ─── 5. Permissions API ─────────────────────────────────────────
if (navigator.permissions) {
  const origQuery = navigator.permissions.query.bind(navigator.permissions);
  navigator.permissions.query = (params) => {
    if (params.name === 'notifications') {
      return Promise.resolve({ state: Notification.permission, onchange: null });
    }
    return origQuery(params);
  };
}

// ─── 6. Console.debug leak ──────────────────────────────────────
(function() {
  const origDebug = console.debug;
  console.debug = function(...args) {
    if (args[0]?.toString?.().includes('Headless')) return;
    return origDebug.apply(this, args);
  };
})();

// ─── 7. User-Agent HeadlessChrome fix ───────────────────────────
(function() {
  const ua = navigator.userAgent;
  if (ua.includes('Headless')) {
    Object.defineProperty(navigator, 'userAgent', {
      get: () => ua.replace('HeadlessChrome', 'Chrome'),
      configurable: true,
    });
  }
})();

// ─── 8. Window outer dimensions ─────────────────────────────────
if (window.outerWidth === 0) {
  Object.defineProperty(window, 'outerWidth', {
    get: () => window.innerWidth,
    configurable: true,
  });
  Object.defineProperty(window, 'outerHeight', {
    get: () => window.innerHeight + 85,
    configurable: true,
  });
}
`;

// ── Public API ───────────────────────────────────────────────────────────────

/**
 * Apply stealth patches to a Playwright BrowserContext.
 *
 * Injects JS patches via `addInitScript` (runs before any page script)
 * and optionally overrides UA at the CDP network level for HTTP headers.
 */
export async function applyStealthToContext(context: BrowserContext): Promise<void> {
  // JS-level patches (injected before every page load)
  await context.addInitScript(STEALTH_INIT_SCRIPT);

  // CDP-level UA override (affects HTTP request headers, not just JS)
  try {
    const existingPage = context.pages()[0];
    const createdPage = existingPage ? null : await context.newPage();
    const targetPage = existingPage ?? createdPage!;

    const cdp: CDPSession = await context.newCDPSession(targetPage);
    await cdp.send("Network.setUserAgentOverride", {
      userAgent: CHROME_UA,
      acceptLanguage: ACCEPT_LANGUAGE,
      platform: "macOS",
    });
    await cdp.detach();

    // Clean up page created solely for CDP session
    if (createdPage) await createdPage.close().catch(() => {});
  } catch {
    // CDP session may not be available (e.g. remote contexts) — JS patches still active
  }
}
