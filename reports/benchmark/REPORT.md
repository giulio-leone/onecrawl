# OneCrawl E2E Benchmark Report

**Date:** 2026-03-08T04:22:43+0100
**Platform:** macos aarch64

## Results

| Metric | Value |
|--------|-------|
| onecrawl_browser_launch_ms | 424 |
| onecrawl_browser_nav_ms | 503 |
| onecrawl_browser_screenshot_ms | 71 |
| onecrawl_browser_screenshot_bytes | 16567 |
| stealth_injection_ms | 1 |
| stealth_checks_passed | 6/6 |
| crypto_encrypt_avg_us | 8406 |
| crypto_decrypt_avg_us | 8383 |
| crypto_pkce_us | 2 |
| crypto_totp_us | 2 |
| parser_a11y_us | 184 |
| parser_query_us | 49 |
| parser_text_us | 24 |
| parser_links_us | 12 |
| storage_write_avg_us | 8474 |
| storage_read_avg_us | 8464 |
| storage_list_us | 52 |

## Screenshots

### Chromiumoxide — example.com
![Chromiumoxide — example.com](onecrawl_browser_example.png)

### Chromiumoxide — Stealth Patched
![Chromiumoxide — Stealth Patched](onecrawl_browser_stealth.png)

### Playwright-rs — Chromium
![Playwright-rs — Chromium](playwright_chromium.png)

### Playwright-rs — Firefox
![Playwright-rs — Firefox](playwright_firefox.png)

### Playwright-rs — WebKit
![Playwright-rs — WebKit](playwright_webkit.png)

