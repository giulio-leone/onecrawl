---
name: "Planning and Tracking"
description: "Execution plan with milestone/issue hierarchy, explicit dependencies, and safe parallelism."
---

# Planning & Tracking Skill

## Purpose

Create and maintain an execution plan with milestone/issue hierarchy, explicit dependencies, and safe parallelism.

## Use when

- Starting any non-trivial task
- Scope changes during execution
- Multiple files/workstreams must be coordinated

## Mandatory Schema

```typescript
interface Plan {
  PRD: string;
  context: string;
  milestones: Record<string, Milestone>;
}
interface Milestone {
  id: string;
  description: string;
  priority: "critical" | "high" | "medium" | "low";
  status: "todo" | "in_progress" | "review" | "done";
  depends_on: string[];
  issues: Record<string, Issue>;
}
interface Issue {
  id: string;
  task: string;
  priority: "critical" | "high" | "medium" | "low";
  status: "todo" | "in_progress" | "review" | "done" | "blocked";
  depends_on: string[];
  children: Record<string, Issue>;
}
```

## Procedure

1. Build plan before implementation.
2. Assign unique IDs to milestones/issues.
3. Declare dependencies for every issue (`depends_on`).
4. Execute by dependency order, then priority (`critical` → `high` → `medium` → `low`).
5. Run independent same-priority milestones in parallel when safe.
6. Update statuses continuously and append concise progress summaries.

## Done Criteria

- Plan exists, is up-to-date, and reflects actual execution state.
- Dependencies are respected and parallel work is safe.

## Anti-patterns

- Starting implementation without a plan
- Missing dependency declarations
- Running blocked items in parallel
