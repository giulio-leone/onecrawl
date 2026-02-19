/**
 * OneCrawl Port Interfaces
 * Hexagonal architecture contracts for browser automation and scraping.
 */

import type {
  ScrapeResult,
  ScrapeOptions,
  SearchResults,
  SearchOptions,
  LaunchConfig,
  BatchScrapeResult,
  BatchSearchResult,
  BatchOptions,
  ProgressCallback,
  Metadata,
  ExtractedMedia,
  Link,
} from "../domain/schemas.js";

// =============================================================================
// Browser Port - Browser automation abstraction
// =============================================================================

export interface PageHandle {
  url(): string;
  title(): Promise<string>;
  content(): Promise<string>;
  evaluate<T>(script: string | (() => T)): Promise<T>;
  waitForSelector(
    selector: string,
    options?: { timeout?: number },
  ): Promise<void>;
  waitForNavigation(options?: { timeout?: number }): Promise<void>;
  click(selector: string): Promise<void>;
  type(selector: string, text: string): Promise<void>;
  screenshot(options?: { fullPage?: boolean }): Promise<Buffer>;
  close(): Promise<void>;
}

export interface BrowserContext {
  newPage(): Promise<PageHandle>;
  cookies(): Promise<Array<{ name: string; value: string; domain: string }>>;
  setCookies(
    cookies: Array<{ name: string; value: string; domain: string }>,
  ): Promise<void>;
  close(): Promise<void>;
}

export interface BrowserPort {
  /**
   * Launch a new browser context
   */
  launch(config?: Partial<LaunchConfig>): Promise<BrowserContext>;

  /**
   * Navigate to URL and return page handle
   */
  navigate(url: string, options?: Partial<ScrapeOptions>): Promise<PageHandle>;

  /**
   * Check if browser is available
   */
  isAvailable(): Promise<boolean>;

  /**
   * Get adapter name
   */
  getName(): string;

  /**
   * Close all browser contexts
   */
  closeAll(): Promise<void>;
}

// =============================================================================
// Scraper Port - Web page scraping
// =============================================================================

export interface ScrapeResponse {
  result: ScrapeResult;
  cached: boolean;
  duration: number;
  source: string;
}

export interface ScraperPort {
  /**
   * Scrape a single URL
   */
  scrape(
    url: string,
    options?: Partial<ScrapeOptions> & {
      onProgress?: ProgressCallback;
      signal?: AbortSignal;
    },
  ): Promise<ScrapeResponse>;

  /**
   * Scrape multiple URLs in parallel
   */
  scrapeMany(
    urls: string[],
    options?: Partial<ScrapeOptions & BatchOptions> & {
      onProgress?: ProgressCallback;
      signal?: AbortSignal;
    },
  ): Promise<BatchScrapeResult>;

  /**
   * Check if scraper is available
   */
  isAvailable(): Promise<boolean>;

  /**
   * Get adapter name
   */
  getName(): string;

  /**
   * Clear cache
   */
  clearCache?(): void;
}

// =============================================================================
// Search Port - Web search
// =============================================================================

export interface SearchQuery {
  query: string;
  engine?: "google" | "bing" | "duckduckgo";
  type?: "web" | "image" | "video" | "news";
}

export interface SearchPort {
  /**
   * Search the web
   */
  search(
    query: string,
    options?: Partial<SearchOptions> & {
      onProgress?: ProgressCallback;
      signal?: AbortSignal;
    },
  ): Promise<SearchResults>;

  /**
   * Search multiple queries in parallel
   */
  searchParallel(
    queries: SearchQuery[],
    options?: Partial<BatchOptions> & {
      onProgress?: ProgressCallback;
      signal?: AbortSignal;
    },
  ): Promise<BatchSearchResult>;

  /**
   * Check if search is available
   */
  isAvailable(): Promise<boolean>;

  /**
   * Get adapter name
   */
  getName(): string;
}

// =============================================================================
// Content Extractor Port - Content extraction from HTML
// =============================================================================

export interface ContentExtractorPort {
  /**
   * Extract main text content from HTML
   */
  extractText(html: string): string;

  /**
   * Convert HTML to Markdown
   */
  extractMarkdown(html: string): string;

  /**
   * Extract media (images, videos, audio)
   */
  extractMedia(html: string, baseUrl: string): ExtractedMedia;

  /**
   * Extract links
   */
  extractLinks(html: string, baseUrl: string): Link[];

  /**
   * Extract metadata (OG tags, meta, JSON-LD)
   */
  extractMetadata(html: string): Metadata;

  /**
   * Extract structured data (JSON-LD, microdata)
   */
  extractStructuredData(html: string): Record<string, unknown>[];
}

// =============================================================================
// Cache Port - Result caching
// =============================================================================

export interface CachePort<T> {
  get(key: string): T | undefined;
  set(key: string, value: T, ttl?: number): void;
  has(key: string): boolean;
  delete(key: string): boolean;
  clear(): void;
  size(): number;
}

// =============================================================================
// Stealth Port - Anti-detection measures
// =============================================================================

export interface StealthConfig {
  userAgents: string[];
  viewports: Array<{ width: number; height: number }>;
  languages: string[];
  timezones: string[];
}

export interface StealthPort {
  /**
   * Get a random user agent
   */
  getUserAgent(): string;

  /**
   * Get a random viewport
   */
  getViewport(): { width: number; height: number };

  /**
   * Apply stealth patches to browser context
   */
  applyPatches(context: BrowserContext): Promise<void>;

  /**
   * Get random delay for human-like behavior
   */
  getRandomDelay(min?: number, max?: number): number;
}

// =============================================================================
// Storage Port - Platform-agnostic key-value storage
// =============================================================================

export type { StoragePort } from "./storage.port.js";
export type { LoginPort, LoginOptions, InteractionData } from "./login.port.js";
export {
  SERVICE_LOGIN_URLS,
  SERVICE_VERIFY_URLS,
  SERVICE_LOGGED_IN_SELECTORS,
} from "./login.port.js";
