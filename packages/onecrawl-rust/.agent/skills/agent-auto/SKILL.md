# AI Agent Auto Skill

## Overview

AI Agent Auto provides goal-based autonomous browser automation. Given a natural-language goal (e.g., "log into Gmail and check inbox"), it decomposes the goal into executable steps using the Task Planner, executes them with self-healing retry logic, tracks costs, captures screenshots, and supports state resumption from saved checkpoints.

## Key Files

- `crates/onecrawl-cdp/src/agent_auto.rs` — Core `AgentAuto` engine with plan-execute loop
- `crates/onecrawl-cdp/src/task_planner.rs` — AI task decomposition and planning
- `crates/onecrawl-cdp/src/agent_memory.rs` — Persistent learning across executions
- `crates/onecrawl-mcp-rs/src/handlers/agent.rs` — MCP handlers (agent_auto_* actions + task_* + vision_*)
- `crates/onecrawl-cli-rs/src/commands/browser/` — CLI browser/agent commands

## API Reference

### MCP Actions — Agent Auto

| Action | Description | Parameters |
|--------|-------------|------------|
| `agent_auto_run` | Plan and execute a goal autonomously | `goal`, `model?`, `max_steps?` (default: 50), `max_cost_cents?`, `screenshot_every_step?`, `screenshot_dir?`, `output?`, `output_format?` (csv/json/jsonl), `save_state?`, `verbose?`, `allowed_domains?`, `blocked_domains?`, `timeout_secs?`, `use_memory?` (default: true), `memory_path?` |
| `agent_auto_plan` | Plan only (dry-run, no execution) | `goal`, `verbose?` |
| `agent_auto_status` | Get current agent execution status | _(none)_ |
| `agent_auto_stop` | Stop execution, optionally save state | `save_state?` (file path) |
| `agent_auto_resume` | Resume from a saved state file | `state_file`, `max_steps?`, `max_cost_cents?`, `verbose?` |
| `agent_auto_result` | Get the last execution result | _(none)_ |

### MCP Actions — Task Planning

| Action | Description | Parameters |
|--------|-------------|------------|
| `task_decompose` | Decompose goal into subtasks | `goal`, `context?`, `max_depth?` |
| `task_plan` | Plan tasks with execution strategy | `tasks[]`, `strategy?` (sequential/parallel/dependency) |
| `task_status` | Get current planning status | _(none)_ |

### MCP Actions — Agentic Reasoning

| Action | Description | Parameters |
|--------|-------------|------------|
| `agent_loop` | Continuous observation-decision loop | _(AgentLoopParams)_ |
| `goal_assert` | Assert a goal condition is met | _(GoalAssertParams)_ |
| `think` | Reasoning step (no side effects) | _(ThinkParams)_ |
| `plan_execute` | Plan then execute in one call | _(PlanExecuteParams)_ |
| `agent_execute_chain` | Execute a sequence of commands | `commands[]`, `stop_on_error?` |

### CLI Commands

| Command | Description |
|---------|-------------|
| `onecrawl agent auto` | Run autonomous goal-based automation |

### Core Rust API

```rust
use onecrawl_cdp::{AgentAuto, AgentAutoConfig, OutputFormat};

let config = AgentAutoConfig {
    goal: "Log into LinkedIn and extract connection count".into(),
    model: Some("gpt-4o".into()),
    max_steps: 50,
    max_cost_cents: Some(100),
    screenshot_every_step: true,
    screenshot_dir: Some("./screenshots".into()),
    output: Some("results.json".into()),
    output_format: Some(OutputFormat::Json),
    verbose: true,
    allowed_domains: vec!["linkedin.com".into()],
    blocked_domains: vec![],
    use_memory: true,
    timeout_secs: Some(300),
    ..Default::default()
};

// Plan only
let steps = AgentAuto::plan(&config)?;

// Plan and execute
let result = agent_auto_run(&page, config).await?;
println!("Success: {}, Steps: {}/{}", result.success, result.steps_completed, result.steps_total);
println!("Cost: {} cents, Duration: {:.1}s", result.cost_cents, result.duration_secs);
```

## Architecture

### Execution Pipeline

```
Goal → Task Planner → Steps → Execute Loop → Result
                ↑                    ↓
            Memory ←──── Learning ←──┘
```

1. **Goal Parsing** — Natural language goal is sent to the Task Planner
2. **Step Generation** — Planner produces `Vec<AutoStep>` with action types
3. **Execution Loop** — Each step executes against the browser page
4. **Self-Healing** — On failure, retries up to 3 times with backoff
5. **Memory** — Successful patterns stored in `AgentMemory` for future runs
6. **Cost Tracking** — 1 cent default per step; stops when `max_cost_cents` exceeded

### Step Status Machine

```
Pending → Running → Completed
              ↓         ↑
          Retrying ──────┘
              ↓
           Failed / Skipped
```

### Action Types

The planner generates these action types for steps:

| Action | Description |
|--------|-------------|
| `navigate` | Go to a URL |
| `click` | Click element by selector |
| `smart_click` | Click element by natural language query |
| `smart_fill` | Fill input by natural language query + value |
| `type` | Type text into element |
| `extract` | Extract data from page |
| `wait` | Wait for selector or condition |
| `screenshot` | Capture page state |
| `scroll` | Scroll page |
| `assert` | Verify condition |

### Task Planner (`task_planner.rs`)

The planner produces a `TaskPlan` with:

- **Steps** — Ordered `PlannedStep` list with fallback alternatives
- **Strategy** — `Direct` | `Exploratory` | `MemoryAssisted` | `Hybrid`
- **Confidence** — 0.0–1.0 confidence score
- **Estimated Duration** — Predicted execution time

### State Resumption

Save state with `save_state` parameter → JSON file containing:
- Config, steps (with progress), extracted data, cost, URL, cookies, timestamp

Resume with `agent_auto_resume` → continues from last completed step.

### Safety Policy

- **allowed_domains** — Whitelist for navigation (empty = allow all)
- **blocked_domains** — Blacklist for navigation
- Navigation actions validated against policy before execution

## Best Practices

- Set `max_cost_cents` to prevent runaway execution costs
- Use `allowed_domains` to constrain navigation to expected sites
- Enable `screenshot_every_step` during development for debugging
- Use `save_state` for long-running tasks to enable resume on failure
- Set `timeout_secs` to prevent indefinite execution
- Enable `use_memory` (default) to leverage past execution knowledge
- Use `agent_auto_plan` first to preview steps before execution
- Prefer `smart_click`/`smart_fill` over raw selectors for resilient automation

## Troubleshooting

| Issue | Cause | Solution |
|-------|-------|----------|
| Steps don't match expected flow | Goal too vague | Provide specific, actionable goals with site context |
| Max cost exceeded | Too many retries or steps | Increase `max_cost_cents` or reduce `max_steps` |
| Navigation blocked | Domain not in allowed list | Add domain to `allowed_domains` |
| Resume fails | State file corrupted or outdated | Re-run from scratch; check state JSON integrity |
| Memory not helping | First run on new domain | Memory improves after successful executions on same patterns |
| Timeout reached | Complex workflow or slow site | Increase `timeout_secs`; break goal into smaller sub-goals |
