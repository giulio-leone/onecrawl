---
name: scraping
description: "Smart selectors (CSS/XPath with ::text/::attr), DOM navigation, content extraction, streaming extraction with pagination, and structured data (JSON-LD, OpenGraph, Twitter Card) for comprehensive web scraping."
---

# Scraping & Extraction Skill

Comprehensive web scraping using OneCrawl's Rust-native CDP modules. Covers element selection, DOM traversal, content extraction, and structured data parsing.

## Modules

| Module | Purpose |
|--------|---------|
| `selectors` | CSS/XPath with `::text`/`::attr()`, find-by-text, find-by-regex, auto-selector |
| `dom_nav` | Parent, children, siblings, find-similar, above/below navigation |
| `extract` | Text, HTML, Markdown, JSON extraction + page metadata |
| `streaming` | Schema-based multi-field extraction with pagination |
| `structured_data` | JSON-LD, OpenGraph, Twitter Card, page metadata |
| `form_filler` | Form detection, auto-fill with fuzzy matching, submission |

## How It Works

### Smart Selectors
```bash
# CSS with pseudo-elements
onecrawl select css "h1::text"
onecrawl select css "a::attr(href)"

# Find by text/regex
onecrawl select find-by-text "Sign In"
onecrawl select find-by-regex "price.*\$[0-9]+"

# Auto-generate optimal selector
onecrawl select auto ".product-title"

# XPath
onecrawl select xpath "//div[@class='content']//p"
```

### DOM Navigation
```bash
onecrawl dom parent ".child-element"
onecrawl dom children ".container"
onecrawl dom siblings ".item"
onecrawl dom find-similar ".product-card"
onecrawl dom above ".target" "div"
onecrawl dom below ".target" "p"
```

### Content Extraction
```bash
onecrawl extract text ".article-body"
onecrawl extract html ".content"
onecrawl extract markdown ".post"
onecrawl extract json "table.data"
onecrawl extract metadata
```

### Streaming Extraction (Schema-Based)
```bash
# Define schema: item selector + field rules + optional pagination
onecrawl stream-extract ".product-card" \
  --field "name:css:h3:text" \
  --field "price:css:.price:text" \
  --field "link:css:a:attr:href" \
  --paginate ".next-page" \
  --max-pages 10 \
  --output products.json \
  --format json
```

### Structured Data
```bash
onecrawl structured extract-all      # JSON-LD + OG + Twitter + metadata
onecrawl structured json-ld          # Only JSON-LD scripts
onecrawl structured open-graph       # Only OpenGraph meta tags
onecrawl structured twitter-card     # Only Twitter Card tags
onecrawl structured metadata         # Page meta info
onecrawl structured validate         # Check completeness
```

### Form Filling
```bash
onecrawl form detect                 # List all forms + fields
onecrawl form fill "form#login" --field "email=user@ex.com" --field "password=secret"
onecrawl form auto-fill "form" --profile '{"email":"user@ex.com","name":"John"}'
onecrawl form submit "form#login"
```

## Node.js API
```javascript
const browser = new NativeBrowser();
await browser.launch();
await browser.goto('https://example.com');

// Smart selectors
const titles = await browser.selectCss('h1::text');
const links = await browser.selectXpath('//a/@href');
const btn = await browser.findByText('Submit');

// DOM navigation
const parent = await browser.domParent('.child');
const similar = await browser.domFindSimilar('.product-card');

// Content extraction
const text = await browser.extractText('.article');
const md = await browser.extractMarkdown('.post');

// Streaming extraction
const schema = {
  item_selector: '.product',
  fields: [
    { name: 'title', selector: 'h3', extract: 'text' },
    { name: 'price', selector: '.price', extract: 'text', transform: 'trim' }
  ],
  pagination: { next_selector: '.next', max_pages: 5, delay_ms: 1000 }
};
const results = await browser.streamExtract(JSON.stringify(schema));

// Structured data
const structured = await browser.extractStructuredAll();
const jsonLd = await browser.extractJsonLd();
```

## Python API
```python
browser = Browser()
browser.launch()
browser.goto('https://example.com')

# Smart selectors
titles = browser.select_css('h1::text')
links = browser.select_xpath('//a/@href')

# Streaming extraction
schema = {
    "item_selector": ".product",
    "fields": [
        {"name": "title", "selector": "h3", "extract": "text"},
        {"name": "price", "selector": ".price", "extract": "text"}
    ]
}
results = browser.stream_extract(json.dumps(schema))

# Structured data
all_data = browser.extract_structured_all()
```
