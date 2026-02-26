/**
 * URL Builder - Generate search engine URLs
 */

export type SearchEngine = "google" | "bing" | "duckduckgo";
export type SearchType = "web" | "image" | "video" | "news";

interface UrlBuilderOptions {
  lang?: string;
  region?: string;
  safeSearch?: boolean;
  page?: number;
}

/**
 * Build search URL for a given engine and type
 */
export function buildSearchUrl(
  query: string,
  engine: SearchEngine,
  type: SearchType = "web",
  options: UrlBuilderOptions = {},
): string {
  const encoded = encodeURIComponent(query);
  const { lang, region, page = 1 } = options;

  switch (engine) {
    case "google":
      return buildGoogleUrl(encoded, type, { lang, region, page });
    case "bing":
      return buildBingUrl(encoded, type, { lang, region, page });
    case "duckduckgo":
    default:
      return buildDuckDuckGoUrl(encoded, type, { lang, region });
  }
}

function buildGoogleUrl(
  query: string,
  type: SearchType,
  options: UrlBuilderOptions,
): string {
  const base = "https://www.google.com";
  const params = new URLSearchParams({ q: decodeURIComponent(query) });

  if (options.lang) params.set("hl", options.lang);
  if (options.region) params.set("gl", options.region);
  if (options.page && options.page > 1) {
    params.set("start", String((options.page - 1) * 10));
  }

  switch (type) {
    case "image":
      params.set("tbm", "isch");
      return `${base}/search?${params}`;
    case "video":
      params.set("tbm", "vid");
      return `${base}/search?${params}`;
    case "news":
      params.set("tbm", "nws");
      return `${base}/search?${params}`;
    default:
      return `${base}/search?${params}`;
  }
}

function buildBingUrl(
  query: string,
  type: SearchType,
  options: UrlBuilderOptions,
): string {
  const params = new URLSearchParams({ q: decodeURIComponent(query) });

  if (options.lang) params.set("setlang", options.lang);
  if (options.page && options.page > 1) {
    params.set("first", String((options.page - 1) * 10 + 1));
  }

  switch (type) {
    case "image":
      return `https://www.bing.com/images/search?${params}`;
    case "video":
      return `https://www.bing.com/videos/search?${params}`;
    case "news":
      return `https://www.bing.com/news/search?${params}`;
    default:
      return `https://www.bing.com/search?${params}`;
  }
}

function buildDuckDuckGoUrl(
  query: string,
  type: SearchType,
  options: UrlBuilderOptions,
): string {
  const params = new URLSearchParams({ q: decodeURIComponent(query) });

  if (options.lang) params.set("kl", options.lang);

  // DuckDuckGo HTML version is lighter and more reliable
  switch (type) {
    case "image":
      params.set("iax", "images");
      params.set("ia", "images");
      return `https://duckduckgo.com/?${params}`;
    case "video":
      params.set("iax", "videos");
      params.set("ia", "videos");
      return `https://duckduckgo.com/?${params}`;
    case "news":
      params.set("iar", "news");
      params.set("ia", "news");
      return `https://duckduckgo.com/?${params}`;
    default:
      // Use HTML version for web search (lighter, faster)
      return `https://html.duckduckgo.com/html/?${params}`;
  }
}

/**
 * Normalize URL with base
 */
export function normalizeUrl(url: string, baseUrl: string): string {
  try {
    return new URL(url, baseUrl).href;
  } catch {
    return url;
  }
}

/**
 * Check if URL is absolute
 */
export function isAbsoluteUrl(url: string): boolean {
  return /^https?:\/\//i.test(url);
}

/**
 * Check if URL is same origin
 */
export function isSameOrigin(url: string, baseUrl: string): boolean {
  try {
    const urlObj = new URL(url, baseUrl);
    const baseObj = new URL(baseUrl);
    return urlObj.origin === baseObj.origin;
  } catch {
    return false;
  }
}

/**
 * Extract domain from URL
 */
export function extractDomain(url: string): string {
  try {
    return new URL(url).hostname;
  } catch {
    return "";
  }
}
