/**
 * HTTP request execution helpers for the fetch-pool adapter.
 * Handles conditional requests, timeout, and response parsing.
 */

import type { ScrapeResult } from "../../domain/schemas.js";
import {
  htmlToText,
  htmlToMarkdown,
  extractLinks,
  extractMedia,
  extractMetadata,
} from "../../utils/content-parser.js";
import { getRandomUserAgent } from "../../utils/stealth.js";
import type { CacheEntry } from "./lru-cache.js";

/** Build request headers, including conditional ones from stale cache. */
export function buildHeaders(
  stale?: CacheEntry<ScrapeResult>,
  userAgent?: string,
): Record<string, string> {
  const headers: Record<string, string> = {
    "User-Agent": userAgent ?? getRandomUserAgent(),
    Accept: "text/html,application/xhtml+xml,application/xml;q=0.9,*/*;q=0.8",
    "Accept-Language": "en-US,en;q=0.9",
    "Accept-Encoding": "gzip, deflate, br",
  };
  if (stale?.etag) headers["If-None-Match"] = stale.etag;
  if (stale?.lastModified) headers["If-Modified-Since"] = stale.lastModified;
  return headers;
}

/** Fetch with a timeout via AbortController, forwarding external signal. */
export async function fetchWithTimeout(
  url: string,
  headers: Record<string, string>,
  timeout: number,
  signal?: AbortSignal,
): Promise<Response> {
  const controller = new AbortController();
  const timeoutId = setTimeout(() => controller.abort(), timeout);

  const onAbort = () => controller.abort();
  signal?.addEventListener("abort", onAbort);

  try {
    return await fetch(url, { headers, signal: controller.signal });
  } finally {
    clearTimeout(timeoutId);
    signal?.removeEventListener("abort", onAbort);
  }
}

/** Extraction flags for parseResponse. */
export interface ParseFlags {
  doMedia: boolean;
  doLinks: boolean;
  doMeta: boolean;
}

/** Parse HTML response into ScrapeResult. */
export async function parseResponse(
  response: Response,
  url: string,
  startTime: number,
  opts: ParseFlags,
): Promise<ScrapeResult> {
  const html = await response.text();
  const contentType = response.headers.get("content-type") || "";
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

  if (opts.doLinks) result.links = extractLinks(html, url);
  if (opts.doMedia) result.media = extractMedia(html, url);
  if (opts.doMeta) result.metadata = extractMetadata(html);

  return result;
}
