# AGENTS.md

## Runtime Dispatcher (No Tool-Name Ambiguity)

This repository provides **two runtime adapter files**:

1. `AGENTS.vscode.MD` — VS Code runtime (`vscode_askQuestions`)
2. `AGENTS.copilot-cli.MD` — Copilot CLI runtime (`ask_user`)

Do **not** mix tool names across runtimes.

## Selection Rule (Mandatory)

- If runtime is VS Code, use only `AGENTS.vscode.MD`.
- If runtime is Copilot CLI, use only `AGENTS.copilot-cli.MD`.
- At task start, mode selection and the 5-option interaction loop remain mandatory in the selected file.

## Ownership Model (Skills-First, Mandatory)

`AGENTS.md` + runtime adapters own only **runtime-critical contract**:
- question tool binding (`vscode_askQuestions` vs `ask_user`)
- fixed 5-option interaction shape
- `Freeform` hard invariant (option 4, exact label — "Something else" via freeform tool)
- `Autonomous Mode` placement and stop condition (`"I am satisfied"`)
- escalation trigger for compatibility triad

All operational procedures/checklists are skill-owned and must not be duplicated as full policy prose in runtime adapters.

## Skill Catalog (Global Default)

Operational procedures/checklists are owned by skills. This catalog is the **global source of truth** — runtime adapters inherit it and must not duplicate its content.

Read the corresponding `SKILL.md` when the trigger condition for that skill is met.

- **[`interaction-loop`](.agents/skills/interaction-loop/SKILL.md)** — **Invoke when**: starting a task, hitting a decision point, or completing an autonomous run | **Provides**: 5-option iterative loop template, recommendation tracking, and stop-condition rules (`"I am satisfied"`)

- **[`breaking-change-paths`](.agents/skills/breaking-change-paths/SKILL.md)** — **Invoke when**: a task may affect public contracts, APIs, schemas, or behavioral compatibility; or a root-cause fix implies architectural reshaping | **Provides**: structured pros/cons + risk level + migration steps for Breaking vs Non-Breaking path decision

- **[`planning-tracking`](.agents/skills/planning-tracking/SKILL.md)** — **Invoke when**: starting any non-trivial task, scope changes during execution, or multiple files/workstreams must be coordinated | **Provides**: mandatory Plan/Milestone/Issue schema with dependency-ordered execution procedure and parallel-safe concurrency rules

- **[`completion-gate`](.agents/skills/completion-gate/SKILL.md)** — **Invoke when**: closing an issue, marking a milestone done, or opening/merging a PR | **Provides**: double-consecutive-clean-pass quality gate (lint + build + required tests, zero errors/warnings, no exceptions)

- **[`github-sync`](.agents/skills/github-sync/SKILL.md)** — **Invoke when**: creating/updating a plan, changing issue status, or completing milestones | **Provides**: naming rules, required metadata labels (priority/type/status), and step-by-step procedure to keep local plan and GitHub artifacts in sync

- **[`testing-policy`](.agents/skills/testing-policy/SKILL.md)** — **Invoke when**: implementing or modifying any feature/fix; preparing issue closure or PR merge | **Provides**: required test layers (unit / integration / E2E / non-regression) and a before/after coverage diff procedure

- **[`e2e-testing`](.agents/skills/e2e-testing/SKILL.md)** — **Invoke when**: a user-facing flow changes or a critical integration path is introduced/modified | **Provides**: E2E checklist covering happy path + edge cases, CI-compatible execution, stable selectors, and deterministic setup/teardown

- **[`session-logging`](.agents/skills/session-logging/SKILL.md)** — **Invoke when**: starting/ending a session, completing issues/milestones, or syncing GitHub status | **Provides**: session journal file template with required sections (Status, Work Completed, Completion Gate evidence, Decisions, Blockers, GitHub Sync, Branch, Timestamp)

- **[`rollback-rca`](.agents/skills/rollback-rca/SKILL.md)** — **Invoke when**: an issue fails the completion gate 3 consecutive times | **Provides**: RCA procedure (architecture/dependency/scope analysis) + 3 recovery options (rescope / rollback / redesign) presented via the runtime question tool

- **[`policy-coherence-audit`](.agents/skills/policy-coherence-audit/SKILL.md)** — **Invoke when**: updating `AGENTS.MD`, merging new workflow rules, or noticing behavioral ambiguity during execution | **Provides**: cross-policy contradiction checklist covering language, interaction model, completion gates, scope, and skill reference coherence

- **[`systematic-debugging`](.agents/skills/systematic-debugging/SKILL.md)** — **Invoke when**: a bug or unexpected behavior is encountered, a test fails without obvious cause, or a previous fix attempt did not resolve the issue | **Provides**: evidence-based debugging procedure (reproduce → hypothesize → instrument → collect `debug-data.log` → analyze → fix → clean up), MCP tool integration for browser/network/runtime debugging

## Maintenance Rule

When updating policy logic:

1. If the change is runtime-critical, apply it to **both** runtime adapters (tool names adjusted per runtime).
2. If the change is operational/procedural, update the owning skill (`.agents/skills/*/SKILL.md`) first, then update the entry in the **Skill Catalog** section above.
3. Keep semantic parity unless a runtime-specific behavior is intentionally required.
4. Preserve English-only wording and non-duplicative ownership between AGENTS and skills.
