---
sidebar_position: 1
title: Getting Started
---

# Getting Started

## What is OneCrawl

OneCrawl is a high-performance browser automation engine written in Rust with native bindings for Node.js and Python. It provides **200+ CLI commands**, **51 MCP tools** for AI agent integration, and a full **HTTP API server** for multi-instance Chrome management. Built on top of the Chrome DevTools Protocol (CDP), OneCrawl delivers raw speed and reliability for scraping, testing, stealth automation, and accessibility auditing — all from a single ~5.8 MB binary.

## Why OneCrawl?

OneCrawl replaces an entire stack of browser automation tools with a single, lightweight binary. Here's how it compares:

| Feature | OneCrawl | Playwright | Puppeteer |
|---|---|---|---|
| **Binary size** | ~5.8 MB | ~250 MB | ~170 MB (with Chromium) |
| **Startup time** | &lt;100ms | ~2s | ~1.5s |
| **Memory (idle)** | ~15 MB | ~80 MB | ~60 MB |
| **Stealth built-in** | ✅ Native | ❌ Plugin needed | ❌ Plugin needed |
| **MCP support** | ✅ 51 tools | ❌ | ❌ |
| **HTTP API server** | ✅ Multi-instance | ❌ | ❌ |
| **Language** | Rust (native) | JS/TS | JS/TS |
| **CLI commands** | 200+ | ~10 | N/A |
| **Anti-bot bypass** | ✅ Built-in | ❌ | ❌ |
| **Accessibility refs** | ✅ Snapshot-based | Locators | Selectors |

## Features at a Glance

- 🚀 **Blazing fast** — Native Rust binary, sub-100ms startup
- 🕵️ **Stealth mode** — Fingerprint spoofing, anti-bot bypass, CAPTCHA detection
- 🤖 **AI-native** — 51 MCP tools for seamless AI agent integration
- 🌐 **HTTP API** — Multi-instance Chrome management with REST endpoints
- 📸 **Screenshots & PDF** — Full-page capture, element capture, visual diff
- 🕷️ **Spider & Crawl** — Site-wide crawling with configurable depth and concurrency
- ♿ **Accessibility** — Built-in a11y auditing and accessibility tree snapshots
- 🔑 **Auth & Passkeys** — Native passkey support for modern authentication flows
- 📊 **HAR & Network** — Full network logging, HAR export, request interception
- 🧩 **Multi-SDK** — Node.js and Python bindings alongside the CLI
- 📦 **Single binary** — No external dependencies, just Chrome on `$PATH`
- 🔄 **Pipelines** — Chain commands via YAML/JSON pipeline files

## Prerequisites

| Requirement | Minimum Version | Notes |
|---|---|---|
| **Rust** | 1.75+ | Required only for building from source |
| **Chrome / Chromium** | 120+ | Must be installed and accessible on `$PATH` |
| **Node.js** *(optional)* | 18+ | For the Node.js SDK |
| **Python** *(optional)* | 3.9+ | For the Python SDK |

## Installation

### CLI (from source)

```bash
# Clone the repository
git clone https://github.com/AstroLabs-AI/onecrawl.git
cd onecrawl

# Build and install the CLI binary
cargo install --path packages/onecrawl-rust/crates/onecrawl-cli-rs

# Verify the installation
onecrawl version
```

### Node.js SDK

```bash
npm install @onecrawl/native
```

### Python SDK

```bash
pip install onecrawl
```

## Quick Start

### CLI — Basic

```bash
# Launch a headed Chrome instance
onecrawl navigate "https://example.com"

# Take a full-page screenshot
onecrawl screenshot --full --output example.png

# Extract the visible text from the page
onecrawl get text

# Get the page title
onecrawl get title

# Close the browser
onecrawl close
```

### CLI — Scrape Product Data with Stealth

```bash
# Enable stealth to avoid detection
onecrawl stealth inject

# Navigate to a product page
onecrawl navigate "https://shop.example.com/product/123"

# Extract structured data
onecrawl structured "https://shop.example.com/product/123" \
  '{"name": "h1.product-title", "price": ".price-tag", "rating": ".star-rating"}'
```

### CLI — Crawl a Site and Screenshot Every Page

```bash
# Spider the site and save the URL list
onecrawl spider "https://docs.example.com" --depth 2 --output urls.json

# Take a screenshot of the homepage
onecrawl navigate "https://docs.example.com"
onecrawl screenshot --full --output homepage.png
```

### CLI — Start the HTTP API Server

```bash
# Start the server for multi-instance management
onecrawl serve --port 9867

# In another terminal, create an instance and automate via REST
curl -X POST http://localhost:9867/instances \
  -H "Content-Type: application/json" \
  -d '{"headless": true}'
```

### Node.js

```javascript
const { NativeBrowser } = require("@onecrawl/native");

async function main() {
  // Launch a new browser instance
  const browser = new NativeBrowser();
  await browser.launch({ headless: true });

  // Navigate to a page
  await browser.goto("https://example.com");

  // Take a screenshot
  await browser.screenshot({ path: "example.png", fullPage: true });

  // Extract visible text
  const text = await browser.getText();
  console.log(text);

  // Close the browser
  await browser.close();
}

main();
```

### Python

```python
from onecrawl import Browser

async def main():
    # Launch a new browser instance
    browser = Browser()
    await browser.launch(headless=True)

    # Navigate to a page
    await browser.goto("https://example.com")

    # Take a screenshot
    await browser.screenshot(path="example.png", full_page=True)

    # Extract visible text
    text = await browser.get_text()
    print(text)

    # Close the browser
    await browser.close()

import asyncio
asyncio.run(main())
```

## What's Next

- **[CLI Reference](./cli-reference.md)** — Full list of 200+ commands with examples
- **[HTTP Server API](./http-api.md)** — REST API for multi-instance Chrome management
- **[MCP Tools Reference](./mcp-tools.md)** — 51 tools for AI agent integration
- **[Node.js SDK](./sdk-nodejs.md)** — Native bindings for Node.js
- **[Python SDK](./sdk-python.md)** — Native bindings for Python
- **[Architecture](./architecture.md)** — Internals, crate structure, and design principles
