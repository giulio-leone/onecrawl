/**
 * Native Exports Tests
 * Verifies that the native entry points export the correct types
 * and exclude Node.js-only adapters.
 */

import { describe, it, expect } from "vitest";

describe("Native Exports", () => {
  it("should export adapters from native index", async () => {
    // Verify the barrel exports compile correctly
    const adapters = await import("../../src/adapters/index.native.js");
    expect(adapters).toBeDefined();
    expect(adapters.FetchScraperAdapter).toBeDefined();
    // Should NOT have Playwright, CDP, or Undici (Node-only)
    expect((adapters as any).PlaywrightScraperAdapter).toBeUndefined();
    expect((adapters as any).UndiciScraperAdapter).toBeUndefined();
    expect((adapters as any).CdpScraperAdapter).toBeUndefined();
  });

  it("should export createOneCrawl from native index", async () => {
    const native = await import("../../src/index.native.js");
    expect(native.createOneCrawl).toBeDefined();
    expect(typeof native.createOneCrawl).toBe("function");
  });

  it("should create OneCrawl instance from native", async () => {
    const { createOneCrawl } = await import("../../src/index.native.js");
    const crawler = createOneCrawl();
    expect(crawler.scrape).toBeDefined();
    expect(crawler.scrapeMany).toBeDefined();
    expect(crawler.search).toBeDefined();
    expect(crawler.searchMany).toBeDefined();
    expect(crawler.getAvailableScrapers).toBeDefined();
  });

  it("should only list cross-platform scrapers", async () => {
    const { createOneCrawl } = await import("../../src/index.native.js");
    const crawler = createOneCrawl();
    const scrapers = await crawler.getAvailableScrapers();
    expect(scrapers).toContain("fetch");
    expect(scrapers).toContain("fetch-pool");
    expect(scrapers).not.toContain("playwright");
    expect(scrapers).not.toContain("undici");
  });
});
