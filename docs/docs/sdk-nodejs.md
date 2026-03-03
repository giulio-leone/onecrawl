---
sidebar_position: 5
title: Node.js SDK
---

# Node.js SDK

Native Rust bindings for Node.js via NAPI-RS. Zero-copy where possible, full async/await support, ~130 methods.

## Installation

```bash
npm install @onecrawl/native
```

## Browser Lifecycle

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

### Launch Options

| Option | Type | Default | Description |
|---|---|---|---|
| `headless` | `boolean` | `true` | Run without a visible window |
| `executablePath` | `string?` | auto-detect | Path to Chrome/Chromium binary |
| `userDataDir` | `string?` | temp dir | Path to user data directory |
| `args` | `string[]?` | `[]` | Extra Chrome launch arguments |
| `proxy` | `string?` | `null` | Proxy server URL |

---

## Navigation

```javascript
// Navigate to a URL
await browser.goto("https://example.com");
await browser.goto("https://example.com", { waitUntil: "networkidle" });

// History navigation
await browser.back();
await browser.forward();
await browser.reload();

// Get current URL and title
const url = await browser.getUrl();
const title = await browser.getTitle();
```

---

## Screenshots & PDF

```javascript
// Basic screenshot
await browser.screenshot({ path: "page.png" });

// Full-page screenshot
await browser.screenshot({ path: "full.png", fullPage: true });

// Element screenshot
await browser.screenshot({ path: "hero.png", selector: "#hero" });

// JPEG with quality
await browser.screenshot({ path: "page.jpg", format: "jpeg", quality: 80 });

// Get screenshot as base64 (no file)
const base64 = await browser.screenshot({ encoding: "base64" });

// PDF export
await browser.pdf({ path: "page.pdf" });
await browser.pdf({ path: "report.pdf", landscape: true, scale: 0.8 });
```

---

## DOM Interaction

```javascript
// Click
await browser.click("#submit-btn");
await browser.dblclick("#item");

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

// Scroll into view
await browser.scrollIntoView("#footer");

// Bounding box
const box = await browser.boundingBox("#element");
// { x: 100, y: 200, width: 300, height: 150 }

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

---

## Wait Operations

```javascript
// Wait for a fixed duration
await browser.wait(2000);

// Wait for an element to appear
await browser.waitForSelector(".loaded");
await browser.waitForSelector(".modal", { timeout: 10000 });

// Wait for URL to match
await browser.waitForUrl("**/dashboard**");
```

---

## Cookies

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

---

## Network

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
await browser.blockDomains([
  "google-analytics.com",
  "facebook.net",
]);

// Go offline / online
await browser.setOffline(true);
await browser.setOffline(false);

// HAR recording
await browser.harStart();
await browser.goto("https://example.com");
const har = await browser.harStop();
await browser.harExport("trace.har");
```

---

## Coverage & Performance

```javascript
// Code coverage
await browser.coverageStart();
await browser.goto("https://example.com");
const report = await browser.coverageStop();
console.log(report.unusedBytes, report.totalBytes);

// Performance metrics
const metrics = await browser.performanceMetrics();
// { Timestamp, Documents, Frames, JSEventListeners, LayoutObjects, ... }

// Performance tracing
await browser.traceStart();
await browser.goto("https://example.com");
const trace = await browser.traceStop();
```

---

## Stealth Mode

```javascript
// Inject stealth patches (call before navigation)
await browser.stealthInject();

// Run detection tests
const results = await browser.stealthTest("https://bot.sannysoft.com");
console.log(results.detected); // false

// Randomize fingerprint
await browser.stealthFingerprint({ randomize: true });

// Block tracking domains
await browser.stealthBlockDomains([
  "google-analytics.com",
  "doubleclick.net",
]);

// Detect CAPTCHA
const captcha = await browser.stealthDetectCaptcha();
console.log(captcha.hasCaptcha); // true/false
```

---

## WebAuthn / Passkey

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

---

## Emulation

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
```

---

## Standalone Functions

### Crypto

```javascript
const {
  encrypt,
  decrypt,
  deriveKey,
  generatePkce,
  generateTotp,
  verifyTotp,
} = require("@onecrawl/native");

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
const {
  parseAccessibilityTree,
  querySelector,
  extractText,
  extractLinks,
} = require("@onecrawl/native");

const html = "<div><h1>Title</h1><a href='/about'>About</a></div>";

const tree = parseAccessibilityTree(html);
const elements = querySelector(html, "h1");
const text = extractText(html);
const links = extractLinks(html, { absolute: true, baseUrl: "https://example.com" });
```

### Server

```javascript
const { startServer, getServerInfo } = require("@onecrawl/native");

// Start the HTTP API server programmatically
const server = await startServer({ port: 9867, bind: "127.0.0.1" });
const info = getServerInfo();
// { port: 9867, instances: 0, uptime: 42 }
```

### NativeStore (Encrypted KV)

```javascript
const { NativeStore } = require("@onecrawl/native");

const store = new NativeStore("/path/to/store");
await store.set("api_key", "sk-abc123");
const value = await store.get("api_key");
const keys = await store.list("api_");
await store.delete("api_key");
```

---

## Real-World Examples

### 1. Scrape a page with stealth mode

```javascript
const { NativeBrowser } = require("@onecrawl/native");

async function stealthScrape(url) {
  const browser = new NativeBrowser();
  await browser.launch({ headless: true });
  await browser.stealthInject();
  await browser.stealthBlockDomains(["google-analytics.com"]);

  await browser.goto(url, { waitUntil: "networkidle" });
  await browser.waitForSelector(".content");

  const title = await browser.getTitle();
  const text = await browser.getText(".content");
  const links = await browser.evaluate(`
    JSON.stringify(
      Array.from(document.querySelectorAll("a"))
        .map(a => ({ text: a.textContent.trim(), href: a.href }))
    )
  `);

  await browser.close();
  return { title, text, links: JSON.parse(links) };
}

stealthScrape("https://example.com").then(console.log);
```

### 2. Fill and submit a form

```javascript
const { NativeBrowser } = require("@onecrawl/native");

async function submitApplication(url, data) {
  const browser = new NativeBrowser();
  await browser.launch({ headless: false }); // headed for debugging

  await browser.goto(url);
  await browser.waitForSelector("#application-form");

  // Fill the form fields
  await browser.fill("#first-name", data.firstName);
  await browser.fill("#last-name", data.lastName);
  await browser.fill("#email", data.email);
  await browser.selectOption("#role", data.role);
  await browser.upload("#resume", data.resumePath);
  await browser.check("#terms");

  // Submit
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

### 3. Take annotated screenshots

```javascript
const { NativeBrowser } = require("@onecrawl/native");

async function annotatedScreenshots(urls) {
  const browser = new NativeBrowser();
  await browser.launch({ headless: true });

  for (const [i, url] of urls.entries()) {
    await browser.goto(url, { waitUntil: "networkidle" });

    // Inject annotation
    await browser.evaluate(`
      const banner = document.createElement('div');
      banner.style.cssText = 'position:fixed;top:0;left:0;right:0;background:#1a1a2e;color:#e94560;padding:8px 16px;z-index:99999;font:14px monospace';
      banner.textContent = '${url} — captured ${new Date().toISOString()}';
      document.body.prepend(banner);
    `);

    await browser.screenshot({
      path: `screenshots/page-${i + 1}.png`,
      fullPage: true,
    });
  }

  await browser.close();
}

annotatedScreenshots([
  "https://example.com",
  "https://example.com/about",
  "https://example.com/contact",
]);
```
