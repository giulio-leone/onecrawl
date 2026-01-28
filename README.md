# OneCrawl

Native TypeScript web crawler and scraper. Zero Python dependencies.

## Features

- **Pure TypeScript** - No Python runtime required
- **HTTP/2 Support** - Fast connection pooling with undici
- **Playwright Integration** - Full JavaScript rendering support
- **Fetch Adapter** - Lightweight scraping for simple cases
- **Search Engine Support** - Google, Bing, DuckDuckGo parsers
- **Stealth Mode** - Anti-detection built-in
- **Hexagonal Architecture** - Easy to extend with custom adapters
- **Streaming Support** - Progress callbacks for real-time updates
- **CLI Interface** - Use from command line

## Installation

```bash
npm install onecrawl
# or
pnpm add onecrawl
```

For browser-based scraping, also install Playwright:

```bash
npm install playwright
npx playwright install chromium
```

## Quick Start

```typescript
import { createOneCrawl } from 'onecrawl';

// Create instance (uses HTTP/2 by default)
const crawler = createOneCrawl();

// Scrape a page
const result = await crawler.scrape('https://example.com');
console.log(result.result.content);

// Search the web
const searchResults = await crawler.search('TypeScript tutorial');
console.log(searchResults.results);

// Batch scraping (HTTP/2 connection pooling)
const results = await crawler.scrapeMany([
  'https://example.com/page1',
  'https://example.com/page2',
  'https://example.com/page3',
]);
```

## CLI Usage

```bash
# Scrape a URL
onecrawl scrape https://example.com

# Search the web
onecrawl search "TypeScript tutorial"

# With options
onecrawl scrape https://example.com --browser -o markdown
onecrawl search "query" -e google -n 20
```

## With Playwright (for JS-heavy sites)

```typescript
import { createOneCrawl, PlaywrightBrowserAdapter } from 'onecrawl';

// Create browser adapter
const browserAdapter = new PlaywrightBrowserAdapter();
await browserAdapter.launch({ headless: true });

// Create crawler with browser
const crawler = createOneCrawl(browserAdapter);

// Scrape with JavaScript rendering
const result = await crawler.scrape('https://spa-example.com', {
  preferBrowser: true,
  waitFor: 'networkidle'
});

// Clean up
await browserAdapter.closeAll();
```

## API Reference

### Scraping

```typescript
// Simple scrape
const response = await crawler.scrape(url, options);

// Batch scrape
const results = await crawler.scrapeMany(urls, {
  concurrency: 5,
  onProgress: (event) => console.log(event.phase)
});
```

### Search

```typescript
// Search with DuckDuckGo (no JS needed)
const results = await crawler.search('query', {
  engine: 'duckduckgo',
  maxResults: 10
});

// Search with Google (requires browser)
const results = await crawler.search('query', {
  engine: 'google',
  useBrowser: true
});
```

### Options

```typescript
interface ScrapeOptions {
  timeout?: number;              // Request timeout (default: 30000)
  waitFor?: 'load' | 'domcontentloaded' | 'networkidle';
  extractMedia?: boolean;        // Extract images/videos
  extractLinks?: boolean;        // Extract links
  extractMetadata?: boolean;     // Extract meta tags
  cache?: boolean;               // Use cache
}

interface SearchOptions {
  engine?: 'google' | 'bing' | 'duckduckgo';
  type?: 'web' | 'image' | 'video' | 'news';
  maxResults?: number;
  lang?: string;
  region?: string;
}
```

## Architecture

OneCrawl follows hexagonal architecture:

```
src/
├── domain/        # Types and schemas (Zod)
├── ports/         # Interface contracts
├── adapters/      # Implementations
│   ├── playwright/    # Browser-based scraping
│   ├── fetch/         # HTTP-only scraping
│   └── search-engines/# Google, Bing, DDG parsers
├── use-cases/     # High-level business logic
└── utils/         # Helpers (stealth, parsing)
```

### Creating Custom Adapters

```typescript
import { ScraperPort, ScrapeResponse } from 'onecrawl';

class MyCustomScraper implements ScraperPort {
  async scrape(url: string, options?: ScrapeOptions): Promise<ScrapeResponse> {
    // Your implementation
  }

  async scrapeMany(urls: string[], options?: BatchOptions) {
    // Your implementation
  }

  async isAvailable(): Promise<boolean> {
    return true;
  }

  getName(): string {
    return 'my-custom-scraper';
  }
}
```

## Stealth Mode

OneCrawl includes anti-detection features:

- User agent rotation
- Viewport randomization
- WebDriver detection bypass
- Human-like delays

```typescript
import { getRandomUserAgent, getRandomViewport, generateStealthScript } from 'onecrawl';

const userAgent = getRandomUserAgent();
const viewport = getRandomViewport();
const stealthScript = generateStealthScript();
```

## License

MIT
