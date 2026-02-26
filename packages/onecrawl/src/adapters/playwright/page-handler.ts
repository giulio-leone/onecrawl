/**
 * Playwright page lifecycle and content extraction.
 * Handles browser launch, context creation, page navigation, and scraping.
 */

import type { Browser, BrowserContext, Page, Response } from "playwright";
import type { ScrapeResult, ProgressCallback } from "../../domain/schemas.js";
import {
  htmlToText,
  htmlToMarkdown,
  extractLinks,
  extractMedia,
  extractMetadata,
} from "../../utils/content-parser.js";
import {
  getRandomUserAgent,
  getRandomViewport,
  getStealthScript,
  getRandomTimezone,
  generateFingerprint,
  sleep,
} from "../../utils/stealth.js";
import type { Fingerprint } from "../../utils/stealth.js";

/** Options controlling what to extract from a page. */
export interface PageScrapeOptions {
  timeout: number;
  waitFor?: string;
  waitForSelector?: string;
  jsCode?: string;
  shouldExtractMedia: boolean;
  shouldExtractLinks: boolean;
  shouldExtractMetadata: boolean;
  onProgress?: ProgressCallback;
}

/** Launch a headless Chromium browser. */
export async function launchBrowser(): Promise<Browser> {
  const playwright = await import("playwright");
  return playwright.chromium.launch({
    headless: true,
    args: [
      "--no-sandbox",
      "--disable-setuid-sandbox",
      "--disable-dev-shm-usage",
      "--disable-accelerated-2d-canvas",
      "--no-first-run",
      "--no-zygote",
      "--disable-gpu",
    ],
  });
}

/** Create a stealth browser context with randomised fingerprints. */
export async function createStealthContext(
  browser: Browser,
  fp?: Fingerprint,
): Promise<BrowserContext> {
  const fingerprint = fp ?? generateFingerprint();
  return browser.newContext({
    viewport: fingerprint.viewport,
    userAgent: fingerprint.userAgent,
    locale: fingerprint.locale,
    timezoneId: fingerprint.timezoneId,
    deviceScaleFactor: fingerprint.deviceScaleFactor,
    javaScriptEnabled: true,
    bypassCSP: true,
    extraHTTPHeaders: {
      "Accept-Language": fingerprint.locale + ",en;q=0.9",
    },
  });
}

/** Navigate a page, run optional JS, and extract content into ScrapeResult. */
export async function scrapePage(
  page: Page,
  url: string,
  startTime: number,
  opts: PageScrapeOptions & { fingerprint?: Fingerprint },
): Promise<{ result: ScrapeResult; response: Response | null }> {
  await page.addInitScript(getStealthScript(opts.fingerprint));

  opts.onProgress?.({ phase: "navigating", message: "Loading page...", url });

  const response = await page.goto(url, {
    waitUntil:
      opts.waitFor === "networkidle" ? "networkidle" : (opts.waitFor as "load"),
    timeout: opts.timeout,
  });

  if (opts.waitForSelector) {
    await page.waitForSelector(opts.waitForSelector, { timeout: opts.timeout });
  }

  if (opts.jsCode) {
    await page.evaluate(opts.jsCode);
    await sleep(500);
  }

  opts.onProgress?.({
    phase: "extracting",
    message: "Extracting content...",
    url,
  });

  const html = await page.content();
  const title = await page.title();

  const result: ScrapeResult = {
    url: page.url(),
    title,
    content: htmlToText(html),
    markdown: htmlToMarkdown(html),
    html,
    statusCode: response?.status(),
    contentType: response?.headers()["content-type"],
    loadTime: Date.now() - startTime,
  };

  if (opts.shouldExtractLinks) result.links = extractLinks(html, url);
  if (opts.shouldExtractMedia) result.media = extractMedia(html, url);
  if (opts.shouldExtractMetadata) result.metadata = extractMetadata(html);

  return { result, response };
}
