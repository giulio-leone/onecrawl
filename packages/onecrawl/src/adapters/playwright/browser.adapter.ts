/**
 * Playwright Browser Adapter
 * Native TypeScript browser automation using Playwright.
 */

import type { Browser, BrowserContext as PWContext } from "playwright";
import type {
  BrowserPort,
  BrowserContext,
  PageHandle,
} from "../../ports/index.js";
import type { LaunchConfig, ScrapeOptions } from "../../domain/schemas.js";
import {
  getRandomUserAgent,
  getRandomViewport,
  getStealthScript,
} from "../../utils/stealth.js";
import { PlaywrightPageHandle } from "./page-handle.js";

/**
 * Wrapper for Playwright Browser Context
 */
class PlaywrightContext implements BrowserContext {
  constructor(private context: PWContext) {}

  async newPage(): Promise<PageHandle> {
    const page = await this.context.newPage();
    // Apply stealth scripts
    await page.addInitScript(getStealthScript());
    return new PlaywrightPageHandle(page);
  }

  async cookies(): Promise<
    Array<{ name: string; value: string; domain: string }>
  > {
    const cookies = await this.context.cookies();
    return cookies.map((c) => ({
      name: c.name,
      value: c.value,
      domain: c.domain,
    }));
  }

  async setCookies(
    cookies: Array<{ name: string; value: string; domain: string }>,
  ): Promise<void> {
    await this.context.addCookies(
      cookies.map((c) => ({
        name: c.name,
        value: c.value,
        domain: c.domain,
        path: "/",
      })),
    );
  }

  async close(): Promise<void> {
    await this.context.close();
  }
}

/**
 * PlaywrightBrowserAdapter - BrowserPort implementation using Playwright
 */
export class PlaywrightBrowserAdapter implements BrowserPort {
  private browser: Browser | null = null;
  private contexts: PWContext[] = [];
  private available: boolean | null = null;

  async launch(config?: Partial<LaunchConfig>): Promise<BrowserContext> {
    const playwright = await import("playwright");

    if (!this.browser) {
      this.browser = await playwright.chromium.launch({
        headless: config?.headless ?? true,
        args: [
          "--no-sandbox",
          "--disable-setuid-sandbox",
          "--disable-dev-shm-usage",
          "--disable-accelerated-2d-canvas",
          "--no-first-run",
          "--no-zygote",
          "--disable-gpu",
        ],
      });
    }

    const viewport = config?.viewport ?? getRandomViewport();
    const userAgent = config?.userAgent ?? getRandomUserAgent();

    const contextOptions: Parameters<Browser["newContext"]>[0] = {
      viewport,
      userAgent,
      locale: "en-US",
      timezoneId: "America/New_York",
      deviceScaleFactor: 1,
      hasTouch: false,
      isMobile: false,
      javaScriptEnabled: true,
    };

    if (config?.proxy) {
      contextOptions.proxy = {
        server: config.proxy.server,
        username: config.proxy.username,
        password: config.proxy.password,
      };
    }

    const context = await this.browser.newContext(contextOptions);
    this.contexts.push(context);

    return new PlaywrightContext(context);
  }

  async navigate(
    url: string,
    options?: Partial<ScrapeOptions>,
  ): Promise<PageHandle> {
    const context = await this.launch();
    const page = await context.newPage();

    const waitUntil = options?.waitFor ?? "networkidle";
    const timeout = options?.timeout ?? 30000;

    await (page as PlaywrightPageHandle)["page"].goto(url, {
      waitUntil: waitUntil === "networkidle" ? "networkidle" : waitUntil,
      timeout,
    });

    if (options?.waitForSelector) {
      await page.waitForSelector(options.waitForSelector, { timeout });
    }

    return page;
  }

  async isAvailable(): Promise<boolean> {
    if (this.available !== null) return this.available;

    try {
      const playwright = await import("playwright");
      const browser = await playwright.chromium.launch({ headless: true });
      await browser.close();
      this.available = true;
    } catch {
      this.available = false;
    }

    return this.available;
  }

  getName(): string {
    return "playwright";
  }

  async closeAll(): Promise<void> {
    for (const context of this.contexts) {
      await context.close();
    }
    this.contexts = [];

    if (this.browser) {
      await this.browser.close();
      this.browser = null;
    }
  }
}

/**
 * Create a Playwright browser adapter
 */
export function createPlaywrightAdapter(): BrowserPort {
  return new PlaywrightBrowserAdapter();
}
