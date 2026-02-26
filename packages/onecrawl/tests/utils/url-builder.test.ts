/**
 * URL Builder utility — extended tests.
 */

import { describe, it, expect } from "vitest";
import {
  buildSearchUrl,
  normalizeUrl,
  isAbsoluteUrl,
  isSameOrigin,
  extractDomain,
} from "../../src/utils/url-builder.js";

describe("buildSearchUrl", () => {
  describe("Google", () => {
    it("builds web search URL", () => {
      const url = buildSearchUrl("test query", "google", "web");
      expect(url).toContain("google.com/search");
      expect(url).toContain("q=test+query");
    });

    it("builds image search URL", () => {
      const url = buildSearchUrl("cats", "google", "image");
      expect(url).toContain("tbm=isch");
    });

    it("builds video search URL", () => {
      const url = buildSearchUrl("videos", "google", "video");
      expect(url).toContain("tbm=vid");
    });

    it("builds news search URL", () => {
      const url = buildSearchUrl("news", "google", "news");
      expect(url).toContain("tbm=nws");
    });

    it("includes language param", () => {
      const url = buildSearchUrl("q", "google", "web", { lang: "it" });
      expect(url).toContain("hl=it");
    });

    it("includes region param", () => {
      const url = buildSearchUrl("q", "google", "web", { region: "US" });
      expect(url).toContain("gl=US");
    });

    it("includes page offset", () => {
      const url = buildSearchUrl("q", "google", "web", { page: 3 });
      expect(url).toContain("start=20");
    });

    it("does not include start for page 1", () => {
      const url = buildSearchUrl("q", "google", "web", { page: 1 });
      expect(url).not.toContain("start=");
    });
  });

  describe("Bing", () => {
    it("builds web search URL", () => {
      const url = buildSearchUrl("test", "bing", "web");
      expect(url).toContain("bing.com/search");
    });

    it("builds image search URL", () => {
      const url = buildSearchUrl("cats", "bing", "image");
      expect(url).toContain("bing.com/images/search");
    });

    it("builds video search URL", () => {
      const url = buildSearchUrl("video", "bing", "video");
      expect(url).toContain("bing.com/videos/search");
    });

    it("builds news search URL", () => {
      const url = buildSearchUrl("news", "bing", "news");
      expect(url).toContain("bing.com/news/search");
    });

    it("includes page offset", () => {
      const url = buildSearchUrl("q", "bing", "web", { page: 2 });
      expect(url).toContain("first=11");
    });
  });

  describe("DuckDuckGo", () => {
    it("uses HTML version for web search", () => {
      const url = buildSearchUrl("test", "duckduckgo", "web");
      expect(url).toContain("html.duckduckgo.com/html/");
    });

    it("builds image search URL", () => {
      const url = buildSearchUrl("cats", "duckduckgo", "image");
      expect(url).toContain("iax=images");
    });

    it("builds video search URL", () => {
      const url = buildSearchUrl("video", "duckduckgo", "video");
      expect(url).toContain("iax=videos");
    });

    it("builds news search URL", () => {
      const url = buildSearchUrl("news", "duckduckgo", "news");
      expect(url).toContain("iar=news");
    });

    it("includes language param", () => {
      const url = buildSearchUrl("q", "duckduckgo", "web", { lang: "de" });
      expect(url).toContain("kl=de");
    });
  });

  it("handles special characters in query", () => {
    const url = buildSearchUrl("hello world & foo=bar", "google", "web");
    expect(url).toContain("q=hello+world+%26+foo%3Dbar");
  });
});

describe("normalizeUrl", () => {
  it("resolves relative path", () => {
    expect(normalizeUrl("/page", "https://example.com")).toBe(
      "https://example.com/page",
    );
  });

  it("keeps absolute URL as-is", () => {
    expect(normalizeUrl("https://other.com/x", "https://example.com")).toBe(
      "https://other.com/x",
    );
  });

  it("returns original on invalid URL", () => {
    expect(normalizeUrl(":::bad", ":::bad")).toBe(":::bad");
  });

  it("resolves relative with path", () => {
    expect(normalizeUrl("sub/page", "https://example.com/dir/")).toBe(
      "https://example.com/dir/sub/page",
    );
  });
});

describe("isAbsoluteUrl", () => {
  it("returns true for http", () => {
    expect(isAbsoluteUrl("http://example.com")).toBe(true);
  });

  it("returns true for https", () => {
    expect(isAbsoluteUrl("https://example.com")).toBe(true);
  });

  it("returns false for relative", () => {
    expect(isAbsoluteUrl("/page")).toBe(false);
  });

  it("returns false for protocol-relative", () => {
    expect(isAbsoluteUrl("//example.com")).toBe(false);
  });
});

describe("isSameOrigin", () => {
  it("returns true for same origin", () => {
    expect(isSameOrigin("https://example.com/page", "https://example.com")).toBe(
      true,
    );
  });

  it("returns false for different origin", () => {
    expect(isSameOrigin("https://other.com", "https://example.com")).toBe(false);
  });

  it("resolves relative URLs against base", () => {
    expect(isSameOrigin("/page", "https://example.com")).toBe(true);
  });

  it("returns false for invalid URLs", () => {
    expect(isSameOrigin(":::bad", ":::bad")).toBe(false);
  });
});

describe("extractDomain", () => {
  it("extracts hostname", () => {
    expect(extractDomain("https://www.example.com/page")).toBe(
      "www.example.com",
    );
  });

  it("returns empty for invalid URL", () => {
    expect(extractDomain("not-a-url")).toBe("");
  });
});
