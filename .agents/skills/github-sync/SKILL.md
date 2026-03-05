---
name: "GitHub Sync"
description: "Keep local plan and GitHub project artifacts perfectly aligned with milestones, issues, labels, and statuses."
---

# GitHub Sync Skill

## Purpose

Keep local plan and GitHub project artifacts perfectly aligned (milestones, issues, labels, statuses, dependencies).

## Use when

- Creating/updating plan
- Changing issue status
- Completing milestones

## Naming Rules

- Milestone: `M<id> — <description>`
- Issue: `[M<milestone_id>] I<issue_id> — <task>`

## Required Metadata

- Priority label: `P-critical|P-high|P-medium|P-low`
- Type label: `feat|fix|refactor|chore|test`
- Status label: `in-progress|review|blocked`
- Milestone link
- Dependency note: `depends on #<issue_number>`
- Parent/child note: `part of #<issue_number>`

## Procedure

1. On plan create/update, create/update corresponding GitHub milestones/issues.
2. On local issue status change, sync GitHub status + labels immediately.
3. Close milestone when all linked issues are done.
4. Ensure no orphan issues (every issue belongs to a milestone).

## Done Criteria

- Local plan == GitHub state (titles, labels, status, dependencies).

## Anti-patterns

- Local-only tracking
- Orphan issues
- Stale labels/status
