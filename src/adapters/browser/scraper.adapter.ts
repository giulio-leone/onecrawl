/**
 * Browser Scraper Adapter
 *
 * Full-featured browser adapter with:
 * - CDP connect to existing Chrome/Chromium instances
 * - @sparticuz/chromium support for serverless
 * - Local Chromium launch via playwright-core
 * - Advanced stealth: fingerprint randomisation, canvas/audio noise, WebGL spoofing
 *
 * Fallback chain: CDP → @sparticuz/chromium → local Chromium
 */

import type { Browser, BrowserContext } from "playwright-core";
import type { ScraperPort, ScrapeResponse } from "../../ports/index.js";
import type {
  ScrapeResult,
  ScrapeOptions,
  BatchScrapeResult,
  BatchOptions,
  ProgressCallback,
} from "../../domain/schemas.js";
import {
  generateFingerprint,
  getRandomDelay,
  getStealthScript,
  sleep,
} from "../../utils/stealth.js";
import type { Fingerprint } from "../../utils/stealth.js";
import { batchScrape } from "../shared/batch-scrape.js";
import { LruCache } from "../fetch-pool/lru-cache.js";
import { existsSync } from "node:fs";
import {
  htmlToText,
  htmlToMarkdown,
  extractLinks,
  extractMedia,
  extractMetadata,
} from "../../utils/content-parser.js";

// ── Options ─────────────────────────────────────────────────────────────────

export interface BrowserScraperOptions {
  /** CDP endpoint (e.g. "ws://localhost:9222" or port number "9222"). */
  cdpEndpoint?: string;
  /** Force headless mode when launching local browser. */
  headless?: boolean;
  /** Max concurrent pages. */
  maxPages?: number;
  /** Cache size. */
  cacheSize?: number;
  /** Cache TTL in ms. */
  cacheTTL?: number;
}

// ── Adapter ─────────────────────────────────────────────────────────────────

export class BrowserScraperAdapter implements ScraperPort {
  private browser: Browser | null = null;
  private cache: LruCache<ScrapeResult>;
  private available: boolean | null = null;
  private fingerprint: Fingerprint = generateFingerprint();
  private readonly opts: Required<BrowserScraperOptions>;

  constructor(options: BrowserScraperOptions = {}) {
    this.opts = {
      cdpEndpoint: options.cdpEndpoint ?? "",
      headless: options.headless ?? true,
      maxPages: options.maxPages ?? 3,
      cacheSize: options.cacheSize ?? 200,
      cacheTTL: options.cacheTTL ?? 30 * 60 * 1000,
    };
    this.cache = new LruCache(this.opts.cacheSize, this.opts.cacheTTL);
  }

  // ── Browser lifecycle ───────────────────────────────────────────────────

  private async getBrowser(): Promise<Browser> {
    if (this.browser?.isConnected()) return this.browser;

    // Strategy 1: CDP connect
    if (this.opts.cdpEndpoint) {
      this.browser = await this.connectCDP(this.opts.cdpEndpoint);
      if (this.browser) return this.browser;
    }

    // Strategy 2: @sparticuz/chromium (serverless)
    this.browser = await this.launchSparticuz();
    if (this.browser) return this.browser;

    // Strategy 3: Local playwright-core chromium
    this.browser = await this.launchLocal();
    return this.browser;
  }

  private async connectCDP(endpoint: string): Promise<Browser | null> {
    const { chromium } = await import("playwright-core");
    const url = /^\d+$/.test(endpoint)
      ? `http://127.0.0.1:${endpoint}`
      : endpoint;
    // Try both the provided URL and 127.0.0.1 fallback (IPv6 resolver issue)
    const urls = [url];
    if (url.includes("localhost")) {
      urls.push(url.replace("localhost", "127.0.0.1"));
    }
    for (const u of urls) {
      try {
        return await chromium.connectOverCDP(u);
      } catch {
        continue;
      }
    }
    return null;
  }

  private async launchSparticuz(): Promise<Browser | null> {
    try {
      const mod = await (Function(
        'return import("@sparticuz/chromium")',
      )() as Promise<{ default: { executablePath: () => Promise<string>; args: string[] } }>).catch(
        () => null,
      );
      if (!mod) return null;
      const execPath = await mod.default.executablePath();
      const { chromium } = await import("playwright-core");
      return chromium.launch({
        executablePath: execPath,
        headless: true,
        args: [...mod.default.args, "--no-sandbox", "--disable-setuid-sandbox"],
      });
    } catch {
      return null;
    }
  }

  private async launchLocal(): Promise<Browser> {
    const { chromium } = await import("playwright-core");
    const executablePath = this.findChromePath();
    return chromium.launch({
      executablePath,
      headless: this.opts.headless,
      args: [
        "--no-sandbox",
        "--disable-setuid-sandbox",
        "--disable-dev-shm-usage",
        "--disable-accelerated-2d-canvas",
        "--no-first-run",
        "--no-zygote",
        "--disable-gpu",
        "--disable-blink-features=AutomationControlled",
      ],
    });
  }

  /** Detect system Chrome/Chromium executable. */
  private findChromePath(): string | undefined {
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

  // ── Stealth context ─────────────────────────────────────────────────────

  private async createStealthContext(
    browser: Browser,
  ): Promise<BrowserContext> {
    const fp = this.fingerprint;
    const osPlatform = fp.platform.includes("Mac")
      ? "macOS"
      : fp.platform.includes("Win")
        ? "Windows"
        : "Linux";

    const ctx = await browser.newContext({
      viewport: fp.viewport,
      userAgent: fp.userAgent,
      locale: fp.locale,
      timezoneId: fp.timezoneId,
      deviceScaleFactor: fp.deviceScaleFactor,
      javaScriptEnabled: true,
      bypassCSP: true,
      extraHTTPHeaders: {
        "Accept-Language": `${fp.locale},en;q=0.9`,
        "sec-ch-ua": `"Chromium";v="131", "Not_A Brand";v="24"`,
        "sec-ch-ua-mobile": "?0",
        "sec-ch-ua-platform": `"${osPlatform}"`,
      },
    });

    await ctx.addInitScript(getStealthScript(fp));
    return ctx;
  }

  // ── Scrape ──────────────────────────────────────────────────────────────

  async scrape(
    url: string,
    options: Partial<ScrapeOptions> & {
      onProgress?: ProgressCallback;
      signal?: AbortSignal;
    } = {},
  ): Promise<ScrapeResponse> {
    const {
      timeout = 30000,
      waitFor = "networkidle",
      waitForSelector,
      extractMedia: shouldExtractMedia = true,
      extractLinks: shouldExtractLinks = true,
      extractMetadata: shouldExtractMetadata = true,
      cache: useCache = true,
      jsCode,
      onProgress,
      signal,
    } = options;

    const startTime = Date.now();
    const cacheKey = `${url}|${jsCode || ""}|${waitForSelector || ""}`;

    if (useCache) {
      const cached = this.cache.get(cacheKey);
      if (cached) {
        onProgress?.({ phase: "complete", message: "From cache", url });
        return {
          result: cached.data,
          cached: true,
          duration: Date.now() - startTime,
          source: this.getName(),
        };
      }
    }

    if (signal?.aborted) throw new Error("Scrape aborted");
    onProgress?.({ phase: "starting", message: `Scraping ${url}...`, url });

    let context: BrowserContext | null = null;
    try {
      const browser = await this.getBrowser();
      context = await this.createStealthContext(browser);
      const page = await context.newPage();

      // Human-like delay before navigation
      await sleep(getRandomDelay(200, 800));

      onProgress?.({
        phase: "navigating",
        message: "Loading page...",
        url,
      });

      const response = await page.goto(url, {
        waitUntil:
          waitFor === "networkidle"
            ? "networkidle"
            : (waitFor as "load"),
        timeout,
      });

      if (waitForSelector) {
        await page.waitForSelector(waitForSelector, { timeout });
      }

      if (jsCode) {
        await page.evaluate(jsCode);
        await sleep(500);
      }

      onProgress?.({
        phase: "extracting",
        message: "Extracting content...",
        url,
      });

      const html = await page.content();
      const title = await page.title();

      const result: ScrapeResult = {
        url: page.url(),
        title,
        content: htmlToText(html),
        markdown: htmlToMarkdown(html),
        html,
        statusCode: response?.status(),
        contentType: response?.headers()["content-type"],
        loadTime: Date.now() - startTime,
      };

      if (shouldExtractLinks) result.links = extractLinks(html, url);
      if (shouldExtractMedia) result.media = extractMedia(html, url);
      if (shouldExtractMetadata) result.metadata = extractMetadata(html);

      if (useCache)
        this.cache.set(cacheKey, { data: result, timestamp: Date.now() });

      onProgress?.({
        phase: "complete",
        message: `Scraped ${result.content.length} chars`,
        url,
      });
      return {
        result,
        cached: false,
        duration: Date.now() - startTime,
        source: this.getName(),
      };
    } finally {
      if (context) await context.close();
    }
  }

  async scrapeMany(
    urls: string[],
    options: Partial<ScrapeOptions & BatchOptions> & {
      onProgress?: ProgressCallback;
      signal?: AbortSignal;
    } = {},
  ): Promise<BatchScrapeResult> {
    const {
      concurrency = this.opts.maxPages,
      retries = 2,
      retryDelay = 1000,
      onProgress,
      signal,
      ...scrapeOptions
    } = options;
    const startTime = Date.now();
    const results = new Map<string, ScrapeResult>();
    const failed = new Map<string, Error>();

    for (let i = 0; i < urls.length; i += concurrency) {
      if (signal?.aborted) break;
      const batch = urls.slice(i, i + concurrency);
      const batchResult = await batchScrape(batch, this.scrape.bind(this), {
        concurrency,
        retries,
        retryDelay,
        onProgress,
        signal,
        scrapeOptions,
      });
      for (const [u, r] of batchResult.results) results.set(u, r);
      for (const [u, e] of batchResult.failed) failed.set(u, e);

      // Human-like delay between batches
      if (i + concurrency < urls.length)
        await sleep(getRandomDelay(1000, 3000));
    }

    onProgress?.({
      phase: "complete",
      message: `Completed: ${results.size} success, ${failed.size} failed`,
      url: urls[0]!,
    });
    return { results, failed, totalDuration: Date.now() - startTime };
  }

  async isAvailable(): Promise<boolean> {
    if (this.available !== null) return this.available;
    try {
      const browser = await this.getBrowser();
      this.available = browser.isConnected();
    } catch {
      this.available = false;
    }
    return this.available;
  }

  getName(): string {
    return "browser";
  }

  clearCache(): void {
    this.cache.clear();
  }

  /** Rotate fingerprint for a new session profile. */
  rotateFingerprint(): void {
    this.fingerprint = generateFingerprint();
  }

  async close(): Promise<void> {
    if (this.browser) {
      await this.browser.close().catch(() => {});
      this.browser = null;
    }
  }
}

/** Create a browser-based scraper adapter. */
export function createBrowserScraperAdapter(
  options?: BrowserScraperOptions,
): ScraperPort {
  return new BrowserScraperAdapter(options);
}
