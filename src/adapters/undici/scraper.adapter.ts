/**
 * Undici HTTP/2 Scraper Adapter
 * High-performance scraper with HTTP/2 multiplexing and connection pooling.
 */

import { Pool } from "undici";
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
import { getRandomUserAgent, sleep } from "../../utils/stealth.js";

/** LRU-style cache entry */
interface CacheEntry {
  data: ScrapeResult;
  timestamp: number;
  etag?: string;
  lastModified?: string;
}

/** Connection pool per origin */
const pools = new Map<string, Pool>();

/** Get or create connection pool for origin */
function getPool(origin: string): Pool {
  let pool = pools.get(origin);
  if (!pool) {
    pool = new Pool(origin, {
      connections: 10,
      pipelining: 6,
      keepAliveTimeout: 30000,
      keepAliveMaxTimeout: 60000,
    });
    pools.set(origin, pool);
  }
  return pool;
}

/**
 * UndiciScraperAdapter - HTTP/2 with connection pooling
 *
 * Benefits over fetch:
 * - HTTP/2 multiplexing (multiple requests on single connection)
 * - Connection pooling (reuse connections)
 * - Request pipelining (send before previous completes)
 * - ETag/If-Modified-Since caching
 */
export class UndiciScraperAdapter implements ScraperPort {
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

    // Request deduplication - don't fetch same URL twice simultaneously
    const pending = this.pendingRequests.get(url);
    if (pending) {
      return pending;
    }

    // Check cache with conditional request support
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

    if (signal?.aborted) {
      throw new Error("Scrape aborted");
    }

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
    const { timeout, onProgress, signal, startTime, cached } = opts;

    onProgress?.({ phase: "starting", message: `Fetching ${url}...`, url });

    const parsedUrl = new URL(url);
    const origin = parsedUrl.origin;

    // Build conditional request headers
    const headers: Record<string, string> = {
      "User-Agent": getRandomUserAgent(),
      Accept: "text/html,application/xhtml+xml,application/xml;q=0.9,*/*;q=0.8",
      "Accept-Language": "en-US,en;q=0.9",
      "Accept-Encoding": "gzip, deflate",
    };

    // Add conditional headers if we have cached version
    if (cached?.etag) {
      headers["If-None-Match"] = cached.etag;
    }
    if (cached?.lastModified) {
      headers["If-Modified-Since"] = cached.lastModified;
    }

    const pool = getPool(origin);
    const response = await pool.request({
      path: parsedUrl.pathname + parsedUrl.search,
      method: "GET",
      headers,
      headersTimeout: timeout,
      bodyTimeout: timeout,
    });

    // Handle 304 Not Modified - use cached version
    if (response.statusCode === 304 && cached) {
      onProgress?.({ phase: "complete", message: "Not modified", url });
      return {
        result: cached.data,
        cached: true,
        duration: Date.now() - startTime,
        source: this.getName(),
      };
    }

    if (response.statusCode >= 400) {
      throw new Error(`HTTP ${response.statusCode}`);
    }

    onProgress?.({
      phase: "extracting",
      message: "Extracting content...",
      url,
    });

    const html = await response.body.text();
    const contentType = (response.headers["content-type"] as string) || "";

    // Extract title
    const titleMatch = html.match(/<title[^>]*>(.*?)<\/title>/i);
    const title = titleMatch ? htmlToText(titleMatch[1] || "") : "";

    const result: ScrapeResult = {
      url: response.headers["location"]
        ? new URL(response.headers["location"] as string, url).href
        : url,
      title,
      content: htmlToText(html),
      markdown: htmlToMarkdown(html),
      html,
      statusCode: response.statusCode,
      contentType,
      loadTime: Date.now() - startTime,
    };

    if (opts.shouldExtractLinks) {
      result.links = extractLinks(html, url);
    }

    if (opts.shouldExtractMedia) {
      result.media = extractMedia(html, url);
    }

    if (opts.shouldExtractMetadata) {
      result.metadata = extractMetadata(html);
    }

    // Cache with ETag/Last-Modified for conditional requests
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
    // Evict oldest entries if cache is full
    if (this.cache.size >= this.maxCacheSize) {
      const oldest = [...this.cache.entries()]
        .sort((a, b) => a[1].timestamp - b[1].timestamp)
        .slice(0, Math.floor(this.maxCacheSize * 0.1));
      for (const [key] of oldest) {
        this.cache.delete(key);
      }
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
    const results = new Map<string, ScrapeResult>();
    const failed = new Map<string, Error>();

    // Group URLs by origin for optimal connection reuse
    const byOrigin = new Map<string, string[]>();
    for (const url of urls) {
      try {
        const origin = new URL(url).origin;
        const group = byOrigin.get(origin) || [];
        group.push(url);
        byOrigin.set(origin, group);
      } catch {
        failed.set(url, new Error("Invalid URL"));
      }
    }

    // Process each origin group with its own concurrency
    const originPromises = [...byOrigin.entries()].map(
      async ([origin, originUrls]) => {
        for (let i = 0; i < originUrls.length; i += concurrency) {
          if (signal?.aborted) break;

          const batch = originUrls.slice(i, i + concurrency);

          onProgress?.({
            phase: "extracting",
            message: `Fetching from ${origin}...`,
            url: batch[0]!,
            progress: results.size,
            total: urls.length,
          });

          const promises = batch.map(async (url) => {
            let lastError: Error | null = null;

            for (let attempt = 0; attempt <= retries; attempt++) {
              try {
                const response = await this.scrape(url, {
                  ...scrapeOptions,
                  signal,
                });
                results.set(url, response.result);
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
              failed.set(url, lastError);
            }
          });

          await Promise.all(promises);
        }
      },
    );

    await Promise.all(originPromises);

    onProgress?.({
      phase: "complete",
      message: `Completed: ${results.size} success, ${failed.size} failed`,
      url: urls[0]!,
    });

    return {
      results,
      failed,
      totalDuration: Date.now() - startTime,
    };
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

  /** Close all connection pools */
  async close(): Promise<void> {
    const closePromises = [...pools.values()].map((pool) => pool.close());
    await Promise.all(closePromises);
    pools.clear();
  }
}

/** Create an undici-based scraper adapter */
export function createUndiciScraperAdapter(
  options?: { cacheTTL?: number; maxCacheSize?: number },
): ScraperPort {
  return new UndiciScraperAdapter(options);
}
