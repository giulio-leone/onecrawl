/**
 * Playwright Scraper Adapter
 * Implements ScraperPort using Playwright for web page scraping.
 */

import type { Browser, BrowserContext, Page } from "playwright";
import type { ScraperPort, ScrapeResponse } from "../../ports/index.js";
import type {
  ScrapeResult,
  ScrapeOptions,
  BatchScrapeResult,
  BatchOptions,
  ProgressCallback,
} from "../../domain/schemas.js";
import {
  htmlToText,
  htmlToMarkdown,
  extractLinks,
  extractMedia,
  extractMetadata,
} from "../../utils/content-parser.js";
import {
  getRandomUserAgent,
  getRandomViewport,
  getStealthScript,
  getRandomDelay,
  sleep,
} from "../../utils/stealth.js";
import { batchScrape } from "../shared/batch-scrape.js";
import { LruCache } from "../fetch-pool/lru-cache.js";

/**
 * PlaywrightScraperAdapter - ScraperPort implementation using Playwright
 */
export class PlaywrightScraperAdapter implements ScraperPort {
  private browser: Browser | null = null;
  private cache: LruCache<ScrapeResult>;
  private available: boolean | null = null;

  constructor(cacheSize = 200, cacheTTL = 30 * 60 * 1000) {
    this.cache = new LruCache(cacheSize, cacheTTL);
  }

  private async getBrowser(): Promise<Browser> {
    if (!this.browser) {
      const playwright = await import("playwright");
      this.browser = await playwright.chromium.launch({
        headless: true,
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
    return this.browser;
  }

  private async createContext(): Promise<BrowserContext> {
    const browser = await this.getBrowser();
    const viewport = getRandomViewport();
    const userAgent = getRandomUserAgent();

    return browser.newContext({
      viewport,
      userAgent,
      locale: "en-US",
      timezoneId: "America/New_York",
      deviceScaleFactor: 1,
      javaScriptEnabled: true,
    });
  }

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

    // Check cache
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

    // Check abort
    if (signal?.aborted) {
      throw new Error("Scrape aborted");
    }

    onProgress?.({ phase: "starting", message: `Scraping ${url}...`, url });

    let context: BrowserContext | null = null;
    let page: Page | null = null;

    try {
      context = await this.createContext();
      page = await context.newPage();

      // Apply stealth
      await page.addInitScript(getStealthScript());

      onProgress?.({ phase: "navigating", message: "Loading page...", url });

      // Navigate
      const response = await page.goto(url, {
        waitUntil: waitFor === "networkidle" ? "networkidle" : waitFor,
        timeout,
      });

      // Wait for selector if specified
      if (waitForSelector) {
        await page.waitForSelector(waitForSelector, { timeout });
      }

      // Execute custom JS
      if (jsCode) {
        await page.evaluate(jsCode);
        await sleep(500);
      }

      onProgress?.({
        phase: "extracting",
        message: "Extracting content...",
        url,
      });

      // Get page content
      const html = await page.content();
      const title = await page.title();

      // Extract content
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

      if (shouldExtractLinks) {
        result.links = extractLinks(html, url);
      }

      if (shouldExtractMedia) {
        result.media = extractMedia(html, url);
      }

      if (shouldExtractMetadata) {
        result.metadata = extractMetadata(html);
      }

      // Cache result
      if (useCache) {
        this.cache.set(cacheKey, { data: result, timestamp: Date.now() });
      }

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
      if (page) await page.close();
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
      concurrency = 3,
      retries = 2,
      retryDelay = 1000,
      onProgress,
      signal,
      ...scrapeOptions
    } = options;

    const startTime = Date.now();
    const results = new Map<string, ScrapeResult>();
    const failed = new Map<string, Error>();

    // Process in batches with random delay between them
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

      for (const [url, result] of batchResult.results) results.set(url, result);
      for (const [url, error] of batchResult.failed) failed.set(url, error);

      // Random delay between batches
      if (i + concurrency < urls.length) {
        await sleep(getRandomDelay(500, 1500));
      }
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

  clearCache(): void {
    this.cache.clear();
  }

  async close(): Promise<void> {
    if (this.browser) {
      await this.browser.close();
      this.browser = null;
    }
  }
}

/**
 * Create a Playwright scraper adapter
 */
export function createPlaywrightScraperAdapter(): ScraperPort {
  return new PlaywrightScraperAdapter();
}
