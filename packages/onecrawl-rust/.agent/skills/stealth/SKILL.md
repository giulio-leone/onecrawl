---
name: stealth
description: "Anti-bot bypass with 12 stealth patches, TLS fingerprint impersonation, domain blocking, CAPTCHA detection, and proxy health monitoring for undetectable browser automation."
---

# Stealth & Anti-Detection Skill

OneCrawl's comprehensive anti-bot evasion system. Patches all known browser fingerprint detection vectors, manages proxy health, blocks unwanted domains, and detects CAPTCHAs.

## Modules

| Module | Purpose |
|--------|---------|
| `antibot` | 12 stealth patches (WebDriver, Chrome runtime, WebGL, Canvas, Audio...) |
| `tls_fingerprint` | 6 browser profile presets for fingerprint impersonation |
| `domain_blocker` | Block ads, trackers, social widgets, fonts, media by category |
| `captcha` | CAPTCHA detection framework (reCAPTCHA, hCaptcha, Cloudflare Turnstile) |
| `proxy_health` | Proxy health checking, scoring, and ranking |
| `adaptive` | Element fingerprinting + multi-strategy relocation |

## How It Works

### Anti-Bot Bypass
```bash
# Apply stealth patches (3 levels)
onecrawl antibot inject --level basic       # WebDriver + Chrome runtime + headless
onecrawl antibot inject --level standard    # + plugins, languages, permissions, screen
onecrawl antibot inject --level aggressive  # + WebGL, Canvas, Audio, iframe

# Test bot detection score
onecrawl antibot test                       # Returns score 0-100

# List available profiles
onecrawl antibot profiles
```

### TLS Fingerprint Impersonation
```bash
onecrawl fingerprint list                   # Show 6 browser profiles
onecrawl fingerprint apply chrome-win       # Apply Chrome/Windows fingerprint
onecrawl fingerprint apply safari-mac       # Apply Safari/macOS fingerprint
onecrawl fingerprint random                 # Random realistic fingerprint
onecrawl fingerprint detect                 # Show current fingerprint
```

### Domain Blocking
```bash
onecrawl domain block-category ads          # Block 15+ ad networks
onecrawl domain block-category trackers     # Block 18+ trackers
onecrawl domain block-category social       # Block social widgets
onecrawl domain block-category fonts        # Block web fonts
onecrawl domain block-category media        # Block media embeds
onecrawl domain block example.com custom.io # Block specific domains
onecrawl domain stats                       # Blocking statistics
onecrawl domain list                        # Currently blocked domains
onecrawl domain unblock                     # Clear all blocks
```

### CAPTCHA Detection
```bash
onecrawl captcha detect                     # Detect type + confidence
onecrawl captcha wait                       # Wait for CAPTCHA to appear
onecrawl captcha screenshot                 # Screenshot CAPTCHA element
onecrawl captcha inject "solution-token"    # Inject solver response
onecrawl captcha types                      # Supported CAPTCHA types
```

### Proxy Health
```bash
onecrawl proxy-health check "http://proxy:8080"
onecrawl proxy-health check-all proxies.json
onecrawl proxy-health rank results.json
onecrawl proxy-health filter results.json 70  # min score 70
```

### Adaptive Element Tracking
```bash
onecrawl adaptive fingerprint ".login-btn"
onecrawl adaptive track ".important-el" ".other-el" --save fingerprints.json
onecrawl adaptive relocate fingerprint.json    # Find element after page changes
onecrawl adaptive relocate-all fingerprints.json
```
