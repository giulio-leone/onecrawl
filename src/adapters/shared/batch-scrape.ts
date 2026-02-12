/**
 * Shared batch scrape utility.
 * Extracts the duplicated retry-per-URL logic used across all scraper adapters.
 */

import type { ScrapeResponse } from "../../ports/index.js";
import type {
  ScrapeResult,
  ScrapeOptions,
  BatchScrapeResult,
  ProgressCallback,
} from "../../domain/schemas.js";
import { sleep } from "../../utils/stealth.js";

/** Function that scrapes a single URL (adapter's own scrape method). */
export type ScrapeFn = (
  url: string,
  options: Partial<ScrapeOptions> & { signal?: AbortSignal },
) => Promise<ScrapeResponse>;

/** Options for batch scraping. */
export interface BatchScrapeOptions {
  concurrency: number;
  retries: number;
  retryDelay: number;
  onProgress?: ProgressCallback;
  signal?: AbortSignal;
  scrapeOptions: Partial<ScrapeOptions>;
}

/** Scrape a single URL with retry and exponential backoff. */
async function scrapeWithRetry(
  url: string,
  scrapeFn: ScrapeFn,
  opts: BatchScrapeOptions,
  results: Map<string, ScrapeResult>,
  failed: Map<string, Error>,
): Promise<void> {
  let lastError: Error | null = null;

  for (let attempt = 0; attempt <= opts.retries; attempt++) {
    if (opts.signal?.aborted) break;
    try {
      const response = await scrapeFn(url, {
        ...opts.scrapeOptions,
        signal: opts.signal,
      });
      results.set(url, response.result);
      return;
    } catch (error) {
      lastError = error instanceof Error ? error : new Error(String(error));
      if (attempt < opts.retries) {
        await sleep(opts.retryDelay * (attempt + 1));
      }
    }
  }

  if (lastError) failed.set(url, lastError);
}

/** Process URLs in sequential batches of `concurrency`. */
export async function batchScrape(
  urls: string[],
  scrapeFn: ScrapeFn,
  opts: BatchScrapeOptions,
): Promise<BatchScrapeResult> {
  const startTime = Date.now();
  const results = new Map<string, ScrapeResult>();
  const failed = new Map<string, Error>();

  for (let i = 0; i < urls.length; i += opts.concurrency) {
    if (opts.signal?.aborted) break;

    const batch = urls.slice(i, i + opts.concurrency);
    opts.onProgress?.({
      phase: "extracting",
      message: `Processing batch ${Math.floor(i / opts.concurrency) + 1}...`,
      url: batch[0]!,
      progress: i,
      total: urls.length,
    });

    await Promise.all(
      batch.map((url) => scrapeWithRetry(url, scrapeFn, opts, results, failed)),
    );
  }

  opts.onProgress?.({
    phase: "complete",
    message: `Completed: ${results.size} success, ${failed.size} failed`,
    url: urls[0]!,
  });

  return { results, failed, totalDuration: Date.now() - startTime };
}
