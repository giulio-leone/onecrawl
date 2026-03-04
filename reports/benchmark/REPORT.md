# OneCrawl E2E Benchmark Report

**Date:** 2026-03-04T01:48:27+0100
**Platform:** macos aarch64

## Results

| Metric | Value |
|--------|-------|
| chromiumoxide_launch_ms | 688 |
| chromiumoxide_nav_ms | 65 |
| chromiumoxide_screenshot_ms | 68 |
| chromiumoxide_screenshot_bytes | 16567 |
| stealth_injection_ms | 1 |
| stealth_checks_passed | 6/6 |
| crypto_encrypt_avg_us | 16982 |
| crypto_decrypt_avg_us | 17821 |
| crypto_pkce_us | 5 |
| crypto_totp_us | 6 |
| parser_a11y_us | 384 |
| parser_query_us | 38 |
| parser_text_us | 381 |
| parser_links_us | 40 |
| storage_write_avg_us | 19599 |
| storage_read_avg_us | 24644 |
| storage_list_us | 459 |

## Screenshots

### Chromiumoxide — example.com
![Chromiumoxide — example.com](chromiumoxide_example.png)

### Chromiumoxide — Stealth Patched
![Chromiumoxide — Stealth Patched](chromiumoxide_stealth.png)

### Playwright-rs — Chromium
![Playwright-rs — Chromium](playwright_chromium.png)

### Playwright-rs — Firefox
![Playwright-rs — Firefox](playwright_firefox.png)

### Playwright-rs — WebKit
![Playwright-rs — WebKit](playwright_webkit.png)

