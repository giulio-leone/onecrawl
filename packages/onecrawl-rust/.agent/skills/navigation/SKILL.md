---
name: navigation
description: "Browser navigation, content extraction, element interaction, keyboard input, screenshots, PDF export, iframes, tabs, and page lifecycle management."
---

# Navigation & Interaction Skill

Browser navigation, content extraction, element interaction, and page lifecycle management.
Handles everything from URL navigation to complex multi-tab, multi-page workflows.

## Modules

| Module | Purpose |
|--------|---------|
| `navigation` | URL navigation, history (back/forward/reload) |
| `content` | Text, HTML, URL, title extraction with optional selectors |
| `interaction` | Click, type, fill, hover, focus, scroll, check, select, drag, upload |
| `keyboard` | Key press, key down/up, shortcuts |
| `screenshot` | PNG/JPEG/WebP screenshots, element screenshots, full-page |
| `pdf` | PDF export with landscape, scale, margins, headers/footers |
| `tabs` | Multi-tab management (open, close, switch, list) |
| `iframes` | iframe listing, evaluation, content extraction |
| `wait` | Wait for time, selector, URL pattern |

## CLI Commands

### Navigation

```bash
onecrawl navigate https://example.com          # Navigate to URL
onecrawl navigate https://example.com --wait 2000  # Navigate + wait 2s
onecrawl back                                   # History back
onecrawl forward                                # History forward
onecrawl reload                                 # Reload page
```

### Content Extraction

```bash
onecrawl get text                       # Full page text
onecrawl get text --selector "article"  # Scoped text
onecrawl get html                       # Full HTML
onecrawl get html --selector "nav"      # Scoped HTML
onecrawl get url                        # Current URL
onecrawl get title                      # Page title
onecrawl eval "document.title"          # Any JS expression
onecrawl set-content "<h1>Hello</h1>"   # Set page content

onecrawl extract content --format markdown --output page.md
onecrawl extract metadata
```

### Element Interaction

```bash
onecrawl click "button.submit"                  # Click
onecrawl dblclick ".item"                       # Double-click
onecrawl type "#search" "query text"            # Type key-by-key
onecrawl fill "#email" "user@example.com"       # Clear + set value
onecrawl focus "#input"                         # Focus
onecrawl hover ".menu-item"                     # Hover
onecrawl scroll-into-view ".footer"             # Scroll into view
onecrawl check "#agree"                         # Check checkbox
onecrawl uncheck "#agree"                       # Uncheck
onecrawl select-option "#country" "US"          # Select dropdown
onecrawl tap ".button"                          # Touch tap
onecrawl drag ".source" ".target"               # Drag and drop
onecrawl upload "#file-input" "./document.pdf"  # File upload
onecrawl bounding-box ".element"                # Get bounding box
```

### Keyboard

```bash
onecrawl press-key Enter                        # Press key
onecrawl press-key Tab
onecrawl press-key Escape
onecrawl key-down Shift                         # Hold key
onecrawl key-up Shift                           # Release key
onecrawl keyboard-shortcut "Control+a"          # Shortcut
onecrawl keyboard-shortcut "Meta+c"
```

### Screenshots and PDF

```bash
onecrawl screenshot --output page.png           # Viewport screenshot
onecrawl screenshot --output full.png --full    # Full page
onecrawl screenshot --element ".chart" --output chart.png
onecrawl screenshot --format jpeg --quality 80 --output photo.jpg
onecrawl screenshot --format webp --output page.webp

onecrawl pdf --output page.pdf
onecrawl pdf --output landscape.pdf --landscape --scale 0.8
onecrawl print pdf --output detailed.pdf --background --margins "1cm"
```

### Tabs

```bash
onecrawl tab list                               # List open tabs
onecrawl tab new "https://example.com"          # Open new tab
onecrawl tab close 1                            # Close tab by index
onecrawl tab switch 0                           # Switch to tab
onecrawl tab count                              # Count tabs
onecrawl new-page "https://example.com"         # Open page in new tab
```

### Iframes

```bash
onecrawl iframe list                            # List all iframes
onecrawl iframe eval 0 "document.title"         # Execute JS in iframe
onecrawl iframe content 0                       # Get iframe HTML
```

### Waiting

```bash
onecrawl wait 2000                              # Wait 2 seconds
onecrawl wait-for-selector ".loaded" --timeout 10000
onecrawl wait-for-url "dashboard" --timeout 15000
```

## HTTP API Endpoints

| Method | Endpoint | Description |
|--------|----------|-------------|
| `POST` | `/tabs/{id}/navigate` | Navigate to URL |
| `GET` | `/tabs/{id}/text` | Extract text |
| `GET` | `/tabs/{id}/url` | Get URL |
| `GET` | `/tabs/{id}/title` | Get title |
| `GET` | `/tabs/{id}/html` | Get full HTML |
| `POST` | `/tabs/{id}/evaluate` | Evaluate JavaScript |
| `POST` | `/tabs/{id}/action` | Execute action (click, type...) |
| `POST` | `/tabs/{id}/actions` | Execute action batch |
| `GET` | `/tabs/{id}/screenshot` | Screenshot (base64 PNG) |
| `GET` | `/tabs/{id}/pdf` | Export PDF (base64) |

## MCP Tools

| Tool | Description |
|------|-------------|
| `navigation.goto` | Navigate to URL |
| `navigation.click` | Click by CSS selector |
| `navigation.type` | Type text into element |
| `navigation.screenshot` | Take screenshot |
| `navigation.pdf` | Export PDF |
| `navigation.back` | Go back |
| `navigation.forward` | Go forward |
| `navigation.reload` | Reload page |
| `navigation.wait` | Wait for selector |
| `navigation.evaluate` | Evaluate JavaScript |
| `navigation.cookies` | Get/set cookies |

## Performance Notes

- Proxy fast-path bypasses CDP reconnect for navigate, get-text, screenshot, eval
- Timing displayed on navigate and screenshot (e.g., "167ms")
- `get_tab_page()` helper clones cheap Page channel handles, drops locks immediately
- Shared `element_action_js()` template for all 6 action types
