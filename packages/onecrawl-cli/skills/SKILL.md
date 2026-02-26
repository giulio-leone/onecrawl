---
name: onecrawl-cli
description: Automates browser interactions with protocol-level anti-detection stealth for web testing, form filling, screenshots, and data extraction. Passes CloudFlare, CreepJS, BrowserScan, and all major bot detectors. Use when the user needs to navigate websites, interact with web pages, fill forms, take screenshots, test web applications, or extract information from web pages while avoiding bot detection.
allowed-tools: Bash(onecrawl-cli:*)
---

# Browser Automation with onecrawl-cli (Stealth Mode)

OneCrawl CLI wraps Microsoft's playwright-cli with **two layers** of anti-detection stealth,
applied at runtime via `Module._load` hooks (zero files modified on disk).

## Layer 1: JavaScript injection (stealth.js + stealth-init.js)

1. `navigator.webdriver = false`
2. Chrome runtime + plugins spoofing
3. Languages/locale (it-IT)
4. WebGL vendor/renderer spoofing (Intel)
5. Permissions API normalization
6. Console.debug leak prevention
7. HeadlessChrome UA fix
8. Window outer dimensions fix
9. CDP User-Agent override

## Layer 2: Protocol-level patches (stealth-loader.js + patches/)

Rebrowser-derived patches applied to playwright-core internals at runtime:

1. **Runtime.enable bypass** — `CRSession.send` intercepts `Runtime.enable` CDP calls and returns
   `Promise.resolve()`, preventing the browser from exposing automation context IDs
2. **Lazy execution context creation** — `Frame._context()` creates execution contexts on-demand
   via `Page.createIsolatedWorld` + `Runtime.addBinding` instead of relying on `Runtime.enable` events
3. **utilityWorldName fix** — passes dynamic `__playwright_utility_world_${guid}` name through the
   lazy context chain (critical for playwright 1.59+ compatibility)
4. **Worker extension** — adds `_targetId`, `_session`, `getExecutionContext()` to Worker prototype
5. **PageBinding guard** — prevents crashes from non-JSON `Runtime.bindingCalled` payloads

## Layer 3: Ghost cursor (opt-in)

Bezier-curve mouse movement simulation injected as browser initScript:
- Overrides `Element.prototype.click` with human-like mouse path
- 2-5 random control points, natural jitter, ease-in-out speed
- Enable: `GHOST_CURSOR_ENABLED=true`

## Anti-bot test results (verified)

| Test | Result |
|------|--------|
| bot.sannysoft.com | ✅ 10/10 |
| CreepJS | ✅ Fingerprint generated |
| Cloudflare (nowsecure.nl) | ✅ Passed |
| BrowserScan bot detection | ✅ 8/8 |
| Advanced fingerprint (28 checks) | ✅ 28/28 |

## Quick start

```bash
# open new browser (stealth patches applied automatically)
onecrawl-cli open
# navigate to a page
onecrawl-cli goto https://example.com
# interact with the page using refs from the snapshot
onecrawl-cli click e15
onecrawl-cli type "search query"
onecrawl-cli press Enter
# take a screenshot
onecrawl-cli screenshot
# close the browser
onecrawl-cli close
```

## Commands

### Core

```bash
onecrawl-cli open
onecrawl-cli open https://example.com/
onecrawl-cli goto https://example.com
onecrawl-cli type "search query"
onecrawl-cli click e3
onecrawl-cli dblclick e7
onecrawl-cli fill e5 "user@example.com"
onecrawl-cli drag e2 e8
onecrawl-cli hover e4
onecrawl-cli select e9 "option-value"
onecrawl-cli upload ./document.pdf
onecrawl-cli check e12
onecrawl-cli uncheck e12
onecrawl-cli snapshot
onecrawl-cli snapshot --filename=after-click.yaml
onecrawl-cli eval "document.title"
onecrawl-cli eval "el => el.textContent" e5
onecrawl-cli dialog-accept
onecrawl-cli dialog-dismiss
onecrawl-cli resize 1920 1080
onecrawl-cli close
```

### Navigation

```bash
onecrawl-cli go-back
onecrawl-cli go-forward
onecrawl-cli reload
```

### Keyboard

```bash
onecrawl-cli press Enter
onecrawl-cli press ArrowDown
onecrawl-cli keydown Shift
onecrawl-cli keyup Shift
```

### Mouse

```bash
onecrawl-cli mousemove 150 300
onecrawl-cli mousedown
onecrawl-cli mouseup
onecrawl-cli mousewheel 0 100
```

### Save as

```bash
onecrawl-cli screenshot
onecrawl-cli screenshot e5
onecrawl-cli screenshot --filename=page.png
onecrawl-cli pdf --filename=page.pdf
```

### Tabs

```bash
onecrawl-cli tab-list
onecrawl-cli tab-new
onecrawl-cli tab-new https://example.com/page
onecrawl-cli tab-close
onecrawl-cli tab-select 0
```

### Storage

```bash
onecrawl-cli state-save
onecrawl-cli state-save auth.json
onecrawl-cli state-load auth.json
onecrawl-cli cookie-list
onecrawl-cli cookie-get session_id
onecrawl-cli cookie-set session_id abc123
onecrawl-cli cookie-delete session_id
onecrawl-cli cookie-clear
onecrawl-cli localstorage-list
onecrawl-cli localstorage-get theme
onecrawl-cli localstorage-set theme dark
onecrawl-cli localstorage-delete theme
onecrawl-cli localstorage-clear
```

### Network

```bash
onecrawl-cli route "**/*.jpg" --status=404
onecrawl-cli route "https://api.example.com/**" --body='{"mock": true}'
onecrawl-cli route-list
onecrawl-cli unroute "**/*.jpg"
```

### DevTools

```bash
onecrawl-cli console
onecrawl-cli network
onecrawl-cli run-code "async page => await page.context().grantPermissions(['geolocation'])"
onecrawl-cli tracing-start
onecrawl-cli tracing-stop
onecrawl-cli video-start
onecrawl-cli video-stop video.webm
```

## Open parameters

```bash
onecrawl-cli open --browser=chrome
onecrawl-cli open --browser=firefox
onecrawl-cli open --persistent
onecrawl-cli open --profile=/path/to/profile
onecrawl-cli open --config=my-config.json
onecrawl-cli close
onecrawl-cli delete-data
```

## Browser Sessions

```bash
onecrawl-cli -s=mysession open example.com --persistent
onecrawl-cli -s=mysession click e6
onecrawl-cli -s=mysession close
onecrawl-cli list
onecrawl-cli close-all
onecrawl-cli kill-all
```

## Example: Stealth form submission

```bash
onecrawl-cli open https://example.com/form
onecrawl-cli snapshot
onecrawl-cli fill e1 "user@example.com"
onecrawl-cli fill e2 "password123"
onecrawl-cli click e3
onecrawl-cli snapshot
onecrawl-cli close
```
