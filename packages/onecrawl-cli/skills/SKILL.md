---
name: onecrawl-cli
description: Automates browser interactions with protocol-level anti-detection stealth for web testing, form filling, screenshots, and data extraction. Passes CloudFlare, CreepJS, BrowserScan, and all major bot detectors. Use when the user needs to navigate websites, interact with web pages, fill forms, take screenshots, test web applications, or extract information from web pages while avoiding bot detection.
allowed-tools: Bash(onecrawl:*)
---

# Browser Automation with onecrawl (Stealth Mode)

OneCrawl CLI wraps Microsoft's playwright-cli with **three layers** of anti-detection stealth,
applied at runtime via `Module._load` hooks (zero files modified on disk).
On top of every Playwright built-in command it adds **21 custom commands** designed for
AI-agent workflows: element discovery, browser interaction, data extraction, session management,
property extraction, state checks, assertions, smart scrolling, annotated screenshots,
session diagnostics, and auth management.

> **Backward compatibility**: All standard Playwright CLI commands (`open`, `goto`, `click`,
> `fill`, `snapshot`, `screenshot`, `eval`, `cookie-*`, `tab-*`, etc.) work unchanged.
> The 10 custom commands extend — never replace — the built-in set.

## Stealth Layers

### Layer 1: JavaScript injection (stealth.js + stealth-init.js)

1. `navigator.webdriver = false`
2. Chrome runtime + plugins spoofing
3. Languages/locale (it-IT)
4. WebGL vendor/renderer spoofing (Intel)
5. Permissions API normalization
6. Console.debug leak prevention
7. HeadlessChrome UA fix
8. Window outer dimensions fix
9. CDP User-Agent override

### Layer 2: Protocol-level patches (stealth-loader.js + patches/)

Rebrowser-derived patches applied to playwright-core internals at runtime:

1. **Runtime.enable bypass** — intercepts `Runtime.enable` CDP calls, preventing
   the browser from exposing automation context IDs
2. **Lazy execution context creation** — creates contexts on-demand via
   `Page.createIsolatedWorld` + `Runtime.addBinding`
3. **utilityWorldName fix** — dynamic `__playwright_utility_world_${guid}` name
   (critical for playwright 1.59+ compatibility)
4. **Worker extension** — adds `_targetId`, `_session`, `getExecutionContext()` to Worker
5. **PageBinding guard** — prevents crashes from non-JSON `Runtime.bindingCalled` payloads

### Layer 3: Ghost cursor (opt-in)

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

---

## Custom OneCrawl Commands (21)

### scroll — scroll the page

Scrolls via `mousewheel` in any direction.

```bash
onecrawl scroll down          # 300px default
onecrawl scroll up 500
onecrawl scroll left 200
onecrawl scroll right 150
```

Directions: `up`, `down`, `left`, `right`. Default: 300 pixels.

### find — locate elements by strategy

Returns a JSON array of matching elements with `ref` numbers, tag, text, and visibility.
Assigns `data-oncrawl-ref` attributes for use with `get`, `is`, and `assert`.

```bash
onecrawl find role button
onecrawl find text "Sign in"
onecrawl find label "Email"
onecrawl find placeholder "Search"
onecrawl find testid submit-btn
onecrawl find css ".nav-item.active"
```

Strategies: `role`, `text`, `label`, `placeholder`, `testid`, `css`.

### get — retrieve a property

Returns a single property value to stdout.

```bash
# Page-level (no ref)
onecrawl get url
onecrawl get title

# Element-level (requires ref from find)
onecrawl get text 1
onecrawl get html 3
onecrawl get value 2
onecrawl get attr:href 1
```

Properties: `text`, `html`, `url`, `title`, `value`, `attr:<name>`.

### is — check element state

Returns `true` / `false` to stdout. Exit code 0 = true, 1 = false.

```bash
onecrawl is visible 1
onecrawl is hidden 2
onecrawl is enabled 3
onecrawl is disabled 4
onecrawl is checked 5
onecrawl is editable 6
```

States: `visible`, `hidden`, `enabled`, `disabled`, `checked`, `editable`.

### wait-for — wait for a condition

Blocks until a condition is met or timeout (default 30 000 ms). Exit 0 = success, 1 = timeout.

```bash
onecrawl wait-for load
onecrawl wait-for networkidle
onecrawl wait-for selector:.results-list
onecrawl wait-for text:Success
onecrawl wait-for url:dashboard 10000
```

Targets: `selector:<css>`, `text:<string>`, `url:<regex>`, `load`, `networkidle`.

### assert — assert conditions

Exit 0 = pass, 1 = fail (descriptive message on stderr).

```bash
onecrawl assert visible 1
onecrawl assert hidden 2
onecrawl assert text 1 "Welcome"
onecrawl assert url "dashboard"
onecrawl assert title "My App"
onecrawl assert count ".item" 5
```

Conditions: `visible`, `hidden`, `text`, `url`, `title`, `count`.

### screenshot-annotate — annotated screenshot

Injects numbered red labels on all interactive elements (buttons, links, inputs),
takes a screenshot, then cleans up. Outputs a JSON mapping of numbers → element metadata.

```bash
onecrawl screenshot-annotate
onecrawl screenshot-annotate page-map.png
```

### session-info — session diagnostics

Returns JSON with browser version, viewport, current URL/title, cookie count,
stealth status, and session age.

```bash
onecrawl session-info
```

### health-check — full diagnostic probe

Checks browser connectivity, page responsiveness, cookies (including `li_at` expiry),
passkey availability, and stealth status. Exit 0 = healthy, 1 = critical failure.

```bash
onecrawl health-check
```

### auth — LinkedIn authentication management

Full auth lifecycle: status, login (passkey or cookie injection), passkey registration,
credential export/import. Supports dual-layer authentication (passkey + cookie).

```bash
onecrawl auth status
onecrawl auth login
onecrawl auth login --method=cookie
onecrawl auth login --method=passkey
onecrawl auth register-passkey
onecrawl auth export
onecrawl auth export creds.json
onecrawl auth import creds.json
```

**Auth cascade** (when `--method=auto`, the default):
1. Try cookie injection from `~/.onecrawl/linkedin/cookies.json`
2. Fall back to passkey from `~/.onecrawl/linkedin/passkey.json`
3. Fail with instructions if neither exists

### click — click an element

Click by ref number (from `find`) or CSS selector.

```bash
onecrawl click 1                 # click ref 1
onecrawl click ".submit-btn"     # click by CSS
onecrawl click 2 --right         # right-click
onecrawl click 3 --double        # double-click
onecrawl click 4 --force         # force click (bypass visibility)
```

### type — type into an element

Type text into a focused or targeted element.

```bash
onecrawl type 1 "Hello world"           # type into ref 1
onecrawl type ".search-input" "query"    # type by CSS
onecrawl type 2 "new value" --clear      # clear field first, then type
onecrawl type 1 "slow" --delay=100       # human-like typing (ms per char)
```

### select — select dropdown option

Select a dropdown value by ref or CSS selector.

```bash
onecrawl select 1 "option-value"            # by value (default)
onecrawl select ".dropdown" "Italy" --by=label   # by visible label
onecrawl select 2 "3" --by=index            # by index
```

### hover — hover over element

Hover to trigger tooltips, dropdowns, or hover states.

```bash
onecrawl hover 1
onecrawl hover ".tooltip-trigger"
```

### drag — drag and drop

Drag from one element to another.

```bash
onecrawl drag 1 2                           # drag ref 1 to ref 2
onecrawl drag ".item" ".dropzone"           # by CSS selectors
```

### extract — structured data extraction

Extract structured data from repeated elements on the page.

```bash
onecrawl extract --selector=".job-card"                          # auto-extract all data
onecrawl extract --selector=".job-card" --fields=title,company   # specific fields
onecrawl extract                                                 # extract from full page
```

Output: JSON array of objects with extracted data.

### table — extract HTML tables

Extract `<table>` data as JSON or CSV.

```bash
onecrawl table                        # first table, JSON format
onecrawl table ".results-table"       # specific table
onecrawl table --format=csv           # CSV output
```

### links — extract page links

List all links on the page with filtering.

```bash
onecrawl links                        # all links
onecrawl links --external             # only external links
onecrawl links --internal             # only same-origin links
onecrawl links --filter="linkedin"    # regex filter on href
```

Output: JSON array of `{href, text, rel, target, external}`.

### forms — list page forms

Describe all forms and their fields.

```bash
onecrawl forms                        # all forms
onecrawl forms --selector="#login"    # specific form
```

Output: JSON array with form action, method, and fields (name, type, required, value, placeholder, options).

### session — session state management

Save, restore, and manage named browser sessions.

```bash
onecrawl session list                 # list saved sessions
onecrawl session save my-linkedin     # save current state
onecrawl session restore my-linkedin  # restore saved state
onecrawl session delete my-linkedin   # delete saved session
onecrawl session clone backup         # clone current session
```

Saves full state (URL, cookies, localStorage, viewport) to `~/.onecrawl/sessions/<name>.json`.

### cookie — cookie management

List, export, import, and clear cookies.

```bash
onecrawl cookie list                          # all cookies
onecrawl cookie list --domain=linkedin.com    # filter by domain
onecrawl cookie export cookies.json           # export to file
onecrawl cookie import cookies.json           # import from file
onecrawl cookie clear                         # clear all
onecrawl cookie clear --domain=linkedin.com   # clear specific domain
```

---

## Built-in Playwright Commands (unchanged)

### Core

```bash
onecrawl open [url] [--browser=chrome|firefox] [--persistent] [--profile=<path>]
onecrawl goto <url>
onecrawl click <ref>
onecrawl dblclick <ref>
onecrawl fill <ref> "<value>"
onecrawl type "<text>"
onecrawl drag <from-ref> <to-ref>
onecrawl hover <ref>
onecrawl select <ref> "<value>"
onecrawl upload <file>
onecrawl check <ref> / uncheck <ref>
onecrawl snapshot [--filename=<file>]
onecrawl eval "<js>" [ref]
onecrawl dialog-accept / dialog-dismiss
onecrawl resize <w> <h>
onecrawl close
```

### Navigation

```bash
onecrawl go-back / go-forward / reload
```

### Keyboard & Mouse

```bash
onecrawl press <Key>
onecrawl keydown <Key> / keyup <Key>
onecrawl mousemove <x> <y>
onecrawl mousedown / mouseup
onecrawl mousewheel <dx> <dy>
```

### Screenshots & Recording

```bash
onecrawl screenshot [ref] [--filename=<file>]
onecrawl pdf --filename=<file>
onecrawl tracing-start / tracing-stop
onecrawl video-start / video-stop <file>
```

### Tabs

```bash
onecrawl tab-list / tab-new [url] / tab-close / tab-select <index>
```

### Storage

```bash
onecrawl state-save [file] / state-load <file>
onecrawl cookie-list / cookie-get <name> / cookie-set <name> <val> / cookie-delete <name> / cookie-clear
onecrawl localstorage-list / localstorage-get <key> / localstorage-set <key> <val> / localstorage-delete <key> / localstorage-clear
```

### Network

```bash
onecrawl route "<pattern>" [--status=<n>] [--body='<json>']
onecrawl route-list / unroute "<pattern>"
```

### DevTools

```bash
onecrawl console / network
onecrawl run-code "<async page => ...>"
```

### Sessions

```bash
onecrawl -s=<name> <command>      # named session
onecrawl list / close-all / kill-all
```

---

## AI Agent Usage Patterns

### Data extraction pipeline

```bash
onecrawl goto https://example.com
onecrawl find css ".job-card"
# → [{ ref: 1, tag: "div", text: "Senior Engineer..." }, ...]
onecrawl get text 1
onecrawl get attr:href 1
```

### Form validation loop

```bash
onecrawl find label "Email"
onecrawl fill e1 "user@example.com"
onecrawl is editable 1              # verify field is editable
onecrawl assert text 1 "user@example.com"  # verify value set
```

### Wait-then-act pattern

```bash
onecrawl goto https://example.com/dashboard
onecrawl wait-for selector:.data-loaded
onecrawl screenshot-annotate
# → JSON map of all interactive elements
onecrawl click e3
```

### Health-first session

```bash
onecrawl open --persistent
onecrawl health-check              # verify stealth + browser
onecrawl auth login                # inject stored credentials
onecrawl goto https://linkedin.com
onecrawl wait-for url:feed
onecrawl session-info              # confirm auth state
```

### Stealth form submission

```bash
onecrawl open https://example.com/form
onecrawl snapshot
onecrawl fill e1 "user@example.com"
onecrawl fill e2 "password123"
onecrawl click e3
onecrawl wait-for text:Success
onecrawl assert url "dashboard"
onecrawl close
```
