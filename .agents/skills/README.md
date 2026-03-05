# Skills Index

Use this index to quickly select the right skill file.

## Decision & Interaction

- `interaction-loop/SKILL.md` — 5-option iterative question flow, rating/next action/satisfaction checkpoints.
- `breaking-change-paths/SKILL.md` — non-breaking vs breaking packaging, migration, unchanged quality gates.

## Planning & Delivery

- `planning-tracking/SKILL.md` — plan structure, milestone/issue flow.
- `completion-gate/SKILL.md` — mandatory quality gate and double clean-pass rule.
- `github-sync/SKILL.md` — mirror plan states to GitHub artifacts.
- `session-logging/SKILL.md` — mandatory session reporting format.
- `rollback-rca/SKILL.md` — failure handling, rollback, root-cause escalation.

## Testing

- `testing-policy/SKILL.md` — unit/integration/E2E/non-regression baseline and enforcement.
- `e2e-testing/SKILL.md` — focused E2E execution checklist and anti-flakiness rules.

## Policy Maintenance

- `policy-coherence-audit/SKILL.md` — detect and fix contradictions across AGENTS policies.

## Suggested invocation order (quick)

1. `interaction-loop` (select path)
2. `planning-tracking`
3. `breaking-change-paths` (if relevant)
4. `testing-policy` + `e2e-testing` (if relevant)
5. `completion-gate`
6. `session-logging` + `github-sync`
7. `rollback-rca` (only if blocked/failing repeatedly)
8. `policy-coherence-audit` (when editing policy)
