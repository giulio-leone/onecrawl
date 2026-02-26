/**
 * Utility Functions Tests
 */

import { describe, it, expect } from "vitest";
import { buildSearchUrl } from "../src/utils/url-builder.js";
import {
  htmlToText,
  htmlToMarkdown,
  extractLinks,
  extractMedia,
  extractMetadata,
} from "../src/utils/content-parser.js";
import {
  getRandomUserAgent,
  getRandomViewport,
  getStealthScript,
  getRandomDelay,
} from "../src/utils/stealth.js";

describe("URL Builder", () => {
  describe("buildSearchUrl", () => {
    it("should build DuckDuckGo URL", () => {
      const url = buildSearchUrl("test query", "duckduckgo", "web");
      expect(url).toContain("duckduckgo.com");
      expect(url).toContain("test+query");
    });

    it("should build Google URL", () => {
      const url = buildSearchUrl("test query", "google", "web");
      expect(url).toContain("google.com");
      expect(url).toContain("test");
      expect(url).toContain("query");
    });

    it("should build Bing URL", () => {
      const url = buildSearchUrl("test query", "bing", "web");
      expect(url).toContain("bing.com");
      expect(url).toContain("test");
      expect(url).toContain("query");
    });

    it("should build image search URL", () => {
      const url = buildSearchUrl("cats", "google", "image");
      expect(url).toContain("tbm=isch");
    });

    it("should build video search URL", () => {
      const url = buildSearchUrl("videos", "google", "video");
      expect(url).toContain("tbm=vid");
    });
  });
});

describe("Content Parser", () => {
  describe("htmlToText", () => {
    it("should strip HTML tags", () => {
      const html = "<p>Hello <strong>World</strong></p>";
      const text = htmlToText(html);
      expect(text).toBe("Hello World");
    });

    it("should handle nested tags", () => {
      const html = "<div><p><span>Nested</span> content</p></div>";
      const text = htmlToText(html);
      expect(text).toBe("Nested content");
    });

    it("should handle special entities", () => {
      const html = "<p>&amp; &lt; &gt; &quot;</p>";
      const text = htmlToText(html);
      expect(text).toBe('& < > "');
    });

    it("should handle empty input", () => {
      const text = htmlToText("");
      expect(text).toBe("");
    });
  });

  describe("htmlToMarkdown", () => {
    it("should convert headings", () => {
      const html = "<h1>Title</h1><h2>Subtitle</h2>";
      const md = htmlToMarkdown(html);
      expect(md).toContain("# Title");
      expect(md).toContain("## Subtitle");
    });

    it("should convert paragraphs", () => {
      const html = "<p>First paragraph</p><p>Second paragraph</p>";
      const md = htmlToMarkdown(html);
      expect(md).toContain("First paragraph");
      expect(md).toContain("Second paragraph");
    });

    it("should convert links", () => {
      const html = '<a href="https://example.com">Click here</a>';
      const md = htmlToMarkdown(html);
      expect(md).toContain("[Click here](https://example.com)");
    });

    it("should convert bold and italic", () => {
      const html = "<p><strong>Bold</strong> and <em>italic</em></p>";
      const md = htmlToMarkdown(html);
      expect(md).toContain("**Bold**");
      expect(md).toContain("*italic*");
    });
  });

  describe("extractLinks", () => {
    it("should extract links from HTML", () => {
      const html = `
        <a href="https://example.com">Example</a>
        <a href="/page">Internal</a>
      `;
      const links = extractLinks(html, "https://test.com");
      expect(links.length).toBeGreaterThanOrEqual(2);
    });

    it("should mark external links", () => {
      const html = '<a href="https://external.com">External</a>';
      const links = extractLinks(html, "https://test.com");
      const external = links.find((l) => l.href.includes("external"));
      expect(external?.isExternal).toBe(true);
    });

    it("should resolve relative links", () => {
      const html = '<a href="/page">Page</a>';
      const links = extractLinks(html, "https://test.com");
      const resolved = links.find((l) => l.href.includes("page"));
      expect(resolved?.href).toBe("https://test.com/page");
    });
  });

  describe("extractMedia", () => {
    it("should extract images", () => {
      const html = `
        <img src="https://example.com/image.jpg" alt="Test image" />
        <img src="/local.png" alt="Local" />
      `;
      const media = extractMedia(html, "https://test.com");
      expect(media.images?.length).toBeGreaterThanOrEqual(2);
    });

    it("should extract videos", () => {
      const html = `
        <video src="https://example.com/video.mp4"></video>
        <iframe src="https://www.youtube.com/embed/abc123"></iframe>
      `;
      const media = extractMedia(html, "https://test.com");
      expect(media.videos?.length).toBeGreaterThanOrEqual(1);
    });
  });

  describe("extractMetadata", () => {
    it("should extract title", () => {
      const html = "<head><title>Page Title</title></head>";
      const meta = extractMetadata(html);
      expect(meta.title).toBe("Page Title");
    });

    it("should extract meta description", () => {
      const html =
        '<head><meta name="description" content="Page description"></head>';
      const meta = extractMetadata(html);
      expect(meta.description).toBe("Page description");
    });

    it("should extract Open Graph tags", () => {
      const html = `
        <head>
          <meta property="og:title" content="OG Title">
          <meta property="og:image" content="https://example.com/og.jpg">
        </head>
      `;
      const meta = extractMetadata(html);
      expect(meta.ogTitle).toBe("OG Title");
      expect(meta.ogImage).toBe("https://example.com/og.jpg");
    });

    it("should extract Twitter card tags", () => {
      const html = `
        <head>
          <meta name="twitter:card" content="summary">
          <meta name="twitter:title" content="Twitter Title">
        </head>
      `;
      const meta = extractMetadata(html);
      expect(meta.twitterCard).toBe("summary");
      expect(meta.twitterTitle).toBe("Twitter Title");
    });
  });
});

describe("Stealth Utilities", () => {
  describe("getRandomUserAgent", () => {
    it("should return a user agent string", () => {
      const ua = getRandomUserAgent();
      expect(typeof ua).toBe("string");
      expect(ua.length).toBeGreaterThan(10);
    });

    it("should return valid user agent", () => {
      const ua = getRandomUserAgent();
      expect(ua).toMatch(/Mozilla|Chrome|Safari|Firefox/);
    });
  });

  describe("getRandomViewport", () => {
    it("should return viewport dimensions", () => {
      const vp = getRandomViewport();
      expect(vp.width).toBeGreaterThan(0);
      expect(vp.height).toBeGreaterThan(0);
    });

    it("should return common screen sizes", () => {
      const vp = getRandomViewport();
      expect(vp.width).toBeGreaterThanOrEqual(1280);
      expect(vp.height).toBeGreaterThanOrEqual(720);
    });
  });

  describe("getStealthScript", () => {
    it("should return stealth script", () => {
      const script = getStealthScript();
      expect(typeof script).toBe("string");
      expect(script.length).toBeGreaterThan(100);
    });

    it("should contain webdriver bypass", () => {
      const script = getStealthScript();
      expect(script).toContain("webdriver");
    });
  });

  describe("getRandomDelay", () => {
    it("should return delay within range", () => {
      const delay = getRandomDelay(100, 200);
      expect(delay).toBeGreaterThanOrEqual(100);
      expect(delay).toBeLessThanOrEqual(200);
    });

    it("should use default range", () => {
      const delay = getRandomDelay();
      expect(delay).toBeGreaterThanOrEqual(500);
      expect(delay).toBeLessThanOrEqual(2000);
    });
  });
});
