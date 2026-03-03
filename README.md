# OneCrawl

Native TypeScript web crawler and scraper. Zero Python dependencies.

> **Monorepo** — this repository contains four packages:

| Package | Description | Version |
|---------|-------------|---------|
| [`@giulio-leone/onecrawl`](./packages/onecrawl) | Core crawler library | 2.1.0 |
| [`@giulio-leone/onecrawl-cli`](./packages/onecrawl-cli) | Stealth CLI with AI-first commands | 0.8.0 |
| [`@giulio-leone/onecrawl-client`](./packages/onecrawl-client) | HTTP client for OneCrawl server (zero-dep) | 1.0.1 |
| [`@giulio-leone/onecrawl-mcp`](./packages/onecrawl-mcp) | MCP server for AI agent integration | 1.1.0 |

## Repository Structure

```
packages/
├── onecrawl/          # Core library — crawl, scrape, search, passkey auth
├── onecrawl-cli/      # Stealth CLI — AI-first commands + browser automation
├── onecrawl-client/   # HTTP client — typed fetch wrapper for OneCrawl server API
└── onecrawl-mcp/      # MCP server — Model Context Protocol tools for AI agents
```

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
- **Passkey Auth** - WebAuthn/FIDO2 virtual authenticator for permanent sessions
- **Auth Cascade** - Passkey → Cookie → Manual fallback chain
- **AI-First CLI** - `find`, `get`, `assert`, `wait-for`, `scroll`, `screenshot --annotate`
- **MCP Integration** - 13 tools for AI agent orchestration

## What's New in v2.1.0

### Passkey Authentication (core)

Persistent login via WebAuthn/FIDO2 virtual authenticator over CDP. Credentials are encrypted (AES-256-GCM) and stored locally. Auth cascade: passkey → cookie → manual prompt.

```typescript
import { AuthCascade, PasskeyStore, WebAuthnManager } from '@giulio-leone/onecrawl';

const cascade = new AuthCascade({ storePath: '~/.onecrawl/linkedin' });
const result = await cascade.authenticate(browserContext, page);
// result.method: 'passkey' | 'cookie' | 'manual'
```

### AI-First CLI Commands (onecrawl-cli 0.7.0)

10 new commands for AI agent automation:

| Command | Description |
|---------|-------------|
| `scroll <dir> [px]` | Scroll up/down/left/right |
| `find <strategy> <query>` | Find elements by role, text, label, placeholder, testid |
| `get <prop> [ref]` | Get text, html, url, title, attributes |
| `is <state> <ref>` | Check visible, hidden, enabled, disabled, checked |
| `wait-for <target> [ms]` | Wait for selector, text, url, load, networkidle |
| `assert <condition>` | Assert visible, text, url, title, count → exit 0/1 |
| `screenshot --annotate` | Screenshot with numbered interactive elements |
| `session-info` | Browser version, viewport, URL, stealth status |
| `health-check` | Full diagnostic: browser, page, cookies, passkey |
| `auth <action>` | login, register-passkey, status, export, import |

## Installation

```bash
# Core library
pnpm add @giulio-leone/onecrawl

# HTTP client (for consuming OneCrawl server API)
pnpm add @giulio-leone/onecrawl-client

# Stealth CLI (optional, for AI agent browser automation)
pnpm add @giulio-leone/onecrawl-cli
```

For browser-based scraping, also install Playwright:

```bash
npm install playwright
npx playwright install chromium
```

## Development

```bash
pnpm install
pnpm run build
pnpm run test
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
