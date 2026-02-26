/**
 * OneCrawl - React Native Compatible Entry Point
 * Excludes Node-specific adapters (Playwright, CDP, Undici).
 *
 * @packageDocumentation
 */

// Domain - Types and schemas (cross-platform)
export * from "./domain/index.js";

// Ports - Interface contracts (cross-platform)
export * from "./ports/index.js";

// Adapters - Only cross-platform adapters
export * from "./adapters/index.native.js";

// Use Cases - Cross-platform versions
export * from "./use-cases/index.native.js";

// Auth - Cross-platform auth only
export * from "./auth/index.native.js";

// Utils - Helpers (cross-platform)
export { buildSearchUrl } from "./utils/url-builder.js";

export {
  htmlToText,
  htmlToMarkdown,
  extractLinks,
  extractMedia,
  extractMetadata,
} from "./utils/content-parser.js";

export {
  getRandomUserAgent,
  getRandomViewport,
  getStealthScript,
  getRandomDelay,
  sleep,
} from "./utils/stealth.js";

// Convenience factory function
import {
  createScrapeUseCase,
  createSearchUseCase,
} from "./use-cases/index.native.js";

/** Create an OneCrawl instance for React Native */
export function createOneCrawl() {
  const scrapeUseCase = createScrapeUseCase();
  const searchUseCase = createSearchUseCase();

  return {
    /** Scrape a URL */
    scrape: scrapeUseCase.execute.bind(scrapeUseCase),

    /** Scrape multiple URLs */
    scrapeMany: scrapeUseCase.executeMany.bind(scrapeUseCase),

    /** Search the web */
    search: searchUseCase.execute.bind(searchUseCase),

    /** Search multiple queries */
    searchMany: searchUseCase.executeMany.bind(searchUseCase),

    /** Get available scrapers */
    getAvailableScrapers:
      scrapeUseCase.getAvailableScrapers.bind(scrapeUseCase),
  };
}

export type OneCrawl = ReturnType<typeof createOneCrawl>;
