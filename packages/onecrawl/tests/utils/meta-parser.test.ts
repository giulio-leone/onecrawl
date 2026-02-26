/**
 * Meta Parser tests.
 */

import { describe, it, expect } from "vitest";
import { extractMetadata } from "../../src/utils/meta-parser.js";

describe("extractMetadata", () => {
  it("extracts title", () => {
    const m = extractMetadata("<title>My Page</title>");
    expect(m.title).toBe("My Page");
  });

  it("extracts meta description", () => {
    const m = extractMetadata(
      '<meta name="description" content="Page desc"/>',
    );
    expect(m.description).toBe("Page desc");
  });

  it("extracts author", () => {
    const m = extractMetadata('<meta name="author" content="John"/>');
    expect(m.author).toBe("John");
  });

  it("extracts Open Graph tags", () => {
    const html = `
      <meta property="og:title" content="OG Title"/>
      <meta property="og:description" content="OG Desc"/>
      <meta property="og:image" content="https://img.com/og.jpg"/>
      <meta property="og:type" content="article"/>
    `;
    const m = extractMetadata(html);
    expect(m.ogTitle).toBe("OG Title");
    expect(m.ogDescription).toBe("OG Desc");
    expect(m.ogImage).toBe("https://img.com/og.jpg");
    expect(m.ogType).toBe("article");
  });

  it("extracts Twitter card tags", () => {
    const html = `
      <meta name="twitter:card" content="summary_large_image"/>
      <meta name="twitter:title" content="Twitter Title"/>
      <meta name="twitter:description" content="Twitter Desc"/>
      <meta name="twitter:image" content="https://img.com/tw.jpg"/>
    `;
    const m = extractMetadata(html);
    expect(m.twitterCard).toBe("summary_large_image");
    expect(m.twitterTitle).toBe("Twitter Title");
    expect(m.twitterImage).toBe("https://img.com/tw.jpg");
  });

  it("extracts canonical URL", () => {
    const m = extractMetadata(
      '<link rel="canonical" href="https://example.com/page"/>',
    );
    expect(m.canonical).toBe("https://example.com/page");
  });

  it("extracts language", () => {
    const m = extractMetadata('<html lang="en">');
    expect(m.lang).toBe("en");
  });

  it("extracts JSON-LD structured data", () => {
    const html = `
      <script type="application/ld+json">
        {"@type": "Article", "headline": "Test"}
      </script>
    `;
    const m = extractMetadata(html);
    expect(m.structuredData).toHaveLength(1);
    expect(m.structuredData![0]).toHaveProperty("headline", "Test");
  });

  it("handles invalid JSON-LD gracefully", () => {
    const html = `
      <script type="application/ld+json">
        {invalid json}
      </script>
    `;
    const m = extractMetadata(html);
    expect(m.structuredData).toBeUndefined();
  });

  it("handles content attribute before name attribute", () => {
    const m = extractMetadata(
      '<meta content="Alt order" name="description"/>',
    );
    expect(m.description).toBe("Alt order");
  });

  it("returns empty metadata for empty HTML", () => {
    const m = extractMetadata("");
    expect(m.title).toBeUndefined();
    expect(m.description).toBeUndefined();
  });
});
