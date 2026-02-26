/**
 * Search Use Case
 * High-level API for web search.
 */

import type { SearchPort } from "../ports/index.js";
import type {
  SearchResults,
  SearchOptions,
  ProgressCallback,
} from "../domain/schemas.js";
import { PlaywrightScraperAdapter } from "../adapters/playwright/scraper.adapter.js";
import { FetchScraperAdapter } from "../adapters/fetch/scraper.adapter.js";
import { SearchAdapter } from "../adapters/search-engines/search.adapter.js";

export interface SearchUseCaseOptions extends Partial<SearchOptions> {
  /**
   * Use browser for search (required for some engines)
   */
  useBrowser?: boolean;

  /**
   * Progress callback
   */
  onProgress?: ProgressCallback;

  /**
   * Abort signal
   */
  signal?: AbortSignal;
}

/**
 * SearchUseCase - Web search with multi-engine support
 */
export class SearchUseCase {
  private browserSearch: SearchPort;
  private fetchSearch: SearchPort;

  constructor() {
    this.browserSearch = new SearchAdapter(new PlaywrightScraperAdapter());
    this.fetchSearch = new SearchAdapter(new FetchScraperAdapter());
  }

  /**
   * Search the web
   */
  async execute(
    query: string,
    options: SearchUseCaseOptions = {},
  ): Promise<SearchResults> {
    const {
      useBrowser = false,
      engine = "duckduckgo",
      ...searchOptions
    } = options;

    // DuckDuckGo HTML version works without JS
    const needsBrowser = useBrowser || engine === "google" || engine === "bing";

    if (needsBrowser) {
      return this.browserSearch.search(query, { ...searchOptions, engine });
    }

    return this.fetchSearch.search(query, { ...searchOptions, engine });
  }

  /**
   * Search multiple queries
   */
  async executeMany(
    queries: string[],
    options: SearchUseCaseOptions & { concurrency?: number } = {},
  ): Promise<SearchResults[]> {
    const {
      useBrowser = false,
      engine = "duckduckgo",
      concurrency = 2,
      onProgress,
      signal,
    } = options;

    const searchPort = useBrowser ? this.browserSearch : this.fetchSearch;

    const searchQueries = queries.map((query) => ({ query, engine }));
    const result = await searchPort.searchParallel(searchQueries, {
      concurrency,
      onProgress,
      signal,
    });

    return result.results;
  }
}

/**
 * Create a search use case
 */
export function createSearchUseCase(): SearchUseCase {
  return new SearchUseCase();
}
