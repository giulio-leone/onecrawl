/**
 * FetchPoolScraperAdapter Tests
 * Tests the fetch pool adapter with mocked global fetch.
 */

import { describe, it, expect, vi, beforeEach } from "vitest";
import { FetchPoolScraperAdapter } from "../../src/adapters/fetch-pool/fetch-pool.adapter.js";

// Mock fetch globally for tests
const mockFetch = vi.fn();
vi.stubGlobal("fetch", mockFetch);

function createHtmlResponse(html: string, status = 200) {
  return new Response(html, {
    status,
    headers: { "content-type": "text/html" },
  });
}

describe("FetchPoolScraperAdapter", () => {
  let adapter: FetchPoolScraperAdapter;

  beforeEach(() => {
    adapter = new FetchPoolScraperAdapter({ maxConcurrencyPerOrigin: 2 });
    mockFetch.mockReset();
  });

  it("should return adapter name", () => {
    expect(adapter.getName()).toBe("fetch-pool");
  });

  it("should be always available", async () => {
    expect(await adapter.isAvailable()).toBe(true);
  });

  it("should scrape a URL using fetch", async () => {
    mockFetch.mockResolvedValueOnce(
      createHtmlResponse(
        "<html><head><title>Test</title></head><body>Hello World</body></html>",
      ),
    );

    const response = await adapter.scrape("https://example.com");
    expect(response.result.title).toBe("Test");
    expect(response.result.content).toContain("Hello World");
    expect(response.source).toBe("fetch-pool");
    expect(response.cached).toBe(false);
  });

  it("should cache results", async () => {
    mockFetch.mockResolvedValueOnce(
      createHtmlResponse(
        "<html><head><title>Cached</title></head><body>Content</body></html>",
      ),
    );

    // First call - should fetch
    await adapter.scrape("https://example.com/cached");
    expect(mockFetch).toHaveBeenCalledTimes(1);

    // Second call - should use cache
    const response = await adapter.scrape("https://example.com/cached");
    expect(response.cached).toBe(true);
    expect(mockFetch).toHaveBeenCalledTimes(1); // No additional fetch
  });

  it("should deduplicate concurrent requests for same URL", async () => {
    mockFetch.mockImplementation(
      () =>
        new Promise((resolve) =>
          setTimeout(
            () =>
              resolve(
                createHtmlResponse(
                  "<html><title>Dedup</title><body>OK</body></html>",
                ),
              ),
            50,
          ),
        ),
    );

    // Fire two requests simultaneously
    const [r1, r2] = await Promise.all([
      adapter.scrape("https://example.com/dedup"),
      adapter.scrape("https://example.com/dedup"),
    ]);

    expect(mockFetch).toHaveBeenCalledTimes(1);
    expect(r1.result.title).toBe("Dedup");
    expect(r2.result.title).toBe("Dedup");
  });

  it("should handle HTTP errors", async () => {
    mockFetch.mockResolvedValueOnce(new Response("Not Found", { status: 404 }));

    await expect(adapter.scrape("https://example.com/missing")).rejects.toThrow(
      "HTTP 404",
    );
  });

  it("should respect abort signal", async () => {
    const controller = new AbortController();
    controller.abort();

    await expect(
      adapter.scrape("https://example.com", { signal: controller.signal }),
    ).rejects.toThrow();
  });

  it("should clear cache", async () => {
    mockFetch.mockImplementation(() =>
      Promise.resolve(
        createHtmlResponse("<html><title>X</title><body>Y</body></html>"),
      ),
    );

    await adapter.scrape("https://example.com/clearme");
    adapter.clearCache();

    await adapter.scrape("https://example.com/clearme");
    expect(mockFetch).toHaveBeenCalledTimes(2);
  });

  it("should scrape many URLs", async () => {
    mockFetch.mockImplementation(() =>
      Promise.resolve(
        createHtmlResponse(
          "<html><title>Batch</title><body>Content</body></html>",
        ),
      ),
    );

    const result = await adapter.scrapeMany([
      "https://example.com/1",
      "https://example.com/2",
      "https://example.com/3",
    ]);

    expect(result.results.size).toBe(3);
    expect(result.failed.size).toBe(0);
  });

  it("should limit concurrency per origin", async () => {
    let activeCount = 0;
    let maxActive = 0;

    mockFetch.mockImplementation(async () => {
      activeCount++;
      maxActive = Math.max(maxActive, activeCount);
      await new Promise((r) => setTimeout(r, 50));
      activeCount--;
      return createHtmlResponse(
        "<html><title>Pool</title><body>OK</body></html>",
      );
    });

    await adapter.scrapeMany(
      [
        "https://example.com/a",
        "https://example.com/b",
        "https://example.com/c",
        "https://example.com/d",
      ],
      { concurrency: 2 },
    );

    // maxConcurrencyPerOrigin is 2, so max active should be <= 2
    expect(maxActive).toBeLessThanOrEqual(2);
  });
});
