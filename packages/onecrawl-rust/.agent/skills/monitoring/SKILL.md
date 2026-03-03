---
name: monitoring
description: "Browser automation monitoring: benchmarks, coverage analysis, screenshot diffing, console/dialog handling, DOM observation, and performance tracing."
---

# Monitoring & Testing Skill

Monitor, benchmark, and test browser automation workflows with OneCrawl's diagnostic modules.

## Modules

| Module | Purpose |
|--------|---------|
| `benchmark` | CDP performance benchmarking with percentiles |
| `coverage` | JS/CSS code coverage analysis |
| `screenshot_diff` | Pixel-level screenshot comparison |
| `tracing_cdp` | Chrome DevTools Protocol tracing |
| `page_watcher` | Navigation, title, scroll, resize change tracking |
| `console` | Browser console message capture |
| `dialog` | Alert/confirm/prompt dialog handling |
| `dom_observer` | MutationObserver-based DOM change detection |
| `network_log` | Full request/response logging |

## How It Works

### Benchmarks
```bash
onecrawl benchmark run --iterations 10    # Run CDP benchmark suite
onecrawl benchmark results                # View results with percentiles
```

### Coverage
```bash
onecrawl coverage start --js --css
# ... interact with page ...
onecrawl coverage stop
onecrawl coverage report                  # Usage percentages
```

### Screenshot Diff
```bash
onecrawl screenshot-diff compare before.png after.png
onecrawl screenshot-diff compare before.png after.png --threshold 0.05
```

### Tracing
```bash
onecrawl trace start --categories "devtools.timeline,v8"
# ... perform operations ...
onecrawl trace stop trace.json
```

### Page Watcher
```bash
onecrawl watch start                      # Track all changes
onecrawl watch start --navigation         # Navigation only
onecrawl watch changes                    # Get recorded changes
onecrawl watch stop
```

### Console
```bash
onecrawl console start                    # Start capturing
onecrawl console messages                 # Get all messages
onecrawl console messages --level error   # Errors only
onecrawl console stop
```

### DOM Observer
```bash
onecrawl dom-observer start ".content"    # Watch element for changes
onecrawl dom-observer changes             # Get mutation records
onecrawl dom-observer stop
```

### Network Log
```bash
onecrawl network-log start
onecrawl network-log entries              # All requests/responses
onecrawl network-log entries --filter api # Filter by URL pattern
onecrawl network-log stop
```
