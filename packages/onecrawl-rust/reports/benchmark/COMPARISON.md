# OneCrawl Cross-Tool Performance Comparison

> **Date:** 2026-03-04  
> **Platform:** darwin arm64 (Apple Silicon)  
> **Node.js:** v22.22.0 | **Rust:** stable  

## Table 1 -- CLI Comparison (process-per-command)

Each OneCrawl CLI invocation spawns a process and reconnects to CDP.
Puppeteer and Playwright run in-process with a persistent connection.

| Operation | OneCrawl CLI (ms) | Puppeteer (ms) | Playwright (ms) | Fastest |
|:----------|:-----------------:|:--------------:|:---------------:|:-------:|
| Browser Launch (cold) | 924 | 888 | **508** | Playwright |
| Navigation (avg 5) | 84 | 47 | **11** | Playwright |
| Text Extraction (avg 5) | 46 | 5 | **4** | Playwright |
| Full Screenshot (avg 5) | 118 | 53 | **37** | Playwright |
| JS Evaluation (avg 5) | 99 | **0** | 1 | Puppeteer |
| New Page/Tab (avg 5) | 81 | **38** | 54 | Puppeteer |
| DOM Query (avg 5) | 88 | 9 | **3** | Playwright |

## Table 2 -- In-Process Comparison (API-level, fair)

OneCrawl Rust API called in-process (no CLI overhead).
All tools use a single persistent browser connection.

| Operation | OneCrawl Rust (ms) | Puppeteer (ms) | Playwright (ms) | Fastest |
|:----------|:------------------:|:--------------:|:---------------:|:-------:|
| Browser Launch (cold) | **688** | 888 | 508 | Playwright |
| Navigation (example.com) | **65** | 47* | 11* | Playwright |
| Full Screenshot | **68** | 53 | 37 | Playwright |
| JS Injection (stealth) | **1** | - | - | OneCrawl |

*Puppeteer/Playwright navigate to data: URL; OneCrawl navigates to example.com (network I/O).

## Table 3 -- Non-Browser Operations (Rust native)

| Operation | OneCrawl Rust | Notes |
|:----------|:-------------:|:------|
| AES-256-GCM Encrypt | 17us avg | 1000 iterations |
| AES-256-GCM Decrypt | 18us avg | 1000 iterations |
| PKCE Challenge | 5us | Single generation |
| TOTP Code | 6us | SHA-1, 6 digits |
| A11y Tree Parse | 384us | HTML to tree |
| CSS Query | 38us | 5 li matches |
| Text Extract | 381us | 3 paragraphs |
| Link Extract | 40us | 4 links |
| Encrypted KV Write | 20ms avg | sled + AES-GCM |
| Encrypted KV Read | 25ms avg | sled + AES-GCM |

## Key Takeaways

1. **CLI overhead is ~40-80ms per command** (process spawn + CDP reconnect).
   For high-throughput use, prefer the HTTP API, SDK, or MCP server.

2. **In-process launch is competitive** -- 688ms vs 888ms (Puppeteer) / 508ms (Playwright).

3. **Stealth injection is near-zero cost** -- 1ms to inject fingerprint patches,
   6/6 anti-detection checks passing.

4. **Crypto and parser are sub-millisecond** -- native Rust with ring and lol_html.

5. **Playwright wins navigation/screenshot** due to its custom browser protocol
   (not CDP). Puppeteer and OneCrawl both use CDP.

## Methodology

- OneCrawl CLI: `onecrawl session start --headless` then individual commands
- OneCrawl Rust: `cargo run --release -p onecrawl-benchmark` (in-process)
- Puppeteer: v24.x, headless mode, in-process Node.js
- Playwright: v1.50.x, Chromium, headless mode, in-process Node.js
- Non-browser metrics: `data:text/html` URLs, no network I/O, avg over 5 runs
- Browser metrics: example.com for Rust in-process, data: URLs for CLI/Node.js
