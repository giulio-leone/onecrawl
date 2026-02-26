/**
 * Search Adapter
 * Implements SearchPort using browser-based search engine scraping.
 */

import type { ScraperPort, SearchPort } from "../../ports/index.js";
import type {
  SearchResults,
  SearchOptions,
  BatchSearchResult,
  BatchOptions,
  ProgressCallback,
} from "../../domain/schemas.js";
import type { SearchQuery } from "../../ports/index.js";
import { buildSearchUrl } from "../../utils/url-builder.js";
import { parseSearchResults } from "./parsers.js";
import { sleep } from "../../utils/stealth.js";

/**
 * SearchAdapter - SearchPort implementation using scraper
 */
export class SearchAdapter implements SearchPort {
  constructor(private scraper: ScraperPort) {}

  async search(
    query: string,
    options: Partial<SearchOptions> & {
      onProgress?: ProgressCallback;
      signal?: AbortSignal;
    } = {},
  ): Promise<SearchResults> {
    const {
      engine = "duckduckgo",
      type = "web",
      maxResults = 10,
      lang,
      region,
      onProgress,
      signal,
    } = options;

    const startTime = Date.now();

    onProgress?.({
      phase: "starting",
      message: `Searching "${query}" on ${engine}...`,
    });

    // Build search URL
    const searchUrl = buildSearchUrl(query, engine, type, { lang, region });

    // Scrape the search page
    const response = await this.scraper.scrape(searchUrl, {
      timeout: 30000,
      waitFor: "domcontentloaded",
      extractMedia: type === "image" || type === "video",
      extractLinks: false,
      extractMetadata: false,
      cache: false, // Always fresh for search
      onProgress,
      signal,
    });

    // Parse results
    const results = parseSearchResults(
      response.result.html || response.result.content,
      engine,
      maxResults,
    );

    onProgress?.({
      phase: "complete",
      message: `Found ${results.length} results`,
    });

    return {
      query,
      results,
      totalResults: results.length,
      searchTime: Date.now() - startTime,
    };
  }

  async searchParallel(
    queries: SearchQuery[],
    options: Partial<BatchOptions> & {
      onProgress?: ProgressCallback;
      signal?: AbortSignal;
    } = {},
  ): Promise<BatchSearchResult> {
    const {
      concurrency = 2,
      retries = 1,
      retryDelay = 1000,
      onProgress,
      signal,
    } = options;

    const startTime = Date.now();
    const results: SearchResults[] = [];
    const failed = new Map<string, Error>();

    // Process queries in batches
    for (let i = 0; i < queries.length; i += concurrency) {
      if (signal?.aborted) break;

      const batch = queries.slice(i, i + concurrency);

      onProgress?.({
        phase: "extracting",
        message: `Searching batch ${Math.floor(i / concurrency) + 1}...`,
        progress: i,
        total: queries.length,
      });

      const promises = batch.map(async (q) => {
        let lastError: Error | null = null;

        for (let attempt = 0; attempt <= retries; attempt++) {
          try {
            const result = await this.search(q.query, {
              engine: q.engine,
              type: q.type,
              signal,
            });
            results.push(result);
            return;
          } catch (error) {
            lastError =
              error instanceof Error ? error : new Error(String(error));
            if (attempt < retries) {
              await sleep(retryDelay * (attempt + 1));
            }
          }
        }

        if (lastError) {
          failed.set(q.query, lastError);
        }
      });

      await Promise.all(promises);

      // Delay between batches to avoid rate limiting
      if (i + concurrency < queries.length) {
        await sleep(1000 + Math.random() * 1000);
      }
    }

    onProgress?.({
      phase: "complete",
      message: `Completed: ${results.length} success, ${failed.size} failed`,
    });

    return {
      results,
      failed,
      totalDuration: Date.now() - startTime,
    };
  }

  async isAvailable(): Promise<boolean> {
    return this.scraper.isAvailable();
  }

  getName(): string {
    return `search-${this.scraper.getName()}`;
  }
}

/**
 * Create a search adapter
 */
export function createSearchAdapter(scraper: ScraperPort): SearchPort {
  return new SearchAdapter(scraper);
}
