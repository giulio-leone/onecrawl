/**
 * Undici response handling utilities.
 * Parses HTTP responses into ScrapeResult with content extraction.
 */

import { createGunzip, createInflate } from "node:zlib";
import { pipeline } from "node:stream/promises";
import { Writable } from "node:stream";
import type { Dispatcher } from "undici";
import type { ScrapeResult, ProgressCallback } from "../../domain/schemas.js";
import {
  htmlToText,
  htmlToMarkdown,
  extractLinks,
  extractMedia,
  extractMetadata,
} from "../../utils/content-parser.js";

/** Extraction flags for response parsing. */
export interface ExtractionFlags {
  shouldExtractMedia: boolean;
  shouldExtractLinks: boolean;
  shouldExtractMetadata: boolean;
}

/** Decompress response body if Content-Encoding is gzip/deflate. */
async function readBody(response: Dispatcher.ResponseData): Promise<string> {
  const encoding = (
    (response.headers["content-encoding"] as string) ?? ""
  ).toLowerCase();

  if (encoding === "gzip" || encoding === "deflate") {
    const chunks: Buffer[] = [];
    const collector = new Writable({
      write(chunk, _enc, cb) {
        chunks.push(chunk as Buffer);
        cb();
      },
    });
    const decompressor =
      encoding === "gzip" ? createGunzip() : createInflate();
    await pipeline(response.body, decompressor, collector);
    return Buffer.concat(chunks).toString("utf-8");
  }

  return response.body.text();
}

/** Parse an undici response into a ScrapeResult. */
export async function parseUndiciResponse(
  response: Dispatcher.ResponseData,
  url: string,
  startTime: number,
  flags: ExtractionFlags,
): Promise<ScrapeResult> {
  const html = await readBody(response);
  const contentType = (response.headers["content-type"] as string) || "";

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

  if (flags.shouldExtractLinks) result.links = extractLinks(html, url);
  if (flags.shouldExtractMedia) result.media = extractMedia(html, url);
  if (flags.shouldExtractMetadata) result.metadata = extractMetadata(html);

  return result;
}
