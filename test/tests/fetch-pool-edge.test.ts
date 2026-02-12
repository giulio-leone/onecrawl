/**
 * FetchPoolScraperAdapter – Edge-Case & Stress Tests
 *
 * Covers: origin queue behaviour, LRU cache semantics, request deduplication,
 * error handling / retries, and configuration overrides.
 */

import { describe, it, expect, vi, beforeEach, afterEach } from "vitest";
import { FetchPoolScraperAdapter } from "../../src/adapters/fetch-pool/fetch-pool.adapter.js";
import { OriginQueue } from "../../src/adapters/fetch-pool/origin-queue.js";
import { LruCache } from "../../src/adapters/fetch-pool/lru-cache.js";
import { buildHeaders } from "../../src/adapters/fetch-pool/request-helpers.js";

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

const mockFetch = vi.fn();
vi.stubGlobal("fetch", mockFetch);

function htmlResponse(
  body = "<html><title>T</title><body>B</body></html>",
  status = 200,
  headers: Record<string, string> = {},
): Response {
  return new Response(body, {
    status,
    headers: { "content-type": "text/html", ...headers },
  });
}

/** Returns a fetch mock that delays for `ms` and tracks active count. */
function delayedFetch(ms: number, tracker: { active: number; max: number }) {
  return async () => {
    tracker.active++;
    tracker.max = Math.max(tracker.max, tracker.active);
    await new Promise((r) => setTimeout(r, ms));
    tracker.active--;
    return htmlResponse();
  };
}

// ---------------------------------------------------------------------------
// 1. Origin Queue Behaviour
// ---------------------------------------------------------------------------

describe("Origin Queue – edge cases", () => {
  it("respects concurrency=1: requests to the same origin are serial", async () => {
    const queue = new OriginQueue(1);
    const order: number[] = [];

    const p1 = queue.enqueue("https://a.com", async () => {
      await new Promise((r) => setTimeout(r, 30));
      order.push(1);
      return 1;
    });
    const p2 = queue.enqueue("https://a.com", async () => {
      order.push(2);
      return 2;
    });

    await Promise.all([p1, p2]);
    expect(order).toEqual([1, 2]); // strictly serial
  });

  it("runs requests to different origins in parallel", async () => {
    const queue = new OriginQueue(1);
    const tracker = { active: 0, max: 0 };

    const make = (origin: string) =>
      queue.enqueue(origin, async () => {
        tracker.active++;
        tracker.max = Math.max(tracker.max, tracker.active);
        await new Promise((r) => setTimeout(r, 40));
        tracker.active--;
        return true;
      });

    await Promise.all([
      make("https://a.com"),
      make("https://b.com"),
      make("https://c.com"),
    ]);

    expect(tracker.max).toBe(3); // all three ran in parallel
  });

  it("drains queue completely and cleans up origin state", async () => {
    const queue = new OriginQueue(1);
    const results: number[] = [];

    const tasks = Array.from({ length: 5 }, (_, i) =>
      queue.enqueue("https://x.com", async () => {
        await new Promise((r) => setTimeout(r, 5));
        results.push(i);
        return i;
      }),
    );

    const resolved = await Promise.all(tasks);
    expect(resolved).toEqual([0, 1, 2, 3, 4]);
    expect(results).toEqual([0, 1, 2, 3, 4]);
  });

  it("adapter limits concurrent fetches per origin via queue", async () => {
    const adapter = new FetchPoolScraperAdapter({
      maxConcurrencyPerOrigin: 1,
    });
    const tracker = { active: 0, max: 0 };

    mockFetch.mockImplementation(delayedFetch(30, tracker));

    await Promise.all([
      adapter.scrape("https://same.com/1"),
      adapter.scrape("https://same.com/2"),
      adapter.scrape("https://same.com/3"),
    ]);

    expect(tracker.max).toBe(1);
  });
});

// ---------------------------------------------------------------------------
// 2. LRU Cache Behaviour
// ---------------------------------------------------------------------------

describe("LRU Cache – edge cases", () => {
  it("evicts oldest entries when maxSize is reached", () => {
    const cache = new LruCache<string>(3, 60_000);
    const now = Date.now();

    cache.set("a", { data: "A", timestamp: now - 3000 });
    cache.set("b", { data: "B", timestamp: now - 2000 });
    cache.set("c", { data: "C", timestamp: now - 1000 });
    // cache is full (3); inserting another triggers eviction of oldest 10% (= 1)
    cache.set("d", { data: "D", timestamp: now });

    expect(cache.get("a")).toBeUndefined(); // evicted (oldest timestamp)
    expect(cache.get("b")?.data).toBe("B");
    expect(cache.get("d")?.data).toBe("D");
  });

  it("expired entries are evicted on get()", () => {
    const cache = new LruCache<string>(10, 50); // 50 ms TTL

    cache.set("k", { data: "V", timestamp: Date.now() - 100 }); // already expired
    expect(cache.get("k")).toBeUndefined();
  });

  it("getStale() returns expired entries", () => {
    const cache = new LruCache<string>(10, 50);

    cache.set("k", { data: "stale", timestamp: Date.now() - 100 });
    const entry = cache.getStale("k");
    expect(entry?.data).toBe("stale");
  });

  it("buildHeaders includes conditional headers from stale entry", () => {
    const staleEntry = {
      data: {} as never,
      timestamp: Date.now() - 100_000,
      etag: '"abc123"',
      lastModified: "Wed, 01 Jan 2025 00:00:00 GMT",
    };

    const headers = buildHeaders(staleEntry);
    expect(headers["If-None-Match"]).toBe('"abc123"');
    expect(headers["If-Modified-Since"]).toBe("Wed, 01 Jan 2025 00:00:00 GMT");
  });

  it("buildHeaders omits conditional headers when no stale entry", () => {
    const headers = buildHeaders(undefined);
    expect(headers["If-None-Match"]).toBeUndefined();
    expect(headers["If-Modified-Since"]).toBeUndefined();
  });

  it("custom maxCacheSize limits stored entries in adapter", async () => {
    const adapter = new FetchPoolScraperAdapter({ maxCacheSize: 2 });

    mockFetch.mockImplementation(() => Promise.resolve(htmlResponse()));

    await adapter.scrape("https://c.com/1");
    await adapter.scrape("https://c.com/2");
    await adapter.scrape("https://c.com/3");

    mockFetch.mockClear();

    // /1 should have been evicted, so fetching it hits network again
    await adapter.scrape("https://c.com/1");
    expect(mockFetch).toHaveBeenCalledTimes(1);

    // /3 should still be cached
    await adapter.scrape("https://c.com/3");
    expect(mockFetch).toHaveBeenCalledTimes(1); // no additional fetch
  });
});

// ---------------------------------------------------------------------------
// 3. Request Deduplication
// ---------------------------------------------------------------------------

describe("Request deduplication – edge cases", () => {
  beforeEach(() => mockFetch.mockReset());

  it("three concurrent identical requests result in a single fetch", async () => {
    mockFetch.mockImplementation(
      () =>
        new Promise((resolve) => setTimeout(() => resolve(htmlResponse()), 40)),
    );

    const adapter = new FetchPoolScraperAdapter();
    const results = await Promise.all([
      adapter.scrape("https://d.com/dup"),
      adapter.scrape("https://d.com/dup"),
      adapter.scrape("https://d.com/dup"),
    ]);

    expect(mockFetch).toHaveBeenCalledTimes(1);
    results.forEach((r) => expect(r.result.title).toBe("T"));
  });

  it("different URLs are NOT deduplicated", async () => {
    mockFetch.mockImplementation(
      () =>
        new Promise((resolve) => setTimeout(() => resolve(htmlResponse()), 20)),
    );

    const adapter = new FetchPoolScraperAdapter();
    await Promise.all([
      adapter.scrape("https://d.com/a"),
      adapter.scrape("https://d.com/b"),
    ]);

    expect(mockFetch).toHaveBeenCalledTimes(2);
  });

  it("after dedup promise resolves, a new request fires a new fetch", async () => {
    mockFetch.mockImplementation(() => Promise.resolve(htmlResponse()));

    const adapter = new FetchPoolScraperAdapter();
    adapter.clearCache(); // prevent cache hit

    await adapter.scrape("https://d.com/seq");
    adapter.clearCache();
    await adapter.scrape("https://d.com/seq");

    expect(mockFetch).toHaveBeenCalledTimes(2);
  });
});

// ---------------------------------------------------------------------------
// 4. Error Handling
// ---------------------------------------------------------------------------

describe("Error handling – edge cases", () => {
  beforeEach(() => mockFetch.mockReset());

  it("network error is caught and surfaced", async () => {
    mockFetch.mockRejectedValueOnce(new TypeError("Failed to fetch"));

    const adapter = new FetchPoolScraperAdapter();
    await expect(adapter.scrape("https://err.com/net")).rejects.toThrow(
      "Failed to fetch",
    );
  });

  it("timeout fires when fetch takes too long", async () => {
    mockFetch.mockImplementation(
      (_url: string, init: { signal?: AbortSignal }) => {
        return new Promise((resolve, reject) => {
          const timer = setTimeout(() => resolve(htmlResponse()), 5000);
          if (init?.signal) {
            init.signal.addEventListener("abort", () => {
              clearTimeout(timer);
              reject(
                new DOMException("The operation was aborted.", "AbortError"),
              );
            });
          }
        });
      },
    );

    const adapter = new FetchPoolScraperAdapter();
    await expect(
      adapter.scrape("https://err.com/slow", { timeout: 50 }),
    ).rejects.toThrow(); // AbortError
  });

  it("scrapeMany retries on failure then succeeds", async () => {
    let calls = 0;
    mockFetch.mockImplementation(() => {
      calls++;
      if (calls <= 2) return Promise.reject(new Error("transient"));
      return Promise.resolve(htmlResponse());
    });

    const adapter = new FetchPoolScraperAdapter();
    const batch = await adapter.scrapeMany(["https://err.com/retry"], {
      retries: 3,
      retryDelay: 10,
    });

    expect(batch.results.size).toBe(1);
    expect(batch.failed.size).toBe(0);
  });

  it("scrapeMany records failure after exhausting retries", async () => {
    let callCount = 0;
    mockFetch.mockImplementation(() => {
      callCount++;
      return new Response("Server Error", { status: 500 });
    });

    const adapter = new FetchPoolScraperAdapter();
    const batch = await adapter.scrapeMany(["https://err.com/fail"], {
      retries: 1,
      retryDelay: 5,
    });

    expect(batch.results.size).toBe(0);
    expect(batch.failed.size).toBe(1);
    expect(batch.failed.get("https://err.com/fail")?.message).toBe("HTTP 500");
    expect(callCount).toBe(2); // initial + 1 retry
  });

  it("HTTP 500 is treated as an error", async () => {
    mockFetch.mockResolvedValueOnce(
      new Response("Server Error", { status: 500 }),
    );

    const adapter = new FetchPoolScraperAdapter();
    await expect(adapter.scrape("https://err.com/500")).rejects.toThrow(
      "HTTP 500",
    );
  });

  it("abort signal cancels pending scrape before fetch", async () => {
    const adapter = new FetchPoolScraperAdapter();
    const controller = new AbortController();
    controller.abort();

    await expect(
      adapter.scrape("https://err.com/abort", { signal: controller.signal }),
    ).rejects.toThrow();
    expect(mockFetch).not.toHaveBeenCalled();
  });
});

// ---------------------------------------------------------------------------
// 5. Configuration
// ---------------------------------------------------------------------------

describe("Configuration – edge cases", () => {
  beforeEach(() => mockFetch.mockReset());

  it("cache: false bypasses cache entirely", async () => {
    mockFetch.mockImplementation(() => Promise.resolve(htmlResponse()));

    const adapter = new FetchPoolScraperAdapter();

    await adapter.scrape("https://cfg.com/nc", { cache: false });
    await adapter.scrape("https://cfg.com/nc", { cache: false });

    expect(mockFetch).toHaveBeenCalledTimes(2);
  });

  it("custom concurrency per origin = 3 allows 3 parallel requests", async () => {
    const adapter = new FetchPoolScraperAdapter({
      maxConcurrencyPerOrigin: 3,
    });
    const tracker = { active: 0, max: 0 };

    mockFetch.mockImplementation(delayedFetch(30, tracker));

    await Promise.all(
      Array.from({ length: 6 }, (_, i) =>
        adapter.scrape(`https://cfg.com/${i}`),
      ),
    );

    expect(tracker.max).toBeLessThanOrEqual(3);
  });

  it("onProgress callback fires expected phases", async () => {
    mockFetch.mockResolvedValueOnce(htmlResponse());

    const adapter = new FetchPoolScraperAdapter();
    const phases: string[] = [];

    await adapter.scrape("https://cfg.com/progress", {
      onProgress: (evt) => phases.push(evt.phase),
    });

    expect(phases).toContain("starting");
    expect(phases).toContain("extracting");
    expect(phases).toContain("complete");
  });

  it("clearCache makes the next request fetch from network", async () => {
    mockFetch.mockImplementation(() => Promise.resolve(htmlResponse()));

    const adapter = new FetchPoolScraperAdapter();
    await adapter.scrape("https://cfg.com/clear");
    adapter.clearCache();
    await adapter.scrape("https://cfg.com/clear");

    expect(mockFetch).toHaveBeenCalledTimes(2);
  });
});
