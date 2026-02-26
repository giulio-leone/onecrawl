/**
 * Domain Schemas Tests
 */

import { describe, it, expect } from "vitest";
import {
  ScrapeResultSchema,
  SearchResultsSchema,
  ExtractedMediaSchema,
  MetadataSchema,
  LinkSchema,
  ScrapeOptionsSchema,
  SearchOptionsSchema,
  BatchOptionsSchema,
} from "../src/domain/schemas.js";

describe("Domain Schemas", () => {
  describe("ScrapeResultSchema", () => {
    it("should validate a minimal scrape result", () => {
      const result = {
        url: "https://example.com",
        title: "Example",
        content: "Hello World",
      };

      const parsed = ScrapeResultSchema.parse(result);
      expect(parsed.url).toBe("https://example.com");
      expect(parsed.title).toBe("Example");
      expect(parsed.content).toBe("Hello World");
    });

    it("should validate a full scrape result", () => {
      const result = {
        url: "https://example.com",
        title: "Example",
        content: "Hello World",
        markdown: "# Hello World",
        html: "<h1>Hello World</h1>",
        statusCode: 200,
        contentType: "text/html",
        loadTime: 150,
      };

      const parsed = ScrapeResultSchema.parse(result);
      expect(parsed.markdown).toBe("# Hello World");
      expect(parsed.statusCode).toBe(200);
    });

    it("should allow any string for URL", () => {
      const result = {
        url: "not-a-url",
        title: "Test",
        content: "Test",
      };

      const parsed = ScrapeResultSchema.parse(result);
      expect(parsed.url).toBe("not-a-url");
    });
  });

  describe("SearchResultsSchema", () => {
    it("should validate search results", () => {
      const results = {
        query: "test query",
        results: [
          {
            title: "Result 1",
            url: "https://example.com/1",
            snippet: "This is a snippet",
          },
          {
            title: "Result 2",
            url: "https://example.com/2",
          },
        ],
      };

      const parsed = SearchResultsSchema.parse(results);
      expect(parsed.query).toBe("test query");
      expect(parsed.results).toHaveLength(2);
      expect(parsed.results[0]?.snippet).toBe("This is a snippet");
    });

    it("should validate empty results", () => {
      const results = {
        query: "no results query",
        results: [],
      };

      const parsed = SearchResultsSchema.parse(results);
      expect(parsed.results).toHaveLength(0);
    });
  });

  describe("ExtractedMediaSchema", () => {
    it("should validate media with images", () => {
      const media = {
        images: [
          {
            src: "https://example.com/image.jpg",
            alt: "Test image",
            width: 800,
            height: 600,
          },
        ],
      };

      const parsed = ExtractedMediaSchema.parse(media);
      expect(parsed.images).toHaveLength(1);
      expect(parsed.images?.[0]?.alt).toBe("Test image");
    });

    it("should validate media with videos", () => {
      const media = {
        videos: [
          {
            src: "https://youtube.com/embed/abc123",
            provider: "youtube",
            title: "Test Video",
          },
        ],
      };

      const parsed = ExtractedMediaSchema.parse(media);
      expect(parsed.videos?.[0]?.provider).toBe("youtube");
    });
  });

  describe("MetadataSchema", () => {
    it("should validate page metadata", () => {
      const metadata = {
        title: "Page Title",
        description: "Page description",
        ogTitle: "OG Title",
        ogImage: "https://example.com/og.jpg",
        canonical: "https://example.com/page",
      };

      const parsed = MetadataSchema.parse(metadata);
      expect(parsed.ogTitle).toBe("OG Title");
    });
  });

  describe("LinkSchema", () => {
    it("should validate a link", () => {
      const link = {
        href: "https://example.com/page",
        text: "Link Text",
        isExternal: true,
      };

      const parsed = LinkSchema.parse(link);
      expect(parsed.isExternal).toBe(true);
    });
  });

  describe("ScrapeOptionsSchema", () => {
    it("should provide defaults", () => {
      const parsed = ScrapeOptionsSchema.parse({});
      expect(parsed.timeout).toBe(30000);
      expect(parsed.waitFor).toBe("networkidle");
      expect(parsed.extractMedia).toBe(true);
      expect(parsed.cache).toBe(true);
    });

    it("should override defaults", () => {
      const parsed = ScrapeOptionsSchema.parse({
        timeout: 60000,
        waitFor: "domcontentloaded",
      });
      expect(parsed.timeout).toBe(60000);
      expect(parsed.waitFor).toBe("domcontentloaded");
    });
  });

  describe("SearchOptionsSchema", () => {
    it("should provide defaults", () => {
      const parsed = SearchOptionsSchema.parse({});
      expect(parsed.engine).toBe("duckduckgo");
      expect(parsed.type).toBe("web");
      expect(parsed.maxResults).toBe(10);
    });
  });

  describe("BatchOptionsSchema", () => {
    it("should provide defaults", () => {
      const parsed = BatchOptionsSchema.parse({});
      expect(parsed.concurrency).toBe(3);
      expect(parsed.retries).toBe(2);
    });
  });
});
