---
sidebar_position: 1
title: Getting Started
---

# Getting Started

## What is OneCrawl?

OneCrawl is a high-performance browser automation engine written in Rust with native bindings for Node.js and Python. It ships as a single **~5.8 MB binary** and delivers:

- **409+ CLI commands** for every browser automation task
- **17 MCP super-tools** exposing **421 actions** for AI agent integration
- **43 HTTP API routes** for multi-instance Chrome management
- **391 Node.js methods** (NAPI-RS) and **509 Python methods** (PyO3)
- **97 CDP modules** with **662 functions** powering the engine

Built on top of the Chrome DevTools Protocol (CDP), OneCrawl delivers raw speed and reliability for scraping, testing, stealth automation, and accessibility auditing — all from a single binary.

---

## Why OneCrawl?

OneCrawl replaces an entire stack of browser automation tools with a single, lightweight binary.

| Feature | OneCrawl | Playwright | Puppeteer | Selenium |
|---|---|---|---|---|
| **Binary size** | ~5.8 MB | ~250 MB | ~170 MB | ~30 MB + drivers |
| **Startup time** | &lt;100 ms | ~2 s | ~1.5 s | ~3 s |
| **Memory (idle)** | ~15 MB | ~80 MB | ~60 MB | ~120 MB |
| **Language** | Rust (native) | JS/TS | JS/TS | Java / polyglot |
| **CLI commands** | 409+ | ~10 | N/A | N/A |
| **MCP tools** | 17 super-tools (421 actions) | — | — | — |
| **HTTP API server** | 43 routes, multi-instance | — | — | Grid (separate) |
| **Stealth built-in** | ✅ Native (12 patches) | ❌ Plugin needed | ❌ Plugin needed | ❌ Plugin needed |
| **Anti-bot bypass** | ✅ Built-in | ❌ | ❌ | ❌ |
| **Passkey / WebAuthn** | ✅ Native | ❌ | ❌ | ❌ |
| **CAPTCHA detection** | ✅ Built-in | ❌ | ❌ | ❌ |
| **Accessibility refs** | Snapshot-based | Locators | Selectors | Selectors |
| **Node.js SDK** | 391 methods | Full API | Full API | Via bindings |
| **Python SDK** | 509 methods | Full API | N/A | Full API |
| **Encrypted KV store** | ✅ AES-256-GCM | ❌ | ❌ | ❌ |

### When to choose OneCrawl

- **You need speed.** Sub-100 ms startup, native Rust performance, zero-copy bindings.
- **You need stealth.** Built-in fingerprint spoofing, 12 anti-detection patches, CAPTCHA detection.
- **You work with AI agents.** 17 MCP super-tools integrate directly with Claude, GPT, and other LLM frameworks.
- **You want one tool.** CLI, HTTP API, Node.js SDK, Python SDK — all from a single binary.
- **You need multi-instance management.** Spin up, manage, and tear down Chrome instances via REST API.

---

## Features at a Glance

- **Blazing fast** — Native Rust binary, sub-100 ms startup, ~12k req/s HTTP throughput
- **Stealth mode** — 12 anti-detection patches, fingerprint spoofing, CAPTCHA detection
- **AI-native** — 17 MCP super-tools with 421 actions for seamless AI agent integration
- **HTTP API** — 43 routes for multi-instance Chrome management with tab locking
- **Screenshots & PDF** — Full-page, element, visual diff, and PDF export
- **Spider & Crawl** — Site-wide crawling with configurable depth and concurrency
- **Accessibility** — Built-in a11y auditing and accessibility tree snapshots
- **Auth & Passkeys** — Native WebAuthn/passkey support for modern authentication flows
- **HAR & Network** — Full network logging, HAR export, request interception, WebSocket
- **Multi-SDK** — Node.js (391 methods) and Python (509 methods) native bindings
- **Single binary** — No external dependencies, just Chrome on `$PATH`
- **Pipelines** — Chain commands via YAML/JSON pipeline files
- **Encrypted storage** — AES-256-GCM encrypted key-value store for credentials
- **550+ tests** — 362 unit tests + 188 E2E tests for reliability

---

## Prerequisites

| Requirement | Minimum Version | Notes |
|---|---|---|
| **Chrome / Chromium** | 120+ | Must be installed and accessible on `$PATH` |
| **Rust** | 1.75+ | Required only for building from source |
| **Node.js** *(optional)* | 18+ | For the Node.js SDK (`@onecrawl/native`) |
| **Python** *(optional)* | 3.9+ | For the Python SDK (`onecrawl`) |

> **Tip:** OneCrawl auto-detects Chrome on your system. Run `onecrawl info` to verify.

---

## Installation

### Path 1: CLI (from source)

```bash
# Clone the repository
git clone https://github.com/AstroLabs-AI/onecrawl.git
cd onecrawl

# Build and install the CLI binary
cargo install --path packages/onecrawl-rust/crates/onecrawl-cli-rs

# Verify the installation
onecrawl version
```

### Path 2: Node.js SDK

```bash
npm install @onecrawl/native
```

### Path 3: Python SDK

```bash
pip install onecrawl
```

---

## Your First Automation in 5 Minutes

Choose your preferred path below. Each example navigates to a page, takes a screenshot, and extracts text — in under 10 lines of code.

### CLI — The fastest path

```bash
# 1. Navigate to a page
onecrawl navigate "https://example.com"

# 2. Take a full-page screenshot
onecrawl screenshot --full --output example.png

# 3. Extract visible text
onecrawl get text

# 4. Get the page title
onecrawl get title

# 5. Close the browser
onecrawl close
```

**Time to first result: ~3 seconds.**

### Node.js — Async/await with native speed

```javascript
const { NativeBrowser } = require("@onecrawl/native");

async function main() {
  const browser = new NativeBrowser();
  await browser.launch({ headless: true });

  await browser.goto("https://example.com");
  await browser.screenshot({ path: "example.png", fullPage: true });

  const title = await browser.getTitle();
  const text = await browser.getText();
  console.log(`Title: ${title}`);
  console.log(`Text: ${text.slice(0, 200)}...`);

  await browser.close();
}

main().catch(console.error);
```

### Python — Clean, synchronous-feeling API

```python
import asyncio
from onecrawl import Browser

async def main():
    browser = Browser()
    await browser.launch(headless=True)

    await browser.goto("https://example.com")
    await browser.screenshot(path="example.png", full_page=True)

    title = await browser.get_title()
    text = await browser.get_text()
    print(f"Title: {title}")
    print(f"Text: {text[:200]}...")

    await browser.close()

asyncio.run(main())
```

---

## Quick Examples

### Scrape product data with stealth

```bash
# Enable stealth to avoid detection
onecrawl stealth inject

# Navigate to a product page
onecrawl navigate "https://shop.example.com/product/123"

# Extract structured data
onecrawl structured "https://shop.example.com/product/123" \
  '{"name": "h1.product-title", "price": ".price-tag", "rating": ".star-rating"}'
```

### Start the HTTP API server

```bash
# Start the server for multi-instance management
onecrawl serve --port 9867

# In another terminal, create an instance and automate via REST
curl -X POST http://localhost:9867/instances \
  -H "Content-Type: application/json" \
  -d '{"headless": true}'
```

### Start the MCP server for AI agents

```bash
# stdio transport (for local AI agents like Claude Desktop)
onecrawl mcp --transport stdio

# SSE transport (for remote connections)
onecrawl mcp --transport sse --port 3001
```

### Crawl a site and screenshot every page

```bash
# Spider the site and save the URL list
onecrawl spider "https://docs.example.com" --depth 2 --output urls.json

# Take a screenshot of the homepage
onecrawl navigate "https://docs.example.com"
onecrawl screenshot --full --output homepage.png
```

---

## What's Next

| Guide | Description |
|---|---|
| **[CLI Reference](./cli-reference.md)** | Full list of 409+ commands with examples |
| **[HTTP Server API](./http-api.md)** | 43 REST routes for multi-instance Chrome management |
| **[MCP Tools Reference](./mcp-tools.md)** | 17 super-tools with 421 actions for AI agent integration |
| **[Node.js SDK](./sdk-nodejs.md)** | 391 native NAPI-RS methods |
| **[Python SDK](./sdk-python.md)** | 509 native PyO3 methods |
| **[Architecture](./architecture.md)** | 97 CDP modules, crate structure, and design principles |
