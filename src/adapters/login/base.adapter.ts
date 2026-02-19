/**
 * Base Login Adapter — shared logic for all login strategy adapters.
 * Provides browser launch/connect, session verification, and utility methods.
 */

import type { Browser, BrowserContext, Page } from "playwright-core";
import type {
  BrowserProfile,
  LoginResult,
  SessionInfo,
  SocialService,
} from "../../domain/browser-profile.js";
import type { LoginPort, LoginOptions } from "../../ports/login.port.js";
import {
  SERVICE_VERIFY_URLS,
  SERVICE_LOGGED_IN_SELECTORS,
} from "../../ports/login.port.js";
import {
  generateFingerprint,
  getStealthScript,
  getRandomDelay,
} from "../../utils/stealth.js";
import { existsSync } from "node:fs";

/**
 * Known system Chrome paths per platform.
 */
function findChromePath(): string | undefined {
  const { platform } = process;
  const paths =
    platform === "darwin"
      ? [
          "/Applications/Google Chrome.app/Contents/MacOS/Google Chrome",
          "/Applications/Google Chrome Canary.app/Contents/MacOS/Google Chrome Canary",
          "/Applications/Chromium.app/Contents/MacOS/Chromium",
        ]
      : platform === "win32"
        ? [
            "C:\\Program Files\\Google\\Chrome\\Application\\chrome.exe",
            "C:\\Program Files (x86)\\Google\\Chrome\\Application\\chrome.exe",
          ]
        : [
            "/usr/bin/google-chrome",
            "/usr/bin/google-chrome-stable",
            "/usr/bin/chromium",
            "/usr/bin/chromium-browser",
          ];
  return paths.find((p) => existsSync(p));
}

export abstract class BaseLoginAdapter implements LoginPort {
  abstract readonly name: string;
  abstract readonly priority: number;

  abstract canHandle(service: SocialService): boolean;
  abstract login(
    profile: BrowserProfile,
    service: SocialService,
    options?: LoginOptions,
  ): Promise<LoginResult>;

  protected browser: Browser | null = null;

  /** Launch Chrome with the profile's user-data-dir. */
  protected async launchBrowser(
    profile: BrowserProfile,
    headless: boolean,
  ): Promise<Browser> {
    const { chromium } = await import("playwright-core");
    const executablePath = findChromePath();
    return chromium.launch({
      executablePath,
      headless,
      args: [
        `--user-data-dir=${profile.userDataDir}`,
        "--no-sandbox",
        "--disable-setuid-sandbox",
        "--disable-dev-shm-usage",
        "--disable-blink-features=AutomationControlled",
        "--no-first-run",
      ],
    });
  }

  /** Create a stealth context with fingerprint. */
  protected async createStealthContext(
    browser: Browser,
  ): Promise<BrowserContext> {
    const fp = generateFingerprint();
    const context = await browser.newContext({
      userAgent: fp.userAgent,
      viewport: fp.viewport,
      locale: fp.locale,
      timezoneId: fp.timezoneId ?? "Europe/Rome",
      deviceScaleFactor: fp.deviceScaleFactor,
    });
    await context.addInitScript(getStealthScript(fp));
    return context;
  }

  /** Check if user is logged in by looking for known selectors. */
  protected async isLoggedIn(
    page: Page,
    service: SocialService,
  ): Promise<boolean> {
    const selectors = SERVICE_LOGGED_IN_SELECTORS[service] ?? [];
    for (const sel of selectors) {
      try {
        const el = await page.$(sel);
        if (el) return true;
      } catch {
        continue;
      }
    }
    return false;
  }

  /** Wait for login to complete by polling for logged-in selectors. */
  protected async waitForLogin(
    page: Page,
    service: SocialService,
    timeoutMs: number,
  ): Promise<boolean> {
    const selectors = SERVICE_LOGGED_IN_SELECTORS[service] ?? [];
    if (selectors.length === 0) return false;

    const selectorStr = selectors.join(", ");
    try {
      await page.waitForSelector(selectorStr, { timeout: timeoutMs });
      return true;
    } catch {
      return false;
    }
  }

  /** Take a screenshot of the page as base64. */
  protected async takeScreenshot(page: Page): Promise<string> {
    const buf = await page.screenshot({ type: "png" });
    return buf.toString("base64");
  }

  /** Verify session by loading the service's logged-in page. */
  async verifySession(
    profile: BrowserProfile,
    service: SocialService,
  ): Promise<SessionInfo> {
    let browser: Browser | null = null;
    try {
      browser = await this.launchBrowser(profile, true);
      const context = await this.createStealthContext(browser);
      const page = await context.newPage();

      const verifyUrl = SERVICE_VERIFY_URLS[service];
      await page.goto(verifyUrl, {
        waitUntil: "domcontentloaded",
        timeout: 15_000,
      });
      // Small delay for redirects
      await new Promise((r) => setTimeout(r, getRandomDelay(1000, 2000)));

      const loggedIn = await this.isLoggedIn(page, service);
      await page.close();
      await context.close();

      return {
        valid: loggedIn,
        service,
        expiresAt: null,
        detail: loggedIn
          ? "Session active"
          : "Session expired — re-login required",
      };
    } catch (err) {
      return {
        valid: false,
        service,
        expiresAt: null,
        detail: `Verification failed: ${err instanceof Error ? err.message : "unknown"}`,
      };
    } finally {
      await browser?.close().catch(() => {});
    }
  }

  async close(): Promise<void> {
    await this.browser?.close().catch(() => {});
    this.browser = null;
  }
}
