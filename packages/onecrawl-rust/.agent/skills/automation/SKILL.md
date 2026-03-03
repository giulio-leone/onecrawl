---
name: automation
description: "Rate limiting, retry queues, task scheduling, session pooling, and interactive shell for production-grade browser automation workflows."
---

# Automation & Orchestration Skill

Production-grade browser automation infrastructure. Rate limiting, retry queues, task scheduling, and session pooling for reliable, scalable automation.

## Modules

| Module | Purpose |
|--------|---------|
| `rate_limiter` | Sliding-window rate limiter with per-second/minute/hour caps |
| `retry_queue` | Exponential backoff retry queue with jitter |
| `scheduler` | Cron-like task scheduler with pause/resume |
| `session_pool` | Multi-session manager with rotation strategies |
| `shell` | Interactive REPL with 23 commands and history |

## How It Works

### Rate Limiting
```bash
# Apply preset
onecrawl ratelimit set --preset conservative   # 0.5 req/s
onecrawl ratelimit set --preset moderate        # 2 req/s
onecrawl ratelimit set --preset aggressive      # 5 req/s
onecrawl ratelimit set --preset unlimited       # 1000 req/s

# View current stats
onecrawl ratelimit stats

# Reset counters
onecrawl ratelimit reset
```

### Retry Queue
```bash
# Enqueue failed operations for retry
onecrawl retry enqueue "https://example.com/page" navigate
onecrawl retry enqueue "https://api.com/data" extract --payload '{"selector":".data"}'

# Process queue
onecrawl retry next          # Get next item due for retry
onecrawl retry success <id>  # Mark as successful
onecrawl retry fail <id> "timeout error"  # Record failure, schedule retry

# Manage queue
onecrawl retry stats
onecrawl retry clear         # Remove completed items
onecrawl retry save queue.json
onecrawl retry load queue.json
```

### Task Scheduler
```bash
# Schedule tasks
onecrawl schedule add "scrape-daily" navigate '{"url":"https://example.com"}' \
  --interval 86400000 --delay 0

# Manage tasks
onecrawl schedule list
onecrawl schedule pause <task-id>
onecrawl schedule resume <task-id>
onecrawl schedule remove <task-id>

# View stats
onecrawl schedule stats

# Persist
onecrawl schedule save schedule.json
onecrawl schedule load schedule.json
```

### Session Pool
```bash
# Add sessions
onecrawl pool add "worker-1" --tags "scraping,us-east"
onecrawl pool add "worker-2" --tags "scraping,eu-west"

# Get next available session
onecrawl pool next

# Manage sessions
onecrawl pool stats
onecrawl pool cleanup        # Remove idle sessions

# Persist
onecrawl pool save pool.json
onecrawl pool load pool.json
```

### Interactive Shell
```bash
# Launch REPL
onecrawl shell

# Available commands inside shell:
#   goto <url>      — Navigate to URL
#   select <css>    — Query selector
#   xpath <expr>    — XPath query
#   text [selector] — Extract text
#   html [selector] — Extract HTML
#   click <selector>— Click element
#   type <sel> <text>— Type into input
#   screenshot      — Take screenshot
#   eval <js>       — Execute JavaScript
#   cookies         — Show cookies
#   links           — List all links
#   stealth         — Apply stealth
#   history         — Command history
#   help            — Show all commands
#   exit            — Exit shell
```

## Node.js API
```javascript
const browser = new NativeBrowser();
await browser.launch();

// Rate limiting
browser.rateLimitSetPreset('moderate');
const stats = browser.rateLimitStats();

// Retry queue
const id = browser.retryEnqueue('https://example.com', 'navigate');
const next = browser.retryGetNext();
browser.retryMarkSuccess(id);

// Scheduler
const taskId = browser.scheduleAdd('daily-scrape', 'navigate', 
  '{"url":"https://example.com"}', '{"interval_ms":86400000}');
const dueTasks = browser.scheduleGetDue();

// Session pool
browser.poolAddSession('worker-1', '["scraping"]');
const session = browser.poolGetNext();
```

## Python API
```python
browser = Browser()
browser.launch()

# Rate limiting
browser.rate_limit_set_preset('moderate')
stats = browser.rate_limit_stats()

# Retry queue
item_id = browser.retry_enqueue('https://example.com', 'navigate')
browser.retry_mark_success(item_id)

# Scheduler
task_id = browser.schedule_add('daily', 'navigate', '{"url":"..."}', '{"interval_ms":86400}')

# Session pool
browser.pool_add_session('worker-1', '["tag"]')
```
