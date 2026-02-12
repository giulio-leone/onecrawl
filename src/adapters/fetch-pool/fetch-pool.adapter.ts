/**
 * Fetch Pool Scraper Adapter
 * Platform-agnostic scraper with per-origin concurrency, request deduplication,
 * LRU caching with conditional requests, and retry with exponential backoff.
 * Works on both Node.js and React Native (no Node-specific APIs).
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
import { OriginQueue } from "./origin-queue.js";
import { LruCache, type CacheEntry } from "./lru-cache.js";
import {
  buildHeaders,
  fetchWithTimeout,
  parseResponse,
} from "./request-helpers.js";
import {
  scrapeWithRetry,
  startStalePendingCleanup,
  parseCacheControlMaxAge,
} from "./retry-scrape.js";

/** Configuration for the fetch-pool adapter. */
export interface FetchPoolOptions {
  cacheTTL?: number;
  maxCacheSize?: number;
  maxConcurrencyPerOrigin?: number;
}

export class FetchPoolScraperAdapter implements ScraperPort {
  private cache: LruCache<ScrapeResult>;
  private pendingRequests = new Map<
    string,
    { promise: Promise<ScrapeResponse>; startedAt: number }
  >();
  private originQueue: OriginQueue;
  private userAgent: string;
  private cleanupTimer: ReturnType<typeof setInterval> | null = null;

  constructor(options: FetchPoolOptions = {}) {
    const ttl = options.cacheTTL ?? 30 * 60 * 1000;
    const maxSize = options.maxCacheSize ?? 500;
    this.cache = new LruCache<ScrapeResult>(maxSize, ttl);
    this.originQueue = new OriginQueue(options.maxConcurrencyPerOrigin ?? 6);
    this.userAgent = getRandomUserAgent();
    this.cleanupTimer = startStalePendingCleanup(this.pendingRequests);
  }

  async scrape(
    url: string,
    options: Partial<ScrapeOptions> & {
      onProgress?: ProgressCallback;
      signal?: AbortSignal;
    } = {},
  ): Promise<ScrapeResponse> {
    const startTime = Date.now();
    const { cache: useCache = true, onProgress, signal } = options;

    const pending = this.pendingRequests.get(url);
    if (pending) return pending.promise;

    if (useCache) {
      const { fresh } = this.cache.lookup(url);
      if (fresh) {
        onProgress?.({ phase: "complete", message: "From cache", url });
        return this.cachedResponse(fresh.data, startTime);
      }
    }

    if (signal?.aborted) throw new Error("Scrape aborted");

    const stale = useCache ? this.cache.lookup(url).stale : undefined;
    const staleEntry = stale ?? undefined;
    const origin = new URL(url).origin;
    const promise = this.originQueue.enqueue(origin, () =>
      this.executeFetch(url, options, startTime, staleEntry),
    );
    this.pendingRequests.set(url, { promise, startedAt: Date.now() });

    try {
      return await promise;
    } finally {
      this.pendingRequests.delete(url);
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
      retries = 2,
      retryDelay = 1000,
      onProgress,
      signal,
      ...rest
    } = options;
    const startTime = Date.now();
    const results = new Map<string, ScrapeResult>();
    const failed = new Map<string, Error>();

    await Promise.allSettled(
      urls.map((url) =>
        scrapeWithRetry(url, this.scrape.bind(this), rest, {
          retries,
          retryDelay,
          signal,
          results,
          failed,
        }),
      ),
    );

    onProgress?.({
      phase: "complete",
      message: `Completed: ${results.size} success, ${failed.size} failed`,
      url: urls[0]!,
    });
    return { results, failed, totalDuration: Date.now() - startTime };
  }

  async isAvailable(): Promise<boolean> {
    return typeof fetch === "function";
  }
  getName(): string {
    return "fetch-pool";
  }
  clearCache(): void {
    this.cache.clear();
  }

  destroy(): void {
    if (this.cleanupTimer) {
      clearInterval(this.cleanupTimer);
      this.cleanupTimer = null;
    }
  }

  private async executeFetch(
    url: string,
    options: Partial<ScrapeOptions> & {
      onProgress?: ProgressCallback;
      signal?: AbortSignal;
    },
    startTime: number,
    stale?: CacheEntry<ScrapeResult>,
  ): Promise<ScrapeResponse> {
    const {
      timeout = 30000,
      extractMedia: doMedia = true,
      extractLinks: doLinks = true,
      extractMetadata: doMeta = true,
      cache: useCache = true,
      onProgress,
      signal,
    } = options;

    onProgress?.({ phase: "starting", message: `Fetching ${url}...`, url });

    const headers = buildHeaders(stale, this.userAgent);
    const response = await fetchWithTimeout(url, headers, timeout, signal);

    if (response.status === 304 && stale) {
      onProgress?.({ phase: "complete", message: "Not modified", url });
      return this.cachedResponse(stale.data, startTime);
    }
    if (!response.ok) throw new Error(`HTTP ${response.status}`);

    onProgress?.({
      phase: "extracting",
      message: "Extracting content...",
      url,
    });
    const result = await parseResponse(response, url, startTime, {
      doMedia,
      doLinks,
      doMeta,
    });

    if (useCache) {
      const ttlOverride = parseCacheControlMaxAge(response);
      this.cache.set(url, {
        data: result,
        timestamp: Date.now(),
        etag: response.headers.get("etag") ?? undefined,
        lastModified: response.headers.get("last-modified") ?? undefined,
        ttlOverride,
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

  private cachedResponse(
    data: ScrapeResult,
    startTime: number,
  ): ScrapeResponse {
    return {
      result: data,
      cached: true,
      duration: Date.now() - startTime,
      source: this.getName(),
    };
  }
}

/** Create a fetch-pool scraper adapter. */
export function createFetchPoolScraperAdapter(
  options?: FetchPoolOptions,
): ScraperPort {
  return new FetchPoolScraperAdapter(options);
}
