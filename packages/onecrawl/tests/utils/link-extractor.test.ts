/**
 * Link/Image/Video extractor tests.
 */

import { describe, it, expect } from "vitest";
import {
  extractLinks,
  extractImages,
  extractVideos,
  extractMedia,
} from "../../src/utils/link-extractor.js";

const base = "https://example.com";

describe("extractLinks", () => {
  it("extracts links with text", () => {
    const html = `<a href="https://example.com/page">Page</a>`;
    const links = extractLinks(html, base);
    expect(links).toHaveLength(1);
    expect(links[0]!.href).toBe("https://example.com/page");
    expect(links[0]!.text).toBe("Page");
    expect(links[0]!.isExternal).toBe(false);
  });

  it("marks external links", () => {
    const html = `<a href="https://other.com/x">Ext</a>`;
    const links = extractLinks(html, base);
    expect(links[0]!.isExternal).toBe(true);
  });

  it("resolves relative links", () => {
    const html = `<a href="/about">About</a>`;
    const links = extractLinks(html, base);
    expect(links[0]!.href).toBe("https://example.com/about");
  });

  it("skips fragment-only and javascript: links", () => {
    const html = `
      <a href="#">Top</a>
      <a href="javascript:void(0)">Click</a>
    `;
    const links = extractLinks(html, base);
    expect(links).toHaveLength(0);
  });

  it("strips HTML from link text", () => {
    const html = `<a href="/x"><strong>Bold</strong> text</a>`;
    const links = extractLinks(html, base);
    expect(links[0]!.text).toBe("Bold text");
  });

  it("truncates long link text to 200 chars", () => {
    const longText = "A".repeat(300);
    const html = `<a href="/x">${longText}</a>`;
    const links = extractLinks(html, base);
    expect(links[0]!.text.length).toBeLessThanOrEqual(200);
  });
});

describe("extractImages", () => {
  it("extracts images with attributes", () => {
    const html = `<img src="https://example.com/photo.jpg" alt="Photo" width="800" height="600"/>`;
    const images = extractImages(html, base);
    expect(images).toHaveLength(1);
    expect(images[0]!.alt).toBe("Photo");
    expect(images[0]!.width).toBe(800);
    expect(images[0]!.height).toBe(600);
  });

  it("resolves relative image src", () => {
    const html = `<img src="/images/logo.png" alt="Logo"/>`;
    const images = extractImages(html, base);
    expect(images[0]!.src).toBe("https://example.com/images/logo.png");
  });

  it("skips data URIs", () => {
    const html = `<img src="data:image/png;base64,abc" alt="Inline"/>`;
    const images = extractImages(html, base);
    expect(images).toHaveLength(0);
  });

  it("skips tracking pixels", () => {
    const html = `<img src="https://track.com/1x1.gif" alt=""/>`;
    const images = extractImages(html, base);
    expect(images).toHaveLength(0);
  });
});

describe("extractVideos", () => {
  it("extracts video tags", () => {
    const html = `<video src="https://example.com/video.mp4"></video>`;
    const videos = extractVideos(html, base);
    expect(videos).toHaveLength(1);
    expect(videos[0]!.src).toContain("video.mp4");
  });

  it("extracts YouTube embeds", () => {
    const html = `<iframe src="https://www.youtube.com/embed/abc123"></iframe>`;
    const videos = extractVideos(html, base);
    expect(videos).toHaveLength(1);
    expect(videos[0]!.provider).toBe("youtube");
    expect(videos[0]!.embedUrl).toContain("abc123");
  });

  it("extracts Vimeo embeds", () => {
    const html = `<iframe src="https://player.vimeo.com/video/12345"></iframe>`;
    const videos = extractVideos(html, base);
    expect(videos).toHaveLength(1);
    expect(videos[0]!.provider).toBe("vimeo");
  });
});

describe("extractMedia", () => {
  it("returns both images and videos", () => {
    const html = `
      <img src="https://example.com/a.jpg" alt="A"/>
      <video src="https://example.com/v.mp4"></video>
    `;
    const media = extractMedia(html, base);
    expect(media.images).toHaveLength(1);
    expect(media.videos).toHaveLength(1);
  });

  it("returns empty arrays for no media", () => {
    const media = extractMedia("<p>No media</p>", base);
    expect(media.images).toHaveLength(0);
    expect(media.videos).toHaveLength(0);
  });
});
