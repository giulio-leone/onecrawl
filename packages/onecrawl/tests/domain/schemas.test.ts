/**
 * Domain Schemas — comprehensive validation tests.
 * Tests that Zod schemas accept valid data and reject invalid data.
 */

import { describe, it, expect } from "vitest";
import {
  ExtractedImageSchema,
  ExtractedVideoSchema,
  ExtractedAudioSchema,
  ExtractedMediaSchema,
  LinkSchema,
  MetadataSchema,
  ScrapeResultSchema,
  SearchResultSchema,
  SearchResultsSchema,
  ViewportSchema,
  LaunchConfigSchema,
  ScrapeOptionsSchema,
  SearchOptionsSchema,
  BatchOptionsSchema,
  ProgressEventSchema,
} from "../../src/domain/schemas.js";

// ── Media Schemas ──────────────────────────────────────────────────────────

describe("ExtractedImageSchema", () => {
  it("accepts a valid image with all fields", () => {
    const parsed = ExtractedImageSchema.parse({
      src: "https://example.com/img.jpg",
      alt: "photo",
      title: "Photo title",
      width: 800,
      height: 600,
      score: 0.95,
    });
    expect(parsed.src).toBe("https://example.com/img.jpg");
    expect(parsed.score).toBe(0.95);
  });

  it("accepts minimal image (src only)", () => {
    const parsed = ExtractedImageSchema.parse({ src: "https://x.com/a.png" });
    expect(parsed.alt).toBeUndefined();
  });

  it("rejects missing src", () => {
    expect(() => ExtractedImageSchema.parse({})).toThrow();
  });

  it("rejects invalid URL for src", () => {
    expect(() => ExtractedImageSchema.parse({ src: "not-a-url" })).toThrow();
  });

  it("rejects non-string alt", () => {
    expect(() =>
      ExtractedImageSchema.parse({ src: "https://x.com/a.png", alt: 123 }),
    ).toThrow();
  });
});

describe("ExtractedVideoSchema", () => {
  it("accepts minimal video", () => {
    const v = ExtractedVideoSchema.parse({ src: "video.mp4" });
    expect(v.src).toBe("video.mp4");
  });

  it("accepts full video", () => {
    const v = ExtractedVideoSchema.parse({
      src: "https://yt.com/v",
      embedUrl: "https://yt.com/embed/v",
      provider: "youtube",
      title: "My video",
      description: "desc",
      duration: 120,
      thumbnail: "https://yt.com/thumb.jpg",
    });
    expect(v.provider).toBe("youtube");
    expect(v.duration).toBe(120);
  });

  it("rejects missing src", () => {
    expect(() => ExtractedVideoSchema.parse({})).toThrow();
  });
});

describe("ExtractedAudioSchema", () => {
  it("accepts minimal audio", () => {
    const a = ExtractedAudioSchema.parse({ src: "audio.mp3" });
    expect(a.src).toBe("audio.mp3");
  });

  it("accepts full audio", () => {
    const a = ExtractedAudioSchema.parse({
      src: "audio.mp3",
      title: "Song",
      duration: 300,
    });
    expect(a.title).toBe("Song");
  });
});

describe("ExtractedMediaSchema", () => {
  it("accepts empty object", () => {
    const m = ExtractedMediaSchema.parse({});
    expect(m.images).toBeUndefined();
    expect(m.videos).toBeUndefined();
    expect(m.audio).toBeUndefined();
  });

  it("accepts full media object", () => {
    const m = ExtractedMediaSchema.parse({
      images: [{ src: "https://x.com/a.png" }],
      videos: [{ src: "v.mp4" }],
      audio: [{ src: "a.mp3" }],
    });
    expect(m.images).toHaveLength(1);
    expect(m.videos).toHaveLength(1);
    expect(m.audio).toHaveLength(1);
  });

  it("rejects invalid image inside media", () => {
    expect(() =>
      ExtractedMediaSchema.parse({ images: [{ src: "not-url" }] }),
    ).toThrow();
  });
});

// ── Link & Metadata ────────────────────────────────────────────────────────

describe("LinkSchema", () => {
  it("accepts valid link", () => {
    const l = LinkSchema.parse({
      href: "https://example.com",
      text: "Click",
      isExternal: true,
    });
    expect(l.isExternal).toBe(true);
  });

  it("requires href and text", () => {
    expect(() => LinkSchema.parse({ href: "http://x.com" })).toThrow();
    expect(() => LinkSchema.parse({ text: "hi" })).toThrow();
  });

  it("accepts optional fields", () => {
    const l = LinkSchema.parse({
      href: "/page",
      text: "Page",
      title: "Link title",
      rel: "nofollow",
    });
    expect(l.title).toBe("Link title");
    expect(l.rel).toBe("nofollow");
  });
});

describe("MetadataSchema", () => {
  it("accepts empty object", () => {
    const m = MetadataSchema.parse({});
    expect(m.title).toBeUndefined();
  });

  it("accepts full metadata", () => {
    const m = MetadataSchema.parse({
      title: "Title",
      description: "Desc",
      keywords: ["a", "b"],
      author: "Author",
      publishedTime: "2024-01-01",
      modifiedTime: "2024-06-01",
      ogTitle: "OG",
      ogDescription: "OG Desc",
      ogImage: "https://img.com/og.jpg",
      ogType: "article",
      twitterCard: "summary_large_image",
      twitterTitle: "Twitter Title",
      twitterDescription: "Twitter Desc",
      twitterImage: "https://img.com/tw.jpg",
      canonical: "https://example.com/page",
      lang: "en",
      structuredData: [{ "@type": "Article" }],
    });
    expect(m.keywords).toEqual(["a", "b"]);
    expect(m.structuredData).toHaveLength(1);
  });

  it("rejects non-array keywords", () => {
    expect(() => MetadataSchema.parse({ keywords: "bad" })).toThrow();
  });
});

// ── Scrape Results ─────────────────────────────────────────────────────────

describe("ScrapeResultSchema", () => {
  const minimal = { url: "https://x.com", title: "T", content: "C" };

  it("accepts minimal result", () => {
    const r = ScrapeResultSchema.parse(minimal);
    expect(r.url).toBe("https://x.com");
  });

  it("accepts full result with nested media/metadata", () => {
    const r = ScrapeResultSchema.parse({
      ...minimal,
      markdown: "# T",
      html: "<h1>T</h1>",
      links: [{ href: "/a", text: "A" }],
      media: { images: [{ src: "https://x.com/a.png" }] },
      metadata: { title: "T" },
      statusCode: 200,
      contentType: "text/html",
      loadTime: 150,
    });
    expect(r.links).toHaveLength(1);
    expect(r.media?.images).toHaveLength(1);
    expect(r.loadTime).toBe(150);
  });

  it("rejects missing required fields", () => {
    expect(() => ScrapeResultSchema.parse({ url: "x" })).toThrow();
    expect(() => ScrapeResultSchema.parse({ url: "x", title: "T" })).toThrow();
  });
});

describe("SearchResultSchema", () => {
  it("accepts valid result", () => {
    const r = SearchResultSchema.parse({
      title: "Result",
      url: "https://example.com",
      snippet: "A snippet",
      position: 1,
    });
    expect(r.position).toBe(1);
  });

  it("rejects missing title or url", () => {
    expect(() => SearchResultSchema.parse({ url: "x" })).toThrow();
    expect(() => SearchResultSchema.parse({ title: "T" })).toThrow();
  });
});

describe("SearchResultsSchema", () => {
  it("accepts valid search results", () => {
    const r = SearchResultsSchema.parse({
      query: "test",
      results: [{ title: "A", url: "https://a.com" }],
      totalResults: 100,
      searchTime: 0.5,
    });
    expect(r.results).toHaveLength(1);
    expect(r.totalResults).toBe(100);
  });

  it("accepts empty results", () => {
    const r = SearchResultsSchema.parse({ query: "empty", results: [] });
    expect(r.results).toHaveLength(0);
  });

  it("rejects missing query", () => {
    expect(() => SearchResultsSchema.parse({ results: [] })).toThrow();
  });
});

// ── Configuration Schemas ──────────────────────────────────────────────────

describe("ViewportSchema", () => {
  it("uses defaults", () => {
    const v = ViewportSchema.parse({});
    expect(v.width).toBe(1280);
    expect(v.height).toBe(720);
  });

  it("overrides defaults", () => {
    const v = ViewportSchema.parse({ width: 1920, height: 1080 });
    expect(v.width).toBe(1920);
  });

  it("rejects non-numeric width", () => {
    expect(() => ViewportSchema.parse({ width: "wide" })).toThrow();
  });
});

describe("LaunchConfigSchema", () => {
  it("uses defaults", () => {
    const c = LaunchConfigSchema.parse({});
    expect(c.headless).toBe(true);
    expect(c.timeout).toBe(30000);
    expect(c.stealth).toBe(true);
  });

  it("accepts proxy config", () => {
    const c = LaunchConfigSchema.parse({
      proxy: { server: "http://proxy:8080", username: "u", password: "p" },
    });
    expect(c.proxy?.server).toBe("http://proxy:8080");
  });

  it("rejects proxy without server", () => {
    expect(() =>
      LaunchConfigSchema.parse({ proxy: { username: "u" } }),
    ).toThrow();
  });
});

describe("ScrapeOptionsSchema", () => {
  it("uses all defaults", () => {
    const o = ScrapeOptionsSchema.parse({});
    expect(o.timeout).toBe(30000);
    expect(o.waitFor).toBe("networkidle");
    expect(o.extractMedia).toBe(true);
    expect(o.extractLinks).toBe(true);
    expect(o.extractMetadata).toBe(true);
    expect(o.cache).toBe(true);
    expect(o.screenshot).toBe(false);
  });

  it("rejects invalid waitFor", () => {
    expect(() => ScrapeOptionsSchema.parse({ waitFor: "invalid" })).toThrow();
  });

  it("accepts all valid waitFor values", () => {
    for (const wf of ["load", "domcontentloaded", "networkidle"]) {
      const o = ScrapeOptionsSchema.parse({ waitFor: wf });
      expect(o.waitFor).toBe(wf);
    }
  });
});

describe("SearchOptionsSchema", () => {
  it("uses defaults", () => {
    const o = SearchOptionsSchema.parse({});
    expect(o.engine).toBe("duckduckgo");
    expect(o.type).toBe("web");
    expect(o.maxResults).toBe(10);
    expect(o.safeSearch).toBe(true);
  });

  it("accepts all engines", () => {
    for (const engine of ["google", "bing", "duckduckgo"]) {
      expect(SearchOptionsSchema.parse({ engine }).engine).toBe(engine);
    }
  });

  it("accepts all search types", () => {
    for (const type of ["web", "image", "video", "news"]) {
      expect(SearchOptionsSchema.parse({ type }).type).toBe(type);
    }
  });

  it("rejects invalid engine", () => {
    expect(() => SearchOptionsSchema.parse({ engine: "yahoo" })).toThrow();
  });
});

describe("BatchOptionsSchema", () => {
  it("uses defaults", () => {
    const o = BatchOptionsSchema.parse({});
    expect(o.concurrency).toBe(3);
    expect(o.retries).toBe(2);
    expect(o.retryDelay).toBe(1000);
    expect(o.timeout).toBe(60000);
  });

  it("overrides all fields", () => {
    const o = BatchOptionsSchema.parse({
      concurrency: 10,
      retries: 5,
      retryDelay: 2000,
      timeout: 120000,
    });
    expect(o.concurrency).toBe(10);
    expect(o.timeout).toBe(120000);
  });
});

// ── Progress Events ────────────────────────────────────────────────────────

describe("ProgressEventSchema", () => {
  it("accepts all phases", () => {
    for (const phase of [
      "starting",
      "navigating",
      "extracting",
      "complete",
      "error",
    ]) {
      const p = ProgressEventSchema.parse({ phase, message: "msg" });
      expect(p.phase).toBe(phase);
    }
  });

  it("accepts optional fields", () => {
    const p = ProgressEventSchema.parse({
      phase: "extracting",
      message: "50%",
      url: "https://x.com",
      progress: 5,
      total: 10,
    });
    expect(p.progress).toBe(5);
    expect(p.total).toBe(10);
  });

  it("rejects invalid phase", () => {
    expect(() =>
      ProgressEventSchema.parse({ phase: "unknown", message: "m" }),
    ).toThrow();
  });

  it("rejects missing message", () => {
    expect(() => ProgressEventSchema.parse({ phase: "starting" })).toThrow();
  });
});
