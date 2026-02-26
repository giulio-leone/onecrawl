/**
 * OneCrawl - Native TypeScript Web Crawler
 *
 * A lightweight, efficient web crawler with hexagonal architecture.
 * Zero Python dependencies, pure TypeScript.
 *
 * Features:
 * - HTTP/2 with undici (fast connection pooling)
 * - CDP Direct (Playwright-free browser automation)
 * - Agent Swarm (AI-powered distributed crawling)
 * - Cookie/Proxy support (auth and anti-bot)
 *
 * @packageDocumentation
 */

// Domain - Types and schemas
export * from "./domain/index.js";

// Ports - Interface contracts
export * from "./ports/index.js";

// Adapters - Implementations
export * from "./adapters/index.js";

// Use Cases - High-level API
export * from "./use-cases/index.js";

// Auth - Cookies and proxies
export * from "./auth/index.js";

// Utils - Helpers
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
  getRandomTimezone,
  generateFingerprint,
  sleep,
} from "./utils/stealth.js";

export type { Fingerprint } from "./utils/stealth.js";

export {
  extractToolsFromHTML,
  extractInternalLinks,
  globToRegex,
  matchesPatterns,
} from "./utils/semantic-extractor.js";

export { SemanticCrawlUseCase } from "./use-cases/semantic-crawl.use-case.js";

// Convenience factory function
import { createScrapeUseCase, createSearchUseCase } from "./use-cases/index.js";

/**
 * Create an OneCrawl instance with default configuration
 */
export function createOneCrawl() {
  const scrapeUseCase = createScrapeUseCase();
  const searchUseCase = createSearchUseCase();

  return {
    /**
     * Scrape a URL
     */
    scrape: scrapeUseCase.execute.bind(scrapeUseCase),

    /**
     * Scrape multiple URLs
     */
    scrapeMany: scrapeUseCase.executeMany.bind(scrapeUseCase),

    /**
     * Search the web
     */
    search: searchUseCase.execute.bind(searchUseCase),

    /**
     * Search multiple queries
     */
    searchMany: searchUseCase.executeMany.bind(searchUseCase),

    /**
     * Get available scrapers
     */
    getAvailableScrapers:
      scrapeUseCase.getAvailableScrapers.bind(scrapeUseCase),
  };
}

export type OneCrawl = ReturnType<typeof createOneCrawl>;
