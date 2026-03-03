---
sidebar_position: 6
title: Python SDK
---

# Python SDK

Native Rust bindings for Python via PyO3. Full async/await support with `asyncio`, ~130 methods mirroring the Node.js SDK.

## Installation

```bash
pip install onecrawl
```

## Browser Lifecycle

```python
from onecrawl import Browser

# Launch a new browser
browser = Browser()
await browser.launch(headless=True)

# Or connect to an existing instance
browser2 = Browser()
await browser2.connect("ws://localhost:9222/devtools/browser/...")

# Close when done
await browser.close()
```

### Launch Options

| Option | Type | Default | Description |
|---|---|---|---|
| `headless` | `bool` | `True` | Run without a visible window |
| `executable_path` | `str?` | auto-detect | Path to Chrome/Chromium binary |
| `user_data_dir` | `str?` | temp dir | Path to user data directory |
| `args` | `list[str]?` | `[]` | Extra Chrome launch arguments |
| `proxy` | `str?` | `None` | Proxy server URL |

---

## Navigation

```python
# Navigate to a URL
await browser.goto("https://example.com")
await browser.goto("https://example.com", wait_until="networkidle")

# History navigation
await browser.back()
await browser.forward()
await browser.reload()

# Get current URL and title
url = await browser.get_url()
title = await browser.get_title()
```

---

## Screenshots & PDF

```python
# Basic screenshot
await browser.screenshot(path="page.png")

# Full-page screenshot
await browser.screenshot(path="full.png", full_page=True)

# Element screenshot
await browser.screenshot(path="hero.png", selector="#hero")

# JPEG with quality
await browser.screenshot(path="page.jpg", format="jpeg", quality=80)

# Get as base64
base64_data = await browser.screenshot(encoding="base64")

# PDF export
await browser.pdf(path="page.pdf")
await browser.pdf(path="report.pdf", landscape=True, scale=0.8)
```

---

## DOM Interaction

```python
# Click
await browser.click("#submit-btn")
await browser.dblclick("#item")

# Type (keystroke simulation)
await browser.type("#search", "OneCrawl", delay=50)

# Fill (set value directly)
await browser.fill("#email", "user@example.com")

# Focus and hover
await browser.focus("#input")
await browser.hover(".menu-item")

# Checkbox and select
await browser.check("#agree")
await browser.uncheck("#newsletter")
await browser.select_option("#country", "IT")

# File upload
await browser.upload("#file-input", "/path/to/file.pdf")

# Scroll into view
await browser.scroll_into_view("#footer")

# Bounding box
box = await browser.bounding_box("#element")
# {"x": 100, "y": 200, "width": 300, "height": 150}

# Get text content
text = await browser.get_text()
el_text = await browser.get_text("#specific-element")

# Get HTML
html = await browser.get_html()

# Evaluate JavaScript
count = await browser.evaluate("document.querySelectorAll('a').length")

# Set page content
await browser.set_content("<h1>Hello</h1><p>World</p>")
```

---

## Wait Operations

```python
# Wait for a fixed duration (milliseconds)
await browser.wait(2000)

# Wait for an element
await browser.wait_for_selector(".loaded")
await browser.wait_for_selector(".modal", timeout=10000)

# Wait for URL
await browser.wait_for_url("**/dashboard**")
```

---

## Cookies

```python
# Get all cookies
cookies = await browser.get_cookies()

# Get a specific cookie
session = await browser.get_cookie("session_id")

# Set a cookie
await browser.set_cookie(
    name="token",
    value="abc123",
    domain=".example.com",
    path="/",
    http_only=True,
    secure=True,
)

# Delete a specific cookie
await browser.delete_cookie("token")

# Clear all cookies
await browser.clear_cookies()
```

---

## Network

```python
# Throttle network (simulate 3G)
await browser.throttle(
    download_throughput=750 * 1024,
    upload_throughput=250 * 1024,
    latency=100,
)

# Block domains
await browser.block_domains([
    "google-analytics.com",
    "facebook.net",
])

# Go offline / online
await browser.set_offline(True)
await browser.set_offline(False)

# HAR recording
await browser.har_start()
await browser.goto("https://example.com")
har = await browser.har_stop()
await browser.har_export("trace.har")
```

---

## Coverage & Performance

```python
# Code coverage
await browser.coverage_start()
await browser.goto("https://example.com")
report = await browser.coverage_stop()
print(f"Unused: {report.unused_bytes} / {report.total_bytes}")

# Performance metrics
metrics = await browser.performance_metrics()

# Tracing
await browser.trace_start()
await browser.goto("https://example.com")
trace = await browser.trace_stop()
```

---

## Stealth Mode

```python
# Inject stealth patches (call before navigation)
await browser.stealth_inject()

# Run detection tests
results = await browser.stealth_test("https://bot.sannysoft.com")
print(results.detected)  # False

# Randomize fingerprint
await browser.stealth_fingerprint(randomize=True)

# Block tracking domains
await browser.stealth_block_domains([
    "google-analytics.com",
    "doubleclick.net",
])

# Detect CAPTCHA
captcha = await browser.stealth_detect_captcha()
print(captcha.has_captcha)  # True/False
```

---

## WebAuthn / Passkey

```python
# Enable the virtual authenticator
await browser.passkey_enable()

# Add a credential
await browser.passkey_add(
    rp_id="example.com",
    credential_id="cred_abc",
    user_handle="user_123",
    private_key="MIIEvQIBADANBgkq...",
)

# List credentials
creds = await browser.passkey_list()

# View event log
log = await browser.passkey_log()

# Clean up
await browser.passkey_remove("cred_abc")
await browser.passkey_disable()
```

---

## Emulation

```python
# Emulate a device
await browser.emulate_device("iPhone 15 Pro")

# Set viewport
await browser.emulate_viewport(width=1920, height=1080)

# Override timezone and locale
await browser.emulate_timezone("Europe/Rome")
await browser.emulate_locale("it-IT")

# Override geolocation
await browser.emulate_geolocation(latitude=41.9028, longitude=12.4964)

# Override media type
await browser.emulate_media("print")
```

---

## Standalone Functions

### Crypto

```python
from onecrawl import encrypt, decrypt, derive_key, generate_pkce, generate_totp, verify_totp

# AES-256-GCM encryption
key = derive_key("my-password", "salt-value")
encrypted = encrypt("secret data", key)
decrypted = decrypt(encrypted, key)

# PKCE (OAuth 2.0)
pkce = generate_pkce("S256")
# {"code_verifier": "...", "code_challenge": "...", "method": "S256"}

# TOTP
code = generate_totp("JBSWY3DPEHPK3PXP", digits=6, period=30)
valid = verify_totp("482931", "JBSWY3DPEHPK3PXP", digits=6, period=30)
```

### Parser

```python
from onecrawl import parse_accessibility_tree, query_selector, extract_text, extract_links

html = "<div><h1>Title</h1><a href='/about'>About</a></div>"

tree = parse_accessibility_tree(html)
elements = query_selector(html, "h1")
text = extract_text(html)
links = extract_links(html, absolute=True, base_url="https://example.com")
```

### Server

```python
from onecrawl import start_server, get_server_info

# Start the HTTP API server programmatically
server = await start_server(port=9867, bind="127.0.0.1")
info = get_server_info()
# {"port": 9867, "instances": 0, "uptime": 42}
```

### Store (Encrypted KV)

```python
from onecrawl import Store

store = Store("/path/to/store")
await store.set("api_key", "sk-abc123")
value = await store.get("api_key")
keys = await store.list(prefix="api_")
await store.delete("api_key")
```

---

## Real-World Examples

### 1. Scrape with stealth mode

```python
import asyncio
from onecrawl import Browser

async def stealth_scrape(url: str) -> dict:
    browser = Browser()
    await browser.launch(headless=True)
    await browser.stealth_inject()
    await browser.stealth_block_domains(["google-analytics.com"])

    await browser.goto(url, wait_until="networkidle")
    await browser.wait_for_selector(".content")

    title = await browser.get_title()
    text = await browser.get_text(".content")
    link_count = await browser.evaluate("document.querySelectorAll('a').length")

    await browser.close()
    return {"title": title, "text": text, "links": link_count}

result = asyncio.run(stealth_scrape("https://example.com"))
print(result)
```

### 2. Network monitoring with HAR recording

```python
import asyncio
import json
from onecrawl import Browser

async def monitor_network(url: str, output: str = "trace.har"):
    browser = Browser()
    await browser.launch(headless=True)

    # Start HAR recording
    await browser.har_start()

    # Navigate and interact
    await browser.goto(url, wait_until="networkidle")

    # Get performance metrics
    metrics = await browser.performance_metrics()
    print(f"DOM nodes: {metrics.get('Nodes', 'N/A')}")
    print(f"JS listeners: {metrics.get('JSEventListeners', 'N/A')}")

    # Stop and export HAR
    har_data = await browser.har_stop()
    await browser.har_export(output)
    print(f"HAR exported to {output}")

    # Coverage analysis
    await browser.coverage_start()
    await browser.reload()
    coverage = await browser.coverage_stop()
    unused_pct = (coverage.unused_bytes / coverage.total_bytes) * 100
    print(f"Unused code: {unused_pct:.1f}%")

    await browser.close()

asyncio.run(monitor_network("https://example.com"))
```

### 3. Accessibility audit

```python
import asyncio
import json
from onecrawl import Browser

async def accessibility_audit(urls: list[str]):
    browser = Browser()
    await browser.launch(headless=True)

    results = []
    for url in urls:
        await browser.goto(url, wait_until="networkidle")

        # Get full accessibility tree
        tree = await browser.evaluate("""
            JSON.stringify(
                (function getTree(el, depth = 0) {
                    const role = el.getAttribute('role') || el.tagName.toLowerCase();
                    const name = el.getAttribute('aria-label') || el.textContent?.trim().slice(0, 50);
                    const children = Array.from(el.children).map(c => getTree(c, depth + 1));
                    return { role, name, children: children.length ? children : undefined };
                })(document.body)
            )
        """)

        # Check for common issues
        issues = await browser.evaluate("""
            JSON.stringify({
                imagesWithoutAlt: document.querySelectorAll('img:not([alt])').length,
                inputsWithoutLabel: document.querySelectorAll('input:not([aria-label]):not([id])').length,
                missingLang: !document.documentElement.lang,
                lowContrast: document.querySelectorAll('[style*="color: #ccc"]').length,
            })
        """)

        results.append({
            "url": url,
            "tree": json.loads(tree),
            "issues": json.loads(issues),
        })

    await browser.close()

    for r in results:
        print(f"\n{'='*60}")
        print(f"URL: {r['url']}")
        for key, val in r["issues"].items():
            status = "✅" if not val else f"⚠️  {val}"
            print(f"  {key}: {status}")

    return results

asyncio.run(accessibility_audit([
    "https://example.com",
    "https://example.com/about",
]))
```
