---
name: crawling
description: "Spider/crawler framework with pause/resume, robots.txt compliance, link graph analysis, sitemap generation, and DOM snapshot diffing for comprehensive web crawling."
---

# Crawling & Discovery Skill

OneCrawl's Rust-native web crawling framework. Supports configurable spiders, robots.txt compliance, link graph analysis, sitemap generation, and page change detection.

## Modules

| Module | Purpose |
|--------|---------|
| `spider` | Crawl framework with configurable depth, concurrency, pause/resume |
| `robots` | robots.txt parser with allow/disallow checking |
| `link_graph` | Link extraction, graph building, orphan/hub analysis |
| `sitemap` | XML sitemap generation, parsing, and crawl result conversion |
| `snapshot` | DOM snapshot capture and Jaccard-based diff analysis |

## How It Works

### Spider Crawl
```bash
# Basic crawl
onecrawl spider crawl https://example.com \
  --max-depth 3 \
  --max-pages 100 \
  --delay 500 \
  --same-domain \
  --output results.json

# Extract specific content during crawl
onecrawl spider crawl https://example.com \
  --selector "article" \
  --format markdown \
  --output-format jsonl

# Resume interrupted crawl
onecrawl spider resume crawl-state.json

# View crawl summary
onecrawl spider summary results.json
```

### Robots.txt Compliance
```bash
onecrawl robots parse https://example.com
onecrawl robots check https://example.com /private/page
onecrawl robots sitemaps https://example.com
```

### Link Graph Analysis
```bash
onecrawl graph extract --base-url https://example.com
onecrawl graph build edges.json
onecrawl graph analyze graph.json    # orphans, hubs, stats
onecrawl graph export graph.json output.json
```

### Sitemap Generation
```bash
onecrawl sitemap generate output.xml --entries entries.json
onecrawl sitemap from-crawl results.json
onecrawl sitemap parse existing-sitemap.xml
```

### Page Snapshot & Diff
```bash
onecrawl snapshot take --output before.json
# ... wait for changes ...
onecrawl snapshot take --output after.json
onecrawl snapshot compare before.json after.json

# Watch for changes (auto-diff)
onecrawl snapshot watch --interval 5000 --count 10
```

## Node.js API
```javascript
// Spider crawl
const config = {
  start_urls: ['https://example.com'],
  max_depth: 3,
  max_pages: 100,
  same_domain_only: true,
  delay_ms: 500
};
const results = await browser.crawl(JSON.stringify(config));
const summary = browser.crawlSummary(results);

// Robots.txt
const robots = await browser.fetchRobots('https://example.com');
const allowed = browser.robotsIsAllowed(robots, '*', '/private');

// Link graph
const edges = await browser.extractLinks('https://example.com');
const graph = browser.buildGraph(edges);
const stats = browser.analyzeGraph(graph);

// Snapshot diff
const before = await browser.takeSnapshot();
const after = await browser.takeSnapshot();
const diff = browser.compareSnapshots(before, after);
```
