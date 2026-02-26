/**
 * Stealth utilities for anti-detection
 */

// ── User-Agent pools (Chrome 131-134 current) ──────────────────────────────

const CHROME_VERSIONS = ["131.0.6778.204", "132.0.6834.83", "133.0.6943.53", "134.0.6998.35"];
const FIREFOX_VERSIONS = ["133.0", "134.0"];
const SAFARI_VERSIONS = ["17.6", "18.0", "18.1"];

const PLATFORMS = [
  { os: "Windows NT 10.0; Win64; x64", platform: "Win32" },
  { os: "Windows NT 11.0; Win64; x64", platform: "Win32" },
  { os: "Macintosh; Intel Mac OS X 10_15_7", platform: "MacIntel" },
  { os: "Macintosh; Intel Mac OS X 14_0", platform: "MacIntel" },
  { os: "X11; Linux x86_64", platform: "Linux x86_64" },
];

function buildUserAgents(): string[] {
  const agents: string[] = [];
  for (const p of PLATFORMS) {
    for (const v of CHROME_VERSIONS) {
      agents.push(`Mozilla/5.0 (${p.os}) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/${v} Safari/537.36`);
    }
  }
  for (const v of FIREFOX_VERSIONS) {
    agents.push(`Mozilla/5.0 (Windows NT 10.0; Win64; x64; rv:${v}) Gecko/20100101 Firefox/${v}`);
    agents.push(`Mozilla/5.0 (Macintosh; Intel Mac OS X 10.15; rv:${v}) Gecko/20100101 Firefox/${v}`);
  }
  for (const v of SAFARI_VERSIONS) {
    agents.push(`Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/605.1.15 (KHTML, like Gecko) Version/${v} Safari/605.1.15`);
  }
  return agents;
}

const USER_AGENTS = buildUserAgents();

// ── Viewports ───────────────────────────────────────────────────────────────

const VIEWPORTS = [
  { width: 1920, height: 1080 },
  { width: 1366, height: 768 },
  { width: 1536, height: 864 },
  { width: 1440, height: 900 },
  { width: 1280, height: 720 },
  { width: 2560, height: 1440 },
  { width: 1680, height: 1050 },
  { width: 1600, height: 900 },
];

// ── Languages ───────────────────────────────────────────────────────────────

const LANGUAGES = [
  "en-US,en;q=0.9",
  "en-GB,en;q=0.9",
  "en-US,en;q=0.9,es;q=0.8",
  "en-US,en;q=0.9,fr;q=0.8",
  "it-IT,it;q=0.9,en;q=0.8",
  "de-DE,de;q=0.9,en;q=0.8",
];

// ── Timezones ───────────────────────────────────────────────────────────────

const TIMEZONES = [
  "America/New_York",
  "America/Chicago",
  "America/Los_Angeles",
  "Europe/London",
  "Europe/Paris",
  "Europe/Berlin",
  "Europe/Rome",
  "Asia/Tokyo",
];

// ── WebGL renderers ─────────────────────────────────────────────────────────

const WEBGL_VENDORS = ["Google Inc. (NVIDIA)", "Google Inc. (Intel)", "Google Inc. (AMD)"];
const WEBGL_RENDERERS = [
  "ANGLE (NVIDIA, NVIDIA GeForce RTX 3060 Direct3D11 vs_5_0 ps_5_0, D3D11)",
  "ANGLE (Intel, Intel(R) UHD Graphics 630 Direct3D11 vs_5_0 ps_5_0, D3D11)",
  "ANGLE (AMD, AMD Radeon RX 580 Direct3D11 vs_5_0 ps_5_0, D3D11)",
  "ANGLE (Apple, ANGLE Metal Renderer: Apple M1, Unspecified Version)",
  "ANGLE (Apple, ANGLE Metal Renderer: Apple M2, Unspecified Version)",
  "ANGLE (Intel, Intel(R) Iris(TM) Plus Graphics OpenGL Engine, OpenGL 4.1)",
];

/**
 * Get a random user agent
 */
export function getRandomUserAgent(): string {
  return USER_AGENTS[Math.floor(Math.random() * USER_AGENTS.length)]!;
}

/**
 * Get a random viewport
 */
export function getRandomViewport(): { width: number; height: number } {
  return VIEWPORTS[Math.floor(Math.random() * VIEWPORTS.length)]!;
}

/**
 * Get a random language header
 */
export function getRandomLanguage(): string {
  return LANGUAGES[Math.floor(Math.random() * LANGUAGES.length)]!;
}

/**
 * Get a random timezone
 */
export function getRandomTimezone(): string {
  return TIMEZONES[Math.floor(Math.random() * TIMEZONES.length)]!;
}

/**
 * Get a random delay for human-like behavior
 */
export function getRandomDelay(min = 500, max = 2000): number {
  return Math.floor(Math.random() * (max - min + 1)) + min;
}

/**
 * Sleep for a duration
 */
export function sleep(ms: number): Promise<void> {
  return new Promise((resolve) => setTimeout(resolve, ms));
}

/** Generate a consistent fingerprint for a single session. */
export interface Fingerprint {
  userAgent: string;
  viewport: { width: number; height: number };
  locale: string;
  timezoneId: string;
  platform: string;
  webglVendor: string;
  webglRenderer: string;
  deviceScaleFactor: number;
  hardwareConcurrency: number;
  deviceMemory: number;
}

export function generateFingerprint(): Fingerprint {
  const ua = getRandomUserAgent();
  const platform = PLATFORMS[Math.floor(Math.random() * PLATFORMS.length)]!;
  const vendor = WEBGL_VENDORS[Math.floor(Math.random() * WEBGL_VENDORS.length)]!;
  const renderer = WEBGL_RENDERERS[Math.floor(Math.random() * WEBGL_RENDERERS.length)]!;

  return {
    userAgent: ua,
    viewport: getRandomViewport(),
    locale: getRandomLanguage().split(",")[0]!,
    timezoneId: getRandomTimezone(),
    platform: platform.platform,
    webglVendor: vendor,
    webglRenderer: renderer,
    deviceScaleFactor: [1, 1, 1, 1.25, 1.5, 2][Math.floor(Math.random() * 6)]!,
    hardwareConcurrency: [4, 6, 8, 12, 16][Math.floor(Math.random() * 5)]!,
    deviceMemory: [4, 8, 8, 16, 16, 32][Math.floor(Math.random() * 6)]!,
  };
}

/**
 * Stealth patches to apply to Playwright context
 */
export const STEALTH_SCRIPTS = {
  /** Override navigator.webdriver */
  webdriver: `
    Object.defineProperty(navigator, 'webdriver', {
      get: () => undefined,
    });
    delete navigator.__proto__.webdriver;
  `,

  /** Mock plugins */
  plugins: `
    Object.defineProperty(navigator, 'plugins', {
      get: () => [
        { name: 'Chrome PDF Plugin', filename: 'internal-pdf-viewer', description: 'Portable Document Format', length: 1 },
        { name: 'Chrome PDF Viewer', filename: 'mhjfbmdgcfjbbpaeojofohoefgiehjai', description: '', length: 1 },
        { name: 'Native Client', filename: 'internal-nacl-plugin', description: '', length: 2 },
      ],
    });
    Object.defineProperty(navigator, 'mimeTypes', {
      get: () => [
        { type: 'application/pdf', suffixes: 'pdf', description: 'Portable Document Format' },
      ],
    });
  `,

  /** Mock languages */
  languages: `
    Object.defineProperty(navigator, 'languages', {
      get: () => ['en-US', 'en'],
    });
  `,

  /** Mock permissions */
  permissions: `
    const originalQuery = window.navigator.permissions.query;
    window.navigator.permissions.query = (parameters) => (
      parameters.name === 'notifications' ?
        Promise.resolve({ state: Notification.permission }) :
        originalQuery(parameters)
    );
  `,

  /** Mock chrome object */
  chrome: `
    window.chrome = {
      runtime: { connect: function(){}, sendMessage: function(){} },
      loadTimes: function() { return { commitLoadTime: Date.now() / 1000, connectionInfo: 'h2', finishDocumentLoadTime: Date.now() / 1000, finishLoadTime: Date.now() / 1000, firstPaintAfterLoadTime: 0, firstPaintTime: Date.now() / 1000, navigationType: 'Other', npnNegotiatedProtocol: 'h2', requestTime: Date.now() / 1000, startLoadTime: Date.now() / 1000, wasAlternateProtocolAvailable: false, wasFetchedViaSpdy: true, wasNpnNegotiated: true }; },
      csi: function() { return { onloadT: Date.now(), pageT: Date.now() - performance.timing.navigationStart, startE: performance.timing.navigationStart, tran: 15 }; },
      app: { isInstalled: false, InstallState: { DISABLED: 'disabled', INSTALLED: 'installed', NOT_INSTALLED: 'not_installed' }, RunningState: { CANNOT_RUN: 'cannot_run', READY_TO_RUN: 'ready_to_run', RUNNING: 'running' } },
    };
  `,

  /** Canvas fingerprint noise */
  canvas: `
    const origToDataURL = HTMLCanvasElement.prototype.toDataURL;
    HTMLCanvasElement.prototype.toDataURL = function(type) {
      if (this.width === 0 || this.height === 0) return origToDataURL.apply(this, arguments);
      const ctx = this.getContext('2d');
      if (ctx) {
        const shift = (Math.random() - 0.5) * 0.01;
        const imageData = ctx.getImageData(0, 0, Math.min(this.width, 16), Math.min(this.height, 16));
        for (let i = 0; i < imageData.data.length; i += 4) {
          imageData.data[i] = Math.max(0, Math.min(255, imageData.data[i] + shift));
        }
        ctx.putImageData(imageData, 0, 0);
      }
      return origToDataURL.apply(this, arguments);
    };
  `,

  /** AudioContext fingerprint noise */
  audio: `
    const origGetChannelData = AudioBuffer.prototype.getChannelData;
    AudioBuffer.prototype.getChannelData = function(channel) {
      const data = origGetChannelData.call(this, channel);
      const noise = 0.0000001;
      for (let i = 0; i < Math.min(data.length, 10); i++) {
        data[i] += (Math.random() - 0.5) * noise;
      }
      return data;
    };
  `,
};

/** Build fingerprint-aware stealth script. */
export function getStealthScript(fp?: Fingerprint): string {
  const base = Object.values(STEALTH_SCRIPTS).join("\n");
  if (!fp) return base;

  const fpScript = `
    Object.defineProperty(navigator, 'hardwareConcurrency', { get: () => ${fp.hardwareConcurrency} });
    Object.defineProperty(navigator, 'deviceMemory', { get: () => ${fp.deviceMemory} });
    Object.defineProperty(navigator, 'platform', { get: () => '${fp.platform}' });

    // WebGL fingerprint
    const getParameterOrig = WebGLRenderingContext.prototype.getParameter;
    WebGLRenderingContext.prototype.getParameter = function(param) {
      if (param === 37445) return '${fp.webglVendor}';
      if (param === 37446) return '${fp.webglRenderer}';
      return getParameterOrig.call(this, param);
    };
    const getParameterOrig2 = WebGL2RenderingContext.prototype.getParameter;
    WebGL2RenderingContext.prototype.getParameter = function(param) {
      if (param === 37445) return '${fp.webglVendor}';
      if (param === 37446) return '${fp.webglRenderer}';
      return getParameterOrig2.call(this, param);
    };
  `;
  return base + "\n" + fpScript;
}

