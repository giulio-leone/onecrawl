# OneCrawl SKILL.md -- AI-Native Browser Automation

OneCrawl is a high-performance Rust monorepo for browser automation, web scraping,
and AI agent tooling. It exposes 180+ CLI commands, 21 HTTP API endpoints, and
43 MCP tools across 8 crates.

## Skill Index

Detailed documentation is split by domain in `.agent/skills/`:

| Skill | Description | File |
|-------|-------------|------|
| **navigation** | Navigation, content extraction, interaction, screenshots, tabs | `.agent/skills/navigation/SKILL.md` |
| **scraping** | CSS/XPath selectors, DOM navigation, streaming extraction, structured data | `.agent/skills/scraping/SKILL.md` |
| **crawling** | Spider, robots.txt, link graphs, sitemaps, DOM snapshots | `.agent/skills/crawling/SKILL.md` |
| **stealth** | Anti-bot patches, fingerprints, domain blocking, CAPTCHA detection | `.agent/skills/stealth/SKILL.md` |
| **network** | Throttling, HAR, WebSocket, interception, proxies, downloads | `.agent/skills/network/SKILL.md` |
| **automation** | Rate limiting, retry queues, scheduling, session pools, REPL | `.agent/skills/automation/SKILL.md` |
| **data-processing** | Pipelines, filtering, transforms, CSV/JSON/JSONL export | `.agent/skills/data-processing/SKILL.md` |
| **monitoring** | Benchmarks, coverage, console, dialogs, DOM observation, perf tracing | `.agent/skills/monitoring/SKILL.md` |
| **crypto** | AES-256-GCM, PKCE, TOTP, passkeys, encrypted storage | `.agent/skills/crypto/SKILL.md` |
| **server** | HTTP API with multi-instance Chrome, tabs, snapshots, actions | `.agent/skills/server/SKILL.md` |
| **mcp** | 43 MCP tools across 10 namespaces (stdio + SSE) | `.agent/skills/mcp/SKILL.md` |

## Architecture

```
onecrawl-rust/
  crates/
    onecrawl-core/       Shared types, traits, errors
    onecrawl-crypto/     AES-256-GCM, PKCE, TOTP, PBKDF2 (ring)
    onecrawl-parser/     HTML parsing, a11y tree, extraction (lol_html + scraper)
    onecrawl-storage/    Encrypted KV store (sled)
    onecrawl-cdp/        63 CDP modules: stealth, captcha, spider, rate limiter...
    onecrawl-server/     axum HTTP API with multi-instance Chrome management
    onecrawl-cli-rs/     180+ CLI commands via clap
    onecrawl-mcp-rs/     43 MCP tools (stdio + SSE)
  bindings/
    napi/                NAPI-RS -> npm @onecrawl/native
    python/              PyO3 -> pip onecrawl
```

## Quick Start

```bash
onecrawl session start --headless       # Launch headless browser
onecrawl navigate https://example.com   # Navigate
onecrawl get text                       # Extract text
onecrawl screenshot --output page.png   # Screenshot
onecrawl session close                  # Close
```

## Development

```bash
cargo build -p onecrawl-cdp -p onecrawl-server -p onecrawl-cli-rs -p onecrawl-mcp-rs
cargo test -p onecrawl-cdp --lib        # 148 tests
cargo run -p onecrawl-cli-rs -- <cmd>   # Run CLI
cargo run -p onecrawl-cli-rs -- serve   # Start HTTP server
cargo run -p onecrawl-cli-rs -- mcp     # Start MCP server
```
