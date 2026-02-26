/**
 * Scrape Use Case - React Native Compatible
 * Uses only fetch-based scrapers (no Playwright or Undici).
 */

import type { ScraperPort, ScrapeResponse } from "../ports/index.js";
import type {
  ScrapeResult,
  ScrapeOptions,
  ProgressCallback,
} from "../domain/schemas.js";
import { FetchScraperAdapter } from "../adapters/fetch/scraper.adapter.js";
import { FetchPoolScraperAdapter } from "../adapters/fetch-pool/fetch-pool.adapter.js";

export interface ScrapeUseCaseOptions extends Partial<ScrapeOptions> {
  /** Use fetch pool for connection reuse (default true) */
  usePooling?: boolean;

  /** Fallback to basic fetch if pool unavailable */
  fallbackToFetch?: boolean;

  /** Progress callback */
  onProgress?: ProgressCallback;

  /** Abort signal */
  signal?: AbortSignal;
}

/**
 * ScrapeUseCase - React Native scraping with fetch-based adapters
 */
export class ScrapeUseCase {
  private fetchScraper: ScraperPort;
  private poolScraper: ScraperPort;

  constructor() {
    this.fetchScraper = new FetchScraperAdapter();
    this.poolScraper = new FetchPoolScraperAdapter();
  }

  /** Scrape a URL using fetch-based adapters */
  async execute(
    url: string,
    options: ScrapeUseCaseOptions = {},
  ): Promise<ScrapeResponse> {
    const { usePooling = true, ...scrapeOptions } = options;
    const scraper = usePooling ? this.poolScraper : this.fetchScraper;
    return scraper.scrape(url, scrapeOptions);
  }

  /** Scrape multiple URLs */
  async executeMany(
    urls: string[],
    options: ScrapeUseCaseOptions & { concurrency?: number } = {},
  ): Promise<Map<string, ScrapeResult>> {
    const {
      usePooling = true,
      concurrency = 10,
      onProgress,
      signal,
      ...rest
    } = options;
    const scraper = usePooling ? this.poolScraper : this.fetchScraper;
    const result = await scraper.scrapeMany(urls, {
      concurrency,
      onProgress,
      signal,
      ...rest,
    });
    return result.results;
  }

  /** Get available scrapers on this platform */
  async getAvailableScrapers(): Promise<string[]> {
    return ["fetch", "fetch-pool"];
  }
}

/** Create a scrape use case for React Native */
export function createScrapeUseCase(): ScrapeUseCase {
  return new ScrapeUseCase();
}
