---
name: "Programmatic Tool Calling"
description: "Multi-step tool workflows via code orchestration to reduce latency, context pollution, and token overhead."
---

# Programmatic Tool Calling Skill (Model-Agnostic)

## Purpose

Execute multi-step tool workflows via code orchestration to reduce latency, context pollution, and token overhead.

## Use when

- 3+ dependent tool calls
- Large intermediate outputs (logs, tables, files)
- Branching logic, retries, or fan-out/fan-in workflows

## Core Idea

Treat tools as callable functions inside an orchestration runtime (script/runner), not as one-turn-at-a-time chat actions.

## Procedure

1. Generate/execute orchestration code for loops, conditionals, parallel calls, retries, and early termination.
2. Process intermediate data in runtime (filter/aggregate/transform) instead of returning raw data to model context.
3. Return only high-signal outputs to the model (summary, decision, artifact references).

## Why It Works (provider/model independent)

- Fewer model round-trips for multi-call workflows.
- Intermediate data stays out of context unless needed.
- Explicit code control flow is easier to test, monitor, and debug.

## Guardrails

- Strict input/output schemas.
- Validate tool results before use.
- Idempotent/retry-safe tool design when possible.
- Timeout/cancellation/expiry handling.
- Sandbox execution for untrusted code; never blindly execute external payloads.

## Done Criteria

- Workflow completes with reduced context load and deterministic control flow.

## Anti-patterns

- Returning raw intermediate payloads to the model by default
- Unbounded loops without stop conditions
- Executing unvalidated tool output
