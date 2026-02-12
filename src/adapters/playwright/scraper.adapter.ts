/**
 * Playwright Scraper Adapter
 * Implements ScraperPort using Playwright for web page scraping.
 */

import type { Browser, BrowserContext } from "playwright";
import type { ScraperPort, ScrapeResponse } from "../../ports/index.js";
import type {
  ScrapeResult,
  ScrapeOptions,
  BatchScrapeResult,
  BatchOptions,
  ProgressCallback,
} from "../../domain/schemas.js";
import { getRandomDelay, sleep } from "../../utils/stealth.js";
import { batchScrape } from "../shared/batch-scrape.js";
import { LruCache } from "../fetch-pool/lru-cache.js";
import {
  launchBrowser,
  createStealthContext,
  scrapePage,
} from "./page-handler.js";

export class PlaywrightScraperAdapter implements ScraperPort {
  private browser: Browser | null = null;
  private cache: LruCache<ScrapeResult>;
  private available: boolean | null = null;

  constructor(cacheSize = 200, cacheTTL = 30 * 60 * 1000) {
    this.cache = new LruCache(cacheSize, cacheTTL);
  }

  private async getBrowser(): Promise<Browser> {
    if (!this.browser) this.browser = await launchBrowser();
    return this.browser;
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
      context = await createStealthContext(browser);
      const page = await context.newPage();

      const { result } = await scrapePage(page, url, startTime, {
        timeout,
        waitFor,
        waitForSelector,
        jsCode,
        shouldExtractMedia,
        shouldExtractLinks,
        shouldExtractMetadata,
        onProgress,
      });

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
      if (i + concurrency < urls.length) await sleep(getRandomDelay(500, 1500));
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
      const pw = await import("playwright");
      const b = await pw.chromium.launch({ headless: true });
      await b.close();
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

export function createPlaywrightScraperAdapter(): ScraperPort {
  return new PlaywrightScraperAdapter();
}
