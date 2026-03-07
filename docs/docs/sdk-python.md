---
sidebar_position: 6
title: Python SDK
---

# Python SDK

Native Rust bindings for Python via PyO3. Full async/await and synchronous API support, **509 exported methods** across 4 core classes and standalone functions.

## Installation

```bash
pip install onecrawl
```

**Prebuilt wheels** available for:
- Linux (x64, ARM64)
- macOS (x64, Apple Silicon)
- Windows (x64)

**Requirements:** Python 3.9+. No Rust toolchain required.

---

## Type Hints

The SDK ships with comprehensive type stubs (`.pyi`) for IDE autocompletion:

```python
from onecrawl import (
    Browser,
    Orchestrator,
    PluginManager,
    Studio,
    Store,
)

from onecrawl.types import (
    LaunchOptions,
    ScreenshotOptions,
    CookieParam,
    Viewport,
    HarEntry,
    CoverageReport,
    AccessibilityNode,
    StealthTestResult,
    PasskeyCredential,
)
```

---

## Synchronous API

All browser methods are available in both sync and async variants. Use the sync API for scripts and the async API for production services.

### Sync Usage (no `asyncio` needed)

```python
from onecrawl import Browser

browser = Browser()
browser.launch_sync(headless=True)

browser.goto_sync("https://example.com")
title = browser.get_title_sync()
text = browser.get_text_sync()
browser.screenshot_sync(path="page.png", full_page=True)

print(f"Title: {title}")
print(f"Text: {text[:200]}...")

browser.close_sync()
```

### Async Usage

```python
import asyncio
from onecrawl import Browser

async def main():
    browser = Browser()
    await browser.launch(headless=True)

    await browser.goto("https://example.com")
    title = await browser.get_title()
    text = await browser.get_text()
    await browser.screenshot(path="page.png", full_page=True)

    print(f"Title: {title}")
    print(f"Text: {text[:200]}...")

    await browser.close()

asyncio.run(main())
```

---

## Class Reference

### `Browser`

The primary class for browser automation. **~240 methods.**

#### Lifecycle

```python
from onecrawl import Browser

# Launch a new browser
browser = Browser()
await browser.launch(headless=True)

# Or connect to an existing Chrome DevTools instance
browser2 = Browser()
await browser2.connect("ws://localhost:9222/devtools/browser/...")

# Close when done
await browser.close()
```

##### Launch Options

| Option | Type | Default | Description |
|---|---|---|---|
| `headless` | `bool` | `True` | Run without a visible window |
| `executable_path` | `str \| None` | auto-detect | Path to Chrome/Chromium binary |
| `user_data_dir` | `str \| None` | temp dir | Path to user data directory |
| `args` | `list[str] \| None` | `[]` | Extra Chrome launch arguments |
| `proxy` | `str \| None` | `None` | Proxy server URL |
| `timeout` | `int \| None` | `30000` | Default timeout in ms |
| `slow_mo` | `int \| None` | `0` | Slow down operations by this many ms |

#### Navigation

```python
# Navigate to a URL
await browser.goto("https://example.com")
await browser.goto("https://example.com", wait_until="networkidle")

# History navigation
await browser.back()
await browser.forward()
await browser.reload()
await browser.reload(hard=True)

# Get current page info
url = await browser.get_url()
title = await browser.get_title()
```

#### Screenshots & PDF

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

# Get as bytes
image_bytes = await browser.screenshot(encoding="bytes")

# PDF export
await browser.pdf(path="page.pdf")
await browser.pdf(path="report.pdf", landscape=True, scale=0.8)
```

#### DOM Interaction

```python
# Click
await browser.click("#submit-btn")
await browser.dblclick("#item")
await browser.click("#menu", button="right")

# Type (keystroke simulation)
await browser.type("#search", "OneCrawl", delay=50)

# Fill (set value directly — faster)
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

# Scroll
await browser.scroll_into_view("#footer")
await browser.scroll_to(0, 500)

# Bounding box
box = await browser.bounding_box("#element")
# {"x": 100, "y": 200, "width": 300, "height": 150}

# Drag and drop
await browser.drag("#card", "#column-done")

# Get text content
text = await browser.get_text()
el_text = await browser.get_text("#specific-element")

# Get HTML
html = await browser.get_html()
inner_html = await browser.get_html("#container")

# Evaluate JavaScript
count = await browser.evaluate("document.querySelectorAll('a').length")
data = await browser.evaluate("JSON.stringify(window.__DATA__)")

# Set page content
await browser.set_content("<h1>Hello</h1><p>World</p>")
```

#### Wait Operations

```python
# Wait for a fixed duration (milliseconds)
await browser.wait(2000)

# Wait for an element
await browser.wait_for_selector(".loaded")
await browser.wait_for_selector(".modal", timeout=10000)

# Wait for element to disappear
await browser.wait_for_selector(".spinner", state="hidden")

# Wait for URL
await browser.wait_for_url("**/dashboard**")

# Wait for network idle
await browser.wait_for_network_idle(idle_time=500)
```

#### Keyboard

```python
await browser.press_key("Enter")
await browser.press_key("Tab")
await browser.keyboard_shortcut("Ctrl+A")
await browser.key_down("Shift")
await browser.key_up("Shift")
```

#### Cookies

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

#### Network

```python
# Throttle network (simulate 3G)
await browser.throttle(
    download_throughput=750 * 1024,
    upload_throughput=250 * 1024,
    latency=100,
)

# Block domains
await browser.block_domains(["google-analytics.com", "facebook.net"])

# Go offline / online
await browser.set_offline(True)
await browser.set_offline(False)

# HAR recording
await browser.har_start()
await browser.goto("https://example.com")
har = await browser.har_stop()
await browser.har_export("trace.har")

# WebSocket
await browser.ws_connect("wss://stream.example.com")
await browser.ws_send('{"subscribe": "prices"}')
await browser.ws_close()
```

#### Coverage & Performance

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

#### Stealth Mode

```python
# Inject stealth patches (call before navigation)
await browser.stealth_inject()

# Run detection tests
results = await browser.stealth_test("https://bot.sannysoft.com")
print(results.detected)  # False

# Randomize fingerprint
await browser.stealth_fingerprint(randomize=True)

# Block tracking domains
await browser.stealth_block_domains(["google-analytics.com", "doubleclick.net"])

# Detect CAPTCHA
captcha = await browser.stealth_detect_captcha()
print(captcha.has_captcha)  # True/False
```

#### WebAuthn / Passkey

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

#### Emulation

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

# CPU throttling
await browser.emulate_cpu_throttle(4)  # 4x slowdown
```

#### Accessibility

```python
# Get accessibility snapshot
snapshot = await browser.accessibility_snapshot()
interactive = await browser.accessibility_snapshot(filter="interactive")

# Run accessibility audit
audit = await browser.accessibility_audit()
print(audit.violations)
```

#### Console & Dialog

```python
# Capture console messages
await browser.console_start()
await browser.goto("https://example.com")
messages = await browser.console_messages()
await browser.console_stop()

# Handle dialogs
await browser.dialog_accept("Yes")
await browser.dialog_dismiss()
```

#### Tabs & iFrames

```python
# Tab management
tab_id = await browser.tab_open("https://example.com")
tabs = await browser.tab_list()
await browser.tab_switch(tab_id)
await browser.tab_close(tab_id)

# iFrame navigation
iframes = await browser.iframe_list()
await browser.iframe_switch(0)
await browser.iframe_switch_main()
```

---

### `Orchestrator`

Multi-instance browser management. **~60 methods.**

```python
from onecrawl import Orchestrator

orchestrator = Orchestrator()

# Create multiple browser instances
browser1 = await orchestrator.create(headless=True)
browser2 = await orchestrator.create(headless=True, profile="stealth")

# List all instances
instances = await orchestrator.list()

# Get instance by ID
instance = await orchestrator.get(browser1.id)

# Stop a specific instance
await orchestrator.stop(browser1.id)

# Stop all instances
await orchestrator.stop_all()

# Pool management
pool = await orchestrator.create_pool(size=5)
browser = await pool.acquire()
# ... use browser ...
await pool.release(browser)
```

---

### `PluginManager`

Plugin system for extending OneCrawl. **~35 methods.**

```python
from onecrawl import PluginManager

plugins = PluginManager()

# Load a plugin
await plugins.load("stealth")
await plugins.load("captcha-solver")

# List loaded plugins
loaded = plugins.list()

# Configure a plugin
await plugins.configure("stealth", level="maximum")

# Execute a plugin action
result = await plugins.execute("stealth", "inject")
```

---

### `Studio`

Visual debugging and recording. **~30 methods.**

```python
from onecrawl import Studio

studio = Studio()

# Start recording
await studio.start_recording(output="session.json")

# Replay a recorded session
await studio.replay("session.json")

# Visual diff
diff = await studio.screenshot_diff("before.png", "after.png")
print(diff.change_percentage)
```

---

### `Store`

Encrypted key-value store. **~15 methods.**

```python
from onecrawl import Store

store = Store("/path/to/store")

# CRUD operations
await store.set("api_key", "sk-abc123")
value = await store.get("api_key")
exists = await store.has("api_key")
keys = await store.list(prefix="api_")
await store.delete("api_key")
await store.clear()
```

---

## Standalone Functions (~129 exports)

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
from onecrawl import start_server, stop_server, get_server_info

# Start the HTTP API server programmatically
server = await start_server(port=9867, bind="127.0.0.1")
info = get_server_info()
# {"port": 9867, "instances": 0, "uptime": 42}
await stop_server()
```

---

## Error Handling

All errors are subclasses of `OneCrawlError`:

```python
from onecrawl import Browser, OneCrawlError
from onecrawl.errors import (
    ElementNotFoundError,
    TimeoutError,
    NavigationError,
    BrowserDisconnectedError,
    BrowserLaunchError,
    CryptoError,
    StorageError,
)

async def safe_scrape(url: str) -> str | None:
    browser = Browser()
    try:
        await browser.launch(headless=True)
        await browser.goto(url)
        return await browser.get_text()
    except ElementNotFoundError as e:
        print(f"Element not found: {e.selector}")
        return None
    except TimeoutError as e:
        print(f"Timeout after {e.timeout_ms}ms")
        return None
    except NavigationError as e:
        print(f"Failed to load {e.url}: {e.status_code}")
        return None
    except BrowserDisconnectedError:
        print("Chrome crashed unexpectedly")
        return None
    except OneCrawlError as e:
        print(f"OneCrawl error: {e}")
        return None
    finally:
        await browser.close()
```

### Error Classes

| Error Class | Description |
|---|---|
| `OneCrawlError` | Base error class for all OneCrawl errors |
| `ElementNotFoundError` | CSS selector matched no elements |
| `TimeoutError` | Operation exceeded timeout |
| `NavigationError` | Page failed to load |
| `BrowserDisconnectedError` | Chrome process crashed |
| `BrowserLaunchError` | Chrome failed to start |
| `CryptoError` | Encryption/decryption failure |
| `StorageError` | KV store failure |
| `InvalidArgumentError` | Invalid parameter value |

---

## Agent-in-the-Loop Patterns

### LangChain Integration

```python
from langchain.tools import tool
from onecrawl import Browser

browser = Browser()

@tool
def navigate_to(url: str) -> str:
    """Navigate to a URL and return the page text."""
    browser.launch_sync(headless=True)
    browser.goto_sync(url, wait_until="networkidle")
    text = browser.get_text_sync()
    return text[:2000]

@tool
def take_screenshot(url: str) -> str:
    """Take a screenshot of a URL and return the base64 image."""
    browser.goto_sync(url)
    return browser.screenshot_sync(encoding="base64")

@tool
def extract_links(url: str) -> list[dict]:
    """Extract all links from a URL."""
    browser.goto_sync(url)
    count = browser.evaluate_sync("document.querySelectorAll('a').length")
    links = browser.evaluate_sync("""
        JSON.stringify(
            Array.from(document.querySelectorAll('a'))
                .map(a => ({text: a.textContent.trim(), href: a.href}))
        )
    """)
    return json.loads(links)
```

### CrewAI Integration

```python
from crewai import Agent, Task, Crew
from crewai_tools import tool
from onecrawl import Browser

@tool("web_scraper")
def web_scraper(url: str) -> str:
    """Scrape a web page and return its content."""
    browser = Browser()
    browser.launch_sync(headless=True)
    browser.stealth_inject_sync()
    browser.goto_sync(url, wait_until="networkidle")
    text = browser.get_text_sync()
    browser.close_sync()
    return text

researcher = Agent(
    role="Web Researcher",
    goal="Research topics by scraping web pages",
    tools=[web_scraper],
)
```

### OpenAI Function Calling

```python
import json
import openai
from onecrawl import Browser

browser = Browser()
browser.launch_sync(headless=True)

tools = [
    {
        "type": "function",
        "function": {
            "name": "navigate",
            "description": "Navigate browser to a URL and return page text",
            "parameters": {
                "type": "object",
                "properties": {"url": {"type": "string", "description": "URL to navigate to"}},
                "required": ["url"],
            },
        },
    },
    {
        "type": "function",
        "function": {
            "name": "click_element",
            "description": "Click an element on the page",
            "parameters": {
                "type": "object",
                "properties": {"selector": {"type": "string", "description": "CSS selector"}},
                "required": ["selector"],
            },
        },
    },
]

def handle_tool_call(name: str, args: dict) -> str:
    if name == "navigate":
        browser.goto_sync(args["url"], wait_until="networkidle")
        return browser.get_text_sync()[:2000]
    elif name == "click_element":
        browser.click_sync(args["selector"])
        return browser.get_text_sync()[:500]
    return "Unknown tool"
```

---

## Real-World Examples

### 1. Stealth scraping with retry

```python
import asyncio
from onecrawl import Browser, OneCrawlError

async def stealth_scrape(url: str, retries: int = 3) -> dict:
    browser = Browser()
    await browser.launch(headless=True)
    await browser.stealth_inject()
    await browser.stealth_block_domains(["google-analytics.com"])

    for attempt in range(1, retries + 1):
        try:
            await browser.goto(url, wait_until="networkidle")

            captcha = await browser.stealth_detect_captcha()
            if captcha.has_captcha:
                print(f"CAPTCHA detected on attempt {attempt}")
                await browser.wait(5000)
                continue

            title = await browser.get_title()
            text = await browser.get_text(".content")
            await browser.close()
            return {"title": title, "text": text}

        except OneCrawlError as e:
            print(f"Attempt {attempt} failed: {e}")
            if attempt == retries:
                raise
            await browser.wait(2000 * attempt)

    await browser.close()
    raise RuntimeError("All retries exhausted")

result = asyncio.run(stealth_scrape("https://example.com"))
print(result)
```

### 2. Network monitoring with HAR

```python
import asyncio
from onecrawl import Browser

async def monitor_network(url: str, output: str = "trace.har"):
    browser = Browser()
    await browser.launch(headless=True)

    await browser.har_start()
    await browser.goto(url, wait_until="networkidle")

    metrics = await browser.performance_metrics()
    print(f"DOM nodes: {metrics.get('Nodes', 'N/A')}")
    print(f"JS listeners: {metrics.get('JSEventListeners', 'N/A')}")

    await browser.har_stop()
    await browser.har_export(output)
    print(f"HAR exported to {output}")

    await browser.coverage_start()
    await browser.reload()
    coverage = await browser.coverage_stop()
    unused_pct = (coverage.unused_bytes / coverage.total_bytes) * 100
    print(f"Unused code: {unused_pct:.1f}%")

    await browser.close()

asyncio.run(monitor_network("https://example.com"))
```

### 3. Parallel scraping with orchestrator

```python
import asyncio
from onecrawl import Orchestrator

async def parallel_scrape(urls: list[str]) -> list[dict]:
    orchestrator = Orchestrator()

    async def scrape_one(url: str) -> dict:
        browser = await orchestrator.create(headless=True)
        await browser.stealth_inject()
        await browser.goto(url, wait_until="networkidle")
        title = await browser.get_title()
        text = await browser.get_text()
        return {"url": url, "title": title, "text_length": len(text)}

    results = await asyncio.gather(*[scrape_one(url) for url in urls])
    await orchestrator.stop_all()
    return list(results)

data = asyncio.run(parallel_scrape([
    "https://example.com",
    "https://example.com/about",
    "https://example.com/contact",
]))
print(data)
```

### 4. Accessibility audit

```python
import asyncio
import json
from onecrawl import Browser

async def accessibility_audit(urls: list[str]):
    browser = Browser()
    await browser.launch(headless=True)

    for url in urls:
        await browser.goto(url, wait_until="networkidle")

        audit = await browser.accessibility_audit()
        print(f"\n{'='*60}")
        print(f"URL: {url}")
        print(f"Score: {audit.score}")
        print(f"Violations: {len(audit.violations)}")

        for violation in audit.violations:
            print(f"  ⚠️  {violation.id}: {violation.description}")

    await browser.close()

asyncio.run(accessibility_audit([
    "https://example.com",
    "https://example.com/about",
]))
```
