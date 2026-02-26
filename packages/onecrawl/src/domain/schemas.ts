/**
 * OneCrawl Domain Schemas
 * Core types for web crawling and scraping operations.
 */

import { z } from "zod";

// =============================================================================
// Media Types
// =============================================================================

export const ExtractedImageSchema = z.object({
  src: z.string().url(),
  alt: z.string().optional(),
  title: z.string().optional(),
  width: z.number().optional(),
  height: z.number().optional(),
  score: z.number().optional(),
});

export const ExtractedVideoSchema = z.object({
  src: z.string(),
  embedUrl: z.string().optional(),
  provider: z.string().optional(),
  title: z.string().optional(),
  description: z.string().optional(),
  duration: z.number().optional(),
  thumbnail: z.string().optional(),
});

export const ExtractedAudioSchema = z.object({
  src: z.string(),
  title: z.string().optional(),
  duration: z.number().optional(),
});

export const ExtractedMediaSchema = z.object({
  images: z.array(ExtractedImageSchema).optional(),
  videos: z.array(ExtractedVideoSchema).optional(),
  audio: z.array(ExtractedAudioSchema).optional(),
});

export type ExtractedImage = z.infer<typeof ExtractedImageSchema>;
export type ExtractedVideo = z.infer<typeof ExtractedVideoSchema>;
export type ExtractedAudio = z.infer<typeof ExtractedAudioSchema>;
export type ExtractedMedia = z.infer<typeof ExtractedMediaSchema>;

// =============================================================================
// Link Types
// =============================================================================

export const LinkSchema = z.object({
  href: z.string(),
  text: z.string(),
  title: z.string().optional(),
  rel: z.string().optional(),
  isExternal: z.boolean().optional(),
});

export type Link = z.infer<typeof LinkSchema>;

// =============================================================================
// Metadata Types
// =============================================================================

export const MetadataSchema = z.object({
  title: z.string().optional(),
  description: z.string().optional(),
  keywords: z.array(z.string()).optional(),
  author: z.string().optional(),
  publishedTime: z.string().optional(),
  modifiedTime: z.string().optional(),
  ogTitle: z.string().optional(),
  ogDescription: z.string().optional(),
  ogImage: z.string().optional(),
  ogType: z.string().optional(),
  twitterCard: z.string().optional(),
  twitterTitle: z.string().optional(),
  twitterDescription: z.string().optional(),
  twitterImage: z.string().optional(),
  canonical: z.string().optional(),
  lang: z.string().optional(),
  structuredData: z.array(z.record(z.string(), z.unknown())).optional(),
});

export type Metadata = z.infer<typeof MetadataSchema>;

// =============================================================================
// Scrape Result Types
// =============================================================================

export const ScrapeResultSchema = z.object({
  url: z.string(),
  title: z.string(),
  content: z.string(),
  markdown: z.string().optional(),
  html: z.string().optional(),
  links: z.array(LinkSchema).optional(),
  media: ExtractedMediaSchema.optional(),
  metadata: MetadataSchema.optional(),
  statusCode: z.number().optional(),
  contentType: z.string().optional(),
  loadTime: z.number().optional(),
});

export type ScrapeResult = z.infer<typeof ScrapeResultSchema>;

// =============================================================================
// Search Result Types
// =============================================================================

export const SearchResultSchema = z.object({
  title: z.string(),
  url: z.string(),
  snippet: z.string().optional(),
  position: z.number().optional(),
  thumbnailUrl: z.string().optional(),
  displayUrl: z.string().optional(),
  date: z.string().optional(),
  source: z.string().optional(),
});

export const SearchResultsSchema = z.object({
  query: z.string(),
  results: z.array(SearchResultSchema),
  totalResults: z.number().optional(),
  searchTime: z.number().optional(),
  nextPageUrl: z.string().optional(),
});

export type SearchResult = z.infer<typeof SearchResultSchema>;
export type SearchResults = z.infer<typeof SearchResultsSchema>;

// =============================================================================
// Configuration Types
// =============================================================================

export const ViewportSchema = z.object({
  width: z.number().default(1280),
  height: z.number().default(720),
});

export const LaunchConfigSchema = z.object({
  headless: z.boolean().default(true),
  viewport: ViewportSchema.optional(),
  userAgent: z.string().optional(),
  proxy: z
    .object({
      server: z.string(),
      username: z.string().optional(),
      password: z.string().optional(),
    })
    .optional(),
  timeout: z.number().default(30000),
  stealth: z.boolean().default(true),
});

export const ScrapeOptionsSchema = z.object({
  timeout: z.number().default(30000),
  waitFor: z
    .enum(["load", "domcontentloaded", "networkidle"])
    .default("networkidle"),
  waitForSelector: z.string().optional(),
  extractMedia: z.boolean().default(true),
  extractLinks: z.boolean().default(true),
  extractMetadata: z.boolean().default(true),
  maxContentLength: z.number().optional(),
  jsCode: z.string().optional(),
  cache: z.boolean().default(true),
  screenshot: z.boolean().default(false),
});

export const SearchOptionsSchema = z.object({
  engine: z.enum(["google", "bing", "duckduckgo"]).default("duckduckgo"),
  type: z.enum(["web", "image", "video", "news"]).default("web"),
  maxResults: z.number().default(10),
  lang: z.string().optional(),
  region: z.string().optional(),
  safeSearch: z.boolean().default(true),
});

export const BatchOptionsSchema = z.object({
  concurrency: z.number().default(3),
  retries: z.number().default(2),
  retryDelay: z.number().default(1000),
  timeout: z.number().default(60000),
});

export type Viewport = z.infer<typeof ViewportSchema>;
export type LaunchConfig = z.infer<typeof LaunchConfigSchema>;
export type ScrapeOptions = z.infer<typeof ScrapeOptionsSchema>;
export type SearchOptions = z.infer<typeof SearchOptionsSchema>;
export type BatchOptions = z.infer<typeof BatchOptionsSchema>;

// =============================================================================
// Progress & Events
// =============================================================================

export const ProgressEventSchema = z.object({
  phase: z.enum(["starting", "navigating", "extracting", "complete", "error"]),
  message: z.string(),
  url: z.string().optional(),
  progress: z.number().optional(),
  total: z.number().optional(),
});

export type ProgressEvent = z.infer<typeof ProgressEventSchema>;

export type ProgressCallback = (event: ProgressEvent) => void;

// =============================================================================
// Batch Results
// =============================================================================

export interface BatchScrapeResult {
  results: Map<string, ScrapeResult>;
  failed: Map<string, Error>;
  totalDuration: number;
}

export interface BatchSearchResult {
  results: SearchResults[];
  failed: Map<string, Error>;
  totalDuration: number;
}
