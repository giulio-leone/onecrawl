# AGENTS.copilot-cli.MD

## 1. Interaction Protocol

Interaction must occur **exclusively via `ask_user` tool** and remain iterative, interactive, and unlimited.
If `ask_user` is unavailable in the current runtime, use an equivalent structured question tool while preserving the same interaction contract.
> **Execution Skill:** Follow `.agents/skills/interaction-loop/SKILL.md` for the iterative 5-option interaction loop.

Each iteration must ask exactly **one clear question** and provide exactly **5 options**.

Default option composition (for normal development decisions):
1. **Recommended Development Path** (mark as ⭐ most future-proof)
2. **Alternative Development Path A**
3. **Alternative Development Path B**
4. **Freeform**
5. **Autonomous Mode**

Continuity is mandatory: options **1-3** must, by default, carry forward all still-valid options from the previous iteration.
Do not drop an option silently: if one is removed, explicitly state why and what replaced it.

For each option, always include:
- why the option exists
- where the option leads next (expected outcome / milestone impact)
- risk level (low/medium/high)

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

Always explicitly indicate the **most future-proof** option.
At the beginning of each task, mode selection is mandatory: **never assume a default mode globally**.
Mode selection at task start must use the same 5-option structure above (development-options-first unless escalation is justified).

When the compatibility triad is used, always include at minimum:
- pros/cons and risk level
- migration steps
- explicit statement that quality gates are unchanged: unit/integration/E2E/non-regression tests must still pass
> **Execution Skill:** Follow `.agents/skills/breaking-change-paths/SKILL.md` for decision packaging and acceptance rules.

### Autonomous Mode

`Autonomous Mode` means:
- Work autonomously until **all milestones are completed**
- Always choose the **best, most future-proof** option within the selected path
- If a new breaking impact appears that was not explicitly approved, pause and use `ask_user`
- At the end of each autonomous run, always ask for:
   1. rating
   2. next action
   3. satisfaction confirmation

The interaction loop stops **only** when the user explicitly says:
> "I am satisfied"

Until then, the loop continues indefinitely.

### 1.1 Skill Quick Map (Operational)

- Interaction loop and checkpoints: `.agents/skills/interaction-loop/SKILL.md`
- Breaking vs Non-Breaking decisions: `.agents/skills/breaking-change-paths/SKILL.md`
- Planning and tracking: `.agents/skills/planning-tracking/SKILL.md`
- Universal completion gate: `.agents/skills/completion-gate/SKILL.md`
- GitHub synchronization: `.agents/skills/github-sync/SKILL.md`
- Testing policy (all layers): `.agents/skills/testing-policy/SKILL.md`
- E2E execution focus: `.agents/skills/e2e-testing/SKILL.md`
- Session logging: `.agents/skills/session-logging/SKILL.md`
- Rollback and RCA: `.agents/skills/rollback-rca/SKILL.md`
- Policy coherence audits: `.agents/skills/policy-coherence-audit/SKILL.md`

---

## 2. Planning, Tracking & Universal Completion Gate

A `plan` is mandatory: create it before execution, keep it continuously updated, and include a concise progress summary after each update.

### Mandatory Plan Schema

```typescript
interface Plan { PRD: string; context: string; milestones: Record<string, Milestone>; }
interface Milestone { id: string; description: string; priority: "critical"|"high"|"medium"|"low"; status: "todo"|"in_progress"|"review"|"done"; depends_on: string[]; issues: Record<string, Issue>; }
interface Issue { id: string; task: string; priority: "critical"|"high"|"medium"|"low"; status: "todo"|"in_progress"|"review"|"done"|"blocked"; depends_on: string[]; children: Record<string, Issue>; }
```

### Structural Rules

- Milestones/issues are hierarchical and IDs are unique
- Every issue explicitly declares `depends_on`
- Child issues are allowed
- Dependency graph must enable safe parallelization

### Milestone Execution Order

1. Respect dependency order (`depends_on` must be `done`)
2. Then priority: `critical` → `high` → `medium` → `low`
3. Independent milestones with same priority may run in parallel

### Universal Completion Gate (MANDATORY: Issue, Milestone, PR)

No issue, milestone, or PR can be marked complete/merged unless **all** checks pass; this gate applies **without exceptions**.
This applies equally to **Non-Breaking** and **Breaking** paths.
1. **Double consecutive review**: two consecutive full reviews with **0 errors** and **0 warnings**
2. Resolve all newly found issues and all **pre-existing issues in touched scope** that materially affect correctness, reliability, security, or maintainability of the current task; for non-critical leftovers, create linked follow-up issues before completion.
3. Lint/type-check/build/static analysis pass with 0 errors/0 warnings
4. Required tests pass: unit, integration (if applicable), E2E (if applicable), non-regression
5. Full test suite passes and coverage is **not below baseline**
No review/merge action is allowed before these checks are green.

### Milestone Completion

A milestone is `done` only when all issues are `done`, merged into the milestone branch, and final integration verification (build + full suite) passes with **0 errors/0 warnings**.

### Issue Completion Gate (Definition of Done)

An issue cannot be `done` until this review loop passes:
1. Run full review (lint, type-check, build, static analysis, required tests)
2. Review reports **0 errors** and **0 warnings**
3. If any issue appears, fix and restart from step 1
4. Mark `done` only after **two consecutive** clean passes

#### Additional Criteria

- [ ] Implementation complete and aligned with architectural principles
- [ ] Unit tests written and passing
- [ ] Integration tests written and passing (if applicable)
- [ ] E2E tests written and passing (if user-facing flow is affected)
- [ ] Regression tests passing (no previously passing test broken)
- [ ] Coverage not decreased vs baseline
- [ ] Code is self-documenting or documented where non-obvious
- [ ] Corresponding GitHub Issue updated with status and relevant notes

---

## 3. GitHub Project Sync
The local plan must be **mirrored on GitHub** at all times (single source of truth).
### Milestones
- Every plan milestone maps to a GitHub Milestone `M<id> — <description>` with priority, dependencies, acceptance criteria, and due date (when applicable).
### Issues
- Every plan issue maps to a GitHub Issue titled `[M<milestone_id>] I<issue_id> — <task>`.
- Required metadata:
   - labels (priority: `P-critical|P-high|P-medium|P-low`; type: `feat|fix|refactor|chore|test`; status: `in-progress|review|blocked`)
   - milestone link
   - dependencies: `depends on #<issue_number>`
   - parent/child: `part of #<issue_number>`
### Sync Rules
- Plan create/update ⇒ create/update matching GitHub milestones/issues.
- Local issue status change ⇒ immediate status/label sync (same iteration).
- Close milestone when all linked issues are `done`.
- Keep labels synchronized with plan status.
- **No orphan issues**: every issue belongs to a milestone.

---

## 4. Testing Policy
All code must be tested before completion/review gates.
### Requirements
- **Unit tests** for business logic, utilities, pure functions
- **Integration tests** for API routes, DB interactions, cross-module flows
- **E2E tests** for user-facing and critical flows
- **Non-regression tests** mandatory; no previously passing test may break
- Coverage must not decrease; required tests must pass with zero failures, and the required set must be green before any review/completion step
### E2E Test Rules
End-to-end tests validate complete flows across UI/API/CLI (or other entry points).
> **Execution Skill:** Follow `.agents/skills/e2e-testing/SKILL.md` before writing/running E2E tests.
#### Scope
- At least one E2E test per critical flow
- Cover happy path + key error/edge cases
- If a flow changes, add/update E2E for that flow
#### Execution
- E2E execution must be **programmatic** (never manual)
- Primary method: CLI scripts (e.g., `test:e2e`) with exit codes + structured output
- MCP/runtime verification is complementary (not a substitute for automated suite)
- Choose tooling by stack; for multi-layer systems, cover each layer
#### Requirements
- Non-interactive (CI-compatible)
- Deterministic (stable data/selectors, avoid flaky waits)
- Independent (setup/teardown per test)
- Must run and pass in CI before merge
#### Organization
- Store E2E tests in `e2e/` or `tests/e2e/`
- Group by feature/user flow
### Non-Regression Test Rules
#### Baseline & Comparison
- Before work: run full suite (unit + integration + E2E), record totals + coverage (line/branch)
- After work: rerun full suite and produce diff (added/removed/broken tests + coverage delta)
#### Enforcement
- Any previously passing test now failing blocks completion
- Coverage decrease is forbidden (restore/exceed baseline)
- Regression fixes must not introduce new regressions
- Each bug fix must include a regression test for that bug
### Principles
- Tests must be deterministic, isolated, repeatable
- No external service dependency unless explicitly mocked
- Test names must clearly describe expected behavior
- Cover edge/error paths, not only happy paths
- Prefer `data-testid` for E2E selectors over CSS/DOM structure

---

## 5. Session Logging
For each working session, create/update:
```text
sessions-<ISO-date>.md
```
### Format
```md
# Session <ISO date>
## Status
Milestones: m1 (done), m2 (in_progress), m3 (todo)
## Work Completed
- [m2/i1] Implemented authentication flow
- [m2/i2] Fixed type errors in user model
## Completion Gate Passed
- [m2/i1] ✅ 2 consecutive passes
- [m2/i2] ✅ 2 consecutive passes
## Decisions Made
- Chose JWT over session-based auth (see doc-first analysis)
## Blockers
- None | <describe blocker and status>
## GitHub Sync
- Created: #12, #13
- Closed: #10, #11
- Updated: #14 (status → in_progress)
## Branch
feat/m2-authentication
## Date
<ISO timestamp>
```
Session logging is mandatory and must accurately reflect progress.

---

## 6. Git Workflow

### Authorship

All commits/contributions must be authored only by the **repository owner** (no co-authors).
```bash
git config user.name "<owner-name>"
git config user.email "<owner-email>"
```
Never include `Co-authored-by` trailers or third-party attribution (including AI agents).

### Branch Strategy

For every milestone/major phase: create a dedicated branch + dedicated worktree (`git worktree add`), use worktrees for safe parallel execution, and preserve traceability across worktrees/branches/milestones/issues.
```bash
git worktree add ../project-m2 -b feat/m2-authentication
```

#### Branch Naming Convention
```text
<type>/<milestone-id>-<short-description>
```

Types: `feat` (feature/milestone), `fix` (bug fix), `refactor` (restructure), `chore` (tooling/config/CI).
Examples: `feat/m1-project-setup`, `fix/m2-i3-login-redirect`, `refactor/m4-hexagonal-migration`.

### Commit Convention

Use Conventional Commits and always reference issue ID:
```text
<type>(<scope>): <description>

Refs: #<github-issue-number>
```
Examples: `feat(auth): implement JWT token generation` + `Refs: #12`; `fix(m2-i3): resolve login redirect loop` + `Refs: #15`.
Rules: one logical change per commit, clear message, issue reference required, no `Co-authored-by`.

### Pull Request Policy

Every milestone branch must merge via PR:
- Title: `M<id> — <milestone description>`
- Body includes:
   - summary of changes
   - related issues (`Closes #...`)
   - confirmation of Completion Gate pass (**two consecutive clean reviews**)
   - test results (unit, integration, E2E, non-regression)
- Merge blocked unless Completion Gate is fully satisfied
- Squash merge preferred

### Structural Change Policy

If unexpected architectural/structural changes emerge: stop immediately, use `ask_user`, proceed only after approval.
No unapproved structural deviation is allowed.

---

## 7. Architectural Principles & Future-Proof Strategy
Always apply:
- **KISS**
- **DRY**
- **SOLID**
- **Hexagonal Architecture** (Ports & Adapters)
### Structural-vs-Surgical Conflict Resolution (Mandatory)
If "definitive structural fix" and "surgical change" seem in tension, resolve with this precedence:
1. Root cause elimination is mandatory (definitive fix takes priority).
2. "Surgical" means **bounded scope**, not superficial patching.
3. Deep local refactoring inside the impacted boundary is allowed and preferred when needed.
4. Temporary workarounds/hacks are never acceptable as a final state.
### Mandatory Triggers for Structural Refactor
Structural refactor is required when at least one condition applies:
- recurring bug or repeated regressions
- SOLID violations or harmful duplication
- security or reliability risk
- technical debt blocks safe evolution
- performance bottleneck impacts expected behavior
- recurring delivery loop where local patches fail repeatedly
Every decision must maximize extensibility, minimize technical debt, preserve architectural coherence, support long-term evolution, and be production-grade.
Strict constraints:
- No workarounds
- No temporary patches
- No short-term fixes
- No uncontrolled technical debt
- No destructive actions unless necessary
- No tactical shortcuts
Only definitive, scalable, long-term solutions are allowed.

---

## 8. Error Handling & Rollback Strategy
### When Completion Gate Fails 3 Times Consecutively
1. Stop all work on that issue
2. Perform root cause analysis (architecture/dependency/scope)
3. Use `ask_user` with the mandatory 5-option interaction format. For this case, present:
   - **Rescope** (split into smaller, testable sub-issues)
   - **Rollback** (return to last known good state)
   - **Redesign** (revisit architecture)
   - **Freeform** (custom user instruction)
   - **Autonomous Mode** (continue with the selected strategy)
4. Do not continue iterating blindly
### Rollback Rules
- Keep history clean and revertible; if rollback is chosen, revert to the last commit that passed Completion Gate and document the reason in the session log.

---

## 9. Subagent Orchestration
Use subagents whenever they improve quality, speed, or separation of concerns.
Consider subagents when:
- Work can be safely parallelized
- Responsibilities can be clearly isolated
- Specialized analysis/implementation is needed
- Independent streams reduce execution time
- Delegation can occur without dependency violations
All subagent activity must remain aligned with plan, dependencies, architecture, and future-proof strategy.
### Batch Tool Execution
For **3+ independent tool calls**, prefer programmatic batching (loop + aggregation + summary) to reduce latency, token use, and round-trips.
### Programmatic Code Transformations
For multi-file/repeated edits, prefer codemods/AST/batch scripts over manual edits for speed, consistency, and lower error rate.

---

## 10. Document-First Approach
Before implementation:
- Analyze official documentation
- Use documentation retrieval tools (MCP/context7/equivalent) and web search when required
All decisions must be documentation-aligned, standards-compliant, evidence-based, and verified before implementation.
For trivial, well-known fixes with no uncertainty in expected behavior, concise prior knowledge is acceptable for initial implementation, but documentation alignment must still be verified before completion/merge.
No implementation without prior documentation review.

---

## 11. Problem-Solving Framework

> **Maintenance Skill:** Use `.agents/skills/policy-coherence-audit/SKILL.md` when editing this policy to prevent contradictions.

When a problem arises:
1. Structured root cause analysis
2. Identify optimal long-term solution
3. Validate against architectural principles
4. Implement cleanly
5. Execute Completion Gate feedback loop
6. Fine-tune
If progress stalls: change strategy, reassess assumptions, avoid technical stubbornness, and stop ineffective iteration.
Objective: full autonomous resolution while preserving architectural integrity, scalability, and long-term system quality.
