/**
 * Retry and cleanup utilities for fetch-pool adapter.
 * Handles scrape-with-retry logic and stale pending request cleanup.
 */

import type { ScrapeResult, ScrapeOptions } from "../../domain/schemas.js";
import type { ScrapeResponse } from "../../ports/index.js";
import { sleep } from "../../utils/stealth.js";

const STALE_PENDING_THRESHOLD_MS = 30_000;

/** Scrape a single URL with retry and exponential backoff + jitter. */
export async function scrapeWithRetry(
  url: string,
  scrapeFn: (
    url: string,
    options: Partial<ScrapeOptions> & { signal?: AbortSignal },
  ) => Promise<ScrapeResponse>,
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
      const response = await scrapeFn(url, {
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

/** Start a periodic timer that removes stale pending requests. */
export function startStalePendingCleanup(
  pendingRequests: Map<string, { startedAt: number }>,
): ReturnType<typeof setInterval> {
  const timer = setInterval(() => {
    const now = Date.now();
    for (const [url, entry] of pendingRequests) {
      if (now - entry.startedAt > STALE_PENDING_THRESHOLD_MS) {
        pendingRequests.delete(url);
      }
    }
  }, STALE_PENDING_THRESHOLD_MS);

  if (typeof timer === "object" && "unref" in timer) {
    timer.unref();
  }
  return timer;
}

/** Parse Cache-Control max-age into milliseconds, or undefined. */
export function parseCacheControlMaxAge(
  response: Response,
): number | undefined {
  const cc = response.headers.get("cache-control");
  if (!cc) return undefined;
  const match = cc.match(/max-age=(\d+)/);
  if (!match) return undefined;
  const seconds = parseInt(match[1]!, 10);
  return seconds > 0 ? seconds * 1000 : undefined;
}
