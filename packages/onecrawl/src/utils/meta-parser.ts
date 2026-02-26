/**
 * Metadata extraction from HTML
 */

import type { Metadata } from "../domain/schemas.js";
import { htmlToText } from "./content-parser.js";

/** Extract a meta tag value by name or property. */
function getMeta(html: string, name: string): string | undefined {
  const regex = new RegExp(
    `<meta[^>]+(?:name|property)=["']${name}["'][^>]+content=["']([^"']*)["']`,
    "i",
  );
  const altRegex = new RegExp(
    `<meta[^>]+content=["']([^"']*)["'][^>]+(?:name|property)=["']${name}["']`,
    "i",
  );
  const match = html.match(regex) || html.match(altRegex);
  return match?.[1];
}

/** Extract metadata (OG tags, meta, JSON-LD) from HTML. */
export function extractMetadata(html: string): Metadata {
  const metadata: Metadata = {};

  // Title
  const titleMatch = html.match(/<title[^>]*>(.*?)<\/title>/i);
  if (titleMatch) metadata.title = htmlToText(titleMatch[1] || "");

  // Standard meta
  metadata.description = getMeta(html, "description");
  metadata.author = getMeta(html, "author");

  // Open Graph
  metadata.ogTitle = getMeta(html, "og:title");
  metadata.ogDescription = getMeta(html, "og:description");
  metadata.ogImage = getMeta(html, "og:image");
  metadata.ogType = getMeta(html, "og:type");

  // Twitter
  metadata.twitterCard = getMeta(html, "twitter:card");
  metadata.twitterTitle = getMeta(html, "twitter:title");
  metadata.twitterDescription = getMeta(html, "twitter:description");
  metadata.twitterImage = getMeta(html, "twitter:image");

  // Canonical
  const canonicalMatch = html.match(
    /<link[^>]+rel=["']canonical["'][^>]+href=["']([^"']*)["']/i,
  );
  if (canonicalMatch) metadata.canonical = canonicalMatch[1];

  // Language
  const langMatch = html.match(/<html[^>]+lang=["']([^"']*)["']/i);
  if (langMatch) metadata.lang = langMatch[1];

  // JSON-LD
  const jsonLdMatches = html.matchAll(
    /<script[^>]+type=["']application\/ld\+json["'][^>]*>([\s\S]*?)<\/script>/gi,
  );
  const structuredData: Record<string, unknown>[] = [];
  for (const m of jsonLdMatches) {
    try {
      structuredData.push(JSON.parse(m[1] || "{}"));
    } catch {
      // Invalid JSON-LD
    }
  }
  if (structuredData.length > 0) {
    metadata.structuredData = structuredData;
  }

  return metadata;
}
