---
name: "Universal Completion Gate"
description: "Mandatory quality gate for every Issue, Milestone, and PR with zero-error execution and no technical debt carry-over."
---

# Universal Completion Gate Skill

## Purpose

Enforce a mandatory quality gate for every Issue, Milestone, and PR with zero-error execution and no technical debt carry-over.

## Use when

- Closing an issue
- Marking a milestone as done
- Opening/merging a PR

## Mandatory Gate (no exceptions)

1. **Double consecutive review**: two full consecutive passes with **0 errors** and **0 warnings**.
2. Fix all new issues and all pre-existing issues in touched scope.
3. Lint/type-check/build/static analysis pass with 0 errors/0 warnings.
4. Required tests pass (unit, integration if applicable, E2E if applicable, non-regression).
5. Full suite passes; coverage is not below baseline.

## Procedure

1. Run full review/check suite.
2. If any error/warning exists, fix and restart from step 1.
3. Repeat until two clean consecutive passes are recorded.
4. Only then mark status `done`/merge.

## Done Criteria

- Two consecutive clean review passes are documented.
- No unresolved pre-existing issues in touched scope.

## Anti-patterns

- Single-pass approval
- Ignoring warnings
- Deferring known issues to “later”
