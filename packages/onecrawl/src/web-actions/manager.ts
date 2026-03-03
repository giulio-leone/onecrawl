/**
 * WebActionManager — manages persistent browser sessions tied to profiles.
 *
 * Each profile gets a Playwright persistent context (with login cookies),
 * reused across multiple web actions. Sessions auto-close after inactivity.
 *
 * Supports two connection modes:
 * 1. **Launch**: `launchPersistentContext` — starts a new browser (default)
 * 2. **Autoconnect**: `connectOverCDP` — attaches to an already-running Chrome
 *    with `--remote-debugging-port` (e.g. your daily Chrome session).
 */
import { chromium, type Browser, type BrowserContext, type Page } from "rebrowser-playwright";
import { join } from "node:path";
import { homedir } from "node:os";
import { readFileSync } from "node:fs";
import { applyStealthToContext, CHROME_UA } from "./stealth.js";
import { createHumanCursor, type HumanCursor } from "./human-behavior.js";
import { PasskeyStore } from "../auth/passkey-store.js";
import { WebAuthnManager } from "../auth/webauthn-manager.js";

// ── Types ────────────────────────────────────────────────────────────────────

export type WebSession = {
  profileId: string;
  context: BrowserContext;
  page: Page;
  cursor: HumanCursor | null;
  createdAt: number;
  lastActivity: number;
  /** If connected via CDP, we hold a Browser ref for cleanup. */
  cdpBrowser?: Browser;
  /** WebAuthn manager for passkey auth, if configured. */
  webAuthnManager?: WebAuthnManager;
};

export type WebActionError = {
  code:
    | "PROFILE_NOT_FOUND"
    | "SESSION_EXPIRED"
    | "ELEMENT_NOT_FOUND"
    | "TIMEOUT"
    | "NAVIGATION_ERROR"
    | "EVALUATION_ERROR"
    | "UPLOAD_ERROR"
    | "UNKNOWN";
  message: string;
  screenshot?: string; // base64 on error
};

// ── Config ───────────────────────────────────────────────────────────────────

const PROFILES_DIR =
  process.env.ONECRAWL_PROFILES_DIR ?? join(homedir(), ".onecrawl", "profiles");
const SESSION_IDLE_TIMEOUT_MS = 30 * 60 * 1000; // 30 min
const CLEANUP_INTERVAL_MS = 5 * 60 * 1000; // check every 5 min
const DEFAULT_TIMEOUT_MS = 30_000;
const DEFAULT_CDP_URL = "http://127.0.0.1:9222";
const DEBUG =
  process.env.ONECRAWL_DEBUG === "true" || process.env.ONECRAWL_DEBUG === "1";

function debugLog(msg: string): void {
  if (DEBUG) process.stderr.write(`${msg}\n`);
}

// ── Manager ──────────────────────────────────────────────────────────────────

export interface WebActionManagerConfig {
  /** Path to the encrypted passkey store. When set, sessions auto-inject virtual authenticator. */
  passkeyStorePath?: string;
}

export class WebActionManager {
  private sessions = new Map<string, WebSession>();
  private cleanupTimer: ReturnType<typeof setInterval> | null = null;
  private readonly passkeyStorePath?: string;

  constructor(config?: WebActionManagerConfig) {
    this.passkeyStorePath = config?.passkeyStorePath;
  }

  /** Start the idle-session cleanup loop. */
  start(): void {
    if (this.cleanupTimer) return;
    this.cleanupTimer = setInterval(() => void this.cleanupIdle(), CLEANUP_INTERVAL_MS);
  }

  /** Stop cleanup and close all sessions. */
  async stop(): Promise<void> {
    if (this.cleanupTimer) {
      clearInterval(this.cleanupTimer);
      this.cleanupTimer = null;
    }
    const closing = [...this.sessions.values()].map((s) =>
      s.context.close().catch(() => {}),
    );
    await Promise.allSettled(closing);
    this.sessions.clear();
  }

  /** Get or create a browser session for a profile. */
  async getOrCreateSession(
    profileId: string,
    options?: { headless?: boolean; cdpUrl?: string },
  ): Promise<WebSession> {
    const existing = this.sessions.get(profileId);
    if (existing) {
      existing.lastActivity = Date.now();
      // Ensure page is still open
      if (existing.page.isClosed()) {
        existing.page = await existing.context.newPage();
      }
      return existing;
    }

    // Autoconnect mode: attach to an already-running Chrome via CDP
    if (options?.cdpUrl) {
      return this.connectViaCDP(profileId, options.cdpUrl);
    }

    const userDataDir = join(PROFILES_DIR, profileId, "browser-data");
    const isHeaded = !(options?.headless ?? true);
    const context = await chromium.launchPersistentContext(userDataDir, {
      headless: !isHeaded,
      channel: "chrome", // use system Chrome for stealth
      viewport: { width: 1280, height: 800 },
      locale: "en-US",
      userAgent: CHROME_UA,
      args: [
        "--disable-blink-features=AutomationControlled",
        "--no-first-run",
        "--no-default-browser-check",
      ],
    });

    await applyStealthToContext(context);

    const page = context.pages()[0] ?? (await context.newPage());
    let cursor: HumanCursor | null = null;
    try {
      cursor = await createHumanCursor(page);
    } catch {
      console.warn(`[web-actions] ghost-cursor init failed for ${profileId}, continuing without`);
    }
    const now = Date.now();
    const session: WebSession = {
      profileId,
      context,
      page,
      cursor,
      createdAt: now,
      lastActivity: now,
    };

    this.sessions.set(profileId, session);
    await this.setupPasskeys(session);
    return session;
  }

  /**
   * Connect to an already-running Chrome via CDP (Chrome DevTools Protocol).
   *
   * Accepts either:
   * - `http://host:port` — resolves WS endpoint from DevToolsActivePort file
   * - `ws://host:port/devtools/browser/...` — direct WebSocket URL
   *
   * Chrome Canary must be running (it auto-enables remote debugging on macOS).
   */
  async connectViaCDP(
    profileId: string,
    cdpUrl: string = DEFAULT_CDP_URL,
  ): Promise<WebSession> {
    const existing = this.sessions.get(profileId);
    if (existing) {
      existing.lastActivity = Date.now();
      if (existing.page.isClosed()) {
        existing.page = await existing.context.newPage();
      }
      return existing;
    }

    const wsUrl = this.resolveCdpWebSocketUrl(cdpUrl);
    debugLog(`[web-actions] connecting to Chrome via CDP: ${wsUrl}`);
    const browser = await chromium.connectOverCDP(wsUrl);
    const contexts = browser.contexts();
    // Use the first (default) context — that's the user's normal session
    const context = contexts[0] ?? (await browser.newContext());
    await applyStealthToContext(context);
    const page = context.pages()[0] ?? (await context.newPage());

    // Ensure a usable viewport — CDP sessions inherit Chrome's window size,
    // which can be tiny (e.g. 56px) if the window is minimized.
    await page.setViewportSize({ width: 1280, height: 800 });

    const now = Date.now();
    let cursor: HumanCursor | null = null;
    try {
      cursor = await createHumanCursor(page);
    } catch {
      console.warn(`[web-actions] ghost-cursor init failed for ${profileId} (CDP), continuing without`);
    }
    const session: WebSession = {
      profileId,
      context,
      page,
      cursor,
      cdpBrowser: browser,
      createdAt: now,
      lastActivity: now,
    };

    this.sessions.set(profileId, session);
    await this.setupPasskeys(session);
    return session;
  }

  /**
   * Resolve a CDP URL to a WebSocket endpoint.
   *
   * If the URL is already ws://, return as-is.
   * Otherwise, read the DevToolsActivePort file to build the ws:// URL.
   */
  private resolveCdpWebSocketUrl(cdpUrl: string): string {
    if (cdpUrl.startsWith("ws://") || cdpUrl.startsWith("wss://")) {
      return cdpUrl;
    }

    // Try to build WS URL from DevToolsActivePort file
    const chromeVariants = [
      "Google/Chrome Canary",
      "Google/Chrome",
      "Google/Chrome Dev",
      "Google/Chrome Beta",
    ];
    for (const variant of chromeVariants) {
      try {
        const dtpPath = join(
          homedir(),
          "Library",
          "Application Support",
          variant,
          "DevToolsActivePort",
        );
        const content = readFileSync(dtpPath, "utf-8").trim();
        const [port, wsPath] = content.split("\n");
        if (port && wsPath) {
          return `ws://127.0.0.1:${port}${wsPath}`;
        }
      } catch {
        // File not found, try next variant
      }
    }

    // Fallback: try the URL as-is (might work if Chrome has /json/version)
    return cdpUrl;
  }

  /** Close a specific session. */
  async closeSession(profileId: string): Promise<boolean> {
    const session = this.sessions.get(profileId);
    if (!session) return false;
    // For CDP sessions, disconnect (don't close the user's browser)
    if (session.cdpBrowser) {
      await session.cdpBrowser.close().catch(() => {});
    } else {
      await session.context.close().catch(() => {});
    }
    this.sessions.delete(profileId);
    return true;
  }

  /** List active sessions. */
  listSessions(): Array<{
    profileId: string;
    createdAt: number;
    lastActivity: number;
    url: string;
  }> {
    return [...this.sessions.entries()].map(([id, s]) => ({
      profileId: id,
      createdAt: s.createdAt,
      lastActivity: s.lastActivity,
      url: s.page.isClosed() ? "(closed)" : s.page.url(),
    }));
  }

  /** Check whether passkey auth is active for a session. */
  getPasskeyStatus(profileId: string): { active: boolean; profileId: string } {
    const session = this.sessions.get(profileId);
    return { profileId, active: session?.webAuthnManager != null };
  }

  /** Get page for a profile (throws WebActionError if not found). */
  async getPage(
    profileId: string,
    options?: { headless?: boolean; cdpUrl?: string },
  ): Promise<Page> {
    const session = await this.getOrCreateSession(profileId, options);
    return session.page;
  }

  /** Take screenshot (for error debugging or explicit request). */
  async captureScreenshot(
    profileId: string,
    opts?: { fullPage?: boolean; selector?: string },
  ): Promise<string> {
    const page = await this.getPage(profileId);
    let buffer: Buffer;

    if (opts?.selector) {
      const el = page.locator(opts.selector).first();
      buffer = await el.screenshot({ timeout: DEFAULT_TIMEOUT_MS });
    } else {
      buffer = await page.screenshot({
        fullPage: opts?.fullPage ?? false,
        timeout: DEFAULT_TIMEOUT_MS,
      });
    }

    return buffer.toString("base64");
  }

  /** Auto-capture screenshot on error for debugging. */
  async errorScreenshot(profileId: string): Promise<string | undefined> {
    try {
      return await this.captureScreenshot(profileId);
    } catch {
      return undefined;
    }
  }

  // ── Private ──────────────────────────────────────────────────────────────

  private async setupPasskeys(session: WebSession): Promise<void> {
    if (!this.passkeyStorePath) return;
    try {
      const store = new PasskeyStore({ storagePath: this.passkeyStorePath });
      const credentials = await store.getCredentials();
      if (credentials.length === 0) {
        debugLog(`[web-actions] no passkey credentials at ${this.passkeyStorePath}`);
        return;
      }

      const cdpSession = await session.context.newCDPSession(session.page);
      const manager = new WebAuthnManager(cdpSession);
      await manager.injectCredentials(credentials);
      session.webAuthnManager = manager;
      debugLog(`[web-actions] passkey injected for ${session.profileId} (${credentials.length} creds)`);
    } catch (err) {
      console.warn(
        `[web-actions] passkey setup failed for ${session.profileId}: ${err instanceof Error ? err.message : String(err)}`,
      );
    }
  }

  private async cleanupIdle(): Promise<void> {
    const now = Date.now();
    const expired: string[] = [];

    for (const [id, session] of this.sessions) {
      if (now - session.lastActivity > SESSION_IDLE_TIMEOUT_MS) {
        expired.push(id);
      }
    }

    for (const id of expired) {
      debugLog(`[web-actions] closing idle session: ${id}`);
      await this.closeSession(id);
    }
  }
}

export const webActionManager = new WebActionManager();
