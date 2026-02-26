---
name: onecrawl-cli
description: Automates browser interactions with protocol-level anti-detection stealth for web testing, form filling, screenshots, and data extraction. Passes CloudFlare, CreepJS, BrowserScan, and all major bot detectors. Use when the user needs to navigate websites, interact with web pages, fill forms, take screenshots, test web applications, or extract information from web pages while avoiding bot detection.
allowed-tools: Bash(onecrawl:*)
---

# Browser Automation with onecrawl (Stealth Mode)

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
onecrawl open
# navigate to a page
onecrawl goto https://example.com
# interact with the page using refs from the snapshot
onecrawl click e15
onecrawl type "search query"
onecrawl press Enter
# take a screenshot
onecrawl screenshot
# close the browser
onecrawl close
```

## Commands

### Core

```bash
onecrawl open
onecrawl open https://example.com/
onecrawl goto https://example.com
onecrawl type "search query"
onecrawl click e3
onecrawl dblclick e7
onecrawl fill e5 "user@example.com"
onecrawl drag e2 e8
onecrawl hover e4
onecrawl select e9 "option-value"
onecrawl upload ./document.pdf
onecrawl check e12
onecrawl uncheck e12
onecrawl snapshot
onecrawl snapshot --filename=after-click.yaml
onecrawl eval "document.title"
onecrawl eval "el => el.textContent" e5
onecrawl dialog-accept
onecrawl dialog-dismiss
onecrawl resize 1920 1080
onecrawl close
```

### Navigation

```bash
onecrawl go-back
onecrawl go-forward
onecrawl reload
```

### Keyboard

```bash
onecrawl press Enter
onecrawl press ArrowDown
onecrawl keydown Shift
onecrawl keyup Shift
```

### Mouse

```bash
onecrawl mousemove 150 300
onecrawl mousedown
onecrawl mouseup
onecrawl mousewheel 0 100
```

### Save as

```bash
onecrawl screenshot
onecrawl screenshot e5
onecrawl screenshot --filename=page.png
onecrawl pdf --filename=page.pdf
```

### Tabs

```bash
onecrawl tab-list
onecrawl tab-new
onecrawl tab-new https://example.com/page
onecrawl tab-close
onecrawl tab-select 0
```

### Storage

```bash
onecrawl state-save
onecrawl state-save auth.json
onecrawl state-load auth.json
onecrawl cookie-list
onecrawl cookie-get session_id
onecrawl cookie-set session_id abc123
onecrawl cookie-delete session_id
onecrawl cookie-clear
onecrawl localstorage-list
onecrawl localstorage-get theme
onecrawl localstorage-set theme dark
onecrawl localstorage-delete theme
onecrawl localstorage-clear
```

### Network

```bash
onecrawl route "**/*.jpg" --status=404
onecrawl route "https://api.example.com/**" --body='{"mock": true}'
onecrawl route-list
onecrawl unroute "**/*.jpg"
```

### DevTools

```bash
onecrawl console
onecrawl network
onecrawl run-code "async page => await page.context().grantPermissions(['geolocation'])"
onecrawl tracing-start
onecrawl tracing-stop
onecrawl video-start
onecrawl video-stop video.webm
```

## Open parameters

```bash
onecrawl open --browser=chrome
onecrawl open --browser=firefox
onecrawl open --persistent
onecrawl open --profile=/path/to/profile
onecrawl open --config=my-config.json
onecrawl close
onecrawl delete-data
```

## Browser Sessions

```bash
onecrawl -s=mysession open example.com --persistent
onecrawl -s=mysession click e6
onecrawl -s=mysession close
onecrawl list
onecrawl close-all
onecrawl kill-all
```

## Example: Stealth form submission

```bash
onecrawl open https://example.com/form
onecrawl snapshot
onecrawl fill e1 "user@example.com"
onecrawl fill e2 "password123"
onecrawl click e3
onecrawl snapshot
onecrawl close
```
