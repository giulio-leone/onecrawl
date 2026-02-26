/**
 * Fetch Pool Adapter - native fetch() with connection pooling semantics.
 * Platform-agnostic alternative to UndiciScraperAdapter.
 */

export {
  FetchPoolScraperAdapter,
  createFetchPoolScraperAdapter,
  type FetchPoolOptions,
} from "./fetch-pool.adapter.js";
