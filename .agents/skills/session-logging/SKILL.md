---
name: "Session Logging"
description: "Accurate, auditable execution journal for each working session."
---

# Session Logging Skill

## Purpose

Maintain an accurate, auditable execution journal for each working session.

## Use when

- Starting/ending a session
- Completing issues/milestones
- Syncing GitHub status

## Required File

`sessions-<ISO-date>.md`

## Required Sections

- Status (milestone states)
- Work Completed (`[mX/iY]` references)
- Completion Gate Passed (include ✅ and consecutive-pass evidence)
- Decisions Made
- Blockers
- GitHub Sync (created/closed/updated issue IDs)
- Branch
- Date (ISO timestamp)

## Procedure

1. Create/update the session file at session start and after meaningful milestones.
2. Keep entries factual and aligned with plan/GitHub state.
3. Record gate-pass evidence per completed issue.

## Done Criteria

- Session file reflects real progress and traceable references.

## Anti-patterns

- Retroactive guesswork
- Missing gate evidence
- Inconsistent milestone/issue IDs
