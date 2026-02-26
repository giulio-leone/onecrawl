/**
 * SemanticCrawlUseCase Tests — with mocked ScraperPort.
 * Verifies crawl orchestration: depth limiting, include/exclude, progress, cancellation.
 */

import { describe, it, expect, vi, beforeEach } from "vitest";
import { SemanticCrawlUseCase } from "../../src/use-cases/semantic-crawl.use-case.js";
import type { ScraperPort, ScrapeResponse } from "../../src/ports/index.js";
import type { CrawlTarget } from "../../src/domain/semantic-tool.js";

function makePageHtml(links: string[] = [], formName?: string): string {
  const linkHtml = links.map((l) => `<a href="${l}">Link</a>`).join("\n");
  const formHtml = formName
    ? `<form name="${formName}"><input name="q" type="text" placeholder="Search" required/></form>`
    : "";
  return `<html><body>${formHtml}${linkHtml}</body></html>`;
}

function makeMockScraper(
  pages: Record<string, string>,
): ScraperPort {
  return {
    scrape: vi.fn().mockImplementation(async (url: string): Promise<ScrapeResponse> => {
      const html = pages[url];
      if (!html) throw new Error(`Not found: ${url}`);
      return {
        result: { url, title: "Page", content: "text", html },
        cached: false,
        duration: 50,
        source: "mock",
      };
    }),
    scrapeMany: vi.fn().mockResolvedValue({
      results: new Map(),
      failed: new Map(),
      totalDuration: 0,
    }),
    isAvailable: vi.fn().mockResolvedValue(true),
    getName: vi.fn().mockReturnValue("mock"),
  };
}

describe("SemanticCrawlUseCase", () => {
  let scraper: ScraperPort;
  let useCase: SemanticCrawlUseCase;

  beforeEach(() => {
    scraper = makeMockScraper({
      "https://example.com/": makePageHtml(
        ["https://example.com/about", "https://example.com/docs"],
        "search",
      ),
      "https://example.com/about": makePageHtml([], "contact"),
      "https://example.com/docs": makePageHtml(["https://example.com/docs/api"]),
      "https://example.com/docs/api": makePageHtml(),
    });
    useCase = new SemanticCrawlUseCase(scraper);
  });

  it("crawls entry points and follows links", async () => {
    const target: CrawlTarget = {
      site: "example.com",
      entryPoints: ["https://example.com/"],
      maxPages: 50,
      maxDepth: 3,
    };

    const result = await useCase.execute(target);
    expect(result.site).toBe("example.com");
    expect(result.pagesScanned).toBeGreaterThanOrEqual(1);
    expect(result.errors).toHaveLength(0);
  });

  it("discovers tools from forms", async () => {
    const target: CrawlTarget = {
      site: "example.com",
      entryPoints: ["https://example.com/"],
      maxPages: 10,
      maxDepth: 2,
    };

    const result = await useCase.execute(target);
    expect(result.toolsDiscovered).toBeGreaterThanOrEqual(1);
    // The entry page has a form named "search"
    const entryTools = result.toolsByPage.get("https://example.com/");
    expect(entryTools).toBeDefined();
    expect(entryTools!.some((t) => t.name.includes("search"))).toBe(true);
  });

  it("respects maxPages limit", async () => {
    const target: CrawlTarget = {
      site: "example.com",
      entryPoints: ["https://example.com/"],
      maxPages: 2,
      maxDepth: 10,
    };

    const result = await useCase.execute(target);
    expect(result.pagesScanned).toBeLessThanOrEqual(2);
  });

  it("respects maxDepth limit", async () => {
    const target: CrawlTarget = {
      site: "example.com",
      entryPoints: ["https://example.com/"],
      maxPages: 50,
      maxDepth: 0,
    };

    const result = await useCase.execute(target);
    // maxDepth 0 means only entry points, no following links
    expect(result.pagesScanned).toBe(1);
    expect(scraper.scrape).toHaveBeenCalledTimes(1);
  });

  it("reports errors for failed pages", async () => {
    scraper = makeMockScraper({
      "https://example.com/": makePageHtml(["https://example.com/broken"]),
      // "broken" page is not in the map, so it throws
    });
    useCase = new SemanticCrawlUseCase(scraper);

    const target: CrawlTarget = {
      site: "example.com",
      entryPoints: ["https://example.com/"],
      maxPages: 10,
      maxDepth: 1,
    };

    const result = await useCase.execute(target);
    expect(result.errors.length).toBeGreaterThanOrEqual(1);
    expect(result.errors[0]).toContain("broken");
  });

  it("calls progress callback", async () => {
    const onProgress = vi.fn();
    const target: CrawlTarget = {
      site: "example.com",
      entryPoints: ["https://example.com/"],
      maxPages: 10,
      maxDepth: 1,
    };

    await useCase.execute(target, onProgress);
    expect(onProgress).toHaveBeenCalled();
    const lastCall = onProgress.mock.calls[onProgress.mock.calls.length - 1]![0];
    expect(lastCall.pagesScanned).toBeGreaterThanOrEqual(1);
  });

  it("can be cancelled", async () => {
    // Start crawl and immediately cancel
    const target: CrawlTarget = {
      site: "example.com",
      entryPoints: ["https://example.com/"],
      maxPages: 100,
      maxDepth: 10,
    };

    // Run execute but cancel quickly
    const promise = useCase.execute(target);
    useCase.cancel();
    const result = await promise;

    // Should complete early
    expect(result.pagesScanned).toBeLessThanOrEqual(2);
  });

  it("tracks running state", async () => {
    expect(useCase.isRunning()).toBe(false);

    const target: CrawlTarget = {
      site: "example.com",
      entryPoints: ["https://example.com/"],
      maxPages: 1,
      maxDepth: 0,
    };

    const promise = useCase.execute(target);
    // After execute completes:
    await promise;
    expect(useCase.isRunning()).toBe(false);
  });

  it("filters pages by includePatterns", async () => {
    const target: CrawlTarget = {
      site: "example.com",
      entryPoints: ["https://example.com/"],
      maxPages: 50,
      maxDepth: 3,
      includePatterns: ["**/docs/**"],
    };

    const result = await useCase.execute(target);
    // Entry point doesn't match include pattern, so it gets skipped
    // Only docs pages should be crawled
    for (const [url] of result.toolsByPage) {
      expect(url).toContain("docs");
    }
  });

  it("filters pages by excludePatterns", async () => {
    const target: CrawlTarget = {
      site: "example.com",
      entryPoints: ["https://example.com/"],
      maxPages: 50,
      maxDepth: 3,
      excludePatterns: ["**/about"],
    };

    const result = await useCase.execute(target);
    // About page should be excluded
    expect(result.toolsByPage.has("https://example.com/about")).toBe(false);
  });
});
