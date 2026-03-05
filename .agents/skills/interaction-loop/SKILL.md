---
name: "Interaction Loop"
description: "Iterative decision loop with consistent user checkpoints and explicit stop criteria."
---

# Interaction Loop Skill

## Purpose

Enforce a strict iterative decision loop with consistent user checkpoints and explicit stop criteria.

## Use when

- Starting a new task
- Hitting a decision point
- Completing an autonomous run

## Checklist

- Ask exactly one clear question per iteration.
- Provide exactly 5 options:
  1. Non-Breaking Path
  2. Breaking Path
  3. Alternative Structural Path
  4. Freeform
  5. Autonomous Mode
- Mark the recommended (most future-proof) option.
- At autonomous completion, ask for: rating, next action, satisfaction.
- Continue iterating until the exact stop phrase is provided: **"I am satisfied"**.

## Short examples

- Start-of-task question: "Which path should I execute first?" (5 options above).
- End-of-run question: "Rate this result, choose the next action, and confirm satisfaction."

## Anti-patterns

- Multi-question prompts in one iteration
- Missing one or more of the 5 mandatory options
- Stopping without explicit "I am satisfied"
