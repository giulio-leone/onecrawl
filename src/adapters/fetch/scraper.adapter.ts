/**
 * Fetch-based Scraper Adapter
 */

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
import { getRandomUserAgent } from "../../utils/stealth.js";
import { batchScrape } from "../shared/batch-scrape.js";

/** Lightweight scraper for sites that don't require JS rendering. */
export class FetchScraperAdapter implements ScraperPort {
  private cache = new Map<string, { data: ScrapeResult; timestamp: number }>();
  private cacheTTL: number;

  constructor(cacheTTL = 30 * 60 * 1000) {
    this.cacheTTL = cacheTTL;
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
      extractMedia: shouldExtractMedia = true,
      extractLinks: shouldExtractLinks = true,
      extractMetadata: shouldExtractMetadata = true,
      cache: useCache = true,
      onProgress,
      signal,
    } = options;

    const startTime = Date.now();

    // Check cache
    if (useCache) {
      const cached = this.cache.get(url);
      if (cached && Date.now() - cached.timestamp < this.cacheTTL) {
        onProgress?.({ phase: "complete", message: "From cache", url });
        return {
          result: cached.data,
          cached: true,
          duration: Date.now() - startTime,
          source: this.getName(),
        };
      }
    }

    if (signal?.aborted) {
      throw new Error("Scrape aborted");
    }

    onProgress?.({ phase: "starting", message: `Fetching ${url}...`, url });

    const controller = new AbortController();
    const timeoutId = setTimeout(() => controller.abort(), timeout);

    // Forward external abort signal
    if (signal) {
      signal.addEventListener("abort", () => controller.abort());
    }

    try {
      const response = await fetch(url, {
        signal: controller.signal,
        headers: {
          "User-Agent": getRandomUserAgent(),
          Accept:
            "text/html,application/xhtml+xml,application/xml;q=0.9,*/*;q=0.8",
          "Accept-Language": "en-US,en;q=0.9",
        },
      });

      clearTimeout(timeoutId);

      if (!response.ok) {
        throw new Error(`HTTP ${response.status}: ${response.statusText}`);
      }

      onProgress?.({
        phase: "extracting",
        message: "Extracting content...",
        url,
      });

      const html = await response.text();
      const contentType = response.headers.get("content-type") || "";

      // Extract title from HTML
      const titleMatch = html.match(/<title[^>]*>(.*?)<\/title>/i);
      const title = titleMatch ? htmlToText(titleMatch[1] || "") : "";

      const result: ScrapeResult = {
        url: response.url,
        title,
        content: htmlToText(html),
        markdown: htmlToMarkdown(html),
        html,
        statusCode: response.status,
        contentType,
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
        this.cache.set(url, { data: result, timestamp: Date.now() });
      }

      onProgress?.({
        phase: "complete",
        message: `Fetched ${result.content.length} chars`,
        url,
      });

      return {
        result,
        cached: false,
        duration: Date.now() - startTime,
        source: this.getName(),
      };
    } catch (error) {
      clearTimeout(timeoutId);
      throw error;
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
      concurrency = 5,
      retries = 2,
      retryDelay = 1000,
      onProgress,
      signal,
      ...scrapeOptions
    } = options;

    return batchScrape(urls, this.scrape.bind(this), {
      concurrency,
      retries,
      retryDelay,
      onProgress,
      signal,
      scrapeOptions,
    });
  }

  async isAvailable(): Promise<boolean> {
    return true;
  }

  getName(): string {
    return "fetch";
  }

  clearCache(): void {
    this.cache.clear();
  }
}

/** Create a fetch-based scraper adapter */
export function createFetchScraperAdapter(): ScraperPort {
  return new FetchScraperAdapter();
}
