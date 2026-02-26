/**
 * Link, image, and video extraction from HTML
 */

import type { Link, ExtractedMedia } from "../domain/schemas.js";
import { normalizeUrl, isAbsoluteUrl } from "./url-builder.js";
import { htmlToText } from "./content-parser.js";

/** Extract links from HTML. */
export function extractLinks(html: string, baseUrl: string): Link[] {
  const links: Link[] = [];
  const linkRegex = /<a[^>]+href="([^"]*)"[^>]*>(.*?)<\/a>/gi;
  let match;

  while ((match = linkRegex.exec(html)) !== null) {
    const href = match[1] || "";
    const text = htmlToText(match[2] || "");

    if (!href || href.startsWith("#") || href.startsWith("javascript:")) {
      continue;
    }

    const normalizedUrl = normalizeUrl(href, baseUrl);
    const isExternal = !normalizedUrl.startsWith(new URL(baseUrl).origin);

    links.push({
      href: normalizedUrl,
      text: text.slice(0, 200),
      isExternal,
    });
  }

  return links;
}

/** Extract images from HTML. */
export function extractImages(
  html: string,
  baseUrl: string,
): NonNullable<ExtractedMedia["images"]> {
  const images: NonNullable<ExtractedMedia["images"]> = [];
  const imgRegex = /<img[^>]+>/gi;
  let match;

  while ((match = imgRegex.exec(html)) !== null) {
    const tag = match[0];
    const srcMatch = tag.match(/src="([^"]*)"/i);
    const altMatch = tag.match(/alt="([^"]*)"/i);
    const titleMatch = tag.match(/title="([^"]*)"/i);
    const widthMatch = tag.match(/width="(\d+)"/i);
    const heightMatch = tag.match(/height="(\d+)"/i);

    if (!srcMatch?.[1]) continue;

    const src = srcMatch[1];

    // Skip data URIs and tracking pixels
    if (
      src.startsWith("data:") ||
      src.includes("1x1") ||
      src.includes("pixel")
    ) {
      continue;
    }

    images.push({
      src: isAbsoluteUrl(src) ? src : normalizeUrl(src, baseUrl),
      alt: altMatch?.[1],
      title: titleMatch?.[1],
      width: widthMatch ? parseInt(widthMatch[1]!, 10) : undefined,
      height: heightMatch ? parseInt(heightMatch[1]!, 10) : undefined,
    });
  }

  return images;
}

/** Extract videos from HTML. */
export function extractVideos(
  html: string,
  baseUrl: string,
): NonNullable<ExtractedMedia["videos"]> {
  const videos: NonNullable<ExtractedMedia["videos"]> = [];
  let match;

  // Video tags
  const videoRegex = /<video[^>]*>[\s\S]*?<\/video>/gi;
  while ((match = videoRegex.exec(html)) !== null) {
    const tag = match[0];
    const srcMatch =
      tag.match(/src="([^"]*)"/i) || tag.match(/<source[^>]+src="([^"]*)"/i);
    if (srcMatch?.[1]) {
      videos.push({ src: normalizeUrl(srcMatch[1], baseUrl) });
    }
  }

  // YouTube embeds
  const ytRegex =
    /(?:youtube\.com\/embed\/|youtube\.com\/watch\?v=|youtu\.be\/)([a-zA-Z0-9_-]+)/gi;
  while ((match = ytRegex.exec(html)) !== null) {
    videos.push({
      src: `https://www.youtube.com/watch?v=${match[1]}`,
      embedUrl: `https://www.youtube.com/embed/${match[1]}`,
      provider: "youtube",
    });
  }

  // Vimeo embeds
  const vimeoRegex = /vimeo\.com\/(?:video\/)?(\d+)/gi;
  while ((match = vimeoRegex.exec(html)) !== null) {
    videos.push({
      src: `https://vimeo.com/${match[1]}`,
      embedUrl: `https://player.vimeo.com/video/${match[1]}`,
      provider: "vimeo",
    });
  }

  return videos;
}

/** Extract all media (images, videos) from HTML. */
export function extractMedia(html: string, baseUrl: string): ExtractedMedia {
  return {
    images: extractImages(html, baseUrl),
    videos: extractVideos(html, baseUrl),
  };
}
