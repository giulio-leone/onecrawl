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

export interface ScrapeUseCaseOptions extends Partial<ScrapeOptions> {
  /**
   * Prefer Playwright for JS-heavy sites
   */
  preferBrowser?: boolean;

  /**
   * Use HTTP/2 with undici (faster for batch requests)
   */
  useHttp2?: boolean;

  /**
   * Fallback to fetch if browser unavailable
   */
  fallbackToFetch?: boolean;

  /**
   * Progress callback
   */
  onProgress?: ProgressCallback;

  /**
   * Abort signal
   */
  signal?: AbortSignal;
}

/**
 * ScrapeUseCase - Intelligent scraping with fallback
 */
export class ScrapeUseCase {
  private playwrightScraper: ScraperPort;
  private fetchScraper: ScraperPort;
  private undiciScraper: ScraperPort;

  constructor() {
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
      try {
        if (await this.playwrightScraper.isAvailable()) {
          return await this.playwrightScraper.scrape(url, {
            ...scrapeOptions,
            waitFor,
          });
        }
      } catch (error) {
        if (!fallbackToFetch) throw error;
        // Fall through to HTTP scraper
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
      scraper = this.playwrightScraper;
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
