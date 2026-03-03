---
sidebar_position: 1
title: Getting Started
---

# Getting Started

## What is OneCrawl

OneCrawl is a high-performance browser automation engine written in Rust with native bindings for Node.js and Python. It provides **200+ CLI commands**, **51 MCP tools** for AI agent integration, and a full **HTTP API server** for multi-instance Chrome management. Built on top of the Chrome DevTools Protocol (CDP), OneCrawl delivers raw speed and reliability for scraping, testing, stealth automation, and accessibility auditing — all from a single ~5.8 MB binary.

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

### CLI

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
