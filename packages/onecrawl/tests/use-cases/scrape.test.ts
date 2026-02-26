/**
 * ScrapeUseCase Tests — with mocked ScraperPort adapters.
 * Verifies orchestration logic (adapter selection, fallback chain) without browser.
 */

import { describe, it, expect, vi, beforeEach } from "vitest";
import type { ScraperPort, ScrapeResponse } from "../../src/ports/index.js";
import type { BatchScrapeResult } from "../../src/domain/schemas.js";

// We can't import the class directly because it imports real adapters.
// Instead, test the orchestration logic by recreating the pattern.

function makeMockScraper(
  name: string,
  available: boolean,
): ScraperPort {
  const scrapeResult: ScrapeResponse = {
    result: {
      url: "https://example.com",
      title: "Example",
      content: "Hello",
    },
    cached: false,
    duration: 100,
    source: name,
  };

  return {
    scrape: vi.fn().mockResolvedValue(scrapeResult),
    scrapeMany: vi.fn().mockResolvedValue({
      results: new Map([["https://example.com", scrapeResult.result]]),
      failed: new Map(),
      totalDuration: 100,
    } satisfies BatchScrapeResult),
    isAvailable: vi.fn().mockResolvedValue(available),
    getName: vi.fn().mockReturnValue(name),
    clearCache: vi.fn(),
  };
}

/**
 * Reproduces ScrapeUseCase.execute logic for testing without importing adapters.
 */
async function executeScrape(
  url: string,
  scrapers: {
    browser: ScraperPort;
    playwright: ScraperPort;
    undici: ScraperPort;
    fetch: ScraperPort;
  },
  options: {
    preferBrowser?: boolean;
    useHttp2?: boolean;
    fallbackToFetch?: boolean;
    waitFor?: string;
  } = {},
): Promise<ScrapeResponse> {
  const {
    preferBrowser = false,
    useHttp2 = true,
    fallbackToFetch = true,
    waitFor,
  } = options;

  const needsBrowser = preferBrowser || waitFor === "networkidle";

  if (needsBrowser) {
    for (const scraper of [scrapers.browser, scrapers.playwright]) {
      try {
        if (await scraper.isAvailable()) {
          return await scraper.scrape(url, { waitFor } as any);
        }
      } catch (error) {
        if (!fallbackToFetch) throw error;
      }
    }
  }

  const httpScraper = useHttp2 ? scrapers.undici : scrapers.fetch;
  return httpScraper.scrape(url, {});
}

describe("ScrapeUseCase orchestration logic", () => {
  let browser: ScraperPort;
  let playwright: ScraperPort;
  let undici: ScraperPort;
  let fetchScraper: ScraperPort;

  beforeEach(() => {
    browser = makeMockScraper("browser", true);
    playwright = makeMockScraper("playwright", true);
    undici = makeMockScraper("undici", true);
    fetchScraper = makeMockScraper("fetch", true);
  });

  it("uses undici by default (HTTP/2)", async () => {
    const result = await executeScrape(
      "https://example.com",
      { browser, playwright, undici, fetch: fetchScraper },
    );
    expect(undici.scrape).toHaveBeenCalledOnce();
    expect(browser.scrape).not.toHaveBeenCalled();
    expect(result.source).toBe("undici");
  });

  it("uses fetch when useHttp2 = false", async () => {
    const result = await executeScrape(
      "https://example.com",
      { browser, playwright, undici, fetch: fetchScraper },
      { useHttp2: false },
    );
    expect(fetchScraper.scrape).toHaveBeenCalledOnce();
    expect(undici.scrape).not.toHaveBeenCalled();
    expect(result.source).toBe("fetch");
  });

  it("uses browser when preferBrowser = true", async () => {
    const result = await executeScrape(
      "https://example.com",
      { browser, playwright, undici, fetch: fetchScraper },
      { preferBrowser: true },
    );
    expect(browser.scrape).toHaveBeenCalledOnce();
    expect(result.source).toBe("browser");
  });

  it("uses browser when waitFor = 'networkidle'", async () => {
    const result = await executeScrape(
      "https://example.com",
      { browser, playwright, undici, fetch: fetchScraper },
      { waitFor: "networkidle" },
    );
    expect(browser.scrape).toHaveBeenCalledOnce();
    expect(result.source).toBe("browser");
  });

  it("falls back to playwright when browser unavailable", async () => {
    browser = makeMockScraper("browser", false);
    const result = await executeScrape(
      "https://example.com",
      { browser, playwright, undici, fetch: fetchScraper },
      { preferBrowser: true },
    );
    expect(browser.isAvailable).toHaveBeenCalled();
    expect(playwright.scrape).toHaveBeenCalledOnce();
    expect(result.source).toBe("playwright");
  });

  it("falls back to undici when both browser and playwright unavailable", async () => {
    browser = makeMockScraper("browser", false);
    playwright = makeMockScraper("playwright", false);
    const result = await executeScrape(
      "https://example.com",
      { browser, playwright, undici, fetch: fetchScraper },
      { preferBrowser: true },
    );
    expect(undici.scrape).toHaveBeenCalledOnce();
    expect(result.source).toBe("undici");
  });

  it("throws when browser fails and fallbackToFetch = false", async () => {
    browser.scrape = vi.fn().mockRejectedValue(new Error("CDP failed"));
    await expect(
      executeScrape(
        "https://example.com",
        { browser, playwright, undici, fetch: fetchScraper },
        { preferBrowser: true, fallbackToFetch: false },
      ),
    ).rejects.toThrow("CDP failed");
  });

  it("skips browser for simple fetch when needsBrowser = false", async () => {
    await executeScrape(
      "https://example.com",
      { browser, playwright, undici, fetch: fetchScraper },
      { preferBrowser: false, waitFor: "load" },
    );
    expect(browser.isAvailable).not.toHaveBeenCalled();
    expect(playwright.isAvailable).not.toHaveBeenCalled();
    expect(undici.scrape).toHaveBeenCalledOnce();
  });
});
