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
import { sleep, getRandomUserAgent } from "../../utils/stealth.js";
import { OriginQueue } from "./origin-queue.js";
import { LruCache, type CacheEntry } from "./lru-cache.js";
import {
  buildHeaders,
  fetchWithTimeout,
  parseResponse,
} from "./request-helpers.js";

const STALE_PENDING_THRESHOLD_MS = 30_000;

/** Configuration for the fetch-pool adapter. */
export interface FetchPoolOptions {
  /** Cache TTL in milliseconds (default: 30 minutes). */
  cacheTTL?: number;
  /** Maximum number of cached entries (default: 500). */
  maxCacheSize?: number;
  /** Maximum concurrent requests per origin (default: 6). */
  maxConcurrencyPerOrigin?: number;
}

/**
 * FetchPoolScraperAdapter - native fetch() with connection pooling semantics.
 *
 * Features over the basic FetchScraperAdapter:
 * - Per-origin request queue with configurable concurrency
 * - Request deduplication (same URL in-flight shares one promise)
 * - LRU cache with ETag / If-Modified-Since conditional requests
 * - Batch scraping grouped by origin for optimal throughput
 */
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
    this.startCleanupTimer();
  }

  /** Scrape a single URL with deduplication and caching. */
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

  /** Scrape many URLs with retry, leveraging origin queue for throughput. */
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
        this.scrapeWithRetry(url, rest, {
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

  /** Stop the periodic stale-pending cleanup timer. */
  destroy(): void {
    if (this.cleanupTimer) {
      clearInterval(this.cleanupTimer);
      this.cleanupTimer = null;
    }
  }

  // ---------------------------------------------------------------------------
  // Private helpers
  // ---------------------------------------------------------------------------

  /** Execute a single fetch with conditional request support. */
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

  /** Scrape a single URL with retry and exponential backoff. */
  private async scrapeWithRetry(
    url: string,
    scrapeOptions: Partial<ScrapeOptions>,
    ctx: {
      retries: number;
      retryDelay: number;
      signal?: AbortSignal;
      results: Map<string, ScrapeResult>;
      failed: Map<string, Error>;
    },
  ): Promise<void> {
    let lastError: Error | null = null;

    for (let attempt = 0; attempt <= ctx.retries; attempt++) {
      if (ctx.signal?.aborted) break;
      try {
        const response = await this.scrape(url, {
          ...scrapeOptions,
          signal: ctx.signal,
        });
        ctx.results.set(url, response.result);
        return;
      } catch (error) {
        lastError = error instanceof Error ? error : new Error(String(error));
        if (attempt < ctx.retries)
          await sleep(ctx.retryDelay * (attempt + 1) + Math.random() * 100);
      }
    }

    if (lastError) ctx.failed.set(url, lastError);
  }

  /** Periodically remove stale entries from pendingRequests. */
  private startCleanupTimer(): void {
    this.cleanupTimer = setInterval(() => {
      const now = Date.now();
      for (const [url, entry] of this.pendingRequests) {
        if (now - entry.startedAt > STALE_PENDING_THRESHOLD_MS) {
          this.pendingRequests.delete(url);
        }
      }
    }, STALE_PENDING_THRESHOLD_MS);
    // Allow the process to exit even if the timer is active
    if (typeof this.cleanupTimer === "object" && "unref" in this.cleanupTimer) {
      this.cleanupTimer.unref();
    }
  }
}

/** Parse Cache-Control max-age into milliseconds, or undefined. */
function parseCacheControlMaxAge(response: Response): number | undefined {
  const cc = response.headers.get("cache-control");
  if (!cc) return undefined;
  const match = cc.match(/max-age=(\d+)/);
  if (!match) return undefined;
  const seconds = parseInt(match[1]!, 10);
  return seconds > 0 ? seconds * 1000 : undefined;
}

/** Create a fetch-pool scraper adapter. */
export function createFetchPoolScraperAdapter(
  options?: FetchPoolOptions,
): ScraperPort {
  return new FetchPoolScraperAdapter(options);
}
