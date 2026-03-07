---
sidebar_position: 5
title: Node.js SDK
---

# Node.js SDK

Native Rust bindings for Node.js via NAPI-RS. Zero-copy where possible, full async/await support, **391 exported methods** across 5 core classes and standalone functions.

## Installation

```bash
npm install @onecrawl/native
```

**Prebuilt binaries** available for:
- Linux (x64, ARM64)
- macOS (x64, Apple Silicon)
- Windows (x64)

No Rust toolchain required for installation.

---

## TypeScript Support

The SDK ships with full TypeScript definitions:

```typescript
import {
  NativeBrowser,
  NativeOrchestrator,
  NativePlugins,
  NativeStudio,
  NativeStore,
  type LaunchOptions,
  type ScreenshotOptions,
  type CookieParam,
  type Viewport,
  type HarEntry,
  type CoverageReport,
  type AccessibilityNode,
  type StealthTestResult,
  type PasskeyCredential,
} from "@onecrawl/native";
```

---

## Class Reference

### `NativeBrowser`

The primary class for browser automation. **~180 methods.**

#### Lifecycle

```javascript
const { NativeBrowser } = require("@onecrawl/native");

// Launch a new browser
const browser = new NativeBrowser();
await browser.launch({ headless: true });

// Or connect to an existing Chrome DevTools instance
const browser2 = new NativeBrowser();
await browser2.connect("ws://localhost:9222/devtools/browser/...");

// Close when done
await browser.close();
```

##### Launch Options

| Option | Type | Default | Description |
|---|---|---|---|
| `headless` | `boolean` | `true` | Run without a visible window |
| `executablePath` | `string?` | auto-detect | Path to Chrome/Chromium binary |
| `userDataDir` | `string?` | temp dir | Path to user data directory |
| `args` | `string[]?` | `[]` | Extra Chrome launch arguments |
| `proxy` | `string?` | `null` | Proxy server URL |
| `timeout` | `number?` | `30000` | Default timeout in ms |
| `slowMo` | `number?` | `0` | Slow down operations by this many ms |

#### Navigation

```javascript
// Navigate to a URL
await browser.goto("https://example.com");
await browser.goto("https://example.com", { waitUntil: "networkidle" });

// History navigation
await browser.back();
await browser.forward();
await browser.reload();
await browser.reload({ hard: true });

// Get current page info
const url = await browser.getUrl();
const title = await browser.getTitle();
```

#### Screenshots & PDF

```javascript
// Basic screenshot
await browser.screenshot({ path: "page.png" });

// Full-page screenshot
await browser.screenshot({ path: "full.png", fullPage: true });

// Element screenshot
await browser.screenshot({ path: "hero.png", selector: "#hero" });

// JPEG with quality
await browser.screenshot({ path: "page.jpg", format: "jpeg", quality: 80 });

// Get screenshot as base64 (no file written)
const base64 = await browser.screenshot({ encoding: "base64" });

// Get screenshot as Buffer
const buffer = await browser.screenshot({ encoding: "buffer" });

// PDF export
await browser.pdf({ path: "page.pdf" });
await browser.pdf({ path: "report.pdf", landscape: true, scale: 0.8 });
```

#### DOM Interaction

```javascript
// Click
await browser.click("#submit-btn");
await browser.dblclick("#item");
await browser.click("#menu", { button: "right" });

// Type (keystroke simulation with optional delay)
await browser.type("#search", "OneCrawl", { delay: 50 });

// Fill (set value directly — faster)
await browser.fill("#email", "user@example.com");

// Focus and hover
await browser.focus("#input");
await browser.hover(".menu-item");

// Checkbox and select
await browser.check("#agree");
await browser.uncheck("#newsletter");
await browser.selectOption("#country", "IT");

// File upload
await browser.upload("#file-input", "/path/to/file.pdf");

// Scroll
await browser.scrollIntoView("#footer");
await browser.scrollTo(0, 500);

// Bounding box
const box = await browser.boundingBox("#element");
// { x: 100, y: 200, width: 300, height: 150 }

// Drag and drop
await browser.drag("#card", "#column-done");

// Get text content
const text = await browser.getText();
const elText = await browser.getText("#specific-element");

// Get HTML
const html = await browser.getHtml();
const innerHtml = await browser.getHtml("#container");

// Evaluate JavaScript
const count = await browser.evaluate("document.querySelectorAll('a').length");
const data = await browser.evaluate("JSON.stringify(window.__DATA__)");

// Set page content
await browser.setContent("<h1>Hello</h1><p>World</p>");
```

#### Wait Operations

```javascript
// Wait for a fixed duration
await browser.wait(2000);

// Wait for an element to appear
await browser.waitForSelector(".loaded");
await browser.waitForSelector(".modal", { timeout: 10000 });

// Wait for element to be hidden
await browser.waitForSelector(".spinner", { state: "hidden" });

// Wait for URL to match
await browser.waitForUrl("**/dashboard**");

// Wait for network idle
await browser.waitForNetworkIdle({ idleTime: 500 });
```

#### Keyboard

```javascript
await browser.pressKey("Enter");
await browser.pressKey("Tab");
await browser.keyboardShortcut("Ctrl+A");
await browser.keyDown("Shift");
await browser.keyUp("Shift");
```

#### Cookies

```javascript
// Get all cookies
const cookies = await browser.getCookies();

// Get a specific cookie
const session = await browser.getCookie("session_id");

// Set a cookie
await browser.setCookie({
  name: "token",
  value: "abc123",
  domain: ".example.com",
  path: "/",
  httpOnly: true,
  secure: true,
});

// Delete a specific cookie
await browser.deleteCookie("token");

// Clear all cookies
await browser.clearCookies();
```

#### Network

```javascript
// Throttle network (simulate 3G)
await browser.throttle({
  downloadThroughput: 750 * 1024,
  uploadThroughput: 250 * 1024,
  latency: 100,
});

// Intercept requests
await browser.intercept("**/*.png", (req) => {
  req.abort(); // Block images
});

// Block domains
await browser.blockDomains(["google-analytics.com", "facebook.net"]);

// Go offline / online
await browser.setOffline(true);
await browser.setOffline(false);

// HAR recording
await browser.harStart();
await browser.goto("https://example.com");
const har = await browser.harStop();
await browser.harExport("trace.har");

// WebSocket
await browser.wsConnect("wss://stream.example.com");
await browser.wsSend('{"subscribe": "prices"}');
await browser.wsClose();
```

#### Coverage & Performance

```javascript
// Code coverage
await browser.coverageStart();
await browser.goto("https://example.com");
const report = await browser.coverageStop();
console.log(`Unused: ${report.unusedBytes} / ${report.totalBytes}`);

// Performance metrics
const metrics = await browser.performanceMetrics();
// { Timestamp, Documents, Frames, JSEventListeners, Nodes, LayoutObjects, ... }

// Performance tracing
await browser.traceStart();
await browser.goto("https://example.com");
const trace = await browser.traceStop();

// Benchmarking
const result = await browser.bench("goto https://example.com");
```

#### Stealth Mode

```javascript
// Inject stealth patches (call before navigation)
await browser.stealthInject();

// Run detection tests
const results = await browser.stealthTest("https://bot.sannysoft.com");
console.log(results.detected); // false

// Randomize fingerprint
await browser.stealthFingerprint({ randomize: true });

// Block tracking domains
await browser.stealthBlockDomains(["google-analytics.com", "doubleclick.net"]);

// Detect CAPTCHA
const captcha = await browser.stealthDetectCaptcha();
console.log(captcha.hasCaptcha); // true/false
```

#### WebAuthn / Passkey

```javascript
// Enable the virtual authenticator
await browser.passkeyEnable();

// Add a credential
await browser.passkeyAdd({
  rpId: "example.com",
  credentialId: "cred_abc",
  userHandle: "user_123",
  privateKey: "MIIEvQIBADANBgkq...",
});

// List credentials
const creds = await browser.passkeyList();

// View event log
const log = await browser.passkeyLog();

// Clean up
await browser.passkeyRemove("cred_abc");
await browser.passkeyDisable();
```

#### Emulation

```javascript
// Emulate a device
await browser.emulateDevice("iPhone 15 Pro");

// Set viewport
await browser.emulateViewport({ width: 1920, height: 1080 });

// Override timezone and locale
await browser.emulateTimezone("Europe/Rome");
await browser.emulateLocale("it-IT");

// Override geolocation
await browser.emulateGeolocation({ latitude: 41.9028, longitude: 12.4964 });

// Override media type
await browser.emulateMedia("print");

// CPU throttling
await browser.emulateCpuThrottle(4); // 4x slowdown
```

#### Accessibility

```javascript
// Get accessibility snapshot
const snapshot = await browser.accessibilitySnapshot();
const interactive = await browser.accessibilitySnapshot({ filter: "interactive" });

// Run accessibility audit
const audit = await browser.accessibilityAudit();
console.log(audit.violations);
```

#### Console & Dialog

```javascript
// Capture console messages
await browser.consoleStart();
await browser.goto("https://example.com");
const messages = await browser.consoleMessages();
await browser.consoleStop();

// Handle dialogs
await browser.dialogAccept("Yes");
await browser.dialogDismiss();
```

#### Tabs & iFrames

```javascript
// Tab management
const tabId = await browser.tabOpen("https://example.com");
const tabs = await browser.tabList();
await browser.tabSwitch(tabId);
await browser.tabClose(tabId);

// iFrame navigation
const iframes = await browser.iframeList();
await browser.iframeSwitch(0);
await browser.iframeSwitchMain();
```

---

### `NativeOrchestrator`

Multi-instance browser management. **~45 methods.**

```javascript
const { NativeOrchestrator } = require("@onecrawl/native");

const orchestrator = new NativeOrchestrator();

// Create multiple browser instances
const browser1 = await orchestrator.create({ headless: true });
const browser2 = await orchestrator.create({ headless: true, profile: "stealth" });

// List all instances
const instances = await orchestrator.list();

// Get instance by ID
const instance = await orchestrator.get(browser1.id);

// Stop a specific instance
await orchestrator.stop(browser1.id);

// Stop all instances
await orchestrator.stopAll();

// Pool management
const pool = await orchestrator.createPool(5);
const available = await pool.acquire();
await pool.release(available);
```

---

### `NativePlugins`

Plugin system for extending OneCrawl. **~30 methods.**

```javascript
const { NativePlugins } = require("@onecrawl/native");

const plugins = new NativePlugins();

// Load a plugin
await plugins.load("stealth");
await plugins.load("captcha-solver");

// List loaded plugins
const loaded = plugins.list();

// Configure a plugin
await plugins.configure("stealth", { level: "maximum" });

// Execute a plugin action
const result = await plugins.execute("stealth", "inject");
```

---

### `NativeStudio`

Visual debugging and recording. **~25 methods.**

```javascript
const { NativeStudio } = require("@onecrawl/native");

const studio = new NativeStudio();

// Start recording
await studio.startRecording({ output: "session.json" });

// Replay a recorded session
await studio.replay("session.json");

// Visual diff
const diff = await studio.screenshotDiff("before.png", "after.png");
console.log(diff.changePercentage);
```

---

### `NativeStore`

Encrypted key-value store. **~15 methods.**

```javascript
const { NativeStore } = require("@onecrawl/native");

const store = new NativeStore("/path/to/store");

// CRUD operations
await store.set("api_key", "sk-abc123");
const value = await store.get("api_key");
const exists = await store.has("api_key");
const keys = await store.list("api_");
await store.delete("api_key");
await store.clear();
```

---

## Standalone Functions (~96 exports)

### Crypto

```javascript
const { encrypt, decrypt, deriveKey, generatePkce, generateTotp, verifyTotp } = require("@onecrawl/native");

// AES-256-GCM encryption
const key = deriveKey("my-password", "salt-value");
const encrypted = encrypt("secret data", key);
const decrypted = decrypt(encrypted, key);

// PKCE (OAuth 2.0)
const pkce = generatePkce("S256");
// { codeVerifier: "...", codeChallenge: "...", method: "S256" }

// TOTP
const code = generateTotp("JBSWY3DPEHPK3PXP", { digits: 6, period: 30 });
const valid = verifyTotp("482931", "JBSWY3DPEHPK3PXP", { digits: 6, period: 30 });
```

### Parser

```javascript
const { parseAccessibilityTree, querySelector, extractText, extractLinks } = require("@onecrawl/native");

const html = "<div><h1>Title</h1><a href='/about'>About</a></div>";

const tree = parseAccessibilityTree(html);
const elements = querySelector(html, "h1");
const text = extractText(html);
const links = extractLinks(html, { absolute: true, baseUrl: "https://example.com" });
```

### Server

```javascript
const { startServer, stopServer, getServerInfo } = require("@onecrawl/native");

// Start the HTTP API server programmatically
const server = await startServer({ port: 9867, bind: "127.0.0.1" });
const info = getServerInfo();
// { port: 9867, instances: 0, uptime: 42 }
await stopServer();
```

---

## Error Handling

All errors extend `OneCrawlError`:

```javascript
const { NativeBrowser, OneCrawlError } = require("@onecrawl/native");

try {
  const browser = new NativeBrowser();
  await browser.launch({ headless: true });
  await browser.goto("https://example.com");
  await browser.click("#nonexistent");
} catch (error) {
  if (error instanceof OneCrawlError) {
    switch (error.code) {
      case "ELEMENT_NOT_FOUND":
        console.error("Selector did not match any elements");
        break;
      case "TIMEOUT":
        console.error(`Operation timed out after ${error.timeout}ms`);
        break;
      case "NAVIGATION_ERROR":
        console.error(`Failed to load: ${error.url}`);
        break;
      case "BROWSER_DISCONNECTED":
        console.error("Chrome process terminated unexpectedly");
        break;
      default:
        console.error(`OneCrawl error: ${error.message}`);
    }
  }
} finally {
  await browser?.close();
}
```

### Error Codes

| Code | Description |
|---|---|
| `ELEMENT_NOT_FOUND` | CSS selector matched no elements |
| `TIMEOUT` | Operation exceeded the configured timeout |
| `NAVIGATION_ERROR` | Page failed to load (DNS, SSL, HTTP error) |
| `BROWSER_DISCONNECTED` | Chrome process crashed or was killed |
| `BROWSER_LAUNCH_ERROR` | Chrome failed to start |
| `CRYPTO_ERROR` | Encryption/decryption failure |
| `STORAGE_ERROR` | KV store read/write failure |
| `INVALID_ARGUMENT` | Invalid parameter value |

---

## Real-World Examples

### 1. Scrape with stealth and retry

```javascript
const { NativeBrowser } = require("@onecrawl/native");

async function stealthScrape(url, retries = 3) {
  const browser = new NativeBrowser();
  await browser.launch({ headless: true });
  await browser.stealthInject();
  await browser.stealthBlockDomains(["google-analytics.com"]);

  for (let attempt = 1; attempt <= retries; attempt++) {
    try {
      await browser.goto(url, { waitUntil: "networkidle" });

      const captcha = await browser.stealthDetectCaptcha();
      if (captcha.hasCaptcha) {
        console.warn(`CAPTCHA detected on attempt ${attempt}`);
        await browser.wait(5000);
        continue;
      }

      const title = await browser.getTitle();
      const text = await browser.getText(".content");
      await browser.close();
      return { title, text };
    } catch (error) {
      console.warn(`Attempt ${attempt} failed: ${error.message}`);
      if (attempt === retries) throw error;
      await browser.wait(2000 * attempt);
    }
  }
}

stealthScrape("https://example.com").then(console.log).catch(console.error);
```

### 2. Multi-page form submission

```javascript
const { NativeBrowser } = require("@onecrawl/native");

async function submitApplication(url, data) {
  const browser = new NativeBrowser();
  await browser.launch({ headless: false });

  await browser.goto(url);
  await browser.waitForSelector("#application-form");

  await browser.fill("#first-name", data.firstName);
  await browser.fill("#last-name", data.lastName);
  await browser.fill("#email", data.email);
  await browser.selectOption("#role", data.role);
  await browser.upload("#resume", data.resumePath);
  await browser.check("#terms");

  await browser.click("#submit-btn");
  await browser.waitForUrl("**/confirmation**");

  const confirmation = await browser.getText(".confirmation-number");
  await browser.screenshot({ path: "confirmation.png" });

  await browser.close();
  return confirmation;
}

submitApplication("https://jobs.example.com/apply", {
  firstName: "Giulio",
  lastName: "Leone",
  email: "giulio@example.com",
  role: "senior-engineer",
  resumePath: "./cv-eng.pdf",
}).then(console.log);
```

### 3. Parallel scraping with orchestrator

```javascript
const { NativeOrchestrator } = require("@onecrawl/native");

async function parallelScrape(urls) {
  const orchestrator = new NativeOrchestrator();
  const results = [];

  const browsers = await Promise.all(
    urls.map(() => orchestrator.create({ headless: true }))
  );

  const tasks = urls.map(async (url, i) => {
    const browser = browsers[i];
    await browser.stealthInject();
    await browser.goto(url, { waitUntil: "networkidle" });
    const title = await browser.getTitle();
    const text = await browser.getText();
    return { url, title, textLength: text.length };
  });

  const data = await Promise.all(tasks);
  await orchestrator.stopAll();
  return data;
}

parallelScrape([
  "https://example.com",
  "https://example.com/about",
  "https://example.com/contact",
]).then(console.log);
```
