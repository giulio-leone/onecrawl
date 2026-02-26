/**
 * Stealth init script — injected into every browser page before any JS runs.
 *
 * This file is referenced by PLAYWRIGHT_MCP_INIT_SCRIPT and loaded by
 * Playwright's addInitScript({ path }) mechanism. It must be self-contained
 * (no require/import) as it runs inside the browser context.
 */

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
