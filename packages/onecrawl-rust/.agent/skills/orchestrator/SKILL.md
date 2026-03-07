# Multi-Device Orchestration Skill

## Overview

The Orchestrator coordinates browser automation across multiple devices simultaneously — desktop browsers, Android devices, and iOS devices. It executes orchestration workflows defined as JSON files with parallel step execution, variable interpolation, conditional logic, error policies, and cross-device data sharing.

## Key Files

- `crates/onecrawl-cdp/src/orchestrator.rs` — Core `Orchestrator` engine with parallel execution
- `crates/onecrawl-cdp/src/android.rs` — Android device client (ADB/Appium)
- `crates/onecrawl-cdp/src/ios.rs` — iOS device client (WebDriverAgent)
- `crates/onecrawl-mcp-rs/src/handlers/orchestrator.rs` — 5 MCP action handlers
- `crates/onecrawl-cli-rs/src/commands/orchestrator.rs` — CLI commands

## API Reference

### MCP Actions

| Action | Description | Parameters |
|--------|-------------|------------|
| `orchestrator_run` | Execute an orchestration workflow | `file?` (JSON path), `config?` (inline JSON) |
| `orchestrator_validate` | Validate orchestration without executing | `file?`, `config?` |
| `orchestrator_status` | Get current orchestration status | _(none)_ |
| `orchestrator_stop` | Stop running orchestration | _(none)_ |
| `orchestrator_devices` | List connected devices and their status | _(none)_ |

### CLI Commands

| Command | Description |
|---------|-------------|
| `onecrawl orchestrate run <file>` | Execute orchestration from JSON file |
| `onecrawl orchestrate validate <file>` | Validate orchestration file |
| `onecrawl orchestrate devices` | List connected devices |
| `onecrawl orchestrate stop` | Stop running orchestration |

Options: `--verbose`, `--timeout <secs>`

### Orchestration JSON Schema

```json
{
  "name": "cross-device-login-test",
  "description": "Test login flow across browser and mobile",
  "devices": {
    "desktop": {
      "device_type": "browser",
      "headless": false,
      "viewport": [1920, 1080]
    },
    "pixel": {
      "device_type": "android",
      "adb_serial": "emulator-5554",
      "package_name": "com.android.chrome",
      "activity_name": "com.google.android.apps.chrome.Main"
    },
    "iphone": {
      "device_type": "ios",
      "udid": "00008030-...",
      "wda_url": "http://localhost:8100",
      "bundle_id": "com.apple.mobilesafari"
    }
  },
  "variables": {
    "login_url": "https://example.com/login",
    "username": "test@example.com"
  },
  "steps": [
    {
      "name": "Navigate all devices",
      "actions": [
        { "device": "desktop", "action": { "type": "navigate", "url": "${login_url}" } },
        { "device": "pixel", "action": { "type": "navigate", "url": "${login_url}" } },
        { "device": "iphone", "action": { "type": "navigate", "url": "${login_url}" } }
      ]
    },
    {
      "name": "Fill login on desktop",
      "actions": [
        { "device": "desktop", "action": { "type": "smart_fill", "query": "email", "value": "${username}" } }
      ],
      "on_error": "retry",
      "retry": 3
    }
  ],
  "on_error": "stop",
  "timeout_secs": 300
}
```

### Device Action Types

| Action | Description | Fields |
|--------|-------------|--------|
| `navigate` | Navigate to URL | `url` |
| `click` | Click element | `selector?`, `x?`, `y?`, `text?` |
| `type` | Type text | `selector?`, `text`, `x?`, `y?` |
| `smart_click` | Click by natural language | `query` |
| `smart_fill` | Fill input by natural language | `query`, `value` |
| `screenshot` | Capture screen | `path?` |
| `wait` | Wait for element | `selector`, `timeout_ms?` |
| `swipe` | Swipe gesture (mobile) | `start_x`, `start_y`, `end_x`, `end_y`, `duration_ms?` |
| `back` | Navigate back | _(none)_ |
| `evaluate` | Execute JavaScript | `script` |
| `extract` | Extract data from element | `selector`, `attribute?` |
| `assert` | Assert condition | `condition`, `value?` |
| `sleep` | Wait N milliseconds | `ms` |
| `log` | Log a message | `message` |
| `set_variable` | Set a runtime variable | `name`, `value` |
| `launch_app` | Launch app (mobile) | `package?` (Android), `bundle_id?` (iOS) |

### Core Rust API

```rust
use onecrawl_cdp::{Orchestrator, Orchestration};

// Load from file
let orch = Orchestrator::from_file("workflow.json")?;

// Validate
Orchestrator::validate(&orch)?;

// Execute
let mut engine = Orchestrator::new(orch);
engine.connect_devices().await?;
let result = engine.execute().await?;
engine.disconnect().await?;

println!("Success: {}, Steps: {}/{}", result.success, result.steps_completed, result.steps_total);
```

## Architecture

### Parallel Execution Model

Within each step, actions targeting **different devices** execute in parallel (via `tokio::join!`). Actions targeting the **same device** within a step are serialized.

```
Step 1: [desktop:navigate, pixel:navigate, iphone:navigate]  ← all parallel
Step 2: [desktop:fill, desktop:click]                         ← serialized (same device)
Step 3: [desktop:screenshot, pixel:screenshot]                ← parallel
```

### Variable Interpolation

`${var}` substitution in string fields. Variables come from:
1. Initial `variables` map in orchestration config
2. Runtime `set_variable` actions
3. `save_as` step results captured into variables

Single-pass substitution prevents injection attacks.

### Error Policies

| Policy | Behavior |
|--------|----------|
| `stop` | Abort orchestration on first error (default) |
| `continue` | Log error and proceed to next step |
| `retry` | Retry step up to `retry` count |
| `skip` | Skip the failed step entirely |

Policies apply globally (`on_error` at root) or per-step (`on_error` on step).

### Device Types

| Type | Connection | Requirements |
|------|-----------|--------------|
| `browser` | CDP (Chrome DevTools Protocol) | Chrome/Chromium installed |
| `android` | ADB + Appium | ADB connected device, optional Appium server |
| `ios` | WebDriverAgent | Connected iOS device/simulator, WDA running |

### Result Structure

```rust
OrchestrationResult {
    name: String,
    success: bool,
    steps_completed: usize,
    steps_total: usize,
    step_results: Vec<StepResult>,   // Per-step with device-level results
    variables: HashMap<String, String>,
    duration_secs: f64,
    errors: Vec<String>,
}
```

## Best Practices

- Define `timeout_secs` at orchestration level to prevent indefinite hangs
- Use `validate` before `run` to catch configuration errors early
- Use `save_as` to capture extracted data and pass between steps
- Set per-step `on_error: "continue"` for non-critical steps (e.g., screenshots)
- Use `set_variable` for dynamic values discovered during execution
- Test with browser-only first, then add mobile devices incrementally
- Keep step names descriptive — they appear in result output and logs

## Troubleshooting

| Issue | Cause | Solution |
|-------|-------|----------|
| Device connection failed | ADB/WDA not running or device not connected | Check `adb devices` or WDA status; verify `adb_serial`/`udid` |
| Validation error | Missing device reference in step actions | Ensure all `device` values in actions match `devices` keys |
| Step timeout | Slow device response | Increase `timeout_ms` on `wait` actions; increase global `timeout_secs` |
| Variable not substituted | Typo in `${var}` name | Check variable name matches exactly; variables are case-sensitive |
| Partial success | Some devices failed, others succeeded | Check `step_results` → `device_results` for per-device errors |
| iOS tap misses | Coordinate mismatch | Verify device resolution; use `smart_click` instead of raw coordinates |
