/**
 * Undici HTTP/2 Scraper Adapter
 * High-performance scraper with HTTP/2 multiplexing and connection pooling.
 */

import type { ScraperPort, ScrapeResponse } from "../../ports/index.js";
import type {
  ScrapeResult,
  ScrapeOptions,
  BatchScrapeResult,
  BatchOptions,
  ProgressCallback,
} from "../../domain/schemas.js";
import { getRandomUserAgent } from "../../utils/stealth.js";
import { batchScrape } from "../shared/batch-scrape.js";
import { PoolManager } from "./pool-manager.js";
import { parseUndiciResponse } from "./response-handler.js";

/** LRU-style cache entry */
interface CacheEntry {
  data: ScrapeResult;
  timestamp: number;
  etag?: string;
  lastModified?: string;
}

/** Connection pool per origin - instance-scoped */
export class UndiciScraperAdapter implements ScraperPort {
  private poolManager = new PoolManager();
  private cache = new Map<string, CacheEntry>();
  private cacheTTL: number;
  private maxCacheSize: number;
  private pendingRequests = new Map<string, Promise<ScrapeResponse>>();

  constructor(options: { cacheTTL?: number; maxCacheSize?: number } = {}) {
    this.cacheTTL = options.cacheTTL ?? 30 * 60 * 1000;
    this.maxCacheSize = options.maxCacheSize ?? 500;
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

    const pending = this.pendingRequests.get(url);
    if (pending) return pending;

    const cached = useCache ? this.cache.get(url) : undefined;
    if (cached && Date.now() - cached.timestamp < this.cacheTTL) {
      onProgress?.({ phase: "complete", message: "From cache", url });
      return {
        result: cached.data,
        cached: true,
        duration: Date.now() - startTime,
        source: this.getName(),
      };
    }

    if (signal?.aborted) throw new Error("Scrape aborted");

    const requestPromise = this.executeRequest(url, {
      timeout,
      shouldExtractMedia,
      shouldExtractLinks,
      shouldExtractMetadata,
      useCache,
      onProgress,
      signal,
      startTime,
      cached,
    });

    this.pendingRequests.set(url, requestPromise);
    try {
      return await requestPromise;
    } finally {
      this.pendingRequests.delete(url);
    }
  }

  private async executeRequest(
    url: string,
    opts: {
      timeout: number;
      shouldExtractMedia: boolean;
      shouldExtractLinks: boolean;
      shouldExtractMetadata: boolean;
      useCache: boolean;
      onProgress?: ProgressCallback;
      signal?: AbortSignal;
      startTime: number;
      cached?: CacheEntry;
    },
  ): Promise<ScrapeResponse> {
    const { timeout, onProgress, startTime, cached } = opts;
    onProgress?.({ phase: "starting", message: `Fetching ${url}...`, url });

    const parsedUrl = new URL(url);
    const headers: Record<string, string> = {
      "User-Agent": getRandomUserAgent(),
      Accept: "text/html,application/xhtml+xml,application/xml;q=0.9,*/*;q=0.8",
      "Accept-Language": "en-US,en;q=0.9",
      "Accept-Encoding": "gzip, deflate",
    };
    if (cached?.etag) headers["If-None-Match"] = cached.etag;
    if (cached?.lastModified)
      headers["If-Modified-Since"] = cached.lastModified;

    const pool = this.poolManager.getPool(parsedUrl.origin);
    const response = await pool.request({
      path: parsedUrl.pathname + parsedUrl.search,
      method: "GET",
      headers,
      headersTimeout: timeout,
      bodyTimeout: timeout,
    });

    if (response.statusCode === 304 && cached) {
      onProgress?.({ phase: "complete", message: "Not modified", url });
      return {
        result: cached.data,
        cached: true,
        duration: Date.now() - startTime,
        source: this.getName(),
      };
    }
    if (response.statusCode >= 400)
      throw new Error(`HTTP ${response.statusCode}`);

    onProgress?.({
      phase: "extracting",
      message: "Extracting content...",
      url,
    });
    const result = await parseUndiciResponse(response, url, startTime, opts);

    if (opts.useCache) {
      this.setCache(url, {
        data: result,
        timestamp: Date.now(),
        etag: response.headers["etag"] as string | undefined,
        lastModified: response.headers["last-modified"] as string | undefined,
      });
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
  }

  private setCache(url: string, entry: CacheEntry): void {
    if (this.cache.size >= this.maxCacheSize) {
      const oldest = [...this.cache.entries()]
        .sort((a, b) => a[1].timestamp - b[1].timestamp)
        .slice(0, Math.floor(this.maxCacheSize * 0.1));
      for (const [key] of oldest) this.cache.delete(key);
    }
    this.cache.set(url, entry);
  }

  async scrapeMany(
    urls: string[],
    options: Partial<ScrapeOptions & BatchOptions> & {
      onProgress?: ProgressCallback;
      signal?: AbortSignal;
    } = {},
  ): Promise<BatchScrapeResult> {
    const {
      concurrency = 10,
      retries = 2,
      retryDelay = 1000,
      onProgress,
      signal,
      ...scrapeOptions
    } = options;
    const startTime = Date.now();
    const failed = new Map<string, Error>();

    const byOrigin = new Map<string, string[]>();
    for (const url of urls) {
      try {
        const group = byOrigin.get(new URL(url).origin) || [];
        group.push(url);
        byOrigin.set(new URL(url).origin, group);
      } catch {
        failed.set(url, new Error("Invalid URL"));
      }
    }

    const batchResults = await Promise.all(
      [...byOrigin.values()].map((originUrls) =>
        batchScrape(originUrls, this.scrape.bind(this), {
          concurrency,
          retries,
          retryDelay,
          onProgress,
          signal,
          scrapeOptions,
        }),
      ),
    );

    const results = new Map<string, ScrapeResult>();
    for (const batch of batchResults) {
      for (const [url, result] of batch.results) results.set(url, result);
      for (const [url, error] of batch.failed) failed.set(url, error);
    }

    onProgress?.({
      phase: "complete",
      message: `Completed: ${results.size} success, ${failed.size} failed`,
      url: urls[0]!,
    });
    return { results, failed, totalDuration: Date.now() - startTime };
  }

  async isAvailable(): Promise<boolean> {
    return true;
  }
  getName(): string {
    return "undici";
  }
  clearCache(): void {
    this.cache.clear();
  }

  async close(): Promise<void> {
    await this.poolManager.closeAll();
  }
}

/** Create an undici-based scraper adapter */
export function createUndiciScraperAdapter(options?: {
  cacheTTL?: number;
  maxCacheSize?: number;
}): ScraperPort {
  return new UndiciScraperAdapter(options);
}
