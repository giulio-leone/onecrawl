# Streaming AI Vision Skill

## Overview

Streaming AI Vision provides continuous visual understanding of browser content by capturing screencast frames at configurable FPS and sending them to Vision Language Models (VLMs) for analysis. It detects UI elements (CAPTCHAs, error dialogs, popups), suggests actions, tracks costs, and enables reactive automation based on visual observations.

## Key Files

- `crates/onecrawl-cdp/src/vision.rs` — Core `VisionStream` engine with frame capture and observation
- `crates/onecrawl-mcp-rs/src/handlers/vision.rs` — 7 MCP action handlers
- `crates/onecrawl-cli-rs/src/commands/vision.rs` — CLI vision commands

## API Reference

### MCP Actions

| Action | Description | Parameters |
|--------|-------------|------------|
| `vision_stream_start` | Start vision stream | `model?` (default: "gpt-4o"), `format?` (png/jpeg), `quality?` (0-100, default: 70), `width?`, `height?`, `fps?` (default: 0.5), `max_fps?` (default: 2.0), `describe?`, `react_to?` (categories), `prompt?`, `max_tokens?`, `max_cost_cents?`, `output?` |
| `vision_stream_stop` | Stop vision stream | _(none)_ |
| `vision_stream_status` | Get stream status and statistics | _(none)_ |
| `vision_stream_describe` | One-shot frame capture + VLM description | `prompt?` |
| `vision_stream_observations` | Get recent observations | `limit?` (default: 10) |
| `vision_stream_set_fps` | Adjust capture rate at runtime | `fps` (0 < fps ≤ max_fps) |
| `vision_stream_react` | Parse LLM response and record observation | `response_text`, `frame_index?` |

### CLI Commands

| Command | Description |
|---------|-------------|
| `onecrawl vision start` | Start vision stream (`--model`, `--fps`, `--describe`, `--react-to`, `--prompt`, `--max-tokens`, `--max-cost-cents`, `--format`, `--quality`, `--output`) |
| `onecrawl vision stop` | Stop vision stream |
| `onecrawl vision status` | Show stream status |
| `onecrawl vision describe` | One-shot describe current page |
| `onecrawl vision observations [limit]` | Show recent observations |
| `onecrawl vision set-fps <fps>` | Adjust capture rate |

### Core Rust API

```rust
use onecrawl_cdp::{VisionStream, VisionConfig, ScreenshotFormat};

let stream = VisionStream::new(VisionConfig {
    model: "gpt-4o".into(),
    api_key: Some("sk-...".into()),
    fps: 0.5,
    max_fps: 2.0,
    describe: true,
    react_to: vec!["captcha".into(), "error_dialog".into()],
    prompt: Some("Describe what's on the page".into()),
    max_tokens: Some(300),
    max_cost_cents: Some(50),
    screenshot_format: ScreenshotFormat::Jpeg { quality: 70 },
    resolution: Some((1280, 720)),
    ..Default::default()
});

// Start continuous streaming
stream.start(&page).await?;

// One-shot describe
let obs = stream.describe_once(&page).await?;

// Get observations
let recent = stream.observations(10).await;

// Adjust FPS
stream.set_fps(1.0).await?;

// Get detections matching react_to categories
let detections = stream.matching_detections(5).await;

// Status
let status = stream.status().await;
println!("Frames: {}, Observations: {}, Cost: {}¢", 
    status.frames_captured, status.observations_made, status.total_cost_cents);

// Stop
let final_status = stream.stop().await?;
```

## Architecture

### Pipeline

```
Screencast Capture → Frame Encoding → VisionRequest Build → [External LLM] → Parse Response → Record Observation
     (CDP)             (JPEG/PNG)        (OpenAI-compatible)                     (JSON parse)      (ring buffer)
```

**Important**: The VisionStream prepares requests but does **not** call the LLM directly. The caller is responsible for:
1. Calling `prepare_vision_request()` to get the API payload
2. Sending it to the VLM API
3. Calling `parse_vision_response()` with the response text
4. Calling `record_observation()` to store the result

For MCP integration, `vision_stream_react` handles steps 3-4.

### Observation Structure

```rust
VisionObservation {
    timestamp: String,          // ISO 8601
    frame_index: u64,           // Sequential frame number
    description: String,        // Human-readable page description
    detected: Vec<Detection>,   // Detected UI elements/issues
    suggested_actions: Vec<SuggestedAction>,
    confidence: f32,            // 0.0-1.0
    tokens_used: u32,
    cost_cents: u32,
}
```

### Detection Categories

| Category | Description | Example |
|----------|-------------|---------|
| `error_dialog` | Error dialog or modal | JavaScript error popup |
| `captcha` | CAPTCHA challenge | reCAPTCHA, hCaptcha |
| `popup` | Modal, overlay, or banner | Cookie consent, newsletter |
| `loading` | Loading indicator | Spinner, skeleton screen |
| `content_change` | Significant content change | Page updated, new data |

### Suggested Action Types

| Action Type | Description | Target |
|-------------|-------------|--------|
| `click` | Click an element | CSS selector or description |
| `dismiss` | Dismiss popup/dialog | Close button reference |
| `solve_captcha` | Solve CAPTCHA challenge | CAPTCHA element |
| `wait` | Wait for condition | Timeout or selector |
| `navigate` | Navigate to URL | Target URL |
| `scroll` | Scroll page | Direction |

### Screenshot Format

- **JPEG** (default): Quality 70%, smaller payload, faster transmission
- **PNG**: Lossless, larger, better for pixel-precise analysis

### Cost Tracking

- Per-observation token and cost tracking
- `max_cost_cents` cap stops streaming when budget exceeded
- Status includes running totals: `total_tokens`, `total_cost_cents`

### Ring Buffer

- Observations stored in ring buffer (last 1000)
- `detections_by_category` counter in status for aggregate analysis

## Best Practices

- Start with low FPS (0.5) and increase only if needed — each frame costs tokens
- Use JPEG format with quality 70 for optimal cost/quality balance
- Set `max_cost_cents` to prevent runaway API costs
- Use `react_to` to filter detections to categories you care about
- Use `describe_once` for one-shot analysis instead of starting a full stream
- Combine with Event Reactor: emit events on specific detections for automated response
- Use `vision_stream_set_fps` to dynamically adjust based on page activity
- Set custom `prompt` for domain-specific analysis (e.g., "Look for pricing changes")

## Troubleshooting

| Issue | Cause | Solution |
|-------|-------|----------|
| No observations produced | LLM API not called by caller | Implement the request-response cycle; VisionStream only prepares requests |
| Cost exceeding budget | High FPS or verbose model | Reduce FPS; set `max_cost_cents`; use smaller model |
| Frame capture blank | Page not loaded or headless rendering issue | Wait for page load; check viewport dimensions |
| Parse error on response | LLM response format unexpected | Verify model supports vision; check `parse_vision_response` error |
| FPS won't increase | `max_fps` limit (default: 2.0) | Increase `max_fps` in config (not recommended above 5.0) |
| Detections empty | Model not detecting categories | Use more specific `prompt`; try different model |
