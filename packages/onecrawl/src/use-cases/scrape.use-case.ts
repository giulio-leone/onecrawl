/**
 * Scrape Use Case
 * High-level API for scraping web pages.
 */

import type { ScraperPort, ScrapeResponse } from "../ports/index.js";
import type {
  ScrapeResult,
  ScrapeOptions,
  ProgressCallback,
} from "../domain/schemas.js";
import { PlaywrightScraperAdapter } from "../adapters/playwright/scraper.adapter.js";
import { FetchScraperAdapter } from "../adapters/fetch/scraper.adapter.js";
import { UndiciScraperAdapter } from "../adapters/undici/scraper.adapter.js";
import { BrowserScraperAdapter } from "../adapters/browser/scraper.adapter.js";
import type { BrowserScraperOptions } from "../adapters/browser/scraper.adapter.js";

export interface ScrapeUseCaseOptions extends Partial<ScrapeOptions> {
  /** Prefer browser (Playwright or Browser adapter) for JS-heavy sites. */
  preferBrowser?: boolean;

  /** Use HTTP/2 with undici (faster for batch requests). */
  useHttp2?: boolean;

  /** Fallback to fetch if browser unavailable. */
  fallbackToFetch?: boolean;

  /** Progress callback. */
  onProgress?: ProgressCallback;

  /** Abort signal. */
  signal?: AbortSignal;
}

/** Options for the ScrapeUseCase constructor. */
export interface ScrapeUseCaseConfig {
  /** BrowserScraperAdapter options (CDP endpoint, etc.). */
  browser?: BrowserScraperOptions;
}

/**
 * ScrapeUseCase - Intelligent scraping with fallback
 *
 * Fallback chain: Browser (CDP/stealth) → Playwright → undici → fetch
 */
export class ScrapeUseCase {
  private browserScraper: ScraperPort;
  private playwrightScraper: ScraperPort;
  private fetchScraper: ScraperPort;
  private undiciScraper: ScraperPort;

  constructor(config: ScrapeUseCaseConfig = {}) {
    this.browserScraper = new BrowserScraperAdapter(config.browser);
    this.playwrightScraper = new PlaywrightScraperAdapter();
    this.fetchScraper = new FetchScraperAdapter();
    this.undiciScraper = new UndiciScraperAdapter();
  }

  /**
   * Scrape a URL with automatic adapter selection
   */
  async execute(
    url: string,
    options: ScrapeUseCaseOptions = {},
  ): Promise<ScrapeResponse> {
    const {
      preferBrowser = false,
      useHttp2 = true,
      fallbackToFetch = true,
      waitFor,
      ...scrapeOptions
    } = options;

    // Determine if we need browser
    const needsBrowser = preferBrowser || waitFor === "networkidle";

    if (needsBrowser) {
      // Try Browser adapter first (CDP/stealth), then Playwright
      for (const scraper of [this.browserScraper, this.playwrightScraper]) {
        try {
          if (await scraper.isAvailable()) {
            return await scraper.scrape(url, { ...scrapeOptions, waitFor });
          }
        } catch (error) {
          if (!fallbackToFetch) throw error;
        }
      }
    }

    // Use undici for HTTP/2 (faster), fallback to fetch
    const httpScraper = useHttp2 ? this.undiciScraper : this.fetchScraper;
    return httpScraper.scrape(url, scrapeOptions);
  }

  /**
   * Scrape multiple URLs
   */
  async executeMany(
    urls: string[],
    options: ScrapeUseCaseOptions & { concurrency?: number } = {},
  ): Promise<Map<string, ScrapeResult>> {
    const {
      preferBrowser,
      useHttp2 = true,
      concurrency = 10,
      onProgress,
      signal,
      ...rest
    } = options;

    // For batch scraping, prefer undici (HTTP/2 connection pooling)
    let scraper: ScraperPort;
    if (preferBrowser) {
      // Try browser adapter first, then playwright
      scraper = (await this.browserScraper.isAvailable())
        ? this.browserScraper
        : this.playwrightScraper;
    } else if (useHttp2) {
      scraper = this.undiciScraper;
    } else {
      scraper = this.fetchScraper;
    }

    const result = await scraper.scrapeMany(urls, {
      concurrency,
      onProgress,
      signal,
      ...rest,
    });
    return result.results;
  }

  /**
   * Get available scrapers
   */
  async getAvailableScrapers(): Promise<string[]> {
    const available: string[] = ["fetch", "undici"];

    if (await this.browserScraper.isAvailable()) {
      available.push("browser");
    }
    if (await this.playwrightScraper.isAvailable()) {
      available.push("playwright");
    }

    return available;
  }
}

/**
 * Create a scrape use case
 */
export function createScrapeUseCase(): ScrapeUseCase {
  return new ScrapeUseCase();
}
