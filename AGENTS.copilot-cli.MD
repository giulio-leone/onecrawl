# AGENTS.copilot-cli.MD

## 1. Interaction Protocol (Runtime-Critical Contract)

Interaction must occur **exclusively via `ask_user`** and remain iterative, interactive, and unlimited.
If `ask_user` is unavailable in the current runtime, use an equivalent structured question tool while preserving the same interaction contract.

Each iteration must ask exactly **one clear question** and provide exactly **5 options**.

Default option composition (for normal development decisions):
1. **Recommended Development Path** (mark as ⭐ most future-proof)
2. **Alternative Development Path A**
3. **Alternative Development Path B**
4. **Freeform** — "Something else": lets the user type a custom direction via the freeform tool
5. **Autonomous Mode**

`Freeform` is a hard invariant across all iterations:
- option **4** must always be **Freeform** — the "Something else" escape hatch via the runtime freeform tool
- label must remain exactly `Freeform` (no rename, merge, or synonym replacement)
- when selected, prompt the user via the freeform tool to describe their preferred direction in free text
- applies to default mode, compatibility-triad escalation, rollback/RCA prompts, and task-start mode selection
- if `Freeform` is missing, the iteration is invalid and must be regenerated before proceeding

Continuity is mandatory:
- options **1-3** must, by default, carry forward all still-valid options from the previous iteration
- do not drop an option silently; if one is removed, explicitly state why and what replaced it

For each option, always include:
- why the option exists
- where the option leads next (expected outcome / milestone impact)
- risk level (`low` / `medium` / `high`)

Use this mandatory option card format:
`<Option title> — Why: <reason> | Leads to: <next step/result> | Risk: <low|medium|high>`

Recommendation tracking is mandatory:
- mark the current recommendation with ⭐
- if the recommendation changes from the previous iteration, explain why in one sentence

The compatibility triad below is an **escalation package**, not the default:
- **Non-Breaking Path**
- **Breaking Path**
- **Alternative Structural Path**

Use the compatibility triad only when at least one condition is true:
- there is a concrete compatibility/contract impact
- migration effort must be evaluated
- architectural break/refactor trade-off is the core decision
- the user explicitly asks for breaking/non-breaking analysis

When the compatibility triad is used:
- the interaction still remains exactly 5 options: triad options + **Freeform** + **Autonomous Mode**
- include at minimum: pros/cons, risk level, migration steps
- explicitly state that quality gates are unchanged (unit/integration/E2E/non-regression must still pass)

At the beginning of each task, mode selection is mandatory: **never assume a default mode globally**.
Mode selection at task start must use the same 5-option structure above (development-options-first unless escalation is justified).

### Autonomous Mode

`Autonomous Mode` means:
- work autonomously until all milestones are completed
- always choose the best, most future-proof option within the selected path
- if a new unapproved breaking impact appears, pause and ask via `ask_user`
- **at the end of every autonomous run** — **mandatory, no exceptions**: invoke `ask_user` to re-enter the feedback loop; the run is considered incomplete until this call is made — ask for:
  1. rating of the completed run
  2. next action to take
  3. explicit satisfaction confirmation

### Feedback Loop — Never Interrupt

The feedback loop is **perpetual by design**:
- the loop **must never self-terminate** for any reason (task completion, milestone reached, silence, assumed satisfaction)
- the agent must **never presume** the user is satisfied
- **no reason is sufficient** to break the loop other than the user's explicit stop phrase
- after every autonomous run, after every decision, after every completion — always re-enter the loop with `ask_user`

The interaction loop stops **only** when the user explicitly says:
> "I am satisfied"

Until that exact phrase is received, the loop continues — unconditionally and indefinitely.

---

## 2. Operational Procedures (Skill-Owned)

Operational procedures/checklists are owned by skills and are intentionally **not duplicated** here.
Read the corresponding `SKILL.md` when the trigger condition for that skill is met.

- **[`interaction-loop`](.agents/skills/interaction-loop/SKILL.md)** — **Invoke when**: starting a task, hitting a decision point, or completing an autonomous run | **Provides**: 5-option iterative loop template, recommendation tracking, and stop-condition rules (`"I am satisfied"`)

- **[`breaking-change-paths`](.agents/skills/breaking-change-paths/SKILL.md)** — **Invoke when**: a task may affect public contracts, APIs, schemas, or behavioral compatibility; or a root-cause fix implies architectural reshaping | **Provides**: structured pros/cons + risk level + migration steps for Breaking vs Non-Breaking path decision

- **[`planning-tracking`](.agents/skills/planning-tracking/SKILL.md)** — **Invoke when**: starting any non-trivial task, scope changes during execution, or multiple files/workstreams must be coordinated | **Provides**: mandatory Plan/Milestone/Issue schema with dependency-ordered execution procedure and parallel-safe concurrency rules

- **[`completion-gate`](.agents/skills/completion-gate/SKILL.md)** — **Invoke when**: closing an issue, marking a milestone done, or opening/merging a PR | **Provides**: double-consecutive-clean-pass quality gate (lint + build + required tests, zero errors/warnings, no exceptions)

- **[`github-sync`](.agents/skills/github-sync/SKILL.md)** — **Invoke when**: creating/updating a plan, changing issue status, or completing milestones | **Provides**: naming rules, required metadata labels (priority/type/status), and step-by-step procedure to keep local plan and GitHub artifacts in sync

- **[`testing-policy`](.agents/skills/testing-policy/SKILL.md)** — **Invoke when**: implementing or modifying any feature/fix; preparing issue closure or PR merge | **Provides**: required test layers (unit / integration / E2E / non-regression) and a before/after coverage diff procedure

- **[`e2e-testing`](.agents/skills/e2e-testing/SKILL.md)** — **Invoke when**: a user-facing flow changes or a critical integration path is introduced/modified | **Provides**: E2E checklist covering happy path + edge cases, CI-compatible execution, stable selectors, and deterministic setup/teardown

- **[`session-logging`](.agents/skills/session-logging/SKILL.md)** — **Invoke when**: starting/ending a session, completing issues/milestones, or syncing GitHub status | **Provides**: session journal file template with required sections (Status, Work Completed, Completion Gate evidence, Decisions, Blockers, GitHub Sync, Branch, Timestamp)

- **[`rollback-rca`](.agents/skills/rollback-rca/SKILL.md)** — **Invoke when**: an issue fails the completion gate 3 consecutive times | **Provides**: RCA procedure (architecture/dependency/scope analysis) + 3 recovery options (rescope / rollback / redesign) to present via `ask_user`

- **[`policy-coherence-audit`](.agents/skills/policy-coherence-audit/SKILL.md)** — **Invoke when**: updating `AGENTS.MD`, merging new workflow rules, or noticing behavioral ambiguity during execution | **Provides**: cross-policy contradiction checklist covering language, interaction model, completion gates, scope, and skill reference coherence

- **[`systematic-debugging`](.agents/skills/systematic-debugging/SKILL.md)** — **Invoke when**: a bug or unexpected behavior is encountered, a test fails without obvious cause, or a previous fix attempt did not resolve the issue | **Provides**: evidence-based debugging procedure (reproduce → hypothesize → instrument → collect `debug-data.log` → analyze → fix → clean up), MCP tool integration for browser/network/runtime debugging

Runtime binding rule: when a skill requires asking the user, use `ask_user` and preserve the 5-option contract with option 4 fixed as `Freeform`.
