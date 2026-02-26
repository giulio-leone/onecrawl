/**
 * Search Use Case - React Native Compatible
 * Uses only fetch-based search (no Playwright).
 */

import type { SearchPort } from "../ports/index.js";
import type {
  SearchResults,
  SearchOptions,
  ProgressCallback,
} from "../domain/schemas.js";
import { FetchScraperAdapter } from "../adapters/fetch/scraper.adapter.js";
import { SearchAdapter } from "../adapters/search-engines/search.adapter.js";

export interface SearchUseCaseOptions extends Partial<SearchOptions> {
  /** Progress callback */
  onProgress?: ProgressCallback;

  /** Abort signal */
  signal?: AbortSignal;
}

/**
 * SearchUseCase - React Native search with fetch-only adapters.
 * DuckDuckGo HTML works without JS; Google/Bing may have limited results.
 */
export class SearchUseCase {
  private fetchSearch: SearchPort;

  constructor() {
    this.fetchSearch = new SearchAdapter(new FetchScraperAdapter());
  }

  /** Search the web using fetch-based adapter */
  async execute(
    query: string,
    options: SearchUseCaseOptions = {},
  ): Promise<SearchResults> {
    const { engine = "duckduckgo", ...searchOptions } = options;
    return this.fetchSearch.search(query, { ...searchOptions, engine });
  }

  /** Search multiple queries */
  async executeMany(
    queries: string[],
    options: SearchUseCaseOptions & { concurrency?: number } = {},
  ): Promise<SearchResults[]> {
    const {
      engine = "duckduckgo",
      concurrency = 2,
      onProgress,
      signal,
    } = options;

    const searchQueries = queries.map((query) => ({ query, engine }));
    const result = await this.fetchSearch.searchParallel(searchQueries, {
      concurrency,
      onProgress,
      signal,
    });

    return result.results;
  }
}

/** Create a search use case for React Native */
export function createSearchUseCase(): SearchUseCase {
  return new SearchUseCase();
}
