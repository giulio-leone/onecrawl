/**
 * LRU cache with TTL and conditional request support (ETag/Last-Modified).
 * Uses watermark eviction: triggers at 90% capacity, evicts to 70%.
 */

/** Cached scrape result with validation headers. */
export interface CacheEntry<T> {
  data: T;
  timestamp: number;
  etag?: string;
  lastModified?: string;
  /** Per-entry TTL override in milliseconds (from Cache-Control max-age). */
  ttlOverride?: number;
}

/** Result of a consolidated cache lookup. */
export interface CacheLookup<T> {
  fresh: CacheEntry<T> | null;
  stale: CacheEntry<T> | null;
}

/**
 * LRU cache backed by a Map (insertion order = access order).
 * On get(), entries are moved to the end to reflect recent access.
 * Eviction uses Map iteration order (oldest-first) — O(k) where k = entries removed.
 */
export class LruCache<T> {
  private entries = new Map<string, CacheEntry<T>>();
  private maxSize: number;
  private ttl: number;

  constructor(maxSize = 500, ttl = 30 * 60 * 1000) {
    this.maxSize = maxSize;
    this.ttl = ttl;
  }

  /** Get a non-expired entry, or undefined. Bumps entry to most-recent. */
  get(key: string): CacheEntry<T> | undefined {
    const entry = this.entries.get(key);
    if (!entry) return undefined;
    const effectiveTtl = entry.ttlOverride ?? this.ttl;
    if (Date.now() - entry.timestamp > effectiveTtl) {
      this.entries.delete(key);
      return undefined;
    }
    // Re-insert to move to end (most-recently used)
    this.entries.delete(key);
    this.entries.set(key, entry);
    return entry;
  }

  /** Get entry even if expired (for conditional requests). */
  getStale(key: string): CacheEntry<T> | undefined {
    return this.entries.get(key);
  }

  /** Consolidated lookup: returns fresh and stale in one call. Bumps fresh entry. */
  lookup(key: string): CacheLookup<T> {
    const entry = this.entries.get(key);
    if (!entry) return { fresh: null, stale: null };
    const effectiveTtl = entry.ttlOverride ?? this.ttl;
    if (Date.now() - entry.timestamp > effectiveTtl) {
      return { fresh: null, stale: entry };
    }
    // Bump to most-recent on fresh hit
    this.entries.delete(key);
    this.entries.set(key, entry);
    return { fresh: entry, stale: null };
  }

  /** Store an entry, evicting via watermark if at capacity. */
  set(key: string, entry: CacheEntry<T>): void {
    // Delete first so re-setting an existing key doesn't inflate size
    this.entries.delete(key);
    if (this.entries.size >= Math.ceil(this.maxSize * 0.9)) {
      this.evict();
    }
    this.entries.set(key, entry);
  }

  /** Remove all entries. */
  clear(): void {
    this.entries.clear();
  }

  /** Evict oldest entries (by Map insertion order) until at 70% capacity. */
  private evict(): void {
    const target = Math.floor(this.maxSize * 0.7);
    const iter = this.entries.keys();
    while (this.entries.size > target) {
      const { value, done } = iter.next();
      if (done) break;
      this.entries.delete(value);
    }
  }
}
