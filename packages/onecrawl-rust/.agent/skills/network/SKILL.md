---
name: network
description: "Network interception, throttling, HAR recording, WebSocket capture, network logging, domain blocking, proxy management, and request queuing."
---

# Network Skill

Full network control: intercept requests, throttle bandwidth, record HAR files,
capture WebSocket frames, block domains, manage proxies, and queue requests.

## Modules

| Module | Purpose |
|--------|---------|
| `throttle` | Network throttling presets and custom profiles |
| `har` | HAR 1.2 recording, drain, export |
| `websocket` | WebSocket frame interception and export |
| `network-log` | Request/response logging with summary stats |
| `intercept` | Request interception with pattern matching |
| `domain` | Domain blocking by name or category |
| `proxy` | Proxy pool management and health checking |
| `request` | HTTP request execution and batch processing |

## CLI Commands

### Network Throttling

```bash
onecrawl throttle set fast3g        # Fast 3G preset
onecrawl throttle set slow3g        # Slow 3G preset
onecrawl throttle set wifi          # WiFi preset
onecrawl throttle set regular4g     # Regular 4G preset
onecrawl throttle set offline       # Offline mode

onecrawl throttle custom 500 100 200   # 500kbps down, 100kbps up, 200ms latency
onecrawl throttle clear                # Remove throttling
```

### Resource Blocking

```bash
onecrawl network block image,stylesheet,font   # Block resource types
```

### HAR Recording

```bash
onecrawl har start                    # Start recording
onecrawl har drain                    # Get new entries
onecrawl har export --output recording.har   # Export HAR 1.2
```

### WebSocket Capture

```bash
onecrawl ws start                     # Start interception
onecrawl ws drain                     # Get captured frames
onecrawl ws connections               # Active connection count
onecrawl ws export --output frames.json
```

### Network Logging

```bash
onecrawl network-log start            # Start logging
onecrawl network-log drain            # Drain entries (JSON)
onecrawl network-log summary          # Statistics
onecrawl network-log stop             # Stop logging
onecrawl network-log export log.json  # Export to file
```

### Request Interception

```bash
# Set interception rules
onecrawl intercept set '[{"pattern": "*.analytics.com/*", "action": "block"}]'
onecrawl intercept log                # View intercepted requests
onecrawl intercept clear              # Clear rules
```

### Domain Blocking

```bash
onecrawl domain block ads.example.com tracker.example.com
onecrawl domain block-category ads        # Built-in category
onecrawl domain block-category trackers
onecrawl domain block-category social
onecrawl domain block-category fonts
onecrawl domain block-category media
onecrawl domain list                      # Currently blocked
onecrawl domain stats                     # Blocking stats
onecrawl domain categories                # Available categories
onecrawl domain unblock                   # Remove all blocks
```

### Proxy Management

```bash
# Create proxy pool
onecrawl proxy create-pool '{"proxies": ["http://proxy1:8080", "socks5://proxy2:1080"]}'

# Get Chrome launch args for proxy
onecrawl proxy chrome-args '{"proxies": [...]}'

# Rotate to next proxy
onecrawl proxy next '{"proxies": [...]}'

# Health check single proxy
onecrawl proxy-health check "http://proxy:8080" --timeout 5000

# Check all proxies
onecrawl proxy-health check-all '["http://p1:8080", "http://p2:8080"]'

# Rank by health score
onecrawl proxy-health rank '<results_json>'

# Filter by minimum score
onecrawl proxy-health filter '<results_json>' 80
```

### Request Execution

```bash
# Single request
onecrawl request execute '{"url": "https://api.example.com/data", "method": "GET"}'

# Batch requests with concurrency control
onecrawl request batch '[{"url":"..."},{"url":"..."}]' --concurrency 3 --delay 100
```

### HTTP Client

```bash
onecrawl http get "https://api.example.com/data"
onecrawl http post "https://api.example.com/submit" --body '{"key":"value"}'
onecrawl http head "https://example.com"
onecrawl http fetch '{"url":"...", "method":"POST", "headers":{"Auth":"Bearer ..."}}'
```

## MCP Tools

| Tool | Description |
|------|-------------|
| `data.http_get` | HTTP GET through browser session |
| `data.http_post` | HTTP POST through browser session |
| `stealth.block_domains` | Block domains by name or category |

## Download Management

```bash
onecrawl download set-path ./downloads    # Set download directory
onecrawl download list                    # List tracked downloads
onecrawl download fetch "https://..."     # Download file (base64)
onecrawl download wait --timeout 10000    # Wait for download
onecrawl download clear                   # Clear history
```
